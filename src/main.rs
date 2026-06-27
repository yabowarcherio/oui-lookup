//! Command-line interface for `oui-lookup`.
//!
//! ```text
//! oui-lookup 00:11:22:33:44:55
//! oui-lookup --json a4:83:e7 28:cf:e9
//! echo "00:11:22" | oui-lookup -
//! ```

use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use oui_lookup::{classify, format_oui, parse_mac48, parse_oui, scope, search, to_eui64};

/// Output format for lookup results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable text (default).
    Text,
    /// Tab-separated values (OUI TAB vendor).
    Tsv,
    /// Comma-separated values (OUI,vendor).
    Csv,
    /// Bare OUI (no separators) followed by vendor.
    Bare,
    /// Newline-delimited JSON, one record per line.
    Ndjson,
}

/// Offline MAC-address vendor (OUI) lookup.
#[derive(Parser, Debug)]
#[command(
    name = "oui-lookup",
    version,
    about,
    long_about = None,
    after_help = "Reads addresses from arguments, or one per line from stdin when given `-`.\n\
                  Exit code is 1 if any address parsed but matched no vendor, 2 on a parse error."
)]
struct Cli {
    /// MAC addresses or OUI prefixes to look up. Use `-` to read from stdin.
    #[arg(value_name = "MAC", required_unless_present_any = ["count", "search", "input", "vendors", "vendor", "prefix_range", "from_eui64"])]
    addrs: Vec<String>,

    /// Read addresses from a file, one per line (repeatable). Use `-` for
    /// stdin. Blank lines and lines beginning with `#` are ignored.
    #[arg(short = 'i', long = "input", value_name = "FILE")]
    input: Vec<String>,

    /// Emit results as a JSON array.
    #[arg(long)]
    json: bool,

    /// Suppress the "(unknown)" lines for unmatched addresses (human output).
    #[arg(long, conflicts_with = "json")]
    quiet: bool,

    /// Print the number of OUI entries embedded in this build and exit.
    #[arg(long, exclusive = true)]
    count: bool,

    /// In human output, also print the address class (unicast/multicast/...).
    #[arg(long, conflicts_with = "json")]
    class: bool,

    /// Search the registry for vendors whose name contains this text, print
    /// their OUI prefixes, and exit.
    #[arg(long, value_name = "TEXT")]
    search: Option<String>,

    /// Limit the number of rows printed by --search (0 = no limit).
    #[arg(long, value_name = "N", default_value_t = 0, requires = "search")]
    limit: usize,

    /// Output format for lookup results (text, tsv, csv).
    #[arg(long, value_enum, default_value_t = Format::Text, conflicts_with = "json")]
    format: Format,

    /// Print only the vendor name for each input (empty line if unknown),
    /// one per line — convenient for shell pipelines.
    #[arg(long, conflicts_with_all = ["json", "class"])]
    vendor_only: bool,

    /// Drop duplicate inputs, keeping the first occurrence of each.
    #[arg(long)]
    unique: bool,

    /// Drop inputs whose OUI is not in the embedded registry. Combine with
    /// --vendor-only to get only the names that resolved.
    #[arg(long, conflicts_with = "search")]
    known_only: bool,

    /// Print the Modified EUI-64 identifier for each full MAC, then exit.
    #[arg(long, conflicts_with_all = ["json", "class", "vendor_only"])]
    eui64: bool,

    /// Print every distinct vendor name in the registry, sorted, then exit.
    #[arg(long, exclusive = true)]
    vendors: bool,

    /// Print the top-N vendors by OUI-block count, then exit.
    #[arg(long, value_name = "N", exclusive = true)]
    stats: Option<usize>,

    /// Print every OUI prefix registered to a vendor name (exact match,
    /// case-insensitive) and exit. Use --search for substring matching.
    #[arg(long, value_name = "NAME", exclusive = true)]
    vendor: Option<String>,

    /// List every entry whose OUI prefix falls in `FROM..TO` (inclusive,
    /// `AA:BB:CC..DD:EE:FF`). Output is `OUI<TAB>VENDOR`.
    #[arg(long, value_name = "FROM..TO", exclusive = true)]
    prefix_range: Option<String>,

    /// For each Modified EUI-64 identifier (8 bytes, any separator), recover
    /// the underlying 48-bit MAC and print `MAC<TAB>VENDOR` (vendor empty when
    /// unregistered). Exits 2 if any EUI lacks the `FF:FE` marker.
    #[arg(long, value_name = "EUI64", exclusive = true, num_args = 1..)]
    from_eui64: Vec<String>,

    /// Print the canonical form of each full MAC, then exit.
    #[arg(long, conflicts_with_all = ["json", "class", "vendor_only", "eui64"])]
    normalize: bool,

    /// Print the IPv6 link-local address (fe80::/64) for each full MAC, then
    /// exit.
    #[arg(long, conflicts_with_all = ["json", "class", "vendor_only", "eui64", "normalize"])]
    link_local: bool,

    /// Print the address scope (broadcast, ipv4-multicast, vrrp, …) for each
    /// full MAC, then exit. More specific than --class.
    #[arg(long, conflicts_with_all = ["json", "class", "vendor_only", "eui64", "normalize", "link_local"])]
    scope: bool,

    /// For each IPv6 address argument, print the solicited-node multicast MAC
    /// (33:33:FF:xx:xx:xx) per RFC 4861 §7.1, then exit.
    #[arg(long, exclusive = true, value_name = "IPV6", num_args = 1..)]
    solicited_node: Vec<String>,

    /// With --normalize/--eui64 output, use the lower-case colon form.
    #[arg(long)]
    lower: bool,

    /// Print "total<TAB>parseable<TAB>known" and exit. Each tally is computed
    /// once over the resolved input list; useful for batch sanitization
    /// pipelines.
    #[arg(long, conflicts_with_all = ["json", "class", "vendor_only", "eui64", "normalize", "link_local", "scope"])]
    tally: bool,
}

/// One resolved (or unresolved) lookup result.
#[derive(serde::Serialize)]
struct Record {
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn resolve(input: &str) -> Record {
    match parse_oui(input) {
        Ok(oui) => {
            let prefix = format_oui(oui);
            let vendor = oui_lookup::lookup(input).map(str::to_string);
            Record {
                input: input.to_string(),
                prefix: Some(prefix),
                vendor,
                error: None,
            }
        }
        Err(e) => Record {
            input: input.to_string(),
            prefix: None,
            vendor: None,
            error: Some(e.to_string()),
        },
    }
}

/// Append every non-blank, non-comment line from `reader` to `out`.
fn read_lines<R: BufRead>(reader: R, out: &mut Vec<String>) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            out.push(trimmed.to_string());
        }
    }
    Ok(())
}

/// Read addresses from a file path, treating `-` as stdin.
fn read_path(path: &str, out: &mut Vec<String>) -> io::Result<()> {
    if path == "-" {
        read_lines(io::stdin().lock(), out)
    } else {
        let file =
            File::open(path).map_err(|e| io::Error::new(e.kind(), format!("{path}: {e}")))?;
        read_lines(io::BufReader::new(file), out)
    }
}

/// Build the full address list from positional args (where `-` means stdin) and
/// any `--input` files, preserving order: positionals first, then each file.
fn collect_inputs(args: &[String], files: &[String]) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for a in args {
        if a == "-" {
            read_path("-", &mut out)?;
        } else {
            out.push(a.clone());
        }
    }
    for f in files {
        read_path(f, &mut out)?;
    }
    Ok(out)
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.count {
        println!("{}", oui_lookup::ENTRY_COUNT);
        return ExitCode::SUCCESS;
    }

    if cli.vendors {
        for name in oui_lookup::vendors() {
            println!("{name}");
        }
        return ExitCode::SUCCESS;
    }

    if let Some(n) = cli.stats {
        for (name, count) in oui_lookup::top_vendors(n) {
            println!("{count}\t{name}");
        }
        return ExitCode::SUCCESS;
    }

    if let Some(name) = &cli.vendor {
        let mut found = 0usize;
        for e in oui_lookup::prefixes_for(name) {
            println!("{}", e.prefix_str());
            found += 1;
        }
        return if found > 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    if !cli.from_eui64.is_empty() {
        let mut bad = false;
        for s in &cli.from_eui64 {
            match oui_lookup::parse_eui64(s) {
                Ok(eui) => match oui_lookup::eui64_to_mac(eui) {
                    Some(mac) => {
                        let mac_s = if cli.lower {
                            oui_lookup::format_mac48_lower(mac)
                        } else {
                            oui_lookup::format_mac48(mac)
                        };
                        let vendor = oui_lookup::lookup_octets(mac).unwrap_or("");
                        println!("{mac_s}\t{vendor}");
                    }
                    None => {
                        eprintln!("oui-lookup: {s:?}: not derived from a 48-bit MAC (missing FF:FE marker)");
                        bad = true;
                    }
                },
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if let Some(spec) = &cli.prefix_range {
        let Some((lo_s, hi_s)) = spec.split_once("..") else {
            eprintln!("oui-lookup: --prefix-range expected FROM..TO, got {spec:?}");
            return ExitCode::from(2);
        };
        let lo = match parse_oui(lo_s.trim()) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("oui-lookup: bad range start {lo_s:?}: {err}");
                return ExitCode::from(2);
            }
        };
        let hi = match parse_oui(hi_s.trim()) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("oui-lookup: bad range end {hi_s:?}: {err}");
                return ExitCode::from(2);
            }
        };
        let mut found = 0usize;
        for e in oui_lookup::entries_between(lo, hi) {
            println!("{}\t{}", e.prefix_str(), e.name);
            found += 1;
        }
        return if found > 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    if !cli.solicited_node.is_empty() {
        let mut bad = false;
        for s in &cli.solicited_node {
            match s.parse::<std::net::Ipv6Addr>() {
                Ok(addr) => {
                    let m = oui_lookup::solicited_node_mac(addr);
                    println!(
                        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        m[0], m[1], m[2], m[3], m[4], m[5]
                    );
                }
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if let Some(term) = &cli.search {
        let mut found = 0usize;
        for e in search(term) {
            if cli.limit != 0 && found >= cli.limit {
                break;
            }
            println!("{}\t{}", e.prefix_str(), e.name);
            found += 1;
        }
        return if found > 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    let mut inputs = match collect_inputs(&cli.addrs, &cli.input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oui-lookup: failed to read input: {e}");
            return ExitCode::from(2);
        }
    };

    if cli.unique {
        let mut seen = std::collections::HashSet::new();
        inputs.retain(|s| seen.insert(s.clone()));
    }

    if cli.known_only {
        inputs.retain(|s| oui_lookup::is_registered(s));
    }

    if cli.tally {
        let total = inputs.len();
        let parseable = oui_lookup::count_parseable(&inputs);
        let known = oui_lookup::count_known(&inputs);
        println!("{total}\t{parseable}\t{known}");
        return ExitCode::SUCCESS;
    }

    let records: Vec<Record> = inputs.iter().map(|s| resolve(s)).collect();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if cli.normalize {
        let mut bad = false;
        for s in &inputs {
            let normalized = if cli.lower {
                oui_lookup::normalize_mac_lower(s)
            } else {
                oui_lookup::normalize_mac(s)
            };
            match normalized {
                Ok(text) => {
                    let _ = writeln!(out, "{text}");
                }
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if cli.link_local {
        let mut bad = false;
        for s in &inputs {
            match parse_mac48(s) {
                Ok(mac) => {
                    let _ = writeln!(out, "{}", oui_lookup::link_local_ipv6(mac));
                }
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if cli.scope {
        let mut bad = false;
        for s in &inputs {
            match parse_mac48(s) {
                Ok(mac) => {
                    let _ = writeln!(out, "{}", scope(mac));
                }
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if cli.eui64 {
        let mut bad = false;
        for s in &inputs {
            match parse_mac48(s) {
                Ok(mac) => {
                    let e = to_eui64(mac);
                    let line = if cli.lower {
                        format!(
                            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]
                        )
                    } else {
                        format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]
                        )
                    };
                    let _ = writeln!(out, "{line}");
                }
                Err(err) => {
                    eprintln!("oui-lookup: {s:?}: {err}");
                    bad = true;
                }
            }
        }
        return if bad {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    if cli.vendor_only {
        for r in &records {
            let _ = writeln!(out, "{}", r.vendor.as_deref().unwrap_or(""));
        }
    } else if cli.json {
        match serde_json::to_writer_pretty(&mut out, &records) {
            Ok(()) => {
                let _ = writeln!(out);
            }
            Err(e) => {
                eprintln!("oui-lookup: failed to serialize JSON: {e}");
                return ExitCode::from(2);
            }
        }
    } else if cli.format == Format::Tsv {
        for r in &records {
            let prefix = r.prefix.as_deref().unwrap_or(&r.input);
            let vendor = r.vendor.as_deref().unwrap_or("");
            let _ = writeln!(out, "{prefix}\t{vendor}");
        }
    } else if cli.format == Format::Csv {
        for r in &records {
            let prefix = r.prefix.as_deref().unwrap_or(&r.input);
            // Quote the vendor name in case it contains a comma.
            let vendor = r.vendor.as_deref().unwrap_or("");
            if vendor.contains(',') || vendor.contains('"') {
                let escaped = vendor.replace('"', "\"\"");
                let _ = writeln!(out, "{prefix},\"{escaped}\"");
            } else {
                let _ = writeln!(out, "{prefix},{vendor}");
            }
        }
    } else if cli.format == Format::Bare {
        for r in &records {
            // Strip the colons from the formatted prefix so the output is six
            // contiguous hex digits — friendly for downstream tools that
            // don't tolerate punctuation.
            let prefix = r
                .prefix
                .as_deref()
                .map(|p| p.replace(':', ""))
                .unwrap_or_else(|| r.input.clone());
            let vendor = r.vendor.as_deref().unwrap_or("");
            let _ = writeln!(out, "{prefix}\t{vendor}");
        }
    } else if cli.format == Format::Ndjson {
        for r in &records {
            match serde_json::to_string(r) {
                Ok(line) => {
                    let _ = writeln!(out, "{line}");
                }
                Err(e) => {
                    eprintln!("oui-lookup: failed to serialize JSON: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    } else {
        for r in &records {
            if let Some(err) = &r.error {
                let _ = writeln!(out, "{:<20} error: {err}", r.input);
            } else if let Some(v) = &r.vendor {
                if cli.class {
                    let cls = parse_mac48(&r.input)
                        .map(|m| classify(m).to_string())
                        .unwrap_or_else(|_| "-".to_string());
                    let _ = writeln!(
                        out,
                        "{:<20} {:<16} {}",
                        r.prefix.as_deref().unwrap_or(""),
                        cls,
                        v
                    );
                } else {
                    let _ = writeln!(out, "{:<20} {}", r.prefix.as_deref().unwrap_or(""), v);
                }
            } else if !cli.quiet {
                let _ = writeln!(
                    out,
                    "{:<20} (unknown)",
                    r.prefix.as_deref().unwrap_or(&r.input)
                );
            }
        }
    }

    // Exit 1 if every-thing parsed but at least one had no vendor; exit 2 on a
    // hard parse error so scripts can tell "unknown vendor" from "bad input".
    let any_parse_error = records.iter().any(|r| r.error.is_some());
    let any_unmatched = records
        .iter()
        .any(|r| r.error.is_none() && r.vendor.is_none());

    if any_parse_error {
        ExitCode::from(2)
    } else if any_unmatched {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
