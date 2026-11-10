//! Everything Agents' rows are made from, as each source last told it:
//! the node's sessions and the machine's queue, the to-do daemon's
//! to-dos, spend-watch's collection, the harness's list of orchestrators,
//! and whether the shell says the screen is shared. Each source changes
//! its own part, and `made` makes every record from all of them.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::row::{self, Known, Words};
use crate::served;
use crate::session::{self, JoelCan, Session};
use crate::summary;
use crate::watch::Held;

/// What each source last said.
#[derive(Default)]
pub struct Inputs {
    pub words: Words,
    /// The node's `session` records.
    pub sessions: Held,
    /// The node's `job` records, each job of the machine's queue.
    pub jobs: Held,
    /// The to-do daemon's `to-do` records.
    pub to_dos: Held,
    /// Spend-watch's collection, or none before it's written.
    pub spend: Option<Value>,
    /// The harness's orchestrators file, or none.
    pub orchestrators: Option<Value>,
    /// Whether the screen is shared, so no amount is said.
    pub sharing: bool,
}

impl Inputs {
    /// What a session spent today, by pi's id for it.
    fn spent(&self, id: &str) -> Option<String> {
        let said = self.spend.as_ref()?.get("sessions")?.get(id)?.as_str()?;
        Some(said.to_owned())
    }

    /// What a quest and those under it spent today, in all.
    fn family(&self, quest: &str) -> Option<String> {
        let all = self
            .spend
            .as_ref()?
            .get("families")?
            .get(quest)?
            .get("all")?
            .as_str()?;
        Some(all.to_owned())
    }

    /// What the harness lists an orchestrator as: its short name and its
    /// quest's title.
    fn listed(&self, address: &str) -> Option<(String, String)> {
        let listed = self
            .orchestrators
            .as_ref()?
            .get("orchestrators")?
            .as_array()?;
        let one = listed
            .iter()
            .find(|one| one.get("address").and_then(Value::as_str) == Some(address))?;
        let said = |field| {
            one.get(field)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        Some((said("short"), said("title")))
    }

    /// Every record served, made from what each source last said.
    pub fn made(&self) -> BTreeMap<String, Value> {
        let joel_can = JoelCan::from_to_dos(self.to_dos.records().map(|(_, record)| record));
        let jobs: Vec<Value> = self.jobs.records().map(|(_, job)| job.clone()).collect();
        let spent = |id: &str| self.spent(id);
        let family = |quest: &str| self.family(quest);
        let listed = |address: &str| self.listed(address);
        // Whether a session has a window to go to comes from the windows
        // daemon (3TXW9E, Thu 12 Nov); until then one here is taken to
        // have its own, as a held session's kitty window is.
        let windowed = |session: Session| session.here();
        let known = Known {
            joel_can: &joel_can,
            jobs: &jobs,
            spent: &spent,
            family: &family,
            listed: &listed,
            windowed: &windowed,
            sharing: self.sharing,
        };
        let sessions =
            session::in_order(self.sessions.records().map(|(_, one)| Session(one)), &|_| 0);
        let states: Vec<_> = sessions
            .iter()
            .map(|one| one.stands(&joel_can, &jobs))
            .collect();
        let today = if self.sharing {
            ""
        } else {
            self.spend
                .as_ref()
                .and_then(|spend| spend.get("agentsToday"))
                .and_then(Value::as_str)
                .unwrap_or("")
        };
        let rows = row::rows(&sessions, &known, &self.words);
        served::records(&rows, &summary::summary(&states, today, &self.words.agents))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::said::Said;

    fn inputs() -> Inputs {
        let mut inputs = Inputs {
            words: Words {
                agents: Said::of(&[
                    ("working", "working"),
                    ("listening", "listening"),
                    ("workingCount", "{count} working"),
                    ("today", "{spent} today"),
                    ("allOf", "{spent} in all"),
                    ("needsCount", "1 needs you"),
                ]),
                machine: Said::default(),
                wants: Said::of(&[("question", "needs you")]),
            },
            spend: Some(json!({"agentsToday": "$214", "sessions": {"s-p": "$4"},
                "families": {"QEST-O": {"all": "$9"}}})),
            orchestrators: Some(json!({"orchestrators": [
                {"address": "O", "short": "kelp", "title": "Kelp", "quest": "QEST-O"}]})),
            ..Inputs::default()
        };
        inputs.sessions.took(&json!({"records": [
            {"key": "session:o", "standing": "running", "here": true,
             "presence": {"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"]}},
            {"key": "session:p", "standing": "running", "here": true,
             "presence": {"addresses": ["P"], "session": "s-p", "busy": true,
                          "orchestratedBy": "O"}},
            {"key": "session:x", "standing": "closed",
             "presence": {"addresses": ["X"]}}
        ]}));
        inputs
    }

    #[test]
    fn every_record_is_made_from_what_each_source_said() {
        let made = inputs().made();
        assert_eq!(made[served::ORDER]["keys"], json!(["agent:O", "agent:P"]));
        assert_eq!(
            made["agent:O"]["side"], "listening \u{b7} $9 in all",
            "one the harness lists orchestrates, saying its orchestration's spend"
        );
        assert_eq!(made["agent:O"]["also"], "O  QEST-O kelp Kelp");
        assert_eq!(made["agent:P"]["side"], "working \u{b7} $4");
        assert_eq!(made[served::SUMMARY]["said"], "1 working \u{b7} $214 today");
        assert!(!made.contains_key("agent:X"), "one closed is Recent's");
    }

    #[test]
    fn sharing_the_screen_says_no_amount() {
        let mut inputs = inputs();
        inputs.sharing = true;
        let made = inputs.made();
        assert_eq!(made["agent:P"]["side"], "working");
        assert_eq!(made[served::SUMMARY]["said"], "1 working");
    }

    #[test]
    fn a_question_to_joel_counts_him_needed_and_only_that_session_moves() {
        let mut inputs = inputs();
        let mut shown = served::Served::default();
        shown.take(inputs.made());
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:o",
            "presence": {"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"],
                "wants": {"kind": "question"}}}]}));
        let moved = shown.take(inputs.made());
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(
            keys,
            [json!("agent:O"), json!(served::SUMMARY)],
            "an orchestrator's own question is Joel's, and moves its row and the summary"
        );
        assert_eq!(
            shown.get(served::SUMMARY).map(|one| one["said"].clone()),
            Some(json!("1 needs you \u{b7} 1 working \u{b7} $214 today"))
        );
    }
}
