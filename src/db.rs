//! Runtime access to the embedded, pre-packed OUI table.
//!
//! `build.rs` writes a compact binary blob (see its module docs for the
//! layout) which is `include_bytes!`-d here. The blob itself is gzip-free —
//! it is the *vendored source* (`data/oui.tsv.gz`) that is compressed, and the
//! build script decompresses it once at compile time. The embedded blob is
//! parsed lazily into borrowed slices on first lookup; no allocation or
//! decompression happens at runtime.

use std::sync::OnceLock;

/// The packed blob produced by `build.rs`.
static BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/oui.bin"));

const MAGIC: &[u8; 4] = b"OUI1";
const HEADER_LEN: usize = 8; // 4 magic + 4 count
const ENTRY_LEN: usize = 8; // u32 prefix + u32 offset

/// Number of OUI entries embedded in this build (set by `build.rs`).
pub const ENTRY_COUNT: usize = {
    // env! gives a &str at compile time; parse it in a const context.
    match usize::from_str_radix(env!("OUI_ENTRY_COUNT"), 10) {
        Ok(n) => n,
        Err(_) => 0,
    }
};

/// A parsed, borrowed view over the embedded blob.
struct Table {
    /// `count` (u32 prefix, u32 offset) records, sorted ascending by prefix.
    entries: &'static [u8],
    /// Length-prefixed UTF-8 vendor strings.
    pool: &'static [u8],
    count: usize,
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        assert!(BLOB.len() >= HEADER_LEN, "embedded OUI blob is truncated");
        assert_eq!(&BLOB[0..4], MAGIC, "embedded OUI blob has wrong magic");
        let count = u32::from_le_bytes([BLOB[4], BLOB[5], BLOB[6], BLOB[7]]) as usize;
        let entries_start = HEADER_LEN;
        let entries_end = entries_start + count * ENTRY_LEN;
        assert!(
            BLOB.len() >= entries_end,
            "embedded OUI blob is shorter than its declared entry count"
        );
        Table {
            entries: &BLOB[entries_start..entries_end],
            pool: &BLOB[entries_end..],
            count,
        }
    })
}

impl Table {
    #[inline]
    fn prefix_at(&self, i: usize) -> u32 {
        let b = &self.entries[i * ENTRY_LEN..];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    #[inline]
    fn offset_at(&self, i: usize) -> usize {
        let b = &self.entries[i * ENTRY_LEN + 4..];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
    }

    #[inline]
    fn vendor_at(&self, i: usize) -> &'static str {
        let off = self.offset_at(i);
        let len = u16::from_le_bytes([self.pool[off], self.pool[off + 1]]) as usize;
        let start = off + 2;
        // The pool is built from valid UTF-8 in build.rs.
        std::str::from_utf8(&self.pool[start..start + len])
            .expect("vendor name in embedded blob is not valid UTF-8")
    }

    /// Binary-search the sorted entry table for an exact 24-bit OUI prefix.
    fn find(&self, oui: u32) -> Option<&'static str> {
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let p = self.prefix_at(mid);
            if p == oui {
                return Some(self.vendor_at(mid));
            } else if p < oui {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        None
    }
}

/// Look up a 24-bit OUI prefix (low 24 bits of the `u32`) in the embedded table.
///
/// Returns the vendor name as a `'static` string slice borrowed directly from
/// the embedded blob, or `None` if the prefix is not registered.
#[inline]
pub fn lookup_prefix(oui: u32) -> Option<&'static str> {
    table().find(oui & 0x00FF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_count_is_populated() {
        const { assert!(ENTRY_COUNT > 1000, "expected a full IEEE table") };
        // The compile-time constant must match what the runtime blob reports.
        assert_eq!(ENTRY_COUNT, table().count);
    }

    #[test]
    fn table_is_sorted() {
        let t = table();
        for i in 1..t.count {
            assert!(
                t.prefix_at(i - 1) < t.prefix_at(i),
                "table not sorted at {i}"
            );
        }
    }

    #[test]
    fn unknown_prefix_returns_none() {
        // FF:FF:FF is reserved (broadcast) and not an assigned OUI.
        assert_eq!(lookup_prefix(0xFFFFFF), None);
    }

    #[test]
    fn high_bits_are_masked() {
        // Passing a full 32-bit value should mask to the low 24 bits.
        let first = table().prefix_at(0);
        assert_eq!(lookup_prefix(first), lookup_prefix(0xAB00_0000 | first));
    }
}
