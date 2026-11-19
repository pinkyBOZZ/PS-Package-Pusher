//! Whitelist `(item_id, safe-name)` → file (DESIGN 9.4). URL parts never become paths.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: Option<SystemTime>,
}

#[derive(Clone, Default)]
pub struct Registry {
    map: Arc<RwLock<HashMap<(String, String), Part>>>,
}

/// `<ContentID>[_n].pkg`, filtered to `[A-Za-z0-9._-]`.
pub fn safe_name(content_id: &str, part: Option<usize>) -> String {
    let mut s: String = content_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if s.is_empty() {
        s.push_str("package");
    }
    if let Some(n) = part {
        s += &format!("_{n}");
    }
    s + ".pkg"
}

impl Registry {
    /// Registers all parts of one item; returns the URL paths in part order.
    pub fn register(&self, item_id: &str, content_id: &str, parts: &[Part]) -> Vec<String> {
        let split = parts.len() > 1;
        let mut map = self.map.write().unwrap_or_else(|e| e.into_inner());
        map.retain(|(id, _), _| id != item_id);
        parts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = safe_name(content_id, split.then_some(i));
                map.insert((item_id.to_string(), name.clone()), p.clone());
                format!("/pkg/{item_id}/{name}")
            })
            .collect()
    }

    pub fn unregister(&self, item_id: &str) {
        self.map
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(id, _), _| id != item_id);
    }

    /// Exact match only; anything else is a 404.
    pub fn lookup(&self, url_path: &str) -> Option<(String, Part)> {
        let rest = url_path.strip_prefix("/pkg/")?;
        let (id, name) = rest.split_once('/')?;
        let map = self.map.read().unwrap_or_else(|e| e.into_inner());
        map.get(&(id.to_string(), name.to_string()))
            .map(|p| (id.to_string(), p.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_lookup() {
        assert_eq!(
            safe_name("UP0000-CUSA12345_00-0000000000000000", Some(1)),
            "UP0000-CUSA12345_00-0000000000000000_1.pkg"
        );
        assert_eq!(safe_name("../evil?x=1", None), "..evilx1.pkg");
        let r = Registry::default();
        let p = Part {
            path: "C:/x.pkg".into(),
            size: 10,
            mtime: None,
        };
        let urls = r.register("abc", "UP0000-CUSA00001_00-X", &[p.clone(), p.clone()]);
        assert_eq!(
            urls,
            [
                "/pkg/abc/UP0000-CUSA00001_00-X_0.pkg",
                "/pkg/abc/UP0000-CUSA00001_00-X_1.pkg"
            ]
        );
        assert!(r.lookup(&urls[1]).is_some());
        assert!(r.lookup("/pkg/abc/../x.pkg").is_none());
        assert!(r.lookup("/pkg/abc/UP0000-CUSA00001_00-X_0.pkg?x").is_none());
        r.unregister("abc");
        assert!(r.lookup(&urls[0]).is_none());
    }
}
