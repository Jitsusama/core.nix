//! warp-cli as the contract has it answer (contract/), for the tests: each
//! question by its words, as an answer or as an error, and a question it
//! isn't given refused, as warp-cli refuses one it doesn't know.

use std::collections::HashMap;

use crate::ask::{Ask, Said, Unasked};

const CONTRACT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/contract");

pub fn file(name: &str) -> String {
    std::fs::read_to_string(format!("{CONTRACT}/{name}")).expect("the contract has it")
}

/// An answer given as it was, or as an error, from the contract's file.
#[expect(
    clippy::unnecessary_wraps,
    reason = "it's an answer as `Contract::with` takes one, beside those warp-cli couldn't give"
)]
pub fn said(ok: bool, name: &str) -> Result<Said, Unasked> {
    Ok(Said {
        ok,
        text: file(name),
    })
}

pub struct Contract {
    answers: HashMap<String, Result<Said, Unasked>>,
}

impl Contract {
    /// warp-cli answering nothing it's asked.
    pub fn none() -> Self {
        Self {
            answers: HashMap::new(),
        }
    }

    /// warp-cli answering each question as the contract has it.
    pub fn whole() -> Self {
        Self::none()
            .with("status", said(true, "status.json"))
            .with("tunnel stats", said(true, "tunnel-stats.json"))
            .with("settings", said(true, "settings.json"))
            .with("dns stats", said(true, "dns-stats.json"))
    }

    /// The same, but answering this question so.
    pub fn with(mut self, question: &str, said: Result<Said, Unasked>) -> Self {
        self.answers.insert(question.to_string(), said);
        self
    }
}

impl Ask for Contract {
    fn ask(&self, question: &[&str]) -> Result<Said, Unasked> {
        match self.answers.get(&question.join(" ")) {
            Some(Ok(said)) => Ok(Said {
                ok: said.ok,
                text: said.text.clone(),
            }),
            Some(Err(Unasked::Missing(at))) => Err(Unasked::Missing(at.clone())),
            Some(Err(Unasked::Stuck(why))) => Err(Unasked::Stuck(why.clone())),
            None => said(false, "refused.json"),
        }
    }
}
