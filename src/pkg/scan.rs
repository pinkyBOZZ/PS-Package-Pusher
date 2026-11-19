//! Folder expansion, split grouping and new-group ordering (DESIGN 6.1 steps 1, 2, 5).

use super::{Kind, PkgInfo};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub const MAX_DEPTH: usize = 16;

#[derive(Debug, Default)]
pub struct Expanded {
    pub pkgs: Vec<PathBuf>,
    /// Files that were not `.pkg` (reported once as info).
    pub skipped: usize,
    pub unreadable: Vec<PathBuf>,
}

fn is_pkg(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pkg"))
}

/// Files stay as given; folders are walked recursively without following symlinks/junctions.
pub fn expand(inputs: &[PathBuf]) -> Expanded {
    let mut out = Expanded::default();
    let mut visited = HashSet::new();
    for p in inputs {
        walk(p, 0, &mut visited, &mut out);
    }
    out
}

fn walk(p: &Path, depth: usize, visited: &mut HashSet<PathBuf>, out: &mut Expanded) {
    let Ok(meta) = fs::symlink_metadata(p) else {
        out.unreadable.push(p.to_path_buf());
        return;
    };
    if meta.is_file() {
        if is_pkg(p) {
            out.pkgs.push(p.to_path_buf())
        } else {
            out.skipped += 1
        }
        return;
    }
    // symlinks and junctions are reported as non-dir by symlink_metadata on Windows → skipped
    if !meta.is_dir() || depth > MAX_DEPTH {
        return;
    }
    let canon = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    if !visited.insert(canon) {
        return;
    }
    let Ok(rd) = fs::read_dir(p) else {
        out.unreadable.push(p.to_path_buf());
        return;
    };
    let mut children: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    children.sort();
    for c in children {
        walk(&c, depth + 1, visited, out);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Group {
    Single(PathBuf),
    /// Parts in index order `_0.._n`.
    Split(Vec<PathBuf>),
    /// `_0` exists but indices have gaps (PKG-04).
    SplitMissing {
        present: Vec<PathBuf>,
        missing: Vec<u32>,
    },
}

impl Group {
    pub fn first(&self) -> &Path {
        match self {
            Group::Single(p) => p,
            Group::Split(v) | Group::SplitMissing { present: v, .. } => &v[0],
        }
    }
}

/// `name_<n>.pkg` → (`name`, n), case-insensitive extension.
fn split_name(p: &Path) -> Option<(String, u32)> {
    let stem = p.file_stem()?.to_str()?;
    let (prefix, idx) = stem.rsplit_once('_')?;
    if prefix.is_empty()
        || idx.is_empty()
        || !idx.bytes().all(|b| b.is_ascii_digit())
        || idx.len() > 4
    {
        return None;
    }
    Some((prefix.to_lowercase(), idx.parse().ok()?))
}

/// DESIGN 6.1/2: group `X_0.pkg, X_1.pkg …` in the same folder. Without `_0` every `X_n` is a
/// standalone package (seen on real files, e.g. a game named `…_2.pkg`). `X_0` alone is single.
pub fn group(files: Vec<PathBuf>) -> Vec<Group> {
    let mut by_key: BTreeMap<(PathBuf, String), BTreeMap<u32, PathBuf>> = BTreeMap::new();
    let mut out = Vec::new();
    let mut order: Vec<(PathBuf, String)> = Vec::new();
    for f in files {
        match split_name(&f) {
            Some((prefix, idx)) => {
                let key = (f.parent().unwrap_or(Path::new("")).to_path_buf(), prefix);
                if !by_key.contains_key(&key) {
                    order.push(key.clone());
                }
                by_key.entry(key).or_default().insert(idx, f);
            }
            None => out.push(Group::Single(f)),
        }
    }
    for key in order {
        let parts = by_key.remove(&key).expect("key from order");
        if !parts.contains_key(&0) || parts.len() == 1 {
            out.extend(parts.into_values().map(Group::Single));
            continue;
        }
        let max = *parts.keys().last().expect("non-empty");
        let missing: Vec<u32> = (0..=max).filter(|i| !parts.contains_key(i)).collect();
        let present: Vec<PathBuf> = parts.into_values().collect();
        out.push(if missing.is_empty() {
            Group::Split(present)
        } else {
            Group::SplitMissing { present, missing }
        });
    }
    out
}

/// Ordering of one add action (DESIGN 6.1/5): games, updates by version, DLC, others; then title.
pub fn sort_new<T>(items: &mut [T], info: impl Fn(&T) -> &PkgInfo) {
    let rank = |k: &Kind| match k {
        Kind::Game => 0,
        Kind::Update => 1,
        Kind::Dlc => 2,
        Kind::Other(_) => 3,
    };
    items.sort_by(|a, b| {
        let (a, b) = (info(a), info(b));
        let (ka, kb) = (a.kind(), b.kind());
        rank(&ka)
            .cmp(&rank(&kb))
            .then_with(|| {
                if ka == Kind::Update {
                    a.display_version().cmp(&b.display_version())
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| a.title.cmp(&b.title))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::{fixture::Fixture, read_info};

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn grouping_rules() {
        let g = group(vec![
            p("a/X_1.pkg"),
            p("a/X_0.pkg"),
            p("a/X_2.pkg"),
            p("a/single.pkg"),
            p("a/game_2.pkg"),
            p("a/Y_0.pkg"),
            p("b/X_1.pkg"),
            p("a/Z_0.pkg"),
            p("a/Z_2.pkg"),
        ]);
        assert_eq!(
            g,
            vec![
                Group::Single(p("a/single.pkg")),
                Group::Split(vec![p("a/X_0.pkg"), p("a/X_1.pkg"), p("a/X_2.pkg")]),
                Group::Single(p("a/game_2.pkg")),
                Group::Single(p("a/Y_0.pkg")),
                Group::Single(p("b/X_1.pkg")),
                Group::SplitMissing {
                    present: vec![p("a/Z_0.pkg"), p("a/Z_2.pkg")],
                    missing: vec![1]
                },
            ]
        );
    }

    #[test]
    fn expand_and_sort() {
        let d = std::env::temp_dir().join(format!("pspp-test-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("sub/deeper")).unwrap();
        Fixture::dlc("CUSA00001").write(&d.join("c.PKG")).unwrap();
        Fixture::update("CUSA00001", "01.10")
            .write(&d.join("sub/u2.pkg"))
            .unwrap();
        Fixture::update("CUSA00001", "01.05")
            .write(&d.join("sub/deeper/u1.pkg"))
            .unwrap();
        Fixture::game("CUSA00001")
            .write(&d.join("sub/g.pkg"))
            .unwrap();
        fs::write(d.join("readme.txt"), "x").unwrap();
        let e = expand(&[d.clone(), d.join("sub")]);
        assert_eq!((e.pkgs.len(), e.skipped), (4, 1), "{e:?}");
        let mut infos: Vec<PkgInfo> = e.pkgs.iter().map(|p| read_info(p).unwrap()).collect();
        sort_new(&mut infos, |i| i);
        let order: Vec<_> = infos
            .iter()
            .map(|i| (i.kind(), i.display_version().unwrap().to_string()))
            .collect();
        assert_eq!(order[0].0, Kind::Game);
        assert_eq!(order[1], (Kind::Update, "01.05".into()));
        assert_eq!(order[2], (Kind::Update, "01.10".into()));
        assert_eq!(order[3].0, Kind::Dlc);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn split_fixture_over_4gb() {
        let d = std::env::temp_dir().join(format!("pspp-test-split-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let mut f = Fixture::game("CUSA00009");
        f.total_size = 5 << 30;
        let parts = f.write_split(&d, "Big", 3).unwrap();
        let sizes: u64 = parts.iter().map(|p| fs::metadata(p).unwrap().len()).sum();
        assert_eq!(sizes, 5 << 30);
        let g = group(expand(std::slice::from_ref(&d)).pkgs);
        assert!(matches!(&g[..], [Group::Split(v)] if v.len() == 3));
        assert_eq!(read_info(&parts[0]).unwrap().pkg_size, 5 << 30);
        fs::remove_dir_all(d).unwrap();
    }
}
