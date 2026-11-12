//! Matching by words, as the shell's Fuzzy.js matches them, so a daemon
//! answering a search finds and ranks exactly what the launcher would:
//! each word typed has to start a word of the text, its letters
//! unbroken, in any order and any case, and matches rank by fzf's
//! constants, a word typed whole beating one it only begins. Fuzzy.js
//! says why each rule is; this follows it line for line.
//!
//! It counts in UTF-16 code units, as Qt's JavaScript engine indexes a
//! string, so a score and the letters a match brings forward are the
//! shell's own, and a mark lands on the letter the shell draws. The cases
//! both are held to are `fuzzy.json`: every text there against every
//! query, made by Fuzzy.js itself, and read again by the shell's
//! `the-launcher-finds-what-is-meant`.

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
// Ours, not fzf's: a word typed whole beats one it only begins, so "pi"
// finds pi before a picture.
const BONUS_WHOLE: i32 = BONUS_BOUNDARY;

const HYPHEN: u16 = 0x2d;
const APOSTROPHE: u16 = 0x27;
const RIGHT_QUOTE: u16 = 0x2019;

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

/// What JavaScript's `\s` matches: Rust's whitespace but for the next
/// line character, and the byte order mark beside it.
fn spacing(c: char) -> bool {
    c == '\u{feff}' || (c.is_whitespace() && c != '\u{85}')
}

/// One code unit as JavaScript's case mapping gives it, where that is one
/// code unit too.
fn one_unit(mapped: impl Iterator<Item = char>) -> Option<u16> {
    let mut units = [0; 2];
    let mut out = None;
    for (at, c) in mapped.enumerate() {
        if at > 0 || c.encode_utf16(&mut units).len() > 1 {
            return None;
        }
        out = Some(units[0]);
    }
    out
}

fn kind_of(unit: u16) -> Kind {
    match unit {
        0x20 | 0x09 | 0x0a => Kind::White,
        0x30..=0x39 => Kind::Number,
        0x61..=0x7a => Kind::Lower,
        0x41..=0x5a => Kind::Upper,
        0x2f | 0x2c | 0x3a | 0x3b | 0x7c => Kind::Delimiter,
        0..=0x7f => Kind::NonWord,
        // Half of a pair is a letter too: no case changes it, and `\s`
        // isn't it.
        _ => match char::from_u32(unit.into()) {
            Some(c) if !c.to_lowercase().eq([c]) => Kind::Upper,
            Some(c) if !c.to_uppercase().eq([c]) => Kind::Lower,
            Some(c) if spacing(c) => Kind::White,
            _ => Kind::Letter,
        },
    }
}

/// What landing on a character is worth, given the one before it.
fn bonus_for(before: Kind, kind: Kind) -> i32 {
    if kind > Kind::NonWord {
        match before {
            Kind::White => return BONUS_BOUNDARY_WHITE,
            Kind::Delimiter => return BONUS_BOUNDARY_DELIMITER,
            Kind::NonWord => return BONUS_BOUNDARY,
            _ => {}
        }
    }
    if (before == Kind::Lower || before == Kind::Number) && kind == Kind::Upper {
        return BONUS_CAMEL;
    }
    if before != Kind::Number && kind == Kind::Number {
        return BONUS_CAMEL;
    }
    match kind {
        Kind::NonWord | Kind::Delimiter => BONUS_NON_WORD,
        Kind::White => BONUS_BOUNDARY_WHITE,
        _ => 0,
    }
}

/// A bit each for a to z, one for any digit, and one shared by the rest.
fn bit_of(unit: u16) -> u32 {
    match unit {
        0x61..=0x7a => 1 << (unit - 0x61),
        0x30..=0x39 => 1 << 26,
        _ => 1 << 27,
    }
}

/// A text in lower case as Qt's engine makes it, each letter alone: a
/// sigma ending a word stays σ, not the ς that `str::to_lowercase` and
/// other engines make of it, so "σασ" is found in "ΣΑΣ" as the shell
/// finds it.
fn lowered(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// A code unit in lower case, where its lower case is one code unit.
fn lower_of(unit: u16) -> u16 {
    if unit < 0x80 {
        return u16::from(u8::try_from(unit).map_or(0, |b| b.to_ascii_lowercase()));
    }
    char::from_u32(unit.into())
        .and_then(|c| one_unit(c.to_lowercase()))
        .unwrap_or(unit)
}

/// A code unit as a JavaScript pattern without `u` compares it ignoring
/// case: in upper case, unless that takes more than one unit or takes a
/// letter past ASCII into it.
fn canonical(unit: u16) -> u16 {
    let upper = char::from_u32(unit.into())
        .and_then(|c| one_unit(c.to_uppercase()))
        .unwrap_or(unit);
    if unit >= 0x80 && upper < 0x80 {
        unit
    } else {
        upper
    }
}

/// Whether a code unit is a letter or a digit either side of a hyphen
/// that joins a word, as Fuzzy.js's `/[a-z0-9\u00c0-\u024f]/i` says.
fn joined(unit: u16) -> bool {
    const LATIN: std::ops::RangeInclusive<u16> = 0xc0..=0x24f;
    if unit < 0x80 {
        return u8::try_from(unit).is_ok_and(|b| b.is_ascii_alphanumeric());
    }
    if LATIN.contains(&unit) {
        return true;
    }
    let found = canonical(unit);
    LATIN.into_iter().any(|letter| canonical(letter) == found)
}

/// The words of a text in lower case, split where JavaScript's `\s+`
/// splits, as Fuzzy.js's `words` gives them.
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    lowered(text)
        .split(spacing)
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What's typed, made ready to match: its words in lower case, and the
/// letters they hold.
pub struct Typed {
    words: Vec<Vec<u16>>,
    need: u32,
}

impl Typed {
    /// The words of what's typed, split where JavaScript's `\s+` splits.
    #[must_use]
    pub fn of(typed: &str) -> Typed {
        let words: Vec<Vec<u16>> = words(typed)
            .iter()
            .map(|word| word.encode_utf16().collect())
            .collect();
        let need = words
            .iter()
            .flatten()
            .fold(0, |mask, &unit| mask | bit_of(unit));
        Typed { words, need }
    }

    /// Whether nothing was typed but spaces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether each word starts with a letter some word of the text does.
    fn begins(&self, starts: u32) -> bool {
        self.words.iter().all(|word| starts & bit_of(word[0]) != 0)
    }
}

/// A text written with a hyphen inside a word, closed up: its letters,
/// where each is in the text, and their bonuses as the closed-up word
/// has them.
struct Closed {
    text: Vec<u16>,
    at: Vec<usize>,
    bonus: Vec<i32>,
}

/// A text made ready to match, once: its letters in lower case, one for
/// one, so positions line up, each letter's bonus, which letters it
/// holds and which its words start with.
pub struct Item {
    folded: Vec<u16>,
    bonus: Vec<i32>,
    mask: u32,
    starts: u32,
    closed: Option<Closed>,
}

impl Item {
    #[must_use]
    pub fn of(text: &str) -> Item {
        let source: Vec<u16> = text.encode_utf16().collect();
        let mut bonus = Vec::with_capacity(source.len());
        let mut codes = Vec::with_capacity(source.len());
        let (mut mask, mut starts, mut before) = (0, 0, Kind::White);
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
        // The whole text's own lower case, wherever it is one for one, as
        // it is but for a letter whose lower case is two.
        let lower: Vec<u16> = lowered(text).encode_utf16().collect();
        let folded = if lower.len() == source.len() {
            lower
        } else {
            codes
        };
        let closed = closed_up(&source, &folded, &bonus);
        Item {
            folded,
            bonus,
            mask,
            starts,
            closed,
        }
    }

    /// How well it answers what's typed, the words landing in any order:
    /// the sum of each word's best landing, or nothing if any word starts
    /// none of its words.
    #[must_use]
    pub fn score(&self, typed: &Typed) -> Option<i32> {
        if self.mask & typed.need != typed.need || !typed.begins(self.starts) {
            return None;
        }
        typed
            .words
            .iter()
            .try_fold(0, |total, word| align(self, word, None).map(|s| total + s))
    }

    /// Its score, with the letters to bring forward, in order.
    #[must_use]
    pub fn marked(&self, typed: &Typed) -> Option<(i32, Vec<usize>)> {
        let mut marks = Vec::new();
        let mut total = 0;
        for word in &typed.words {
            total += align(self, word, Some(&mut marks))?;
        }
        marks.sort_unstable();
        marks.dedup();
        Some((total, marks))
    }
}

/// The text with each hyphen inside a word taken out, or nothing for a
/// text without one.
fn closed_up(source: &[u16], folded: &[u16], bonus: &[i32]) -> Option<Closed> {
    if !folded.contains(&HYPHEN) {
        return None;
    }
    let n = source.len();
    let inside = |j: usize| {
        folded[j] == HYPHEN && j > 0 && j + 1 < n && joined(source[j - 1]) && joined(source[j + 1])
    };
    if !(1..n.saturating_sub(1)).any(inside) {
        return None;
    }
    let mut closed = Closed {
        text: Vec::new(),
        at: Vec::new(),
        bonus: Vec::new(),
    };
    for j in (0..n).filter(|&j| !inside(j)) {
        let follows = closed.at.last().is_none_or(|&last| j == last + 1);
        closed.bonus.push(if follows { bonus[j] } else { 0 });
        closed.at.push(j);
        closed.text.push(folded[j]);
    }
    Some(closed)
}

/// What a word is worth landing at `at`: fzf's score for a run of
/// letters, the first worth more where a stronger word starts and
/// doubled, each later one keeping the bonus of the start of its run,
/// unless a stronger word start inside it begins a new one, and more
/// again where the run ends where its word does.
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
    if whole && bonus.get(at + m).is_none_or(|&b| b > 0) {
        score += BONUS_WHOLE;
    }
    score
}

/// Whether the letter at `at` follows an apostrophe inside a word, as
/// the s of "what's" does, so it's never a word typed whole.
fn broken(text: &[u16], at: usize) -> bool {
    at > 1
        && (text[at - 1] == APOSTROPHE || text[at - 1] == RIGHT_QUOTE)
        && kind_of(text[at - 2]) > Kind::Delimiter
}

/// Every place `word` stands in `text`, first to last.
fn places<'a>(text: &'a [u16], word: &'a [u16]) -> impl Iterator<Item = usize> + 'a {
    text.windows(word.len())
        .enumerate()
        .filter(move |(_, w)| *w == word)
        .map(|(at, _)| at)
}

/// The best place one word starts a word in `text`, and its score.
fn best_in(text: &[u16], bonus: &[i32], word: &[u16]) -> Option<(usize, i32)> {
    let m = word.len();
    let mut best: Option<(usize, i32)> = None;
    for at in places(text, word) {
        if bonus[at] <= 0 {
            continue;
        }
        let score = worth(bonus, at, m, !broken(text, at));
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((at, score));
        }
    }
    best
}

/// The best place one word (in lower case) starts a word in an item:
/// its score, putting where each letter landed into `marks`.
fn align(item: &Item, word: &[u16], marks: Option<&mut Vec<usize>>) -> Option<i32> {
    let m = word.len();
    if m == 0 {
        return Some(0);
    }
    if let Some((at, score)) = best_in(&item.folded, &item.bonus, word) {
        if let Some(marks) = marks {
            marks.extend(at..at + m);
        }
        return Some(score);
    }
    let closed = item.closed.as_ref()?;
    let (at, score) = best_in(&closed.text, &closed.bonus, word)?;
    if let Some(marks) = marks {
        marks.extend(closed.at[at..at + m].iter().copied());
    }
    Some(score)
}

/// The letters a match brought forward as runs, half-open, the letters
/// side by side run together, as Fuzzy.js's `runs` gives them.
#[must_use]
pub fn runs(positions: &[usize]) -> Vec<[usize; 2]> {
    let mut out: Vec<[usize; 2]> = Vec::new();
    for &at in positions {
        match out.last_mut() {
            Some(last) if last[1] == at => last[1] = at + 1,
            _ => out.push([at, at + 1]),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn every_case_scores_and_marks_as_the_shells_fuzzy_does() {
        let cases: Value =
            serde_json::from_str(include_str!("../fuzzy.json")).expect("fuzzy.json reads");
        let texts: Vec<&str> = cases["texts"]
            .as_array()
            .expect("a list of texts")
            .iter()
            .map(|text| text.as_str().expect("each text is a string"))
            .collect();
        let items: Vec<Item> = texts.iter().map(|text| Item::of(text)).collect();
        let found = cases["found"].as_object().expect("what each query finds");
        assert!(
            found.len() > 50,
            "the cases hold many queries, not {}",
            found.len()
        );
        for (query, hits) in found {
            let typed = Typed::of(query);
            let mut expected = vec![None; texts.len()];
            for hit in hits.as_array().expect("a query's hits") {
                let at = usize::try_from(hit[0].as_u64().expect("a text's place")).unwrap_or(0);
                let score = i32::try_from(hit[1].as_i64().expect("a score")).unwrap_or(-1);
                let marks: Vec<usize> = hit[2]
                    .as_array()
                    .expect("its marks")
                    .iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|mark| usize::try_from(mark).ok())
                    .collect();
                expected[at] = Some((score, marks));
            }
            for (at, item) in items.iter().enumerate() {
                let text = texts[at];
                let want = expected[at].clone();
                assert_eq!(
                    item.marked(&typed),
                    want,
                    "{query:?} in {text:?}, with its marks"
                );
                let score = want.map(|(score, _)| score);
                assert_eq!(item.score(&typed), score, "{query:?} in {text:?}");
            }
        }
    }

    #[test]
    fn marks_run_together_as_the_shell_washes_them() {
        let item = Item::of("Open a terminal");
        let (_, marks) = item
            .marked(&Typed::of("op term"))
            .expect("it finds the terminal");
        assert_eq!(runs(&marks), vec![[0, 2], [7, 11]], "two runs, one a word");
        assert!(runs(&[]).is_empty(), "no marks are no runs");
    }

    #[test]
    fn only_spaces_typed_is_nothing_typed() {
        assert!(Typed::of(" \u{a0}\t").is_empty(), "spaces are no words");
        assert!(!Typed::of(" a ").is_empty(), "a word is");
        assert_eq!(
            Item::of("Open a terminal").marked(&Typed::of("")),
            Some((0, vec![])),
            "nothing typed finds every text, marking nothing"
        );
    }
}
