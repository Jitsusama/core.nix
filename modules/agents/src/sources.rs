//! Each source followed on a thread of its own, its answers handed to the
//! daemon's one loop, which holds every record (inputs.rs): the node's
//! watches, the to-do daemon's, the files heard as they land, and what
//! the shell says over the daemon's own socket. A daemon's watch is
//! followed as every daemon of the shell follows one (daemon-serve's
//! `watch`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

pub use daemon_serve::watch::{follow, follow_asking};
use serde_json::Value;

use crate::answered;
use crate::arrangement::{self, Arrangement, Still};
use crate::carried::{Carried, Said};
use crate::clock;
use crate::file::{self, Files};
use crate::inboxes::{self, Inboxes};
use crate::inputs::Inputs;
use crate::kept_places::KeptPlaces;
use crate::starts_stopped::Stood;
use crate::talks::Talk;
use crate::wants;

/// Something a source said.
#[derive(Debug)]
pub enum Heard {
    /// An answer to the node's watch of its sessions.
    Sessions(Value),
    /// An answer to the node's watch of the machine's queue's jobs.
    Jobs(Value),
    /// An answer to the node's watch of what the realm's sessions landed.
    Landings(Value),
    /// An answer to the node's watch of the realm's quests.
    Quests(Value),
    /// An answer to the node's watch of the talks Joel had with sessions.
    Talks(Value),
    /// An answer to the node's watch of its teams, each orchestration's.
    Teams(Value),
    /// An answer to the node's watch of the mycelium's networks.
    Networks(Value),
    /// An answer to the node's watch of the carries it ordered.
    Carries(Value),
    /// An answer to the to-do daemon's watch of its to-dos.
    ToDos(Value),
    /// An answer to the windows daemon's watch of its windows.
    Windows(Value),
    /// An answer to the windows daemon's watch of its workspaces.
    Workspaces(Value),
    /// An answer to the windows daemon's watch of its Screens rows.
    Screens(Value),
    /// Files that landed or went.
    Files(Vec<PathBuf>),
    /// Whether the shell says the screen is shared.
    Sharing(bool),
    /// Joel puts off the last boot's sessions, as the shell says.
    PutOff,
    /// Joel forgets one of the sessions to reopen, by pi's id for it.
    Forget(String),
    /// Joel reads these landings, by their keys, at this moment in
    /// milliseconds, as the shell says.
    Read(Vec<String>, i64),
    /// Joel opened a session's drawer, by its address, at this moment in
    /// milliseconds, as the shell says, so he's read what it answered.
    Opened(String, i64),
    /// Where the windows of sessions moving onto pi's host sat, all of
    /// them, as the shell says each time one's kept or let go.
    KeepPlaces(KeptPlaces),
    /// The line kept of Joel's words on a landing sent as a talk, which
    /// the landing says how it went of (feedback.rs).
    FedBack(Value),
    /// The talks waiting in each session's inbox, all of them, as one
    /// lands or is taken (inboxes.rs).
    Inboxes(HashMap<String, Vec<Talk>>),
    /// Joel puts off the want known so (wants.rs's `known`), at this
    /// moment in milliseconds, for an hour.
    PutOffWant(String, u128),
    /// The starts kept moved, one asked, ended or told, each kept on disk
    /// before its ask was answered (starts.rs's Keeping), so their
    /// records are made again.
    StartsMoved,
    /// How agents' starts stand, as the node says (`starts_stopped.rs`).
    Starts(Stood),
}

/// Where the files the rows are made from are kept.
#[derive(Clone, Debug)]
pub struct Paths {
    /// Spend-watch's collection.
    pub spend: PathBuf,
    /// Spend-watch's budgets, beside its collection.
    pub budgets: PathBuf,
    /// The harness's list of orchestrators.
    pub orchestrators: PathBuf,
    /// The harness's list of sessions to reopen.
    pub restore: PathBuf,
    /// What Joel said of those, which the daemon alone writes, so it's
    /// read at the start rather than followed.
    pub kept: PathBuf,
    /// What of the landings Joel has read, the daemon's alone too.
    pub landed: PathBuf,
    /// The harness's steer folders, in which each session's inbox keeps
    /// its talks, where the daemon says Joel read an answer.
    pub steer: PathBuf,
    /// The shell's own folders, where the launcher kept what Joel read
    /// before the daemon did, read once while `landed` isn't there, and
    /// where its sessions' windows sat, read once while `arrangement`
    /// isn't there.
    pub shells: PathBuf,
    /// Where each session's window sat, which the daemon writes and
    /// follows, so a hand that writes it is heard.
    pub arrangement: PathBuf,
    /// Where the windows of sessions moving onto pi's host sat, which the
    /// daemon writes and follows too.
    pub kept_places: PathBuf,
    /// update-to-main's answer to its last ask, which says the core it
    /// followed, read again at each switch; none where it isn't given,
    /// when no landing waits on one.
    pub switched: Option<PathBuf>,
    /// The last done ask's core, which the daemon keeps, since a boot
    /// takes the answer away.
    pub carried: PathBuf,
    /// A clone of core's whose history says what that core merged, read
    /// again at each of its fetches.
    pub clone: Option<PathBuf>,
    /// Where pi-packages installs the harness, whose `current` link says
    /// the commit installed, read again as it moves.
    pub harness: Option<PathBuf>,
    /// The integration's record of switches, which says when each core
    /// came, read again as a switch adds to it; none where it isn't
    /// given, when no landing says since when it runs.
    pub switches: Option<PathBuf>,
}

impl Heard {
    /// Whether it can move where a session's window sits, which is kept
    /// once they've been still a moment.
    pub fn moves_places(&self) -> bool {
        matches!(
            self,
            Self::Sessions(_) | Self::Windows(_) | Self::Workspaces(_) | Self::Screens(_)
        )
    }
}

impl Paths {
    /// The files followed.
    pub fn all(&self) -> Vec<PathBuf> {
        vec![
            self.spend.clone(),
            self.budgets.clone(),
            self.orchestrators.clone(),
            self.restore.clone(),
            self.arrangement.clone(),
            self.kept_places.clone(),
        ]
        .into_iter()
        .chain(self.switched.clone())
        .chain(self.clone.as_ref().map(|clone| clone.join("FETCH_HEAD")))
        .chain(self.harness.as_ref().map(|harness| harness.join("current")))
        .chain(self.switches.clone())
        .collect()
    }

    /// Whether `path` says what of core or the harness runs, or what that
    /// holds.
    fn says_carried(&self, path: &Path) -> bool {
        self.switched.as_deref() == Some(path)
            || self.switches.as_deref() == Some(path)
            || self
                .clone
                .as_ref()
                .is_some_and(|clone| clone.join("FETCH_HEAD") == path)
            || self
                .harness
                .as_ref()
                .is_some_and(|harness| harness.join("current") == path)
    }
}

/// Follows the talks waiting in each session's inbox under `steer`,
/// handing them all to `to` at the start and again as one lands or is
/// taken.
pub fn follow_inboxes(steer: &Path, to: Sender<Heard>) -> std::io::Result<()> {
    let mut watched = Inboxes::open(steer)?;
    let root = steer.to_owned();
    let _ = to.send(Heard::Inboxes(inboxes::waiting(&root)));
    thread::spawn(move || {
        while watched.heard().is_ok() {
            if to.send(Heard::Inboxes(inboxes::waiting(&root))).is_err() {
                return;
            }
        }
    });
    Ok(())
}

/// Follows the files the rows are made from, handing each that lands to
/// `to`, after handing them all once, as they stand at the start.
pub fn follow_files(paths: &Paths, to: Sender<Heard>) -> std::io::Result<()> {
    let mut files = Files::open(&paths.all())?;
    let _ = to.send(Heard::Files(paths.all()));
    thread::spawn(move || {
        while let Ok(landed) = files.heard() {
            if to.send(Heard::Files(landed)).is_err() {
                return;
            }
        }
    });
    Ok(())
}

impl Inputs {
    /// Keeps both places files once the windows have been `still` long
    /// enough at `now`, and asks `still` to be woken for what's to come
    /// though nothing moves: a place closed by hand let go when due, and
    /// kept places whose write failed, at the shell's word or here, tried
    /// again soon though the shell says nothing more.
    pub fn keep_when_still(&mut self, still: &mut Still, paths: &Paths, now: Instant) {
        if still.due(now) {
            self.keep_kept_places(&paths.kept_places);
            if let Some(let_go) = self.keep_places(&paths.arrangement) {
                let ms = u64::try_from(clock::now_ms()).unwrap_or(u64::MAX);
                still.again(now + arrangement::again_in(let_go, ms));
            }
        }
        let ms = u64::try_from(clock::now_ms()).unwrap_or(u64::MAX);
        if let Some(again) = self.kept_places_again_at(ms) {
            still.again(now + arrangement::again_in(again, ms));
        }
    }

    /// Takes what a source said into the part of the inputs it changes.
    pub fn hear(&mut self, heard: Heard, paths: &Paths) {
        match heard {
            Heard::Sessions(answer) => {
                self.sessions.took(&answer);
            }
            Heard::Jobs(answer) => {
                self.jobs.took(&answer);
            }
            Heard::Landings(answer) => {
                self.landings.took(&answer);
            }
            Heard::Quests(answer) => {
                self.quests.took(&answer);
            }
            Heard::Talks(answer) => {
                self.talks.took(&answer);
            }
            Heard::Teams(answer) => {
                self.teams.took(&answer);
            }
            Heard::Networks(answer) => {
                self.networks.took(&answer);
            }
            Heard::Carries(answer) => {
                self.carries.took(&answer);
            }
            Heard::ToDos(answer) => {
                self.to_dos.took(&answer);
            }
            Heard::Windows(answer) => {
                self.windows.took(&answer);
            }
            Heard::Workspaces(answer) => {
                self.workspaces.took(&answer);
            }
            Heard::Screens(answer) => {
                self.screens.took(&answer);
            }
            Heard::Files(landed) => {
                for path in landed {
                    self.read(&path, paths);
                }
            }
            Heard::Sharing(sharing) => self.sharing = sharing,
            Heard::PutOff => {
                self.kept.put_off = self.restore.as_ref().map_or(String::new(), |said| {
                    said["boot"].as_str().unwrap_or("").to_owned()
                });
                self.keep(paths);
            }
            Heard::Forget(id) => {
                self.kept.forget(&id, self.restore.as_ref());
                self.keep(paths);
            }
            Heard::Read(keys, now) => {
                let (_, held) = self.landed();
                self.read.mark(&keys, now, &held);
                if let Err(why) = self.read.write(&paths.landed) {
                    eprintln!("agents: can't keep {}: {why}", paths.landed.display());
                }
            }
            // The node tells the read line back with the rest of the talk,
            // which takes the answer away, so nothing is held here.
            Heard::Opened(address, now) => {
                let answers = self.answers();
                if let Some(answer) = answers
                    .iter()
                    .find(|one| one.address == address && !one.read)
                    && let Err(why) = answered::read(&paths.steer, answer, now)
                {
                    eprintln!("agents: can't say {} was read: {why}", answer.talk);
                }
            }
            Heard::KeepPlaces(kept) => self.write_kept_places(kept, &paths.kept_places),
            Heard::FedBack(line) => self.fed_back.push(line),
            Heard::Inboxes(waiting) => self.unsent = waiting,
            Heard::Starts(stood) => self.stood = Some(stood),
            Heard::PutOffWant(known, now) => {
                self.put_off.put(&known, now);
                let path = paths.kept.with_file_name(wants::PUT_OFF_FILE);
                if let Err(why) = self.put_off.write(&path) {
                    eprintln!("agents: can't keep {}: {why}", path.display());
                }
            }
            Heard::StartsMoved => {}
        }
    }

    /// Writes what Joel said, saying when it can't, as the next start
    /// would offer again what he put off or forgot.
    fn keep(&self, paths: &Paths) {
        if let Err(why) = self.kept.write(&paths.kept) {
            eprintln!("agents: can't keep {}: {why}", paths.kept.display());
        }
    }

    fn read(&mut self, path: &Path, paths: &Paths) {
        if path == paths.spend {
            self.spend = file::read(path);
        } else if path == paths.budgets {
            self.budgets = file::read(path);
        } else if path == paths.orchestrators {
            self.orchestrators = file::read(path);
        } else if path == paths.restore {
            self.restore = file::read(path);
        } else if path == paths.arrangement {
            // Read again whenever it's written, by the daemon or another
            // hand, as the shell read its own file of them. While a write
            // has failed, memory is ahead of the file and is written over
            // it at the next try, so the file isn't read back over it: a
            // hand edit made meanwhile is ignored, then written over.
            if self.unwritten.is_some() {
                return;
            }
            if let Some(kept) = Arrangement::read(path) {
                self.arrangement = kept;
            }
        } else if path == paths.kept_places {
            // As with the arrangement: while their write has failed,
            // memory holds what the shell said last and the file doesn't,
            // until a try soon after writes it (keep_kept_places).
            if self.kept_places_unwritten.is_some() {
                return;
            }
            if let Some(kept) = KeptPlaces::read(path) {
                self.kept_places = kept;
            }
        } else if paths.says_carried(path) {
            self.carried = Carried::read(Said {
                switched: paths.switched.as_deref(),
                kept: &paths.carried,
                clone: paths.clone.as_deref(),
                harness: paths.harness.as_deref(),
                switches: paths.switches.as_deref(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::reopen::Kept;

    fn paths_in(folder: &Path) -> Paths {
        Paths {
            spend: folder.join("collection.json"),
            budgets: folder.join("budgets.json"),
            orchestrators: folder.join("orchestrators.json"),
            restore: folder.join("restore.json"),
            kept: folder.join("agents").join("reopen.json"),
            landed: folder.join("agents").join("landed.json"),
            steer: folder.join("steer"),
            shells: folder.join("by-shell"),
            arrangement: folder.join("agents").join("arrangement.json"),
            kept_places: folder.join("agents").join("kept-places.json"),
            switched: Some(folder.join("last.json")),
            carried: folder.join("agents").join("carried.json"),
            clone: Some(folder.join("core.git")),
            harness: Some(folder.join("agentic-harness.pi")),
            switches: Some(folder.join("switches.jsonl")),
        }
    }

    #[test]
    fn what_each_source_says_changes_its_own_part() {
        let folder = std::env::temp_dir().join(format!("agents-sources-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        fs::write(&paths.spend, r#"{"agentsToday": "$4"}"#).expect("the collection");
        fs::write(&paths.budgets, r#"{"sessions": {}}"#).expect("the budgets");
        let mut inputs = Inputs::default();
        inputs.hear(
            Heard::Sessions(json!({"records": [{"key": "session:a"}]})),
            &paths,
        );
        inputs.hear(Heard::Jobs(json!({"records": [{"key": "job:1"}]})), &paths);
        inputs.hear(Heard::Files(paths.all()), &paths);
        inputs.hear(Heard::Sharing(true), &paths);
        assert!(
            inputs.sessions.get("session:a").is_some(),
            "the node's session held"
        );
        assert!(inputs.jobs.get("job:1").is_some(), "the node's job held");
        assert_eq!(inputs.spend, Some(json!({"agentsToday": "$4"})));
        assert_eq!(inputs.budgets, Some(json!({"sessions": {}})));
        assert_eq!(inputs.orchestrators, None, "a file not there holds nothing");
        assert!(inputs.sharing, "the shell's sharing held");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_joel_says_of_the_sessions_to_reopen_is_kept() {
        let folder = std::env::temp_dir().join(format!("agents-kept-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        fs::write(
            &paths.restore,
            r#"{"boot": "the-last", "sessions": [{"id": "01-a", "cwd": "/tmp"}]}"#,
        )
        .expect("the restore list");
        let mut inputs = Inputs::default();
        inputs.hear(Heard::Files(paths.all()), &paths);
        inputs.hear(Heard::PutOff, &paths);
        inputs.hear(Heard::Forget("01-a".into()), &paths);
        let kept = Kept {
            put_off: "the-last".into(),
            forgotten: vec!["01-a".into()],
        };
        assert_eq!(
            inputs.kept, kept,
            "the boot offered put off, and the one forgotten"
        );
        assert_eq!(Kept::read(&paths.kept), kept, "written for the next start");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_joel_reads_of_the_landings_is_kept() {
        let folder = std::env::temp_dir().join(format!("agents-landed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        let mut inputs = Inputs::default();
        inputs.hear(
            Heard::Landings(json!({"records": [{"key": "landing:s:o#1", "session": "s",
                "ref": "o#1", "at": "2026-11-11T01:00:00Z", "what": "One"}]})),
            &paths,
        );
        inputs.hear(
            Heard::Read(vec!["s o#1 2026-11-11T01:00:00Z".into(), "gone".into()], 7),
            &paths,
        );
        let (records, _) = inputs.landed();
        assert_eq!(records[0]["landed"]["read"], 7, "read when Joel read it");
        assert_eq!(
            crate::landed::Reading::read(&paths.landed),
            Some(inputs.read.clone()),
            "written for the next start"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_joel_said_of_a_landing_is_served_on_it_as_its_talk_goes() {
        let paths = paths_in(&std::env::temp_dir().join("agents-fed-back-unwritten"));
        let mut inputs = Inputs::default();
        inputs.hear(
            Heard::Landings(json!({"records": [{"key": "landing:s:o#1", "session": "s",
                "ref": "o#1", "at": "2026-11-11T01:00:00Z", "what": "One"}]})),
            &paths,
        );
        inputs.hear(
            Heard::FedBack(
                json!({"at": 7, "landing": "landed:agents:s o#1 2026-11-11T01:00:00Z",
                "words": "too dim", "by": "talk", "sent": "talk-7000", "to": "The Skiff"}),
            ),
            &paths,
        );
        inputs.hear(
            Heard::Talks(
                json!({"records": [{"key": "talk:S:talk-7000", "address": "S",
                "id": "talk-7000", "lines": [{"stage": "delivered", "text": "too dim",
                "at": "2026-11-11T01:02:00Z"}]}]}),
            ),
            &paths,
        );
        let (records, _) = inputs.landed();
        assert_eq!(
            (
                &records[0]["landed"]["saying"],
                &records[0]["landed"]["said"][0]["stage"]
            ),
            (&json!("delivered"), &json!("delivered")),
            "the talk's stage on the landing's row and in its drawer"
        );
    }

    #[test]
    fn where_the_shell_says_the_moving_sessions_sat_is_kept_and_served() {
        let folder = std::env::temp_dir().join(format!("agents-places-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        let said = KeptPlaces::said(&json!({"01-a": {"workspace": 3, "column": 2, "at": 7}}))
            .expect("places");
        let mut inputs = Inputs::default();
        inputs.hear(Heard::KeepPlaces(said.clone()), &paths);
        assert_eq!(inputs.kept_places, said, "held");
        assert_eq!(
            KeptPlaces::read(&paths.kept_places),
            Some(said.clone()),
            "written for the next start"
        );
        let mut served = crate::served::Served::default();
        served.take(inputs.made());
        assert_eq!(
            served.get("kept-places:agents"),
            Some(&said.record()),
            "served with the sessions to reopen"
        );
        // Written by another hand, as a test does with the shell stopped.
        let elsewhere = KeptPlaces::default();
        elsewhere.write(&paths.kept_places).expect("written");
        inputs.hear(Heard::Files(vec![paths.kept_places.clone()]), &paths);
        assert_eq!(inputs.kept_places, elsewhere, "heard as it lands");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn places_heard_written_while_a_write_has_failed_leave_memory_as_it_is() {
        let folder = std::env::temp_dir().join(format!("agents-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let paths = paths_in(&folder);
        let stale = Arrangement::of(&json!({"v": 1, "boots": {"this": {"s-shut": {}}}}));
        stale.write(&paths.arrangement).expect("written");
        let ahead = Arrangement::of(&json!({"v": 1, "boots": {"this": {}}}));
        let mut inputs = Inputs {
            arrangement: ahead.clone(),
            unwritten: Some(crate::inputs::Unwritten {
                why: "No space left on device".into(),
                since: 0,
                said: 0,
            }),
            ..Inputs::default()
        };
        inputs.hear(Heard::Files(vec![paths.arrangement.clone()]), &paths);
        assert_eq!(
            inputs.arrangement, ahead,
            "memory ahead of the file isn't thrown back to it, let-gos and all"
        );
        inputs.unwritten = None;
        inputs.hear(Heard::Files(vec![paths.arrangement.clone()]), &paths);
        assert_eq!(
            inputs.arrangement, stale,
            "with no write failed, a file written by another hand is read as it lands"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn kept_places_heard_written_while_their_write_has_failed_leave_memory_as_it_is() {
        // A folder of its own, as cases run side by side.
        let folder =
            std::env::temp_dir().join(format!("agents-kept-unwritten-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        // A file where the places' folder would be, so their write fails.
        fs::write(folder.join("agents"), "").expect("a file in the folder's way");
        let said = KeptPlaces::said(&json!({"01-a": {"workspace": 3, "column": 2, "at": 7}}))
            .expect("places");
        let mut inputs = Inputs::default();
        inputs.hear(Heard::KeepPlaces(said.clone()), &paths);
        assert!(
            inputs.kept_places_unwritten.is_some(),
            "the failed write is held, so the file isn't read back over memory"
        );
        // The file as it stood before, landing as the folder comes back.
        fs::remove_file(folder.join("agents")).expect("the way cleared");
        KeptPlaces::default()
            .write(&paths.kept_places)
            .expect("written");
        inputs.hear(Heard::Files(vec![paths.kept_places.clone()]), &paths);
        assert_eq!(
            inputs.kept_places, said,
            "memory ahead of the file isn't thrown back to it"
        );
        // Written by the next try, as the loop's case has it.
        inputs.keep_kept_places(&paths.kept_places);
        let by_hand = KeptPlaces::said(&json!({"01-b": {"workspace": 4, "column": 1, "at": 8}}))
            .expect("places");
        by_hand.write(&paths.kept_places).expect("written");
        inputs.hear(Heard::Files(vec![paths.kept_places.clone()]), &paths);
        assert_eq!(
            inputs.kept_places, by_hand,
            "and once they're written, a file another hand writes is read as it lands"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn kept_places_whose_write_failed_are_written_again_though_nothing_moves() {
        let folder = std::env::temp_dir().join(format!("agents-kept-again-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        // A file where the places' folder would be, so their write fails.
        fs::write(folder.join("agents"), "").expect("a file in the folder's way");
        let said = KeptPlaces::said(&json!({"01-a": {"workspace": 3, "column": 2, "at": 7}}))
            .expect("places");
        let mut inputs = Inputs::default();
        let mut still = Still::default();
        let start = Instant::now();
        let ten = Duration::from_secs(10);
        inputs.hear(Heard::KeepPlaces(said.clone()), &paths);
        inputs.keep_when_still(&mut still, &paths, start);
        assert_eq!(
            still.wait(start),
            Some(ten),
            "a failed write asks to be woken in 10 s, though no window moved"
        );
        let woken = start + ten;
        inputs.keep_when_still(&mut still, &paths, woken);
        assert_eq!(
            (inputs.kept_places_unwritten.is_some(), still.wait(woken)),
            (true, Some(ten)),
            "failing again when woken, it asks to be woken 10 s on again"
        );
        // Windows moving all the while: kept places don't hang on where
        // they are, so their try isn't put off until they're still.
        still.moved(woken + Duration::from_secs(9));
        assert_eq!(
            still.wait(woken + Duration::from_secs(9)),
            Some(Duration::from_secs(1)),
            "windows moving put off no try of the kept places"
        );
        fs::remove_file(folder.join("agents")).expect("the way cleared");
        let tried = woken + ten;
        inputs.keep_when_still(&mut still, &paths, tried);
        assert_eq!(
            (
                inputs.kept_places_unwritten.clone(),
                KeptPlaces::read(&paths.kept_places)
            ),
            (None, Some(said)),
            "woken then, it writes them though the windows still move and the shell said \
             nothing more"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_files_are_handed_over_once_as_they_stand_at_the_start() {
        let folder = std::env::temp_dir().join(format!("agents-start-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        let (to, heard) = mpsc::channel();
        follow_files(&paths, to).expect("the files are followed");
        let Ok(Heard::Files(first)) = heard.recv() else {
            panic!("the files first");
        };
        assert_eq!(first, paths.all());
        let _ = fs::remove_dir_all(&folder);
    }
}
