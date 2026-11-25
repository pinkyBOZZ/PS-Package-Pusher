//! Error catalog (DESIGN 10) and console code table (DESIGN 10.1).

use serde::{Deserialize, Serialize};

macro_rules! codes {
    ($($v:ident = $s:literal),* $(,)?) => {
        /// Stable error codes; used in logs, i18n keys (`error.<CODE>.what|action`) and README.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Code { $(#[serde(rename = $s)] $v),* }
        impl Code {
            pub const ALL: &[Code] = &[$(Code::$v),*];
            pub fn as_str(self) -> &'static str { match self { $(Code::$v => $s),* } }
        }
    };
}

codes! {
    Cfg01 = "CFG-01", Cfg02 = "CFG-02", Cfg03 = "CFG-03",
    Net01 = "NET-01", Net02 = "NET-02", Net03 = "NET-03", Net04 = "NET-04", Net05 = "NET-05", Net06 = "NET-06",
    Fw01 = "FW-01", Fw02 = "FW-02", Fw03 = "FW-03", Fw04 = "FW-04",
    Srv01 = "SRV-01", Srv02 = "SRV-02", Srv03 = "SRV-03", Srv04 = "SRV-04",
    Rpi01 = "RPI-01", Rpi02 = "RPI-02", Rpi04 = "RPI-04", Rpi05 = "RPI-05", Rpi06 = "RPI-06", Rpi07 = "RPI-07", Rpi08 = "RPI-08",
    Pkg01 = "PKG-01", Pkg02 = "PKG-02", Pkg03 = "PKG-03", Pkg04 = "PKG-04", Pkg05 = "PKG-05", Pkg06 = "PKG-06", Pkg07 = "PKG-07", Pkg08 = "PKG-08",
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Code {
    /// Errors where the console took part: attach `/api/log` (DESIGN 6.4/2).
    pub fn wants_console_log(self) -> bool {
        matches!(
            self,
            Code::Rpi01 | Code::Rpi04 | Code::Rpi05 | Code::Rpi06 | Code::Rpi08
        )
    }
}

/// What the UI and the log show for one failure (DESIGN 6.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorReport {
    pub code: Code,
    /// Short, already-localisable detail (e.g. RPI `error` text, missing split parts).
    pub detail: String,
    /// Technical details for the expandable section / bug reports.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub technical: String,
    /// Path of the saved console log, if one was fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_log: Option<String>,
}

impl ErrorReport {
    pub fn new(code: Code, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            technical: String::new(),
            console_log: None,
        }
    }
    pub fn with_technical(mut self, t: impl Into<String>) -> Self {
        self.technical = t.into();
        self
    }
}

/// Known console codes (DESIGN 10.1). `verified` stays false until reproduced on a console.
pub struct ConsoleCode {
    pub code: &'static str,
    /// i18n key prefix: `console.<key>.meaning|action`.
    pub key: &'static str,
    pub verified: bool,
}

pub const CONSOLE_CODES: &[ConsoleCode] = &[
    ConsoleCode {
        code: "CE-30005-8",
        key: "ce30005",
        verified: false,
    },
    ConsoleCode {
        code: "CE-36329-3",
        key: "ce36329",
        verified: false,
    },
    ConsoleCode {
        code: "CE-32809-2",
        key: "ce32809",
        verified: false,
    },
    ConsoleCode {
        code: "0x80990004",
        key: "bgft80990004",
        verified: true,
    },
    ConsoleCode {
        code: "0x80990015",
        key: "bgft80990015",
        verified: true,
    },
    ConsoleCode {
        code: "0x80990019",
        key: "bgft80990019",
        verified: true,
    },
    ConsoleCode {
        code: "0x80991404",
        key: "bgft80991404",
        verified: true,
    },
];

pub fn lookup_console_code(code: &str) -> Option<&'static ConsoleCode> {
    CONSOLE_CODES
        .iter()
        .find(|c| c.code.eq_ignore_ascii_case(code))
}

/// Finds `CE-12345-6` and `0x8099XXXX` codes in free text (RPI `error` field, console log).
pub fn find_console_codes(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: String| {
        if !out.contains(&s) {
            out.push(s);
        }
    };
    let mut i = 0;
    while i < b.len() {
        let rest = &b[i..];
        let digits = |r: &[u8], n: usize| r.len() >= n && r[..n].iter().all(u8::is_ascii_digit);
        let hexes = |r: &[u8], n: usize| r.len() >= n && r[..n].iter().all(u8::is_ascii_hexdigit);
        if rest.len() >= 10
            && rest[..3].eq_ignore_ascii_case(b"CE-")
            && digits(&rest[3..], 5)
            && rest[8] == b'-'
            && digits(&rest[9..], 1)
        {
            push(format!(
                "CE-{}-{}",
                &text[i + 3..i + 8],
                &text[i + 9..i + 10]
            ));
            i += 10;
        } else if rest.len() >= 10
            && rest[..2].eq_ignore_ascii_case(b"0x")
            && rest[2..6].eq_ignore_ascii_case(b"8099")
            && hexes(&rest[6..], 4)
        {
            push(format!("0x{}", text[i + 2..i + 10].to_ascii_uppercase()));
            i += 10;
        } else {
            i += 1;
        }
    }
    out
}

/// Localised `what`/`action` text with every `{placeholder}` filled by the report's detail.
pub fn describe(t: &crate::i18n::I18n, e: &ErrorReport, part: &str) -> String {
    let mut s = t.t(&format!("error.{}.{part}", e.code));
    while let (Some(a), Some(b)) = (s.find('{'), s.find('}')) {
        if b < a {
            break;
        }
        s.replace_range(a..=b, e.detail.trim());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_strings_unique_and_serde() {
        let mut seen = std::collections::HashSet::new();
        for c in Code::ALL {
            assert!(seen.insert(c.as_str()), "duplicate {c}");
            let j = serde_json::to_string(c).unwrap();
            assert_eq!(j, format!("\"{c}\""));
            assert_eq!(serde_json::from_str::<Code>(&j).unwrap(), *c);
        }
    }

    #[test]
    fn scanner() {
        let t = "fail: ce-36329-3 then 0x80990085, again CE-36329-3; not CE-1234-5 nor 0x8098ffff";
        assert_eq!(find_console_codes(t), vec!["CE-36329-3", "0x80990085"]);
        assert!(find_console_codes("").is_empty());
        assert!(find_console_codes("CE-12345-").is_empty());
        assert!(lookup_console_code("ce-30005-8").is_some());
    }
}
