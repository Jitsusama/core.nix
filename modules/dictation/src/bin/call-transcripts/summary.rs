//! Sums a call up from its transcript in nothing but its own words: a
//! language model picks the lines worth reading and the words in each
//! worth keeping, and code copies those words from the line, so every
//! quote is what was said, by whom and when.
//!
//! Every earlier summarizer wrote sentences of its own, and each bent or
//! made up what was said (Phi-3.5 and Qwen3-8B on the NPU, Qwen3-14B on
//! the processor, and Gemma 4's own sentences, judged by itself), and a
//! wrong summary is worse than none. So the model here writes nothing
//! that's kept: it names lines by number, and its copy of a line's words
//! is kept only where code finds those words, in order, in that line.
//!
//! It asks in three narrow passes, as the prototype that held on the
//! Chad and Mohit calls did (2026-11-03):
//!
//! 1. Pick: a part of the call as numbered lines, and which numbers
//!    matter for someone who missed it.
//! 2. Quote: each picked line, with the lines around it, and the words
//!    in it that carry its point, found in the line or dropped.
//! 3. Keep: whether the quote alone says something worth knowing and
//!    reads clearly without the lines around it, the whole line in view,
//!    leaving out small talk and pay. It can only drop a quote.
//!
//! Each part keeps about a quote for every two minutes it lasts, taken
//! in the order the pick ranks its lines, so a long call reads in a few
//! minutes and what's cut is what mattered least.
//!
//! What a speaker reports someone else saying is dropped by code before
//! the keep pass, from the words just before the quote, reaching back
//! into the speaker's line before (she said, he was like), from she's
//! like inside it, and from the speaker called by their own name (Joel,
//! you keep...), since the model passed such quotes as the speaker's own
//! on the Chad call.

use std::fmt::Write as _;

/// The most of a call's words one pick is asked over, in characters:
/// about a thousand tokens, which Gemma read whole and picked well from.
const PART_CHARS: usize = 4000;

/// How many lines a part's pick is asked for. Asked for eight, Gemma
/// passed over a speaker's news (an illness, a review on Thursday) that
/// mattered.
const MOST_PICKS: usize = 10;

/// How many of a part's picks are taken. Asked for eight, Gemma named
/// up to seventeen; past a few more, they cost time and add little.
const MOST_TAKEN: usize = 14;

/// How long of a call each quote kept stands for, in seconds. Kept
/// wherever a pick passed, the Chad call's 91 minutes gave 101 quotes,
/// too many to read.
const SECONDS_PER_QUOTE: u64 = 120;

/// A line of fewer words than this makes no point worth a quote.
const FEWEST_WORDS: usize = 6;

/// How many words a quote is asked to keep.
const MOST_QUOTE_WORDS: usize = 20;

/// How many it may keep: a longer copy is still the speaker's words, and
/// one past this is cut short, marked so.
const MOST_QUOTE_WORDS_TAKEN: usize = 30;

/// How many of a line's words a quote may skip between two it keeps, as
/// filler left out, each skip shown by an ellipsis. Where the model's
/// copy says it left words out with an ellipsis of its own, any number
/// may be, since the summary shows that gap as one too.
const MOST_SKIPPED: usize = 3;

/// How many of a line's words before a quote are looked at for a speaker
/// telling what someone else said.
const REPORT_REACH: usize = 6;

/// The words that tell what someone said, after who said it: she said.
const REPORTING: [&str; 5] = ["said", "says", "told", "asked", "goes"];

/// The words before a reporting word that name nobody else: the speaker
/// in their own words (I was like, I went and said) or no one (it goes,
/// that said).
const OWN: [&str; 10] = [
    "i", "we", "and", "then", "it", "that", "this", "there", "which", "what",
];

/// Someone else, said with what they said: she's like.
const SOMEONE_LIKE: [&str; 3] = ["she's", "he's", "they're"];

/// The answer each pass is given room for, in tokens.
const PICK_TOKENS: usize = 80;
const QUOTE_TOKENS: usize = 70;
const VERDICT_TOKENS: usize = 3;

/// What the others on a call are called in its transcript.
const OTHERS: &str = "Others";

/// What the summary says of itself, so nobody reads a quote as more
/// than it is.
pub const HOW_MADE: &str = "These are the lines a language model on this laptop picked as \
    worth reading, each copied word for word from the transcript.";

/// What a summary with no line picked says.
const NONE_KEPT: &str = "- No line was picked as worth reading.";

/// One paragraph of a transcript: who said it, when, and what.
#[derive(Debug, PartialEq)]
pub struct Said {
    pub who: String,
    pub stamp: String,
    pub text: String,
}

/// The paragraphs of a transcript as transcript.rs writes them,
/// `**Who**, 1:23: what was said`.
pub fn lines(transcript: &str) -> Vec<Said> {
    transcript
        .split("\n\n")
        .filter_map(|paragraph| {
            let (who, rest) = paragraph.trim().strip_prefix("**")?.split_once("**, ")?;
            let (stamp, text) = rest.split_once(": ")?;
            let timed = !stamp.is_empty()
                && stamp
                    .split(':')
                    .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
            timed.then(|| Said {
                who: who.to_owned(),
                stamp: stamp.to_owned(),
                text: text.split_whitespace().collect::<Vec<_>>().join(" "),
            })
        })
        .collect()
}

/// The summary of the call `transcript` writes down, titled `title`,
/// asking `ask` each question with the most tokens its answer may take.
/// The others on it are called `them` where the call knows who they are,
/// and Others where it doesn't.
///
/// # Errors
///
/// When `ask` fails, as it does once it's stopped; nothing is written
/// then, since a summary of part of a call reads as the whole.
pub fn summarize(
    ask: &mut dyn FnMut(&str, usize) -> Result<String, String>,
    title: &str,
    them: Option<&str>,
    transcript: &str,
) -> Result<String, String> {
    let mut said = lines(transcript);
    let me = said
        .iter()
        .map(|line| line.who.as_str())
        .find(|who| *who != OTHERS)
        .unwrap_or("Me")
        .to_owned();
    if let Some(them) = them {
        for line in said.iter_mut().filter(|line| line.who == OTHERS) {
            them.clone_into(&mut line.who);
        }
    }
    let call = Call {
        said: &said,
        me,
        them: them.map(str::to_owned),
    };
    let parts = parts(&said, PART_CHARS);
    let mut kept = Vec::new();
    for (at, part) in parts.iter().enumerate() {
        let end = parts
            .get(at + 1)
            .map_or(&said[said.len() - 1], |next| &said[next[0]]);
        let lasts = seconds(&end.stamp).saturating_sub(seconds(&said[part[0]].stamp));
        let room = ((lasts + SECONDS_PER_QUOTE / 2) / SECONDS_PER_QUOTE).max(1);
        let answer = ask(&call.pick_prompt(part), PICK_TOKENS)?;
        let mut chosen: Vec<usize> = Vec::new();
        for index in numbers(&answer) {
            if part.contains(&index) && !chosen.contains(&index) {
                chosen.push(index);
            }
        }
        let mut taken = 0;
        for index in chosen.into_iter().take(MOST_TAKEN) {
            if taken == room {
                break;
            }
            if let Some(quote) = call.quote(ask, index)? {
                kept.push((index, quote));
                taken += 1;
            }
        }
    }
    kept.sort_unstable_by_key(|(index, _)| *index);
    let kept: Vec<String> = kept.into_iter().map(|(_, quote)| quote).collect();
    let mut summary = String::new();
    if !title.is_empty() {
        let _ = write!(summary, "# {title}\n\n");
    }
    let _ = write!(summary, "{HOW_MADE}\n\n");
    if kept.is_empty() {
        summary.push_str(NONE_KEPT);
    } else {
        summary.push_str(&kept.join("\n"));
    }
    Ok(summary)
}

/// A call's lines, what Joel is called in them, and who the others are
/// where the call knows.
struct Call<'a> {
    said: &'a [Said],
    me: String,
    them: Option<String>,
}

impl Call<'_> {
    /// Who a line is said to: Joel for the others' lines, the others for
    /// his. Their names are never guessed from a meeting's title.
    fn listener(&self, speaker: &str) -> String {
        if speaker == self.me {
            self.them.clone().unwrap_or_else(|| "the others".to_owned())
        } else {
            self.me.clone()
        }
    }

    /// What the others' lines are under.
    fn others(&self) -> &str {
        self.them.as_deref().unwrap_or(OTHERS)
    }

    /// Who the call is between, for a prompt.
    fn between(&self) -> String {
        match &self.them {
            Some(them) => format!("a call between {} and {them}", self.me),
            None => format!(
                "a call between {} and the others on it, called {OTHERS}",
                self.me
            ),
        }
    }

    /// The line `index` as a quote to keep, or nothing when the model
    /// finds no point in it, its copy isn't the line's, the words are
    /// someone else's, or they aren't worth keeping.
    fn quote(
        &self,
        ask: &mut dyn FnMut(&str, usize) -> Result<String, String>,
        index: usize,
    ) -> Result<Option<String>, String> {
        let line = &self.said[index];
        if line.text.split_whitespace().count() < FEWEST_WORDS {
            return Ok(None);
        }
        let answer = ask(&self.quote_prompt(index), QUOTE_TOKENS)?;
        if none(&answer) {
            return Ok(None);
        }
        let mut quote = quoted(&line.text, &answer);
        if quote.is_none() {
            // Asked once more, since the Chad call's review on Thursday
            // was lost to a copy that left words out unmarked.
            let again = ask(&self.again_prompt(index, &answer), QUOTE_TOKENS)?;
            if none(&again) {
                return Ok(None);
            }
            quote = quoted(&line.text, &again);
        }
        let Some(Quote {
            words,
            reported: false,
            start,
        }) = quote
        else {
            return Ok(None);
        };
        if self.told_before(index, start) || named(&words, &line.who) {
            return Ok(None);
        }
        let verdict = ask(&self.keep_prompt(index, &words), VERDICT_TOKENS)?;
        // Gemma sometimes answers in bold, **NO**.
        let verdict = verdict.trim().trim_matches('*').trim().to_uppercase();
        Ok(verdict.starts_with("YES").then(|| {
            format!(
                "- {}, to {}: \"{words}\" [{}]",
                line.who,
                self.listener(&line.who),
                line.stamp
            )
        }))
    }

    /// Whether a quote starting `start` words into line `index` follows
    /// its speaker telling what someone else said at the end of their
    /// line before: on the Chad call, "she's like but also", then "Yeah,
    /// you're autistic" in the next.
    fn told_before(&self, index: usize, start: usize) -> bool {
        if start >= REPORT_REACH {
            return false;
        }
        let who = &self.said[index].who;
        let Some(before) = self.said[..index]
            .iter()
            .rev()
            .find(|line| &line.who == who)
        else {
            return false;
        };
        let mut words = plain_words(&before.text);
        words.extend(plain_words(&self.said[index].text).into_iter().take(start));
        reported(&words[words.len().saturating_sub(REPORT_REACH)..])
    }

    fn shown(&self, index: usize) -> String {
        let line = &self.said[index];
        format!("L{index} [{}] {}: {}", line.stamp, line.who, line.text)
    }

    fn pick_prompt(&self, part: &[usize]) -> String {
        let numbered: Vec<String> = part.iter().map(|&index| self.shown(index)).collect();
        format!(
            "These are numbered lines from {}.\n\n{}\n\nWhich lines matter most for someone \
             who missed the call: news about someone's health, work or plans, what {them} \
             told {me}, what {me} told them, every request, offer or worry, and every line \
             where someone says they or another will do something? Skip greetings, filler and \
             small talk. Answer with at most {MOST_PICKS} line numbers like L12, the most \
             important first, separated by commas, and nothing else.",
            self.between(),
            numbered.join("\n"),
            me = self.me,
            them = self.others(),
        )
    }

    /// The lines around a line, for a prompt: the two before, the line
    /// marked, and the one after.
    fn around(&self, index: usize) -> String {
        let mut shown: Vec<String> = (index.saturating_sub(2)..index)
            .map(|at| self.shown(at))
            .collect();
        shown.push(format!(">>> {}", self.shown(index)));
        if index + 1 < self.said.len() {
            shown.push(self.shown(index + 1));
        }
        shown.join("\n")
    }

    fn quote_prompt(&self, index: usize) -> String {
        let speaker = &self.said[index].who;
        format!(
            "From {}, the lines before, the marked line, and the line after:\n\n{}\n\nCopy, \
             word for word from the marked line, the words that carry the point {speaker} \
             makes, at most {MOST_QUOTE_WORDS} words. You may leave out filler such as you \
             know, I mean, like or um, but change no word and add none. Answer with the copied \
             words only, or NONE if the line makes no clear point.",
            self.between(),
            self.around(index),
        )
    }

    /// The quote question again, after a copy that wasn't the line's.
    fn again_prompt(&self, index: usize, answer: &str) -> String {
        format!(
            "{}\n\nIt was answered \"{answer}\", which isn't the marked line's words as \
             written. Copy one unbroken run of the marked line's words, exactly as written, \
             leaving none out, at most {MOST_QUOTE_WORDS} words. Answer with the copied words \
             only, or NONE.",
            self.quote_prompt(index),
        )
    }

    /// Whether a quote is worth showing, asked with the whole line in
    /// view, so a quote that drops what turns its meaning is seen to.
    /// Asked for a point worth knowing and the speaker's own view, Gemma
    /// turned down plain news (a review on Thursday, an offer to help) on
    /// the Chad call, so it's asked what someone would learn, and what
    /// would make a quote unfit, rather than whether it's a view.
    fn keep_prompt(&self, index: usize, quote: &str) -> String {
        let line = &self.said[index];
        format!(
            "From {}, {} said: \"{}\"\n\nQuoted from it: \"{quote}\"\n\nWould someone who \
             missed the call learn something from the quoted words alone: news, a worry, a \
             plan, a request, an offer or a view? Answer NO if the quoted words are small \
             talk, are garbled, are about pay, money or a raise, or can't be understood \
             without the lines around them, such as a they or that with nothing it points to. \
             Otherwise answer YES.",
            self.between(),
            line.who,
            line.text,
        )
    }
}

/// The lines cut into parts of about `most` characters of what was said,
/// by index, every line in one part.
fn parts(said: &[Said], most: usize) -> Vec<Vec<usize>> {
    let mut parts = Vec::new();
    let mut part = Vec::new();
    let mut size = 0;
    for (index, line) in said.iter().enumerate() {
        if !part.is_empty() && size + line.text.len() > most {
            parts.push(std::mem::take(&mut part));
            size = 0;
        }
        part.push(index);
        size += line.text.len();
    }
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

/// How far into a call a stamp, 1:05:28 or 3:14, is, in seconds.
fn seconds(stamp: &str) -> u64 {
    stamp
        .split(':')
        .fold(0, |sum, part| sum * 60 + part.parse::<u64>().unwrap_or(0))
}

/// Whether a quote question was answered that the line makes no point.
fn none(answer: &str) -> bool {
    answer.is_empty() || answer.to_uppercase().starts_with("NONE")
}

/// Whether a quote calls its own speaker by name, as in "Joel, you keep
/// giving them fuel" or "it's like Joel" said by Joel: words someone
/// else said to them, as Joel told his wife's on the Chad call.
fn named(quote: &str, speaker: &str) -> bool {
    if speaker == OTHERS {
        return false;
    }
    let Some(name) = speaker.split_whitespace().next().map(plain) else {
        return false;
    };
    let spans = spans(quote);
    spans.iter().enumerate().any(|(at, &(a, b))| {
        plain(&quote[a..b]) == name
            && (quote[b..].starts_with(',')
                || at.checked_sub(1).is_some_and(|before| {
                    plain(&quote[spans[before].0..spans[before].1]) == "like"
                }))
    })
}

/// The line numbers an answer names, as L12, in the order it names them.
fn numbers(answer: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut rest = answer;
    while let Some(at) = rest.find('L') {
        rest = &rest[at + 1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(number) = digits.parse() {
            found.push(number);
        }
    }
    found
}

/// A word as it's compared: lowercase, with a typeset apostrophe as a
/// plain one.
fn plain(word: &str) -> String {
    word.to_lowercase().replace('\u{2019}', "'")
}

/// Where each of `text`'s words is, a word being letters, digits and
/// apostrophes.
fn spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = None;
    for (at, c) in text.char_indices() {
        let inside = c.is_alphanumeric() || c == '\'' || c == '\u{2019}';
        match (inside, start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                spans.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        spans.push((from, text.len()));
    }
    spans
}

/// A quote copied from a line, and whether the words just before it, or
/// within it, say the speaker is telling what someone else said.
#[derive(Debug, PartialEq)]
struct Quote {
    words: String,
    reported: bool,
    /// How many of the line's words come before it.
    start: usize,
}

/// A line's words as they're compared.
fn plain_words(text: &str) -> Vec<String> {
    spans(text)
        .into_iter()
        .map(|(a, b)| plain(&text[a..b]))
        .collect()
}

/// The words of `answer` as `text` spells them, or nothing when they
/// aren't `text`'s: each must come in the line's order, at most
/// `MOST_SKIPPED` of its words apart unless the answer marks a gap with
/// an ellipsis, and words left out between show as an ellipsis, so
/// nothing reads as said that wasn't.
fn quoted(text: &str, answer: &str) -> Option<Quote> {
    let answer = answer.trim().trim_matches(['"', '\u{201c}', '\u{201d}']);
    // Each word wanted, and whether the answer left words out before it.
    let mut want: Vec<(String, bool)> = Vec::new();
    for piece in answer
        .split("...")
        .flat_map(|piece| piece.split('\u{2026}'))
    {
        let gap = !want.is_empty();
        for (at, (a, b)) in spans(piece).into_iter().enumerate() {
            want.push((plain(&piece[a..b]), gap && at == 0));
        }
    }
    if want.len() < 3 {
        return None;
    }
    let spans = spans(text);
    let line = plain_words(text);
    // Of every place the words are found, the closest together, so a
    // word said twice (I was like, I do not) starts where it's meant.
    let found = (0..line.len())
        .filter(|&start| line[start] == want[0].0)
        .filter_map(|start| {
            let mut picked = vec![start];
            for (word, gap) in &want[1..] {
                let at = *picked.last()?;
                let reach = if *gap {
                    line.len()
                } else {
                    (at + 2 + MOST_SKIPPED).min(line.len())
                };
                let next = (at + 1..reach).find(|&j| line[j] == *word)?;
                picked.push(next);
            }
            Some(picked)
        })
        .min_by_key(|picked| picked[picked.len() - 1] - picked[0])?;
    let tail = if found.len() > MOST_QUOTE_WORDS_TAKEN {
        " ..."
    } else {
        ""
    };
    let found = &found[..found.len().min(MOST_QUOTE_WORDS_TAKEN)];
    // Someone else's words told inside the quote count too: on the Chad
    // call a quote ran on into "She's like your 50th percentile".
    let reported = reported(&line[found[0].saturating_sub(REPORT_REACH)..found[0]])
        || someone_like(&line[found[0]..=found[found.len() - 1]]);
    // Each unbroken run keeps the line's own text, punctuation and all.
    let mut runs = Vec::new();
    let mut first = found[0];
    for (at, &word) in found.iter().enumerate() {
        let next = found.get(at + 1);
        if next != Some(&(word + 1)) {
            runs.push(&text[spans[first].0..spans[word].1]);
            if let Some(&next) = next {
                first = next;
            }
        }
    }
    Some(Quote {
        words: runs.join(" ... ") + tail,
        reported,
        start: found[0],
    })
}

/// Whether the words before a quote say its speaker is telling what
/// someone else said (she said, my boss told me, he was like, she's
/// like) rather than saying it themselves (I was like, it goes).
fn reported(before: &[String]) -> bool {
    let someone = |who: &String| !OWN.contains(&who.as_str());
    let said = before
        .windows(2)
        .any(|pair| someone(&pair[0]) && REPORTING.contains(&pair[1].as_str()))
        || someone_like(before);
    let was_like = before.windows(3).any(|three| {
        someone(&three[0])
            && matches!(three[1].as_str(), "was" | "is" | "were")
            && three[2] == "like"
    });
    said || was_like
}

/// Whether `words` tell what someone else said with she's like, he's like
/// or they're like. The other reporting words are looked for only before a
/// quote, since inside one they're as often the speaker's own: you have
/// asked, my brain is like.
fn someone_like(words: &[String]) -> bool {
    words
        .windows(2)
        .any(|pair| SOMEONE_LIKE.contains(&pair[0].as_str()) && pair[1] == "like")
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
