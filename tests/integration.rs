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

#[test]
fn lookup_many_preserves_order_and_count() {
    use oui_lookup::lookup_many;
    let input = ["a4:83:e7:00:00:00", "garbage", "ff:ff:ff:00:00:00"];
    let out = lookup_many(input);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], oui_lookup::lookup(input[0]));
    assert_eq!(out[1], None);
    assert_eq!(out[2], None);
}

#[test]
fn classify_is_consistent_with_bit_helpers() {
    use oui_lookup::{classify, is_broadcast, is_multicast, parse_mac48, MacKind};
    let bcast = parse_mac48("ff:ff:ff:ff:ff:ff").unwrap();
    assert!(is_broadcast(bcast));
    assert_eq!(classify(bcast), MacKind::Broadcast);

    let mc = parse_mac48("01:80:c2:00:00:00").unwrap();
    assert!(is_multicast(mc));
    assert_eq!(classify(mc), MacKind::Multicast);
}

#[test]
fn entries_iterates_whole_table_in_order() {
    use oui_lookup::{entries, ENTRY_COUNT};
    let mut count = 0usize;
    let mut prev = 0u32;
    for e in entries() {
        assert!(e.prefix >= prev, "entries must be sorted by prefix");
        prev = e.prefix;
        count += 1;
    }
    assert_eq!(count, ENTRY_COUNT);
}

#[test]
fn lookup_octets_matches_string_lookup() {
    use oui_lookup::{lookup, lookup_octets};
    let octets = [0xA4, 0x83, 0xE7, 0x00, 0x00, 0x00];
    assert_eq!(lookup_octets(octets), lookup("a4:83:e7:00:00:00"));
}

#[test]
fn search_finds_known_vendor() {
    use oui_lookup::search;
    // Apple has many registered OUIs; at least one must match.
    let n = search("apple").count();
    assert!(n > 0, "expected at least one Apple OUI");
    // Case-insensitive.
    assert_eq!(search("APPLE").count(), n);
}

#[test]
fn search_empty_needle_matches_all() {
    use oui_lookup::{search, ENTRY_COUNT};
    assert_eq!(search("").count(), ENTRY_COUNT);
}

#[test]
fn normalize_mac_canonicalizes_spellings() {
    use oui_lookup::normalize_mac;
    for spelling in ["00-11-22-33-44-55", "0011.2233.4455", "001122334455"] {
        assert_eq!(normalize_mac(spelling).unwrap(), "00:11:22:33:44:55");
    }
    assert!(normalize_mac("00:11:22").is_err());
}

#[cfg(feature = "serde")]
#[test]
fn vendor_serializes_with_serde() {
    use oui_lookup::lookup_vendor;
    if let Some(v) = lookup_vendor("a4:83:e7:00:00:00") {
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("prefix"));
        assert!(json.contains("name"));
    }
}

#[test]
fn entry_display_contains_prefix_and_name() {
    use oui_lookup::entries;
    let e = entries().next().unwrap();
    let s = format!("{e}");
    assert!(s.contains(&e.prefix_str()));
    assert!(s.contains(e.name));
}

#[test]
fn lookup_entry_agrees_with_lookup() {
    use oui_lookup::{lookup, lookup_entry};
    match (
        lookup("a4:83:e7:00:00:00"),
        lookup_entry("a4:83:e7:00:00:00"),
    ) {
        (Some(name), Some(e)) => assert_eq!(name, e.name),
        (None, None) => {}
        _ => panic!("mismatch"),
    }
}
