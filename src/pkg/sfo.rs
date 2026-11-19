//! param.sfo (little-endian key/value table), DESIGN 9.5.

use std::collections::HashMap;

pub const MAX_SFO: usize = 64 * 1024;
const MAX_KEYS: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(u32),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Int(_) => None,
        }
    }
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?) as usize)
}

/// Parses param.sfo. Returns None for anything malformed (never panics).
pub fn parse(b: &[u8]) -> Option<HashMap<String, Value>> {
    if b.len() > MAX_SFO || b.get(0..4)? != b"\0PSF" {
        return None;
    }
    let (keys, data, count) = (u32_at(b, 0x08)?, u32_at(b, 0x0C)?, u32_at(b, 0x10)?);
    if count > MAX_KEYS {
        return None;
    }
    let mut out = HashMap::new();
    for i in 0..count {
        let ix = 0x14 + i * 16;
        let ko = u16_at(b, ix)? as usize;
        let fmt = u16_at(b, ix + 2)?;
        let len = u32_at(b, ix + 4)?;
        let off = u32_at(b, ix + 12)?;
        let kstart = keys.checked_add(ko)?;
        let kslice = b.get(kstart..)?;
        let kend = kslice.iter().position(|&c| c == 0)?;
        let key = std::str::from_utf8(&kslice[..kend]).ok()?.to_string();
        let vstart = data.checked_add(off)?;
        let v = b.get(vstart..vstart.checked_add(len)?)?;
        let value = match fmt {
            0x0404 => Value::Int(u32::from_le_bytes(v.get(0..4)?.try_into().ok()?)),
            _ => Value::Str(
                String::from_utf8_lossy(v)
                    .trim_end_matches('\0')
                    .to_string(),
            ),
        };
        out.insert(key, value);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::fixture::sfo_bytes;

    #[test]
    fn roundtrip() {
        let b = sfo_bytes(&[("TITLE", "Igra"), ("CATEGORY", "gd")], &[("APP_TYPE", 1)]);
        let m = parse(&b).unwrap();
        assert_eq!(m["TITLE"].as_str(), Some("Igra"));
        assert_eq!(m["APP_TYPE"], Value::Int(1));
        for cut in 0..b.len() {
            let _ = parse(&b[..cut]);
        }
        assert!(parse(b"\0PSX").is_none());
    }
}
