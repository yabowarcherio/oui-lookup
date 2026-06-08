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
    classify, format_mac48, format_oui, is_broadcast, is_locally_administered, is_multicast,
    parse_mac48, parse_oui, to_eui64, MacKind, ParseMacError,
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

/// Look up the vendor for a MAC address already parsed into octets.
///
/// Useful when you have the raw bytes (e.g. from an ARP table) and want to
/// avoid re-parsing a string.
#[inline]
pub fn lookup_octets(octets: [u8; 6]) -> Option<&'static str> {
    let oui = (u32::from(octets[0]) << 16) | (u32::from(octets[1]) << 8) | u32::from(octets[2]);
    db::lookup_prefix(oui)
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
}
