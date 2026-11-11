//! Everything Agents' rows are made from, as each source last told it:
//! the node's sessions and the machine's queue, the to-do daemon's
//! to-dos, spend-watch's collection and budgets, the harness's list of
//! orchestrators, and whether the shell says the screen is shared. Each
//! source changes its own part, and `made` makes every record from all of
//! them.

use std::collections::HashSet;

use daemon_serve::watch::Held;
use serde_json::Value;

use crate::clock;
use crate::drawer;
use crate::row::{self, Known, Shown, Words};
use crate::served;
use crate::session::{self, JoelCan, Session};
use crate::summary;

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
    /// Spend-watch's budgets, or none before they're written.
    pub budgets: Option<Value>,
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

    /// The titles of what a quest waits on Joel for, as For You says them:
    /// what he can do now, then what he can't yet, each in the to-do
    /// daemon's order, and nothing done.
    fn needs(&self, quest: &str) -> Vec<String> {
        let mut waiting: Vec<(bool, i64, String)> = self
            .to_dos
            .records()
            .filter_map(|(_, record)| {
                let said = |path| record.pointer(path).and_then(Value::as_str);
                let state = said("/state")?;
                let title = said("/item/title").filter(|title| !title.is_empty())?;
                (state != "done" && said("/item/quest") == Some(quest)).then(|| {
                    let order = record.get("order").and_then(Value::as_i64).unwrap_or(0);
                    (state != "now", order, title.to_owned())
                })
            })
            .collect();
        waiting.sort();
        waiting.into_iter().map(|(_, _, title)| title).collect()
    }

    /// Every record served, made from what each source last said.
    pub fn made(&self) -> served::Made {
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
        let today_spent = if self.sharing {
            ""
        } else {
            self.spend
                .as_ref()
                .and_then(|spend| spend.get("agentsToday"))
                .and_then(Value::as_str)
                .unwrap_or("")
        };
        let rows = row::rows(&sessions, &known, &self.words);
        let mut orchestrating = HashSet::new();
        orchestrators(&rows, &mut orchestrating);
        let needs = |quest: &str| self.needs(quest);
        let today = clock::today();
        let drawn = drawer::Known {
            needs: &needs,
            spend: self.spend.as_ref(),
            budgets: self.budgets.as_ref(),
            sharing: self.sharing,
            today: &today,
        };
        let drawers = sessions
            .iter()
            .map(|one| {
                let orchestrates = orchestrating.contains(one.address());
                drawer::drawer(*one, orchestrates, &self.words, &drawn)
            })
            .collect();
        served::made(
            &rows,
            &summary::summary(&states, today_spent, &self.words.agents),
        )
        .with_drawers(drawers)
    }
}

/// The addresses of those laid out as orchestrating others, folded or not.
fn orchestrators<'a>(shown: &'a [Shown], into: &mut HashSet<&'a str>) {
    for one in shown {
        match one {
            Shown::Session(row) if row.orchestrates => {
                into.insert(row.key.trim_start_matches("agent:"));
            }
            Shown::Session(_) => {}
            Shown::Fold(fold) => orchestrators(&fold.held, into),
        }
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
                names: Said::of(&[("spent", "Spent Today")]),
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

    /// Every record served of what the inputs say now.
    fn served(inputs: &Inputs) -> served::Served {
        let mut served = served::Served::default();
        served.take(inputs.made());
        served
    }

    /// What a session's row says at its right, its items joined.
    fn side(served: &served::Served, address: &str) -> String {
        let row = &served
            .get(&format!("row:agents:{address}"))
            .expect("its row")["row"];
        let items = row["side"].as_array().expect("its items");
        let said: Vec<_> = items
            .iter()
            .filter_map(|item| item["said"].as_str())
            .collect();
        said.join(" \u{b7} ")
    }

    fn summary_of(served: &served::Served) -> Value {
        served.get(served::SUMMARY).expect("the summary")["summary"]["said"].clone()
    }

    #[test]
    fn every_record_is_made_from_what_each_source_said() {
        let served = served(&inputs());
        let order = |address: &str| {
            served
                .get(&format!("row:agents:{address}"))
                .expect("its row")["row"]["order"]
                .clone()
        };
        assert_eq!((order("O"), order("P")), (json!(1024), json!(2048)));
        assert_eq!(
            side(&served, "O"),
            "listening \u{b7} $9 in all",
            "one the harness lists orchestrates, saying its orchestration's spend"
        );
        let also = &served.get("row:agents:O").expect("its row")["row"]["held"]["also"];
        assert_eq!(also, "O  QEST-O kelp Kelp");
        assert_eq!(side(&served, "P"), "working \u{b7} $4");
        assert_eq!(summary_of(&served), "1 working \u{b7} $214 today");
        assert!(
            served.get("row:agents:X").is_none(),
            "one closed is Recent's"
        );
    }

    #[test]
    fn each_session_listed_has_a_drawer_an_orchestrators_saying_all_it_spent() {
        let served = served(&inputs());
        let drawer = &served.get("drawer:agents:O").expect("O's drawer")["drawer"];
        assert_eq!(
            drawer["facts"][0],
            json!({"label": "Spent Today", "value": "$9 in all"}),
            "O orchestrates P, so says its whole orchestration's spend"
        );
        let drawer = &served.get("drawer:agents:P").expect("P's drawer")["drawer"];
        assert_eq!(
            drawer["facts"][0],
            json!({"label": "Spent Today", "value": "$4"})
        );
        assert!(
            served.get("drawer:agents:X").is_none(),
            "one closed is listed nowhere, so has no drawer"
        );
    }

    #[test]
    fn a_quests_needs_are_what_joel_can_do_now_then_later_never_done() {
        let mut inputs = inputs();
        inputs.to_dos.took(&json!({"records": [
            {"key": "to-do:QEST-O:b", "state": "later", "order": 1,
             "item": {"title": "Later", "quest": "QEST-O"}},
            {"key": "to-do:QEST-O:a", "state": "now", "order": 2,
             "item": {"title": "Now", "quest": "QEST-O"}},
            {"key": "to-do:QEST-O:c", "state": "done", "order": 0,
             "item": {"title": "Done", "quest": "QEST-O"}},
            {"key": "to-do:QEST-B:d", "state": "now", "order": 0,
             "item": {"title": "Elsewhere", "quest": "QEST-B"}}
        ]}));
        assert_eq!(inputs.needs("QEST-O"), ["Now", "Later"]);
    }

    #[test]
    fn sharing_the_screen_says_no_amount() {
        let mut inputs = inputs();
        inputs.sharing = true;
        let served = served(&inputs);
        assert_eq!(side(&served, "P"), "working");
        assert_eq!(summary_of(&served), "1 working");
    }

    #[test]
    fn a_question_to_joel_counts_him_needed_and_only_that_session_moves() {
        let mut inputs = inputs();
        let mut shown = served(&inputs);
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:o",
            "presence": {"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"],
                "wants": {"kind": "question"}}}]}));
        let moved = shown.take(inputs.made());
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(
            keys,
            [json!("row:agents:O"), json!(served::SUMMARY)],
            "an orchestrator's own question is Joel's, and moves its row and the summary"
        );
        assert_eq!(
            summary_of(&shown),
            "1 needs you \u{b7} 1 working \u{b7} $214 today"
        );
    }
}
