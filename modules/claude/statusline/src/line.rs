//! The status line's parts, from the session Claude Code gives and its transcript.

use std::ffi::OsStr;
use std::path::Path;

use serde_json::Value;

/// Marks from the icon font, which the theme's monospaced face carries.
const FOLDER: char = '\u{f07b}';
const MODEL: char = '\u{f2db}';
const CONTEXT: char = '\u{f019}';
const SEPARATOR: char = '\u{e0b1}';

/// Tokens in a thousand, and the count of thousands from which a whole number
/// of them reads plainly enough.
const THOUSAND: f64 = 1000.0;
const PLAIN_FROM: f64 = 10.0;

/// What an answer's usage counts of what it read, cached or not.
const READ: [&str; 3] = [
    "input_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

/// A colour, as a terminal draws one from its red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour {
    red: u8,
    green: u8,
    blue: u8,
}

impl Colour {
    /// Read a colour written `#rrggbb`.
    pub fn parse(written: &str) -> Result<Self, String> {
        let refused = || format!("{written} isn't a colour written #rrggbb");
        let hex = written
            .strip_prefix('#')
            .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(refused)?;
        let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).map_err(|_| refused());
        Ok(Self {
            red: channel(0)?,
            green: channel(2)?,
            blue: channel(4)?,
        })
    }
}

/// Each part's colour, by what the part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colours {
    pub folder: Colour,
    pub model: Colour,
    pub context: Colour,
    pub separator: Colour,
}

impl Colours {
    /// Read each part's colour from a JSON object naming them.
    pub fn from_json(written: &str) -> Result<Self, String> {
        let colours: Value = serde_json::from_str(written)
            .map_err(|error| format!("the colours aren't JSON: {error}"))?;
        let part = |name: &str| {
            colours
                .get(name)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("the colours give no {name}"))
                .and_then(Colour::parse)
        };
        Ok(Self {
            folder: part("folder")?,
            model: part("model")?,
            context: part("context")?,
            separator: part("separator")?,
        })
    }
}

/// Whether a part is drawn bold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

/// The session Claude Code describes after a turn.
pub struct Session(Value);

impl Session {
    /// Read the session from the JSON Claude Code writes.
    pub fn from_json(written: &str) -> Result<Self, String> {
        serde_json::from_str(written)
            .map(Self)
            .map_err(|error| format!("the session isn't JSON: {error}"))
    }

    /// Give the text at a JSON pointer, unless it's absent or empty.
    fn text(&self, pointer: &str) -> Option<&str> {
        self.0
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    }

    /// Give the name of the folder the session works in.
    pub fn folder(&self) -> &str {
        let folder = self.text("/workspace/current_dir").unwrap_or("unknown");
        Path::new(folder)
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
    }

    /// Give the model's name, as Claude Code shows it when it says.
    pub fn model(&self) -> &str {
        self.text("/model/display_name")
            .or_else(|| self.text("/model/id"))
            .unwrap_or("unknown")
    }

    /// Give where the session's transcript is written, when it says.
    pub fn transcript(&self) -> Option<&Path> {
        self.text("/transcript_path").map(Path::new)
    }
}

/// Give the text in a colour, as a terminal draws it.
pub fn paint(text: &str, colour: Colour, weight: Weight) -> String {
    let Colour { red, green, blue } = colour;
    let bold = match weight {
        Weight::Regular => "",
        Weight::Bold => "1;",
    };
    format!("\x1b[{bold}38;2;{red};{green};{blue}m{text}\x1b[0m")
}

/// Give a count of tokens in thousands, to a tenth below ten thousand.
pub fn thousands(tokens: u64) -> String {
    if tokens == 0 {
        return "0".to_owned();
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a context's tokens are far fewer than a float holds exactly"
    )]
    let count = tokens as f64 / THOUSAND;
    if count >= PLAIN_FROM {
        format!("{count:.0}k")
    } else {
        format!("{count:.1}k")
    }
}

/// Give how many tokens the last answer read, cached ones too, or nothing if
/// there's no answer yet.
///
/// That's the context's size as it stands. The transcript is read from its
/// end, so a long one is parsed only back to its last answer. A line that
/// isn't JSON is passed over, as a line still being written is.
pub fn context_tokens(transcript: &str) -> Option<u64> {
    transcript.lines().rev().find_map(|line| {
        let entry: Value = serde_json::from_str(line).ok()?;
        if entry.get("type").and_then(Value::as_str) != Some("assistant") {
            return None;
        }
        let usage = entry
            .pointer("/message/usage")
            .filter(|usage| usage.is_object())?;
        Some(
            READ.iter()
                .filter_map(|kind| usage.get(kind).and_then(Value::as_u64))
                .sum(),
        )
    })
}

/// Give the status line for the session, and the context's size when an
/// answer has read some.
pub fn status(session: &Session, colours: &Colours, read: Option<u64>) -> String {
    let mut parts = vec![
        paint(
            &format!("{FOLDER} {}", session.folder()),
            colours.folder,
            Weight::Bold,
        ),
        paint(
            &format!("{MODEL} {}", session.model()),
            colours.model,
            Weight::Regular,
        ),
    ];
    if let Some(read) = read {
        parts.push(paint(
            &format!("{CONTEXT} {}", thousands(read)),
            colours.context,
            Weight::Regular,
        ));
    }
    let separator = paint(&SEPARATOR.to_string(), colours.separator, Weight::Regular);
    parts.join(&format!(" {separator} "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colours() -> Colours {
        Colours::from_json(
            r##"{"folder": "#010203", "model": "#040506", "context": "#070809",
                "separator": "#0a0b0c"}"##,
        )
        .expect("the colours are good")
    }

    fn session(written: &str) -> Session {
        Session::from_json(written).expect("the session is JSON")
    }

    fn answer(usage: &str) -> String {
        format!(r#"{{"type": "assistant", "message": {{"usage": {usage}}}}}"#)
    }

    #[test]
    fn under_ten_thousand_reads_to_a_tenth() {
        assert_eq!(thousands(1234), "1.2k");
    }

    #[test]
    fn ten_thousand_and_more_reads_whole() {
        assert_eq!(thousands(45_678), "46k");
    }

    #[test]
    fn a_half_reads_to_the_even_thousand_below() {
        assert_eq!(thousands(12_500), "12k");
    }

    #[test]
    fn nothing_reads_as_nought() {
        assert_eq!(thousands(0), "0");
    }

    #[test]
    fn the_last_answer_counts_every_token_it_read() {
        let transcript = [
            answer(r#"{"input_tokens": 9, "output_tokens": 500}"#),
            r#"{"type": "user", "message": {"content": "hi"}}"#.to_owned(),
            answer(concat!(
                r#"{"input_tokens": 10, "cache_creation_input_tokens": 200,"#,
                r#" "cache_read_input_tokens": 3000, "output_tokens": 40}"#,
            )),
        ]
        .join("\n");
        assert_eq!(context_tokens(&transcript), Some(3210));
    }

    #[test]
    fn a_line_that_isnt_an_answer_is_passed_over() {
        let transcript = [
            answer(r#"{"input_tokens": 7}"#),
            "{not json".to_owned(),
            "42".to_owned(),
        ];
        assert_eq!(context_tokens(&transcript.join("\n")), Some(7));
    }

    #[test]
    fn no_answer_says_nothing() {
        assert_eq!(context_tokens(""), None);
        assert_eq!(context_tokens(r#"{"type": "user"}"#), None);
    }

    #[test]
    fn a_colour_is_drawn_as_its_red_green_and_blue() {
        let orange = Colour::parse("#ff8000").expect("a colour");
        assert_eq!(
            paint("a", orange, Weight::Regular),
            "\x1b[38;2;255;128;0ma\x1b[0m"
        );
        assert_eq!(
            paint("a", orange, Weight::Bold),
            "\x1b[1;38;2;255;128;0ma\x1b[0m"
        );
    }

    #[test]
    fn only_a_colour_written_hash_rrggbb_is_read() {
        for written in ["ff8000", "#ff80", "#ff80000", "#gg8000", "#ff800\u{e9}"] {
            assert!(Colour::parse(written).is_err(), "{written}");
        }
    }

    #[test]
    fn colours_missing_a_part_say_which() {
        let refused = Colours::from_json(r##"{"folder": "#000000", "model": "#000000"}"##);
        assert_eq!(refused, Err("the colours give no context".to_owned()));
    }

    #[test]
    fn it_shows_the_folder_and_the_model_in_the_themes_colours() {
        let shown = status(
            &session(
                r#"{"workspace": {"current_dir": "/home/someone/src/core.nix"},
                    "model": {"id": "claude-opus-4-6", "display_name": "Opus 4.6"}}"#,
            ),
            &colours(),
            None,
        );
        let separator = paint("\u{e0b1}", colours().separator, Weight::Regular);
        assert_eq!(
            shown,
            [
                paint("\u{f07b} core.nix", colours().folder, Weight::Bold),
                paint("\u{f2db} Opus 4.6", colours().model, Weight::Regular),
            ]
            .join(&format!(" {separator} "))
        );
    }

    #[test]
    fn it_shows_the_context_when_an_answer_has_read_some() {
        let shown = status(
            &session(r#"{"model": {"id": "x"}}"#),
            &colours(),
            Some(12_500),
        );
        assert!(shown.ends_with(&paint("\u{f019} 12k", colours().context, Weight::Regular)));
    }

    #[test]
    fn a_model_without_a_name_is_shown_by_its_id_or_as_unknown() {
        assert_eq!(
            session(r#"{"model": {"id": "x", "display_name": ""}}"#).model(),
            "x"
        );
        assert_eq!(session("{}").model(), "unknown");
    }

    #[test]
    fn a_session_without_a_folder_shows_one_unknown() {
        assert_eq!(session("{}").folder(), "unknown");
    }
}
