//! Parsing of MAC addresses and OUI prefixes.

pub fn parse_oui(input: &str) -> Option<u32> {
    let mut digits = 0u32;
    let mut oui = 0u32;
    for c in input.chars() {
        if c == ':' || c == '-' { continue; }
        let v = c.to_digit(16)?;
        if digits < 6 { oui = (oui << 4) | v; }
        digits += 1;
    }
    if digits < 6 { return None; }
    Some(oui)
}
