//! One connection: its request read, the door asked (door.rs), and then a
//! file sent (files.rs) or a feed opened (page.rs). The request's head is
//! peeked rather than read, so a feed's handshake, which tungstenite reads
//! itself, still finds it there. Each answer closes its connection: on
//! loopback a new one costs nothing worth keeping one open for. A file
//! goes with a tag of what it is served as, its real path, size, inode and
//! times, so a browser asking again with the tag it holds is told it
//! hasn't changed rather than sent the file once more.

use std::fs::{File, Metadata};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tungstenite::handshake::server::{ErrorResponse, Request, Response};

use crate::config::{Config, Feed};
use crate::door::Door;
use crate::files::{self, Found};
use crate::page;
use crate::upstream::Upstreams;

/// The most a request's head may hold.
const HEAD: usize = 16 * 1024;

/// The status of a file the browser already holds.
const NOT_MODIFIED: u16 = 304;

/// How long a request may take to arrive whole.
const PATIENCE: Duration = Duration::from_secs(10);

/// What the server holds for every connection.
pub struct Server {
    pub config: Config,
    pub door: Door,
    pub upstreams: Arc<Upstreams>,
}

/// A request's head, as much of it as the server reads.
struct Head {
    method: String,
    path: String,
    host: Option<String>,
    upgrade: bool,
    length: usize,
    /// The tags of the copies the browser holds, as `If-None-Match` lists them.
    held: Option<String>,
}

/// Answers one connection.
pub fn answer(server: &Server, mut stream: TcpStream) {
    if stream.set_read_timeout(Some(PATIENCE)).is_err() {
        return;
    }
    let head = match head(&stream) {
        Ok(Some(head)) => head,
        Ok(None) => {
            return send(
                &mut stream,
                400,
                "Bad Request",
                "text/plain",
                b"a request is HTTP/1.1\n",
            );
        }
        // It hung up or took too long, and is told nothing.
        Err(_) => return,
    };
    let hosted = head
        .host
        .as_deref()
        .is_some_and(|host| server.door.hosts(host));
    let feed = head
        .path
        .strip_prefix("/feed/")
        .filter(|_| hosted && head.upgrade);
    if let Some(name) = feed.and_then(|name| server.config.feeds.get_key_value(name)) {
        // The handshake reads the head it was peeked from.
        return open(server, stream, name);
    }
    // The head was peeked; it's read now, so the answer follows it, and a
    // connection closed with it unread isn't reset under the answer.
    let mut read = vec![0; head.length];
    if stream.read_exact(&mut read).is_err() {
        return;
    }
    if !hosted {
        send(
            &mut stream,
            403,
            "Forbidden",
            "text/plain",
            b"this server answers its own names alone\n",
        );
    } else if head.upgrade {
        send(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"a feed is at /feed/<name>, named in its config\n",
        );
    } else {
        serve_file(server, &mut stream, &head);
    }
}

fn open(server: &Server, stream: TcpStream, (name, feed): (&String, &Feed)) {
    let door = &server.door;
    #[expect(
        clippy::result_large_err,
        reason = "tungstenite's handshake callback answers this type"
    )]
    let admitted = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
        let origin = request
            .headers()
            .get("origin")
            .and_then(|origin| origin.to_str().ok());
        if origin.is_some_and(|origin| door.admits(origin)) {
            Ok(response)
        } else {
            let mut refused =
                ErrorResponse::new(Some("a feed is for this server's own pages\n".to_owned()));
            *refused.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(refused)
        }
    };
    if let Ok(socket) = tungstenite::accept_hdr(stream, admitted) {
        page::serve(socket, name, feed, &server.upstreams);
    }
}

fn serve_file(server: &Server, stream: &mut TcpStream, head: &Head) {
    if head.method != "GET" && head.method != "HEAD" {
        return send(
            stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"it serves GET and HEAD\n",
        );
    }
    let path = head.path.split(['?', '#']).next().unwrap_or("/");
    match files::find(&server.config, path) {
        Found::File(file) => send_file(stream, head, &file),
        Found::Folder(to) => {
            let told = Told {
                kind: "text/plain",
                body: b"",
                sent: true,
            };
            write_answer(
                stream,
                (301, "Moved Permanently"),
                &told,
                &[("Location", &to)],
            );
        }
        Found::Missing => send(stream, 404, "Not Found", "text/plain", b"nothing is here\n"),
        Found::Refused => send(
            stream,
            403,
            "Forbidden",
            "text/plain",
            b"that path is refused\n",
        ),
    }
}

/// Sends `file`, or says it hasn't changed where the browser holds its tag.
fn send_file(stream: &mut TcpStream, head: &Head, file: &Path) {
    let kind = files::kind_of(file);
    // One open gives both the tag and the body, so a file replaced between
    // the two can't go out under the other's tag.
    let Ok((mut opened, tag)) = opened(file, kind) else {
        return unreadable(stream);
    };
    let tagged = [("ETag", tag.as_str())];
    if head.held.as_deref().is_some_and(|held| holds(held, &tag)) {
        let told = Told {
            kind,
            body: b"",
            sent: false,
        };
        return write_answer(stream, (NOT_MODIFIED, "Not Modified"), &told, &tagged);
    }
    let mut body = Vec::new();
    if opened.read_to_end(&mut body).is_err() {
        return unreadable(stream);
    }
    let told = Told {
        kind,
        body: &body,
        sent: head.method != "HEAD",
    };
    write_answer(stream, (200, "OK"), &told, &tagged);
}

fn unreadable(stream: &mut TcpStream) {
    send(
        stream,
        500,
        "Internal Server Error",
        "text/plain",
        b"the file couldn't be read\n",
    );
}

/// Opens the file a path names, wherever its links lead, with its tag.
fn opened(file: &Path, kind: &str) -> io::Result<(File, String)> {
    let real = file.canonicalize()?;
    let opened = File::open(&real)?;
    let about = opened.metadata()?;
    Ok((opened, tag(&real, kind, &about)))
}

/// A file's tag: what it's served as; its real path, which for a store path
/// names its contents; and its size, inode and the times it was written and
/// changed, so a write moves it, as does a file put in its place.
fn tag(real: &Path, kind: &str, about: &Metadata) -> String {
    let mut hasher = DefaultHasher::new();
    (real, kind, about.len(), about.dev(), about.ino()).hash(&mut hasher);
    let times = (
        about.mtime(),
        about.mtime_nsec(),
        about.ctime(),
        about.ctime_nsec(),
    );
    times.hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}

/// Whether an `If-None-Match` list, `held`, names `tag`, weakly or by `*`.
fn holds(held: &str, tag: &str) -> bool {
    held.split(',')
        .map(|one| one.trim().trim_start_matches("W/"))
        .any(|one| one == tag || one == "*")
}

/// Peeks the request's head, answering none where it isn't HTTP.
fn head(stream: &TcpStream) -> io::Result<Option<Head>> {
    let mut buffer = vec![0; HEAD];
    loop {
        let peeked = stream.peek(&mut buffer)?;
        if peeked == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "it hung up"));
        }
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut request = httparse::Request::new(&mut headers);
        match request.parse(&buffer[..peeked]) {
            Ok(httparse::Status::Complete(length)) => {
                let header = |name: &str| {
                    request
                        .headers
                        .iter()
                        .find(|header| header.name.eq_ignore_ascii_case(name))
                        .and_then(|header| std::str::from_utf8(header.value).ok())
                };
                let upgrade = header("upgrade")
                    .is_some_and(|upgrade| upgrade.eq_ignore_ascii_case("websocket"));
                return Ok(Some(Head {
                    method: request.method.unwrap_or("").to_owned(),
                    path: request.path.unwrap_or("/").to_owned(),
                    host: header("host").map(str::to_owned),
                    upgrade,
                    length,
                    held: header("if-none-match").map(str::to_owned),
                }));
            }
            Ok(httparse::Status::Partial) if peeked < HEAD => {
                // More of the head is on its way; peek again once it's here.
                std::thread::sleep(Duration::from_millis(5));
            }
            _ => return Ok(None),
        }
    }
}

/// An answer's body, what it is, and whether it's sent or only measured,
/// as a HEAD asks.
struct Told<'a> {
    kind: &'a str,
    body: &'a [u8],
    sent: bool,
}

fn send(stream: &mut TcpStream, status: u16, reason: &str, kind: &str, body: &[u8]) {
    write_answer(
        stream,
        (status, reason),
        &Told {
            kind,
            body,
            sent: true,
        },
        &[],
    );
}

fn write_answer(
    stream: &mut TcpStream,
    (status, reason): (u16, &str),
    told: &Told,
    extra: &[(&str, &str)],
) {
    let (kind, length) = (told.kind, told.body.len());
    // A site changes while Joel reads it, so a reload asks again, and a
    // file unchanged since is answered by its tag alone.
    let mut answer = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Cache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n"
    );
    // An unchanged file's answer has no body, so it says neither a length
    // nor a type, which the browser keeps from the copy it holds.
    if status != NOT_MODIFIED {
        answer.extend(["Content-Type: ", kind, "\r\n"]);
        answer.extend(["Content-Length: ", &length.to_string(), "\r\n"]);
    }
    for (name, value) in extra {
        answer.extend([name, ": ", value, "\r\n"]);
    }
    answer.push_str("\r\n");
    // A reader that hung up while it was answered needs nothing more.
    let _ = stream.write_all(answer.as_bytes()).and_then(|()| {
        if told.sent {
            stream.write_all(told.body)
        } else {
            Ok(())
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: &str = "\"00000000000000aa\"";

    #[test]
    fn a_list_naming_the_tag_however_it_names_it_holds_it() {
        assert!(holds(TAG, TAG), "the tag alone");
        assert!(holds(&format!("W/{TAG}"), TAG), "the tag said weakly");
        assert!(holds(&format!("\"b\",  {TAG} "), TAG), "the tag in a list");
        assert!(holds("*", TAG), "any copy at all");
    }

    #[test]
    fn a_list_naming_another_tag_or_nothing_doesnt_hold_it() {
        assert!(!holds("\"00000000000000ab\"", TAG), "another copy");
        assert!(!holds("", TAG), "an empty list");
        assert!(
            !holds("00000000000000aa", TAG),
            "the tag without its quotes"
        );
        assert!(!holds(&format!("{TAG}x"), TAG), "a longer tag");
    }
}
