//! Joel's stop on everything, as before a demo: what it pauses on this
//! machine and asks of the others, and that a resume undoes exactly that.
//! The machine is a fake one here, answering systemctl and mycelium as
//! the real ones would, and writing down everything asked of it. What it
//! says of a machine that refuses it or is never reached is in reach.rs,
//! and the script another machine runs for it in script.rs.

mod reach;
mod script;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pineapple::pause::{self, Plan, Runs, Tended, Took};
use serde_json::Value;

/// A unit on the fake machine: its name, its slice, and whether it's
/// frozen.
struct Unit {
    id: &'static str,
    slice: &'static str,
    frozen: bool,
}

/// A machine whose agents' timers, work and sessions are as given, which
/// writes down each program it's asked to run, stops, starts, freezes and
/// thaws what it's asked to, refuses whatever `refusing` names, and
/// answers as mycelium does a command it never ran for what `unreached`
/// names, and one whose link dropped once it began for what `dropped`
/// names. When `peek` names a file, what it holds as the Mac is asked is
/// kept in `peeked`, and when `block` names one, a folder is made there as
/// the Mac is asked, so no file can be written in its place.
#[derive(Default)]
struct Machine {
    asked: Vec<String>,
    timers: Vec<String>,
    units: Vec<Unit>,
    refusing: Option<&'static str>,
    unreached: Option<&'static str>,
    dropped: Option<&'static str>,
    peek: Option<PathBuf>,
    peeked: Option<String>,
    block: Option<PathBuf>,
    starts_stopped: bool,
}

/// What a run says when mycelium couldn't run the command there at all.
const NEVER_RAN: &str = "mycelium: mac's mycelium is too old to run commands (exit status: 255)";

/// Why a run failed when the node hung up on it, once it began.
const DROPPED: &str = "mycelium: the node hung up before the run ended (exit status: 255)";

impl Machine {
    fn unit(&mut self, id: &str) -> Option<&mut Unit> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    /// Each unit as `systemctl show` says it, its keys in an order other
    /// than the one asked, which nothing promises.
    fn show(&self, names: &[&str]) -> String {
        names
            .iter()
            .map(
                |name| match self.units.iter().find(|unit| unit.id == *name) {
                    Some(unit) => format!(
                        "Slice={}\nId={}\nActiveState=active\nFreezerState={}\n",
                        unit.slice,
                        unit.id,
                        if unit.frozen { "frozen" } else { "running" },
                    ),
                    None => {
                        format!("Slice=\nId={name}\nActiveState=inactive\nFreezerState=running\n")
                    }
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Runs for Machine {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        let asked = format!("{} {}", program.to_string_lossy(), args.join(" "));
        self.asked.push(asked.clone());
        if let Some(peek) = self.peek.as_ref().filter(|_| asked.contains("machine=mac")) {
            self.peeked = fs::read_to_string(peek).ok();
        }
        if let Some(block) = self
            .block
            .as_ref()
            .filter(|_| asked.contains("machine=mac"))
        {
            fs::create_dir_all(block).expect("a folder in the note's way");
        }
        if self.refusing.is_some_and(|refused| asked.contains(refused)) {
            return Err("refused here".to_owned());
        }
        if self.unreached.is_some_and(|never| asked.contains(never)) {
            return Err(NEVER_RAN.to_owned());
        }
        if self.dropped.is_some_and(|dropped| asked.contains(dropped)) {
            // What the script says first, before mycelium's own word.
            return Err(format!("pineapple: begun there\n{DROPPED}"));
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(match words.as_slice() {
            ["--user", "list-units", "--type=timer", "--state=active", ..] => self
                .timers
                .iter()
                .map(|timer| format!("{timer} loaded active waiting"))
                .collect::<Vec<_>>()
                .join("\n"),
            [
                "--user",
                "list-units",
                "--type=service,scope",
                "--state=running",
                ..,
            ] => self
                .units
                .iter()
                .map(|unit| format!("{} loaded active running", unit.id))
                .collect::<Vec<_>>()
                .join("\n"),
            ["--user", "show", _, names @ ..] => self.show(names),
            ["--user", "stop", names @ ..] => {
                self.timers.retain(|timer| !names.contains(&timer.as_str()));
                String::new()
            }
            ["--user", "start", names @ ..] => {
                self.timers
                    .extend(names.iter().map(|name| (*name).to_owned()));
                String::new()
            }
            ["--user", verb @ ("freeze" | "thaw"), names @ ..] => {
                for name in names {
                    if let Some(unit) = self.unit(name) {
                        unit.frozen = *verb == "freeze";
                    }
                }
                String::new()
            }
            ["sessions", "--realm", "personal"] => r#"{"Ok":{"Sessions":[
                {"id":"p1","machine":"builder","state":"idle"},
                {"id":"p2","machine":"here","state":"ended"}]}}"#
                .to_owned(),
            ["sessions", ..] => r#"{"Ok":{"Sessions":[
                {"id":"a","machine":"here","state":"busy"},
                {"id":"b","machine":"here","state":"idle"},
                {"id":"c","machine":"here","state":"ended"},
                {"id":"d","machine":"mac","state":"idle"}]}}"#
                .to_owned(),
            ["starts", "list", ..] => format!(
                r#"{{"Ok":{{"Starts":{{"stopped":{},"newest":[]}}}}}}"#,
                self.starts_stopped
            ),
            _ => String::new(),
        })
    }
}

fn plan() -> Plan {
    Plan {
        systemctl: PathBuf::from("systemctl"),
        mycelium: Some(PathBuf::from("mycelium")),
        timers: vec!["cadence*.timer".to_owned(), "pain-hear.timer".to_owned()],
        machines: vec!["mac".to_owned()],
        realms: vec![],
        keep: vec![
            "sccache.service".to_owned(),
            "merge-net.service".to_owned(),
            "*demo*".to_owned(),
        ],
    }
}

fn running(id: &'static str, slice: &'static str) -> Unit {
    Unit {
        id,
        slice,
        frozen: false,
    }
}

fn machine() -> Machine {
    Machine {
        timers: vec!["cadence.timer".to_owned(), "cadence-day.timer".to_owned()],
        units: vec![
            running("ox4wfv-gate.service", "jobs.slice"),
            running("jobs.service", "jobs.slice"),
            running("sccache.service", "jobs.slice"),
            running("reader-demo-1.service", "jobs.slice"),
            running("timed.service", "jobs-timed.slice"),
            running("merge-net.service", "jobs-timed.slice"),
            running("work-1-2.scope", "jobs-work-unranked.slice"),
            running("show-demo-3.service", "jobs-work-unranked.slice"),
            running("awaited.service", "jobs-awaited.slice"),
            running("work-3-4.scope", "jobs-awaited.slice"),
            running("kitty-5-6.scope", "app.slice"),
            running("pineapple-slack.service", "background.slice"),
            running("run-7-8.scope", "shared.slice"),
        ],
        ..Machine::default()
    }
}

/// What a stop freezes on [`machine`]: agents' work and the push waiting
/// on its walk slot, never Joel's own terminal's, the cache, the merge
/// checks or the demo's.
const FROZEN: &str = "ox4wfv-gate.service jobs.service timed.service work-1-2.scope run-7-8.scope";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-pause-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch state folder");
    dir
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("written")).expect("json")
}

fn asked_with(machine: &Machine, words: &str) -> Vec<String> {
    machine
        .asked
        .iter()
        .filter(|asked| asked.contains(words))
        .cloned()
        .collect()
}

/// Held while a case starts a program, and while one tends: a child holds
/// a copy of every file this process has open until it runs its program,
/// the pause folder's lock among them, and a tend finding it held skips
/// its turn, as it should on a machine but not in a case.
static FORKS: Mutex<()> = Mutex::new(());

fn tend(state: &Path, at: u64, machine: &mut Machine) -> Tended {
    let _held = FORKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pause::tend(&plan(), state, "here", at, machine).expect("tends")
}

fn resumed(state: &Path, at: u64) {
    pineapple::resume::resume(&state.join("stops"), "all", at, "joel").expect("resumed");
}

#[test]
fn a_stop_says_everything_stops_and_pauses_only_agents_work() {
    let state = scratch("stop");
    let mut here = machine();
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");

    let record = json(&state.join("stops/stop-all-1.json"));
    assert_eq!(
        record["reaches"][0], "everything",
        "the stop reaches everything: {record}"
    );
    assert_eq!(record["shadow"], false, "Joel's own stop is live: {record}");
    assert_eq!(record["by_joel"], true, "Joel said it: {record}");
    assert!(
        record["resumed"].is_null(),
        "it stands until he resumes: {record}"
    );

    assert!(
        here.asked
            .iter()
            .any(|asked| asked == "mycelium starts stop"),
        "agents' starts are refused: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "systemctl --user stop "),
        vec!["systemctl --user stop cadence.timer cadence-day.timer".to_owned()],
        "only the agents' timers running are stopped: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec![format!("systemctl --user freeze {FROZEN}")],
        "agents' work is frozen, never Joel's own, the cache, the merge checks or the \
         demo's: {:?}",
        here.asked
    );
    let mac = asked_with(&here, "mycelium run --need machine=mac -- sh -c");
    assert_eq!(mac.len(), 1, "the Mac is asked to stop: {:?}", here.asked);
    assert!(
        mac[0].contains("starts stop") && mac[0].contains("stop-all-1.json"),
        "the Mac writes the same stop and refuses its starts: {}",
        mac[0]
    );
    assert_eq!(
        took,
        vec![
            Took {
                machine: "here".to_owned(),
                failed: vec![],
                recorded: true,
                reached: true,
            },
            Took {
                machine: "mac".to_owned(),
                failed: vec![],
                recorded: true,
                reached: true,
            },
        ],
        "both machines took it"
    );
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping 3 sessions on here and mac",
        "Joel is told in one line"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_resume_undoes_exactly_what_its_stop_paused_and_only_once() {
    let state = scratch("undo");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    assert!(
        pause::settle(&plan(), &state, "here", 2, &mut here)
            .expect("settles")
            .is_empty(),
        "nothing is undone while the stop stands: {:?}",
        here.asked
    );
    assert_eq!(
        pause::stands(&state).as_deref(),
        Some("stop-all-1"),
        "the pause stands"
    );

    resumed(&state, 3);
    let took = pause::settle(&plan(), &state, "here", 3, &mut here).expect("settles");
    assert_eq!(
        here.asked[..3],
        [
            format!("systemctl --user thaw {FROZEN}"),
            "systemctl --user start cadence.timer cadence-day.timer".to_owned(),
            "mycelium starts resume".to_owned(),
        ],
        "what was paused is undone in reverse, and nothing more"
    );
    let mac = asked_with(&here, "mycelium run --need machine=mac");
    assert!(
        mac.len() == 1 && mac[0].contains("starts resume") && mac[0].contains("\"by\":\"joel\""),
        "the Mac is given the resumed record and lets its agents start: {mac:?}"
    );
    assert_eq!(took.len(), 2, "both machines are said: {took:?}");
    assert!(
        pause::stands(&state).is_none(),
        "the pause no longer stands"
    );

    let mut again = Machine::default();
    let took = pause::settle(&plan(), &state, "here", 4, &mut again).expect("settles");
    assert!(
        took.is_empty() && again.asked.is_empty(),
        "an undone pause isn't undone again: {:?}",
        again.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn starts_joel_refused_before_the_stop_stay_refused_after_it() {
    let state = scratch("starts");
    let mut here = Machine {
        starts_stopped: true,
        ..machine()
    };
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    resumed(&state, 2);
    pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    assert!(
        asked_with(&here, "mycelium starts ")
            .iter()
            .all(|asked| asked == "mycelium starts list"),
        "starts already refused are neither refused again nor let go: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn work_started_or_thawed_while_it_stands_is_frozen_and_thawed_with_the_rest() {
    let state = scratch("hold");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    here.units
        .push(running("1dzylj-probe.service", "jobs.slice"));
    // The jobs sorter thaws a scope by writing its cgroup, and systemd
    // goes on saying it's frozen, so what it says can't be what's frozen.
    let tended = tend(&state, 2, &mut here);
    assert_eq!(
        tended.froze,
        vec!["1dzylj-probe.service".to_owned()],
        "only what's new is added: {tended:?}"
    );
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec![format!(
            "systemctl --user freeze {FROZEN} 1dzylj-probe.service"
        )],
        "all of it is frozen again, systemd's frozen or not: {:?}",
        here.asked
    );
    resumed(&state, 3);
    here.asked.clear();
    tend(&state, 3, &mut here);
    assert_eq!(
        here.asked.first().map(String::as_str),
        Some(format!("systemctl --user thaw {FROZEN} 1dzylj-probe.service").as_str()),
        "it's thawed with the rest, once the stop is resumed: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn work_the_sorter_thaws_is_frozen_again_though_nothing_is_new() {
    let state = scratch("sorter");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    // Every unit still reads frozen, as systemd says of a scope the sorter
    // thawed through its cgroup, and systemd 262 writes the cgroup again on
    // a second freeze.
    let tended = tend(&state, 2, &mut here);
    assert!(tended.froze.is_empty(), "nothing new is added: {tended:?}");
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec![format!("systemctl --user freeze {FROZEN}")],
        "what the stop froze is frozen again with nothing new: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_step_a_resume_cannot_take_is_tried_again_after_a_minute() {
    let state = scratch("retry");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    resumed(&state, 2);
    here.refusing = Some("start cadence");
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    assert_eq!(
        pause::said("resuming", &took, None),
        "resuming every session on here and mac; here couldn't start agents' timers: \
         refused here, tried again each minute",
        "Joel is told what's left and that it's tried again"
    );
    assert_eq!(
        pause::left(&state),
        vec!["cadence.timer, cadence-day.timer stopped".to_owned()],
        "and it's read back, whoever did the undo"
    );
    here.asked.clear();
    here.refusing = None;
    tend(&state, 3, &mut here);
    assert!(
        here.asked.is_empty(),
        "it isn't tried again within the minute: {:?}",
        here.asked
    );
    tend(&state, 2 + 60_000, &mut here);
    assert_eq!(
        here.asked,
        vec!["systemctl --user start cadence.timer cadence-day.timer".to_owned()],
        "only what was left is tried again"
    );
    assert!(
        pause::left(&state).is_empty(),
        "nothing's left once it's done"
    );
    here.asked.clear();
    tend(&state, 3 + 120_000, &mut here);
    assert!(
        here.asked.is_empty(),
        "and then it's done: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_resume_left_to_retry_lets_nothing_go_while_a_newer_stop_stands() {
    let state = scratch("retry-held");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    resumed(&state, 2);
    here.refusing = Some("mycelium starts resume");
    pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    // The first stop's refusal still stands, so the second leaves it be.
    here.refusing = None;
    here.starts_stopped = true;
    pause::stop(&plan(), &state, "stop-all-2", "here", 3, &mut here).expect("stops again");
    here.asked.clear();
    tend(&state, 2 + 60_000, &mut here);
    assert!(
        asked_with(&here, "starts resume").is_empty(),
        "agents start again mid-stop: {:?}",
        here.asked
    );
    let mut resuming = Machine::default();
    assert!(
        pause::settle(&plan(), &state, "here", 4, &mut resuming)
            .expect("settles")
            .is_empty(),
        "nor does a resume of the older stop: {:?}",
        resuming.asked
    );
    resumed(&state, 5);
    here.asked.clear();
    pause::settle(&plan(), &state, "here", 5, &mut here).expect("settles");
    assert_eq!(
        asked_with(&here, "mycelium starts resume"),
        vec!["mycelium starts resume".to_owned()],
        "once the newer is resumed, the older lets them go: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_stop_that_cannot_list_agents_work_says_so() {
    let state = scratch("unlisted");
    let mut here = Machine {
        refusing: Some("list-units"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    assert_eq!(
        pause::said("stopping", &took, Some(3)),
        "stopping 3 sessions on here and mac; here couldn't list agents' timers: refused \
         here, couldn't list agents' work: refused here",
        "Joel is told what wasn't paused"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_running_sessions_stops_though_no_plan_names_it() {
    let state = scratch("unnamed");
    let mut here = machine();
    let unnamed = Plan {
        machines: vec![],
        ..plan()
    };
    let took = pause::stop(&unnamed, &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let reached: Vec<&str> = took.iter().map(|one| one.machine.as_str()).collect();
    assert_eq!(
        reached,
        ["here", "mac"],
        "the Mac runs a session, so it stops too"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_stop_reaches_every_realm_this_machine_joins() {
    let state = scratch("realms");
    let mut here = machine();
    let realms = Plan {
        machines: vec![],
        realms: vec!["work".to_owned(), "personal".to_owned()],
        ..plan()
    };
    let took = pause::stop(&realms, &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    assert_eq!(
        asked_with(&here, "mycelium starts stop"),
        vec![
            "mycelium starts stop --realm work".to_owned(),
            "mycelium starts stop --realm personal".to_owned()
        ],
        "each realm's starts are refused: {:?}",
        here.asked
    );
    let builder = asked_with(
        &here,
        "mycelium run --realm personal --need machine=builder",
    );
    assert!(
        builder.len() == 1 && builder[0].contains("starts stop --realm 'personal'"),
        "a machine running a session in the other realm is asked there: {:?}",
        here.asked
    );
    let sessions = pause::sessions(&realms, &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping 4 sessions on here, mac and builder",
        "sessions are counted in each realm, once, and never ended ones"
    );
    resumed(&state, 2);
    here.asked.clear();
    pause::settle(&realms, &state, "here", 2, &mut here).expect("settles");
    assert_eq!(
        asked_with(&here, "mycelium starts resume"),
        vec![
            "mycelium starts resume --realm work".to_owned(),
            "mycelium starts resume --realm personal".to_owned()
        ],
        "and let go in each: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_realm_that_cannot_list_its_sessions_hides_only_its_own_machines() {
    let state = scratch("realm-down");
    let mut here = Machine {
        refusing: Some("sessions --realm personal"),
        ..machine()
    };
    let realms = Plan {
        machines: vec![],
        realms: vec!["work".to_owned(), "personal".to_owned()],
        ..plan()
    };
    let took = pause::stop(&realms, &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let sessions = pause::sessions(&realms, &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping every session on here and mac; here couldn't list the sessions in \
         personal: refused here, so the machines running only those may run on",
        "the Mac, found in the realm that answers, stops, and Joel is told which didn't"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn only_agents_places_refuse_the_stop() {
    let manager = "0::/user.slice/user-1000.slice/user@1000.service/";
    for (place, agents) in [
        ("app.slice/kitty-5-0.scope", false),
        ("app.slice/run-p1-i2.scope", false),
        ("jobs.slice/jobs-awaited.slice/work-3-4.scope", false),
        ("agents.slice/mycelium-pi-work.scope", true),
        ("jobs.slice/jobs.service", true),
        (
            "jobs.slice/jobs-work.slice/jobs-work-here.slice/work-1-2.scope",
            true,
        ),
        ("offload.slice/offload-1.scope", true),
        ("app.slice/run-u7.service", true),
        ("background.slice/pineapple.service", true),
    ] {
        assert_eq!(
            pause::agents_cgroup(&format!("{manager}{place}\n")).is_some(),
            agents,
            "{place} is taken for {}",
            if agents { "Joel's" } else { "agents'" }
        );
    }
    for outside in ["0::/\n", "0::/system.slice/sshd.service\n", ""] {
        assert_eq!(
            pause::agents_cgroup(outside),
            None,
            "{outside:?} is outside the user's manager, so it says nothing"
        );
    }
}

#[test]
fn only_a_stop_whose_record_stands_is_standing() {
    let state = scratch("stray");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    fs::write(state.join("paused/stop-all-1.new"), "{\"id\":\"half").expect("a stray");
    fs::write(state.join("paused/notes.txt"), "{\"id\":\"other\"}").expect("a stray");
    assert_eq!(
        pause::stands(&state).as_deref(),
        Some("stop-all-1"),
        "stray files are passed over"
    );
    resumed(&state, 2);
    assert!(
        pause::stands(&state).is_none(),
        "a stop resumed doesn't stand, though its pause isn't undone yet"
    );
    let _ = fs::remove_dir_all(&state);
}
