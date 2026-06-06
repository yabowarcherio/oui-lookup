//! Parsing of MAC addresses and OUI prefixes.

fn is_separator(c: char) -> bool {
    matches!(c, ':' | '-' | '.') || c.is_whitespace()
}

pub fn parse_oui(input: &str) -> Option<u32> {
    let mut digits = 0u32;
    let mut oui = 0u32;
    for c in input.chars() {
        if is_separator(c) { continue; }
        let v = c.to_digit(16)?;
        if digits < 6 { oui = (oui << 4) | v; }
        digits += 1;
    }
    if digits < 6 { return None; }
    Some(oui)
}
