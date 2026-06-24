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

#[test]
fn count_matching_equals_search_count() {
    use oui_lookup::{count_matching, search};
    assert_eq!(count_matching("apple"), search("apple").count());
}

#[test]
fn search_results_are_sorted_by_prefix() {
    use oui_lookup::search;
    let mut prev = 0u32;
    for e in search("inc") {
        assert!(e.prefix >= prev);
        prev = e.prefix;
    }
}

#[test]
fn format_mac48_bare_round_trips_through_parse() {
    use oui_lookup::{format_mac48_bare, parse_mac48};
    let m = parse_mac48("00:11:22:33:44:55").unwrap();
    let bare = format_mac48_bare(m);
    assert_eq!(bare, "001122334455");
    assert_eq!(parse_mac48(&bare).unwrap(), m);
}

#[test]
fn oui_octet_helpers_are_inverse() {
    use oui_lookup::{octets_to_oui, oui_to_octets, parse_oui};
    let oui = parse_oui("a4:83:e7").unwrap();
    assert_eq!(octets_to_oui(oui_to_octets(oui)), oui);
}

#[test]
fn vendor_accessors_match_parse_oui() {
    use oui_lookup::{lookup_vendor, parse_oui};
    if let Some(v) = lookup_vendor("A4:83:E7") {
        assert_eq!(v.oui(), parse_oui("a4:83:e7").unwrap());
        assert_eq!(v.octets(), [0xA4, 0x83, 0xE7]);
    }
}

#[test]
fn vendor_with_suffix_resolves_back_to_same_vendor() {
    use oui_lookup::{lookup_octets, lookup_vendor};
    if let Some(v) = lookup_vendor("A4:83:E7") {
        let mac = v.with_suffix([0xDE, 0xAD, 0xBE]);
        // The synthesized MAC's OUI must resolve to the same vendor.
        assert_eq!(lookup_octets(mac), Some(v.name.as_str()));
    }
}

#[test]
fn with_oui_round_trips_through_split() {
    use oui_lookup::{split_mac48, with_oui_octets};
    let m = [0xA4, 0x83, 0xE7, 0x11, 0x22, 0x33];
    let (p, s) = split_mac48(m);
    assert_eq!(with_oui_octets(p, s), m);
}

#[test]
fn vendor_oui_returns_first_in_prefix_order() {
    use oui_lookup::{prefixes_for, vendor_oui};
    if let Some(oui) = vendor_oui("Apple, Inc.") {
        // The "first" prefix is the lowest one in numeric order — i.e. the
        // first entry the matching iterator yields.
        let first = prefixes_for("Apple, Inc.").next().unwrap();
        assert_eq!(oui, first.prefix);
    }
}

#[test]
fn vendor_oui_is_case_insensitive() {
    use oui_lookup::vendor_oui;
    let a = vendor_oui("Apple, Inc.");
    let b = vendor_oui("APPLE, INC.");
    assert_eq!(a, b);
}

#[test]
fn vendor_octets_matches_vendor_oui() {
    use oui_lookup::{vendor_octets, vendor_oui};
    if let (Some(oui), Some(octets)) = (vendor_oui("Apple, Inc."), vendor_octets("Apple, Inc.")) {
        assert_eq!(oui >> 16 & 0xFF, octets[0] as u32);
        assert_eq!(oui >> 8 & 0xFF, octets[1] as u32);
        assert_eq!(oui & 0xFF, octets[2] as u32);
    }
}

#[test]
fn vendor_oui_unknown_is_none() {
    use oui_lookup::vendor_oui;
    assert_eq!(vendor_oui("nope ltd no such vendor"), None);
}
