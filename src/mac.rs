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

impl MacKind {
    /// `true` for the two unicast kinds (`LocalUnicast`, `GlobalUnicast`).
    pub fn is_unicast(self) -> bool {
        matches!(self, MacKind::LocalUnicast | MacKind::GlobalUnicast)
    }

    /// `true` only for a globally unique, IEEE-assigned unicast address.
    pub fn is_global(self) -> bool {
        matches!(self, MacKind::GlobalUnicast)
    }

    /// `true` for broadcast or multicast (group) addresses.
    pub fn is_group(self) -> bool {
        matches!(self, MacKind::Broadcast | MacKind::Multicast)
    }

    /// The kind as a static string slice, without allocating. This is the same
    /// text produced by the [`Display`](fmt::Display) impl.
    pub fn as_str(self) -> &'static str {
        match self {
            MacKind::Broadcast => "broadcast",
            MacKind::Multicast => "multicast",
            MacKind::LocalUnicast => "local-unicast",
            MacKind::GlobalUnicast => "global-unicast",
        }
    }
}

impl fmt::Display for MacKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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

/// Returns `true` if the address is a unicast address — the complement of
/// [`is_multicast`].
#[inline]
pub fn is_unicast(octets: [u8; 6]) -> bool {
    !is_multicast(octets)
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

/// Returns `true` if the address is the all-zero address `00:00:00:00:00:00`,
/// which often signals an unset or unknown hardware address.
#[inline]
pub fn is_zero(octets: [u8; 6]) -> bool {
    octets == [0x00; 6]
}

/// Returns `true` if the address falls in the IPv4-multicast Ethernet block
/// `01:00:5E:00:00:00`–`01:00:5E:7F:FF:FF` (RFC 1112 §6.4), the L2 mapping for
/// IPv4 multicast group addresses.
#[inline]
pub fn is_ipv4_multicast(octets: [u8; 6]) -> bool {
    octets[0] == 0x01 && octets[1] == 0x00 && octets[2] == 0x5E && octets[3] < 0x80
}

/// Returns `true` if the address falls in the IPv6-multicast Ethernet block
/// `33:33:00:00:00:00/24` (RFC 2464 §7), the L2 mapping for IPv6 multicast
/// group addresses.
#[inline]
pub fn is_ipv6_multicast(octets: [u8; 6]) -> bool {
    octets[0] == 0x33 && octets[1] == 0x33
}

/// Returns `true` for the VRRP virtual-router MAC range `00:00:5E:00:01:xx`
/// (RFC 5798 §7.3) — used by IPv4 VRRP and HSRP-like protocols.
#[inline]
pub fn is_vrrp(octets: [u8; 6]) -> bool {
    octets[0] == 0x00
        && octets[1] == 0x00
        && octets[2] == 0x5E
        && octets[3] == 0x00
        && octets[4] == 0x01
}

/// Returns `true` for the IEEE 802.3x **PAUSE** frame destination
/// `01:80:C2:00:00:01`.
#[inline]
pub fn is_pause(octets: [u8; 6]) -> bool {
    octets == [0x01, 0x80, 0xC2, 0x00, 0x00, 0x01]
}

/// Returns `true` for the IEEE 802.1D **STP BPDU** destination
/// `01:80:C2:00:00:00` — also used by RSTP/MSTP.
#[inline]
pub fn is_stp_bpdu(octets: [u8; 6]) -> bool {
    octets == [0x01, 0x80, 0xC2, 0x00, 0x00, 0x00]
}

/// Returns `true` for any address in the IEEE-reserved **bridge-protocol**
/// block `01:80:C2:00:00:00`–`01:80:C2:00:00:0F`, which carries STP, LACP,
/// LLDP, and other link-local control traffic.
#[inline]
pub fn is_bridge_protocol(octets: [u8; 6]) -> bool {
    octets[0] == 0x01
        && octets[1] == 0x80
        && octets[2] == 0xC2
        && octets[3] == 0x00
        && octets[4] == 0x00
        && octets[5] <= 0x0F
}

/// A finer-grained classification than [`MacKind`], picking out well-known
/// reserved or protocol-specific blocks.
///
/// Computed by [`scope`]. Plain unicast/multicast/broadcast addresses fall into
/// the [`MacScope::Generic`] catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacScope {
    /// The all-ones broadcast `FF:FF:FF:FF:FF:FF`.
    Broadcast,
    /// L2 mapping of an IPv4 multicast group (`01:00:5E:00:00:00/25`).
    Ipv4Multicast,
    /// L2 mapping of an IPv6 multicast group (`33:33::/24`).
    Ipv6Multicast,
    /// A VRRP/HSRP virtual-router MAC (`00:00:5E:00:01:xx`).
    Vrrp,
    /// An IEEE-reserved bridge-protocol address (`01:80:C2:00:00:00/4`).
    BridgeProtocol,
    /// Any other group/multicast address.
    OtherMulticast,
    /// A locally administered unicast address.
    LocalUnicast,
    /// A globally unique, IEEE-assigned unicast address.
    GlobalUnicast,
    /// The all-zero placeholder `00:00:00:00:00:00`.
    Zero,
    /// A unicast address that didn't fit any of the more specific buckets.
    Generic,
}

/// Classify an address into a [`MacScope`], picking the most specific bucket.
pub fn scope(octets: [u8; 6]) -> MacScope {
    if is_broadcast(octets) {
        MacScope::Broadcast
    } else if is_zero(octets) {
        MacScope::Zero
    } else if is_bridge_protocol(octets) {
        MacScope::BridgeProtocol
    } else if is_ipv4_multicast(octets) {
        MacScope::Ipv4Multicast
    } else if is_ipv6_multicast(octets) {
        MacScope::Ipv6Multicast
    } else if is_multicast(octets) {
        MacScope::OtherMulticast
    } else if is_vrrp(octets) {
        MacScope::Vrrp
    } else if is_locally_administered(octets) {
        MacScope::LocalUnicast
    } else {
        MacScope::GlobalUnicast
    }
}

impl MacScope {
    /// The scope name as a static string, without allocating.
    pub fn as_str(self) -> &'static str {
        match self {
            MacScope::Broadcast => "broadcast",
            MacScope::Ipv4Multicast => "ipv4-multicast",
            MacScope::Ipv6Multicast => "ipv6-multicast",
            MacScope::Vrrp => "vrrp",
            MacScope::BridgeProtocol => "bridge-protocol",
            MacScope::OtherMulticast => "multicast",
            MacScope::LocalUnicast => "local-unicast",
            MacScope::GlobalUnicast => "global-unicast",
            MacScope::Zero => "zero",
            MacScope::Generic => "generic",
        }
    }
}

impl fmt::Display for MacScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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

/// The IPv6 link-local address (`fe80::/64`) formed from a MAC address via its
/// Modified EUI-64 interface identifier (RFC 4291).
pub fn link_local_ipv6(octets: [u8; 6]) -> std::net::Ipv6Addr {
    let e = to_eui64(octets);
    std::net::Ipv6Addr::new(
        0xfe80,
        0,
        0,
        0,
        u16::from_be_bytes([e[0], e[1]]),
        u16::from_be_bytes([e[2], e[3]]),
        u16::from_be_bytes([e[4], e[5]]),
        u16::from_be_bytes([e[6], e[7]]),
    )
}

/// Recover the 48-bit MAC address from a Modified EUI-64 interface identifier,
/// the inverse of [`to_eui64`].
///
/// Returns `None` if the middle bytes are not the `FF:FE` marker inserted by
/// [`to_eui64`], i.e. the EUI-64 was not derived from a 48-bit MAC.
pub fn eui64_to_mac(eui: [u8; 8]) -> Option<[u8; 6]> {
    if eui[3] != 0xFF || eui[4] != 0xFE {
        return None;
    }
    Some([eui[0] ^ 0x02, eui[1], eui[2], eui[5], eui[6], eui[7]])
}

/// Format a full 48-bit MAC address as the canonical colon-separated string,
/// e.g. `00:11:22:33:44:55`.
pub fn format_mac48(octets: [u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Format a 48-bit MAC address with hyphen separators, e.g.
/// `00-11-22-33-44-55` (the IEEE/Windows convention).
pub fn format_mac48_hyphen(octets: [u8; 6]) -> String {
    format!(
        "{:02X}-{:02X}-{:02X}-{:02X}-{:02X}-{:02X}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Format a 48-bit MAC address in Cisco's dotted notation, e.g.
/// `0011.2233.4455`.
pub fn format_mac48_cisco(octets: [u8; 6]) -> String {
    format!(
        "{:02x}{:02x}.{:02x}{:02x}.{:02x}{:02x}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Format a 48-bit MAC address as a lower-case, colon-separated string, e.g.
/// `00:11:22:aa:bb:cc`.
pub fn format_mac48_lower(octets: [u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Format a 48-bit MAC address with no separators, e.g. `001122334455`.
///
/// Useful for compact log lines and identifiers that need to round-trip through
/// systems that strip punctuation. The output re-parses to the same octets via
/// [`parse_mac48`].
pub fn format_mac48_bare(octets: [u8; 6]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5]
    )
}

/// Extract the three bytes of a 24-bit OUI prefix, most-significant first.
///
/// Inverse of [`octets_to_oui`]; the high byte of `oui` is ignored.
#[inline]
pub const fn oui_to_octets(oui: u32) -> [u8; 3] {
    [(oui >> 16) as u8, (oui >> 8) as u8, oui as u8]
}

/// Pack three OUI bytes into the low 24 bits of a `u32`, most-significant
/// first. Inverse of [`oui_to_octets`].
#[inline]
pub const fn octets_to_oui(octets: [u8; 3]) -> u32 {
    ((octets[0] as u32) << 16) | ((octets[1] as u32) << 8) | (octets[2] as u32)
}

/// Format a 24-bit OUI prefix as a lower-case `aa:bb:cc` string.
pub fn format_oui_lower(oui: u32) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}",
        (oui >> 16) & 0xFF,
        (oui >> 8) & 0xFF,
        oui & 0xFF
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
    fn classifies_addresses() {
        assert_eq!(
            classify(parse_mac48("ff:ff:ff:ff:ff:ff").unwrap()),
            MacKind::Broadcast
        );
        assert_eq!(
            classify(parse_mac48("01:00:5e:00:00:01").unwrap()),
            MacKind::Multicast
        );
        assert_eq!(
            classify(parse_mac48("02:00:00:00:00:01").unwrap()),
            MacKind::LocalUnicast
        );
        assert_eq!(
            classify(parse_mac48("a4:83:e7:00:00:01").unwrap()),
            MacKind::GlobalUnicast
        );
    }

    #[test]
    fn detects_broadcast() {
        assert!(is_broadcast(parse_mac48("ff:ff:ff:ff:ff:ff").unwrap()));
        assert!(!is_broadcast(parse_mac48("ff:ff:ff:00:00:00").unwrap()));
    }
    #[test]
    fn unicast_is_complement_of_multicast() {
        let g = parse_mac48("a4:83:e7:00:00:01").unwrap();
        assert!(is_unicast(g) && !is_multicast(g));
        let m = parse_mac48("01:00:5e:00:00:01").unwrap();
        assert!(!is_unicast(m) && is_multicast(m));
    }

    #[test]
    fn mackind_predicates() {
        assert!(MacKind::GlobalUnicast.is_unicast());
        assert!(MacKind::GlobalUnicast.is_global());
        assert!(!MacKind::LocalUnicast.is_global());
        assert!(MacKind::LocalUnicast.is_unicast());
        assert!(MacKind::Broadcast.is_group());
        assert!(MacKind::Multicast.is_group());
        assert!(!MacKind::GlobalUnicast.is_group());
    }
    #[test]
    fn detects_zero_address() {
        assert!(is_zero(parse_mac48("00:00:00:00:00:00").unwrap()));
        assert!(!is_zero(parse_mac48("00:00:00:00:00:01").unwrap()));
    }
    #[test]
    fn eui64_round_trips_mac() {
        let mac = parse_mac48("00:11:22:33:44:55").unwrap();
        let eui = to_eui64(mac);
        assert_eq!(eui64_to_mac(eui), Some(mac));
        // Not a MAC-derived EUI-64 (no FF:FE marker).
        let mut bad = eui;
        bad[3] = 0x00;
        assert_eq!(eui64_to_mac(bad), None);
    }
    #[test]
    fn alternate_formatters() {
        let m = parse_mac48("00:11:22:33:44:55").unwrap();
        assert_eq!(format_mac48_hyphen(m), "00-11-22-33-44-55");
        assert_eq!(format_mac48_cisco(m), "0011.2233.4455");
        assert_eq!(format_mac48_bare(m), "001122334455");
        // All formats must re-parse to the same octets.
        for s in [
            format_mac48(m),
            format_mac48_hyphen(m),
            format_mac48_cisco(m),
            format_mac48_bare(m),
        ] {
            assert_eq!(parse_mac48(&s).unwrap(), m);
        }
    }
    #[test]
    fn lowercase_formatters() {
        let m = parse_mac48("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(format_mac48_lower(m), "aa:bb:cc:dd:ee:ff");
        assert_eq!(format_oui_lower(0xAABBCC), "aa:bb:cc");
    }
    #[test]
    fn mackind_as_str_matches_display() {
        for k in [
            MacKind::Broadcast,
            MacKind::Multicast,
            MacKind::LocalUnicast,
            MacKind::GlobalUnicast,
        ] {
            assert_eq!(k.as_str(), k.to_string());
        }
    }
    #[test]
    fn ipv4_multicast_mac_range() {
        // L2 mapping for 224.0.0.1.
        let m = parse_mac48("01:00:5e:00:00:01").unwrap();
        assert!(is_ipv4_multicast(m));
        // Just past the upper bound of the RFC 1112 block.
        let edge = parse_mac48("01:00:5e:80:00:00").unwrap();
        assert!(!is_ipv4_multicast(edge));
        // Unicast address with a similar prefix is not IPv4 multicast.
        let u = parse_mac48("a4:83:e7:00:00:01").unwrap();
        assert!(!is_ipv4_multicast(u));
    }

    #[test]
    fn ipv6_multicast_mac_range() {
        // L2 mapping for ff02::1.
        let m = parse_mac48("33:33:00:00:00:01").unwrap();
        assert!(is_ipv6_multicast(m));
        assert!(is_multicast(m));
        // Same-first-byte but not 33:33 — not IPv6 multicast.
        let m2 = parse_mac48("33:00:00:00:00:01").unwrap();
        assert!(!is_ipv6_multicast(m2));
    }

    #[test]
    fn vrrp_range() {
        let v = parse_mac48("00:00:5e:00:01:01").unwrap();
        assert!(is_vrrp(v));
        // Same prefix, different sub-byte: not VRRP.
        let other = parse_mac48("00:00:5e:00:02:01").unwrap();
        assert!(!is_vrrp(other));
    }

    #[test]
    fn oui_octet_conversions_are_inverse() {
        for oui in [0u32, 0x001122, 0xA483E7, 0xFFFFFF] {
            assert_eq!(octets_to_oui(oui_to_octets(oui)), oui);
        }
        // High byte is dropped.
        assert_eq!(oui_to_octets(0xFF_A483E7), oui_to_octets(0xA483E7));
    }

    #[test]
    fn link_local_from_mac() {
        let mac = parse_mac48("00:11:22:33:44:55").unwrap();
        let ll = link_local_ipv6(mac);
        // fe80::/64 prefix, EUI-64 interface id with flipped U/L bit + FF:FE.
        assert_eq!(
            ll,
            "fe80::211:22ff:fe33:4455"
                .parse::<std::net::Ipv6Addr>()
                .unwrap()
        );
    }
}
