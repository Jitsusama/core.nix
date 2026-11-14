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
/// request where they said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// What stopped, how to narrow it, and that only Joel resumes.
    WhatStopped,
    /// How to stop his agents: tag him with the phrase.
    HowToStop,
    /// The phrase needs his tag to stop anything.
    TagJoel,
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
    let (reaches, fell_back) = match judged.verdict {
        Verdict::Stop | Verdict::Resume | Verdict::ResumeAsked => {
            let reached = reach(&judged.targets, &heard.context);
            (reached.targets, reached.fell_back)
        }
        _ => (Vec::new(), false),
    };
    let (tell, reply) = match judged.verdict {
        // Joel's own stop needs no answer: he knows what he said.
        Verdict::Stop if judged.by_joel => (Tell::Loud, Reply::Nothing),
        Verdict::Stop => (Tell::Loud, Reply::WhatStopped),
        Verdict::StopAsked => (Tell::Loud, Reply::HowToStop),
        Verdict::ResumeAsked => (Tell::Loud, Reply::JoelAsked),
        // A waved-off joke is told loudly, so a miss is one press from a
        // stop (Joel, 13 November).
        Verdict::Waved(Waved::Joke) => (Tell::Loud, Reply::Nothing),
        Verdict::Waved(Waved::NoTag) => (Tell::Row, Reply::TagJoel),
        Verdict::Resume | Verdict::Waved(Waved::TalkedAbout) => (Tell::Row, Reply::Nothing),
        Verdict::Waved(Waved::Quoted | Waved::Bot) | Verdict::Nothing => {
            (Tell::Log, Reply::Nothing)
        }
    };
    Decision {
        verdict: judged.verdict,
        by_joel: judged.by_joel,
        reaches,
        fell_back,
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
            Self::HowToStop => "how-to-stop",
            Self::TagJoel => "tag-joel",
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
        let heard = Heard {
            platform: Platform::Slack,
            author: author.to_owned(),
            by_bot,
            text: text.to_owned(),
            context: Context::of_thread("review https://code.example.com/example/shop/pull/12"),
        };
        decide(&heard, &joel())
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
                Reply::HowToStop,
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
                Tell::Row,
                Reply::TagJoel,
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
    }
}
