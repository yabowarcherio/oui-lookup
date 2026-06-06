//! oui-lookup: MAC-address vendor (OUI) lookup.

mod db;
mod mac;

pub use mac::{format_oui, parse_oui, ParseMacError};

pub fn lookup(mac: &str) -> Option<&'static str> {
    let oui = mac::parse_oui(mac).ok()?;
    db::lookup_prefix(oui)
}

pub fn try_lookup(mac: &str) -> Result<Option<&'static str>, ParseMacError> {
    let oui = mac::parse_oui(mac)?;
    Ok(db::lookup_prefix(oui))
}
