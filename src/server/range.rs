//! `Range` header parsing (DESIGN 9.4). Pure function, u64 everywhere (> 4 GB files).

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Range {
    /// No Range header → 200 with the whole file.
    Full,
    /// Inclusive byte range → 206.
    Partial(u64, u64),
    /// Invalid, multiple ranges or outside the file → 416.
    Unsatisfiable,
}

pub fn parse(header: Option<&str>, len: u64) -> Range {
    let Some(h) = header else { return Range::Full };
    let Some(spec) = h.trim().strip_prefix("bytes=") else {
        return Range::Unsatisfiable;
    };
    let spec = spec.trim();
    if spec.contains(',') || len == 0 {
        return Range::Unsatisfiable;
    }
    let Some((a, b)) = spec.split_once('-') else {
        return Range::Unsatisfiable;
    };
    let num = |s: &str| s.trim().parse::<u64>().ok();
    let r = match (a.trim().is_empty(), b.trim().is_empty()) {
        (true, false) => num(b)
            .filter(|&n| n > 0)
            .map(|n| (len.saturating_sub(n), len - 1)),
        (false, true) => num(a).map(|s| (s, len - 1)),
        (false, false) => num(a).zip(num(b)).map(|(s, e)| (s, e.min(len - 1))),
        (true, true) => None,
    };
    match r {
        Some((s, e)) if s <= e && s < len => Range::Partial(s, e),
        _ => Range::Unsatisfiable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const BIG: u64 = 5 << 30;

    #[test]
    fn forms() {
        assert_eq!(parse(None, 1000), Range::Full);
        assert_eq!(parse(Some("bytes=0-99"), 1000), Range::Partial(0, 99));
        assert_eq!(parse(Some(" bytes=900- "), 1000), Range::Partial(900, 999));
        assert_eq!(parse(Some("bytes=-100"), 1000), Range::Partial(900, 999));
        assert_eq!(parse(Some("bytes=-5000"), 1000), Range::Partial(0, 999));
        assert_eq!(
            parse(Some("bytes=900-5000"), 1000),
            Range::Partial(900, 999)
        );
        assert_eq!(parse(Some("bytes=1000-"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=5-1"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=0-1,5-6"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=-0"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=-"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("items=0-1"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=a-b"), 1000), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=0-0"), 0), Range::Unsatisfiable);
    }

    #[test]
    fn above_u32() {
        let s = u64::from(u32::MAX) + 10;
        assert_eq!(
            parse(Some(&format!("bytes={s}-")), BIG),
            Range::Partial(s, BIG - 1)
        );
        assert_eq!(
            parse(Some(&format!("bytes={s}-{}", s + 8_388_607)), BIG),
            Range::Partial(s, s + 8_388_607)
        );
        assert_eq!(
            parse(Some("bytes=-16"), BIG),
            Range::Partial(BIG - 16, BIG - 1)
        );
    }
}
