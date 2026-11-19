//! Translations EN/BS (DESIGN 7.7): flat JSON maps embedded at build time.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Bs,
}

impl Lang {
    pub fn from_code(code: &str) -> Lang {
        if code == "bs" { Lang::Bs } else { Lang::En }
    }
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Bs => "bs",
        }
    }
}

const EN_JSON: &str = include_str!("../lang/en.json");
const BS_JSON: &str = include_str!("../lang/bs.json");

pub struct I18n {
    pub lang: Lang,
    en: HashMap<String, String>,
    bs: HashMap<String, String>,
}

impl I18n {
    pub fn new(lang: Lang) -> Self {
        // Embedded files are checked by the parity test; a parse failure is a build bug.
        let parse = |s| serde_json::from_str(s).expect("embedded lang json");
        Self {
            lang,
            en: parse(EN_JSON),
            bs: parse(BS_JSON),
        }
    }

    /// Lookup with fallback bs → en → key.
    pub fn t(&self, key: &str) -> String {
        let hit = match self.lang {
            Lang::Bs => self.bs.get(key).or_else(|| self.en.get(key)),
            Lang::En => self.en.get(key),
        };
        hit.cloned().unwrap_or_else(|| key.to_string())
    }

    /// `{name}` placeholders replaced from `args`.
    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        fill(self.t(key), args)
    }

    /// Plural: `<key>.one|few|other`, `{n}` filled.
    pub fn plural(&self, key: &str, n: u64) -> String {
        let form = plural_form(self.lang, n);
        let map = match self.lang {
            Lang::Bs => &self.bs,
            Lang::En => &self.en,
        };
        let k = [form, "other"]
            .iter()
            .map(|f| format!("{key}.{f}"))
            .find(|k| map.contains_key(k) || self.en.contains_key(k))
            .unwrap_or_else(|| format!("{key}.other"));
        fill(self.t(&k), &[("n", &n.to_string())])
    }

    /// Windows Explorer style size, 1024-based: `61.3 GB` (en) / `61,3 GB` (bs).
    pub fn size(&self, bytes: u64) -> String {
        format_size(self.lang, bytes)
    }
}

fn fill(mut s: String, args: &[(&str, &str)]) -> String {
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

pub fn plural_form(lang: Lang, n: u64) -> &'static str {
    match lang {
        Lang::En => {
            if n == 1 {
                "one"
            } else {
                "other"
            }
        }
        Lang::Bs => {
            let (d, h) = (n % 10, n % 100);
            if d == 1 && h != 11 {
                "one"
            } else if (2..=4).contains(&d) && !(12..=14).contains(&h) {
                "few"
            } else {
                "other"
            }
        }
    }
}

pub fn format_size(lang: Lang, bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    let s = format!("{v:.1} {}", UNITS[u]);
    if lang == Lang::Bs {
        s.replace('.', ",")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn base(k: &str) -> &str {
        ["one", "few", "other"]
            .iter()
            .find_map(|f| {
                k.strip_suffix(&format!(".{f}"))
                    .map(|_| &k[..k.len() - f.len() - 1])
            })
            .unwrap_or(k)
    }

    fn args(s: &str) -> BTreeSet<String> {
        s.split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(a, _)| a.to_string()))
            .collect()
    }

    #[test]
    fn parity_keys_and_args() {
        let i = I18n::new(Lang::En);
        let ek: BTreeSet<_> = i.en.keys().map(|k| base(k)).collect();
        let bk: BTreeSet<_> = i.bs.keys().map(|k| base(k)).collect();
        assert_eq!(ek, bk, "en/bs key sets differ");
        for (k, v) in &i.en {
            if let Some(b) = i.bs.get(k) {
                assert_eq!(args(v), args(b), "args differ for {k}");
            }
        }
        for c in crate::errors::Code::ALL {
            for part in ["what", "action"] {
                assert!(
                    i.en.contains_key(&format!("error.{c}.{part}")),
                    "missing error.{c}.{part}"
                );
            }
        }
        let forbidden = [
            '\u{25B3}', '\u{25CB}', '\u{00D7}', '\u{25A1}', '\u{2715}', '\u{25EF}', '\u{2610}',
        ];
        for v in i.en.values().chain(i.bs.values()) {
            assert!(!v.contains(forbidden), "forbidden symbol in {v}");
        }
    }

    #[test]
    fn plural_and_fallback() {
        let bs = I18n::new(Lang::Bs);
        assert_eq!(bs.plural("queue.packages", 1), "1 paket");
        assert_eq!(bs.plural("queue.packages", 3), "3 paketa");
        assert_eq!(bs.plural("queue.packages", 11), "11 paketa");
        assert_eq!(bs.plural("queue.packages", 21), "21 paket");
        let en = I18n::new(Lang::En);
        assert_eq!(en.plural("queue.packages", 4), "4 packages");
        assert_eq!(en.t("no.such.key"), "no.such.key");
        assert_eq!(
            en.t_args("error.SRV-01.what", &[("port", "8095")]),
            "Port 8095 is used by another program."
        );
    }

    #[test]
    fn sizes() {
        assert_eq!(format_size(Lang::En, 512), "512 B");
        assert_eq!(format_size(Lang::En, 1536), "1.5 KB");
        assert_eq!(format_size(Lang::Bs, 65_820_000_000), "61,3 GB");
        assert_eq!(format_size(Lang::En, 5 << 40), "5.0 TB");
    }
}
