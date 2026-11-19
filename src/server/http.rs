//! Minimal HTTP/1.1 request reader/writer shared by the file server and the mock RPI.

use std::io::{self, BufRead, Read, Write};

pub const MAX_LINE: usize = 8 * 1024;
pub const MAX_HEADERS: usize = 100;
pub const MAX_BODY: usize = 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub method: String,
    /// Path including any query string, exactly as sent.
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    pub fn wants_close(&self) -> bool {
        self.header("connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"))
    }
}

fn read_line<R: BufRead>(r: &mut R) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = r
        .by_ref()
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.len() > MAX_LINE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"));
    }
    while matches!(buf.last(), Some(b'\n' | b'\r')) {
        buf.pop();
    }
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

/// Reads one request. `Ok(None)` = peer closed the keep-alive connection cleanly.
pub fn read_request<R: BufRead>(r: &mut R) -> io::Result<Option<Request>> {
    let Some(line) = read_line(r)? else {
        return Ok(None);
    };
    let mut it = line.split_whitespace();
    let (Some(method), Some(target)) = (it.next(), it.next()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad request line",
        ));
    };
    let mut req = Request {
        method: method.to_string(),
        target: target.to_string(),
        ..Default::default()
    };
    loop {
        let Some(h) = read_line(r)? else {
            return Err(io::ErrorKind::UnexpectedEof.into());
        };
        if h.is_empty() {
            break;
        }
        if req.headers.len() >= MAX_HEADERS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too many headers",
            ));
        }
        if let Some((k, v)) = h.split_once(':') {
            req.headers
                .push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    if let Some(n) = req
        .header("content-length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        if n > MAX_BODY {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "body too large"));
        }
        req.body = vec![0; n];
        r.read_exact(&mut req.body)?;
    }
    Ok(Some(req))
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

/// Writes status line + headers + body (Content-Length always set; no Server header).
pub fn respond<W: Write>(
    w: &mut W,
    status: u16,
    headers: &[(&str, String)],
    body: &[u8],
) -> io::Result<()> {
    let mut s = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (k, v) in headers {
        s += &format!("{k}: {v}\r\n");
    }
    s += &format!("Content-Length: {}\r\n\r\n", body.len());
    w.write_all(s.as_bytes())?;
    w.write_all(body)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_requests() {
        let raw = b"POST /api/install HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\n\r\n{}GET /x HTTP/1.1\r\nRange: bytes=0-1\r\n\r\n";
        let mut c = Cursor::new(&raw[..]);
        let a = read_request(&mut c).unwrap().unwrap();
        assert_eq!(
            (a.method.as_str(), a.target.as_str(), a.body.as_slice()),
            ("POST", "/api/install", &b"{}"[..])
        );
        assert_eq!(a.header("HOST"), Some("127.0.0.1"));
        let b = read_request(&mut c).unwrap().unwrap();
        assert_eq!(b.header("range"), Some("bytes=0-1"));
        assert!(read_request(&mut c).unwrap().is_none());
        let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(MAX_LINE + 10));
        assert!(read_request(&mut Cursor::new(long.as_bytes())).is_err());
    }
}
