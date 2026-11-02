//! Sums a call up from its transcript, with a language model on the NPU
//! (dictation's language.rs) at its background priority, once the call is
//! written and Whisper let go.
//!
//! The NPU's model reads a prompt of at most `PROMPT_TOKENS`, so a call
//! too long for one is read in parts: notes are taken on each, and the
//! summary is written from the notes, which are read in parts again until
//! they fit in one. A part is cut by its characters, which only guesses at
//! its tokens, so a part the model can't read is read again in halves.

use dictation::language::LanguageModel;

use crate::said;

/// The most of a transcript one prompt holds, in characters. English runs
/// about four characters a token, so this is about 600 of the 1024 tokens
/// the NPU reads, leaving room for what it's asked and a word that's
/// longer in tokens than most.
const PART_CHARS: usize = 2400;

/// How many times notes are taken on notes before giving up: far more
/// than a call of a day needs, since each round's notes are a fraction
/// of what they're on, and an end if they somehow stop getting shorter.
const ROUNDS: usize = 12;

/// The shortest part that's halved when the model can't read it. Shorter
/// than this, the part's length isn't what failed.
const HALVED_AT_LEAST: usize = PART_CHARS / 8;

/// What answers a prompt: the language model, or a test's stand-in.
pub trait Answers {
    /// The answer to `prompt`.
    ///
    /// # Errors
    ///
    /// When the prompt can't be answered, as one too long to read.
    fn answer(&mut self, prompt: &str) -> Result<String, String>;
}

impl Answers for LanguageModel {
    fn answer(&mut self, prompt: &str) -> Result<String, String> {
        LanguageModel::answer(self, prompt)
    }
}

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
pub fn grounded(model: &mut impl Answers, title: &str, transcript: &str) -> Result<Summed, String> {
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
    model: &mut impl Answers,
    title: &str,
    transcript: &str,
    strays: &[String],
) -> Result<String, String> {
    let mut texts = parts(transcript, PART_CHARS);
    let mut from = From::Transcript;
    for round in 0..=ROUNDS {
        match texts.as_slice() {
            [] => return model.answer(&summary_of(title, from, "", strays)),
            [text] => match model.answer(&summary_of(title, from, text, strays)) {
                Err(why) => match halves(text) {
                    Some(halves) => {
                        eprintln!(
                            "call-transcripts: a summary of {} characters couldn't be \
                             written, so notes are taken on its halves: {why}",
                            text.len()
                        );
                        texts = halves;
                    }
                    None => return Err(why),
                },
                written => return written,
            },
            _ => {}
        }
        if round == ROUNDS {
            break;
        }
        let count = texts.len();
        let mut notes = Vec::with_capacity(count);
        for (at, text) in texts.iter().enumerate() {
            notes.extend(noted(model, text, &|text| {
                notes_on(title, at + 1, count, from, text)
            })?);
        }
        texts = parts(&notes.join("\n\n"), PART_CHARS);
        eprintln!(
            "call-transcripts: notes taken on {count} parts of {}, in {} now",
            from.what(),
            texts.len()
        );
        from = From::Notes;
    }
    Err(format!(
        "its notes were still {} parts after {ROUNDS} rounds",
        texts.len()
    ))
}

/// The notes `ask` asks for on `text`, read again in halves while the
/// model can't read it whole, each half giving notes of its own.
fn noted(
    model: &mut impl Answers,
    text: &str,
    ask: &dyn Fn(&str) -> String,
) -> Result<Vec<String>, String> {
    let why = match model.answer(&ask(text)) {
        Ok(notes) => return Ok(vec![notes]),
        Err(why) => why,
    };
    let Some(halves) = halves(text) else {
        return Err(why);
    };
    eprintln!(
        "call-transcripts: a part of {} characters couldn't be read, so its halves are: \
         {why}",
        text.len()
    );
    let mut notes = Vec::new();
    for half in halves {
        notes.extend(noted(model, &half, ask)?);
    }
    Ok(notes)
}

/// `text` in two, or more where a line is cut between words, when it's
/// long enough that its length could be what the model couldn't read.
fn halves(text: &str) -> Option<Vec<String>> {
    if text.len() < HALVED_AT_LEAST {
        return None;
    }
    Some(parts(text, text.len().div_ceil(2))).filter(|halves| halves.len() > 1)
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

    /// A model that reads at most `most` characters of what it's given
    /// and notes each part in `noted` characters, keeping what it's asked.
    struct Reader {
        most: usize,
        noted: usize,
        read: Vec<usize>,
        refused: usize,
    }

    impl Reader {
        fn new(most: usize, noted: usize) -> Self {
            Reader {
                most,
                noted,
                read: Vec::new(),
                refused: 0,
            }
        }
    }

    impl Answers for Reader {
        fn answer(&mut self, prompt: &str) -> Result<String, String> {
            let (asked, text) = prompt.split_once("\n\n").unwrap_or((prompt, ""));
            if text.len() > self.most {
                self.refused += 1;
                return Err("GenAI's answer failed with status -17".to_owned());
            }
            self.read.push(text.len());
            if asked.contains("Write a short summary") {
                Ok("The summary.".to_owned())
            } else {
                Ok("noted ".repeat(self.noted / 6))
            }
        }
    }

    fn call(lines: usize) -> String {
        (0..lines)
            .map(|at| format!("**Others**, {at}:00: one two three four five six seven"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn a_long_call_is_summed_up_from_notes_that_fit_in_one_prompt() {
        // Ninety minutes of short turns, noted at near half a part each,
        // takes more rounds of notes on notes than four.
        let mut reader = Reader::new(PART_CHARS, 1000);
        let summary = summarize(&mut reader, "Joel / Chad", &call(2000), &[]).unwrap();
        assert_eq!(summary, "The summary.");
        assert_eq!(reader.refused, 0);
        assert!(reader.read.iter().all(|&read| read <= PART_CHARS));
    }

    #[test]
    fn a_part_too_long_to_read_is_read_in_halves() {
        let mut reader = Reader::new(PART_CHARS / 2, 100);
        let summary = summarize(&mut reader, "", &call(300), &[]).unwrap();
        assert_eq!(summary, "The summary.");
        assert!(reader.refused > 0);
    }

    #[test]
    fn a_part_too_short_to_halve_fails_as_the_model_did() {
        let mut reader = Reader::new(10, 100);
        assert_eq!(
            summarize(&mut reader, "", &call(3), &[]),
            Err("GenAI's answer failed with status -17".to_owned())
        );
    }

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
