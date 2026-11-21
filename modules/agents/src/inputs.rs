//! Everything Agents' rows are made from, as each source last told it:
//! the node's sessions, its quests and teams, what they landed and the
//! machine's queue, what Joel read of those landings, the to-do daemon's to-dos,
//! the windows daemon's windows, spend-watch's collection and budgets,
//! the harness's list of orchestrators and of sessions to reopen, what
//! Joel said of those, and whether the shell says the screen is shared.
//! Each source changes its own part, and `made` makes every record from
//! all of them.
//!
//! Long because its cases are near half of it: each feeds several sources
//! at once to show how they meet in one record, which only this file can.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use daemon_serve::watch::Held;
use serde_json::Value;

use crate::answered::{self, Answer, Waiting};
use crate::arrangement::{self, Arrangement, Desk};
use crate::carried::Carried;
use crate::carries;
use crate::cells;
use crate::clock;
use crate::closed;
use crate::drawer;
use crate::feedback;
use crate::inboxes;
use crate::kept_places::KeptPlaces;
use crate::landed::{self, Reading};
use crate::network_drawer;
use crate::networks;
use crate::recent;
use crate::reopen::{self, Kept};
use crate::row::{self, Known, Row, Shown, Words};
use crate::served;
use crate::session::{self, JoelCan, Lives, Session};
use crate::shape;
use crate::starts;
use crate::starts_stopped::{self, Stood};
use crate::summary;
use crate::talks::{self, Talk};
use crate::top;
use crate::wants;
use crate::windows;

/// What each source last said.
#[derive(Default)]
pub struct Inputs {
    pub words: Words,
    /// The node's `session` records.
    pub sessions: Held,
    /// The node's `job` records, each job of the machine's queue.
    pub jobs: Held,
    /// The node's `landing` records, what the realm's sessions landed.
    pub landings: Held,
    /// The node's `quest` records, which say how a closed orchestrator
    /// reopens.
    pub quests: Held,
    /// What of those Joel has read.
    pub read: Reading,
    /// The node's `talk` records, Joel's talks with each session.
    pub talks: Held,
    /// The node's `team` records, each orchestration's, with the shape
    /// it's drawn by.
    pub teams: Held,
    /// The node's `network` records, the mycelium's networks as
    /// `mycelium networks` draws them.
    pub networks: Held,
    /// The node's `carry` records, each carry an orchestrator ordered
    /// over the last day, as it stands.
    pub carries: Held,
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
    /// The account's home folder, which a folder to reopen in is said
    /// from.
    pub home: String,
    /// The windows daemon's `workspace` records, which say where each
    /// window's workspace is.
    pub workspaces: Held,
    /// The windows daemon's Screens rows, which say what each screen is.
    pub screens: Held,
    /// Where each session's window sat, as kept.
    pub arrangement: Arrangement,
    /// Where the windows of sessions moving onto pi's host sat, as the
    /// shell said last.
    pub kept_places: KeptPlaces,
    /// What of core's merges the running system carries, which a
    /// landing of core's waits on before To Try holds it.
    pub carried: Carried,
    /// The lines kept of Joel's words on landings sent as talks, which
    /// their landings say how each went of.
    pub fed_back: Vec<Value>,
    /// The talks waiting in each session's inbox, not yet taken, by its
    /// address in capitals (inboxes.rs).
    pub unsent: HashMap<String, Vec<Talk>>,
    /// The wants Joel put off, each for an hour (wants.rs).
    pub put_off: wants::PutOff,
    /// The starts Joel gave Start New Work, kept until the shell says it
    /// told him how each ended, shared with the asks that change them
    /// (starts.rs).
    pub starts: starts::Keeping,
    /// Landed's records as last made, with what they were made from.
    pub landed_made: LandedCache,
    /// How agents' starts stand, as the node last said, or none before it
    /// has (`starts_stopped.rs`).
    pub stood: Option<Stood>,
}

/// Landed's records as last made, which only `landed` reads or keeps.
#[derive(Default)]
pub struct LandedCache(RefCell<Option<LandedMade>>);

/// Landed's records as made, and the keys held, kept while what they
/// were made from reads the same.
struct LandedMade {
    from: LandedFrom,
    records: Rc<Vec<Value>>,
    held: Rc<HashSet<String>>,
}

/// All that Landed's records are made from: how many times each source
/// they read moved, and of what moves with every session, only what a
/// landing says of its session. Making them all again whenever any
/// session's step moved was over a third of the agents daemon's work
/// while it kept 49% of a core busy (perf, 2026-11-16).
#[derive(PartialEq)]
struct LandedFrom {
    landings: u64,
    quests: u64,
    /// Read only while Joel's words were sent on a landing, else none.
    talks: u64,
    /// How many landings were fed back, which stands for them only while
    /// one is never replaced or taken back: anything that does needs a
    /// count of its moves, as the sources above have.
    fed_back: usize,
    read: Reading,
    carried: Carried,
    /// Each running or parked session's address, title, id and landings.
    live: Vec<(String, String, String, Value)>,
    /// The minute it's made in, since To Try holds a landing a day.
    minute: i64,
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

    /// The boot restore.json names, whose places are kept beside this
    /// boot's.
    fn restoring(&self) -> &str {
        self.restore
            .as_ref()
            .and_then(|said| said["boot"].as_str())
            .unwrap_or("")
    }

    /// Keeps where each session's window sits now (arrangement.rs),
    /// writing it to `path` when it moved and saying when it can't. Nothing's kept on a boot
    /// Linux doesn't name, nor before the windows daemon has said a window.
    pub fn keep_places(&mut self, path: &Path) {
        if self.boot.is_empty() || self.windows.records().next().is_none() {
            return;
        }
        let sessions: Vec<Session> = self
            .sessions
            .records()
            .map(|(_, one)| Session(one))
            .collect();
        let paired = windows::records(&sessions, &self.windows, &HashSet::<&str>::new());
        let screens = arrangement::screens(&self.screens);
        let desk = Desk {
            windows: &self.windows,
            workspaces: &self.workspaces,
            screens: &screens,
        };
        let now = arrangement::now(sessions.iter().copied(), &paired, &desk);
        let kept = self.arrangement.kept(
            &self.boot,
            self.restoring(),
            now,
            arrangement::others(&desk),
        );
        if kept == self.arrangement {
            return;
        }
        if let Err(why) = kept.write(path) {
            eprintln!("agents: can't keep {}: {why}", path.display());
        }
        self.arrangement = kept;
    }

    /// Every session there is to reopen, by the sessions running or
    /// parked here now, each worded as its row says it, with Sessions to
    /// Reopen's own record last while any is. One pi's host runs
    /// is back only once its viewer is open, or at once when the boot
    /// being restored kept no place of its window (`arranged`), since it
    /// had none to bring back.
    fn reopens(&self) -> Vec<Value> {
        let arranged = self.arrangement.arranged(self.restoring());
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
                    && (!hosted || viewing.contains(id) || !arranged.contains(id))
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
        let mut offered = reopen::offered(self.restore.as_ref(), &self.boot, &self.kept, &here);
        let there = |folder: &str| std::path::Path::new(folder).is_dir();
        let wording = reopen::Wording {
            words: &self.words,
            home: &self.home,
            there: &there,
        };
        let place = reopen::worded(&mut offered, &wording);
        offered.extend(place);
        offered
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
        let mut served =
            recent::recent(self.sessions.records().map(|(_, one)| Session(one)), &shown);
        let place = recent::place(&served, &self.words.recent);
        served.push(place);
        served
    }

    /// What the sessions landed, each a whole record with its drawer apart
    /// and, where For You holds it, its `try`, with
    /// how many landed each day, and the keys of every landing held
    /// (landed.rs). A session ended is named by its quest's title, as the
    /// node's quest records give it.
    /// Made again only once what they're made from moved (`LandedFrom`),
    /// so the same records are shared until then.
    pub fn landed(&self) -> (Rc<Vec<Value>>, Rc<HashSet<String>>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
        self.landed_at(i64::try_from(now).unwrap_or(0))
    }

    fn landed_at(&self, now: i64) -> (Rc<Vec<Value>>, Rc<HashSet<String>>) {
        let live: Vec<Session> = self
            .sessions
            .records()
            .map(|(_, one)| Session(one))
            .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
            .collect();
        let from = LandedFrom {
            landings: self.landings.taken(),
            quests: self.quests.taken(),
            talks: if self.fed_back.is_empty() {
                0
            } else {
                self.talks.taken()
            },
            fed_back: self.fed_back.len(),
            read: self.read.clone(),
            carried: self.carried.clone(),
            live: live
                .iter()
                .map(|one| {
                    let landings = one
                        .presence()
                        .map_or(Value::Null, |presence| presence["landings"].clone());
                    (
                        one.address().to_owned(),
                        one.named().0,
                        one.id().to_owned(),
                        landings,
                    )
                })
                .collect(),
            minute: now.div_euclid(60_000),
        };
        let mut made = self.landed_made.0.borrow_mut();
        if let Some(made) = made.as_ref().filter(|made| made.from == from) {
            return (Rc::clone(&made.records), Rc::clone(&made.held));
        }
        let (records, held) = self.landed_from(&live, now);
        let (records, held) = (Rc::new(records), Rc::new(held));
        *made = Some(LandedMade {
            from,
            records: Rc::clone(&records),
            held: Rc::clone(&held),
        });
        (records, held)
    }

    fn landed_from(&self, live: &[Session], now: i64) -> (Vec<Value>, HashSet<String>) {
        let (mut records, held) = landed::landed(
            self.landings.records().map(|(_, one)| one),
            live,
            &self.read,
            now,
        );
        landed::waiting_for_switch(&mut records, &self.carried);
        let titles: HashMap<&str, &str> = self
            .quests
            .records()
            .filter_map(|(_, one)| Some((one["id"].as_str()?, one["title"].as_str()?)))
            .collect();
        landed::drawn(&mut records, &self.words, &titles, &self.carried);
        if !self.fed_back.is_empty() {
            let talks: HashMap<String, Talk> = self
                .talks
                .records()
                .map(|(_, one)| talks::folded(one))
                .map(|talk| (talk.id.clone(), talk))
                .collect();
            feedback::told(&mut records, &self.fed_back, &talks, &self.words);
        }
        (records, held)
    }

    /// What sessions running or parked answered of Joel's talks, read or
    /// not, while each talk is the session's newest (answered.rs).
    pub fn answers(&self) -> Vec<Answer> {
        let live = self.live();
        answered::answers(self.talks.records().map(|(_, one)| one), &|address| {
            live.contains(address)
        })
    }

    /// Joel's talks to sessions running or parked still on their way to
    /// an answer, while each is the session's newest (answered.rs).
    fn waiting(&self) -> Vec<Waiting> {
        let live = self.live();
        answered::waiting(self.talks.records().map(|(_, one)| one), &|address| {
            live.contains(address)
        })
    }

    /// The addresses of the sessions running or parked.
    fn live(&self) -> HashSet<&str> {
        self.sessions
            .records()
            .map(|(_, one)| Session(one))
            .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
            .map(Session::address)
            .collect()
    }

    /// Every closed orchestrator's record, the one last live first
    /// (closed.rs), each folder it names looked for as it's made.
    fn closed(&self) -> Vec<Value> {
        let listed = self
            .orchestrators
            .as_ref()
            .and_then(|said| said["orchestrators"].as_array())
            .map_or(&[][..], Vec::as_slice);
        let quests: HashMap<&str, &Value> = self
            .quests
            .records()
            .filter_map(|(_, one)| Some((one["id"].as_str()?, one)))
            .collect();
        let live = closed::live(self.sessions.records().map(|(_, one)| Session(one)));
        let family = |quest: &str| {
            if self.sharing {
                None
            } else {
                self.family(quest)
            }
        };
        let there = |folder: &str| std::path::Path::new(folder).is_dir();
        let known = closed::Known {
            family: &family,
            there: &there,
        };
        closed::closed(listed, &quests, &live, &known)
    }

    /// What the sessions spent today as spend-watch says, or nothing while
    /// the screen is shared.
    fn today_spent(&self) -> &str {
        if self.sharing {
            return "";
        }
        self.spend
            .as_ref()
            .and_then(|spend| spend.get("agentsToday"))
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    /// The boot the sessions to reopen are of, and where the sessions'
    /// windows sat, kept for the shell to put them back by.
    fn placed(&self) -> [Value; 3] {
        [
            reopen::restoring(self.restore.as_ref()),
            self.arrangement.record(),
            self.kept_places.record(),
        ]
    }

    /// Every record served, made from what each source last said.
    pub fn made(&self) -> served::Made {
        let joel_can = JoelCan::from_to_dos(self.to_dos.records().map(|(_, record)| record));
        let jobs: Vec<Value> = self.jobs.records().map(|(_, job)| job.clone()).collect();
        let spent = |id: &str| self.spent(id);
        let family = |quest: &str| self.family(quest);
        let listed = |address: &str| self.listed(address);
        let windowed = |session: Session| session.windowed();
        let to_dos = |quest: &str| self.needs(quest).len();
        let by_lead = shape::by_lead(self.teams.records().map(|(_, team)| team));
        let shaped = |id: &str| by_lead.get(id).copied();
        let known = Known {
            joel_can: &joel_can,
            jobs: &jobs,
            spent: &spent,
            family: &family,
            listed: &listed,
            windowed: &windowed,
            sharing: self.sharing,
            needs: &to_dos,
            shape: &shaped,
        };
        let mut reopens = self.reopens();
        let recent = self.recent(&reopens);
        // With them, the boot they're of, and the places kept.
        reopens.extend(self.placed());
        let used = self.used();
        let last_used = |address: &str| used.get(address).copied().unwrap_or(0);
        let sessions = session::in_order(
            self.sessions.records().map(|(_, one)| Session(one)),
            &last_used,
        );
        let states: Vec<_> = sessions
            .iter()
            .map(|one| one.stands_to_joel(&joel_can, &jobs))
            .collect();
        let mut rows = row::rows(&sessions, &known, &self.words);
        let answers = self.answers();
        let answering: HashSet<&str> = answers
            .iter()
            .filter(|one| !one.read)
            .map(|one| one.address.as_str())
            .collect();
        let waiting = self.waiting();
        let answered = self.words.agents.word("answered");
        each_row(&mut rows, &mut |row| {
            let address = row.key.trim_start_matches("agent:");
            if answering.contains(address) {
                row.says_first(answered);
            } else if let Some(one) = waiting.iter().find(|one| one.address == address) {
                let said = self.words.talk_stages.word(&one.stage);
                row.waits(said, &one.talk, &one.stage);
            }
        });
        let mut orchestrating = HashSet::new();
        orchestrators(&rows, &mut orchestrating);
        let mut drawers = self.drawers(&sessions, &orchestrating, (&answers, &waiting));
        let closed = self.closed();
        let top = self.top(&sessions, &rows, &closed);
        drawers.extend(self.network_drawers(&sessions, &top));
        drawers.extend(starts_stopped::drawers(self.stood.as_ref(), &self.words));
        served::made(
            &rows,
            &summary::summary(
                &states,
                answering.len(),
                self.today_spent(),
                &self.words.agents,
            ),
        )
        .with_top(top)
        .with_drawers(drawers)
        .with_recent(recent)
        .with_reopens(reopens)
        .with_landed(self.landed().0)
        .with_closed(closed)
        .with_windows(windows::records(&sessions, &self.windows, &orchestrating))
        .with_cells(cells::records(&sessions))
        .with_wants(wants::records(
            &sessions,
            &joel_can,
            (&self.put_off, clock::now_ms()),
        ))
        .with_talks(inboxes::records(
            &talks::by_address(self.talks.records().map(|(_, one)| one)),
            &self.unsent,
        ))
        .with_starts(self.starts.records())
    }

    /// Each session's drawer, the facts of a talk it answered, or of one
    /// on its way, first, and the top level's drawers made of them.
    fn drawers(
        &self,
        sessions: &[Session],
        orchestrating: &HashSet<&str>,
        (answers, waiting): (&[Answer], &[Waiting]),
    ) -> Vec<Value> {
        let needs = |quest: &str| self.needs(quest);
        let today = clock::today();
        let drawn = drawer::Known {
            needs: &needs,
            spend: self.spend.as_ref(),
            budgets: self.budgets.as_ref(),
            sharing: self.sharing,
            today: &today,
        };
        let called = |name: &str| session::called(sessions, name);
        let mut drawers: Vec<Value> = sessions
            .iter()
            .map(|one| {
                let orchestrates = orchestrating.contains(one.address());
                let mut drawn = drawer::drawer(*one, orchestrates, &self.words, &drawn);
                let words = &self.words;
                let first = if let Some(answer) =
                    answers.iter().find(|said| said.address == one.address())
                {
                    answered::facts(answer, &words.names, &called)
                } else if let Some(waits) =
                    waiting.iter().find(|said| said.address == one.address())
                {
                    let (parts, stages) = (&words.talk_parts, &words.talk_stages);
                    answered::waiting_facts(waits, &words.names, parts, stages)
                } else {
                    Vec::new()
                };
                if let Some(facts) = drawn["drawer"]["facts"].as_array_mut() {
                    facts.splice(0..0, first);
                }
                drawn
            })
            .collect();
        drawers.extend(top::drawers(&drawers));
        drawers
    }

    /// Each network's drawer, what its sessions spent today added up from
    /// what spend-watch serves of each as figures, none while the screen
    /// is shared.
    fn network_drawers(&self, sessions: &[Session], top: &[Value]) -> Vec<Value> {
        let ids: HashMap<String, &str> = sessions
            .iter()
            .map(|one| (one.address().to_lowercase(), one.id()))
            .collect();
        let figures = self
            .spend
            .as_ref()
            .and_then(|spend| spend.get("sessionsSpent"))
            .filter(|_| !self.sharing);
        let spent = |address: &str| {
            let figure = figures?.get(*ids.get(address)?)?;
            Some((
                figure.get("dollars")?.as_f64()?,
                figure.get("estimated")?.as_bool()?,
            ))
        };
        let networks: Vec<&Value> = self.networks.records().map(|(_, one)| one).collect();
        network_drawer::drawers(&networks, top, &self.words, &spent)
    }

    /// Agents' top level made of the `rows` laid out, with what each
    /// orchestration spent in all, the `closed` orchestrators, and Joel's
    /// talks with each session.
    fn top(&self, sessions: &[Session<'_>], rows: &[Shown], closed: &[Value]) -> Vec<Value> {
        let quests: HashMap<&str, &str> = sessions
            .iter()
            .map(|one| (one.address(), one.quest()))
            .collect();
        let all_spent = |address: &str| {
            let quest = quests
                .get(address)
                .copied()
                .filter(|quest| !quest.is_empty())?;
            if self.sharing {
                None
            } else {
                self.family(quest)
            }
        };
        let talks = talks::by_address(self.talks.records().map(|(_, one)| one));
        let known = top::Known {
            spent: &all_spent,
            closed,
            talks: &talks,
        };
        // Stopped starts first, above every orchestration, so a stop is
        // never silent.
        let mut made = starts_stopped::rows(self.stood.as_ref(), &self.words);
        made.extend(top::rows(rows, &known, &self.words));
        let networks: Vec<&Value> = self.networks.records().map(|(_, one)| one).collect();
        let after = networks::rows(&networks, &made, &self.words);
        let kept: Vec<&Value> = self.carries.records().map(|(_, one)| one).collect();
        let carried = carries::rows(&kept, &made, &self.words);
        made.extend(after);
        made.extend(carried);
        made
    }
}

/// Does `to` each session's row, folded or not, as how Joel's talks to
/// them stand is said on them.
fn each_row(shown: &mut [Shown], to: &mut dyn FnMut(&mut Row)) {
    for one in shown {
        match one {
            Shown::Session(row) => to(row),
            Shown::Fold(fold) => each_row(&mut fold.held, to),
        }
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
    use std::rc::Rc;

    use serde_json::json;

    use super::*;

    fn inputs() -> Inputs {
        let mut inputs = Inputs {
            words: Words::from_vocabulary(crate::said::LAUNCHERS)
                .expect("the launcher's words read"),
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

    /// How many landings Landed's records hold.
    fn landings_in(records: &[Value]) -> usize {
        records.iter().filter(|one| one["kind"] == "landed").count()
    }

    #[test]
    fn landed_is_made_again_only_once_what_it_reads_moved() {
        let mut inputs = inputs();
        let landing = |reference: &str| {
            json!({"key": format!("landing:s-p:{reference}"), "session": "s-p",
                "ref": reference, "at": "2026-11-11T01:00:00Z", "what": "One"})
        };
        inputs.landings.took(&json!({"records": [landing("o#1")]}));
        let now = 1_763_000_000_000;
        let (first, _) = inputs.landed_at(now);
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:p", "step": "reading",
            "usage": {"percent": 40}}]}));
        inputs
            .teams
            .took(&json!({"records": [{"key": "team:O", "lead": "s-o"}]}));
        let (again, _) = inputs.landed_at(now + 1_000);
        assert!(
            Rc::ptr_eq(&first, &again),
            "a session's step and a team moved nothing a landing reads, yet it was made again"
        );
        inputs.landings.took(&json!({"changed": [landing("o#2")]}));
        let (second, _) = inputs.landed_at(now);
        assert_eq!(landings_in(&second), 2, "a new landing is made");
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:p",
            "presence": {"addresses": ["P"], "session": "s-p", "busy": true,
                         "orchestratedBy": "O", "questInfo": {"title": "Mend the reef"}}}]}));
        let (renamed, _) = inputs.landed_at(now);
        assert!(
            !Rc::ptr_eq(&second, &renamed),
            "a session named anew names its landings anew, yet they were kept"
        );
        let (later, _) = inputs.landed_at(now + 60_000);
        assert!(
            !Rc::ptr_eq(&renamed, &later),
            "a minute on, To Try's day is weighed again, yet they were kept"
        );
    }

    #[test]
    fn landed_is_made_again_once_any_input_it_reads_moves() {
        let mut inputs = inputs();
        inputs.landings.took(&json!({"records": [
            {"key": "landing:s-p:o#1", "session": "s-p", "ref": "o#1",
             "at": "2026-11-11T01:00:00Z", "what": "One"}]}));
        let now = 1_763_000_000_000;
        let mut was = inputs.landed_at(now).0;
        let remade = |was: &mut Rc<Vec<Value>>, inputs: &Inputs, what: &str| {
            let made = inputs.landed_at(now).0;
            assert!(
                !Rc::ptr_eq(was, &made),
                "{what} moved, yet the landings were kept"
            );
            *was = made;
        };
        inputs
            .quests
            .took(&json!({"changed": [{"key": "quest:QEST-O", "title": "Kelp"}]}));
        remade(&mut was, &inputs, "a quest");
        inputs
            .read
            .at
            .insert("s-p o#1 2026-11-11T01:00:00Z".to_owned(), Some(now));
        remade(&mut was, &inputs, "what Joel read");
        assert_eq!(
            was.iter()
                .find(|one| one["kind"] == "landed")
                .map(|one| &one["landed"]["read"]),
            Some(&json!(now)),
            "made again, the landing says when Joel read it"
        );
        inputs.carried = Carried::of(&[1]);
        remade(&mut was, &inputs, "what the system carries");
        // Each step says the whole presence, which a change replaces, so
        // only the one field it names moves; titled by its quest, the
        // session's title doesn't follow its address.
        let present = |address: &str, id: &str, landings: Value| {
            json!({"changed": [{"key": "session:p", "presence": {
                "addresses": [address], "session": id, "busy": true,
                "orchestratedBy": "O", "questInfo": {"title": "Kelp"},
                "landings": landings}}]})
        };
        // Null reads as the landings the presence named none of before.
        inputs.sessions.took(&present("P", "s-p", Value::Null));
        remade(&mut was, &inputs, "a live session's title");
        inputs
            .sessions
            .took(&present("P", "s-p", json!([{"ref": "o#3"}])));
        remade(&mut was, &inputs, "a live session's landings");
        inputs
            .sessions
            .took(&present("P2", "s-p", json!([{"ref": "o#3"}])));
        remade(&mut was, &inputs, "a live session's address");
        inputs
            .sessions
            .took(&present("P2", "s-p2", json!([{"ref": "o#3"}])));
        remade(&mut was, &inputs, "a live session's id");
        inputs
            .talks
            .took(&json!({"changed": [{"key": "talk:t-2", "id": "t-2", "stage": "sent"}]}));
        assert!(
            Rc::ptr_eq(&was, &inputs.landed_at(now).0),
            "with nothing fed back no landing reads a talk, yet one moving made them again"
        );
        let fed =
            |sent: &str| json!({"by": "talk", "landing": "landing:s-p:o#1", "sent": sent, "at": 1});
        inputs.fed_back.push(fed("t-1"));
        remade(&mut was, &inputs, "the first of Joel's words fed back");
        // Talks have moved by now, so only how many were fed back differs.
        inputs.fed_back.push(fed("t-2"));
        remade(&mut was, &inputs, "more of Joel's words fed back");
        inputs
            .talks
            .took(&json!({"changed": [{"key": "talk:t-1", "id": "t-1", "stage": "read"}]}));
        remade(&mut was, &inputs, "a talk, with something fed back");
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

    #[test]
    fn each_session_here_is_served_with_its_window() {
        let mut inputs = inputs();
        inputs.sessions.took(&json!({"records": [
            {"key": "session:o", "standing": "running", "here": true,
             "window": {"terminal": "kitty", "within": "unix:@kitty-4", "id": 2,
                "app_id": "pi-session.1"},
             "presence": {"addresses": ["O"]}}
        ]}));
        inputs.windows.took(&json!({"records": [
            {"key": "window:7", "id": 7, "pid": 4, "opened": "pi-session.1"}
        ]}));
        let served = served(&inputs);
        let record = served.get("agent-window:agents:O").expect("its record");
        assert_eq!(
            (&record["window"], &record["kitty"]["id"]),
            (&json!("window:7"), &json!("2")),
            "the window opened with its app id, in its kitty's pane"
        );
    }

    #[test]
    fn an_orchestrators_rows_wear_its_teams_shape_once_the_node_says_it() {
        let mut inputs = inputs();
        let mut shown = served(&inputs);
        inputs.teams.took(&json!({"records": [
            {"key": "team:O", "lead": "s-o", "glyph": "hexagon"},
            {"key": "team:P", "lead": "s-p", "glyph": "circle"}
        ]}));
        let moved = shown.take(inputs.made());
        let keys: Vec<_> = moved.changed.iter().map(|one| one["key"].clone()).collect();
        assert_eq!(
            keys,
            [json!("row:agents-top:O"), json!("row:agents:O")],
            "the orchestrator's rows move, and no one else's"
        );
        let glyph = |key: &str| shown.get(key).expect("its row")["row"]["glyph"].clone();
        assert_eq!(
            [
                glyph("row:agents-top:O"),
                glyph("row:agents:O"),
                glyph("row:agents:P")
            ],
            [json!("shapeHexagon"), json!("shapeHexagon"), json!("agent")],
            "its team's shape on both its rows; one orchestrating no one keeps the agent's mark"
        );
    }

    #[test]
    fn what_a_session_asks_joel_is_served_as_its_want() {
        let mut inputs = inputs();
        inputs.sessions.took(&json!({"records": [
            {"key": "session:o", "standing": "parked",
             "presence": {"addresses": ["O"], "wants": {"kind": "question",
                "said": "which tide?", "since": "2026-11-13T05:00:00Z"}}}
        ]}));
        let served = served(&inputs);
        let record = served.get("want:agents:O").expect("its want");
        assert_eq!(
            (&record["asked"], &record["parked"]),
            (&json!("which tide?"), &json!(true)),
            "what it asked, and that it's parked waiting"
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
    fn each_session_holds_how_it_stands_apart_from_its_figures() {
        let standing = served(&inputs());
        let doing = |address: &str| {
            standing
                .get(&format!("row:agents:{address}"))
                .expect("its row")["row"]["held"]["doing"]
                .clone()
        };
        assert_eq!(
            (doing("O"), doing("P")),
            (json!("listening"), json!("working")),
            "For You's heading over a cell says this, never its spend"
        );
        let mut inputs = inputs();
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:p",
            "presence": {"addresses": ["P"], "session": "s-p", "orchestratedBy": "O",
                "wants": {"kind": "approval"}}}]}));
        let needing = served(&inputs);
        assert_eq!(
            needing.get("row:agents:P").expect("its row")["row"]["held"]["doing"],
            json!("needs your yes"),
            "one waiting on Joel says what it wants of him, not its standing's id"
        );
    }

    #[test]
    fn an_answer_is_said_until_read_and_its_drawer_keeps_it_as_joel_reads_it() {
        let mut inputs = inputs();
        let lines = |read: bool| {
            let mut lines = vec![
                json!({"stage": "delivered", "from": "joel", "text": "still lit?"}),
                json!({"stage": "replied", "at": "2026-11-11T21:01:00Z", "text": "Yes."}),
            ];
            if read {
                lines.push(json!({"stage": "read", "by": "joel"}));
            }
            json!({"records": [{"key": "talk:P/talk-1", "address": "P", "id": "talk-1",
                "lines": lines}]})
        };
        inputs.talks.took(&lines(false));
        let served = served(&inputs);
        assert_eq!(summary_of(&served), "1 answer · 1 working · $214 today");
        assert_eq!(side(&served, "P"), "answered · working · $4");
        let facts = |served: &served::Served| {
            let drawer = &served.get("drawer:agents:P").expect("P's drawer")["drawer"];
            let facts = drawer["facts"].as_array().expect("its facts");
            facts
                .iter()
                .take(2)
                .map(|fact| fact["value"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(facts(&served), [json!("still lit?"), json!("Yes.")]);
        inputs.talks.took(&lines(true));
        let served = served_of(&inputs);
        assert_eq!(
            summary_of(&served),
            "1 working · $214 today",
            "read, it's not counted"
        );
        assert_eq!(side(&served, "P"), "working · $4", "nor said on its row");
        assert_eq!(
            facts(&served),
            [json!("still lit?"), json!("Yes.")],
            "but its drawer keeps it"
        );
    }

    #[test]
    fn a_talk_on_its_way_is_said_on_its_row_and_its_drawer_uncounted() {
        let mut inputs = inputs();
        inputs
            .talks
            .took(&json!({"records": [{"key": "talk:P/talk-1", "address": "P",
            "id": "talk-1", "lines": [
                {"stage": "delivered", "from": "joel", "at": "2026-11-11T21:00:00Z",
                    "text": "still lit?"},
                {"stage": "replying", "text": "Ye"}
            ]}]}));
        let served = served(&inputs);
        assert_eq!(
            summary_of(&served),
            "1 working · $214 today",
            "nothing waits on Joel"
        );
        assert_eq!(side(&served, "P"), "replying · working · $4");
        let talk = json!({"id": "talk-1", "stage": "replying"});
        for key in ["row:agents:P", "row:agents-top:P"] {
            let record = served.get(key).expect("its row");
            assert_eq!(record["row"]["talk"], talk, "{key} holds which talk waits");
        }
        let drawer = &served.get("drawer:agents:P").expect("P's drawer")["drawer"];
        let first: Vec<_> = drawer["facts"]
            .as_array()
            .expect("its facts")
            .iter()
            .take(2)
            .map(|fact| (fact["label"].clone(), fact["value"].clone()))
            .collect();
        assert_eq!(
            first,
            [
                (json!("You Asked"), json!("still lit?")),
                (json!("How It Went"), json!("replying")),
            ],
            "its drawer leads with what he asked and how it went"
        );
        assert!(
            served.get("row:agents:O").expect("O's row")["row"]
                .get("talk")
                .is_none(),
            "one with no talk waiting holds none"
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
        assert_eq!(
            (
                served
                    .get("reopen:agents:s-gone")
                    .map(|one| one["row"]["side"].clone()),
                served
                    .get(reopen::TO_REOPEN)
                    .map(|one| one["toReopen"]["side"].clone()),
            ),
            (Some(json!("/tmp")), Some(json!("1 session"))),
            "each is worded as its row, and Sessions to Reopen counts them"
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
        inputs.arrangement =
            Arrangement::of(&json!({"v": 1, "boots": {"the-last": {"s-viewed": {}}}}));
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
            .filter_map(|record| record["recent"].get("id").cloned())
            .collect();
        assert_eq!(
            listed,
            [json!("x")],
            "X closed; Y is to reopen, Z a closed orchestrator"
        );
        assert_eq!(
            served
                .get(recent::RECENT)
                .map(|one| one["place"]["side"].clone()),
            Some(json!("1")),
            "and Recent's place counts it"
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
            [
                json!("row:agents-top:O"),
                json!("row:agents-top:O:close"),
                json!("row:agents:O"),
                json!(served::SUMMARY),
                json!("want:agents:O")
            ],
            "an orchestrator's own question is Joel's, and moves its rows, its Close, which \
             costs now that a turn waits on him, the summary and its want"
        );
        assert_eq!(
            summary_of(&shown),
            "1 needs you \u{b7} 1 working \u{b7} $214 today"
        );
    }

    #[test]
    fn a_session_parked_with_joels_want_open_counts_him_needed_in_the_summary() {
        let mut inputs = inputs();
        inputs
            .sessions
            .took(&json!({"changed": [{"key": "session:o",
            "presence": {"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"],
                "state": "parked", "wants": {"kind": "question"}}}]}));
        let summary = summary_of(&served(&inputs));
        let said = summary.as_str().unwrap_or_default();
        assert!(
            said.starts_with("1 needs you"),
            "parked, its open question is still Joel's: {said}"
        );
    }

    #[test]
    fn a_carry_the_node_kept_is_served_under_carried_named_as_its_session() {
        let mut inputs = inputs();
        assert!(
            served(&inputs).get("row:agents-top:carried").is_none(),
            "no Carried row while the node keeps no carry"
        );
        inputs.carries.took(&json!({"records": [
            {"key": "carry:s-p:2026-11-16T01:00:00Z", "kind": "carry",
             "at": "2026-11-16T01:00:00Z", "by": "O", "session": "s-p", "address": "P",
             "quest": "QEST-P", "from": "skiff", "to": "tern", "why": "it needs the camera",
             "state": "carrying"}]}));
        let shown = served(&inputs);
        let row = |key: &str| shown.get(key).expect("its row")["row"].clone();
        let carry = row("row:agents-top:carried:s-p:2026-11-16T01:00:00Z");
        assert_eq!(carry["parent"], "row:agents-top:carried");
        assert_eq!(
            carry["name"],
            row("row:agents-top:P")["name"],
            "named as its session's row is"
        );
        assert_eq!(row("row:agents-top:carried")["name"], "Carried");
    }

    #[test]
    fn starts_stopped_stands_first_in_agents_top_level_while_the_node_says_so() {
        let mut inputs = inputs();
        // The first of Agents' top-level rows, by the order they're served in.
        let first = |inputs: &Inputs| {
            let moved = served::Served::default().take(inputs.made());
            moved
                .changed
                .iter()
                .filter(|one| {
                    one["key"]
                        .as_str()
                        .is_some_and(|key| key.starts_with("row:agents-top:"))
                        && one["row"]["parent"].is_null()
                })
                .min_by_key(|one| one["row"]["order"].as_i64())
                .map(|one| one["key"].clone())
        };
        assert_eq!(
            first(&inputs),
            Some(json!("row:agents-top:O")),
            "no Starts Stopped before the node has said"
        );
        inputs.stood = Some(Stood {
            stopped: true,
            newest: Vec::new(),
        });
        assert_eq!(
            first(&inputs),
            Some(json!("row:agents-top:starts-stopped")),
            "Starts Stopped first, above every orchestration"
        );
        assert!(
            served(&inputs)
                .get("drawer:agents-top:starts-stopped")
                .is_some(),
            "its drawer served with it"
        );
        inputs.stood = Some(Stood::default());
        assert_eq!(
            first(&inputs),
            Some(json!("row:agents-top:O")),
            "and gone once starts are let start"
        );
    }

    #[test]
    fn a_networks_drawer_adds_up_what_spend_watch_says_its_sessions_spent() {
        let mut inputs = inputs();
        inputs.spend.as_mut().expect("its spend")["sessionsSpent"] = json!({
            "s-o": {"dollars": 5.0, "estimated": false},
            "s-p": {"dollars": 4.0, "estimated": false}});
        inputs.networks.took(&json!({"records": [
            {"key": "network:repo.kelp", "kind": "network", "name": "repo.kelp",
             "shape": "repository", "short": "kelp", "title": "kelp", "heard": 0,
             "cells": ["o", "p"], "machines": [], "counts": {}}]}));
        let spent = |inputs: &Inputs| {
            served(inputs)
                .get("drawer:agents-top:network:repo.kelp")
                .expect("its drawer")["drawer"]["facts"]
                .as_array()
                .expect("its facts")
                .iter()
                .find(|fact| fact["name"] == "Spent Today")
                .map(|fact| fact["said"].clone())
        };
        assert_eq!(
            spent(&inputs),
            Some(json!(ready_rows::money::dollars(9.0))),
            "each of its sessions' figures, added up"
        );
        inputs.sharing = true;
        assert_eq!(spent(&inputs), None, "no amount while the screen is shared");
    }

    #[test]
    fn a_talk_waiting_in_an_inbox_is_served_with_its_sessions_talks_and_moves_them_alone() {
        let mut inputs = inputs();
        let mut shown = served(&inputs);
        let sent = Talk {
            id: "talk-9".to_owned(),
            at: "2026-11-14T09:00:00.000Z".to_owned(),
            words: "too dim".to_owned(),
            stage: "sent".to_owned(),
            ..Talk::default()
        };
        inputs.unsent = [("P".to_owned(), vec![sent])].into();
        let moved = shown.take(inputs.made());
        assert_eq!(
            moved.changed,
            vec![
                json!({"key": "talks:agents:P", "kind": "talks", "address": "P", "talks": [
                {"id": "talk-9", "at": "2026-11-14T09:00:00.000Z", "words": "too dim",
                 "stage": "sent", "after": "", "reply": "", "why": "", "came": ""}]})
            ],
            "its session's talks, served whole, and nothing else moves"
        );
    }
}
