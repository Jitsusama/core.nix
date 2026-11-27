//! Joel's asks landed with nothing showing he has what he asked for: each
//! landed over two hours ago, and its probe, the check its keeper names in
//! his terms, hasn't passed here since. On 26 November, of 586 asks marked
//! landed, one recorded him using the thing, as done was counted at the
//! merge and a switch checks the machine's health, never a given change.
//! So each is asked of the cell that answers for its whole result: run the
//! probe, fix what it shows, or say why it can't pass.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value;

use super::at_risk::{self, Repo};
use super::{Class, DAY, Measure, count, first_owner, lines, misses, seconds, short};
use crate::tracking::{self, Tracking};

/// The measure's key.
pub const KEY: &str = "asks-unproven";

/// Where the probe runs are kept, under the state folder: one line each,
/// naming the ask by its id, when it ran, whether it passed, the switch it
/// ran after and who ran it. Every cell here writes as the same person, so
/// a run is taken as its keeper's probe runner wrote it; who may write one
/// is that runner's to hold, as a cell is asked to run the probe itself.
pub const PROBES: &str = "mission/probes.jsonl";

/// Where the machine's switches are kept, under the state folder.
const SWITCHES: &str = "integration/switches.jsonl";

/// Why the probe runs are unread when none has been written yet, which
/// is no tear to mend.
const NONE_KEPT: &str = "none kept";

/// Who keeps the asks list and writes each ask's probe, so answers for an
/// ask with no cell named and for the probes not yet written.
pub const KEEPER: &str = "J7XHQN";

/// What the line counting the week's asks naming no probe opens with, the
/// same from hour to hour, so its keeper is asked once while any is left.
pub const UNNAMED: &str = "his asks landed in the last week naming no probe";

/// How long after landing an ask may go with no probe passing.
const GRACE: i64 = 2 * 3600;

/// How far back a landing is still asked after one by one: older asks are
/// the probes' backfill to judge, counted on one line, not a week's worth
/// of questions at once.
const SINCE: i64 = 7 * DAY;

/// Whether a switch's line carries a change, by its repository and number,
/// or `None` where that can't be told.
pub type Carried<'a> = dyn Fn(&Value, Repo, u32) -> Option<bool> + 'a;

/// The measure, from the asks list `asks`, and the switches and probe runs
/// kept under `state_home`, at `now`, each switch judged to carry a change
/// by its merge on the main of cadence's checkouts, core's in `folder` and
/// each of `checkouts`, as the misses read them. A probe record that can't
/// be read is unread, never none passed, and so is one that's gone or
/// emptied: its keeper writes it, so a lost one would otherwise ask again
/// of every ask proven. A switch record that's gone is none, as on a
/// machine never switched, but one that can't be read, or is there but
/// emptied, is unread, or every pass would count as though no switch had
/// come.
#[must_use]
pub fn read(
    state_home: &Path,
    folder: &Path,
    checkouts: &[String],
    asks: &str,
    now: i64,
) -> Measure {
    let running = misses::running_generation();
    read_on(state_home, folder, checkouts, asks, now, running)
}

/// The measure as `read` has it, on a machine whose system generation
/// `running` runs, if known: what runs now is judged by the switch the
/// record names for that generation, since a switch back, or a boot into
/// an older generation, leaves the newest naming what doesn't run, as the
/// misses read it; one the record doesn't name can't be judged.
fn read_on(
    state_home: &Path,
    folder: &Path,
    checkouts: &[String],
    asks: &str,
    now: i64,
    running: Option<u64>,
) -> Measure {
    let probes = match fs::read_to_string(state_home.join(PROBES)) {
        Ok(text) if text.trim().is_empty() => Err(NONE_KEPT.to_owned()),
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(NONE_KEPT.to_owned()),
        Err(err) => Err(err.to_string()),
    };
    let switches = match fs::read_to_string(state_home.join(SWITCHES)) {
        Ok(text) if text.trim().is_empty() => Err("emptied".to_owned()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        read => read.map_err(|err| err.to_string()),
    };
    let repos = misses::checkouts_of(folder, checkouts);
    // Each main is read once, from a week before the oldest landing, as a
    // merge comes before its ask is marked landed.
    let since = lines(asks)
        .filter_map(|ask| seconds(&ask["landed"]))
        .min()
        .unwrap_or(now)
        - SINCE;
    unnamed(switches.as_deref().ok(), running);
    let merges: RefCell<BTreeMap<&str, Option<BTreeMap<u64, String>>>> = RefCell::default();
    // The merge of `number` on `name`'s main: none while that main is
    // unread, and none within while it holds no such merge.
    let merge_of = |name: &'static str, number: u32| {
        merges
            .borrow_mut()
            .entry(name)
            .or_insert_with(|| {
                let Some(path) = repos.get(name) else {
                    eprintln!(
                        "cadence: no checkout of {name} is named, so no ask of it is judged: \
                         name one in jitsusama.cadence.checkouts"
                    );
                    return None;
                };
                let read = misses::merges_since(path, since);
                if read.is_none() {
                    eprintln!(
                        "cadence: couldn't read {name}'s main at {}, so no ask of it is judged",
                        path.display()
                    );
                }
                read
            })
            .as_ref()
            .map(|merges| merges.get(&u64::from(number)).cloned())
    };
    let judged: RefCell<BTreeMap<Judging, Option<bool>>> = RefCell::default();
    let carried = |line: &Value, repo: Repo, number: u32| {
        let name = repo_name(repo);
        let key = (line["revs"].to_string(), name, number);
        if let Some(known) = judged.borrow().get(&key) {
            return *known;
        }
        // Only core's main is cadence's own clone, fetched each tick, so a
        // merge missing from another's may be stale sight or a wrong `pr`;
        // it says which number, to be mended.
        let merge = match merge_of(name, number) {
            Some(None) if repo != Repo::Core => {
                eprintln!(
                    "cadence: {name}'s main holds no merge of #{number}, so an ask naming \
                     it is unjudged: fetch its checkout, or mend the ask's pr"
                );
                None
            }
            merge => merge.flatten(),
        };
        let held = merge.and_then(|merge| misses::holds(line, name, &merge, &repos));
        judged.borrow_mut().insert(key, held);
        held
    };
    // Core's main, fetched each tick and read two hours past the merge,
    // holds every merge an ask can name of it, so one it lacks is a wrong
    // `pr`, the ask's to mend, not the hour's sight.
    let unmerged =
        |repo: Repo, number: u32| repo == Repo::Core && merge_of("core", number) == Some(None);
    let sight = Sight {
        carried: &carried,
        unmerged: &unmerged,
        running: running.map_or(Runs::Unknown, Runs::Generation),
    };
    seen(
        asks,
        probes.as_deref().map_err(String::as_str),
        switches.as_deref().map_err(String::as_str),
        &sight,
        now,
    )
}

/// Logs a generation `running` that the switch `record`, read and holding
/// lines, doesn't name, as no ask can be judged by what runs then.
fn unnamed(record: Option<&str>, running: Option<u64>) {
    let (Some(record), Some(running)) = (record, running) else {
        return;
    };
    if !record.trim().is_empty() && !lines(record).any(|line| generation(&line) == Some(running)) {
        eprintln!(
            "cadence: generation {running} runs, which the switch record doesn't name, so \
             no ask is judged by what runs now"
        );
    }
}

/// How cadence sees what the switches carry: `carried` judges a change on
/// a switch's line, `unmerged` says a change cadence's own main holds no
/// merge of, and `running` says which switch runs now.
struct Sight<'a> {
    carried: &'a Carried<'a>,
    unmerged: &'a dyn Fn(Repo, u32) -> bool,
    running: Runs,
}

/// Which switch runs now.
#[derive(Clone, Copy)]
enum Runs {
    /// The newest the record names, as where nothing else is known.
    Newest,
    /// The one of this system generation.
    Generation(u64),
    /// None that can be told.
    Unknown,
}

/// A switch line's system generation, as its record writes it.
fn generation(line: &Value) -> Option<u64> {
    line["generation"]
        .as_u64()
        .or_else(|| line["switch"].as_u64())
}

/// A switch's revisions, as its line writes them, and a change's repository
/// and number: what a judgement of whether it carries the change is kept by.
type Judging = (String, &'static str, u32);

/// The name cadence's checkouts know `repo` by, as the misses call each.
const fn repo_name(repo: Repo) -> &'static str {
    match repo {
        Repo::Core => "core",
        Repo::World => "world",
        Repo::Mycelium => "mycelium",
        Repo::Harness => "harness",
    }
}

/// One probe run, as read from its line.
struct Run {
    at: i64,
    passed: bool,
    switch: Option<i64>,
}

/// A landed ask unproven: when it landed, its line and who answers for it.
type Unproven = (i64, String, String);

/// Joel's own asks in the asks list `text` landed over two hours before
/// `now` whose latest probe run since landing, in `probes`, didn't pass, or
/// which have none. Those landed in the last week naming a probe are each a
/// line, oldest landing first, saying so when the switch in `switches` it
/// runs on now doesn't carry it, as `carried` judges each change it names.
/// Older ones count on one line of their own, the keeper's, as the probes'
/// backfill is, and so do those naming no probe, since its keeper writes
/// the probe and no cell can run one unwritten; so
/// none stays out of the figure, nor asks a cell for what isn't its own.
/// Each ask is read from its latest line by its id, else when it was asked
/// and its words, as the list writes an ask whole each time; a line with no
/// id is the same ask as one with an id and its time and words. One he
/// confirmed himself, whatever `confirmed` says, needs no probe, nor does
/// one put down or closed in any other way; one landed waits on no ruling,
/// as the parked measure reads it, so a ruling it was parked on before
/// can't keep it out. Each line names who answers for the ask's whole
/// result, its `answers`, else its owner, else the list's keeper. Until any ask in the
/// list names a probe, or while the runs can't be read, the measure says so
/// and goes unjudged, so no ask is asked for a probe its keeper hasn't
/// written yet and an unread record never reads as none passed. The
/// newest switch is taken to run, and every merge to be found.
#[must_use]
pub fn unproven(
    text: &str,
    probes: Result<&str, &str>,
    switches: Result<&str, &str>,
    carried: &Carried<'_>,
    now: i64,
) -> Measure {
    let sight = Sight {
        carried,
        unmerged: &|_, _| false,
        running: Runs::Newest,
    };
    seen(text, probes, switches, &sight, now)
}

/// The measure as `unproven` has it, seen as `sight` sees the switches.
fn seen(
    text: &str,
    probes: Result<&str, &str>,
    switches: Result<&str, &str>,
    sight: &Sight<'_>,
    now: i64,
) -> Measure {
    let latest = latest(text);
    let named = latest.values().any(names_probe);
    let runs = probes.map_err(ToOwned::to_owned).and_then(runs);
    match &runs {
        Err(why) if why == NONE_KEPT => {}
        Err(why) => eprintln!("cadence: the probe runs are unread, {why}; its keeper must mend it"),
        Ok(_) => {}
    }
    let by_ask = runs.as_ref().ok();
    let record = switches.unwrap_or_default();
    let mut timed: Vec<(i64, Value)> = lines(record)
        .filter_map(|line| Some((seconds(&line["at"])?, line)))
        .collect();
    timed.sort_by_key(|(at, _)| *at);
    // A record holding lines but no time read is one in another shape, not
    // none, so no pass is taken on it as if no switch had come.
    let switches_unread = match switches {
        Err(why) => Some(format!("the switch record unread: {}", clean(why))),
        Ok(_) if timed.is_empty() && record.lines().any(|l| !l.trim().is_empty()) => {
            Some("the switch record unread: no line names when it ran".to_owned())
        }
        Ok(_) => None,
    };
    let (older, week): (Vec<_>, Vec<_>) = latest
        .values()
        .filter_map(|ask| {
            let switches = Switches {
                switched: &timed,
                sight,
            };
            Some((ask, unproven_ask(ask, by_ask, &switches, now)?))
        })
        .partition(|(_, (landed, _, _))| *landed < now - SINCE);
    let (unnamed, mut open): (Vec<_>, Vec<_>) =
        week.into_iter().partition(|(ask, _)| !names_probe(ask));
    // Those its cell can mend come first, so the card names them first.
    open.sort_by_key(|(_, (landed, line, _))| (blind(line), *landed));
    let mut open: Vec<Unproven> = open.into_iter().map(|(_, unproven)| unproven).collect();
    // A line only cadence's own sight holds stays on the card uncounted, nor
    // names the measure's owner, so the hour's outage can't become the
    // drag, as the misses hold theirs; while it's all the card holds, the
    // measure is unjudged rather than in, as cadence can't say it is. An
    // older ask whose run cadence can't judge is held back the same way.
    let sighted = |(_, line, _): &&Unproven| !blind(line);
    let older_blind = older
        .iter()
        .filter(|(_, (_, line, _))| line == BLIND)
        .count();
    let older = older.len() - older_blind;
    let blinded = open.iter().filter(|unproven| !sighted(unproven)).count() + older_blind;
    let counted = open.iter().filter(sighted).count() + unnamed.len() + older;
    let figure = count(counted);
    // The keeper is asked for the probes it hasn't written; the older asks
    // are its backfill, already dated, so that line holds no "; " and is
    // never filed.
    if !unnamed.is_empty() {
        let n = unnamed.len();
        let line = format!("{UNNAMED}; {n} of them · {KEEPER}");
        open.push((0, line, KEEPER.to_owned()));
    }
    if older > 0 {
        let line = format!("{older} older than a week, left to the probes' backfill · {KEEPER}");
        open.push((0, line, KEEPER.to_owned()));
    }
    let says = "his asks landed over 2 h ago with no probe passing since";
    let unjudged = match (&runs, named) {
        (Err(why), _) => Some(format!("the probe runs unread: {}", clean(why))),
        (Ok(_), false) => Some("no ask names a probe yet".to_owned()),
        (Ok(_), true) => switches_unread.or_else(|| {
            (counted == 0 && blinded > 0).then(|| format!("{SIGHTLESS} {blinded} of them are live"))
        }),
    };
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: unjudged
            .as_ref()
            .map_or_else(|| says.to_owned(), |why| format!("{says} ({why})")),
        now: figure,
        target: unjudged.is_none().then_some(0.0),
        owner: open
            .iter()
            .find(sighted)
            .map(|(_, _, o)| o.clone())
            .unwrap_or_default(),
        behind: open.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// The asks in the list `text`, each by its id, else when it was asked and
/// its words, from its latest line, as the list writes an ask whole each
/// time.
fn latest(text: &str) -> BTreeMap<String, Value> {
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for ask in lines(text) {
        // A twin the keeper folded into a kept record carries the record's
        // id and a `~`, so it neither counts again nor stands in for it.
        let key = match ask["id"].as_str().filter(|id| !id.is_empty()) {
            Some(id) if id.contains('~') => continue,
            Some(id) => id.to_owned(),
            None => said(&ask),
        };
        latest.insert(key, ask);
    }
    // An ask first written with no id and later with one is one ask; the
    // list writes an id on every line it writes now, so one with none is
    // only ever the older.
    let ided: Vec<String> = latest
        .iter()
        .filter(|(key, ask)| **key != said(ask))
        .map(|(_, ask)| said(ask))
        .collect();
    for key in ided {
        latest.remove(&key);
    }
    latest
}

/// The runs in `probes` by the id of the ask each names, as several asks
/// can share the moment they were asked. A line that won't parse, or isn't
/// a run naming its ask, when it ran and whether it passed, leaves them
/// unread: a torn failing run after a pass would otherwise read as proven,
/// and a record in another shape as no probe having run.
fn runs(probes: &str) -> Result<BTreeMap<String, Vec<Run>>, String> {
    let mut by_ask: BTreeMap<String, Vec<Run>> = BTreeMap::new();
    for (n, text) in probes.lines().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let line: Value = serde_json::from_str(text)
            .map_err(|err| format!("line {} won't parse: {err}", n + 1))?;
        // A switch it names is a number, or a later failure would read as
        // the one its cell closed.
        let switch = &line["switch"];
        let (Some(ask), Some(at), Some(passed), true) = (
            line["ask"].as_str(),
            seconds(&line["at"]),
            line["passed"].as_bool(),
            switch.is_null() || switch.is_i64(),
        ) else {
            return Err(format!(
                "line {} isn't a run naming its ask, when it ran, whether it passed and \
                 any switch by number",
                n + 1
            ));
        };
        by_ask.entry(ask.to_owned()).or_default().push(Run {
            at,
            passed,
            switch: switch.as_i64(),
        });
    }
    Ok(by_ask)
}

/// What a line ends with when only cadence's own sight keeps it from being
/// judged: no checkout, git failing, or a generation running that the
/// switch record doesn't name. It
/// stays on the card uncounted, and isn't the answering cell's to mend, so
/// it isn't put to the cell, nor lets go of what was, as the misses hold
/// back what the hour can't see.
pub const BLIND: &str =
    ", not live yet, as cadence can't tell whether the switch running carries it";

/// What the measure says it's unjudged by when every line on it is one
/// only cadence's own sight holds.
const SIGHTLESS: &str = "cadence can't tell whether";

/// Whether `m`, the measure, is unjudged only because every line on it is
/// one cadence's own sight holds: its runs and switches were read, so a
/// line gone from it is still known to be gone, as on a card judged.
#[must_use]
pub fn sightless(m: &Measure) -> bool {
    m.target.is_none()
        && m.says.contains(&format!("({SIGHTLESS} "))
        && m.says.ends_with(" of them are live)")
}

/// Whether a line of the measure's ends its state with `BLIND`, read where
/// it is written, just before its cell, so no ask's own words can mark it.
pub fn blind(line: &str) -> bool {
    line.rsplit_once(" · ")
        .is_some_and(|(line, _)| line.ends_with(BLIND))
}

/// The machine's switches, oldest first, and how cadence sees them.
struct Switches<'a> {
    switched: &'a [(i64, Value)],
    sight: &'a Sight<'a>,
}

/// What the switch running at a moment says of an ask.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Carry {
    /// No switch has come since it landed.
    Unswitched,
    /// The switch carries every change it names.
    Carries,
    /// The switch lacks a change it names.
    Lacks,
    /// Whether the switch carries a change it names can't be told, so it
    /// counts as not carrying it.
    Unjudged,
    /// Cadence's own main holds no merge of a change it names, so its `pr`
    /// names the wrong one.
    Unmerged,
}

impl Carry {
    /// What a line says of an ask whose switch running now is `self`, its
    /// probe `failed_on_it` or not: only a failure on what carries it is
    /// the cell's to mend while cadence can't tell what runs now.
    fn live(self, failed_on_it: bool) -> &'static str {
        match self {
            Carry::Carries | Carry::Unmerged => "",
            Carry::Unswitched => ", not live yet, as the switch running came before it landed",
            Carry::Lacks => ", not live yet, as the switch running doesn't carry it",
            Carry::Unjudged if failed_on_it => {
                ", though cadence can't tell whether the switch running now carries it"
            }
            Carry::Unjudged => BLIND,
        }
    }
}

impl Switches<'_> {
    /// Whether the switch running now carries `ask`, landed at `landed`:
    /// the one the record names for the generation running, as a switch
    /// back, or a boot into an older one, writes no line. One it doesn't
    /// name, or none known to run, can't be judged, unless its `pr` names a
    /// change cadence's own main holds no merge of.
    fn running(&self, ask: &Value, landed: i64) -> Carry {
        let pr = ask["pr"].as_str().unwrap_or_default();
        let unmerged = |&(repo, number): &(Repo, u32)| (self.sight.unmerged)(repo, number);
        if at_risk::changes(pr).iter().any(unmerged) {
            return Carry::Unmerged;
        }
        let line = match self.sight.running {
            Runs::Newest => self.switched.last(),
            Runs::Generation(running) => {
                let named = |(_, line): &&(i64, Value)| generation(line) == Some(running);
                match self.switched.iter().rfind(named) {
                    Some(line) => Some(line),
                    None => return Carry::Unjudged,
                }
            }
            Runs::Unknown => return Carry::Unjudged,
        };
        self.carry(ask, landed, line)
    }

    /// Whether the switch `run` ran on carries `ask`, landed at `landed`:
    /// the one whose generation it names, where the record names it, else
    /// the last before it ran.
    fn ran(&self, ask: &Value, landed: i64, run: &Run) -> Carry {
        let on = run.switch.and_then(|switch| u64::try_from(switch).ok());
        let line = on
            .and_then(|on| {
                self.switched
                    .iter()
                    .rfind(|(_, line)| generation(line) == Some(on))
            })
            .or_else(|| {
                self.switched
                    .iter()
                    .rfind(|(switched, _)| *switched <= run.at)
            });
        self.carry(ask, landed, line)
    }

    /// Whether the switch `line`, if any, carries `ask`: every change its
    /// `pr` names that a switch carries, as `carried` judges it, the
    /// harness's coming by its install instead. Where every part names a
    /// change a switch carries, a switch from before it was marked `landed`
    /// is judged on its merges too, as a merge comes before its mark, and
    /// one git can't judge is unjudged, the hour's to mend; where a part
    /// names no change found, or the harness's, whose install isn't read
    /// for a switch, only a switch since it landed counts, as there's no
    /// merge it can be shown to hold. With no switch, none has come.
    fn carry(&self, ask: &Value, landed: i64, line: Option<&(i64, Value)>) -> Carry {
        let Some((switched, line)) = line else {
            return Carry::Unswitched;
        };
        let pr = ask["pr"].as_str().unwrap_or_default();
        let all = at_risk::changes(pr);
        let changes: Vec<(Repo, u32)> = all
            .iter()
            .copied()
            .filter(|(repo, _)| *repo != Repo::Harness)
            .collect();
        let mut carry = Carry::Carries;
        if changes
            .iter()
            .any(|&(repo, number)| (self.sight.unmerged)(repo, number))
        {
            return Carry::Unmerged;
        }
        for &(repo, number) in &changes {
            match (self.sight.carried)(line, repo, number) {
                Some(true) => {}
                Some(false) => return Carry::Lacks,
                None => carry = Carry::Unjudged,
            }
        }
        let merged =
            !changes.is_empty() && changes.len() == all.len() && at_risk::unfound(pr).is_none();
        if *switched < landed && !merged {
            return Carry::Unswitched;
        }
        carry
    }
}

/// `ask`'s line and who answers for it, when it's Joel's own, landed in
/// the window before `now`, needs a probe, and its latest run since it
/// landed didn't pass or there is none, or did on a switch that no longer
/// runs, as after a switch back. An ask with no id has none of the
/// runs, as its keeper writes the id on every line it writes a probe on;
/// with the `runs` unread, none is said to have run. One the switch running
/// now doesn't carry isn't live yet, so its cell knows to chase the switch
/// rather than the code, and a run on a switch that doesn't carry it proves
/// nothing, as it ran on old code; with no switch record, or for an ask
/// only the harness's install carries, any run since it landed counts.
fn unproven_ask(
    ask: &Value,
    runs: Option<&BTreeMap<String, Vec<Run>>>,
    switches: &Switches<'_>,
    now: i64,
) -> Option<Unproven> {
    let any_run = installed(ask) || switches.switched.is_empty();
    let landed = seconds(&ask["landed"])?;
    // Any `closed` puts it out: a twin folded into the ask that carries its
    // probe, one superseded or answered, or one an older pass closed done.
    // No open ask is ever closed.
    let waits = tracking::of(ask) == Some(Tracking::His)
        && unset(&ask["confirmed"])
        && unset(&ask["closed"])
        && landed <= now - GRACE;
    if !waits {
        return None;
    }
    let last = ask["id"]
        .as_str()
        .and_then(|id| runs?.get(id))
        .into_iter()
        .flatten()
        // A run dated past now would stay the latest whatever runs after.
        .filter(|run| run.at >= landed && run.at <= now)
        .max_by_key(|run| run.at);
    let ran_on = last.map(|run| {
        let on = if any_run {
            Carry::Carries
        } else {
            switches.ran(ask, landed, run)
        };
        (run, on)
    });
    let proven = ran_on.is_some_and(|(run, on)| run.passed && on == Carry::Carries);
    // One past the week only counts, so the switch it would run on now,
    // which costs git, isn't asked of it; one whose run cadence can't
    // judge is as blind as one in the week.
    if landed < now - SINCE {
        let blinded = ran_on.is_some_and(|(_, on)| on == Carry::Unjudged);
        let line = if blinded { BLIND } else { "" };
        return (!proven).then(|| (landed, line.to_owned(), String::new()));
    }
    let now_on = if any_run {
        Carry::Carries
    } else {
        switches.running(ask, landed)
    };
    // A pass stands while what runs now carries it; switched back from
    // what it passed on, it isn't live.
    if proven && !matches!(now_on, Carry::Lacks | Carry::Unswitched) {
        return None;
    }
    // Only a failure on what carries it is the cell's to mend while cadence
    // can't tell what runs now; otherwise the line is the hour's own.
    let failed_on_it = ran_on.is_some_and(|(run, on)| !run.passed && on == Carry::Carries);
    let unswitched = now_on.live(failed_on_it);
    let state = match (!names_probe(ask), ran_on) {
        _ if now_on == Carry::Unmerged => {
            "core's main holds no merge of a change its pr names, so its pr wants mending"
                .to_owned()
        }
        (_, Some((run, Carry::Unjudged))) => format!(
            "its probe {} on a switch cadence can't judge",
            if run.passed { "passed" } else { "failed" }
        ),
        // A failure there says no more of the change than a pass does.
        (_, Some((run, on))) if on != Carry::Carries => format!(
            "its probe {} before a switch could carry it",
            if run.passed { "passed" } else { "failed" }
        ),
        (false, Some((run, _))) if run.passed => "its probe passed".to_owned(),
        (true, _) => "it names no probe".to_owned(),
        (false, None) if runs.is_none() => "its probe runs unread".to_owned(),
        (false, None) => "its probe hasn't run since it landed".to_owned(),
        (false, Some((run, _))) => match run.switch {
            Some(switch) => format!("its probe failed after switch {switch}"),
            None => "its probe failed".to_owned(),
        },
    };
    let owner = [&ask["answers"], &ask["owner"]]
        .into_iter()
        .filter_map(Value::as_str)
        // Only what an address can hold, so nothing in it can split its
        // key, close the span its close command is given in, or reach a
        // terminal.
        .map(|owner| {
            first_owner(owner)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | '-' | '_'))
                .collect::<String>()
        })
        .find(|owner| !owner.is_empty())
        .unwrap_or_else(|| KEEPER.to_owned());
    // Cleaned once shortened, as shortening turns any space into a plain one.
    let words = clean(&short(ask["words"].as_str().unwrap_or_default()));
    // Only a time the list wrote as one, so `at` carries no other text.
    let at = match ask["at"].as_str() {
        Some(at) if seconds(&ask["at"]).is_some() => clean(at),
        _ => "?".to_owned(),
    };
    let line = format!("{words}, asked {at}; {state}{unswitched} · {owner}");
    Some((landed, line, owner))
}

/// The asks list and the probe runs are other programs' text, so a newline
/// or a separator the owing rules split on can't make a line read as
/// another's.
fn clean(text: &str) -> String {
    // Nor can a backtick or double quote close the span the cell's close
    // command is given in.
    // Nor can a character that reorders or hides text make the key a
    // terminal shows differ from the one pasted: the kit keeps every one
    // it keeps from the terminal out, and makes a line break a space.
    let mut text = terminal_kit::layout::one_line(text).replace(['`', '"'], "'");
    // Until none is left, as a replaced separator can close another.
    while text.contains("; ") || text.contains(" · ") {
        text = text.replace("; ", ";?").replace(" · ", " ? ");
    }
    text
}

/// Whether every change `ask`'s `pr` names is the harness's, which reaches
/// the machine by its install rather than a switch. A part naming no change
/// that can be found might name one a switch carries, so it waits on one.
fn installed(ask: &Value) -> bool {
    let pr = ask["pr"].as_str().unwrap_or_default();
    let named = at_risk::changes(pr);
    at_risk::unfound(pr).is_none()
        && !named.is_empty()
        && named.iter().all(|(repo, _)| *repo == Repo::Harness)
}

/// Whether an ask names a probe: a field missing, null, false, blank or an
/// empty list or object names none.
fn names_probe(ask: &Value) -> bool {
    match &ask["probe"] {
        Value::Null | Value::Bool(false) => false,
        Value::String(probe) => !probe.trim().is_empty(),
        Value::Array(probe) => !probe.is_empty(),
        Value::Object(probe) => !probe.is_empty(),
        _ => true,
    }
}

/// Whether a field is missing, null, false or empty.
fn unset(field: &Value) -> bool {
    field.is_null() || field.as_str() == Some("") || field.as_bool() == Some(false)
}

/// An ask known by when it was asked and its words, for one with no id.
fn said(ask: &Value) -> String {
    format!("{}\n{}", ask["at"], ask["words"])
}

#[cfg(test)]
mod tests;
