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

#[test]
fn unique_drops_duplicate_inputs() {
    let out = bin()
        .args([
            "--unique",
            "--vendor-only",
            "a4:83:e7:00:00:00",
            "a4:83:e7:11:22:33",
            "ff:ff:ff:00:00:00",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    // The two Apple OUIs differ only past the OUI but as strings are distinct,
    // so use identical strings to test dedup.
    let out2 = bin()
        .args([
            "--unique",
            "--vendor-only",
            "a4:83:e7:00:00:00",
            "a4:83:e7:00:00:00",
        ])
        .output()
        .unwrap();
    let s2 = String::from_utf8(out2.stdout).unwrap();
    assert_eq!(s2.lines().count(), 1);
    let _ = s;
}

#[test]
fn eui64_flag_outputs_identifier() {
    let out = bin()
        .args(["--eui64", "00:11:22:33:44:55"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert_eq!(s.trim(), "02:11:22:FF:FE:33:44:55");
}

#[test]
fn eui64_rejects_partial_mac() {
    // Only an OUI (3 octets) is not a full MAC.
    let out = bin().args(["--eui64", "00:11:22"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn vendors_flag_lists_names() {
    let out = bin().arg("--vendors").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = s.lines().collect();
    assert!(lines.len() > 1000);
    // Output is sorted.
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted);
}

#[test]
fn normalize_flag_canonicalizes() {
    let out = bin()
        .args(["--normalize", "aabb.ccdd.eeff"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert_eq!(s.trim(), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn normalize_lower_flag() {
    let out = bin()
        .args(["--normalize", "--lower", "AA-BB-CC-DD-EE-FF"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert_eq!(s.trim(), "aa:bb:cc:dd:ee:ff");
}

#[test]
fn normalize_rejects_partial() {
    let out = bin().args(["--normalize", "aa:bb:cc"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn link_local_flag_outputs_fe80() {
    let out = bin()
        .args(["--link-local", "00:11:22:33:44:55"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert_eq!(s.trim(), "fe80::211:22ff:fe33:4455");
}

#[test]
fn solicited_node_flag_derives_multicast_mac() {
    let out = bin()
        .args(["--solicited-node", "fe80::a483:e7ff:fe11:2233"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert_eq!(s.trim(), "33:33:FF:11:22:33");
}

#[test]
fn stats_flag_prints_count_tab_name() {
    let out = bin().args(["--stats", "3"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 3);
    // Each line is "<count>\t<name>" with the count non-increasing.
    let mut prev = usize::MAX;
    for line in lines {
        let (n, _name) = line.split_once('\t').expect("tab separator");
        let count: usize = n.parse().expect("count is an integer");
        assert!(count <= prev);
        prev = count;
    }
}

#[test]
fn scope_flag_classifies_specific_buckets() {
    let out = bin()
        .args([
            "--scope",
            "ff:ff:ff:ff:ff:ff",
            "01:00:5e:00:00:01",
            "33:33:00:00:00:01",
            "01:80:c2:00:00:00",
            "00:00:5e:00:01:0a",
            "a4:83:e7:00:00:01",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let want = "broadcast\nipv4-multicast\nipv6-multicast\nbridge-protocol\nvrrp\nglobal-unicast\n";
    assert_eq!(s, want);
}

#[test]
fn format_bare_strips_separators() {
    // FF:FF:FF is guaranteed not to resolve to any registered vendor, which
    // keeps this test independent of the embedded registry snapshot.
    let out = bin()
        .args(["--format", "bare", "FF:FF:FF:00:00:00"])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    // Bare prefix is six contiguous hex digits, followed by a TAB and the
    // (possibly empty) vendor name.
    let first = s.lines().next().unwrap();
    let (prefix, _) = first.split_once('\t').unwrap_or((first, ""));
    assert_eq!(prefix, "FFFFFF");
}
