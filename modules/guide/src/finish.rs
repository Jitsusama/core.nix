//! What's left once the person is signed in and online (`guide finish`):
//! the account's keys added to GitHub, then each step the machine's own
//! repo adds, such as an employer's enrolment. Those are programs with
//! screens of their own, often driving a browser, so the guide says what
//! to type in another terminal and looks again until the program the
//! machine gives it says the step is done, rather than running them
//! itself.

use serde::Deserialize;

use crate::github::{self, Account};
use crate::{Answers, Fault, Machine, Place, Program, Standing, Step};

/// A step the machine's repo adds, as core's guide module writes it
/// (../nixos.nix's jitsusama.guide.onceSignedIn).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Added {
    /// Stable for good, as every step's id is.
    pub id: String,
    pub title: String,
    /// What the person types for it, in another terminal.
    pub command: String,
    /// What it does, and what to look out for while it runs.
    pub about: String,
    /// A program in the Nix store, run as the account, that exits 0 once
    /// the step is done, printing what shows it, and otherwise prints
    /// what's still to do.
    pub done: String,
}

/// A step the machine's repo adds, which the person runs themselves.
pub struct Theirs {
    id: &'static str,
    title: &'static str,
    added: Added,
    account: Account,
}

impl Theirs {
    pub fn new(added: Added, account: &Account) -> Theirs {
        // Steps are built once a run, so their names live as long as it.
        Theirs {
            id: Box::leak(added.id.clone().into_boxed_str()),
            title: Box::leak(added.title.clone().into_boxed_str()),
            added,
            account: account.clone(),
        }
    }
}

/// The first line a program printed, if it printed one.
fn first_line(printed: &[u8]) -> Option<String> {
    String::from_utf8_lossy(printed)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(String::from)
}

impl Step for Theirs {
    fn id(&self) -> &'static str {
        self.id
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        self.title
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let ran = machine.run_given(
            Program::Carried(&self.added.done),
            &[],
            self.account.given(),
        )?;
        let said = first_line(&ran.stdout);
        let command = &self.added.command;
        Ok(if ran.status == 0 {
            Standing::Done {
                evidence: said.unwrap_or_else(|| format!("{command} is done")),
            }
        } else {
            Standing::Blocked {
                why: said.unwrap_or_else(|| format!("{command} isn't finished yet")),
                remedy: format!(
                    "In another terminal, type {command}. {} This screen looks again by itself, and goes on once it's done.",
                    self.added.about
                ),
            }
        })
    }

    /// Nothing for the guide to do: the person runs it, and the next look
    /// sees it done.
    fn run(&self, _machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        Ok(())
    }
}

/// Every step of `guide finish`: GitHub's, then the machine's own, in the
/// order its repo gives them.
pub fn steps(account: &Account, added: Vec<Added>) -> Vec<Box<dyn Step>> {
    let mut steps = github::steps(account);
    steps.extend(
        added
            .into_iter()
            .map(|added| Box::new(Theirs::new(added, account)) as Box<dyn Step>),
    );
    steps
}
