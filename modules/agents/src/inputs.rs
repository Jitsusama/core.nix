//! Everything Agents' rows are made from, as each source last told it:
//! the node's sessions and the machine's queue, the to-do daemon's
//! to-dos, the windows daemon's windows, spend-watch's collection and
//! budgets, the harness's list of orchestrators and of sessions to
//! reopen, what Joel said of those, and whether the shell says the
//! screen is shared. Each
//! source changes its own part, and `made` makes every record from all of
//! them.

use std::collections::{HashMap, HashSet};

use daemon_serve::watch::Held;
use serde_json::Value;

use crate::clock;
use crate::drawer;
use crate::recent;
use crate::reopen::{self, Kept};
use crate::row::{self, Known, Shown, Words};
use crate::served;
use crate::session::{self, JoelCan, Lives, Session};
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
    /// The windows daemon's `window` records.
    pub windows: Held,
    /// Spend-watch's collection, or none before it's written.
    pub spend: Option<Value>,
    /// Spend-watch's budgets, or none before they're written.
    pub budgets: Option<Value>,
    /// The harness's orchestrators file, or none.
    pub orchestrators: Option<Value>,
    /// Whether the screen is shared, so no amount is said.
    pub sharing: bool,
    /// The harness's restore.json, or none.
    pub restore: Option<Value>,
    /// This boot, as Linux names it, which the last boot's sessions are
    /// offered after.
    pub boot: String,
    /// What Joel said of the sessions to reopen.
    pub kept: Kept,
    /// The sessions the boot being restored kept a place of, by pi's id,
    /// as the shell keeps its arrangement and says it.
    pub arranged: HashSet<String>,
}

impl Inputs {
    /// When each session's window was last used, in milliseconds, by its
    /// address: the window the windows daemon says opened with the app id
    /// the session's did (`Session::opened`), the oldest of them where
    /// several did, as Sessions.qml finds it. One with none isn't said.
    fn used(&self) -> HashMap<String, u64> {
        let mut by_opened: HashMap<&str, (u64, u64)> = HashMap::new();
        for (_, window) in self.windows.records() {
            let opened = window.get("opened").and_then(Value::as_str);
            let id = window.get("id").and_then(Value::as_u64);
            let (Some(opened), Some(id)) = (opened, id) else {
                continue;
            };
            let order = window.get("order").and_then(Value::as_u64).unwrap_or(0);
            let kept = by_opened.entry(opened).or_insert((id, order));
            if id < kept.0 {
                *kept = (id, order);
            }
        }
        self.sessions
            .records()
            .filter_map(|(_, one)| {
                let session = Session(one);
                let (_, order) = by_opened.get(session.opened().as_str())?;
                Some((session.address().to_owned(), *order))
            })
            .collect()
    }

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

    /// Every session there is to reopen, by the sessions running or
    /// parked here now. One pi's host runs
    /// is back only once its viewer is open, or at once when the boot
    /// being restored kept no place of its window (`arranged`), since it
    /// had none to bring back.
    fn reopens(&self) -> Vec<Value> {
        let viewing: HashSet<&str> = self
            .windows
            .records()
            .filter_map(|(_, window)| window["opened"].as_str()?.strip_prefix("pi-hosted."))
            .collect();
        let here: Vec<reopen::Here> = self
            .sessions
            .records()
            .filter(|(_, one)| {
                let standing = one["standing"].as_str();
                let id = one["presence"]["session"].as_str().unwrap_or("");
                let hosted =
                    !one["presence"]["hosted"].is_null() && one["presence"]["hosted"] != false;
                one["here"] == true
                    && matches!(standing, Some("running" | "parked"))
                    && (!hosted || viewing.contains(id) || !self.arranged.contains(id))
            })
            .filter_map(|(_, one)| {
                let presence = &one["presence"];
                let id = presence["session"].as_str()?;
                let quest = presence["questInfo"]["id"]
                    .as_str()
                    .or_else(|| presence["quest"].as_str())
                    .unwrap_or("");
                Some(reopen::Here { id, quest })
            })
            .collect();
        reopen::offered(self.restore.as_ref(), &self.boot, &self.kept, &here)
    }

    /// Recent's sessions, leaving out those Agents shows elsewhere: one
    /// running or parked, any machine's, by its address or its quest, an
    /// orchestrator the harness lists, and those `reopening`.
    fn recent<'a>(&'a self, reopening: &[Value]) -> Vec<Value> {
        let live: Vec<Session<'a>> = self
            .sessions
            .records()
            .map(|(_, one)| Session(one))
            .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
            .collect();
        let listed = self
            .orchestrators
            .as_ref()
            .and_then(|said| said["orchestrators"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|one| one["address"].as_str());
        let quest = |one: &Session<'a>| -> Option<&'a str> {
            let presence = one.presence()?;
            presence["questInfo"]["id"]
                .as_str()
                .or_else(move || presence["quest"].as_str())
        };
        let shown = recent::Shown {
            addresses: live.iter().map(|one| one.address()).chain(listed).collect(),
            quests: live
                .iter()
                .filter_map(quest)
                .filter(|one| !one.is_empty())
                .collect(),
            reopening: reopening
                .iter()
                .filter_map(|one| one["session"]["id"].as_str())
                .map(str::to_owned)
                .collect(),
        };
        recent::recent(self.sessions.records().map(|(_, one)| Session(one)), &shown)
    }

    /// Every record served, made from what each source last said.
    pub fn made(&self) -> served::Made {
        let joel_can = JoelCan::from_to_dos(self.to_dos.records().map(|(_, record)| record));
        let jobs: Vec<Value> = self.jobs.records().map(|(_, job)| job.clone()).collect();
        let spent = |id: &str| self.spent(id);
        let family = |quest: &str| self.family(quest);
        let listed = |address: &str| self.listed(address);
        let windowed = |session: Session| session.windowed();
        let known = Known {
            joel_can: &joel_can,
            jobs: &jobs,
            spent: &spent,
            family: &family,
            listed: &listed,
            windowed: &windowed,
            sharing: self.sharing,
        };
        let mut reopens = self.reopens();
        let recent = self.recent(&reopens);
        // With them, the boot they're of, which the shell keeps places by.
        reopens.push(reopen::restoring(self.restore.as_ref()));
        let used = self.used();
        let last_used = |address: &str| used.get(address).copied().unwrap_or(0);
        let sessions = session::in_order(
            self.sessions.records().map(|(_, one)| Session(one)),
            &last_used,
        );
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
        .with_recent(recent)
        .with_reopens(reopens)
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

    #[test]
    fn a_sessions_window_was_used_when_the_oldest_opened_with_its_app_id_was() {
        let mut inputs = inputs();
        inputs.sessions.took(&json!({"records": [
            {"key": "session:o", "standing": "running", "window": {"app_id": "pi-session.1"},
             "presence": {"addresses": ["O"]}},
            {"key": "session:p", "standing": "running",
             "presence": {"addresses": ["P"], "session": "s-p", "hosted": true}},
            {"key": "session:q", "standing": "running", "presence": {"addresses": ["Q"]}}
        ]}));
        inputs.windows.took(&json!({"records": [
            {"key": "window:7", "id": 7, "opened": "pi-session.1", "order": 10},
            {"key": "window:9", "id": 9, "opened": "pi-session.1", "order": 99},
            {"key": "window:8", "id": 8, "opened": "pi-hosted.s-p", "order": 20},
            {"key": "window:3", "id": 3, "opened": "kitty", "order": 50}
        ]}));
        let used = inputs.used();
        assert_eq!(used.get("O"), Some(&10), "the oldest window of its app id");
        assert_eq!(used.get("P"), Some(&20), "a hosted one's viewer");
        assert_eq!(
            used.get("Q"),
            None,
            "one opened without an app id isn't said"
        );
    }

    /// Every record served of what the inputs say now.
    fn served(inputs: &Inputs) -> served::Served {
        let mut served = served::Served::default();
        served.take(inputs.made());
        served
    }

    /// The same, where a test's own `served` hides it.
    fn served_of(inputs: &Inputs) -> served::Served {
        served(inputs)
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
    fn the_last_boots_sessions_are_served_until_each_is_back_here() {
        let mut inputs = inputs();
        inputs.boot = "this-one".into();
        inputs.restore = Some(json!({"boot": "the-last", "sessions": [
            {"id": "s-o", "cwd": "/tmp"},
            {"id": "s-gone", "cwd": "/tmp"},
            {"id": "s-viewed", "cwd": "/tmp", "hosted": true}
        ]}));
        inputs.sessions.took(&json!({"changed": [
            {"key": "session:v", "standing": "running", "here": true,
             "presence": {"addresses": ["V"], "session": "s-viewed",
                 "hosted": {"host": "attended"}}}
        ]}));
        let served = served(&inputs);
        assert!(served.get("reopen:agents:s-o").is_none(), "s-o runs here");
        assert_eq!(
            served
                .get("reopen:agents:s-gone")
                .map(|one| one["way"].clone()),
            Some(json!("boot")),
            "one not back is served as the last boot's"
        );
        assert!(
            served.get("reopen:agents:s-viewed").is_none(),
            "one pi's host runs is back while no place of its window is kept"
        );
        assert_eq!(
            served
                .get("restoring:agents")
                .map(|one| one["restoring"].clone()),
            Some(json!({"boot": "the-last"})),
            "the boot being restored, for its places"
        );
        inputs.arranged = HashSet::from(["s-viewed".to_owned()]);
        assert!(
            served_of(&inputs).get("reopen:agents:s-viewed").is_some(),
            "one pi's host runs, with a place kept, waits for its viewer"
        );
        inputs.windows.took(&json!({"records": [
            {"key": "window:4", "id": 4, "opened": "pi-hosted.s-viewed", "order": 1}
        ]}));
        assert!(
            served_of(&inputs).get("reopen:agents:s-viewed").is_none(),
            "and is back once its viewer is open"
        );
    }

    #[test]
    fn recent_lists_one_closed_unless_its_to_reopen_or_its_orchestrator_listed() {
        let mut inputs = inputs();
        inputs.boot = "this-one".into();
        inputs.restore = Some(json!({"boot": "the-last", "sessions": [
            {"id": "s-y", "cwd": "/tmp"}
        ]}));
        inputs.orchestrators = Some(json!({"orchestrators": [{"address": "Z"}]}));
        inputs.sessions.took(&json!({"changed": [
            {"key": "session:y", "standing": "closed",
             "presence": {"addresses": ["Y"], "session": "s-y"}},
            {"key": "session:z", "standing": "closed",
             "presence": {"addresses": ["Z"], "session": "s-z"}}
        ]}));
        let served = served(&inputs);
        let listed: Vec<_> = served
            .all()
            .filter(|record| record["kind"] == "recent")
            .map(|record| record["recent"]["id"].clone())
            .collect();
        assert_eq!(
            listed,
            [json!("x")],
            "X closed; Y is to reopen, Z a closed orchestrator"
        );
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
