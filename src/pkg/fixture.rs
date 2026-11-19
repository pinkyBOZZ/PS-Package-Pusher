//! Synthetic PS4 PKG generator for tests, the mock and `make_fixture` (DESIGN 8.3).
//! Produces only header + entry table + param.sfo + tiny PNG; never installable.

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Fixture {
    pub content_id: String,
    pub title: String,
    pub title_id: String,
    pub category: String,
    pub app_ver: String,
    pub version: String,
    pub encrypt_sfo: bool,
    pub with_icon: bool,
    /// Total package size written to @0x430 and used for the file length (≥ header bytes).
    pub total_size: u64,
}

const TABLE: usize = 0x1000;
const SFO_AT: usize = 0x1100;

impl Fixture {
    fn new(tid: &str, category: &str, app_ver: &str, suffix: &str) -> Self {
        Self {
            content_id: format!("UP0000-{tid}_00-{suffix:0<16}"),
            title: format!("Test Game {tid}"),
            title_id: tid.into(),
            category: category.into(),
            app_ver: app_ver.into(),
            version: app_ver.into(),
            encrypt_sfo: false,
            with_icon: true,
            total_size: 0,
        }
    }
    pub fn game(tid: &str) -> Self {
        Self::new(tid, "gd", "01.00", "GAME")
    }
    pub fn update(tid: &str, ver: &str) -> Self {
        let mut f = Self::new(tid, "gp", ver, "PATCH");
        f.title = format!("Test Game {tid} Update");
        f
    }
    pub fn dlc(tid: &str) -> Self {
        let mut f = Self::new(tid, "ac", "01.00", "DLC");
        f.title = format!("Test Game {tid} DLC");
        f
    }

    /// 8×8 PNG in the logo colours.
    pub fn tiny_png() -> Vec<u8> {
        let img = image::RgbaImage::from_fn(8, 8, |x, y| {
            if y < 4 {
                image::Rgba([255, 46 + (x * 20) as u8, 151, 255])
            } else {
                image::Rgba([0, 240, 255, 255])
            }
        });
        let mut out = io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png)
            .expect("png encode");
        out.into_inner()
    }

    /// Header region bytes (what `_0` / a single package starts with).
    pub fn bytes(&self) -> Vec<u8> {
        let sfo = sfo_bytes(
            &[
                ("TITLE", &self.title),
                ("TITLE_ID", &self.title_id),
                ("CONTENT_ID", &self.content_id),
                ("CATEGORY", &self.category),
                ("APP_VER", &self.app_ver),
                ("VERSION", &self.version),
            ],
            &[("APP_TYPE", 1)],
        );
        let icon = if self.with_icon {
            Self::tiny_png()
        } else {
            Vec::new()
        };
        let icon_at = (SFO_AT + sfo.len()).next_multiple_of(16);
        let end = icon_at + icon.len();
        let total = self.total_size.max(end as u64);

        let mut b = vec![0u8; end];
        b[0..4].copy_from_slice(&super::header::MAGIC.to_be_bytes());
        let entries: Vec<(u32, u32, usize, usize)> = [
            Some((
                super::header::ID_SFO,
                if self.encrypt_sfo { 0x8000_0000 } else { 0 },
                SFO_AT,
                sfo.len(),
            )),
            self.with_icon
                .then_some((super::header::ID_ICON0, 0, icon_at, icon.len())),
        ]
        .into_iter()
        .flatten()
        .collect();
        b[0x0C..0x10].copy_from_slice(&(entries.len() as u32).to_be_bytes());
        b[0x10..0x14].copy_from_slice(&(entries.len() as u32).to_be_bytes());
        b[0x18..0x1C].copy_from_slice(&(TABLE as u32).to_be_bytes());
        let cid = self.content_id.as_bytes();
        b[0x40..0x40 + cid.len().min(36)].copy_from_slice(&cid[..cid.len().min(36)]);
        b[0x430..0x438].copy_from_slice(&total.to_be_bytes());
        for (i, (id, flags, off, size)) in entries.iter().enumerate() {
            let e = TABLE + i * 32;
            b[e..e + 4].copy_from_slice(&id.to_be_bytes());
            b[e + 8..e + 12].copy_from_slice(&flags.to_be_bytes());
            b[e + 16..e + 20].copy_from_slice(&(*off as u32).to_be_bytes());
            b[e + 20..e + 24].copy_from_slice(&(*size as u32).to_be_bytes());
        }
        b[SFO_AT..SFO_AT + sfo.len()].copy_from_slice(&sfo);
        b[icon_at..end].copy_from_slice(&icon);
        b
    }

    /// Writes one package of `total_size` (sparse, so multi-GB files cost no disk space).
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let b = self.bytes();
        write_sparse(path, &b, self.total_size.max(b.len() as u64))
    }

    /// Writes `<dir>/<stem>_0.pkg … _<n-1>.pkg`; part 0 carries the header, sizes sum to `total_size`.
    pub fn write_split(&self, dir: &Path, stem: &str, parts: usize) -> io::Result<Vec<PathBuf>> {
        let b = self.bytes();
        let total = self.total_size.max(b.len() as u64 * parts as u64);
        let mut f = self.clone();
        f.total_size = total;
        let b = f.bytes();
        let per = (total / parts as u64).max(b.len() as u64);
        let mut out = Vec::new();
        let mut left = total;
        for i in 0..parts {
            let size = if i + 1 == parts { left } else { per };
            left -= size;
            let p = dir.join(format!("{stem}_{i}.pkg"));
            write_sparse(&p, if i == 0 { &b } else { &[] }, size)?;
            out.push(p);
        }
        Ok(out)
    }
}

/// Writes `head` and extends the file to `len` bytes as an NTFS sparse file when possible.
pub fn write_sparse(path: &Path, head: &[u8], len: u64) -> io::Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    set_sparse(&f);
    f.write_all(head)?;
    f.set_len(len.max(head.len() as u64))?;
    Ok(())
}

fn set_sparse(f: &File) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::{IO::DeviceIoControl, Ioctl::FSCTL_SET_SPARSE};
    let mut ret = 0u32;
    // SAFETY: valid file handle; FSCTL_SET_SPARSE takes no buffers. Failure just means a dense file.
    unsafe {
        DeviceIoControl(
            f.as_raw_handle(),
            FSCTL_SET_SPARSE,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut ret,
            std::ptr::null_mut(),
        );
    }
}

/// Builds a param.sfo (little-endian) with string and int entries.
pub fn sfo_bytes(strs: &[(&str, &str)], ints: &[(&str, u32)]) -> Vec<u8> {
    let n = strs.len() + ints.len();
    let mut keys = Vec::new();
    let mut data = Vec::new();
    let mut index = Vec::new();
    let mut push = |k: &str, fmt: u16, v: &[u8], max: usize| {
        index.extend_from_slice(&(keys.len() as u16).to_le_bytes());
        index.extend_from_slice(&fmt.to_le_bytes());
        index.extend_from_slice(&(v.len() as u32).to_le_bytes());
        index.extend_from_slice(&(max as u32).to_le_bytes());
        index.extend_from_slice(&(data.len() as u32).to_le_bytes());
        keys.extend_from_slice(k.as_bytes());
        keys.push(0);
        let mut v = v.to_vec();
        v.resize(max, 0);
        data.extend_from_slice(&v);
    };
    for (k, v) in strs {
        let mut s = v.as_bytes().to_vec();
        s.push(0);
        let max = s.len().next_multiple_of(4);
        push(k, 0x0204, &s, max);
    }
    for (k, v) in ints {
        push(k, 0x0404, &v.to_le_bytes(), 4);
    }
    while keys.len() % 4 != 0 {
        keys.push(0);
    }
    let key_start = 0x14 + 16 * n;
    let data_start = key_start + keys.len();
    let mut b = b"\0PSF".to_vec();
    b.extend_from_slice(&0x101u32.to_le_bytes());
    b.extend_from_slice(&(key_start as u32).to_le_bytes());
    b.extend_from_slice(&(data_start as u32).to_le_bytes());
    b.extend_from_slice(&(n as u32).to_le_bytes());
    b.extend_from_slice(&index);
    b.extend_from_slice(&keys);
    b.extend_from_slice(&data);
    b
}
