//! oui-lookup: MAC-address vendor (OUI) lookup.

mod db;
mod mac;

pub use mac::{format_oui, parse_oui, ParseMacError};
