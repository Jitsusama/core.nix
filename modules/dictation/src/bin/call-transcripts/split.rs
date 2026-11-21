//! Splits a line of theirs where its voice changes. A take of theirs runs
//! until a pause, and on a call with several people one often runs from
//! one voice into another's; heard whole, its line sat under one voice.
//! Each take is embedded in pieces (`dictation::pieces`), and once the
//! call's pieces are grouped a line whose pieces fall in more than one
//! voice is cut at the pieces' edges, each part given the words Parakeet
//! wrote in it. A part Parakeet wrote no words in joins the part before
//! it, or the one after when it's first, since a line holds words.

use std::collections::HashMap;

use dictation::parakeet::Word;
use dictation::transcript::Line;

use crate::said;

/// A piece of a take of theirs that was embedded: which take it's of,
/// the sample of the call it starts at, and which line it's of, if
/// Parakeet heard words in its take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub take: usize,
    pub start: usize,
    pub line: Option<usize>,
}

/// A call's lines, the pieces embedded and each line's words, timed from
/// the call's start, by the line they're of.
pub struct Heard<'a> {
    pub lines: &'a mut Vec<Line>,
    pub pieces: &'a mut [Piece],
    pub words: &'a mut HashMap<usize, Vec<Word>>,
}

/// Gives each line the voice of its pieces, `voices` holding each
/// piece's, and splits one whose pieces fall in more than one voice where
/// the voice changes, each part keeping the words written in it; the
/// pieces and words then say which of the lines, split, they're of. A
/// line with no pieces keeps the voice it has.
pub fn by_voice(heard: &mut Heard, voices: &[Option<usize>]) {
    let mut lines = Vec::with_capacity(heard.lines.len());
    let mut words = HashMap::new();
    for (index, line) in std::mem::take(heard.lines).into_iter().enumerate() {
        let of: Vec<usize> = (0..heard.pieces.len())
            .filter(|&piece| heard.pieces[piece].line == Some(index))
            .collect();
        let said = heard.words.remove(&index).unwrap_or_default();
        if of.is_empty() {
            if !said.is_empty() {
                words.insert(lines.len(), said);
            }
            lines.push(line);
            continue;
        }
        let runs = runs(&of, heard.pieces, voices);
        let parts = parts(&line, &runs, said);
        // A piece is of the last part starting at or before it, the first
        // part holding any piece that starts before the line does.
        let starts: Vec<usize> = parts.iter().map(|(part, _)| part.start).collect();
        for &piece in &of {
            let at = heard.pieces[piece].start;
            let part = starts.iter().rposition(|&start| start <= at).unwrap_or(0);
            heard.pieces[piece].line = Some(lines.len() + part);
        }
        for (part, said) in parts {
            if !said.is_empty() {
                words.insert(lines.len(), said);
            }
            lines.push(part);
        }
    }
    *heard.lines = lines;
    *heard.words = words;
}

/// Where each run of a line's pieces, `of`, in one voice starts, and its
/// voice.
fn runs(of: &[usize], pieces: &[Piece], voices: &[Option<usize>]) -> Vec<(usize, Option<usize>)> {
    let mut runs: Vec<(usize, Option<usize>)> = Vec::new();
    for &piece in of {
        let voice = voices[piece];
        if runs.last().is_none_or(|&(_, last)| last != voice) {
            runs.push((pieces[piece].start, voice));
        }
    }
    runs
}

/// A line cut into a part for each run of its pieces in one voice, each
/// holding the words written in it, a part with none joined to the one
/// before it, or after it when it's first. A line one voice holds whole,
/// or whose words can't be cut between voices, is kept whole under the
/// voice its words are in, its text as it was.
fn parts(line: &Line, runs: &[(usize, Option<usize>)], said: Vec<Word>) -> Vec<(Line, Vec<Word>)> {
    let mut parts: Vec<(Line, Vec<Word>)> = Vec::new();
    // The words of a first part with none worth writing, kept for the next.
    let mut before: Vec<Word> = Vec::new();
    let mut said = said.into_iter().peekable();
    for (run, &(start, voice)) in runs.iter().enumerate() {
        let end = runs.get(run + 1).map_or(usize::MAX, |&(next, _)| next);
        let mut words = Vec::new();
        while let Some(word) = said.next_if(|word| run + 1 == runs.len() || word.at < end) {
            words.push(word);
        }
        let joined: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        let Some(text) = said::spoken(&joined.join(" ")) else {
            match parts.last_mut() {
                Some((last, kept)) => {
                    kept.extend(words);
                    last.end = end.min(line.end);
                }
                None => before.extend(words),
            }
            continue;
        };
        let start = if parts.is_empty() { line.start } else { start };
        let words: Vec<Word> = before.drain(..).chain(words).collect();
        parts.push((
            Line {
                start,
                end: end.min(line.end),
                text,
                voice,
                name: None,
                ..line.clone()
            },
            words,
        ));
    }
    if parts.len() < 2 {
        let voice = parts.first().map_or(runs[0].1, |(part, _)| part.voice);
        let words = before
            .into_iter()
            .chain(parts.into_iter().flat_map(|(_, words)| words))
            .collect();
        let whole = Line {
            voice,
            name: None,
            ..line.clone()
        };
        return vec![(whole, words)];
    }
    parts
}

#[cfg(test)]
mod tests {
    use dictation::transcript::Side;

    use super::*;

    const SECOND: usize = 16_000;

    fn word(text: &str, at_s: usize) -> Word {
        Word {
            text: text.to_owned(),
            at: at_s * SECOND,
        }
    }

    fn theirs(from_s: usize, to_s: usize, text: &str) -> Line {
        Line {
            side: Side::Theirs,
            start: from_s * SECOND,
            end: to_s * SECOND,
            text: text.to_owned(),
            voice: None,
            name: None,
        }
    }

    fn pieces_of(line: usize, take: usize, starts_s: &[usize]) -> Vec<Piece> {
        starts_s
            .iter()
            .map(|&start| Piece {
                take,
                start: start * SECOND,
                line: Some(line),
            })
            .collect()
    }

    /// What each line says, under which voice, and from when to when in
    /// seconds.
    fn said(lines: &[Line]) -> Vec<(&str, Option<usize>, usize, usize)> {
        lines
            .iter()
            .map(|line| {
                let (from, to) = (line.start / SECOND, line.end / SECOND);
                (line.text.as_str(), line.voice, from, to)
            })
            .collect()
    }

    #[test]
    fn a_line_whose_voice_changes_is_split_there_with_its_words() {
        // One take of 9 s, Ada's for its first two pieces and Bo's for
        // its last, and a line of Joel's after it.
        let mut lines = vec![
            theirs(10, 19, "We could ship it. Not yet."),
            Line {
                side: Side::Mine,
                ..theirs(20, 22, "Why?")
            },
        ];
        let mut pieces = pieces_of(0, 0, &[10, 13, 16]);
        let mut words = HashMap::from([(
            0,
            vec![
                word("We", 10),
                word("could", 11),
                word("ship", 12),
                word("it.", 14),
                word("Not", 16),
                word("yet.", 17),
            ],
        )]);
        let mut heard = Heard {
            lines: &mut lines,
            pieces: &mut pieces,
            words: &mut words,
        };
        by_voice(&mut heard, &[Some(0), Some(0), Some(1)]);
        assert_eq!(
            said(&lines),
            [
                ("We could ship it.", Some(0), 10, 16),
                ("Not yet.", Some(1), 16, 19),
                ("Why?", None, 20, 22),
            ],
            "the take wasn't split where Bo's voice came in, or Joel's line moved"
        );
        let lines_of: Vec<Option<usize>> = pieces.iter().map(|piece| piece.line).collect();
        assert_eq!(
            lines_of,
            [Some(0), Some(0), Some(1)],
            "the pieces don't say which part they're of"
        );
        assert_eq!(
            words.get(&1).map(Vec::len),
            Some(2),
            "Bo's part doesn't hold his two words, so a rejoin couldn't split it again"
        );
    }

    #[test]
    fn a_part_with_no_words_joins_the_part_before_it() {
        let mut lines = vec![theirs(0, 9, "Right, so.")];
        let mut pieces = pieces_of(0, 0, &[0, 3, 6]);
        let mut words = HashMap::from([(0, vec![word("Right,", 1), word("so.", 4)])]);
        let mut heard = Heard {
            lines: &mut lines,
            pieces: &mut pieces,
            words: &mut words,
        };
        by_voice(&mut heard, &[Some(0), Some(0), Some(1)]);
        assert_eq!(
            said(&lines),
            [("Right, so.", Some(0), 0, 9)],
            "a voice no word was written in was given a line of its own"
        );
    }

    #[test]
    fn a_first_part_with_no_words_is_joined_by_the_part_after_it() {
        let mut lines = vec![theirs(0, 9, "Yes. No.")];
        let mut pieces = pieces_of(0, 0, &[0, 3, 6]);
        let mut words = HashMap::from([(0, vec![word("Yes.", 4), word("No.", 7)])]);
        let mut heard = Heard {
            lines: &mut lines,
            pieces: &mut pieces,
            words: &mut words,
        };
        by_voice(&mut heard, &[Some(2), Some(0), Some(1)]);
        assert_eq!(
            said(&lines),
            [("Yes.", Some(0), 0, 6), ("No.", Some(1), 6, 9)],
            "the part after a wordless first one didn't start the line"
        );
    }

    #[test]
    fn a_line_one_voice_holds_keeps_its_text_and_gets_that_voice() {
        // Its text is kept as it was, never spelled again from its words.
        let mut lines = vec![theirs(0, 6, "Fine (laughs)."), theirs(7, 9, "Okay.")];
        let mut pieces = pieces_of(0, 0, &[0, 3]);
        let mut words = HashMap::from([(0, vec![word("Fine", 1), word("(laughs).", 2)])]);
        let mut heard = Heard {
            lines: &mut lines,
            pieces: &mut pieces,
            words: &mut words,
        };
        by_voice(&mut heard, &[Some(1), Some(1)]);
        assert_eq!(
            said(&lines),
            [("Fine (laughs).", Some(1), 0, 6), ("Okay.", None, 7, 9)],
            "a line in one voice was rewritten, or one with no pieces was given a voice"
        );
    }
}
