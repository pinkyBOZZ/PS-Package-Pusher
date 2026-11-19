//! PS4 PKG header + entry table (big-endian), DESIGN 9.5 / PLAN 3.

use super::sfo;
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

pub const MAGIC: u32 = 0x7F43_4E54;
pub const ID_SFO: u32 = 0x1000;
pub const ID_ICON0: u32 = 0x1200;
const HEADER_LEN: usize = 0x438;
const MAX_ENTRIES: u32 = 10_000;
pub const MAX_ICON: usize = 4 * 1024 * 1024;
pub const MAX_ICON_DIM: u32 = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Game,
    Update,
    Dlc,
    /// Raw CATEGORY value (e.g. `gde`, themes) or `?` when param.sfo is missing.
    Other(String),
}

impl Kind {
    pub fn from_category(c: &str) -> Kind {
        match c {
            "gd" => Kind::Game,
            "gp" => Kind::Update,
            "ac" => Kind::Dlc,
            other => Kind::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PkgInfo {
    /// From the header (wins over param.sfo on mismatch).
    pub content_id: String,
    pub title_id: String,
    pub title: Option<String>,
    pub category: Option<String>,
    pub app_ver: Option<String>,
    pub version: Option<String>,
    /// u64 @0x430: total package size.
    pub pkg_size: u64,
    pub icon: Option<Icon>,
    /// PKG-03: param.sfo missing or encrypted; Title ID came from the Content ID.
    pub sfo_missing: bool,
    pub content_id_mismatch: bool,
}

impl PkgInfo {
    pub fn kind(&self) -> Kind {
        match &self.category {
            Some(c) => Kind::from_category(c),
            None => Kind::Other("?".into()),
        }
    }

    /// `APP_VER` for games/updates, `VERSION` for DLC and as fallback (DESIGN 9.5, 14.2/18).
    pub fn display_version(&self) -> Option<&str> {
        let prefer_app = matches!(self.kind(), Kind::Game | Kind::Update);
        let (a, v) = (self.app_ver.as_deref(), self.version.as_deref());
        if prefer_app { a.or(v) } else { v.or(a) }
    }
}

#[derive(Debug)]
pub enum PkgError {
    /// PKG-01
    NotPkg,
    /// PKG-02: offsets/lengths outside the file, truncated
    Damaged(&'static str),
    /// PKG-03 without a usable Title ID in the Content ID → item is Invalid
    NoSfo,
    Io(io::Error),
}

impl From<io::Error> for PkgError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            PkgError::Damaged("truncated")
        } else {
            PkgError::Io(e)
        }
    }
}

pub fn read_info(path: &Path) -> Result<PkgInfo, PkgError> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    read_info_from(&mut f, len)
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn read_at<R: Read + Seek>(r: &mut R, len: u64, off: u64, n: usize) -> Result<Vec<u8>, PkgError> {
    if off.checked_add(n as u64).is_none_or(|end| end > len) {
        return Err(PkgError::Damaged("offset outside file"));
    }
    let mut buf = vec![0; n];
    r.seek(SeekFrom::Start(off))?;
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// Title ID inside a Content ID (`XXYYYY-CUSA12345_00-...` → `CUSA12345`) if well-formed.
pub fn title_id_from_content_id(cid: &str) -> Option<String> {
    let t = cid.get(7..16)?;
    let ok = cid.as_bytes().get(6) == Some(&b'-')
        && t[..4].bytes().all(|c| c.is_ascii_uppercase())
        && t[4..].bytes().all(|c| c.is_ascii_digit());
    ok.then(|| t.to_string())
}

pub fn read_info_from<R: Read + Seek>(r: &mut R, len: u64) -> Result<PkgInfo, PkgError> {
    if len < 4 {
        return Err(PkgError::NotPkg);
    }
    let magic = read_at(r, len, 0, 4)?;
    if be32(&magic, 0) != MAGIC {
        return Err(PkgError::NotPkg);
    }
    let h = read_at(r, len, 0, HEADER_LEN)?;
    let entry_count = be32(&h, 0x10);
    let table = u64::from(be32(&h, 0x18));
    if entry_count > MAX_ENTRIES {
        return Err(PkgError::Damaged("entry count"));
    }
    let content_id = String::from_utf8_lossy(&h[0x40..0x64])
        .trim_end_matches('\0')
        .to_string();
    let pkg_size = u64::from_be_bytes(h[0x430..0x438].try_into().expect("8 bytes"));
    let t = read_at(r, len, table, entry_count as usize * 32)?;

    let (mut sfo_e, mut icon_e) = (None, None);
    for e in t.as_chunks::<32>().0 {
        let (id, flags1, off, size) = (be32(e, 0), be32(e, 8), be32(e, 16), be32(e, 20));
        let encrypted = flags1 & 0x8000_0000 != 0;
        match id {
            ID_SFO if !encrypted => sfo_e = Some((u64::from(off), size as usize)),
            ID_ICON0 if !encrypted => icon_e = Some((u64::from(off), size as usize)),
            _ => {}
        }
    }

    let sfo = match sfo_e {
        Some((off, size)) if size <= sfo::MAX_SFO => sfo::parse(&read_at(r, len, off, size)?),
        Some(_) => return Err(PkgError::Damaged("param.sfo too large")),
        None => None,
    };
    let get = |k: &str| {
        sfo.as_ref()
            .and_then(|m| m.get(k))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .filter(|s| !s.is_empty())
    };

    let sfo_missing = sfo.is_none();
    let title_id = match get("TITLE_ID").or_else(|| title_id_from_content_id(&content_id)) {
        Some(t) => t,
        None => return Err(PkgError::NoSfo),
    };
    let content_id_mismatch = get("CONTENT_ID").is_some_and(|c| c != content_id);

    let icon = match icon_e {
        Some((off, size)) if size <= MAX_ICON => decode_icon(&read_at(r, len, off, size)?),
        _ => None,
    };

    Ok(PkgInfo {
        content_id,
        title_id,
        title: get("TITLE"),
        category: get("CATEGORY"),
        app_ver: get("APP_VER"),
        version: get("VERSION"),
        pkg_size,
        icon,
        sfo_missing,
        content_id_mismatch,
    })
}

/// PNG → RGBA, rejecting images larger than 1024×1024 before decoding (DESIGN 9.5).
pub fn decode_icon(png: &[u8]) -> Option<Icon> {
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let (w, h) = (be32(png, 16), be32(png, 20));
    if w == 0 || h == 0 || w > MAX_ICON_DIM || h > MAX_ICON_DIM {
        return None;
    }
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    Some(Icon {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::fixture::Fixture;
    use std::io::Cursor;

    fn info(b: &[u8]) -> Result<PkgInfo, PkgError> {
        read_info_from(&mut Cursor::new(b), b.len() as u64)
    }

    #[test]
    fn game_update_dlc() {
        let g = info(&Fixture::game("CUSA00001").bytes()).unwrap();
        assert_eq!(g.kind(), Kind::Game);
        assert_eq!(g.title_id, "CUSA00001");
        assert_eq!(g.title.as_deref(), Some("Test Game CUSA00001"));
        assert_eq!(g.display_version(), Some("01.00"));
        assert_eq!(g.icon.as_ref().map(|i| (i.width, i.height)), Some((8, 8)));
        assert!(!g.sfo_missing && !g.content_id_mismatch);

        let u = info(&Fixture::update("CUSA00001", "01.05").bytes()).unwrap();
        assert_eq!(
            (u.kind(), u.display_version()),
            (Kind::Update, Some("01.05"))
        );
        let d = info(&Fixture::dlc("CUSA00001").bytes()).unwrap();
        assert_eq!(d.kind(), Kind::Dlc);
    }

    #[test]
    fn pkg03_policy_and_errors() {
        let mut f = Fixture::game("CUSA00002");
        f.encrypt_sfo = true;
        let i = info(&f.bytes()).unwrap();
        assert!(i.sfo_missing);
        assert_eq!(i.title_id, "CUSA00002");
        assert_eq!(i.kind(), Kind::Other("?".into()));

        f.content_id = "garbage-content-id-without-title-id".into();
        assert!(matches!(info(&f.bytes()), Err(PkgError::NoSfo)));
        assert!(matches!(info(b"PK\x03\x04 zip"), Err(PkgError::NotPkg)));
        let b = Fixture::game("CUSA00003").bytes();
        assert!(matches!(info(&b[..0x200]), Err(PkgError::Damaged(_))));
        assert_eq!(
            title_id_from_content_id("EP0002-CUSA00184_00-ABCDEFGHIJKLMNOP").as_deref(),
            Some("CUSA00184")
        );
    }

    /// PLAN 1 acceptance: 10 000 random and truncated inputs without panic.
    #[test]
    fn no_panic_fuzz() {
        let base = Fixture::game("CUSA00004").bytes();
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for i in 0..10_000 {
            let mut b = base.clone();
            match i % 3 {
                0 => b.truncate((rnd() as usize) % base.len()),
                1 => {
                    for _ in 0..8 {
                        let p = (rnd() as usize) % b.len();
                        b[p] = rnd() as u8;
                    }
                }
                _ => b = (0..(rnd() % 4096) as usize).map(|_| rnd() as u8).collect(),
            }
            let _ = info(&b);
        }
    }

    #[test]
    fn rejects_huge_icon_dims() {
        let mut png = Fixture::tiny_png();
        png[16..20].copy_from_slice(&2048u32.to_be_bytes());
        assert!(decode_icon(&png).is_none());
    }
}
