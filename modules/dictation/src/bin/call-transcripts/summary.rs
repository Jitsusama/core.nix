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
//! What a speaker reports someone else saying is dropped by code before
//! that, from the words just before the quote (she said, he was like),
//! since the model passed such quotes as the speaker's own on the Chad
//! call.

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
    let mut picked = Vec::new();
    for part in parts(&said, PART_CHARS) {
        let answer = ask(&call.pick_prompt(&part), PICK_TOKENS)?;
        let chosen = numbers(&answer)
            .into_iter()
            .filter(|index| part.contains(index))
            .take(MOST_TAKEN);
        for index in chosen {
            if !picked.contains(&index) {
                picked.push(index);
            }
        }
    }
    picked.sort_unstable();
    let mut kept = Vec::new();
    for index in picked {
        let line = &said[index];
        if line.text.split_whitespace().count() < FEWEST_WORDS {
            continue;
        }
        let answer = ask(&call.quote_prompt(index), QUOTE_TOKENS)?;
        if answer.is_empty() || answer.to_uppercase().starts_with("NONE") {
            continue;
        }
        let Some(Quote {
            words: quote,
            reported: false,
        }) = quoted(&line.text, &answer)
        else {
            continue;
        };
        let verdict = ask(&call.keep_prompt(index, &quote), VERDICT_TOKENS)?;
        // Gemma sometimes answers in bold, **NO**.
        let verdict = verdict.trim().trim_matches('*').trim().to_uppercase();
        if verdict.starts_with("YES") {
            kept.push(format!(
                "- {}, to {}: \"{quote}\" [{}]",
                line.who,
                call.listener(&line.who),
                line.stamp
            ));
        }
    }
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
             small talk. Answer with at most {MOST_PICKS} line numbers like L12, separated by \
             commas, and nothing else.",
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
    let line: Vec<String> = spans.iter().map(|&(a, b)| plain(&text[a..b])).collect();
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
mod tests {
    use super::*;

    const CALL: &str = "# Transcript\n\n**Joel**, 0:13: Hi there.\n\n\
         **Others**, 0:19: I think we should move the review to Thursday, you know, because \
         the release slipped.\n\n\
         **Joel**, 1:02: Okay, I will tell the team that the review moves to Thursday \
         morning.\n\n**Others**, 1:05:10: Thanks, bye.\n";

    /// Answers as a model would, by what each prompt asks, noting each.
    struct Model {
        picks: &'static str,
        quotes: Vec<&'static str>,
        verdict: &'static str,
        asked: Vec<String>,
    }

    impl Model {
        fn ask(&mut self, prompt: &str) -> String {
            self.asked.push(prompt.to_owned());
            if prompt.starts_with("These are numbered lines") {
                self.picks.to_owned()
            } else if prompt.contains("Copy, word for word") {
                self.quotes.remove(0).to_owned()
            } else {
                self.verdict.to_owned()
            }
        }
    }

    fn summed(model: &mut Model) -> String {
        summarize(&mut |p, _| Ok(model.ask(p)), "Review", None, CALL).unwrap()
    }

    fn words(text: &str, answer: &str) -> Option<String> {
        quoted(text, answer).map(|quote| quote.words)
    }

    #[test]
    fn a_transcript_reads_as_its_speakers_times_and_words() {
        let said = lines(CALL);
        assert_eq!(said.len(), 4);
        assert_eq!(
            said[3],
            Said {
                who: "Others".to_owned(),
                stamp: "1:05:10".to_owned(),
                text: "Thanks, bye.".to_owned(),
            }
        );
    }

    #[test]
    fn a_quote_is_the_lines_own_words_with_who_said_it_to_whom_and_when() {
        let mut model = Model {
            picks: "L1, L2",
            quotes: vec!["we should move the review to Thursday", "tell the team"],
            verdict: "YES",
            asked: Vec::new(),
        };
        let summary = summed(&mut model);
        assert_eq!(
            summary,
            format!(
                "# Review\n\n{HOW_MADE}\n\n\
                 - Others, to Joel: \"we should move the review to Thursday\" [0:19]\n\
                 - Joel, to the others: \"tell the team\" [1:02]"
            )
        );
    }

    #[test]
    fn the_others_are_named_where_the_call_knows_them() {
        let mut model = Model {
            picks: "L1, L2",
            quotes: vec!["we should move the review to Thursday", "tell the team"],
            verdict: "**YES**",
            asked: Vec::new(),
        };
        let summary = summarize(&mut |p, _| Ok(model.ask(p)), "Review", Some("Ada"), CALL).unwrap();
        assert!(summary.contains("- Ada, to Joel: \"we should move the review to Thursday\""));
        assert!(summary.contains("- Joel, to Ada: \"tell the team\""));
        assert!(model.asked[0].contains("a call between Joel and Ada"));
    }

    #[test]
    fn words_the_line_never_said_are_never_kept() {
        let mut model = Model {
            picks: "L1",
            quotes: vec!["we must move the review to Friday"],
            verdict: "YES",
            asked: Vec::new(),
        };
        assert!(summed(&mut model).ends_with(NONE_KEPT));
        // The worth of a quote that isn't found is never asked.
        assert_eq!(model.asked.len(), 2);
    }

    #[test]
    fn a_quote_judged_not_worth_keeping_is_dropped() {
        let mut model = Model {
            picks: "L1",
            quotes: vec!["we should move the review to Thursday"],
            verdict: "NO",
            asked: Vec::new(),
        };
        assert!(summed(&mut model).ends_with(NONE_KEPT));
    }

    #[test]
    fn a_line_named_outside_the_call_or_too_short_isnt_asked_about() {
        let mut model = Model {
            picks: "L0, L3, L9",
            quotes: Vec::new(),
            verdict: "YES",
            asked: Vec::new(),
        };
        assert!(summed(&mut model).ends_with(NONE_KEPT));
        assert_eq!(model.asked.len(), 1);
    }

    #[test]
    fn the_keep_question_sees_the_whole_line() {
        let mut model = Model {
            picks: "L1",
            quotes: vec!["move the review to Thursday"],
            verdict: "YES",
            asked: Vec::new(),
        };
        summed(&mut model);
        assert!(model.asked[2].contains("because the release slipped"));
    }

    #[test]
    fn a_failed_answer_writes_no_summary() {
        let result = summarize(&mut |_, _| Err("stopped".to_owned()), "Review", None, CALL);
        assert_eq!(result, Err("stopped".to_owned()));
    }

    #[test]
    fn filler_left_out_shows_as_an_ellipsis() {
        let text = "I think we should, you know, move it to Thursday.";
        assert_eq!(
            words(text, "\"we should move it to Thursday\"").as_deref(),
            Some("we should ... move it to Thursday")
        );
    }

    #[test]
    fn words_the_copy_marks_as_left_out_may_be_many() {
        let text = "Add me on LinkedIn, in case you get shut down right after the call, and \
             let me know.";
        assert_eq!(
            words(
                text,
                "Add me on LinkedIn, in case you get shut down... let me know"
            )
            .as_deref(),
            Some("Add me on LinkedIn, in case you get shut down ... let me know")
        );
        assert_eq!(words(text, "Add me on LinkedIn and let me know"), None);
    }

    #[test]
    fn what_someone_else_said_is_known_from_the_words_before_it() {
        let reports = [
            "My psychologist was like dude, she's like if they want your brain, back off",
            "and she said. Like it was nitpicking at this point, right?",
            "so my boss told me it was nitpicking at this point, right?",
            "and he was like it was nitpicking at this point, right?",
        ];
        for text in reports {
            let answer = if text.contains("brain") {
                "if they want your brain, back off"
            } else {
                "it was nitpicking at this point"
            };
            assert!(quoted(text, answer).unwrap().reported, "{text}");
        }
        let within = "but your processing speed is average. She's like your 50th percentile";
        let answer = "your processing speed is average. She's like your 50th percentile";
        assert!(quoted(within, answer).unwrap().reported);
        let own = [
            "and I was like I do not see this as a correction at all",
            "I went and said I do not see this as a correction at all",
            "it goes I do not see this as a correction at all",
            "I do not see this as a correction at all",
        ];
        for text in own {
            let quote = quoted(text, "I do not see this as a correction").unwrap();
            assert!(!quote.reported, "{text}");
            assert_eq!(quote.words, "I do not see this as a correction");
        }
    }

    #[test]
    fn a_quote_of_what_someone_else_said_is_never_asked_about() {
        let call = "**Joel**, 0:13: Hi there.\n\n**Others**, 0:19: and she said we should \
             move the review to Thursday.\n";
        let mut model = Model {
            picks: "L1",
            quotes: vec!["we should move the review to Thursday"],
            verdict: "YES",
            asked: Vec::new(),
        };
        let summary = summarize(&mut |p, _| Ok(model.ask(p)), "", None, call).unwrap();
        assert!(summary.ends_with(NONE_KEPT));
        assert_eq!(model.asked.len(), 2);
    }

    #[test]
    fn a_quote_keeps_the_lines_spelling_and_punctuation() {
        let text = "Well, Q2 is done; it's shipped.";
        assert_eq!(
            words(text, "q2 is done it’s shipped").as_deref(),
            Some("Q2 is done; it's shipped")
        );
    }

    #[test]
    fn words_too_far_apart_or_out_of_order_arent_a_quote() {
        let text = "move it, and then after all of that, to Thursday";
        assert_eq!(words(text, "move it to Thursday"), None);
        assert_eq!(words("Thursday we move it", "we move Thursday"), None);
    }

    #[test]
    fn a_copy_of_too_few_words_isnt_a_quote() {
        assert_eq!(words("move it to Thursday", "Thursday"), None);
    }

    #[test]
    fn a_long_copy_is_cut_short_and_marked() {
        let text: Vec<String> = (0..40).map(|n| format!("w{n}")).collect();
        let text = text.join(" ");
        let quote = words(&text, &text).unwrap();
        assert!(quote.ends_with("w29 ..."));
    }

    #[test]
    fn a_long_call_is_picked_from_in_parts_and_loses_no_line() {
        let said: Vec<Said> = (0..10)
            .map(|n| Said {
                who: "Joel".to_owned(),
                stamp: format!("0:{n:02}"),
                text: "x".repeat(30),
            })
            .collect();
        let cut = parts(&said, 100);
        assert_eq!(cut, [vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8], vec![9]]);
    }

    #[test]
    fn a_pick_reads_each_number_it_names() {
        assert_eq!(numbers("L3, L12,L40 and L"), [3, 12, 40]);
    }
}
