//! Command-line interface for `oui-lookup`.
//!
//! ```text
//! oui-lookup 00:11:22:33:44:55
//! oui-lookup --json a4:83:e7 28:cf:e9
//! echo "00:11:22" | oui-lookup -
//! ```

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
    #[arg(value_name = "MAC", required_unless_present_any = ["count", "search"])]
    addrs: Vec<String>,

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

/// Expand the argument list, replacing a `-` with lines read from stdin.
fn collect_inputs(args: &[String]) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for a in args {
        if a == "-" {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let line = line?;
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        } else {
            out.push(a.clone());
        }
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

    let inputs = match collect_inputs(&cli.addrs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("oui-lookup: failed to read stdin: {e}");
            return ExitCode::from(2);
        }
    };

    let records: Vec<Record> = inputs.iter().map(|s| resolve(s)).collect();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if cli.json {
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
