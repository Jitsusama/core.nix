//! Splits a line of theirs where its voice changes. A take of theirs runs
//! until a pause, and on a call with several people one often runs from
//! one voice into another's; heard whole, its line sat under one voice.
//! Each take is embedded in pieces (`dictation::pieces`), and once the
//! call's pieces are grouped a take whose pieces fall in more than one
//! voice is cut at the pieces' edges, each part given the words Parakeet
//! wrote in it.
//!
//! A run of pieces whose words say nothing once tidied (said.rs), none
//! at all or only a hesitation, joins the part before it, or the one
//! after when it's first, words and all, since a line holds words; two
//! parts that are then in one voice are one. A bracketed note is never
//! cut, so tidying each part leaves it out as tidying the whole did. A
//! take split when the call was last told apart is joined again before
//! it's split, so a rejoin's grouping decides its parts afresh.

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

/// A line as its take was heard, the parts it was split into before
/// joined again: its pieces, its words, and the take it's of, none for a
/// line with no pieces.
struct Whole {
    line: Line,
    pieces: Vec<usize>,
    words: Vec<Word>,
    take: Option<usize>,
}

/// Gives each line the voice of its pieces, `voices` holding each
/// piece's, and splits one whose pieces fall in more than one voice where
/// the voice changes, each part keeping the words written in it; the
/// pieces and words then say which of the lines, split, they're of. A
/// line with no pieces keeps the voice it has.
pub fn by_voice(heard: &mut Heard, voices: &[Option<usize>]) {
    let mut lines = Vec::with_capacity(heard.lines.len());
    let mut words = HashMap::new();
    for whole in wholes(heard) {
        if whole.pieces.is_empty() {
            if !whole.words.is_empty() {
                words.insert(lines.len(), whole.words);
            }
            lines.push(whole.line);
            continue;
        }
        let runs = runs(&whole.pieces, heard.pieces, voices);
        let parts = parts(&whole.line, &runs, whole.words);
        // A piece is of the last part starting at or before it; the first
        // starts with the take.
        let starts: Vec<usize> = parts.iter().map(|(part, _)| part.start).collect();
        for &piece in &whole.pieces {
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

/// The lines as their takes were heard: next to each other, the parts of
/// a take split before are joined again. Each line's pieces are read
/// before any is renumbered, since one renumbered for a part would
/// otherwise be read as a later line's.
fn wholes(heard: &mut Heard) -> Vec<Whole> {
    let mut of_line: HashMap<usize, Vec<usize>> = HashMap::new();
    for (piece, of) in heard.pieces.iter().enumerate() {
        if let Some(line) = of.line {
            of_line.entry(line).or_default().push(piece);
        }
    }
    let mut wholes: Vec<Whole> = Vec::new();
    for (index, line) in std::mem::take(heard.lines).into_iter().enumerate() {
        let pieces = of_line.remove(&index).unwrap_or_default();
        let words = heard.words.remove(&index).unwrap_or_default();
        let take = pieces.first().map(|&piece| heard.pieces[piece].take);
        match wholes.last_mut() {
            Some(whole) if take.is_some() && whole.take == take => {
                whole.line.end = line.end;
                whole.line.text = format!("{} {}", whole.line.text, line.text);
                whole.pieces.extend(pieces);
                whole.words.extend(words);
            }
            _ => wholes.push(Whole {
                line,
                pieces,
                words,
                take,
            }),
        }
    }
    wholes
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
/// holding the words written in it, as the module says. A line left in
/// one part is kept whole under that part's voice, its text as it was.
fn parts(line: &Line, runs: &[(usize, Option<usize>)], said: Vec<Word>) -> Vec<(Line, Vec<Word>)> {
    // Each part kept: where it starts, its voice and its words.
    let mut kept: Vec<(usize, Option<usize>, Vec<Word>)> = Vec::new();
    // The words of runs before the first worth a line.
    let mut before: Vec<Word> = Vec::new();
    for (&(start, voice), words) in runs.iter().zip(in_runs(runs, said)) {
        let worth = said::spoken(&spelled(&words)).is_some();
        match kept.last_mut() {
            Some(last) if !worth || last.1 == voice => last.2.extend(words),
            _ if !worth => before.extend(words),
            _ => kept.push((start, voice, before.drain(..).chain(words).collect())),
        }
    }
    if kept.len() < 2 {
        let voice = kept.first().map_or(runs[0].1, |&(_, voice, _)| voice);
        let words = before
            .into_iter()
            .chain(kept.into_iter().flat_map(|(_, _, words)| words));
        let whole = Line {
            voice,
            name: None,
            ..line.clone()
        };
        return vec![(whole, words.collect())];
    }
    let ends: Vec<usize> = kept
        .iter()
        .skip(1)
        .map(|&(start, _, _)| start)
        .chain([line.end])
        .collect();
    kept.into_iter()
        .zip(ends)
        .enumerate()
        .map(|(part, ((start, voice, words), end))| {
            // A kept part holds words worth a line, so it's never empty.
            let text = said::spoken(&spelled(&words)).unwrap_or_default();
            let start = if part == 0 { line.start } else { start };
            let line = Line {
                start,
                end,
                text,
                voice,
                name: None,
                ..line.clone()
            };
            (line, words)
        })
        .collect()
}

/// The words in each run, by when each was said, a word inside a bracket
/// opened before it kept with the word before.
fn in_runs(runs: &[(usize, Option<usize>)], said: Vec<Word>) -> Vec<Vec<Word>> {
    let mut in_runs: Vec<Vec<Word>> = runs.iter().map(|_| Vec::new()).collect();
    let mut depth = 0_usize;
    let mut last = 0;
    for word in said {
        if depth == 0 {
            let by_time = runs.iter().rposition(|&(start, _)| start <= word.at);
            last = by_time.unwrap_or(0).max(last);
        }
        for c in word.text.chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        in_runs[last].push(word);
    }
    in_runs
}

/// Words, a space between each.
fn spelled(words: &[Word]) -> String {
    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    texts.join(" ")
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

    /// `lines` split by `voices`, a voice for each of `pieces`.
    fn split(
        lines: &mut Vec<Line>,
        pieces: &mut [Piece],
        words: &mut HashMap<usize, Vec<Word>>,
        voices: &[Option<usize>],
    ) {
        let mut heard = Heard {
            lines,
            pieces,
            words,
        };
        by_voice(&mut heard, voices);
    }

    #[test]
    fn a_hesitation_in_another_voice_stays_in_the_part_before_it() {
        let mut lines = vec![theirs(0, 9, "Ship it. Mm-hmm. Not yet.")];
        let mut pieces = pieces_of(0, 0, &[0, 3, 6]);
        let said = [
            ("Ship", 1),
            ("it.", 2),
            ("Mm-hmm.", 4),
            ("Not", 7),
            ("yet.", 8),
        ];
        let mut words = HashMap::from([(0, said.map(|(text, at)| word(text, at)).into())]);
        split(
            &mut lines,
            &mut pieces,
            &mut words,
            &[Some(0), Some(1), Some(2)],
        );
        assert_eq!(
            said(&lines),
            [
                ("Ship it. Mm-hmm.", Some(0), 0, 6),
                ("Not yet.", Some(2), 6, 9)
            ],
            "the hesitation between two voices was dropped, or given a line of its own"
        );
        assert_eq!(
            words.get(&0).map(Vec::len),
            Some(3),
            "the part's words don't hold what its text says"
        );
    }

    #[test]
    fn one_voice_either_side_of_a_wordless_run_is_one_line() {
        let mut lines = vec![theirs(0, 9, "Ship it, then.")];
        let mut pieces = pieces_of(0, 0, &[0, 3, 6]);
        let mut words =
            HashMap::from([(0, vec![word("Ship", 1), word("it,", 2), word("then.", 7)])]);
        split(
            &mut lines,
            &mut pieces,
            &mut words,
            &[Some(0), Some(1), Some(0)],
        );
        assert_eq!(
            said(&lines),
            [("Ship it, then.", Some(0), 0, 9)],
            "one voice's line was cut in two around a piece holding no words"
        );
    }

    #[test]
    fn a_bracketed_note_is_never_cut_between_two_parts() {
        let mut lines = vec![theirs(0, 9, "Fine. Not yet.")];
        let mut pieces = pieces_of(0, 0, &[0, 3]);
        let said = [
            ("Fine.", 1),
            ("(wind", 2),
            ("blowing)", 4),
            ("Not", 5),
            ("yet.", 6),
        ];
        let mut words = HashMap::from([(0, said.map(|(text, at)| word(text, at)).into())]);
        split(&mut lines, &mut pieces, &mut words, &[Some(0), Some(1)]);
        assert_eq!(
            said(&lines),
            [("Fine.", Some(0), 0, 3), ("Not yet.", Some(1), 3, 9)],
            "half a bracketed note was written as words"
        );
    }

    #[test]
    fn a_take_split_before_is_split_afresh_by_a_later_grouping() {
        let mut lines = vec![theirs(10, 19, "We could ship it. Not yet.")];
        let mut pieces = pieces_of(0, 0, &[10, 13, 16]);
        let said = [
            ("We", 10),
            ("could", 11),
            ("ship", 12),
            ("it.", 14),
            ("Not", 16),
            ("yet.", 17),
        ];
        let mut words = HashMap::from([(0, said.map(|(text, at)| word(text, at)).into())]);
        split(
            &mut lines,
            &mut pieces,
            &mut words,
            &[Some(0), Some(0), Some(1)],
        );
        split(
            &mut lines,
            &mut pieces,
            &mut words,
            &[Some(2), Some(2), Some(2)],
        );
        assert_eq!(
            said(&lines),
            [("We could ship it. Not yet.", Some(2), 10, 19)],
            "a take split by the first grouping stayed split when the next put it in one voice"
        );
        assert_eq!(
            words.get(&0).map(Vec::len),
            Some(6),
            "the take joined again lost some of its words"
        );
    }

    #[test]
    fn a_later_line_is_split_and_numbered_past_an_earlier_split() {
        let mine = Line {
            side: Side::Mine,
            ..theirs(7, 8, "Why?")
        };
        let mut lines = vec![
            theirs(0, 6, "Yes. No."),
            mine,
            theirs(9, 15, "Maybe. Friday."),
        ];
        let mut pieces = pieces_of(0, 0, &[0, 3]);
        pieces.extend(pieces_of(2, 1, &[9, 12]));
        let mut words = HashMap::from([
            (0, vec![word("Yes.", 1), word("No.", 4)]),
            (2, vec![word("Maybe.", 10), word("Friday.", 13)]),
        ]);
        let voices = [Some(0), Some(1), Some(1), Some(0)];
        split(&mut lines, &mut pieces, &mut words, &voices);
        split(&mut lines, &mut pieces, &mut words, &voices);
        assert_eq!(
            said(&lines),
            [
                ("Yes.", Some(0), 0, 3),
                ("No.", Some(1), 3, 6),
                ("Why?", None, 7, 8),
                ("Maybe.", Some(1), 9, 12),
                ("Friday.", Some(0), 12, 15),
            ],
            "a take after an earlier split wasn't split by its own pieces, told apart twice"
        );
        let lines_of: Vec<Option<usize>> = pieces.iter().map(|piece| piece.line).collect();
        assert_eq!(
            lines_of,
            [Some(0), Some(1), Some(3), Some(4)],
            "the later take's pieces weren't numbered past the earlier split"
        );
        let mut keys: Vec<usize> = words.keys().copied().collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [0, 1, 3, 4],
            "the words aren't kept by the lines they're in"
        );
    }
}
