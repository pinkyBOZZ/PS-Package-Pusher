//! PLAN 5.7 / DESIGN 1.3: no controller-button symbols in any user-visible string.
//! Scans string literals in `src/` and all values in `lang/*.json`.

use std::path::Path;

const FORBIDDEN: [char; 7] = [
    '\u{25B3}', '\u{25CB}', '\u{00D7}', '\u{25A1}', '\u{2715}', '\u{25EF}', '\u{2610}',
];

/// Contents of "..." literals on one line (escaped quotes are skipped; good enough for this codebase).
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    let mut esc = false;
    for c in line.chars() {
        match (&mut cur, c) {
            (Some(s), _) if esc => {
                s.push(c);
                esc = false;
            }
            (Some(_), '\\') => esc = true,
            (Some(s), '"') => {
                out.push(std::mem::take(s));
                cur = None;
            }
            (Some(s), _) => s.push(c),
            (None, '"') => cur = Some(String::new()),
            (None, _) => {}
        }
    }
    out
}

fn walk(dir: &Path, hits: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, hits);
        } else if p.extension().is_some_and(|x| x == "rs") {
            for (n, line) in std::fs::read_to_string(&p).unwrap().lines().enumerate() {
                let code = line.split("//").next().unwrap_or_default();
                for lit in literals(code) {
                    if lit.contains(FORBIDDEN) {
                        hits.push(format!("{}:{}: {lit}", p.display(), n + 1));
                    }
                }
            }
        }
    }
}

#[test]
fn no_forbidden_symbols() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    walk(&root.join("src"), &mut hits);
    for lang in ["en", "bs"] {
        let text = std::fs::read_to_string(root.join(format!("lang/{lang}.json"))).unwrap();
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&text).unwrap();
        for (k, v) in map {
            if v.as_str().is_some_and(|s| s.contains(FORBIDDEN)) {
                hits.push(format!("lang/{lang}.json: {k}"));
            }
        }
    }
    assert!(hits.is_empty(), "forbidden symbols:\n{}", hits.join("\n"));
}

#[test]
fn literal_scanner_works() {
    assert_eq!(literals(r#"let a = "x\"y"; f("z")"#), vec!["x\"y", "z"]);
    assert!(literals("no strings").is_empty());
}
