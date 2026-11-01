//! Sums a call up from its transcript, with a language model on the NPU
//! (dictation's language.rs) at its background priority, once the call is
//! written and Whisper let go.
//!
//! The NPU's model reads a prompt of at most `PROMPT_TOKENS`, so a call
//! too long for one is read in parts: notes are taken on each, and the
//! summary is written from the notes, which are read in parts again while
//! they're still too long.

use dictation::language::LanguageModel;

use crate::said;

/// The most of a transcript one prompt holds, in characters. English runs
/// about four characters a token, so this is about 600 of the 1024 tokens
/// the NPU reads, leaving room for what it's asked and a word that's
/// longer in tokens than most.
const PART_CHARS: usize = 2400;

/// How many times notes are taken on notes before what's left is read as
/// it is: enough for a call of several hours, and an end if the notes
/// somehow stop getting shorter.
const ROUNDS: usize = 4;

/// A summary as written, kept or not.
pub enum Summed {
    /// It names only what the call does.
    Kept(String),
    /// Written twice, it named what the call never did each time, these.
    Strayed(Vec<String>),
}

/// The summary of the call titled `title` that `transcript` writes down,
/// naming only what the call does. One naming what the call never did
/// is written once more, told not to name it, and given up on if it
/// still does; each such summary is logged, since nothing else keeps it,
/// to tell a made-up name from a word the check mistook for one.
///
/// # Errors
///
/// When the model fails to answer.
pub fn grounded(
    model: &mut LanguageModel,
    title: &str,
    transcript: &str,
) -> Result<Summed, String> {
    let mut strays = Vec::new();
    for _ in 0..2 {
        let written = summarize(model, title, transcript, &strays)?;
        let named = said::strays(&written, transcript);
        if named.is_empty() {
            return Ok(Summed::Kept(written));
        }
        eprintln!(
            "call-transcripts: a summary named {}, which the call never did:\n{written}",
            named.join(", ")
        );
        strays.extend(named);
    }
    Ok(Summed::Strayed(strays))
}

/// The summary of the call titled `title` that `transcript` writes down,
/// told not to name what `strays` holds, which an earlier summary named
/// and the call never did.
fn summarize(
    model: &mut LanguageModel,
    title: &str,
    transcript: &str,
    strays: &[String],
) -> Result<String, String> {
    let mut texts = parts(transcript, PART_CHARS);
    let mut from = From::Transcript;
    for _ in 0..ROUNDS {
        if texts.len() <= 1 {
            break;
        }
        let count = texts.len();
        let notes = texts
            .iter()
            .enumerate()
            .map(|(at, text)| model.answer(&notes_on(title, at + 1, count, from, text)))
            .collect::<Result<Vec<_>, _>>()?;
        texts = parts(&notes.join("\n\n"), PART_CHARS);
        from = From::Notes;
    }
    model.answer(&summary_of(title, from, &texts.join("\n\n"), strays))
}

/// What a prompt gives the model to read.
#[derive(Clone, Copy)]
enum From {
    Transcript,
    Notes,
}

impl From {
    fn what(self) -> &'static str {
        match self {
            Self::Transcript => "the transcript",
            Self::Notes => "notes taken on the transcript",
        }
    }
}

/// What the call is called in a prompt.
fn named(title: &str) -> String {
    if title.is_empty() {
        "a call".to_owned()
    } else {
        format!("a call titled \"{title}\"")
    }
}

fn summary_of(title: &str, from: From, text: &str, strays: &[String]) -> String {
    let unsaid = if strays.is_empty() {
        String::new()
    } else {
        format!(
            " The call never mentions {}, so don't either.",
            strays.join(" or ")
        )
    };
    format!(
        "Below is {} of {}. Write a short summary of it for Joel, who was on the call. Start \
         with a sentence or two saying what it was about. Then list, as bullets, what was \
         decided and what anyone said they would do, naming who. Say only what the call \
         says, and name nothing it doesn't name.{unsaid} If nothing was decided or promised, \
         stop after those sentences. Write only the summary, with no note of your own.\n\n{text}",
        from.what(),
        named(title),
    )
}

fn notes_on(title: &str, part: usize, parts: usize, from: From, text: &str) -> String {
    format!(
        "Below is part {part} of {parts} of {} of {}. Note briefly what was discussed, what was \
         decided and what anyone said they would do in this part, naming who.\n\n{text}",
        from.what(),
        named(title),
    )
}

/// `text` cut into parts of at most `most` characters, each part whole
/// lines where a line fits, and a line too long for one part cut between
/// words.
fn parts(text: &str, most: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut part = String::new();
    for line in text.lines().flat_map(|line| pieces(line, most)) {
        if !part.is_empty() && part.len() + 1 + line.len() > most {
            parts.push(std::mem::take(&mut part));
        }
        if !part.is_empty() {
            part.push('\n');
        }
        part.push_str(line);
    }
    if !part.trim().is_empty() {
        parts.push(part);
    }
    parts
}

/// A line cut between words into pieces of at most `most` characters, a
/// word longer than that being a piece of its own.
fn pieces(line: &str, most: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut rest = line;
    while rest.len() > most {
        // Up to the character after the last that fits, so a space there
        // is a cut too, and never inside a character.
        let window = (0..=most + 1)
            .rev()
            .find(|&end| rest.is_char_boundary(end))
            .unwrap_or(0);
        let cut = rest[..window]
            .rfind(' ')
            .filter(|&cut| cut > 0)
            .or_else(|| rest.find(' '))
            .unwrap_or(rest.len());
        pieces.push(&rest[..cut]);
        rest = rest[cut..].trim_start();
    }
    pieces.push(rest);
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_call_is_one_part() {
        assert_eq!(
            parts("**Me**, 0:01: Hi.\n\n**Others**, 0:02: Hello.", 100).len(),
            1
        );
    }

    #[test]
    fn a_long_call_is_cut_between_lines_and_loses_none() {
        let line = "**Me**, 0:01: one two three four";
        let text = [line; 10].join("\n");
        let cut = parts(&text, 3 * line.len() + 2);
        assert_eq!(cut.len(), 4);
        assert!(cut.iter().all(|part| part.lines().all(|l| l == line)));
        assert_eq!(
            cut.iter().map(|part| part.lines().count()).sum::<usize>(),
            10
        );
    }

    #[test]
    fn a_line_too_long_for_a_part_is_cut_between_words() {
        let cut = parts("one two three four five", 9);
        assert_eq!(cut, ["one two", "three", "four five"]);
    }

    #[test]
    fn a_word_longer_than_a_part_is_a_part_of_its_own() {
        assert_eq!(parts("a abcdefghijk b", 4), ["a", "abcdefghijk", "b"]);
    }

    #[test]
    fn a_line_is_never_cut_inside_a_character() {
        assert_eq!(
            parts("caf\u{e9} cr\u{e8}me", 4),
            ["caf\u{e9}", "cr\u{e8}me"]
        );
    }

    #[test]
    fn nothing_said_is_no_part() {
        assert!(parts("\n\n", 10).is_empty());
    }
}
