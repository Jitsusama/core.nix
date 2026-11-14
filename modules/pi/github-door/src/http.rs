//! HTTP/1.1 as far as the door reads it: a request's head and its whole
//! body, and a response's status line.
//!
//! The door reads a request whole before judging it, since a GraphQL
//! document is in the body, and sends one request on each connection,
//! closing it after the answer, so it never has to find where one answer
//! ends and the next begins. A request that could be read two ways, with
//! both a length and chunks, or chunks named some other way, is refused
//! rather than guessed at.

use std::io::{self, Read, Write};

/// The most a head may take.
const MOST_HEAD: usize = 64 * 1024;
/// The most headers a head may hold.
const MOST_HEADERS: usize = 100;

/// A request's head.
#[derive(Debug, Clone)]
pub struct Head {
    pub method: String,
    pub target: String,
    /// Each header, its name as sent.
    pub headers: Vec<(String, Vec<u8>)>,
}

impl Head {
    /// The value of the header named `name`, when there's exactly one,
    /// as text.
    pub fn header(&self, name: &str) -> Option<&str> {
        let mut found = self
            .headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name));
        let first = found.next()?;
        if found.next().is_some() {
            return None;
        }
        std::str::from_utf8(&first.1).ok().map(str::trim)
    }

    /// How many headers named `name` it holds.
    pub fn count(&self, name: &str) -> usize {
        self.headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .count()
    }
}

/// Why a request couldn't be read.
#[derive(Debug)]
pub enum Unread {
    /// The client went before saying anything.
    Gone,
    /// What it said isn't a request the door reads.
    Malformed(String),
    /// Its body is longer than the most the door reads.
    TooLarge,
    /// Reading failed.
    Io(io::Error),
}

impl From<io::Error> for Unread {
    fn from(error: io::Error) -> Self {
        Unread::Io(error)
    }
}

/// A request's head read from `stream`, with whatever was read past it
/// left in `buffer`.
pub fn read_head(stream: &mut impl Read, buffer: &mut Vec<u8>) -> Result<Head, Unread> {
    loop {
        let mut headers = [httparse::EMPTY_HEADER; MOST_HEADERS];
        let mut request = httparse::Request::new(&mut headers);
        match request.parse(buffer) {
            Ok(httparse::Status::Complete(length)) => {
                if request.version != Some(1) {
                    return Err(Unread::Malformed(
                        "a request that isn't HTTP/1.1".to_owned(),
                    ));
                }
                let head = Head {
                    method: request.method.unwrap_or_default().to_owned(),
                    target: request.path.unwrap_or_default().to_owned(),
                    headers: request
                        .headers
                        .iter()
                        .map(|header| (header.name.to_owned(), header.value.to_vec()))
                        .collect(),
                };
                buffer.drain(..length);
                return Ok(head);
            }
            Ok(httparse::Status::Partial) => {}
            Err(error) => {
                return Err(Unread::Malformed(format!(
                    "a head that won't parse: {error}"
                )));
            }
        }
        if buffer.len() >= MOST_HEAD {
            return Err(Unread::Malformed("a head past 64 KiB".to_owned()));
        }
        if more(stream, buffer)? == 0 {
            return Err(if buffer.is_empty() {
                Unread::Gone
            } else {
                Unread::Malformed("a head cut short".to_owned())
            });
        }
    }
}

/// Read more of `stream` onto `buffer`, saying how much came.
fn more(stream: &mut impl Read, buffer: &mut Vec<u8>) -> io::Result<usize> {
    let mut chunk = [0u8; 16 * 1024];
    let read = stream.read(&mut chunk)?;
    buffer.extend_from_slice(&chunk[..read]);
    Ok(read)
}

/// Read exactly `wanted` bytes past what `buffer` holds from its start,
/// taking them from `buffer` first.
fn take(stream: &mut impl Read, buffer: &mut Vec<u8>, wanted: usize) -> Result<Vec<u8>, Unread> {
    while buffer.len() < wanted {
        if more(stream, buffer)? == 0 {
            return Err(Unread::Malformed("a body cut short".to_owned()));
        }
    }
    Ok(buffer.drain(..wanted).collect())
}

/// The body `head` says follows, read whole from `buffer` and `stream`,
/// at most `most` bytes, chunks joined.
pub fn read_body(
    stream: &mut impl Read,
    buffer: &mut Vec<u8>,
    head: &Head,
    most: usize,
) -> Result<Vec<u8>, Unread> {
    let lengths = head.count("content-length");
    let encodings = head.count("transfer-encoding");
    if lengths > 0 && encodings > 0 {
        return Err(Unread::Malformed(
            "a body with both a length and chunks".to_owned(),
        ));
    }
    if encodings > 0 {
        let chunked = head
            .header("transfer-encoding")
            .is_some_and(|value| value.eq_ignore_ascii_case("chunked"));
        if !chunked {
            return Err(Unread::Malformed(
                "a transfer encoding other than chunked".to_owned(),
            ));
        }
        return chunks(stream, buffer, most);
    }
    if lengths == 0 {
        return Ok(Vec::new());
    }
    let length: usize = head
        .header("content-length")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| Unread::Malformed("a length that isn't one number".to_owned()))?;
    if length > most {
        return Err(Unread::TooLarge);
    }
    take(stream, buffer, length)
}

/// A chunked body, joined.
fn chunks(stream: &mut impl Read, buffer: &mut Vec<u8>, most: usize) -> Result<Vec<u8>, Unread> {
    let mut body = Vec::new();
    loop {
        let sized = line(stream, buffer)?;
        let size = sized.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size, 16)
            .map_err(|_| Unread::Malformed("a chunk whose size isn't one".to_owned()))?;
        if size == 0 {
            // Trailers, which the door drops, end at an empty line.
            while !line(stream, buffer)?.is_empty() {}
            return Ok(body);
        }
        if body.len() + size > most {
            return Err(Unread::TooLarge);
        }
        body.extend(take(stream, buffer, size)?);
        if !take(stream, buffer, 2)?.eq(b"\r\n") {
            return Err(Unread::Malformed(
                "a chunk not ended by its line".to_owned(),
            ));
        }
    }
}

/// One line, without its ending.
fn line(stream: &mut impl Read, buffer: &mut Vec<u8>) -> Result<String, Unread> {
    loop {
        if let Some(end) = buffer.windows(2).position(|pair| pair == b"\r\n") {
            let line: Vec<u8> = buffer.drain(..end + 2).take(end).collect();
            return String::from_utf8(line)
                .map_err(|_| Unread::Malformed("a chunk line that isn't text".to_owned()));
        }
        if buffer.len() > MOST_HEAD {
            return Err(Unread::Malformed("a chunk line past 64 KiB".to_owned()));
        }
        if more(stream, buffer)? == 0 {
            return Err(Unread::Malformed("a body cut short".to_owned()));
        }
    }
}

/// A response's status, read from its head's start in `buffer`, reading
/// more of `stream` as needed; the head stays in `buffer` to be passed on.
pub fn read_status(stream: &mut impl Read, buffer: &mut Vec<u8>) -> Result<u16, Unread> {
    loop {
        let mut headers = [httparse::EMPTY_HEADER; MOST_HEADERS];
        let mut response = httparse::Response::new(&mut headers);
        match response.parse(buffer) {
            Ok(httparse::Status::Complete(_)) => {
                return response
                    .code
                    .ok_or_else(|| Unread::Malformed("an answer with no status".to_owned()));
            }
            Ok(httparse::Status::Partial) => {}
            Err(error) => {
                return Err(Unread::Malformed(format!(
                    "an answer that won't parse: {error}"
                )));
            }
        }
        if buffer.len() >= MOST_HEAD {
            return Err(Unread::Malformed("an answer's head past 64 KiB".to_owned()));
        }
        if more(stream, buffer)? == 0 {
            return Err(Unread::Malformed("an answer cut short".to_owned()));
        }
    }
}

/// Write a whole request: its line, `headers`, a length and `body`.
pub fn write_request(
    out: &mut impl Write,
    method: &str,
    target: &str,
    headers: &[(String, Vec<u8>)],
    body: &[u8],
) -> io::Result<()> {
    let mut head = format!("{method} {target} HTTP/1.1\r\n").into_bytes();
    for (name, value) in headers {
        head.extend_from_slice(name.as_bytes());
        head.extend_from_slice(b": ");
        head.extend_from_slice(value);
        head.extend_from_slice(b"\r\n");
    }
    if !body.is_empty() || !matches!(method, "GET" | "HEAD" | "OPTIONS") {
        head.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    head.extend_from_slice(b"\r\n");
    out.write_all(&head)?;
    out.write_all(body)?;
    out.flush()
}

/// Write an answer the door makes itself: `status`, and `message` as
/// GitHub's own JSON error shape, so gh says it as it would GitHub's.
pub fn write_answer(out: &mut impl Write, status: u16, message: &str) -> io::Result<()> {
    let body = serde_json::json!({ "message": message }).to_string();
    let reason = match status {
        400 => "Bad Request",
        403 => "Forbidden",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Refused",
    };
    write!(
        out,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(raw: &[u8]) -> Result<(Head, Vec<u8>), Unread> {
        let mut stream = raw;
        let mut buffer = Vec::new();
        let head = read_head(&mut stream, &mut buffer)?;
        let body = read_body(&mut stream, &mut buffer, &head, 1024)?;
        Ok((head, body))
    }

    #[test]
    fn a_request_with_a_length_is_read_whole() {
        let (head, body) = read(
            b"POST /graphql HTTP/1.1\r\nHost: api.github.com\r\nContent-Length: 5\r\n\r\nhello",
        )
        .expect("read");
        assert_eq!(head.method, "POST");
        assert_eq!(head.target, "/graphql");
        assert_eq!(head.header("host"), Some("api.github.com"));
        assert_eq!(body, b"hello");
    }

    #[test]
    fn chunks_are_joined() {
        let (_, body) = read(
            b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
              3\r\nabc\r\n2;x=y\r\nde\r\n0\r\nTrailer: t\r\n\r\n",
        )
        .expect("read");
        assert_eq!(body, b"abcde");
    }

    #[test]
    fn a_request_read_two_ways_is_refused() {
        assert!(matches!(
            read(b"POST / HTTP/1.1\r\nContent-Length: 3\r\nTransfer-Encoding: chunked\r\n\r\nabc"),
            Err(Unread::Malformed(_))
        ));
        assert!(matches!(
            read(b"POST / HTTP/1.1\r\nContent-Length: 3\r\nContent-Length: 4\r\n\r\nabcd"),
            Err(Unread::Malformed(_))
        ));
        assert!(matches!(
            read(b"POST / HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\n\r\n"),
            Err(Unread::Malformed(_))
        ));
        assert!(matches!(
            read(b"GET / HTTP/1.0\r\n\r\n"),
            Err(Unread::Malformed(_))
        ));
    }

    #[test]
    fn a_body_past_the_most_is_refused() {
        assert!(matches!(
            read(b"POST / HTTP/1.1\r\nContent-Length: 4096\r\n\r\n"),
            Err(Unread::TooLarge)
        ));
    }

    #[test]
    fn a_status_is_read_and_its_head_kept() {
        let mut stream = &b"rest"[..];
        let mut buffer = b"HTTP/1.1 401 Unauthorized\r\nX: y\r\n\r\nbody".to_vec();
        assert_eq!(read_status(&mut stream, &mut buffer).expect("read"), 401);
        assert!(buffer.starts_with(b"HTTP/1.1 401"));
    }

    #[test]
    fn an_answer_is_githubs_json_shape() {
        let mut out = Vec::new();
        write_answer(&mut out, 403, "refused \"x\"").expect("written");
        let text = String::from_utf8(out).expect("text");
        assert!(text.starts_with("HTTP/1.1 403 Forbidden\r\n"), "{text}");
        assert!(text.ends_with(r#"{"message":"refused \"x\""}"#), "{text}");
    }
}
