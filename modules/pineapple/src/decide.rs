//! What follows from a saying a reader heard: whether it stops or resumes
//! anything and what, how loudly Joel is told, and what the person who
//! said it is answered. This is the plan's table of near misses as code,
//! so a reader only finds sayings and the enforcer only acts.

use std::fmt;

use crate::judge::{Joel, Platform, Saying, Verdict, Waved, judge};
use crate::scope::{Context, Target, reach};

/// A saying as a reader found it, with the place it was said in.
#[derive(Clone, Debug)]
pub struct Heard {
    /// How its platform marks a tag.
    pub platform: Platform,
    /// Who wrote it, as the platform names them.
    pub author: String,
    /// Whether a bot or an app wrote it.
    pub by_bot: bool,
    /// Whether it was written where any account may comment, by one that
    /// isn't the repository's own, which only Joel's own words outweigh.
    pub outsider: bool,
    /// What it says, as the platform gives it, tags and quotes and all.
    pub text: String,
    /// What the place it was said in is about.
    pub context: Context,
}

/// How Joel is told of a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tell {
    /// At once, with a sound, wherever he is.
    Loud,
    /// A row in For You, for when he looks.
    Row,
    /// Only in the log, where a miss can be found.
    Log,
}

/// What the person who said it is answered, in the thread or on the pull
/// request where they said it, if [`crate::gate`] lets it go. Only these
/// two are ever said in public (Joel, 13 November): someone asking for a
/// stop without the phrase, or saying it without his tag, is told to Joel
/// loudly, and he decides whether to answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// What stopped, how to narrow it, and that only Joel resumes.
    WhatStopped,
    /// A resume is Joel's, and he has been asked.
    JoelAsked,
    /// Nothing is said.
    Nothing,
}

/// What a saying comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// What it asks for.
    pub verdict: Verdict,
    /// Whether Joel said it.
    pub by_joel: bool,
    /// What it stops or resumes; empty when it does neither.
    pub reaches: Vec<Target>,
    /// Whether it reaches everything only because nothing narrower was
    /// found, which Joel is told loudly.
    pub fell_back: bool,
    /// What a stop would reach, for a saying Joel is asked to judge (an
    /// ask without the phrase, a joke, the phrase without his tag), so
    /// his one press to stop it knows what; empty for any other.
    pub would_reach: Vec<Target>,
    /// Whether that would be everything only because nothing narrower was
    /// found.
    pub would_fall_back: bool,
    /// How Joel is told.
    pub tell: Tell,
    /// What its writer is answered.
    pub reply: Reply,
}

impl Decision {
    /// Whether it stops what it reaches.
    #[must_use]
    pub fn stops(&self) -> bool {
        self.verdict == Verdict::Stop
    }
}

/// What follows from a saying, judged against who Joel is.
#[must_use]
pub fn decide(heard: &Heard, joel: &Joel) -> Decision {
    let judged = judge(
        &Saying {
            platform: heard.platform,
            author: &heard.author,
            by_bot: heard.by_bot,
            text: &heard.text,
        },
        joel,
    );
    // An outsider's words are only kept in the log: they stop nothing,
    // sound nothing and are answered nothing, so a stranger can neither
    // halt Joel's work nor fill his screen. A bot's stay a bot's.
    let verdict = match judged.verdict {
        Verdict::Nothing | Verdict::Waved(Waved::Bot) => judged.verdict,
        _ if heard.outsider && !judged.by_joel => Verdict::Waved(Waved::Outsider),
        verdict => verdict,
    };
    let (reaches, fell_back) = match verdict {
        Verdict::Stop | Verdict::Resume | Verdict::ResumeAsked => {
            let reached = reach(&judged.targets, &heard.context);
            (reached.targets, reached.fell_back)
        }
        _ => (Vec::new(), false),
    };
    let (would_reach, would_fall_back) = match verdict {
        Verdict::StopAsked | Verdict::Waved(Waved::Joke | Waved::NoTag) => {
            let would = reach(&judged.targets, &heard.context);
            (would.targets, would.fell_back)
        }
        _ => (Vec::new(), false),
    };
    let (tell, reply) = match verdict {
        // Joel's own stop needs no answer: he knows what he said.
        Verdict::Stop if judged.by_joel => (Tell::Loud, Reply::Nothing),
        Verdict::Stop => (Tell::Loud, Reply::WhatStopped),
        Verdict::ResumeAsked => (Tell::Loud, Reply::JoelAsked),
        // A waved-off joke is told loudly, so a miss is one press from a
        // stop; so are an ask without the phrase and the phrase without
        // his tag, which only he answers (Joel, 13 November).
        Verdict::StopAsked | Verdict::Waved(Waved::Joke | Waved::NoTag) => {
            (Tell::Loud, Reply::Nothing)
        }
        Verdict::Resume | Verdict::Waved(Waved::TalkedAbout) => (Tell::Row, Reply::Nothing),
        // Joel's account saying the phrase other than as its opening is
        // most often one of his agents writing about it, so it's only
        // kept in the log, like a quote.
        Verdict::Waved(Waved::Quoted | Waved::Bot | Waved::Outsider | Waved::Unopened)
        | Verdict::Nothing => (Tell::Log, Reply::Nothing),
    };
    Decision {
        verdict,
        by_joel: judged.by_joel,
        reaches,
        fell_back,
        would_reach,
        would_fall_back,
        tell,
        reply,
    }
}

impl fmt::Display for Tell {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Loud => "loud",
            Self::Row => "row",
            Self::Log => "log",
        })
    }
}

impl fmt::Display for Reply {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::WhatStopped => "what-stopped",
            Self::JoelAsked => "joel-asked",
            Self::Nothing => "nothing",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table's Joel, by his stand-in Slack id.
    fn joel() -> Joel {
        Joel {
            slack_ids: vec!["U0JOEL0001".to_owned()],
            logins: Vec::new(),
            emails: Vec::new(),
        }
    }

    /// What a person, or a bot, saying `text` in Slack comes to, in a
    /// thread about pull request 12.
    fn decided(author: &str, by_bot: bool, text: &str) -> Decision {
        heard_from(author, by_bot, false, text)
    }

    /// The same, written by `author` where any account may comment, as
    /// one that isn't the repository's own.
    fn outsider(author: &str, text: &str) -> Decision {
        heard_from(author, false, true, text)
    }

    fn heard_from(author: &str, by_bot: bool, outsider: bool, text: &str) -> Decision {
        let heard = Heard {
            platform: Platform::Slack,
            author: author.to_owned(),
            by_bot,
            outsider,
            text: text.to_owned(),
            context: Context::of_thread("review https://code.example.com/example/shop/pull/12"),
        };
        decide(&heard, &joel())
    }

    #[test]
    fn an_outsider_stops_nothing_and_is_only_logged_but_joel_still_stops() {
        for text in [
            "<@U0JOEL0001> pineapple override",
            "<@U0JOEL0001> pineapple resume",
            "<@U0JOEL0001> STOP your robots",
            "<@U0JOEL0001> pineapple override jk",
            "pineapple override",
            "<@U0JOEL0001> our safe word pineapple override is set",
        ] {
            let decision = outsider("U0OTHER001", text);
            assert_eq!(
                decision.verdict,
                Verdict::Waved(Waved::Outsider),
                "{text:?} by an outsider is waved off"
            );
            assert!(
                decision.reaches.is_empty() && decision.would_reach.is_empty(),
                "{text:?} by an outsider reaches nothing, not even for a press"
            );
            assert_eq!(decision.tell, Tell::Log, "{text:?} only goes in the log");
            assert_eq!(
                decision.reply,
                Reply::Nothing,
                "{text:?} is answered nothing"
            );
        }
        let lunch = outsider("U0OTHER001", "<@U0JOEL0001> lunch?");
        assert_eq!(lunch.verdict, Verdict::Nothing, "nothing stays nothing");
        let own = outsider("U0JOEL0001", "pineapple override");
        assert!(own.stops(), "Joel's own stop stands wherever he says it");
        let bot = heard_from("B0BOT00001", true, true, "<@U0JOEL0001> pineapple override");
        assert_eq!(
            bot.verdict,
            Verdict::Waved(Waved::Bot),
            "an outsider's bot is still told as a bot's"
        );
    }

    #[test]
    fn each_verdict_is_told_and_answered_as_the_plan_says() {
        let cases = [
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> pineapple override",
                Tell::Loud,
                Reply::WhatStopped,
            ),
            (
                "U0JOEL0001",
                false,
                "pineapple override",
                Tell::Loud,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> STOP your robots",
                Tell::Loud,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> pineapple resume",
                Tell::Loud,
                Reply::JoelAsked,
            ),
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> pineapple override jk",
                Tell::Loud,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "pineapple override",
                Tell::Loud,
                Reply::Nothing,
            ),
            (
                "U0JOEL0001",
                false,
                "pineapple resume",
                Tell::Row,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> our safe word pineapple override is set",
                Tell::Row,
                Reply::Nothing,
            ),
            (
                "B0BOT00001",
                true,
                "<@U0JOEL0001> pineapple override",
                Tell::Log,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "> <@U0JOEL0001> pineapple override",
                Tell::Log,
                Reply::Nothing,
            ),
            (
                "U0OTHER001",
                false,
                "<@U0JOEL0001> lunch?",
                Tell::Log,
                Reply::Nothing,
            ),
        ];
        for (author, by_bot, text, tell, reply) in cases {
            let decision = decided(author, by_bot, text);
            assert_eq!(decision.tell, tell, "{text:?} by {author} is told {tell}");
            assert_eq!(
                decision.reply, reply,
                "{text:?} by {author} is answered {reply}"
            );
        }
    }

    #[test]
    fn only_a_stop_stops_and_it_reaches_the_threads_pull_request() {
        let stop = decided("U0OTHER001", false, "<@U0JOEL0001> pineapple override");
        assert!(stop.stops(), "a tagged override stops");
        assert_eq!(
            stop.reaches,
            [Target::Pr {
                repo: Some("example/shop".to_owned()),
                number: 12
            }],
            "it reaches the pull request its thread is about"
        );
        assert!(
            !stop.fell_back,
            "a pull request is narrower than everything"
        );
        let asked = decided("U0OTHER001", false, "<@U0JOEL0001> STOP your robots");
        assert!(
            !asked.stops(),
            "asking to stop without the phrase stops nothing"
        );
        assert!(asked.reaches.is_empty(), "and reaches nothing");
        assert_eq!(
            asked.would_reach, stop.reaches,
            "but would reach what a stop said there would, for Joel's press"
        );
        let joke = decided("U0OTHER001", false, "<@U0JOEL0001> pineapple override jk");
        assert_eq!(joke.would_reach, stop.reaches, "so would a joke waved off");
        assert!(
            stop.would_reach.is_empty(),
            "a stop itself has its reaches, not a would-reach"
        );
    }
}
