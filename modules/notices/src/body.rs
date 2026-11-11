//! What a notice's body says, read once as its row is made, rather than
//! by the shell for each list it shows in (modules/quickshell/`NoticeBody.js`,
//! whose cases these are): as plain text, for a row's note, a search and a
//! copy; as styled text a drawer draws, carrying only the markup the
//! notification spec allows (bold, italics, underline and links) with
//! everything else escaped, so a sender can't draw what the desktop
//! doesn't; and its links, each its words and where it goes. A bare
//! address in the words is a link too, since chat programs send their
//! links that way.

use serde::Serialize;
// Writing into a String can't fail, so what write! says is let go.
use std::fmt::Write;

/// A link in what a notice said: its words and where it goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Link {
    pub text: String,
    pub href: String,
}

/// A body read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Body {
    pub plain: String,
    pub styled: String,
    pub links: Vec<Link>,
}

/// The spec's tags a body may carry, and the line break the styled text
/// draws; any other `<` is the sender's own words.
const TAGS: [&str; 6] = ["b", "i", "u", "a", "br", "img"];

/// The entities a sender may write by name.
const NAMED: [(&str, &str); 6] = [
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", "\u{a0}"),
];

/// The punctuation that closes a sentence round a bare address, taken
/// off its end.
const CLOSING: &[char] = &['.', ',', ';', ':', '!', '?', ')', ']'];

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `text` starts with `prefix`, in any case.
fn starts_folded(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// A tag at the start of `text`: whether it closes, its name in lower
/// case, its attributes, and how long it is; none where `text` doesn't
/// start with one of the spec's tags.
fn tag_at(text: &str) -> Option<(bool, String, &str, usize)> {
    let rest = text.strip_prefix('<')?.trim_start();
    let (closing, rest) = match rest.strip_prefix('/') {
        Some(after) => (true, after.trim_start()),
        None => (false, rest),
    };
    let name_ends = rest.find(|c: char| !is_word(c)).unwrap_or(rest.len());
    let name = rest[..name_ends].to_ascii_lowercase();
    if !TAGS.contains(&name.as_str()) {
        return None;
    }
    let after = &rest[name_ends..];
    let ends = after.find('>')?;
    let length = text.len() - after.len() + ends + 1;
    Some((closing, name, &after[..ends], length))
}

/// The character an entity at the start of `text` names, and how long it
/// is; none where it isn't one, or names no character.
fn entity_at(text: &str) -> Option<(String, usize)> {
    let rest = text.strip_prefix('&')?;
    let ends = rest.find(';')?;
    let name = &rest[..ends];
    let length = ends + 2;
    if let Some(number) = name.strip_prefix('#') {
        let (digits, radix) = match number.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16),
            None => (number, 10),
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return None;
        }
        let code = u32::from_str_radix(digits, radix).ok()?;
        return char::from_u32(code)
            .filter(|_| code > 0)
            .map(|c| (c.to_string(), length));
    }
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    NAMED
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|(_, said)| ((*said).to_owned(), length))
}

fn decoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = text[at..].find('&') {
        let from = at + found;
        out.push_str(&text[at..from]);
        if let Some((said, length)) = entity_at(&text[from..]) {
            out.push_str(&said);
            at = from + length;
        } else {
            out.push('&');
            at = from + 1;
        }
    }
    out.push_str(&text[at..]);
    out
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// An attribute's value, quoted either way or bare, decoded; "" where the
/// attributes don't name it.
fn attribute(attributes: &str, name: &str) -> String {
    let mut from = 0;
    while let Some(found) = attributes[from..]
        .to_ascii_lowercase()
        .find(&name.to_ascii_lowercase())
    {
        let at = from + found;
        from = at + name.len();
        let whole_word = attributes[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word(c));
        let Some(rest) = attributes[from..].trim_start().strip_prefix('=') else {
            continue;
        };
        if !whole_word {
            continue;
        }
        let rest = rest.trim_start();
        // Quoted either way where the quote closes, else bare up to a
        // space, the quote it opened with its words.
        let quoted = ['"', '\''].into_iter().find_map(|quote| {
            let inside = rest.strip_prefix(quote)?;
            inside.find(quote).map(|ends| &inside[..ends])
        });
        let bare = || {
            let ends = rest
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(rest.len());
            (ends > 0).then(|| &rest[..ends])
        };
        if let Some(value) = quoted.or_else(bare) {
            return decoded(value);
        }
    }
    String::new()
}

/// Whether a link may go where it says: the web, or mail. Anything else,
/// such as a script's `javascript:` or a file, is said as words and never
/// opened.
fn goes(href: &str) -> bool {
    starts_folded(href, "http://")
        || starts_folded(href, "https://")
        || starts_folded(href, "mailto:")
}

/// Where a bare address starts in `text` at or after `from`, and where it
/// ends before the punctuation closing a sentence round it.
fn bare_at(text: &str, from: usize) -> Option<(usize, usize)> {
    let mut at = from;
    while let Some(found) = text[at..].char_indices().find(|(index, _)| {
        let rest = &text[at + index..];
        starts_folded(rest, "http://") || starts_folded(rest, "https://")
    }) {
        let start = at + found.0;
        let scheme = if starts_folded(&text[start..], "https://") {
            8
        } else {
            7
        };
        let boundary = text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word(c));
        let rest = &text[start + scheme..];
        let length = rest
            .find(|c: char| c.is_whitespace() || "<>\"'".contains(c))
            .unwrap_or(rest.len());
        if boundary && length > 0 {
            let whole = &text[start..start + scheme + length];
            let kept = whole.trim_end_matches(CLOSING);
            return Some((start, start + kept.len()));
        }
        at = start + 1;
    }
    None
}

/// Plain words as styled text, each bare address in them a link, which
/// `links` gains.
fn linked(text: &str, links: &mut Vec<Link>) -> String {
    let mut out = String::new();
    let mut from = 0;
    while let Some((start, end)) = bare_at(text, from) {
        let href = &text[start..end];
        out.push_str(&escaped(&text[from..start]));
        links.push(Link {
            text: href.to_owned(),
            href: href.to_owned(),
        });
        let _ = write!(out, "<a href=\"{0}\">{0}</a>", escaped(href));
        from = end;
    }
    out + &escaped(&text[from..])
}

/// What's read so far: the plain words, the styled text, the links, the
/// tags open, and the link being read, its words gathered until it
/// closes.
#[derive(Default)]
struct Reading {
    plain: String,
    styled: String,
    links: Vec<Link>,
    open: Vec<String>,
    link: Option<Link>,
}

impl Reading {
    fn words(&mut self, text: &str) {
        let said = decoded(text);
        self.plain.push_str(&said);
        if let Some(link) = &mut self.link {
            link.text.push_str(&said);
            self.styled.push_str(&escaped(&said));
        } else {
            let styled = linked(&said, &mut self.links);
            self.styled.push_str(&styled);
        }
    }

    /// A link ends, kept where it goes somewhere it may.
    fn link_ends(&mut self) {
        if let Some(link) = self.link.take()
            && !link.href.is_empty()
        {
            self.styled.push_str("</a>");
            let text = link.text.trim();
            self.links.push(Link {
                text: if text.is_empty() {
                    link.href.clone()
                } else {
                    text.to_owned()
                },
                href: link.href,
            });
        }
    }

    fn anchor(&mut self, closing: bool, attributes: &str) {
        if !closing && self.link.is_none() {
            let href = attribute(attributes, "href");
            let href = if goes(&href) { href } else { String::new() };
            if !href.is_empty() {
                let _ = write!(self.styled, "<a href=\"{}\">", escaped(&href));
            }
            self.link = Some(Link {
                text: String::new(),
                href,
            });
        } else if closing && self.link.is_some() {
            self.link_ends();
        }
    }

    /// Closed in order, as styled text needs, those opened inside it
    /// closing with it; one closed that never opened is dropped.
    fn closes(&mut self, tag: &str) {
        if !self.open.iter().any(|open| open == tag) {
            return;
        }
        while let Some(last) = self.open.pop() {
            let _ = write!(self.styled, "</{last}>");
            if last == tag {
                break;
            }
        }
    }

    fn tagged(&mut self, closing: bool, tag: &str, attributes: &str) {
        match tag {
            "br" => {
                self.plain.push('\n');
                self.styled.push_str("<br>");
            }
            // A picture in the words is said by what it shows, never
            // fetched, since the desktop draws nothing it wasn't handed.
            "img" => self.words(&attribute(attributes, "alt")),
            "a" => self.anchor(closing, attributes),
            _ if closing => self.closes(tag),
            _ => {
                let _ = write!(self.styled, "<{tag}>");
                self.open.push(tag.to_owned());
            }
        }
    }
}

/// The body read, its links in the order they come. A tag left open is
/// closed at the end, so the styled text is always whole; plain text
/// keeps the sender's own line breaks, and styled text says them as
/// breaks, which it draws.
pub fn read(body: &str) -> Body {
    let mut state = Reading::default();
    let mut from = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find('<') {
        let start = at + found;
        match tag_at(&body[start..]) {
            Some((closing, tag, attributes, length)) => {
                state.words(&body[from..start]);
                state.tagged(closing, &tag, attributes);
                from = start + length;
                at = from;
            }
            None => at = start + 1,
        }
    }
    state.words(&body[from..]);
    state.link_ends();
    while let Some(last) = state.open.pop() {
        let _ = write!(state.styled, "</{last}>");
    }
    Body {
        plain: state.plain,
        styled: state.styled.replace('\n', "<br>"),
        links: state.links,
    }
}

/// Whether what a notice said carries markup or a link of its own, which
/// its drawer draws styled, rather than plain, so a long log keeps its
/// own lines.
pub fn marked(body: &Body) -> bool {
    ["<a ", "<b>", "<i>", "<u>"]
        .iter()
        .any(|tag| body.styled.contains(tag))
}

/// `%` escapes made bytes again, as `decodeURIComponent` does, or none
/// where one is malformed or the bytes aren't UTF-8.
fn unescaped(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let pair = text.get(at + 1..at + 3)?;
            if !pair.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(pair, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Where a link goes, said short: its host, without a leading www, or the
/// address of a mail, as it was written where its escapes don't read.
pub fn host(href: &str) -> String {
    if starts_folded(href, "mailto:") {
        let address = href["mailto:".len()..].split('?').next().unwrap_or("");
        return unescaped(address).unwrap_or_else(|| address.to_owned());
    }
    let scheme = href
        .find("://")
        .filter(|&at| at > 0 && href[..at].chars().all(|c| c.is_ascii_alphabetic()));
    let Some(at) = scheme else {
        return href.to_owned();
    };
    let rest = &href[at + 3..];
    let ends = rest.find(['/', '?', '#', ':']).unwrap_or(rest.len());
    if ends == 0 {
        return href.to_owned();
    }
    let name = &rest[..ends];
    let name = if starts_folded(name, "www.") {
        &name[4..]
    } else {
        name
    };
    name.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(text: &str, href: &str) -> Link {
        Link {
            text: text.to_owned(),
            href: href.to_owned(),
        }
    }

    fn body(plain: &str, styled: &str, links: Vec<Link>) -> Body {
        Body {
            plain: plain.to_owned(),
            styled: styled.to_owned(),
            links,
        }
    }

    #[test]
    fn words_and_markup_read_as_the_spec_allows() {
        assert_eq!(
            read("Lunch at noon?"),
            body("Lunch at noon?", "Lunch at noon?", vec![])
        );
        assert_eq!(
            read("<b>Build</b> passed &amp; merged"),
            body(
                "Build passed & merged",
                "<b>Build</b> passed &amp; merged",
                vec![]
            )
        );
        assert_eq!(
            read("<font color=\"red\">hi</font> a < b"),
            body(
                "<font color=\"red\">hi</font> a < b",
                "&lt;font color=&quot;red&quot;&gt;hi&lt;/font&gt; a &lt; b",
                vec![]
            ),
            "other tags are escaped"
        );
        assert_eq!(
            read("<img src=\"https://x/y.png\" alt=\"a cat\">"),
            body("a cat", "a cat", vec![]),
            "a picture is said by its words"
        );
        assert_eq!(
            read("one\ntwo<br/>three"),
            body("one\ntwo\nthree", "one<br>two<br>three", vec![])
        );
        assert_eq!(
            read("<b><i>loud</b> quiet</u>").styled,
            "<b><i>loud</i></b> quiet",
            "tags are made whole"
        );
        assert_eq!(read(""), Body::default());
        assert_eq!(
            read("&#x1F600; &#0; &bogus; &").plain,
            "\u{1f600} &#0; &bogus; &"
        );
    }

    #[test]
    fn links_are_kept_where_they_may_go() {
        let href = "https://github.com/x/y/pull/3";
        assert_eq!(
            read(&format!("See <a href=\"{href}\">#3</a>")),
            body(
                "See #3",
                &format!("See <a href=\"{href}\">#3</a>"),
                vec![link("#3", href)]
            )
        );
        let two =
            read("<a href='https://a.example/1'>one</a> and <a href=https://b.example/2>two</a>");
        let hrefs: Vec<&str> = two.links.iter().map(|link| link.href.as_str()).collect();
        assert_eq!(
            hrefs,
            ["https://a.example/1", "https://b.example/2"],
            "in order"
        );
        let docs = "https://docs.google.com/d/abc";
        assert_eq!(
            read(&format!("Docs at {docs}.")),
            body(
                &format!("Docs at {docs}."),
                &format!("Docs at <a href=\"{docs}\">{docs}</a>."),
                vec![link(docs, docs)]
            ),
            "a bare address is a link, without the sentence's stop"
        );
        assert_eq!(
            read("<a href=\"javascript:alert(1)\">x</a>"),
            body("x", "x", vec![]),
            "a script is no link"
        );
        assert!(marked(&read("<b>a</b>")), "markup is drawn styled");
        assert!(!marked(&read("a < b")), "escaped words aren't");
    }

    #[test]
    fn a_link_is_said_by_its_host() {
        assert_eq!(host("https://www.GitHub.com/x?y=1"), "github.com");
        assert_eq!(
            host("mailto:joel%40example.com?subject=hi"),
            "joel@example.com"
        );
        assert_eq!(
            host("mailto:100%off@example.com"),
            "100%off@example.com",
            "a malformed mail as written"
        );
        assert_eq!(host("nowhere"), "nowhere");
    }
}
