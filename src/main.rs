//! Command-line interface for `oui-lookup`.
//!
//! ```text
//! oui-lookup 00:11:22:33:44:55
//! oui-lookup --json a4:83:e7 28:cf:e9
//! echo "00:11:22" | oui-lookup -
//! ```

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::Parser;
use oui_lookup::{format_oui, parse_oui};

/// Offline MAC-address vendor (OUI) lookup.
#[derive(Parser, Debug)]
#[command(
    name = "oui-lookup",
    version,
    about,
    long_about = None,
    after_help = "Reads addresses from arguments, or one per line from stdin when given `-`.\n\
                  Exit code is 1 if any address parsed but matched no vendor."
)]
struct Cli {
    /// MAC addresses or OUI prefixes to look up. Use `-` to read from stdin.
    #[arg(value_name = "MAC", required = true)]
    addrs: Vec<String>,

    /// Emit results as a JSON array.
    #[arg(long)]
    json: bool,

    /// Suppress the "(unknown)" lines for unmatched addresses (human output).
    #[arg(long, conflicts_with = "json")]
    quiet: bool,
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
    } else {
        for r in &records {
            if let Some(err) = &r.error {
                let _ = writeln!(out, "{:<20} error: {err}", r.input);
            } else if let Some(v) = &r.vendor {
                let _ = writeln!(out, "{:<20} {}", r.prefix.as_deref().unwrap_or(""), v);
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
