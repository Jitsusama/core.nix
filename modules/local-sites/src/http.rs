//! One connection: its request read, the door asked (door.rs), and then a
//! file sent (files.rs) or a feed opened (page.rs). The request's head is
//! peeked rather than read, so a feed's handshake, which tungstenite reads
//! itself, still finds it there. Each answer closes its connection: on
//! loopback a new one costs nothing worth keeping one open for.

use std::io::{self, Read, Write};
use std::net::TcpStream;
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
        Found::File(file) => match std::fs::read(&file) {
            Ok(body) => {
                let told = Told {
                    kind: files::kind_of(&file),
                    body: &body,
                    sent: head.method != "HEAD",
                };
                write_answer(stream, (200, "OK"), &told, &[]);
            }
            Err(_) => send(
                stream,
                500,
                "Internal Server Error",
                "text/plain",
                b"the file couldn't be read\n",
            ),
        },
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
    // A site changes while Joel reads it, so a reload shows the change.
    let mut answer = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {length}\r\n\
         Cache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n"
    );
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
