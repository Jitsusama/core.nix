//! An answer pressed on a page of the sites' own, as the connection hands
//! it over: its body read, at most what an answer holds, then checked and
//! carried by answer.rs and journalled; or the cookie its answers carry
//! given to the page that asks for it. Kept apart from the serving of
//! files and feeds, which never act.

use std::io::Read;
use std::net::TcpStream;

use serde_json::json;

use super::{Head, JOURNALLED, Server, Told, send, write_answer};
use crate::answer::{self, Answers, Asked};

/// Carries an answer pressed on a page of the sites' own (answer.rs), and
/// journals it, carried or refused with the word saying why, and that it
/// came by the sites; or, asked with GET, gives the person's own page the
/// cookie its answers carry.
pub(super) fn press(server: &Server, stream: &mut TcpStream, head: &Head) {
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
            answers.journal(&format!(
                "local-sites: refused an answer pressed on the sites ({}): {pressed:?}: {}",
                refused.word, refused.why
            ));
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
            answers.journal(&format!(
                "local-sites: gave no page the answers' cookie ({}): {}",
                refused.word, refused.why
            ));
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

/// The reason phrase an answer's refusal is sent with, by its status; one
/// not named here is said only as refused, so it never reads as a daemon's
/// failure it wasn't.
const fn reason_for(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        502 => "Bad Gateway",
        _ => "Refused",
    }
}

#[cfg(test)]
mod tests {
    use regex_lite::Regex;

    use super::reason_for;

    #[test]
    fn every_status_an_answer_is_refused_with_has_its_own_phrase() {
        let refusing = Regex::new(r#"refused\(\s*"[a-z]+",\s*(\d{3}),"#).expect("a pattern");
        let statuses: Vec<u16> = refusing
            .captures_iter(include_str!("../answer.rs"))
            .map(|found| found[1].parse().expect("a status"))
            .collect();
        assert!(statuses.len() > 20, "answer.rs's refusals weren't found");
        for status in statuses {
            assert_ne!(
                reason_for(status),
                "Refused",
                "{status} has no phrase of its own"
            );
        }
        assert_eq!(
            reason_for(503),
            "Refused",
            "an unnamed status read as another"
        );
    }
}
