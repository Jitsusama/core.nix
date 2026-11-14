//! The status line's parts, from the session Claude Code gives and its transcript.

use std::ffi::OsStr;
use std::path::Path;

use serde_json::Value;
use terminal_kit::Pen;

/// What parts the line, drawn in the rules' colour.
const BETWEEN: &str = "·";

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
/// answer has read some. Colour goes on the marks, each its kind's: the
/// folder a place on the machine, the model a program, the context's
/// size the room left. Of the words, only the folder is strong, as a
/// title.
pub fn status(session: &Session, pen: &Pen, read: Option<u64>) -> String {
    let mut parts = vec![
        format!(
            "{}{}",
            pen.marked("folder", "machine"),
            pen.title(session.folder())
        ),
        format!("{}{}", pen.marked("agent", "program"), session.model()),
    ];
    if let Some(read) = read {
        parts.push(format!(
            "{}{}",
            pen.marked("tokens", "success"),
            thousands(read)
        ));
    }
    parts.join(&format!(" {} ", pen.role("rule", BETWEEN)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::Ink;
    use terminal_kit::ink::Colours;

    /// Write a colour as the theme does, from its red, green and blue. The
    /// tests build theirs from channels, so no look is named in the code.
    fn hex(red: u8, green: u8, blue: u8) -> String {
        format!("#{red:02x}{green:02x}{blue:02x}")
    }

    /// A theme of the roles and marks the line draws with.
    fn theme() -> Value {
        let mark = |glyph: &str| serde_json::json!({"glyph": glyph, "console": glyph});
        serde_json::json!({
            "colours": {
                "strong": hex(1, 2, 3),
                "machine": hex(4, 5, 6),
                "program": hex(7, 8, 9),
                "success": hex(10, 11, 12),
                "rule": hex(13, 14, 15),
            },
            "terminal": {},
            "marks": {"folder": mark("F"), "agent": mark("A"), "tokens": mark("T")},
        })
    }

    /// The pen a terminal drawing 24-bit colour has.
    fn inked() -> Pen {
        let ink = Ink {
            colours: Colours::Truecolour,
            console: false,
        };
        Pen::new(ink, Some(&theme()))
    }

    /// The same marks with no ink, as into a pipe.
    fn plain() -> Pen {
        Pen::new(Ink::plain(), Some(&theme()))
    }

    fn session(written: &str) -> Session {
        Session::from_json(written).expect("the session is JSON")
    }

    fn answer(usage: &str) -> String {
        format!(r#"{{"type": "assistant", "message": {{"usage": {usage}}}}}"#)
    }

    #[test]
    fn under_ten_thousand_reads_to_a_tenth() {
        assert_eq!(thousands(1234), "1.2k", "a tenth of a thousand");
    }

    #[test]
    fn ten_thousand_and_more_reads_whole() {
        assert_eq!(thousands(45_678), "46k", "whole thousands");
    }

    #[test]
    fn a_half_reads_to_the_even_thousand_below() {
        assert_eq!(thousands(12_500), "12k", "a half rounds to even");
    }

    #[test]
    fn nothing_reads_as_nought() {
        assert_eq!(thousands(0), "0", "nothing isn't 0.0k");
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
        assert_eq!(
            context_tokens(&transcript),
            Some(3210),
            "the last answer's read tokens, cached ones too"
        );
    }

    #[test]
    fn a_line_that_isnt_an_answer_is_passed_over() {
        let transcript = [
            answer(r#"{"input_tokens": 7}"#),
            "{not json".to_owned(),
            "42".to_owned(),
        ];
        assert_eq!(
            context_tokens(&transcript.join("\n")),
            Some(7),
            "lines that aren't answers are passed over"
        );
    }

    #[test]
    fn no_answer_says_nothing() {
        assert_eq!(context_tokens(""), None, "an empty transcript");
        assert_eq!(
            context_tokens(r#"{"type": "user"}"#),
            None,
            "a transcript with no answer"
        );
    }

    #[test]
    fn it_shows_the_folder_and_the_model() {
        let shown = status(
            &session(
                r#"{"workspace": {"current_dir": "/home/someone/src/core.nix"},
                    "model": {"id": "claude-opus-4-6", "display_name": "Opus 4.6"}}"#,
            ),
            &plain(),
            None,
        );
        assert_eq!(
            shown, "F core.nix · A Opus 4.6",
            "the folder, then the model"
        );
    }

    #[test]
    fn it_shows_the_context_when_an_answer_has_read_some() {
        let shown = status(
            &session(r#"{"model": {"id": "x"}}"#),
            &plain(),
            Some(12_500),
        );
        assert!(
            shown.ends_with(" · T 12k"),
            "the context's size last: {shown:?}"
        );
    }

    #[test]
    fn colour_goes_on_the_marks_and_the_folder_is_strong() {
        let shown = status(
            &session(r#"{"workspace": {"current_dir": "/src/core.nix"}, "model": {"id": "x"}}"#),
            &inked(),
            Some(1234),
        );
        assert_eq!(
            shown,
            concat!(
                "\x1b[38;2;4;5;6mF\x1b[0m \x1b[1;38;2;1;2;3mcore.nix\x1b[0m",
                " \x1b[38;2;13;14;15m·\x1b[0m \x1b[38;2;7;8;9mA\x1b[0m x",
                " \x1b[38;2;13;14;15m·\x1b[0m \x1b[38;2;10;11;12mT\x1b[0m 1.2k",
            ),
            "each mark in its kind's colour, the model and the size plain"
        );
    }

    #[test]
    fn a_model_without_a_name_is_shown_by_its_id_or_as_unknown() {
        assert_eq!(
            session(r#"{"model": {"id": "x", "display_name": ""}}"#).model(),
            "x",
            "an empty name gives way to the id"
        );
        assert_eq!(session("{}").model(), "unknown", "no model at all");
    }

    #[test]
    fn a_session_without_a_folder_shows_one_unknown() {
        assert_eq!(session("{}").folder(), "unknown", "no folder at all");
    }
}
