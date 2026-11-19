//! %APPDATA% paths and atomic JSON files (DESIGN 7.4).

use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const APP_DIR: &str = "PS Package Pusher";

/// `%APPDATA%\PS Package Pusher` (falls back to the current dir if APPDATA is missing).
pub fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

/// `tmp → fsync → rename`, so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    write_atomic(path, &data)
}

#[derive(Debug)]
pub enum Loaded<T> {
    Ok(T),
    Missing,
    /// File was not valid JSON; it was renamed to the given path (CFG-02).
    Broken(PathBuf),
}

/// Reads JSON; a corrupt file is moved aside to `<stem>.broken-<unix>.json`.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<Loaded<T>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Loaded::Missing),
        Err(e) => return Err(e),
    };
    match serde_json::from_slice(&bytes) {
        Ok(v) => Ok(Loaded::Ok(v)),
        Err(_) => {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let broken = path.with_file_name(format!("{stem}.broken-{ts}.json"));
            fs::rename(path, &broken)?;
            Ok(Loaded::Broken(broken))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pspp-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn atomic_roundtrip_and_broken() {
        let d = temp_dir("storage");
        let p = d.join("sub").join("x.json");
        assert!(matches!(
            read_json::<Vec<u32>>(&p).unwrap(),
            Loaded::Missing
        ));
        write_json(&p, &vec![1u32, 2]).unwrap();
        write_json(&p, &vec![3u32]).unwrap();
        assert!(matches!(read_json::<Vec<u32>>(&p).unwrap(), Loaded::Ok(v) if v == [3]));
        assert!(!p.with_extension("tmp").exists());
        fs::write(&p, b"{ not json").unwrap();
        match read_json::<Vec<u32>>(&p).unwrap() {
            Loaded::Broken(b) => {
                assert!(b.exists() && !p.exists());
                assert!(
                    b.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("x.broken-")
                );
            }
            other => panic!("{other:?}"),
        }
        fs::remove_dir_all(d).unwrap();
    }
}
