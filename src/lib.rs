//! # oui-lookup
//!
//! Fast, **offline** MAC-address vendor (OUI) lookup.
//!
//! The full IEEE MA-L registry is compiled directly into the binary, so there
//! are no network calls, no cache files, and no runtime dependencies beyond a
//! one-time in-memory decompression. Lookups are an allocation-free binary
//! search over data borrowed straight from the embedded blob.
//!
//! ```
//! use oui_lookup::lookup;
//!
//! // Any common MAC format works; only the first three octets matter.
//! let vendor = lookup("00:11:22:33:44:55");
//! // `vendor` is `Some(name)` if the OUI is registered, else `None`.
//! # let _ = vendor;
//! ```
//!
//! ## What counts as a match
//!
//! Lookups are by the 24-bit Organizationally Unique Identifier (OUI) — the
//! first three octets. The crate embeds the **MA-L** registry (the classic
//! `/24` block assignments). Finer-grained MA-M (`/28`) and MA-S (`/36`)
//! sub-allocations are not yet resolved; an address inside one of those blocks
//! resolves to the owner of the enclosing `/24`, which is usually the IEEE
//! registration authority rather than the end vendor.
//!
//! ## Features
//!
//! - `cli` *(default)* — pulls in the dependencies for the `oui-lookup` binary.
//!   Disable it (`default-features = false`) for a slim library dependency.
//! - `serde` — derives [`serde::Serialize`]/[`serde::Deserialize`] on
//!   [`Vendor`].

//!
//! ## Searching
//!
//! ```
//! use oui_lookup::search;
//! // Every match is an `Entry`; the iterator is lazy.
//! let _apple_ouis: Vec<_> = search("apple").collect();
//! ```
//!
//! ## Address utilities
//!
//! ```
//! use oui_lookup::{parse_mac48, classify, MacKind};
//! let mac = parse_mac48("ff:ff:ff:ff:ff:ff").unwrap();
//! assert_eq!(classify(mac), MacKind::Broadcast);
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod db;
mod mac;

pub use mac::{
    classify, eui64_to_mac, format_mac48, format_mac48_bare, format_mac48_cisco,
    format_mac48_hyphen, format_mac48_lower, format_oui, format_oui_lower, is_broadcast,
    is_locally_administered, is_multicast, is_unicast, is_zero, link_local_ipv6, parse_mac48,
    parse_oui, to_eui64, MacKind, ParseMacError,
};

/// The number of OUI prefixes embedded in this build of the crate.
///
/// Reflects the snapshot of the IEEE registry that was vendored when the crate
/// was built.
pub const ENTRY_COUNT: usize = db::ENTRY_COUNT;

/// A resolved vendor for an OUI prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vendor {
    /// The 24-bit OUI prefix that matched, formatted as `AA:BB:CC`.
    pub prefix: String,
    /// The registered organization name.
    pub name: String,
}

impl Vendor {
    /// Parse the canonical `AA:BB:CC` prefix back to its three bytes.
    ///
    /// Always succeeds for values produced by this crate; the parsing logic is
    /// only exposed in case the [`Vendor`] was deserialized from an untrusted
    /// source — a malformed `prefix` returns `[0, 0, 0]`.
    pub fn octets(&self) -> [u8; 3] {
        let oui = mac::parse_oui(&self.prefix).unwrap_or(0);
        [
            ((oui >> 16) & 0xFF) as u8,
            ((oui >> 8) & 0xFF) as u8,
            (oui & 0xFF) as u8,
        ]
    }

    /// The 24-bit OUI prefix as a raw integer, in the low bits of the returned
    /// `u32`.
    pub fn oui(&self) -> u32 {
        mac::parse_oui(&self.prefix).unwrap_or(0)
    }
}

/// Look up the vendor name for a MAC address or OUI prefix.
///
/// Accepts any of the usual textual MAC formats (`:`, `-`, `.`, or no
/// separators; upper or lower case). Only the first three octets are
/// significant. Returns the registered vendor name as a `'static` slice
/// borrowed from the embedded table, or `None` if the input parses but the OUI
/// is unregistered.
///
/// Inputs that cannot be parsed as a MAC/OUI (too short, bad characters) also
/// return `None`. Use [`try_lookup`] when you need to distinguish "unparseable"
/// from "parsed but unknown".
///
/// ```
/// # use oui_lookup::lookup;
/// assert!(lookup("not a mac").is_none());
/// ```
#[inline]
pub fn lookup(mac: &str) -> Option<&'static str> {
    let oui = mac::parse_oui(mac).ok()?;
    db::lookup_prefix(oui)
}

/// Like [`lookup`], but reports parse errors instead of collapsing them to
/// `None`.
///
/// - `Ok(Some(name))` — the input parsed and the OUI is registered.
/// - `Ok(None)` — the input parsed but the OUI is not in the registry.
/// - `Err(_)` — the input is not a valid MAC address or OUI prefix.
#[inline]
pub fn try_lookup(mac: &str) -> Result<Option<&'static str>, ParseMacError> {
    let oui = mac::parse_oui(mac)?;
    Ok(db::lookup_prefix(oui))
}

/// Look up a MAC address and return an owned [`Vendor`] (prefix + name).
///
/// Convenience over [`lookup`] for callers that want both the canonical prefix
/// and the name as owned strings — e.g. for serialization. Returns `None` if
/// the input is unparseable or the OUI is unregistered.
pub fn lookup_vendor(mac: &str) -> Option<Vendor> {
    let oui = mac::parse_oui(mac).ok()?;
    let name = db::lookup_prefix(oui)?;
    Some(Vendor {
        prefix: mac::format_oui(oui),
        name: name.to_string(),
    })
}

/// Normalize any accepted MAC spelling to the canonical upper-case,
/// colon-separated 48-bit form (`AA:BB:CC:DD:EE:FF`).
///
/// # Errors
///
/// Returns the same [`ParseMacError`] as [`parse_mac48`] if the input is not a
/// full, valid 48-bit address.
pub fn normalize_mac(mac: &str) -> Result<String, ParseMacError> {
    Ok(mac::format_mac48(mac::parse_mac48(mac)?))
}

/// Like [`normalize_mac`], but produces the lower-case canonical form
/// (`aa:bb:cc:dd:ee:ff`).
///
/// # Errors
///
/// Returns the same [`ParseMacError`] as [`parse_mac48`] if the input is not a
/// full, valid 48-bit address.
pub fn normalize_mac_lower(mac: &str) -> Result<String, ParseMacError> {
    Ok(mac::format_mac48_lower(mac::parse_mac48(mac)?))
}

/// Look up the vendor for a MAC address already parsed into octets.
///
/// Useful when you have the raw bytes (e.g. from an ARP table) and want to
/// avoid re-parsing a string.
#[inline]
pub fn lookup_octets(octets: [u8; 6]) -> Option<&'static str> {
    let oui = (u32::from(octets[0]) << 16) | (u32::from(octets[1]) << 8) | u32::from(octets[2]);
    db::lookup_prefix(oui)
}

/// Look up a pre-parsed address and return the matching [`Entry`].
///
/// The octet equivalent of [`lookup_entry`]; returns `None` if the OUI is
/// unregistered.
pub fn lookup_entry_octets(octets: [u8; 6]) -> Option<Entry> {
    let oui = (u32::from(octets[0]) << 16) | (u32::from(octets[1]) << 8) | u32::from(octets[2]);
    db::lookup_prefix(oui).map(|name| Entry { prefix: oui, name })
}

/// Look up a pre-parsed address and return an owned [`Vendor`].
///
/// The octet equivalent of [`lookup_vendor`].
pub fn lookup_vendor_octets(octets: [u8; 6]) -> Option<Vendor> {
    let oui = (u32::from(octets[0]) << 16) | (u32::from(octets[1]) << 8) | u32::from(octets[2]);
    db::lookup_prefix(oui).map(|name| Vendor {
        prefix: mac::format_oui(oui),
        name: name.to_string(),
    })
}

/// Look up the vendor for a raw 24-bit OUI prefix (only the low 24 bits are
/// used).
///
/// The most direct entry point when you already hold the OUI as an integer —
/// e.g. the value returned by [`parse_oui`].
#[inline]
pub fn lookup_oui(oui: u32) -> Option<&'static str> {
    db::lookup_prefix(oui & 0x00FF_FFFF)
}

/// Return every distinct vendor name in the embedded registry, sorted
/// alphabetically.
///
/// A single organization usually holds several OUI prefixes, so this collapses
/// the ~`ENTRY_COUNT` entries down to the set of unique names. This allocates
/// and sorts, so it is a cold helper — cache the result if you need it
/// repeatedly.
pub fn vendors() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = entries().map(|e| e.name).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Look up a MAC address and return the matching [`Entry`] (prefix + name),
/// or `None` if unparseable or unregistered.
pub fn lookup_entry(mac: &str) -> Option<Entry> {
    let oui = mac::parse_oui(mac).ok()?;
    let name = db::lookup_prefix(oui)?;
    Some(Entry { prefix: oui, name })
}

/// Returns `true` if the OUI of the given MAC address is present in the
/// embedded registry. Equivalent to `lookup(mac).is_some()`.
#[inline]
pub fn is_registered(mac: &str) -> bool {
    lookup(mac).is_some()
}

/// Look up many MAC addresses at once, returning a vector of results in the
/// same order as the input. Each element is `Some(name)` for a registered OUI
/// or `None` if the input is unparseable or unregistered.
pub fn lookup_many<I, S>(macs: I) -> Vec<Option<&'static str>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    macs.into_iter().map(|m| lookup(m.as_ref())).collect()
}

/// Look up many pre-parsed addresses at once, in input order. The octet
/// equivalent of [`lookup_many`].
pub fn lookup_octets_many<I>(macs: I) -> Vec<Option<&'static str>>
where
    I: IntoIterator<Item = [u8; 6]>,
{
    macs.into_iter().map(lookup_octets).collect()
}

/// Look up many MAC addresses at once, returning an owned [`Vendor`] per input.
///
/// Like [`lookup_many`], but each element carries both the canonical prefix and
/// the name as owned strings — convenient for serialization or when the results
/// must outlive the borrow. Order matches the input; an element is `None` if the
/// input is unparseable or its OUI is unregistered.
pub fn lookup_vendor_many<I, S>(macs: I) -> Vec<Option<Vendor>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    macs.into_iter()
        .map(|m| lookup_vendor(m.as_ref()))
        .collect()
}

/// A single entry from the embedded OUI table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The 24-bit OUI prefix, in the low bits of the `u32`.
    pub prefix: u32,
    /// The registered organization name.
    pub name: &'static str,
}

impl Entry {
    /// The OUI prefix formatted as the canonical `AA:BB:CC` string.
    pub fn prefix_str(&self) -> String {
        mac::format_oui(self.prefix)
    }

    /// The OUI prefix as its three bytes, most-significant first.
    pub fn octets(&self) -> [u8; 3] {
        [
            (self.prefix >> 16) as u8,
            (self.prefix >> 8) as u8,
            self.prefix as u8,
        ]
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}  {}", self.prefix_str(), self.name)
    }
}

/// Find all entries whose vendor name contains `needle`, case-insensitively.
///
/// Returns a lazy iterator over matching [`Entry`] values in prefix order.
/// Note this is a linear scan over the whole table — fine for interactive use,
/// but call it sparingly in hot loops.
pub fn search(needle: &str) -> impl Iterator<Item = Entry> + '_ {
    let needle = needle.to_ascii_lowercase();
    entries().filter(move |e| e.name.to_ascii_lowercase().contains(&needle))
}

/// Find every OUI prefix registered to a vendor whose name matches `name`
/// **exactly**, case-insensitively.
///
/// Unlike [`search`] (a substring match), this returns only entries whose full
/// name equals `name` — useful for enumerating all the blocks a single
/// organization holds.
pub fn prefixes_for(name: &str) -> impl Iterator<Item = Entry> + '_ {
    let name = name.to_ascii_lowercase();
    entries().filter(move |e| e.name.to_ascii_lowercase() == name)
}

/// Count how many OUI prefixes are registered to vendors matching `needle`.
///
/// Convenience wrapper over [`search`] for the common "how many?" question.
pub fn count_matching(needle: &str) -> usize {
    search(needle).count()
}

/// Iterate over every entry in the embedded registry, in ascending prefix
/// order.
pub fn entries() -> impl Iterator<Item = Entry> {
    (0..db::len()).filter_map(|i| db::entry(i).map(|(prefix, name)| Entry { prefix, name }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unparseable_input_is_none_not_panic() {
        assert_eq!(lookup(""), None);
        assert_eq!(lookup("zzz"), None);
    }

    #[test]
    fn try_lookup_distinguishes_errors() {
        assert!(try_lookup("garbage!").is_err());
        // FF:FF:FF parses fine but is not a registered OUI.
        assert_eq!(try_lookup("FF:FF:FF:00:00:00"), Ok(None));
    }

    #[test]
    fn lookup_vendor_round_trips_prefix() {
        if let Some(v) = lookup_vendor("FF:FF:FF") {
            assert_eq!(v.prefix, "FF:FF:FF");
            let _ = v.name;
        }
    }

    #[test]
    fn entry_count_exposed() {
        assert_eq!(ENTRY_COUNT, db::ENTRY_COUNT);
    }

    #[test]
    fn lookup_vendor_many_preserves_order_and_gaps() {
        let out = lookup_vendor_many(["FF:FF:FF:00:00:00", "garbage", "FF:FF:FF"]);
        assert_eq!(out.len(), 3);
        // Unregistered and unparseable inputs both collapse to None.
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        // A registered OUI (if present) round-trips its prefix.
        if let Some(v) = &out[2] {
            assert_eq!(v.prefix, "FF:FF:FF");
        }
    }

    #[test]
    fn lookup_vendor_many_matches_singular() {
        let macs = ["00:11:22:33:44:55", "a4:83:e7:00:00:00", "zz"];
        let many = lookup_vendor_many(macs);
        for (m, got) in macs.iter().zip(many) {
            assert_eq!(lookup_vendor(m), got);
        }
    }
    #[test]
    fn lookup_oui_matches_string_lookup() {
        // Whatever the string form resolves to, the integer form must agree.
        let oui = parse_oui("a4:83:e7:00:00:00").unwrap();
        assert_eq!(lookup_oui(oui), lookup("a4:83:e7:00:00:00"));
        // High bits above 24 are ignored.
        assert_eq!(lookup_oui(oui | 0xFF00_0000), lookup_oui(oui));
    }
    #[test]
    fn vendors_are_unique_and_sorted() {
        let v = vendors();
        assert!(!v.is_empty());
        assert!(v.len() <= ENTRY_COUNT);
        // Sorted and deduplicated.
        assert!(v.windows(2).all(|w| w[0] < w[1]));
    }
    #[test]
    fn vendor_octets_round_trip() {
        let v = Vendor {
            prefix: "A4:83:E7".to_string(),
            name: "Apple".to_string(),
        };
        assert_eq!(v.octets(), [0xA4, 0x83, 0xE7]);
        assert_eq!(v.oui(), 0xA483E7);
        // Malformed prefix collapses to zero rather than panicking.
        let bad = Vendor {
            prefix: "not a prefix".to_string(),
            name: String::new(),
        };
        assert_eq!(bad.octets(), [0, 0, 0]);
        assert_eq!(bad.oui(), 0);
    }

    #[test]
    fn entry_octets_match_prefix() {
        let e = Entry {
            prefix: 0xA483E7,
            name: "Apple",
        };
        assert_eq!(e.octets(), [0xA4, 0x83, 0xE7]);
        assert_eq!(e.prefix_str(), "A4:83:E7");
    }
    #[test]
    fn octet_lookups_agree_with_string_lookups() {
        let octets = parse_mac48("a4:83:e7:11:22:33").unwrap();
        assert_eq!(
            lookup_entry_octets(octets).map(|e| e.name),
            lookup("a4:83:e7:11:22:33")
        );
        assert_eq!(
            lookup_vendor_octets(octets),
            lookup_vendor("a4:83:e7:11:22:33")
        );
        // Unregistered prefix.
        let ff = parse_mac48("ff:ff:ff:00:00:00").unwrap();
        assert!(lookup_entry_octets(ff).is_none());
    }
    #[test]
    fn prefixes_for_is_exact_match() {
        // Pick a real vendor name from the table and ensure all returned
        // entries carry exactly that name.
        if let Some(first) = entries().next() {
            let name = first.name;
            let all: Vec<_> = prefixes_for(name).collect();
            assert!(!all.is_empty());
            assert!(all.iter().all(|e| e.name.eq_ignore_ascii_case(name)));
            // Exact match excludes pure-substring hits.
            assert!(prefixes_for("definitely not a real vendor xyz")
                .next()
                .is_none());
        }
    }
    #[test]
    fn normalize_lower_round_trips() {
        assert_eq!(
            normalize_mac_lower("AA-BB-CC-DD-EE-FF").unwrap(),
            "aa:bb:cc:dd:ee:ff"
        );
        assert_eq!(
            normalize_mac("aa:bb:cc:dd:ee:ff").unwrap(),
            "AA:BB:CC:DD:EE:FF"
        );
        assert!(normalize_mac_lower("aa:bb:cc").is_err());
    }
    #[test]
    fn lookup_octets_many_matches_singular() {
        let macs = [
            parse_mac48("a4:83:e7:00:00:00").unwrap(),
            parse_mac48("ff:ff:ff:00:00:00").unwrap(),
        ];
        let many = lookup_octets_many(macs);
        assert_eq!(many.len(), 2);
        assert_eq!(many[0], lookup_octets(macs[0]));
        assert_eq!(many[1], None);
    }
}
