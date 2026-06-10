//! Black-box tests for the `oui-lookup` binary.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oui-lookup"))
}

#[test]
fn count_flag_prints_a_number() {
    let out = bin().arg("--count").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let n: usize = s.trim().parse().expect("--count should print a number");
    assert!(n > 10_000);
}

#[test]
fn unknown_oui_exits_one() {
    let out = bin().arg("FF:FF:FF:00:00:00").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn bad_input_exits_two() {
    let out = bin().arg("not-a-mac!").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn json_flag_emits_array() {
    let out = bin()
        .args(["--json", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.trim_start().starts_with('['));
}

#[test]
fn class_flag_labels_global_unicast() {
    let out = bin()
        .args(["--class", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("global-unicast"), "got: {s}");
}

#[test]
fn search_flag_lists_matches() {
    let out = bin().args(["--search", "apple"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.to_lowercase().contains("apple"));
    assert!(s.contains(':'), "should print OUI prefixes");
}

#[test]
fn search_no_match_exits_one() {
    let out = bin()
        .args(["--search", "zzz-no-such-vendor-zzz"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn search_limit_caps_rows() {
    let out = bin()
        .args(["--search", "inc", "--limit", "3"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    let rows = s.lines().filter(|l| !l.is_empty()).count();
    assert!(rows <= 3, "got {rows} rows");
}

#[test]
fn format_tsv_contains_tab() {
    let out = bin()
        .args(["--format", "tsv", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains('\t'), "tsv output should contain a tab: {s:?}");
}

#[test]
fn format_csv_contains_comma() {
    let out = bin()
        .args(["--format", "csv", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains(','), "csv output should contain a comma: {s:?}");
}

#[test]
fn format_tsv_two_columns() {
    let out = bin()
        .args(["--format", "tsv", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    let line = s.lines().next().expect("at least one line");
    let cols: Vec<&str> = line.splitn(2, '\t').collect();
    assert_eq!(
        cols.len(),
        2,
        "expected two tab-separated columns: {line:?}"
    );
}

#[test]
fn format_csv_two_columns() {
    let out = bin()
        .args(["--format", "csv", "a4:83:e7:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    let line = s.lines().next().expect("at least one line");
    // At least one comma must be present.
    assert!(
        line.contains(','),
        "expected comma-separated output: {line:?}"
    );
}

#[test]
fn input_file_reads_addresses() {
    use std::io::Write;
    let mut path = std::env::temp_dir();
    path.push(format!("oui-input-{}.txt", std::process::id()));
    {
        let mut f = std::fs::File::create(&path).unwrap();
        // Blank lines and comments must be ignored.
        writeln!(f, "# a comment").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "a4:83:e7:00:00:00").unwrap();
    }
    let out = bin().args(["--input"]).arg(&path).output().unwrap();
    let _ = std::fs::remove_file(&path);
    let s = String::from_utf8(out.stdout).unwrap();
    let rows = s.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(rows, 1, "exactly one address should resolve: {s:?}");
}

#[test]
fn missing_input_file_exits_two() {
    let out = bin()
        .args(["--input", "/no/such/oui/file.txt"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn vendor_only_prints_just_names() {
    let out = bin()
        .args(["--vendor-only", "a4:83:e7:00:00:00", "ff:ff:ff:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = s.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(!lines[0].is_empty()); // known vendor
    assert_eq!(lines[1], ""); // unknown -> blank
}
