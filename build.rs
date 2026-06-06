//! Build script for `oui-lookup`.
//!
//! Reads the vendored, gzip-compressed IEEE OUI table (`data/oui.tsv.gz`) and
//! re-packs it into a compact binary blob that is embedded into the crate via
//! `include_bytes!`. No network access is performed at build time — the source
//! data is committed to the repository so `cargo build` works fully offline and
//! CI is deterministic. Refreshing the data is a separate, explicit step
//! (`scripts/update-oui.sh`).
//!
//! ## On-disk blob layout (all integers little-endian)
//!
//! ```text
//! magic:   [u8; 4] = b"OUI1"
//! count:   u32                       number of entries
//! entries: [ (u32 prefix, u32 off) ; count ]   sorted ascending by prefix
//! pool:    UTF-8 bytes               vendor names, each length-prefixed
//! ```
//!
//! For each entry, `off` is a byte offset into `pool`. At that offset is a
//! `u16` length followed by that many UTF-8 bytes (the vendor name). Entries
//! are sorted by `prefix` so the runtime can binary-search them.

use std::env;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use flate2::read::GzDecoder;

const MAGIC: &[u8; 4] = b"OUI1";

fn main() {
    let src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("data/oui.tsv.gz");
    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed=build.rs");

    // Decompress the vendored table.
    let gz = File::open(&src).unwrap_or_else(|e| panic!("failed to open {}: {e}", src.display()));
    let mut tsv = String::new();
    GzDecoder::new(gz)
        .read_to_string(&mut tsv)
        .expect("failed to gunzip data/oui.tsv.gz");

    // Parse "PREFIX\tVendor" lines into sorted (u32, &str) pairs.
    let mut entries: Vec<(u32, &str)> = Vec::with_capacity(64 * 1024);
    for (lineno, line) in tsv.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let (hex, vendor) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("malformed line {} (no tab): {line:?}", lineno + 1));
        let prefix = u32::from_str_radix(hex, 16)
            .unwrap_or_else(|e| panic!("bad hex prefix on line {}: {hex:?}: {e}", lineno + 1));
        let vendor = vendor.trim();
        if vendor.is_empty() {
            continue;
        }
        entries.push((prefix, vendor));
    }

    entries.sort_by_key(|(p, _)| *p);
    // The IEEE registry should not contain duplicate MA-L prefixes; if a refresh
    // ever introduces one, keep the first and warn rather than silently shadow.
    entries.dedup_by(|a, b| {
        if a.0 == b.0 {
            println!(
                "cargo:warning=duplicate OUI prefix {:06X}, keeping first",
                a.0
            );
            true
        } else {
            false
        }
    });

    // Build the string pool and the offset for each entry.
    let mut pool: Vec<u8> = Vec::with_capacity(entries.len() * 16);
    let mut offsets: Vec<u32> = Vec::with_capacity(entries.len());
    for (_, vendor) in &entries {
        let bytes = vendor.as_bytes();
        assert!(
            bytes.len() <= u16::MAX as usize,
            "vendor name longer than 65535 bytes: {vendor:?}"
        );
        offsets.push(pool.len() as u32);
        pool.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
        pool.extend_from_slice(bytes);
    }

    // Write the packed blob to OUT_DIR.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("oui.bin");
    let mut w = BufWriter::new(File::create(&out).expect("create oui.bin"));
    w.write_all(MAGIC).unwrap();
    w.write_all(&(entries.len() as u32).to_le_bytes()).unwrap();
    for ((prefix, _), off) in entries.iter().zip(&offsets) {
        w.write_all(&prefix.to_le_bytes()).unwrap();
        w.write_all(&off.to_le_bytes()).unwrap();
    }
    w.write_all(&pool).unwrap();
    w.flush().unwrap();

    println!("cargo:rustc-env=OUI_ENTRY_COUNT={}", entries.len());
}
