//! A session's row, made ready for the launcher as Agents.qml's madeAnew
//! made it: its name, what it says at its right and in its note, its
//! glyph's and hue's roles, how deep it sits, what Return does and its
//! acts by id, and itself as it reads ranked flat. The launcher draws it
//! and maps each act's id to what it does; it works nothing out.

use serde::Serialize;
use serde_json::{Value, json};

use crate::figures::{self, Item};
use crate::said::{Said, fill};
use crate::session::{self, JoelCan, Session};
use crate::standing::{self, Stands};
use crate::tree::{self, Laid, Member};

/// The vocabulary's words a row says: the agents' own, those for a
/// session elsewhere (`agents:machine`) and those for what it wants
/// (`agents:wants`); the names a drawer gives its facts, each
/// `agents:<id>`'s by its id; and Sessions to Reopen's, with Reopen's
/// for a folder gone (`agents:to-reopen`, `verb:agents:to-reopen:one`).
#[derive(Debug, Default)]
pub struct Words {
    pub agents: Said,
    pub machine: Said,
    pub wants: Said,
    pub names: Said,
    pub to_reopen: Said,
    pub reopen_one: Said,
    pub landed: Said,
    /// Recent's, for what its place says of what it holds.
    pub recent: Said,
    /// A session's acts' names and marks, by their ids (`talk` is Talk).
    pub acts: Said,
    pub act_glyphs: Said,
    /// How a talk went, by its stage, and its parts' names (talks.rs).
    pub talk_stages: Said,
    pub talk_parts: Said,
}

impl Words {
    /// The words from the launcher's vocabulary's text.
    pub fn from_vocabulary(text: &str) -> serde_json::Result<Self> {
        Ok(Self {
            agents: Said::of_thing(text, "agents")?,
            machine: Said::of_thing(text, "agents:machine")?,
            wants: Said::of_thing(text, "agents:wants")?,
            names: Said::names_under(text, "agents")?,
            to_reopen: Said::of_thing(text, "agents:to-reopen")?,
            reopen_one: Said::of_thing(text, "verb:agents:to-reopen:one")?,
            landed: Said::of_thing(text, "agents:landed")?,
            recent: Said::of_thing(text, "agents:recent")?,
            acts: Said::names_under(text, "verb:agent")?,
            act_glyphs: Said::glyphs_under(text, "verb:agent")?,
            talk_stages: Said::of_thing(text, "orchestrators:talk")?,
            talk_parts: Said::names_under(text, "orchestrators:talk")?,
        })
    }
}

/// What a row is made from beyond the session's own record, each as its
/// daemon tells of it.
pub struct Known<'a> {
    /// Who has something Joel can do now.
    pub joel_can: &'a JoelCan,
    /// The machine's queue of heavy work.
    pub jobs: &'a [Value],
    /// What a session spent today, by pi's id for it (spend-watch).
    pub spent: &'a dyn Fn(&str) -> Option<String>,
    /// What a quest and those under it spent today in all, by its id.
    pub family: &'a dyn Fn(&str) -> Option<String>,
    /// What the harness lists an orchestrator as, its short name and its
    /// quest's title, by its address.
    pub listed: &'a dyn Fn(&str) -> Option<(String, String)>,
    /// Whether a session has a window to go to.
    pub windowed: &'a dyn Fn(Session) -> bool,
    /// Whether the screen is shared, so no amount is said.
    pub sharing: bool,
    /// How many to-dos a quest waits on Joel for, which its session's
    /// Close says stay.
    pub needs: &'a dyn Fn(&str) -> usize,
}

/// Where a row is placed: how deep, and whether it orchestrates others.
#[derive(Clone, Copy, Debug, Default)]
pub struct Place {
    pub depth: usize,
    pub orchestrates: bool,
}

/// A session's row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub key: String,
    pub name: String,
    /// The words typing finds it by beyond its name: its address, folder
    /// and quest, and what the harness lists an orchestrator as.
    pub also: String,
    pub glyph: &'static str,
    pub hue: &'static str,
    pub side: String,
    pub side_items: Vec<Item>,
    pub note: String,
    pub depth: usize,
    /// Whether it orchestrates others, so its drawer says what its whole
    /// orchestration spent.
    pub orchestrates: bool,
    /// Out of reach, so it does nothing.
    pub gone: bool,
    pub state: &'static str,
    pub returns: &'static str,
    pub acts: Vec<&'static str>,
    /// What Close's ask says stays, where it has a Close, so the launcher
    /// asks it as served rather than working it out from the session's
    /// folder; "" where it has none.
    pub leaves: String,
    /// Joel's talk to it on its way to an answer, its id and stage, so
    /// the launcher ties what it says of the talk to the row; none
    /// where no talk waits.
    pub talk: Option<serde_json::Value>,
    pub ranked: Ranked,
}

/// A row as it reads ranked flat: at no depth, whom it's under leading
/// its note and what it says at its right, given way first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ranked {
    pub note: String,
    pub side: String,
    pub side_items: Vec<Item>,
    pub belongs: String,
}

/// An orchestration's members that only wait, folded into one row named
/// by how many stand each way, "22 listening, 3 paused", holding theirs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fold {
    pub key: String,
    pub name: String,
    pub glyph: &'static str,
    pub hue: &'static str,
    pub depth: usize,
    pub held: Vec<Shown>,
}

/// One entry of Agents' list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Shown {
    Session(Box<Row>),
    Fold(Fold),
}

impl Shown {
    pub fn key(&self) -> &str {
        match self {
            Self::Session(row) => &row.key,
            Self::Fold(fold) => &fold.key,
        }
    }
}

const BETWEEN: &str = " \u{b7} ";

fn joined(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(BETWEEN)
}

/// What it works on, less the lead its row's name already says: the
/// record's `worksOn` leads with its quest's title (SessionDrawer.about).
pub fn about(works: &str, titles: &[&str]) -> String {
    titles
        .iter()
        .filter(|title| !title.is_empty())
        .find_map(|title| works.strip_prefix(&format!("{title}: ")))
        .unwrap_or(works)
        .to_owned()
}

/// What a session does, as its row says it: what it wants of Joel, its
/// place in line, or the word for how it stands.
fn doing(stands: Stands, session: Session, known: &Known, words: &Words) -> String {
    match stands {
        Stands::Off => String::new(),
        Stands::Needs => {
            let kind = session
                .presence()
                .and_then(|presence| presence.pointer("/wants/kind"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let said = words.wants.word(kind);
            (if said.is_empty() {
                words.wants.word("question")
            } else {
                said
            })
            .to_owned()
        }
        Stands::InLine => {
            session::in_line(known.jobs, session.id()).map_or_else(String::new, |place| {
                words
                    .agents
                    .fill("inLine", &[("place", &session::ordinal(place))])
            })
        }
        _ => words.agents.word(stands.id()).to_owned(),
    }
}

/// What a presence says at `field`, a JSON pointer, as words.
fn text<'a>(presence: Option<&'a Value>, field: &str) -> Option<&'a str> {
    presence?.pointer(field)?.as_str()
}

/// What else a row is found by: an orchestration's short id and title as
/// its quest lists them, then the words a session standing at the top goes
/// by, each after a space.
fn known_as(listed: Option<(String, String)>, keywords: &[&str]) -> String {
    let mut known = listed.map_or_else(String::new, |(short, title)| format!(" {short} {title}"));
    for keyword in keywords {
        known.push(' ');
        known.push_str(keyword);
    }
    known
}

/// What a session's row notes: the machine it's on where it isn't here,
/// then its `note`, which one out of reach doesn't say.
fn noted(session: Session, note: &str, words: &Words) -> String {
    let machine = [("machine", session.machine())];
    let place_said = if session.here() {
        String::new()
    } else {
        fill(
            words
                .machine
                .word(if session.reachable() { "on" } else { "away" }),
            &machine,
        )
    };
    if session.reachable() {
        joined(&[place_said.as_str(), note])
    } else {
        place_said
    }
}

/// A session's row, `place`d in the tree.
pub fn row(session: Session, place: Place, known: &Known, words: &Words) -> Row {
    let presence = session.presence();
    let said = |field| text(presence, field);
    let address = session.address();
    let (title, folder) = session.named();
    let quest = session.quest();
    let asks = session::asks_joel(presence, known.joel_can);
    let stands = session.stands(known.joel_can, known.jobs);
    let note = match said("/wants/said").filter(|wants| asks && !wants.is_empty()) {
        Some(wants) => wants.to_owned(),
        None => about(
            said("/worksOn").unwrap_or(""),
            &[&title, said("/questInfo/title").unwrap_or("")],
        ),
    };
    let noted = noted(session, &note, words);
    let orchestrator = session.under();
    let under = if orchestrator.is_empty() || orchestrator == address {
        String::new()
    } else {
        words.agents.fill("under", &[("address", orchestrator)])
    };
    let spent = if known.sharing {
        None
    } else {
        (known.spent)(session.id())
    };
    let family = if place.orchestrates && !quest.is_empty() && !known.sharing {
        (known.family)(quest)
    } else {
        None
    };
    let figure = family.map_or_else(
        || spent.unwrap_or_default(),
        |all| words.agents.fill("allOf", &[("spent", &all)]),
    );
    let listed = place
        .orchestrates
        .then(|| (known.listed)(address))
        .flatten();
    let at_top = session.at_top();
    let known_as = known_as(listed, at_top.as_ref().map_or(&[], |(_, words)| words));
    let full = if stands == Stands::Off {
        String::new()
    } else {
        figures::full(
            presence.and_then(|presence| presence.get("usage")),
            &words.agents,
        )
    };
    let items = figures::side(&doing(stands, session, known, words), &figure, &full);
    let windowed = (known.windowed)(session);
    let ranked_items: Vec<Item> = std::iter::once(Item {
        said: under.clone(),
        keep: 1,
    })
    .chain(items.iter().cloned())
    .filter(|item| !item.said.is_empty())
    .collect();
    Row {
        key: format!("agent:{address}"),
        name: title,
        also: format!("{address} {folder} {quest}{known_as}"),
        glyph: "agent",
        hue: standing::hue(stands, !full.is_empty()),
        side: said_of(&items),
        side_items: items,
        note: noted.clone(),
        depth: place.depth,
        orchestrates: place.orchestrates,
        gone: !session.reachable(),
        state: stands.id(),
        // One standing at the top is gone to to be told something, the
        // void above all, so Return talks to it however it stands.
        returns: if at_top.is_some() {
            "talk"
        } else {
            standing::returns(stands, windowed)
        },
        acts: standing::acts(stands, windowed, session.here()),
        leaves: leaves(session, known, words),
        talk: None,
        ranked: Ranked {
            note: joined(&[&under, &noted]),
            side: said_of(&ranked_items),
            side_items: ranked_items,
            belongs: under,
        },
    }
}

impl Row {
    /// The act `id` as the launcher's rows hold it: its verb's key and the
    /// ask that does it, which the shell sends back as it is. Close says
    /// too what its ask names and what stays, so the launcher asks it as
    /// served ("Close 8IDU4E? Its worktree and 1 to-do stay.").
    pub fn act(&self, id: &str) -> Value {
        let address = self.key.trim_start_matches("agent:");
        let mut act = json!({"key": format!("verb:agent:{id}"),
            "ask": {"ask": id, "address": address}});
        if id == "close" && !self.leaves.is_empty() {
            act["askedAs"] = json!(address);
            act["leaves"] = json!(self.leaves);
        }
        act
    }

    /// Says first at its right that it answered a talk Joel hasn't read,
    /// or how one of his stands on its way, given way last, flat or under
    /// its orchestrator alike (Joel, 2026-11-11: "how do i know when a
    /// sessions answers a talk?").
    pub fn says_first(&mut self, said: &str) {
        let item = Item {
            said: said.to_owned(),
            keep: 6,
        };
        self.side_items.insert(0, item.clone());
        self.side = said_of(&self.side_items);
        self.ranked.side_items.insert(0, item);
        self.ranked.side = said_of(&self.ranked.side_items);
    }

    /// Says first how Joel's talk to it stands on its way (`said`), and
    /// holds which talk it is and its stage.
    pub fn waits(&mut self, said: &str, talk: &str, stage: &str) {
        self.says_first(said);
        self.talk = Some(serde_json::json!({"id": talk, "stage": stage}));
    }
}

/// What Close leaves of a session here: its worktree, where it works in a
/// folder under a trees folder, as the harness's and World's trees both
/// are cut, else its folder, and its quest's to-dos; "" for one
/// elsewhere, which has no Close.
fn leaves(session: Session, known: &Known, words: &Words) -> String {
    if !session.here() {
        return String::new();
    }
    let in_tree = text(session.presence(), "/cwd").is_some_and(|cwd| cwd.contains("/trees/"));
    let quest = session.quest();
    let needs = if quest.is_empty() {
        0
    } else {
        (known.needs)(quest)
    };
    standing::leaves(in_tree, needs, &words.agents)
}

fn said_of(items: &[Item]) -> String {
    items
        .iter()
        .map(|item| item.said.as_str())
        .collect::<Vec<_>>()
        .join(BETWEEN)
}

/// A fold's name: how many of its sessions stand each way.
pub fn fold_name(counts: &[(Stands, usize)], words: &Words) -> String {
    counts
        .iter()
        .map(|(stands, count)| format!("{count} {}", words.agents.word(stands.id())))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every session's row, in Sessions' order (`session::in_order`), laid
/// under their orchestrators with the waiting folded (tree.rs). One the
/// harness lists as an orchestrator orchestrates, though none of its
/// own runs, so it says what its whole orchestration spent.
pub fn rows(sessions: &[Session], known: &Known, words: &Words) -> Vec<Shown> {
    let members: Vec<Member> = sessions
        .iter()
        .map(|session| Member {
            address: session.address(),
            under: session.under(),
            stands: session.stands(known.joel_can, known.jobs),
            at_top: session.at_top().is_some(),
        })
        .collect();
    shown(&tree::tree(&members, true), sessions, known, words)
}

fn shown(laid: &[Laid], sessions: &[Session], known: &Known, words: &Words) -> Vec<Shown> {
    laid.iter()
        .map(|one| match one {
            Laid::Row {
                at,
                depth,
                orchestrates,
                ..
            } => {
                let session = sessions[*at];
                let orchestrates = *orchestrates || (known.listed)(session.address()).is_some();
                Shown::Session(Box::new(row(
                    session,
                    Place {
                        depth: *depth,
                        orchestrates,
                    },
                    known,
                    words,
                )))
            }
            Laid::Fold {
                depth,
                under,
                members,
                counts,
            } => Shown::Fold(Fold {
                key: format!("agents:fold:{under}"),
                name: fold_name(counts, words),
                glyph: "agent",
                hue: "program",
                depth: *depth,
                held: shown(members, sessions, known, words),
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    struct Given {
        joel_can: JoelCan,
        jobs: Vec<Value>,
        sharing: bool,
    }

    fn given() -> Given {
        Given {
            joel_can: JoelCan::default(),
            jobs: Vec::new(),
            sharing: false,
        }
    }

    fn made(record: &Value, place: Place, given: &Given) -> Row {
        let spent = |id: &str| (id == "s-1").then(|| "$4.20".to_owned());
        let family = |quest: &str| (quest == "QEST-A").then(|| "$9".to_owned());
        let listed =
            |address: &str| (address == "O").then(|| ("kelp".to_owned(), "Kelp".to_owned()));
        let windowed = |_: Session| true;
        let known = Known {
            joel_can: &given.joel_can,
            jobs: &given.jobs,
            spent: &spent,
            family: &family,
            listed: &listed,
            windowed: &windowed,
            sharing: given.sharing,
            needs: &|quest| usize::from(quest == "QEST-B"),
        };
        row(Session(record), place, &known, &words())
    }

    fn working() -> Value {
        json!({"presence": {"addresses": ["P"], "session": "s-1", "busy": true,
            "orchestratedBy": "O", "cwd": "/src/reef", "owns": ["QEST-B"],
            "questInfo": {"title": "Reef"}, "worksOn": "Reef: mends the reef"}})
    }

    #[test]
    fn a_working_session_says_what_it_does_and_spent() {
        let row = made(
            &working(),
            Place {
                depth: 1,
                orchestrates: false,
            },
            &given(),
        );
        assert_eq!(row.key, "agent:P");
        assert_eq!(row.name, "Reef");
        assert_eq!(row.also, "P reef QEST-B");
        assert_eq!(row.side, "working \u{b7} $4.20");
        assert_eq!(
            row.note, "mends the reef",
            "its quest's title isn't said twice"
        );
        assert_eq!((row.depth, row.state, row.hue), (1, "working", "program"));
        assert_eq!(row.ranked.note, "under O \u{b7} mends the reef");
        assert_eq!(row.ranked.side, "under O \u{b7} working \u{b7} $4.20");
        assert_eq!(
            row.ranked.side_items[0],
            Item {
                said: "under O".into(),
                keep: 1
            }
        );
        assert_eq!(row.ranked.belongs, "under O");
    }

    #[test]
    fn nothing_spent_is_said_while_the_screen_is_shared() {
        let sharing = Given {
            sharing: true,
            ..given()
        };
        assert_eq!(made(&working(), Place::default(), &sharing).side, "working");
    }

    #[test]
    fn an_orchestrator_says_its_whole_orchestrations_spend_and_listing() {
        let record = json!({"presence": {"addresses": ["O"], "session": "s-1",
            "owns": ["QEST-A"]}});
        let row = made(
            &record,
            Place {
                depth: 0,
                orchestrates: true,
            },
            &given(),
        );
        assert_eq!(row.side, "listening \u{b7} $9 in all");
        assert_eq!(row.also, "O  QEST-A kelp Kelp");
        assert_eq!(row.ranked.belongs, "", "an orchestrator is under nobody");
    }

    #[test]
    fn a_want_that_is_joels_leads_its_note() {
        let record = json!({"presence": {"addresses": ["P"],
            "wants": {"kind": "approval", "said": "May I push?"}, "worksOn": "x"}});
        let row = made(&record, Place::default(), &given());
        assert_eq!((row.state, row.side.as_str()), ("needs", "needs your yes"));
        assert_eq!(row.note, "May I push?");
    }

    #[test]
    fn a_session_waiting_its_turn_says_its_place() {
        let queued = Given {
            jobs: vec![json!({"owner": {"session": "s-1"}, "state": "waiting", "place": 2})],
            ..given()
        };
        let row = made(&working(), Place::default(), &queued);
        assert_eq!(row.side, "2nd in line \u{b7} $4.20");
    }

    #[test]
    fn a_session_elsewhere_leads_with_its_machine_and_out_of_reach_says_only_that() {
        let mut record = working();
        record["here"] = json!(false);
        record["machine"] = json!("skiff");
        assert_eq!(
            made(&record, Place::default(), &given()).note,
            "on skiff \u{b7} mends the reef"
        );
        record["reachable"] = json!(false);
        let row = made(&record, Place::default(), &given());
        assert_eq!(row.note, "skiff, out of reach");
        assert!(row.gone, "out of reach, it does nothing");
    }

    #[test]
    fn close_says_its_tree_or_folder_and_its_quests_to_dos_stay_and_elsewhere_has_none() {
        let mut record = working();
        let leaves = |record: &Value| made(record, Place::default(), &given()).leaves;
        assert_eq!(leaves(&record), "its folder and 1 to-do stay.");
        record["presence"]["cwd"] = json!("/src/trees/reef-mend");
        record["presence"]["owns"] = json!([]);
        assert_eq!(leaves(&record), "its worktree stays.");
        record["here"] = json!(false);
        assert_eq!(leaves(&record), "", "one elsewhere has no Close");
    }

    #[test]
    fn its_close_is_asked_naming_it_and_saying_what_stays_and_its_other_acts_only_ask() {
        let row = made(&working(), Place::default(), &given());
        assert_eq!(
            row.act("close"),
            json!({"key": "verb:agent:close", "ask": {"ask": "close", "address": "P"},
                "askedAs": "P", "leaves": "its folder and 1 to-do stay."})
        );
        assert_eq!(
            row.act("pause"),
            json!({"key": "verb:agent:pause", "ask": {"ask": "pause", "address": "P"}})
        );
    }

    #[test]
    fn a_full_context_leads_and_warns() {
        let mut record = working();
        record["presence"]["usage"] = json!({"context": {"used": 842, "limit": 1000}});
        let row = made(&record, Place::default(), &given());
        assert!(row.side.starts_with("84% full"), "{}", row.side);
        assert_eq!(row.hue, "warning");
    }

    #[test]
    fn about_drops_only_a_title_that_leads() {
        assert_eq!(about("Reef: mends it", &["", "Reef"]), "mends it");
        assert_eq!(about("mends Reef: it", &["Reef"]), "mends Reef: it");
    }

    #[test]
    fn waiting_members_fold_under_their_orchestrator_named_by_their_counts() {
        let records = [
            json!({"presence": {"addresses": ["O"], "busy": true}}),
            json!({"presence": {"addresses": ["A"], "orchestratedBy": "O"}}),
            json!({"presence": {"addresses": ["B"], "orchestratedBy": "O", "state": "paused"}}),
            json!({"presence": {"addresses": ["X"], "orchestratedBy": "O", "busy": true}}),
        ];
        let sessions: Vec<_> = records.iter().map(Session).collect();
        let given = given();
        let none = |_: &str| None::<String>;
        let unlisted = |_: &str| None::<(String, String)>;
        let windowed = |_: Session| false;
        let known = Known {
            joel_can: &given.joel_can,
            jobs: &given.jobs,
            spent: &none,
            family: &none,
            listed: &unlisted,
            windowed: &windowed,
            sharing: false,
            needs: &|_| 0,
        };
        let shown = rows(&sessions, &known, &words());
        let keys: Vec<_> = shown.iter().map(Shown::key).collect();
        assert_eq!(keys, ["agent:O", "agent:X", "agents:fold:O"]);
        let Shown::Fold(fold) = &shown[2] else {
            panic!("the waiting fold: {shown:?}")
        };
        assert_eq!(
            (fold.name.as_str(), fold.depth),
            ("1 listening, 1 paused", 1)
        );
        let held: Vec<_> = fold.held.iter().map(Shown::key).collect();
        assert_eq!(held, ["agent:A", "agent:B"]);
    }

    #[test]
    fn a_session_standing_at_the_top_is_never_folded_and_goes_by_its_own_name() {
        // Joel looked for the void and found nothing: it was one of 23 in
        // its orchestrator's fold (QCXZEB, 2026-11-12).
        let top = json!({"name": "The Void", "keywords": ["void", "say it once"]});
        let records = [
            json!({"presence": {"addresses": ["O"], "busy": true}}),
            json!({"presence": {"addresses": ["A"], "orchestratedBy": "O"}}),
            json!({"presence": {"addresses": ["B"], "orchestratedBy": "O", "state": "paused"}}),
            json!({"presence": {"addresses": ["V"], "orchestratedBy": "O", "busy": true,
                "questInfo": {"title": "Say It Once"}, "standsAtTop": top}}),
        ];
        let sessions: Vec<_> = records.iter().map(Session).collect();
        let given = given();
        let none = |_: &str| None::<String>;
        let unlisted = |_: &str| None::<(String, String)>;
        let windowed = |_: Session| true;
        let known = Known {
            joel_can: &given.joel_can,
            jobs: &given.jobs,
            spent: &none,
            family: &none,
            listed: &unlisted,
            windowed: &windowed,
            sharing: false,
            needs: &|_| 0,
        };
        let shown = rows(&sessions, &known, &words());
        let keys: Vec<_> = shown.iter().map(Shown::key).collect();
        assert_eq!(
            keys,
            ["agent:V", "agent:O", "agents:fold:O"],
            "at the top, under nobody, before the orchestration it left"
        );
        let Shown::Session(row) = &shown[0] else {
            panic!("its own row: {shown:?}")
        };
        assert_eq!((row.name.as_str(), row.depth), ("The Void", 0));
        assert!(
            row.also.ends_with(" void say it once"),
            "found by its words too: {}",
            row.also
        );
        assert_eq!(row.ranked.belongs, "", "it belongs to no orchestration");
        assert_eq!(row.returns, "talk", "Return talks to it, at work or not");
    }
}
