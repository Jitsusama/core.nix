//! Matching by words as the shell's launcher matches
//! (modules/quickshell/Fuzzy.js), so a search the daemon answers finds
//! and ranks its notices as the launcher would have: each word typed has
//! to start a word of the text, its letters unbroken, in any order, and
//! each landing is scored by fzf's constants. It works on UTF-16 code
//! units, as Qt's engine does, so a score agrees with Fuzzy.js's letter
//! for letter; the cases both are held to are one file,
//! modules/quickshell/fuzzy-scores.json.

const SCORE_MATCH: i32 = 16;
const GAP_START: i32 = -3;
const GAP_EXTENSION: i32 = -1;
const BONUS_BOUNDARY: i32 = SCORE_MATCH / 2;
const BONUS_NON_WORD: i32 = SCORE_MATCH / 2;
const BONUS_CAMEL: i32 = BONUS_BOUNDARY + GAP_EXTENSION;
const BONUS_CONSECUTIVE: i32 = -(GAP_START + GAP_EXTENSION);
const BONUS_FIRST_LETTER: i32 = 2;
const BONUS_BOUNDARY_WHITE: i32 = BONUS_BOUNDARY + 2;
const BONUS_BOUNDARY_DELIMITER: i32 = BONUS_BOUNDARY + 1;
/// Fuzzy.js's own, not fzf's: a word typed whole beats one it only
/// begins, so "pi" finds pi before a picture.
const BONUS_WHOLE: i32 = BONUS_BOUNDARY;

const HYPHEN: u16 = 0x2d;

/// The kinds of character a bonus depends on, in fzf's order: anything
/// past `NonWord` is part of a word.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    White,
    NonWord,
    Delimiter,
    Lower,
    Upper,
    Letter,
    Number,
}

/// Whether a code unit is one JavaScript's `\s` matches.
fn spacing(unit: u16) -> bool {
    matches!(
        unit,
        0x09..=0x0d
            | 0x20
            | 0xa0
            | 0x1680
            | 0x2000..=0x200a
            | 0x2028
            | 0x2029
            | 0x202f
            | 0x205f
            | 0x3000
            | 0xfeff
    )
}

/// A code unit's lower case as JavaScript's `toLowerCase` says it, or the
/// unit itself where that's more than one unit, as Fuzzy.js's `lowerOf`.
fn lower_of(unit: u16) -> u16 {
    let Some(c) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mut lower = [0; 2];
    let mut lowered = c.to_lowercase();
    match (lowered.next(), lowered.next()) {
        (Some(one), None) => match one.encode_utf16(&mut lower) {
            [only] => *only,
            _ => unit,
        },
        _ => unit,
    }
}

/// Whether `c`, as a string, is changed by a case mapping.
fn changes(c: char, mut mapped: impl Iterator<Item = char>) -> bool {
    !(mapped.next() == Some(c) && mapped.next().is_none())
}

fn kind_of(unit: u16) -> Kind {
    match unit {
        0x20 | 0x09 | 0x0a => Kind::White,
        0x30..=0x39 => Kind::Number,
        0x61..=0x7a => Kind::Lower,
        0x41..=0x5a => Kind::Upper,
        0x2f | 0x2c | 0x3a | 0x3b | 0x7c => Kind::Delimiter,
        0..=0x7f => Kind::NonWord,
        // Half of a pair (None) is a letter too, which no case mapping
        // changes and `\s` never matches.
        _ => match char::from_u32(u32::from(unit)) {
            Some(c) if changes(c, c.to_lowercase()) => Kind::Upper,
            Some(c) if changes(c, c.to_uppercase()) => Kind::Lower,
            Some(_) if spacing(unit) => Kind::White,
            Some(_) | None => Kind::Letter,
        },
    }
}

/// What landing on a character is worth, given the one before it. A
/// capital after a digit counts as a new word too, so the P of
/// "1Password" is a start.
fn bonus_for(before: Kind, kind: Kind) -> i32 {
    if kind > Kind::NonWord {
        match before {
            Kind::White => return BONUS_BOUNDARY_WHITE,
            Kind::Delimiter => return BONUS_BOUNDARY_DELIMITER,
            Kind::NonWord => return BONUS_BOUNDARY,
            _ => {}
        }
    }
    match (before, kind) {
        (Kind::Lower | Kind::Number, Kind::Upper) => BONUS_CAMEL,
        (before, Kind::Number) if before != Kind::Number => BONUS_CAMEL,
        (_, Kind::NonWord | Kind::Delimiter) => BONUS_NON_WORD,
        (_, Kind::White) => BONUS_BOUNDARY_WHITE,
        _ => 0,
    }
}

/// Which letters a text holds, one bit each for a to z and one for any
/// digit, anything else sharing the last.
fn bit_of(unit: u16) -> u32 {
    match unit {
        0x61..=0x7a => 1 << (unit - 0x61),
        0x30..=0x39 => 1 << 26,
        _ => 1 << 27,
    }
}

/// A letter or a digit, either side of a hyphen that joins a word, as
/// Fuzzy.js's `JOINED` says.
fn joined(unit: u16) -> bool {
    matches!(unit, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a | 0xc0..=0x24f)
}

/// A text with each hyphen inside a word taken out, and where each of its
/// letters is in the text, with its bonus as the closed-up word has it.
#[derive(Debug)]
struct Closed {
    text: Vec<u16>,
    at: Vec<usize>,
    bonus: Vec<i32>,
}

/// A text ready to be matched: its letters in lower case, one for one so
/// places line up, each letter's bonus, which letters it holds and which
/// its words start with, and its closed-up form where a hyphen joins a
/// word.
#[derive(Debug)]
pub struct Prepared {
    folded: Vec<u16>,
    bonus: Vec<i32>,
    mask: u32,
    starts: u32,
    closed: Option<Closed>,
}

impl Prepared {
    pub fn of(text: &str) -> Prepared {
        let source: Vec<u16> = text.encode_utf16().collect();
        let mut bonus = Vec::with_capacity(source.len());
        let mut codes = Vec::with_capacity(source.len());
        let (mut mask, mut starts) = (0, 0);
        let mut before = Kind::White;
        for &unit in &source {
            let kind = kind_of(unit);
            let worth = bonus_for(before, kind);
            before = kind;
            let code = lower_of(unit);
            mask |= bit_of(code);
            if worth > 0 {
                starts |= bit_of(code);
            }
            bonus.push(worth);
            codes.push(code);
        }
        // The whole text's lower case, where it's one for one, as the
        // engine's own is; otherwise each unit's.
        let lower: Vec<u16> = text.to_lowercase().encode_utf16().collect();
        let folded = if lower.len() == source.len() {
            lower
        } else {
            codes
        };
        let closed = closed_up(&source, &folded, &bonus);
        Prepared {
            folded,
            bonus,
            mask,
            starts,
            closed,
        }
    }

    /// How well it answers `words`, each landing anywhere: the sum of
    /// each word's best landing, or nothing when any word starts none of
    /// its words.
    pub fn score(&self, words: &Words) -> Option<i32> {
        if self.mask & words.letters != words.letters
            || words.each.iter().any(|word| {
                word.first()
                    .is_some_and(|&first| self.starts & bit_of(first) == 0)
            })
        {
            return None;
        }
        words.each.iter().map(|word| self.align(word)).sum()
    }

    /// The best landing of one word, which must start a word of the text.
    fn align(&self, word: &[u16]) -> Option<i32> {
        if word.is_empty() {
            return Some(0);
        }
        landing(&self.folded, &self.bonus, word).or_else(|| {
            self.closed
                .as_ref()
                .and_then(|closed| landing(&closed.text, &closed.bonus, word))
        })
    }
}

fn closed_up(source: &[u16], folded: &[u16], bonus: &[i32]) -> Option<Closed> {
    if !folded.contains(&HYPHEN) {
        return None;
    }
    let n = source.len();
    let inside = |j: usize| {
        folded[j] == HYPHEN && j > 0 && j < n - 1 && joined(source[j - 1]) && joined(source[j + 1])
    };
    if !(1..n.saturating_sub(1)).any(inside) {
        return None;
    }
    let mut closed = Closed {
        text: Vec::with_capacity(n),
        at: Vec::with_capacity(n),
        bonus: Vec::with_capacity(n),
    };
    for j in (0..n).filter(|&j| !inside(j)) {
        let apart = closed.at.last().is_some_and(|&last| j != last + 1);
        closed.bonus.push(if apart { 0 } else { bonus[j] });
        closed.at.push(j);
        closed.text.push(folded[j]);
    }
    Some(closed)
}

/// The best place `word` starts a word in `text`, scored.
fn landing(text: &[u16], bonus: &[i32], word: &[u16]) -> Option<i32> {
    let m = word.len();
    if m > text.len() {
        return None;
    }
    (0..=text.len() - m)
        .filter(|&at| bonus[at] > 0 && text[at..at + m] == *word)
        .map(|at| worth(bonus, at, m, !broken(text, at)))
        .max()
}

/// What a word is worth landing at `at`: fzf's score for a run of
/// letters, the first worth more where a stronger word starts and
/// doubled, each later one keeping the bonus of the start of its run,
/// unless a stronger word start inside it begins a new one. A run ending
/// where its word ends is worth more again, unless an apostrophe broke
/// that word.
fn worth(bonus: &[i32], at: usize, m: usize, whole: bool) -> i32 {
    let mut opened = bonus[at];
    let mut score = SCORE_MATCH + opened * BONUS_FIRST_LETTER;
    for &b in &bonus[at + 1..at + m] {
        if b >= BONUS_BOUNDARY && b > opened {
            opened = b;
            score += SCORE_MATCH + b;
        } else {
            score += SCORE_MATCH + b.max(BONUS_CONSECUTIVE).max(opened);
        }
    }
    if whole && bonus.get(at + m).is_none_or(|&after| after > 0) {
        score += BONUS_WHOLE;
    }
    score
}

/// Whether the letter at `at` follows an apostrophe inside a word, as the
/// s of "what's" does.
fn broken(text: &[u16], at: usize) -> bool {
    at > 1 && matches!(text[at - 1], 0x27 | 0x2019) && kind_of(text[at - 2]) > Kind::Delimiter
}

/// The words of what's typed, in lower case, and the letters they hold.
#[derive(Debug)]
pub struct Words {
    each: Vec<Vec<u16>>,
    letters: u32,
}

impl Words {
    pub fn of(typed: &str) -> Words {
        let lower: Vec<u16> = typed.to_lowercase().encode_utf16().collect();
        let each: Vec<Vec<u16>> = lower
            .split(|&unit| spacing(unit))
            .filter(|word| !word.is_empty())
            .map(<[u16]>::to_vec)
            .collect();
        let letters = each
            .iter()
            .flatten()
            .fold(0, |all, &unit| all | bit_of(unit));
        Words { each, letters }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn each_case_scores_as_fuzzy_js_does() {
        let cases: Value = serde_json::from_str(include_str!("../../quickshell/fuzzy-scores.json"))
            .expect("fuzzy-scores.json reads");
        let cases = cases["cases"].as_array().expect("its cases");
        assert!(cases.len() > 20, "there are cases to hold it to");
        for case in cases {
            let (text, typed) = (case["text"].as_str().unwrap_or(""), case["typed"].as_str());
            let typed = typed.unwrap_or("");
            let want = case["score"].as_i64().and_then(|s| i32::try_from(s).ok());
            let got = Prepared::of(text).score(&Words::of(typed)).unwrap_or(-1);
            assert_eq!(Some(got), want, "{typed:?} in {text:?}");
        }
    }
}
