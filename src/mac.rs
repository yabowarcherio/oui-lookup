//! Parsing and normalization of MAC addresses and OUI prefixes.

use core::fmt;

/// An error produced while parsing a MAC address or OUI prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseMacError {
    /// The input did not contain enough hex digits to form a 24-bit OUI.
    TooShort,
    /// The input contained a character that is neither a hex digit nor an
    /// accepted separator (`:`, `-`, `.`, or whitespace).
    InvalidChar(char),
    /// More than 12 hex digits were supplied (longer than a 48-bit address).
    TooLong,
}

impl fmt::Display for ParseMacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseMacError::TooShort => {
                f.write_str("not enough hex digits for a 24-bit OUI (need at least 6)")
            }
            ParseMacError::TooLong => f.write_str("too many hex digits (max 12 for a 48-bit MAC)"),
            ParseMacError::InvalidChar(c) => write!(f, "invalid character in MAC address: {c:?}"),
        }
    }
}

impl std::error::Error for ParseMacError {}

#[inline]
fn hex_val(c: char) -> Option<u32> {
    c.to_digit(16)
}

#[inline]
fn is_separator(c: char) -> bool {
    matches!(c, ':' | '-' | '.') || c.is_whitespace()
}

/// Extract the 24-bit OUI prefix from any reasonable textual MAC representation.
///
/// Accepts the common formats, mixing case and separators freely:
///
/// - `00:11:22:33:44:55`
/// - `00-11-22-33-44-55`
/// - `0011.2233.4455` (Cisco)
/// - `001122334455`
/// - `00:11:22` (just the OUI)
///
/// Only the first three octets (six hex digits) are significant; any remaining
/// digits are validated but ignored. The returned value is the OUI packed into
/// the low 24 bits of a `u32` (e.g. `0x001122`).
///
/// # Errors
///
/// Returns [`ParseMacError`] if the input has fewer than six hex digits, more
/// than twelve, or contains an unexpected character.
pub fn parse_oui(input: &str) -> Result<u32, ParseMacError> {
    let mut digits = 0u32;
    let mut oui = 0u32;

    for c in input.chars() {
        if is_separator(c) {
            continue;
        }
        let v = hex_val(c).ok_or(ParseMacError::InvalidChar(c))?;
        if digits < 6 {
            oui = (oui << 4) | v;
        }
        digits += 1;
        if digits > 12 {
            return Err(ParseMacError::TooLong);
        }
    }

    if digits < 6 {
        return Err(ParseMacError::TooShort);
    }
    Ok(oui)
}

/// A coarse classification of a MAC address based on the two low bits of its
/// first octet and the all-ones broadcast special case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacKind {
    /// The broadcast address `FF:FF:FF:FF:FF:FF`.
    Broadcast,
    /// A group/multicast address (LSB of the first octet set).
    Multicast,
    /// A locally administered unicast address (U/L bit set).
    LocalUnicast,
    /// A globally unique, IEEE-assigned unicast address.
    GlobalUnicast,
}

/// Classify a 48-bit MAC address into a [`MacKind`].
pub fn classify(octets: [u8; 6]) -> MacKind {
    if is_broadcast(octets) {
        MacKind::Broadcast
    } else if is_multicast(octets) {
        MacKind::Multicast
    } else if is_locally_administered(octets) {
        MacKind::LocalUnicast
    } else {
        MacKind::GlobalUnicast
    }
}

/// Returns `true` if the address is a multicast address (least-significant bit
/// of the first octet set). Unicast addresses have this bit clear.
#[inline]
pub fn is_multicast(octets: [u8; 6]) -> bool {
    octets[0] & 0x01 != 0
}

/// Returns `true` if the address is locally administered (the second-least-
/// significant bit of the first octet set), as opposed to a globally unique,
/// IEEE-assigned address.
#[inline]
pub fn is_locally_administered(octets: [u8; 6]) -> bool {
    octets[0] & 0x02 != 0
}

/// Returns `true` if the address is the broadcast address `FF:FF:FF:FF:FF:FF`.
#[inline]
pub fn is_broadcast(octets: [u8; 6]) -> bool {
    octets == [0xFF; 6]
}

/// Parse a full 48-bit MAC address into its six octets.
///
/// Unlike [`parse_oui`], this requires all six octets (twelve hex digits) to be
/// present and returns the complete address. Separators are handled the same
/// way as in [`parse_oui`].
///
/// # Errors
///
/// Returns [`ParseMacError::TooShort`] if fewer than twelve hex digits are
/// supplied, [`ParseMacError::TooLong`] if more, or
/// [`ParseMacError::InvalidChar`] for any non-hex, non-separator character.
pub fn parse_mac48(input: &str) -> Result<[u8; 6], ParseMacError> {
    let mut octets = [0u8; 6];
    let mut digits = 0usize;

    for c in input.chars() {
        if is_separator(c) {
            continue;
        }
        let v = hex_val(c).ok_or(ParseMacError::InvalidChar(c))? as u8;
        if digits >= 12 {
            return Err(ParseMacError::TooLong);
        }
        let idx = digits / 2;
        octets[idx] = (octets[idx] << 4) | v;
        digits += 1;
    }

    if digits < 12 {
        return Err(ParseMacError::TooShort);
    }
    Ok(octets)
}

/// Convert a 48-bit MAC address to its Modified EUI-64 interface identifier,
/// as used when forming IPv6 link-local addresses (RFC 4291). The `FF:FE` is
/// inserted in the middle and the universal/local bit is flipped.
pub fn to_eui64(octets: [u8; 6]) -> [u8; 8] {
    let mut eui = [0u8; 8];
    eui[0] = octets[0] ^ 0x02;
    eui[1] = octets[1];
    eui[2] = octets[2];
    eui[3] = 0xFF;
    eui[4] = 0xFE;
    eui[5] = octets[3];
    eui[6] = octets[4];
    eui[7] = octets[5];
    eui
}

/// Format a full 48-bit MAC address as the canonical colon-separated string,
/// e.g. `00:11:22:33:44:55`.
pub fn format_mac48(octets: [u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Format a 24-bit OUI prefix as the canonical `AA:BB:CC` string.
pub fn format_oui(oui: u32) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}",
        (oui >> 16) & 0xFF,
        (oui >> 8) & 0xFF,
        oui & 0xFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_common_formats() {
        let cases = [
            "00:11:22:33:44:55",
            "00-11-22-33-44-55",
            "0011.2233.4455",
            "001122334455",
            "00 11 22 33 44 55",
            "00:11:22",
            "001122",
        ];
        for c in cases {
            assert_eq!(parse_oui(c).unwrap(), 0x001122, "input: {c}");
        }
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(parse_oui("aa:bB:Cc").unwrap(), 0xAABBCC);
    }

    #[test]
    fn rejects_short_input() {
        assert_eq!(parse_oui("00:11").unwrap_err(), ParseMacError::TooShort);
        assert_eq!(parse_oui("").unwrap_err(), ParseMacError::TooShort);
    }

    #[test]
    fn rejects_too_long() {
        assert_eq!(
            parse_oui("00112233445566").unwrap_err(),
            ParseMacError::TooLong
        );
    }

    #[test]
    fn rejects_invalid_char() {
        assert_eq!(
            parse_oui("00:11:2g").unwrap_err(),
            ParseMacError::InvalidChar('g')
        );
    }

    #[test]
    fn ignores_octets_past_the_oui() {
        // The 4th-6th octets must not change the parsed OUI.
        assert_eq!(parse_oui("00:11:22:ff:ee:dd").unwrap(), 0x001122);
    }

    #[test]
    fn formats_canonically() {
        assert_eq!(format_oui(0x001122), "00:11:22");
        assert_eq!(format_oui(0xAABBCC), "AA:BB:CC");
    }

    #[test]
    fn parses_full_mac48() {
        assert_eq!(
            parse_mac48("00:11:22:33:44:55").unwrap(),
            [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
        );
        assert_eq!(
            parse_mac48("aabbccddeeff").unwrap(),
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        );
    }

    #[test]
    fn mac48_requires_all_six_octets() {
        assert_eq!(
            parse_mac48("00:11:22").unwrap_err(),
            ParseMacError::TooShort
        );
    }

    #[test]
    fn detects_multicast_and_local_bits() {
        // 01:00:5e:... is the IPv4 multicast range — LSB of first octet set.
        let mc = parse_mac48("01:00:5e:00:00:01").unwrap();
        assert!(is_multicast(mc));
        assert!(!is_locally_administered(mc));

        // 02:... has the locally-administered bit set, unicast.
        let local = parse_mac48("02:00:00:00:00:01").unwrap();
        assert!(is_locally_administered(local));
        assert!(!is_multicast(local));

        // A normal IEEE-assigned unicast address: both bits clear.
        let global = parse_mac48("a4:83:e7:00:00:01").unwrap();
        assert!(!is_multicast(global));
        assert!(!is_locally_administered(global));
    }

    #[test]
    fn converts_to_eui64() {
        // RFC 4291 example-style: flip the U/L bit, insert FF:FE.
        let mac = parse_mac48("00:11:22:33:44:55").unwrap();
        assert_eq!(
            to_eui64(mac),
            [0x02, 0x11, 0x22, 0xFF, 0xFE, 0x33, 0x44, 0x55]
        );
    }

    #[test]
    fn detects_broadcast() {
        assert!(is_broadcast(parse_mac48("ff:ff:ff:ff:ff:ff").unwrap()));
        assert!(!is_broadcast(parse_mac48("ff:ff:ff:00:00:00").unwrap()));
    }
}
