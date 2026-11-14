//! The types a copy is offered as: which to offer for what's copied,
//! which to read for what's asked, and which mark a copy as secret.

use crate::wire::Wanted;

/// Text's types, in the order a paste prefers them: the first an offer
/// has wins, and a copy of text is offered as all of them, as wl-copy
/// offers it.
pub const TEXT: [&str; 5] = [
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];

/// The type offered for bytes that are neither text nor a picture.
const BYTES: &str = "application/octet-stream";

/// What marks a copy as a password manager's secret, as a type's name
/// holds it, case aside: KDE's hint, which `KeePassXC`, Bitwarden and the
/// passwords module set, under its own name and the `text/` one
/// `KeePassXC` proposed, and the macOS pasteboard's concealed type, which password
/// managers there set and a cross-platform one may carry here.
const SECRET_MARKS: [&str; 2] = ["passwordmanagerhint", "org.nspasteboard.concealedtype"];

/// The type Windows' clipboard monitors are told to pass a copy over by,
/// which a cross-platform password manager may set on every system.
const SECRET_KINDS: [&str; 1] = ["excludeclipboardcontentfrommonitorprocessing"];

/// The longest type a journal line says, past which it says `other`.
const LONGEST_SAID: usize = 100;

/// Whether a copy offered as `types` is a password manager's secret.
pub fn secret(types: &[String]) -> bool {
    types.iter().any(|kind| {
        let kind = kind.to_ascii_lowercase();
        SECRET_MARKS.iter().any(|mark| kind.contains(mark)) || SECRET_KINDS.contains(&kind.as_str())
    })
}

/// Whether a type is text, as wl-clipboard counts it.
pub fn is_text(kind: &str) -> bool {
    kind.starts_with("text/") || TEXT.contains(&kind)
}

/// The type of `types` to read for what's `wanted`, or none when none
/// will do.
pub fn chosen<'a>(types: &'a [String], wanted: &Wanted) -> Option<&'a str> {
    let first_text = || {
        TEXT.iter()
            .find_map(|text| types.iter().find(|kind| kind == text))
            .or_else(|| types.iter().find(|kind| kind.starts_with("text/")))
    };
    let found = match wanted {
        Wanted::Any => first_text().or_else(|| types.first()),
        Wanted::Kind(kind) if kind == "text" => first_text(),
        Wanted::Kind(kind) if kind.contains('/') => types.iter().find(|offered| *offered == kind),
        Wanted::Kind(family) => types.iter().find(|offered| {
            offered
                .split_once('/')
                .is_some_and(|(top, _)| top == family)
        }),
    };
    found.map(String::as_str)
}

/// The types a copy of `bytes` is offered as: `kind`, with text's other
/// types beside it when it's text, or else what the bytes look like.
pub fn offered(kind: Option<&str>, bytes: &[u8]) -> Vec<String> {
    let texts = |first: Option<&str>| -> Vec<String> {
        first
            .into_iter()
            .chain(TEXT.into_iter().filter(|text| Some(*text) != first))
            .map(str::to_owned)
            .collect()
    };
    match kind {
        Some("text") => texts(None),
        Some(kind) if is_text(kind) => texts(Some(kind)),
        Some(kind) => vec![kind.to_owned()],
        None => match picture(bytes) {
            Some(kind) => vec![kind.to_owned()],
            None if std::str::from_utf8(bytes).is_ok() => texts(None),
            None => vec![BYTES.to_owned()],
        },
    }
}

/// The picture type `bytes` begin as, of those a desktop pastes.
fn picture(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// A type as a journal line may say it, or `other` when it holds what a
/// field shouldn't.
pub fn sayable(kind: &str) -> &str {
    let plain = !kind.is_empty()
        && kind.len() <= LONGEST_SAID
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/+-.;=_".contains(&byte));
    if plain { kind } else { "other" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn each_password_managers_mark_is_secret() {
        for mark in [
            "x-kde-passwordManagerHint",
            "text/x-kde-passwordManagerHint",
            "org.nspasteboard.ConcealedType",
            "ExcludeClipboardContentFromMonitorProcessing",
        ] {
            assert!(secret(&types(&["text/plain", mark])), "{mark}");
        }
        assert!(
            !secret(&types(&["text/plain", "UTF8_STRING", "image/png"])),
            "plain text and an image were taken for a secret"
        );
    }

    #[test]
    fn a_paste_prefers_utf8_text() {
        let offered = types(&["image/png", "STRING", "text/plain;charset=utf-8"]);
        assert_eq!(
            chosen(&offered, &Wanted::Any),
            Some("text/plain;charset=utf-8")
        );
        assert_eq!(
            chosen(&offered, &Wanted::Kind("text".into())),
            Some("text/plain;charset=utf-8")
        );
    }

    #[test]
    fn a_paste_takes_the_first_type_when_none_is_text() {
        let offered = types(&["image/webp", "image/png"]);
        assert_eq!(chosen(&offered, &Wanted::Any), Some("image/webp"));
        assert_eq!(chosen(&offered, &Wanted::Kind("text".into())), None);
    }

    #[test]
    fn a_type_asked_is_matched_whole_or_by_family() {
        let offered = types(&["text/html", "text/plain", "image/png"]);
        assert_eq!(
            chosen(&offered, &Wanted::Kind("image/png".into())),
            Some("image/png")
        );
        assert_eq!(
            chosen(&offered, &Wanted::Kind("image".into())),
            Some("image/png")
        );
        assert_eq!(
            chosen(&offered, &Wanted::Kind("text".into())),
            Some("text/plain")
        );
        assert_eq!(chosen(&offered, &Wanted::Kind("image/jpeg".into())), None);
        assert_eq!(chosen(&offered, &Wanted::Kind("audio".into())), None);
        assert_eq!(chosen(&[], &Wanted::Any), None);
    }

    #[test]
    fn text_is_offered_as_every_text_type() {
        let all = types(&TEXT);
        assert_eq!(offered(None, b"hello"), all);
        assert_eq!(offered(Some("text"), b"\xff"), all);
        let html = offered(Some("text/html"), b"<b>");
        assert_eq!(html[0], "text/html");
        assert_eq!(&html[1..], all.as_slice());
        let plain = offered(Some("text/plain"), b"a");
        assert_eq!(plain.len(), TEXT.len());
        assert_eq!(plain[0], "text/plain");
    }

    #[test]
    fn a_picture_or_bytes_are_offered_as_what_they_are() {
        assert_eq!(
            offered(None, b"\x89PNG\r\n\x1a\nrest"),
            types(&["image/png"])
        );
        assert_eq!(offered(None, b"\xff\xd8\xff\xe0"), types(&["image/jpeg"]));
        assert_eq!(offered(None, b"GIF89a"), types(&["image/gif"]));
        assert_eq!(
            offered(None, b"RIFF\0\0\0\0WEBPVP8 "),
            types(&["image/webp"])
        );
        assert_eq!(offered(None, b"\xff\xfe\0"), types(&[BYTES]));
        assert_eq!(offered(Some("image/png"), b"x"), types(&["image/png"]));
    }

    #[test]
    fn a_type_that_could_forge_a_field_is_said_as_other() {
        assert_eq!(
            sayable("text/plain;charset=utf-8"),
            "text/plain;charset=utf-8"
        );
        for kind in ["", "a b", "a\nwhy=x", "a\"b", &"a".repeat(101)] {
            assert_eq!(sayable(kind), "other", "{kind:?}");
        }
    }
}
