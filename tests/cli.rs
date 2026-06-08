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
