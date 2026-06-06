//! Runtime access to the embedded, pre-packed OUI table.

static BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/oui.bin"));

const MAGIC: &[u8; 4] = b"OUI1";
const HEADER_LEN: usize = 8;
const ENTRY_LEN: usize = 8;

pub fn lookup_prefix(_oui: u32) -> Option<&'static str> {
    let _ = (BLOB, MAGIC, HEADER_LEN, ENTRY_LEN);
    None
}
