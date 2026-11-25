//! One connection: its request read, the door asked (door.rs), and then a
//! file sent (files.rs) or a feed opened (page.rs). The request's head is
//! peeked rather than read, so a feed's handshake, which tungstenite reads
//! itself, still finds it there. Each answer closes its connection: on
//! loopback a new one costs nothing worth keeping one open for. A file
//! goes with a tag of what it is served as, its real path, size, inode and
//! times, so a browser asking again with the tag it holds is told it
//! hasn't changed rather than sent the file once more.
//!
//! The documents share the sites' origin, and a session writes them, so
//! what a session's page runs must never act as one of the sites' own, which
//! read Joel's feeds and will answer what he's asked. Each document is
//! served sandboxed, its script running in an origin of no one's (an opaque
//! one), which no feed admits and no answer takes; the sites' own pages,
//! which Nix or the home's builder made, run only the scripts served beside
//! them, never one written into the page, so words a session wrote into a
//! record they draw can't run there either. No page may install a worker,
//! which would stand between every later page and the server, and one a
//! document installed before is swept out once by the first page of the
//! sites' own that a browser is served. Kit code and fonts are shared with
//! a sandboxed document, whose reads of them are another origin's; so is a
//! document with one beside it, but only where the page asking is one this
//! server served.

use std::fs::{File, Metadata};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};

use crate::answer::{self, Answers, Asked};
use crate::config::{Config, Feed};
use crate::door::Door;
use crate::files::{self, Found};
use crate::page;
use crate::upstream::Upstreams;

/// The most a request's head may hold.
const HEAD: usize = 16 * 1024;

/// The status of a file the browser already holds.
const NOT_MODIFIED: u16 = 304;

/// How much of an answer pressed on a page the journal keeps.
const JOURNALLED: usize = 256;

/// How long a request may take to arrive whole.
const PATIENCE: Duration = Duration::from_secs(10);

/// The sandbox every document runs in: its scripts, its forms and the
/// windows it opens, each in an origin of no one's, never this server's,
/// and no way out of it to the page holding it or a window it opens; and
/// held in a frame only by a page served here.
const SANDBOXED: &str = "sandbox allow-scripts allow-popups allow-forms; frame-ancestors 'self'";

/// What the sites' own pages may run and be held in: only scripts served
/// from here, none written into the page or evaluated from words, no
/// plugin, no base moving its links, and no frame around it to press its
/// buttons through.
const OWN: &str = "script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

/// The cookie saying a browser's storage was swept of what a document could
/// have left there, so it's swept once.
const SWEPT: &str = "sites-swept=1";

/// The sweep's cookie as it's set: for every path, a year, sent to no
/// other site and read by no script.
const SWEEPING: &str = "sites-swept=1; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly";

/// What the answers a file is sent with are, by version, hashed into its
/// tag, so a copy a browser holds from before the answers it was sent with
/// changed is sent again whole, with them, rather than kept under the old.
const ANSWERS: u8 = 2;

/// What the server holds for every connection.
pub struct Server {
    pub config: Config,
    pub door: Door,
    pub upstreams: Arc<Upstreams>,
    /// Where an answer pressed on a page is checked and sent, where any is.
    pub answers: Option<Answers>,
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
    /// The page asking, as `Origin` says it, where it says one.
    origin: Option<String>,
    /// The page the request came from, as `Referer` says it.
    referer: Option<String>,
    /// Its cookies, as `Cookie` lists them.
    cookies: Option<String>,
    /// Whether the browser asks for a worker's script, as `Service-Worker` says.
    worker: bool,
    /// Where the browser says the request came from, as `Sec-Fetch-Site` says.
    fetch_site: Option<String>,
    /// How long its body is, as `Content-Length` says, where it says.
    body_length: Option<usize>,
    /// What its body is, as `Content-Type` says.
    content_type: Option<String>,
    /// The connection's ends, the asker's then the server's.
    ends: Option<(SocketAddr, SocketAddr)>,
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
    } else if head.worker {
        send(
            &mut stream,
            403,
            "Forbidden",
            "text/plain",
            b"no page installs a worker here\n",
        );
    } else if head.upgrade {
        send(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"a feed is at /feed/<name>, named in its config\n",
        );
    } else if head.path.split(['?', '#']).next() == Some("/answer") {
        press(server, &mut stream, &head);
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
        Found::File(file, made) => {
            let told = told_for(server, head, &file, made);
            send_file(stream, head, &file, &told);
        }
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
        Found::Unready => send(
            stream,
            503,
            "Service Unavailable",
            "text/plain",
            b"the home is still being written\n",
        ),
    }
}

/// What a file is sent with beside its tag, by who made it.
struct Guarded {
    /// Its answers' own headers, sent with it whole or not.
    headers: Vec<(&'static str, String)>,
    /// Whether it must go whole, since what it's sent with does what a
    /// browser would skip for a copy it keeps.
    whole: bool,
}

/// What `file`, which `made` made, is sent with to `head`.
fn told_for(server: &Server, head: &Head, file: &Path, made: files::Made) -> Guarded {
    let door = &server.door;
    let kind = files::kind_of(file);
    let mut headers = Vec::new();
    let mut whole = false;
    match made {
        files::Made::Written => {
            headers.push(("Content-Security-Policy", SANDBOXED.to_owned()));
            // A sandboxed document reads what sits beside it as another
            // origin, which only a page served here is let do.
            if head.origin.as_deref() == Some("null") && served_here(door, head) {
                headers.push(("Access-Control-Allow-Origin", "null".to_owned()));
                headers.push(("Vary", "Origin, Referer".to_owned()));
            }
        }
        files::Made::Built => {
            headers.push(("Content-Security-Policy", OWN.to_owned()));
            if shared(kind) {
                headers.push(("Access-Control-Allow-Origin", "*".to_owned()));
            }
            if kind.starts_with("text/html") && !swept(head) {
                headers.push(("Clear-Site-Data", "\"storage\"".to_owned()));
                headers.push(("Set-Cookie", SWEEPING.to_owned()));
                whole = true;
            }
        }
    }
    Guarded { headers, whole }
}

/// Carries an answer pressed on a page of the sites' own (answer.rs), and
/// journals it, carried or refused with the word saying why, and that it
/// came by the sites; or, asked with GET, gives the person's own page the
/// cookie its answers carry.
fn press(server: &Server, stream: &mut TcpStream, head: &Head) {
    let Some(answers) = &server.answers else {
        return send(
            stream,
            404,
            "Not Found",
            "text/plain",
            b"this server takes no answers\n",
        );
    };
    if head.method == "GET" {
        return given(answers, stream, head);
    }
    if head.method != "POST" {
        return send(
            stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"an answer is posted\n",
        );
    }
    let Some(length) = head.body_length.filter(|length| *length <= answer::LONGEST) else {
        return send(
            stream,
            413,
            "Content Too Large",
            "text/plain",
            b"an answer is a look and a value\n",
        );
    };
    let mut body = vec![0; length];
    if stream.read_exact(&mut body).is_err() {
        return;
    }
    let asked = Asked {
        host: head.host.as_deref().unwrap_or(""),
        origin: head.origin.as_deref(),
        fetch_site: head.fetch_site.as_deref(),
        cookies: head.cookies.as_deref(),
        content_type: head.content_type.as_deref(),
        ends: head.ends,
        body: &body,
    };
    // Only so much of what was pressed is journalled, so a refused one
    // can't fill the journal.
    let pressed = String::from_utf8_lossy(&body[..body.len().min(JOURNALLED)]);
    match answers.carry(&asked) {
        Ok(told) => {
            eprintln!("local-sites: carried an answer pressed on the sites: {pressed:?}");
            send(
                stream,
                200,
                "OK",
                "application/json",
                told.to_string().as_bytes(),
            );
        }
        Err(refused) => {
            eprintln!(
                "local-sites: refused an answer pressed on the sites ({}): {pressed:?}: {}",
                refused.word, refused.why
            );
            let said = json!({"refused": refused.word, "why": refused.why});
            send(
                stream,
                refused.status,
                reason_for(refused.status),
                "application/json",
                said.to_string().as_bytes(),
            );
        }
    }
}

/// Gives the page asking the cookie its answers carry, where the person's
/// own process asks from this origin, saying why not otherwise.
fn given(answers: &Answers, stream: &mut TcpStream, head: &Head) {
    let asking = Asked {
        host: head.host.as_deref().unwrap_or(""),
        origin: head.origin.as_deref(),
        fetch_site: head.fetch_site.as_deref(),
        cookies: None,
        content_type: None,
        ends: head.ends,
        body: &[],
    };
    match answers.cookie_for(&asking) {
        Ok(cookie) => write_answer(
            stream,
            (204, "No Content"),
            &Told {
                kind: "text/plain",
                body: b"",
                sent: true,
            },
            &[("Set-Cookie", &cookie)],
        ),
        Err(refused) => {
            eprintln!(
                "local-sites: gave no page the answers' cookie ({}): {}",
                refused.word, refused.why
            );
            let said = json!({"refused": refused.word, "why": refused.why});
            let reason = reason_for(refused.status);
            let body = said.to_string();
            send(
                stream,
                refused.status,
                reason,
                "application/json",
                body.as_bytes(),
            );
        }
    }
}

/// The reason phrase an answer's refusal is sent with, by its status.
const fn reason_for(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        415 => "Unsupported Media Type",
        _ => "Bad Gateway",
    }
}

/// Whether a file of `kind` is code or type every page may share: the
/// kit's scripts and styles, and the fonts, which a sandboxed document
/// reads as another origin's.
fn shared(kind: &str) -> bool {
    kind.starts_with("text/javascript") || kind.starts_with("text/css") || kind.starts_with("font/")
}

/// Whether the page `head` came from is one this server served, as its
/// `Referer` says, which a browser writes and no page can.
fn served_here(door: &Door, head: &Head) -> bool {
    head.referer
        .as_deref()
        .and_then(|referer| referer.strip_prefix("http://"))
        .is_some_and(|rest| door.hosts(rest.split(['/', '?', '#']).next().unwrap_or("")))
}

/// Whether the browser asking has been swept already, by its cookie.
fn swept(head: &Head) -> bool {
    head.cookies
        .as_deref()
        .is_some_and(|cookies| cookies.split(';').any(|cookie| cookie.trim() == SWEPT))
}

/// Sends `file`, or says it hasn't changed where the browser holds its tag.
fn send_file(stream: &mut TcpStream, head: &Head, file: &Path, guarded: &Guarded) {
    let kind = files::kind_of(file);
    // One open gives both the tag and the body, so a file replaced between
    // the two can't go out under the other's tag.
    let Ok((mut opened, tag)) = opened(file, kind) else {
        return unreadable(stream);
    };
    let mut tagged = vec![("ETag", tag.as_str())];
    tagged.extend(
        guarded
            .headers
            .iter()
            .map(|(name, value)| (*name, value.as_str())),
    );
    if !guarded.whole && head.held.as_deref().is_some_and(|held| holds(held, &tag)) {
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
    (ANSWERS, real, kind, about.len(), about.dev(), about.ino()).hash(&mut hasher);
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
                    origin: header("origin").map(str::to_owned),
                    referer: header("referer").map(str::to_owned),
                    cookies: header("cookie").map(str::to_owned),
                    worker: header("service-worker").is_some(),
                    fetch_site: header("sec-fetch-site").map(str::to_owned),
                    body_length: header("content-length")
                        .and_then(|length| length.trim().parse().ok()),
                    content_type: header("content-type").map(str::to_owned),
                    ends: stream.peer_addr().ok().zip(stream.local_addr().ok()),
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
