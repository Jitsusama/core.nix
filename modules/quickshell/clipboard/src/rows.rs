//! Copies' rows, ready for the launcher to draw (Y2YBTS's ready-rows.md):
//! each copy kept, newest first, worded here from the launcher's own
//! vocabulary, so Copies.qml builds and words nothing. A text copy's row
//! runs its lines together, cut short, and is marked by what it is, a link,
//! an address, a path or a number standing out from text; a picture's is
//! named by its size. When it was copied travels as a time the shell words,
//! since a row saying "2 min ago" would change each minute. Beside the rows
//! go how many there are and what the clipboard holds now, for the shell's
//! acts on a copy (Copied.qml).

use std::collections::BTreeMap;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

/// The launcher's place these rows are drawn in.
pub const PLACE: &str = "copies";

/// The thing in the vocabulary the clipboard's words are under.
const THING: &str = "clipboard";

/// The most characters a row's name runs to, its last an ellipsis.
const LONGEST: usize = 120;

/// A measure takes the next unit a thousand at a time, rounding into it
/// from here, so it never reads a thousand of its unit, as Machine's do.
const STEP: f64 = 1000.0;
const ROUNDS_UP: f64 = 999.5;
const BYTES: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];

/// The clipboard's words, as the launcher's vocabulary says them
/// (modules/quickshell/launcher/words.json, the `clipboard` thing's
/// `said`), and how the account writes numbers.
#[derive(Debug, Default)]
pub struct Words {
    said: BTreeMap<String, String>,
    numbers: Numbers,
}

impl Words {
    /// The clipboard's words from the vocabulary's text, none where it
    /// names no clipboard, and an error where it isn't JSON.
    pub fn from_vocabulary(text: &str, numbers: Numbers) -> serde_json::Result<Self> {
        let vocabulary: Value = serde_json::from_str(text)?;
        let things = vocabulary["things"].as_array().into_iter().flatten();
        let said = things
            .filter(|thing| thing["id"] == THING)
            .filter_map(|thing| thing["said"].as_object())
            .flatten()
            .filter_map(|(key, word)| Some((key.clone(), word.as_str()?.to_owned())))
            .collect();
        Ok(Words { said, numbers })
    }

    /// The words a key says, each `{part}` filled, or none for a key it
    /// lacks, as the launcher says nothing for a word it lacks.
    fn fill(&self, key: &str, parts: &[(&str, &str)]) -> String {
        let words = self.said.get(key).map_or("", String::as_str);
        parts.iter().fold(words.to_owned(), |text, (part, value)| {
            text.replacen(&format!("{{{part}}}"), value, 1)
        })
    }

    /// The words a key says, or none for a key it lacks.
    pub fn said(&self, key: &str) -> String {
        self.fill(key, &[])
    }

    /// A count in the words for one or many of it.
    fn counted(&self, one: &str, many: &str, count: usize) -> String {
        // As the launcher says a collection holding nothing (Numbers.js).
        if count == 0 {
            return "empty".to_owned();
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of copies or lines is far below 2^52"
        )]
        let figure = self.numbers.number(count as f64, 0);
        self.fill(if count == 1 { one } else { many }, &[("count", &figure)])
    }

    /// A size in bytes to three digits and its unit, a thousand a step,
    /// bytes whole below a kilobyte: "915 B", "1.52 MB".
    fn bytes(&self, count: usize) -> String {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a picture's size is said to three digits"
        )]
        let mut scaled = count as f64;
        let mut at = 0;
        while scaled >= ROUNDS_UP && at < BYTES.len() - 1 {
            scaled /= STEP;
            at += 1;
        }
        let said = if at == 0 {
            self.numbers.number(scaled.round(), 0)
        } else {
            self.numbers.significant(scaled)
        };
        format!("{said} {}", BYTES[at])
    }
}

/// How a copy reads in a row: its lines run together, cut short.
pub fn shown(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= LONGEST {
        return flat;
    }
    let mut cut: String = flat.chars().take(LONGEST - 1).collect();
    cut.push('\u{2026}');
    cut
}

/// How many lines a copy runs to, blank ones at its ends aside.
fn lines(text: &str) -> usize {
    text.trim().lines().count()
}

/// What a copy is, by the theme's glyph for it, so a link, an address, a
/// path or a number stands out from the text around it.
fn glyph(text: &str) -> &'static str {
    let one = text.trim();
    if lines(text) > 1 {
        "lines"
    } else if is_link(one) {
        "link"
    } else if is_address(one) {
        "email"
    } else if is_path(one) {
        "path"
    } else if is_number(one) {
        "number"
    } else {
        "clipboard"
    }
}

/// A scheme, `://`, and something after it, all of it unbroken.
fn is_link(one: &str) -> bool {
    let Some((scheme, rest)) = one.split_once("://") else {
        return false;
    };
    let mut letters = scheme.chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && letters.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        && !rest.is_empty()
        && !rest.chars().any(char::is_whitespace)
}

/// Someone at a domain whose last part is two letters or more.
fn is_address(one: &str) -> bool {
    if one.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((who, domain)) = one.split_once('@') else {
        return false;
    };
    let Some((name, last)) = domain.rsplit_once('.') else {
        return false;
    };
    !who.is_empty()
        && !domain.contains('@')
        && !name.is_empty()
        && last.len() >= 2
        && last.chars().all(|c| c.is_ascii_alphabetic())
}

/// A path from home, the root or here, unbroken.
fn is_path(one: &str) -> bool {
    let rooted = ["~/", "/", "./", "../"]
        .iter()
        .any(|start| one.starts_with(start));
    rooted && !one.chars().any(char::is_whitespace)
}

/// A number, signed or not, its digits grouped by commas, underscores or
/// spaces, with a fraction after one point.
fn is_number(one: &str) -> bool {
    let unsigned = one.strip_prefix(['-', '+']).unwrap_or(one);
    let whole = unsigned.trim_end_matches(|c: char| c.is_ascii_digit());
    if whole.len() == unsigned.len() {
        return false;
    }
    let before = whole.strip_suffix('.').unwrap_or(whole);
    before
        .chars()
        .all(|c| c.is_ascii_digit() || ",_ ".contains(c))
}

/// When a copy was made, as the shell words it: "just now", "12 min ago",
/// "14:02", "yesterday", as a message's is.
fn came(at: u128) -> Value {
    json!({ "at": u64::try_from(at).unwrap_or(u64::MAX), "as": "came" })
}

/// What a row asks for when it's pasted and when it's copied, in the one
/// verb order; the shell adds which keys the window under takes.
fn acts(id: u64) -> Value {
    json!([
        { "key": "verb:copy:paste", "ask": { "ask": "paste", "copy": id } },
        { "key": "verb:copy:copy", "ask": { "ask": "copy", "copy": id } }
    ])
}

/// A text copy's row.
pub fn text(id: u64, at: u128, text: &str, words: &Words) -> Value {
    let name = shown(text);
    let count = lines(text);
    let mut side = Vec::new();
    if count > 1 {
        side.push(json!({ "said": words.counted("line", "lines", count) }));
    }
    side.push(came(at));
    json!({
        "key": format!("row:{PLACE}:{id}"),
        "thing": format!("copy:{name}"),
        "place": PLACE,
        "kind": "text",
        "name": name,
        "glyph": glyph(text),
        "hue": "text",
        "side": side,
        "acts": acts(id),
        // As it was copied, lines and all, which the name cuts short.
        "held": { "id": id, "text": text, "reads": text.trim() },
    })
}

/// A picture kept, as its row is made from it.
pub struct Picture<'a> {
    pub id: u64,
    pub at: u128,
    pub kind: &'a str,
    pub path: &'a str,
    pub bytes: usize,
    pub size: Option<(u32, u32)>,
}

/// A picture's row, named by its width and height where its type says
/// them, which a search for "image" finds, and said by its type and size.
pub fn picture(picture: &Picture, words: &Words) -> Value {
    let name = match picture.size {
        Some((across, down)) => words.fill(
            "imageSized",
            &[("across", &across.to_string()), ("down", &down.to_string())],
        ),
        None => words.fill("image", &[]),
    };
    let (across, down) = picture.size.unwrap_or((0, 0));
    // Said by its subtype, "png" for image/png.
    let subtype = picture.kind.strip_prefix("image/").unwrap_or(picture.kind);
    let kind = words.fill(subtype, &[]);
    let kind = if kind.is_empty() {
        words.fill("picture", &[])
    } else {
        kind
    };
    json!({
        "key": format!("row:{PLACE}:{}", picture.id),
        "thing": format!("picture:{}", picture.id),
        "place": PLACE,
        "kind": "picture",
        "name": name,
        "glyph": "picture",
        "hue": "text",
        "side": [{ "said": kind }, { "said": words.bytes(picture.bytes) }, came(picture.at)],
        "acts": acts(picture.id),
        "held": { "id": picture.id, "picture": picture.path, "across": across, "down": down },
    })
}

/// How many copies are kept, in the place's words.
pub fn summary(count: usize, words: &Words) -> Value {
    json!({
        "key": format!("summary:{PLACE}"),
        "kind": "summary",
        "summary": { "place": PLACE, "count": count, "said": words.counted("one", "many", count) },
    })
}

/// What the clipboard holds now: the types it's offered as, and its text,
/// "" where it has none to read.
pub fn now<T: AsRef<str>>(types: &[T], text: &str) -> Value {
    let types: Vec<&str> = types.iter().map(AsRef::as_ref).collect();
    json!({
        "key": "now:clipboard",
        "kind": "now",
        "now": { "types": types, "text": text },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        let vocabulary = include_str!("../../launcher/words.json");
        Words::from_vocabulary(vocabulary, Numbers::default()).expect("words.json is JSON")
    }

    #[test]
    fn a_copy_runs_its_lines_together_cut_short() {
        assert_eq!(shown("  one\n\ttwo  three \n"), "one two three");
        let long = "a".repeat(130);
        let cut = shown(&long);
        assert_eq!(cut.chars().count(), 120);
        assert!(cut.ends_with('\u{2026}'), "{cut} ends in an ellipsis");
        assert_eq!(shown(&"é".repeat(120)), "é".repeat(120), "120 is whole");
    }

    #[test]
    fn a_copy_is_marked_by_what_it_is() {
        let cases = [
            ("one\ntwo", "lines"),
            ("https://example.com/a?b=c", "link"),
            ("git+ssh://host/repo", "link"),
            ("1http://x", "clipboard"),
            ("https://", "clipboard"),
            ("see https://example.com", "clipboard"),
            ("joel@example.ca", "email"),
            ("joel@example.c", "clipboard"),
            ("a@b@c.ca", "clipboard"),
            ("@example.ca", "clipboard"),
            ("~/src/core.nix", "path"),
            ("/etc/hosts", "path"),
            ("../up", "path"),
            ("./here and there", "clipboard"),
            ("1,234.5", "number"),
            ("-12", "number"),
            ("+.5", "number"),
            ("1 000 000", "number"),
            ("1.2.3", "clipboard"),
            ("12.", "clipboard"),
            ("-", "clipboard"),
            ("just words", "clipboard"),
        ];
        for (text, mark) in cases {
            assert_eq!(glyph(text), mark, "{text:?}");
        }
    }

    #[test]
    fn a_text_row_is_worded_and_says_when_it_was_copied() {
        let row = text(7, 1_000, "  first\nsecond\n", &words());
        assert_eq!(row["key"], "row:copies:7");
        assert_eq!(row["thing"], "copy:first second");
        assert_eq!(row["name"], "first second");
        assert_eq!(row["glyph"], "lines");
        assert_eq!(
            row["side"],
            json!([{ "said": "2 lines" }, { "at": 1_000, "as": "came" }])
        );
        assert_eq!(row["held"]["reads"], "first\nsecond");
        assert_eq!(row["acts"][0]["ask"], json!({ "ask": "paste", "copy": 7 }));
        let one = text(8, 2_000, "one line", &words());
        assert_eq!(one["side"], json!([{ "at": 2_000, "as": "came" }]));
    }

    #[test]
    fn a_picture_row_is_named_by_its_size_and_said_by_its_type() {
        let mut kept = Picture {
            id: 3,
            at: 5,
            kind: "image/png",
            path: "/run/pictures/3.png",
            bytes: 1_523_000,
            size: Some((640, 480)),
        };
        let row = picture(&kept, &words());
        assert_eq!(row["thing"], "picture:3");
        assert_eq!(row["name"], "Image 640 \u{d7} 480");
        assert_eq!(
            row["side"],
            json!([{ "said": "PNG" }, { "said": "1.52 MB" }, { "at": 5, "as": "came" }])
        );
        assert_eq!(row["held"]["picture"], "/run/pictures/3.png");
        kept.size = None;
        kept.kind = "image/x-unknown";
        kept.bytes = 512;
        let unknown = picture(&kept, &words());
        assert_eq!(unknown["name"], "Image");
        assert_eq!(unknown["side"][0]["said"], "Picture");
        assert_eq!(unknown["side"][1]["said"], "512 B");
    }

    #[test]
    fn sizes_take_the_next_unit_before_they_read_a_thousand() {
        let words = words();
        assert_eq!(words.bytes(999), "999 B");
        assert_eq!(words.bytes(1_000), "1.00 kB");
        assert_eq!(words.bytes(999_600), "1.00 MB");
        assert_eq!(words.bytes(64 << 20), "67.1 MB");
    }

    #[test]
    fn the_copies_are_counted_in_the_places_words() {
        let words = words();
        assert_eq!(summary(1, &words)["summary"]["said"], "1 copy");
        assert_eq!(summary(1_200, &words)["summary"]["said"], "1,200 copies");
        assert_eq!(summary(0, &words)["key"], "summary:copies");
        assert_eq!(summary(0, &words)["summary"]["said"], "empty");
    }

    #[test]
    fn what_the_clipboard_holds_is_its_types_and_its_text() {
        assert_eq!(
            now(&["image/png"], ""),
            json!({ "key": "now:clipboard", "kind": "now",
                    "now": { "types": ["image/png"], "text": "" } })
        );
    }
}
