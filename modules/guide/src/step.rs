//! A step: one thing the journey does, which observes the machine to say
//! whether it's done, names everything it needs from the person before it
//! starts, and acts.

use std::collections::BTreeMap;
use std::fmt;

use zeroize::Zeroizing;

use crate::{Fault, Machine, Place};

pub trait Step {
    /// Stable for good, since the journal and the logs name a step by it.
    fn id(&self) -> &'static str;

    /// Where it runs. The journey takes steps in order, place by place.
    fn place(&self) -> Place;

    /// What it does, said before it starts: one line, a person's words.
    fn title(&self) -> &'static str;

    /// What the machine says about this step now. Never changes anything,
    /// so it can be asked again whenever the guide starts.
    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault>;

    /// Does the step's work with what the person gave. It's only ever done
    /// once observing again says so, whatever this returns.
    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault>;
}

/// Where a step stands, as the machine shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// Done, with what shows it, said back to the person.
    Done { evidence: String },
    /// Still to do, with everything it'll ask for, all said before it starts.
    Needed { asks: Vec<Ask> },
    /// It can't run here and now: why, and what the person can do about it.
    Blocked { why: String, remedy: String },
}

impl fmt::Display for Standing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Standing::Done { evidence } => write!(f, "done: {evidence}"),
            Standing::Needed { .. } => f.write_str("still to do"),
            Standing::Blocked { why, .. } => write!(f, "blocked: {why}"),
        }
    }
}

/// Something a step needs from the person, each with an id the step reads
/// its answer by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A secret they type, named the way they know it: "the Nano's PIN".
    Secret { id: &'static str, name: String },
    /// Something they do away from the keyboard, said plainly.
    Act { id: &'static str, what: String },
    /// A word they type back to show they mean it, such as a disk's name.
    TypeBack {
        id: &'static str,
        expected: String,
        why: String,
    },
}

impl Ask {
    pub fn id(&self) -> &'static str {
        match self {
            Ask::Secret { id, .. } | Ask::Act { id, .. } | Ask::TypeBack { id, .. } => id,
        }
    }
}

/// What the person gave, by each ask's id.
#[derive(Debug, Default)]
pub struct Answers {
    given: BTreeMap<&'static str, Answer>,
}

impl Answers {
    pub fn new() -> Answers {
        Answers::default()
    }

    pub fn give(&mut self, id: &'static str, answer: Answer) {
        self.given.insert(id, answer);
    }

    pub fn get(&self, id: &str) -> Option<&Answer> {
        self.given.get(id)
    }

    pub fn has(&self, id: &str) -> bool {
        self.given.contains_key(id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Secret(Secret),
    /// Done, for an `Ask::Act`.
    Done,
    /// What they typed, for an `Ask::TypeBack`.
    Typed(String),
}

/// A secret the person typed: wiped from memory when it's dropped, and
/// never shown by Debug, so no log or panic can carry it.
#[derive(PartialEq, Eq)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(value: String) -> Secret {
        Secret(Zeroizing::new(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_never_shows_in_debug() {
        let mut answers = Answers::new();
        answers.give("pin", Answer::Secret(Secret::new("123456".into())));
        let shown = format!("{answers:?}");
        assert!(!shown.contains("123456"), "{shown}");
        assert!(shown.contains("Secret(..)"), "{shown}");
    }
}
