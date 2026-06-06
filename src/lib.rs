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

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod db;
mod mac;

pub use mac::{format_oui, parse_oui, ParseMacError};

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
}
