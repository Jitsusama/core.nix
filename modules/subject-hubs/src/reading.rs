//! How a need's words are read, and what a subject's word is said by, by
//! one rule with the harness's routing: its `wordsOf`, `subjectWords` and
//! `saysWord` in `lib/mesh/need-route.ts`. A hub core's senders read takes
//! what the harness's own routing would only while the two read words
//! alike, so each piece here names its twin there, and the case table in
//! `tests.rs` pins the harness's own cases for `wordsOf`.

/// Words shorter than this tell no cell's work from another's: the
/// harness's `SHORTEST_WORD`. Measured as the harness measures a word
/// (`length`).
const SHORTEST_WORD: usize = 3;

/// The longest word a need keeps, since longer is a token rather than a
/// word of work: the harness's `LONGEST_WORD`, measured as it measures
/// one (`length`).
const LONGEST_WORD: usize = 32;

/// Words that say nothing about any work, left out of a need's words and
/// a subject's alike: the harness's `COMMON`, group by group, so the two
/// lists are compared at a glance. Without it a subject such as
/// `for-the-dock` needs "for" and "the" said to take a thing, and a need
/// is matched by words any sentence has. Kept inside an id, which names
/// its work whole.
const COMMON: &[&str] = &[
    // Determiners and pronouns.
    "a all an another any anyone anything both each either every everyone",
    "everything few her hers herself him himself his its itself many mine",
    "more most much myself neither nobody none nothing one other others our",
    "ours ourselves own same several she some someone something such that",
    "the their theirs them themselves these they this those what whatever",
    "which who whom whose you your yours yourself",
    // Prepositions.
    "about above across after against along among around before behind",
    "below beneath beside besides between beyond during except for from",
    "inside into near off onto out outside over past per since than",
    "through throughout till toward towards under underneath until unto",
    "upon via with within without",
    // Conjunctions.
    "although and because but how however nor once though unless when",
    "whenever where whereas wherever whether while why yet",
    // Helping verbs, and a contraction's stubs.
    "are aren been being can cannot could couldn did didn does doesn doing",
    "don had hadn has hasn have haven having isn may might must mustn",
    "shall should shouldn was wasn were weren will won would wouldn",
    // Adverbs any sentence has.
    "again already also always even ever here instead just maybe never not",
    "now often only perhaps please quite rather really soon still then",
    "there too very",
    // Those of two letters, which only a number after them is read beside:
    // "on 24" names no work, where "PR 42" and "v 2" do.
    "am as at be by do he if in is it me my no of on or so to up us we",
];

/// The particles a word before them makes a phrase of, as in hand over:
/// the harness's `PARTICLES`. Over and off are common words, so without
/// the phrase "hand over" says only hand.
const PARTICLES: &[&str] = &["over", "off", "out", "up", "down", "back", "away"];

/// What prose writes for an apostrophe besides the plain one, read as the
/// plain one: the curly single quotes, the modifier letter, which would
/// otherwise join a contraction into one word, and the acute accent. The
/// harness's `APOSTROPHE_LIKE`.
const APOSTROPHE_LIKE: &[char] = &['\u{2018}', '\u{2019}', '\u{2BC}', '\u{B4}'];

/// How long the harness's `length` says `word` is: in UTF-16 code units,
/// so a letter past the Basic Multilingual Plane counts two, and the
/// shortest and longest words are cut where the harness cuts them.
fn length(word: &str) -> usize {
    word.encode_utf16().count()
}

/// Whether `word` is one of `COMMON`.
fn common(word: &str) -> bool {
    COMMON
        .iter()
        .flat_map(|group| group.split(' '))
        .any(|c| c == word)
}

/// Whether a word may name work, long enough and not common: the
/// harness's `isWorkWord`.
fn work_word(word: &str) -> bool {
    length(word) >= SHORTEST_WORD && !common(word)
}

/// Whether `c` is a letter or a digit, which the harness reads as
/// Unicode's letter and number categories. Rust's standard library knows
/// no categories, so its alphabetic stands in for a letter, less the
/// circled and squared Latin letters it counts, which are symbols to the
/// harness. It still counts the vowel signs some scripts write as marks,
/// at which the harness splits a word.
fn letter_or_digit(c: char) -> bool {
    (c.is_alphabetic() || c.is_numeric())
        && !matches!(c, '\u{24B6}'..='\u{24E9}' | '\u{1F130}'..='\u{1F189}')
}

/// Whether `c` is a letter, rather than a digit or a numeral written as
/// a letter, such as Ⅻ, which the harness reads as a number.
fn letter(c: char) -> bool {
    letter_or_digit(c) && !c.is_numeric()
}

/// Whether `c` is white space as the harness's `\s` reads it: Unicode's
/// white space, less the next-line control and with the byte order mark.
fn space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// A run of letters and digits, by where it starts and ends in the text.
#[derive(Debug, Clone, Copy)]
struct Run {
    start: usize,
    end: usize,
}

/// Each run of letters and digits in `text`, in order.
fn runs(text: &[char]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (at, c) in text.iter().enumerate() {
        if !letter_or_digit(*c) {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.end == at => run.end = at + 1,
            _ => runs.push(Run {
                start: at,
                end: at + 1,
            }),
        }
    }
    runs
}

/// The text from where `first` starts to where `last` ends.
fn span(text: &[char], first: Run, last: Run) -> String {
    text[first.start..last.end].iter().collect()
}

/// Whether `run` starts clear of a join, so it isn't a joined id's later
/// part, whose number or particle would join that part alone.
fn starts_clear(text: &[char], run: Run) -> bool {
    run.start == 0 || !matches!(text[run.start - 1], '-' | '_')
}

/// The run after the one at `at`, where only white space stands between.
fn after_space(text: &[char], runs: &[Run], at: usize) -> Option<Run> {
    let next = *runs.get(at + 1)?;
    text[runs[at].end..next.start]
        .iter()
        .all(|c| space(*c))
        .then_some(next)
}

/// Each id written as runs joined by hyphens or underscores, whole, as
/// the harness's `JOINED_ID` reads one: leg-26, I-5 or `node_kubelet_cri`.
/// Split into words, I-5 gives none and leg-26 only leg, which would take
/// a need for leg 27, so each is kept whole beside its words.
fn joined_ids(text: &[char], runs: &[Run]) -> Vec<String> {
    let joins = |a: Run, b: Run| b.start == a.end + 1 && matches!(text[a.end], '-' | '_');
    let mut ids = Vec::new();
    let mut first = 0;
    while first < runs.len() {
        let mut last = first;
        while runs
            .get(last + 1)
            .is_some_and(|next| joins(runs[last], *next))
        {
            last += 1;
        }
        if last > first {
            ids.push(span(text, runs[first], runs[last]));
        }
        first = last + 1;
    }
    ids
}

/// Each word and the number after it in prose, "leg 26" or "PR 42", as
/// their id, leg-26 or pr-42, as the harness's `NUMBER_IN_PROSE` reads
/// one; split into words the number is lost, being short. Not a common
/// word's, since "on 24" names no work, nor a contraction's stub's, since
/// "it's 5" names no s-5, nor a joined id's last part's.
fn numbered_ids(text: &[char], runs: &[Run]) -> Vec<String> {
    let after_stub =
        |run: Run| run.start >= 2 && text[run.start - 1] == '\'' && letter(text[run.start - 2]);
    (0..runs.len())
        .filter_map(|at| {
            let run = runs[at];
            let number = after_space(text, runs, at)?;
            let word = span(text, run, run);
            let reads = letter(text[run.start])
                && starts_clear(text, run)
                && !after_stub(run)
                && text[number.start..number.end]
                    .iter()
                    .all(|c| c.is_numeric())
                && !common(&word);
            reads.then(|| format!("{word}-{}", span(text, number, number)))
        })
        .collect()
}

/// Each word of work and the particle after it in prose, "hand over", as
/// their id, hand-over, as the harness's `PARTICLE_IN_PROSE` reads one,
/// so a hub of hand-over takes a need saying hand over and no other.
fn phrased_ids(text: &[char], runs: &[Run]) -> Vec<String> {
    (0..runs.len())
        .filter_map(|at| {
            let run = runs[at];
            let next = after_space(text, runs, at)?;
            let particle = span(text, next, next);
            let ends_clear = text.get(next.end).is_none_or(|c| !matches!(*c, '-' | '_'));
            let word = span(text, run, run);
            let reads = letter(text[run.start])
                && starts_clear(text, run)
                && PARTICLES.contains(&particle.as_str())
                && ends_clear
                && work_word(&word);
            reads.then(|| format!("{word}-{particle}"))
        })
        .collect()
}

/// The words `said` gives, as the harness's `wordsAndIdsOf` reads them,
/// keeping of its ids those no longer than `longest`: lower case, its
/// apostrophes made plain, each once, its words of work first, then its
/// joined ids, its words with a number after them, and its words with a
/// particle after them.
fn read(said: &str, longest: usize) -> Vec<String> {
    let text: Vec<char> = said
        .to_lowercase()
        .chars()
        .map(|c| {
            if APOSTROPHE_LIKE.contains(&c) {
                '\''
            } else {
                c
            }
        })
        .collect();
    let runs = runs(&text);
    let plain = runs
        .iter()
        .map(|run| span(&text, *run, *run))
        .filter(|word| work_word(word));
    let ids = joined_ids(&text, &runs)
        .into_iter()
        .chain(numbered_ids(&text, &runs))
        .chain(phrased_ids(&text, &runs))
        .filter(|id| length(id) <= longest);
    let mut words: Vec<String> = Vec::new();
    for word in plain.chain(ids) {
        if !words.contains(&word) {
            words.push(word);
        }
    }
    words
}

/// The words a need, a key or his words give, as the harness's `wordsOf`
/// reads a need's: no id past `LONGEST_WORD`, which no route keeps.
#[must_use]
pub fn words(said: &str) -> Vec<String> {
    read(said, LONGEST_WORD)
}

/// The words of a subject, read as a need's are but keeping every id
/// however long, as the harness's `subjectWords` keeps them: a hub is
/// matched by every word of its subject, so one left out would let it take
/// a need saying only the subject's shorter words. Unlike `subjectWords`,
/// they're read from every part of the subject, not its narrowest alone,
/// since the subject of most words is the narrowest here. A subject whose
/// narrowest part, after its last colon, dot or slash, gives no word gives
/// none, as the harness's `hubTakesNone` says its hub takes nothing: read
/// whole, `launcher:for` would take everything `launcher` does.
pub(crate) fn subject_words(subject: &str) -> Vec<String> {
    let narrowest = subject
        .rsplit([':', '.', '/'])
        .find(|part| !part.is_empty())
        .unwrap_or_default();
    if read(narrowest, usize::MAX).is_empty() {
        return Vec::new();
    }
    read(subject, usize::MAX)
}

/// Whether `id` has a hyphen or an underscore, so is a joined id.
fn joined(id: &str) -> bool {
    id.contains(['-', '_'])
}

/// Whether `id` is a joined id with a part of digits alone, as leg-26 is.
fn numbered(id: &str) -> bool {
    joined(id)
        && id
            .split(['-', '_'])
            .any(|part| !part.is_empty() && part.chars().all(char::is_numeric))
}

/// Whether joined id `id` is said inside a longer joined id of `said`, as
/// leg-26 is inside leg-26-follow-up: the harness's `saidInside`.
fn said_inside(id: &str, said: &[String]) -> bool {
    let bounded = |of: &str| format!("-{}-", of.replace('_', "-"));
    let inside = bounded(id);
    said.iter()
        .any(|other| other != id && joined(other) && bounded(other.as_str()).contains(&inside))
}

/// Whether `said`, a need's words, says `word` of a subject, as the
/// harness's `saysWord` judges it: said whole; or, for a joined id, said
/// inside a longer one, or part by part, its parts of work each said, so
/// "the dock" says for-the-dock. A joined id with a number is said only
/// whole or inside another, since its number said apart may belong to
/// anything: neither "leg 27" nor "leg 27 in 100 runs" says leg-26.
pub(crate) fn says(word: &str, said: &[String]) -> bool {
    if said.iter().any(|w| w == word) {
        return true;
    }
    if !joined(word) {
        return false;
    }
    if said_inside(word, said) {
        return true;
    }
    if numbered(word) {
        return false;
    }
    let mut needed = word
        .split(['-', '_'])
        .filter(|part| work_word(part))
        .peekable();
    needed.peek().is_some() && needed.all(|part| said.iter().any(|w| w == part))
}

/// The words of `said` routing reads, none past `LONGEST_WORD`: the
/// harness routes a need by those alone, so a subject's word longer than
/// the cap is said by no need, even one saying it whole.
pub(crate) fn routed(said: &[String]) -> Vec<String> {
    said.iter()
        .filter(|word| length(word) <= LONGEST_WORD)
        .cloned()
        .collect()
}
