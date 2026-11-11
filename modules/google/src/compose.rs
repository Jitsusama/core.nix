//! Gmail's links to write a message: a reply to a mail, and a message to
//! a meeting's guests, as Gmail's own link to compose takes them, which
//! the shell opens in Gmail's window of its own.

use std::fmt::Write;

/// Gmail, where a new message is written in a window of its own.
const COMPOSE: &str = "https://mail.google.com/mail/u/0/";

/// A reply's subject, marked a reply once, as mail programs write it.
pub(crate) fn answering(subject: &str) -> String {
    let replied = subject
        .get(..3)
        .is_some_and(|start| start.eq_ignore_ascii_case("re:"));
    if replied {
        subject.to_owned()
    } else {
        format!("Re: {subject}")
    }
}

/// Where a new message to these addresses is written, about this subject
/// and copied to those, as Gmail's own link to compose takes them, each
/// address once; nothing with nobody to write to.
pub(crate) fn writing(to: &[&str], subject: &str, copied: &[&str]) -> String {
    let mut addresses: Vec<&str> = Vec::new();
    for address in to {
        if !address.is_empty() && !addresses.contains(address) {
            addresses.push(address);
        }
    }
    if addresses.is_empty() {
        return String::new();
    }
    let mut asked = vec![
        ("view", "cm".to_owned()),
        ("fs", "1".to_owned()),
        ("to", addresses.join(",")),
    ];
    let mut cc: Vec<&str> = Vec::new();
    for address in copied {
        if !address.is_empty() && !addresses.contains(address) && !cc.contains(address) {
            cc.push(address);
        }
    }
    if !cc.is_empty() {
        asked.push(("cc", cc.join(",")));
    }
    if !subject.is_empty() {
        asked.push(("su", subject.to_owned()));
    }
    let query: Vec<String> = asked
        .iter()
        .map(|(name, value)| format!("{name}={}", encoded(value)))
        .collect();
    format!("{COMPOSE}?{}", query.join("&"))
}

/// A link's part as JavaScript's encodeURIComponent writes it, which
/// Mail.js's links were.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            // Writing to a String never fails.
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}
