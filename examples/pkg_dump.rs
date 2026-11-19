//! Spike S-6 (PLAN Faza 0): read-only PS4 PKG header dump (DESIGN 9.5).
//! Prints header fields, entry table and param.sfo keys. Never prints file contents.
//! Run: cargo run --example pkg_dump -- <file.pkg> [more.pkg ...]

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}
fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
}

fn read_at(f: &mut File, off: u64, len: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    f.seek(SeekFrom::Start(off))?;
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn dump(path: &str) -> Result<(), String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let file_len = f.metadata().map_err(|e| e.to_string())?.len();
    let h = read_at(&mut f, 0, 0x438).map_err(|e| format!("header: {e}"))?;
    let magic = be32(&h, 0);
    println!(
        "magic        0x{magic:08X} {}",
        if magic == 0x7F43_4E54 {
            "ok"
        } else {
            "NOT A PS4 PKG"
        }
    );
    if magic != 0x7F43_4E54 {
        return Ok(());
    }
    let (file_count, entry_count, table) = (be32(&h, 0x0C), be32(&h, 0x10), be32(&h, 0x18));
    let content_id = String::from_utf8_lossy(&h[0x40..0x64])
        .trim_end_matches('\0')
        .to_string();
    let pkg_size = u64::from_be_bytes(h[0x430..0x438].try_into().unwrap());
    println!("file_count   {file_count}\nentry_count  {entry_count}\nentry_table  0x{table:X}");
    println!(
        "content_id   {content_id}\npkg_size     {pkg_size} (file on disk {file_len}{})",
        if pkg_size == file_len {
            ", equal"
        } else {
            ", DIFFERENT"
        }
    );
    if entry_count > 10_000 || u64::from(table) + u64::from(entry_count) * 32 > file_len {
        return Err("entry table out of bounds".into());
    }
    let t = read_at(&mut f, table.into(), entry_count as usize * 32).map_err(|e| e.to_string())?;
    let mut sfo = None;
    for i in 0..entry_count as usize {
        let e = &t[i * 32..i * 32 + 32];
        let (id, flags1, off, size) = (be32(e, 0), be32(e, 8), be32(e, 16), be32(e, 20));
        let enc = flags1 & 0x8000_0000 != 0;
        if id < 0x2000 {
            println!("entry 0x{id:04X} off=0x{off:X} size={size} encrypted={enc}");
        }
        if id == 0x1000 && !enc {
            sfo = Some((off, size));
        }
    }
    let Some((off, size)) = sfo else {
        println!("param.sfo    missing or encrypted (PKG-03)");
        return Ok(());
    };
    if size > 1 << 20 || u64::from(off) + u64::from(size) > file_len {
        return Err("param.sfo out of bounds".into());
    }
    let s = read_at(&mut f, off.into(), size as usize).map_err(|e| e.to_string())?;
    if &s[0..4] != b"\0PSF" {
        return Err("param.sfo bad magic".into());
    }
    let (keys, data, count) = (
        le32(&s, 8) as usize,
        le32(&s, 12) as usize,
        le32(&s, 16) as usize,
    );
    for i in 0..count.min(256) {
        let ix = 0x14 + i * 16;
        let (ko, fmt, len, doff) = (
            le16(&s, ix) as usize,
            le16(&s, ix + 2),
            le32(&s, ix + 4) as usize,
            le32(&s, ix + 12) as usize,
        );
        let kstart = keys + ko;
        let kend = s[kstart..]
            .iter()
            .position(|&b| b == 0)
            .map_or(s.len(), |p| kstart + p);
        let key = String::from_utf8_lossy(&s[kstart..kend]);
        let v = &s[data + doff..(data + doff + len).min(s.len())];
        let val = match fmt {
            0x0404 if v.len() >= 4 => format!("0x{:08X}", le32(v, 0)),
            _ => String::from_utf8_lossy(v)
                .trim_end_matches('\0')
                .to_string(),
        };
        if [
            "TITLE",
            "TITLE_ID",
            "CONTENT_ID",
            "CATEGORY",
            "APP_VER",
            "VERSION",
            "APP_TYPE",
        ]
        .contains(&key.as_ref())
        {
            println!("sfo {key:<10} = {val}");
        }
    }
    Ok(())
}

fn main() {
    for p in std::env::args().skip(1) {
        println!(
            "== {}",
            std::path::Path::new(&p)
                .file_name()
                .map_or(p.clone(), |n| n.to_string_lossy().into())
        );
        if let Err(e) = dump(&p) {
            println!("ERROR: {e}");
        }
    }
}
