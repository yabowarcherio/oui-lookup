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
use oui_lookup::{classify, format_oui, parse_mac48, parse_oui, search};

/// Output format for lookup results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable text (default).
    Text,
    /// Tab-separated values (OUI TAB vendor).
    Tsv,
    /// Comma-separated values (OUI,vendor).
    Csv,
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
    #[arg(value_name = "MAC", required_unless_present_any = ["count", "search", "input"])]
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

    let inputs = match collect_inputs(&cli.addrs, &cli.input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oui-lookup: failed to read input: {e}");
            return ExitCode::from(2);
        }
    };

    let records: Vec<Record> = inputs.iter().map(|s| resolve(s)).collect();

    let stdout = io::stdout();
    let mut out = stdout.lock();

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
