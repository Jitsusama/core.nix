//! Whether a saying is the safe word: who Joel is on each platform, what
//! counts as tagging him, and the near misses that don't stop anything
//! but are still worth telling him about.

use std::fmt;

use crate::scope::{Target, targets};
use crate::words::{find_run, has_run, own_words, tokens};

/// The safe word that stops Joel's agents.
const OVERRIDE: [&str; 2] = ["pineapple", "override"];
/// The word only Joel may use to start them again.
const RESUME: [&str; 2] = ["pineapple", "resume"];

/// How a platform marks a tag, which decides what counts as tagging Joel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Slack, which writes a real mention as `<@ID>` whatever name it
    /// shows, so a display name typed as plain text is no tag.
    Slack,
    /// A markdown host, GitHub or Delta, where a tag is `@login`.
    Markdown,
}

/// Who Joel is wherever a saying can be read: his Slack user IDs, his
/// logins on the markdown hosts, and the addresses a host may name an
/// author by. All are compared without regard to case.
#[derive(Clone, Debug, Default)]
pub struct Joel {
    /// His Slack user IDs, as a mention writes them.
    pub slack_ids: Vec<String>,
    /// His logins on GitHub and Delta, without the `@`.
    pub logins: Vec<String>,
    /// Addresses a host names him by as an author, as Delta does.
    pub emails: Vec<String>,
}

impl Joel {
    /// Whether `author`, as a platform names a message's writer, is Joel.
    #[must_use]
    pub fn wrote(&self, author: &str) -> bool {
        self.slack_ids
            .iter()
            .chain(&self.logins)
            .chain(&self.emails)
            .any(|him| him.eq_ignore_ascii_case(author))
    }

    /// Whether `words` tag Joel as `platform` marks a tag.
    #[must_use]
    pub fn is_tagged(&self, platform: Platform, words: &str) -> bool {
        match platform {
            Platform::Slack => self.slack_ids.iter().any(|id| {
                words.contains(&format!("<@{id}>")) || words.contains(&format!("<@{id}|"))
            }),
            Platform::Markdown => self.logins.iter().any(|login| tags_login(words, login)),
        }
    }
}

/// Whether `words` hold `@login` as a tag: not inside an address or a
/// longer name, and not the owner half of a path such as `@login/repo`.
fn tags_login(words: &str, login: &str) -> bool {
    let is_name = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    words.match_indices('@').any(|(at, _)| {
        let before = words[..at].chars().next_back();
        if before.is_some_and(|c| is_name(c) || c == '.' || c == '/') {
            return false;
        }
        let start = at + 1;
        let Some(name) = words.get(start..start + login.len()) else {
            return false;
        };
        if !name.eq_ignore_ascii_case(login) {
            return false;
        }
        let mut after = words[start + login.len()..].chars();
        match after.next() {
            None => true,
            Some('/') => false,
            // A full stop ends the tag unless a name runs on past it.
            Some('.') => !after.next().is_some_and(is_name),
            Some(c) => !is_name(c),
        }
    })
}

/// A message as a reader hands it over.
#[derive(Clone, Copy, Debug)]
pub struct Saying<'a> {
    /// Where it was said.
    pub platform: Platform,
    /// Its writer, as the platform names them.
    pub author: &'a str,
    /// Whether its writer is a bot or an app rather than a person.
    pub by_bot: bool,
    /// Its own text, without attachments or forwards.
    pub text: &'a str,
}

/// What a saying asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Stop what it names.
    Stop,
    /// Joel starting what it names again.
    Resume,
    /// Someone other than Joel asking for a resume, which is his to grant.
    ResumeAsked,
    /// Joel tagged and asked to stop his agents, without the phrase.
    StopAsked,
    /// The phrase was there but doesn't count, for this reason.
    Waved(Waved),
    /// Nothing to do with the safe word.
    Nothing,
}

/// Why a saying holding the phrase stops nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waved {
    /// Said as a joke or a what-if, which Joel hears about at once.
    Joke,
    /// Joel wasn't tagged.
    NoTag,
    /// Talked about rather than said: "the words pineapple override".
    TalkedAbout,
    /// Only inside a quote, code or quotation marks.
    Quoted,
    /// Written by a bot or an app.
    Bot,
}

impl fmt::Display for Verdict {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stop => out.write_str("stop"),
            Self::Resume => out.write_str("resume"),
            Self::ResumeAsked => out.write_str("resume-asked"),
            Self::StopAsked => out.write_str("stop-asked"),
            Self::Waved(why) => write!(out, "waved:{why}"),
            Self::Nothing => out.write_str("nothing"),
        }
    }
}

impl fmt::Display for Waved {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Joke => "joke",
            Self::NoTag => "no-tag",
            Self::TalkedAbout => "talked-about",
            Self::Quoted => "quoted",
            Self::Bot => "bot",
        })
    }
}

/// A saying judged: what it asks, whether Joel said it, and what its own
/// words name, before the context it was said in is taken into account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judgement {
    /// What it asks for.
    pub verdict: Verdict,
    /// Whether Joel wrote it.
    pub by_joel: bool,
    /// What its words name, for a stop or a resume; empty otherwise.
    pub targets: Vec<Target>,
}

/// Judge a saying against who Joel is.
#[must_use]
pub fn judge(saying: &Saying<'_>, joel: &Joel) -> Judgement {
    let own = own_words(saying.text);
    let words = tokens(&own);
    let by_joel = joel.wrote(saying.author);
    let tagged = joel.is_tagged(saying.platform, &own);
    let found = |verdict, targets| Judgement {
        verdict,
        by_joel,
        targets,
    };
    let anywhere = tokens(saying.text);
    let phrase_anywhere = has_run(&anywhere, &OVERRIDE) || has_run(&anywhere, &RESUME);

    if saying.by_bot {
        let verdict = if phrase_anywhere {
            Verdict::Waved(Waved::Bot)
        } else {
            Verdict::Nothing
        };
        return found(verdict, Vec::new());
    }
    if let Some(at) = find_run(&words, &OVERRIDE) {
        let verdict = if talked_about(&words, at) {
            Verdict::Waved(Waved::TalkedAbout)
        } else if !tagged && !by_joel {
            Verdict::Waved(Waved::NoTag)
        } else if is_joke(&own, &words) {
            Verdict::Waved(Waved::Joke)
        } else {
            return found(Verdict::Stop, targets(&own, &words, at + OVERRIDE.len()));
        };
        return found(verdict, Vec::new());
    }
    if let Some(at) = find_run(&words, &RESUME) {
        let named = targets(&own, &words, at + RESUME.len());
        return if talked_about(&words, at) {
            found(Verdict::Waved(Waved::TalkedAbout), Vec::new())
        } else if by_joel {
            found(Verdict::Resume, named)
        } else if tagged {
            found(Verdict::ResumeAsked, named)
        } else {
            found(Verdict::Waved(Waved::NoTag), Vec::new())
        };
    }
    if phrase_anywhere {
        return found(Verdict::Waved(Waved::Quoted), Vec::new());
    }
    if tagged && asks_to_stop(&words) {
        return found(Verdict::StopAsked, Vec::new());
    }
    found(Verdict::Nothing, Vec::new())
}

/// Whether the phrase at `at` is being talked about rather than said, as
/// in "tag him with the words pineapple override": a word naming it as
/// words stands just before it.
fn talked_about(words: &[String], at: usize) -> bool {
    const CUES: [&str; 13] = [
        "words", "word", "say", "says", "said", "saying", "type", "typing", "phrase", "called",
        "tag", "with", "means",
    ];
    words[at.saturating_sub(2)..at]
        .iter()
        .any(|word| CUES.contains(&word.as_str()))
}

/// Whether a saying is plainly a joke or a what-if, and nothing in it is
/// urgent. A laugh alone doesn't make one: Joel asked the judge to lean
/// toward stopping when unsure, so only a plain marker or a hypothetical
/// waves a saying off, and every one waved off is told to him at once.
fn is_joke(own: &str, words: &[String]) -> bool {
    const MARKERS: [&[&str]; 6] = [
        &["jk"],
        &["kidding"],
        &["joking"],
        &["not", "really"],
        &["just", "a", "joke"],
        &["imagine"],
    ];
    const WHAT_IFS: [&[&str]; 5] = [
        &["what", "if"],
        &["what", "happens"],
        &["should", "i"],
        &["wonder", "if"],
        &["would", "it"],
    ];
    let sarcasm = own.split_whitespace().any(|piece| piece == "/s");
    let joking = sarcasm
        || MARKERS.iter().any(|run| has_run(words, run))
        || WHAT_IFS.iter().any(|run| has_run(words, run));
    joking && !is_urgent(own, words)
}

/// Whether a saying sounds urgent, which beats any sign of a joke.
fn is_urgent(own: &str, words: &[String]) -> bool {
    const URGENT: [&str; 11] = [
        "now",
        "stop",
        "please",
        "asap",
        "urgent",
        "rogue",
        "immediately",
        "seriously",
        "help",
        "serious",
        "real",
    ];
    let shouted = own.split(|c: char| !c.is_alphanumeric()).any(|word| {
        word.len() >= 3
            && word.chars().all(|c| c.is_ascii_uppercase())
            && !word.eq_ignore_ascii_case("pineapple")
            && !word.eq_ignore_ascii_case("override")
    });
    own.contains('!') || shouted || words.iter().any(|word| URGENT.contains(&word.as_str()))
}

/// Whether a saying asks Joel to stop his agents in plain words: a word
/// for stopping beside a word for what his agents are.
fn asks_to_stop(words: &[String]) -> bool {
    const STOPPING: [&str; 9] = [
        "stop", "halt", "kill", "pause", "cease", "shut", "disable", "freeze", "rogue",
    ];
    const AGENTS: [&str; 11] = [
        "robot",
        "robots",
        "bot",
        "bots",
        "agent",
        "agents",
        "ai",
        "automation",
        "skynet",
        "swarm",
        "rogue",
    ];
    let has = |list: &[&str]| words.iter().any(|word| list.contains(&word.as_str()));
    has(&STOPPING) && has(&AGENTS)
}
