//! Integration tests against the real embedded IEEE table.
//!
//! These assert behavior, not specific vendor strings (which change as the
//! registry is refreshed). They check that the table is populated, that the
//! public API is consistent across its entry points, and that format handling
//! is robust.

use oui_lookup::{lookup, lookup_vendor, try_lookup, ENTRY_COUNT};

#[test]
fn table_is_substantial() {
    // The IEEE MA-L registry has tens of thousands of entries; guard against a
    // build that embedded an empty or truncated table. `ENTRY_COUNT` is a
    // compile-time constant, so assert it in a const block — the check then
    // fails the build, not just the test, if the embedded data is missing.
    const { assert!(ENTRY_COUNT > 10_000, "embedded table looks too small") };
}

#[test]
fn all_mac_formats_agree() {
    // Whatever the first registered prefix resolves to, every spelling of the
    // same OUI must resolve identically.
    let formats = [
        "3C:5A:B4:00:00:00",
        "3c-5a-b4-00-00-00",
        "3c5a.b400.0000",
        "3c5ab4000000",
        "3C:5A:B4",
    ];
    let first = lookup(formats[0]);
    for f in &formats[1..] {
        assert_eq!(lookup(f), first, "format {f} disagreed");
    }
}

#[test]
fn lookup_and_try_lookup_are_consistent() {
    let samples = ["00:11:22", "a4:83:e7", "FF:FF:FF", "AC:DE:48"];
    for s in samples {
        assert_eq!(lookup(s), try_lookup(s).unwrap(), "mismatch for {s}");
    }
}

#[test]
fn lookup_vendor_matches_lookup() {
    for s in ["a4:83:e7:01:02:03", "FF:FF:FF:FF:FF:FF"] {
        match (lookup(s), lookup_vendor(s)) {
            (Some(name), Some(v)) => assert_eq!(name, v.name),
            (None, None) => {}
            (a, b) => panic!("inconsistent for {s}: {a:?} vs {b:?}"),
        }
    }
}

#[test]
fn bad_input_errors_in_try_lookup_but_is_none_in_lookup() {
    for bad in ["", "xyz", "00:11", "gg:hh:ii"] {
        assert!(try_lookup(bad).is_err(), "{bad:?} should be a parse error");
        assert_eq!(lookup(bad), None, "{bad:?} should be None via lookup");
    }
}
