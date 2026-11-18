//! The machines of the realm, judged from the node's own answer: whether a
//! link reaches each, its disk, its battery and what its own health says
//! of its code. Its own module because it changes for the machines'
//! reasons (what a node reports, and what Joel can act on from a machine),
//! which the other three parts don't share.

use std::io;

use serde_json::Value;

use super::ties::ties;
use super::{Answer, and, count, side_of, unread};
use crate::record::{Deeper, Health, Part, Thing};

/// A gigabyte, as a disk's free room is said.
const GIGABYTE: f64 = 1e9;
/// A battery at or under this, off the charger, is worth a look.
const LOW_CHARGE: u64 = 20;
/// A quest backup older than this, in seconds, is worth a look: it runs
/// every hour, so two missed runs in a row.
const BACKUP_LATE: u64 = 2 * 3_600;
/// A quest backup older than this, in seconds, is failing.
const BACKUP_FAILING: u64 = 6 * 3_600;
/// How far ahead of this clock, in seconds, a landing may say it was
/// before it's taken as a time this clock can't count from: a clock's
/// small step back is no fault.
const BACKUP_AHEAD: i64 = 60;
/// An hour, in seconds, the unit a late backup is said in.
const HOUR: u64 = 3_600;
/// Where a late backup's run says why, where its runs are user services.
const BACKUP_HINT: &str = if cfg!(target_os = "linux") {
    " (journalctl --user -u quest-backup says why)"
} else {
    ""
};

/// How this machine's quest backup last landed, as its landing file says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backup {
    /// It landed this many seconds ago.
    Landed(u64),
    /// Its file is there but says no time this clock can count from, and
    /// why.
    Unread(&'static str),
}

/// What a quest backup's landing file, as read, says at `now`, in seconds
/// since 1970; none where there's no file, as on a machine keeping no
/// backup. A file that can't be read, says no time, or says a time ahead
/// of this clock is said, since a broken writer would otherwise go quiet.
pub fn backup(read: io::Result<String>, now: i64) -> Option<Backup> {
    let text = match read {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(_) => return Some(Backup::Unread("its landing file can't be read")),
    };
    let Ok(landed) = text.trim().parse::<i64>() else {
        return Some(Backup::Unread("its landing file says no time"));
    };
    if landed > now.saturating_add(BACKUP_AHEAD) {
        return Some(Backup::Unread(
            "its landing file says a time ahead of this clock",
        ));
    }
    Some(Backup::Landed(
        now.saturating_sub(landed).max(0).unsigned_abs(),
    ))
}

/// The parts of another machine's own health this takes in: those only
/// that machine reads. Its machines, mycelium and network are the same
/// whole read from every machine, each already a part here, and taking
/// them in would spread one machine's trouble to all. The first two would
/// also latch, as each machine's word fed the other's.
const ITS_OWN: [&str; 1] = ["code"];

/// The machines of the realm, from `mycelium machines`, each with its
/// sessions from `mycelium health` where that was read, and its ties to
/// the network and the code from `net --json` and `repos --json` where
/// those were, and this machine's with how its quest backup last landed,
/// where it keeps one.
pub fn machines(
    machines: &Answer,
    health: &Answer,
    net: Option<&Value>,
    repos: Option<&Value>,
    backup: Option<Backup>,
) -> Part {
    let deeper = Deeper {
        command: "mycelium machines",
        site: "mycelium/",
        launcher: "Agents › Networks › Machines",
    };
    let list = match machines {
        Ok(said) => said
            .pointer("/Ok/Machines")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        Err(why) => return unread("machines", why, deeper),
    };
    if list.is_empty() {
        return unread("machines", "the node knows of no machine", deeper);
    }
    let sessions = health.as_ref().ok().and_then(|said| said.get("sessions"));
    // The node says this machine first, whose own parts are parts here.
    let mut read: Vec<(Thing, Vec<String>)> = list
        .iter()
        .enumerate()
        .map(|(at, said)| {
            let others = (at > 0).then(|| said.get("health")).flatten();
            let its = sessions.and_then(|all| all.get(name_of(said)));
            let here = (at == 0).then_some(backup).flatten();
            let (mut thing, looks) = machine(said, its, others, here);
            // A health read that lists none on a machine says none there.
            let cells = sessions.map(|_| its.unwrap_or(&Value::Null));
            thing.ties = ties(said, cells, net, repos);
            (thing, looks)
        })
        .collect();
    // The worst first, then by name, so the line a look wants leads.
    read.sort_by(|(a, _), (b, _)| b.health.cmp(&a.health).then_with(|| a.name.cmp(&b.name)));
    let side = side_of(
        read.iter()
            .filter(|(thing, _)| thing.health > Health::Good)
            .map(|(thing, looks)| format!("{} {}", thing.name, looks.join(", ")))
            .collect(),
    );
    let things: Vec<Thing> = read.into_iter().map(|(thing, _)| thing).collect();
    let health = things
        .iter()
        .map(|thing| thing.health)
        .max()
        .unwrap_or(Health::Good);
    let says = match things
        .iter()
        .filter(|thing| thing.health > Health::Good)
        .count()
    {
        0 => format!(
            "{} ready",
            count(things.len() as u64, "machine", "machines")
        ),
        _ => things
            .iter()
            .filter(|thing| thing.health > Health::Good)
            .map(|thing| format!("{} {}", thing.name, thing.says))
            .collect::<Vec<_>>()
            .join("; "),
    };
    let holds = count(things.len() as u64, "machine", "machines");
    Part {
        name: "machines",
        health,
        says,
        side,
        holds: Some(holds),
        things,
        deeper,
    }
}

fn name_of(said: &Value) -> &str {
    said.get("name").and_then(Value::as_str).unwrap_or("")
}

/// What a machine's line says, in the order it's found, and what in it is
/// worth a look: how bad, with the words for a row's side, which names no
/// machine and so drops the verb ("filling disk" for "has a filling
/// disk").
#[derive(Default)]
struct Said {
    line: Vec<String>,
    looks: Vec<(Health, String, String)>,
}

impl Said {
    /// Something the line says that is no look.
    fn say(&mut self, words: String) {
        self.line.push(words);
    }

    /// Something worth a look, as the line says it and as a side does.
    fn look(&mut self, health: Health, said: String, side: String) {
        self.line.push(said.clone());
        self.looks.push((health, said, side));
    }
}

/// One machine: whether a link reaches it, what it's short of, its disk,
/// its battery, what its own health says of its own parts and its
/// sessions, each said only when it's worth saying, with the words of
/// what is worth a look in it.
///
/// Being short of room is said but never worth a look: agents' builds
/// hold the processor nearly all day, which isn't Joel's to act on, and
/// memory waits for a day measured to tell a held shortage from a spike.
fn machine(
    said: &Value,
    sessions: Option<&Value>,
    own: Option<&Value>,
    backup: Option<Backup>,
) -> (Thing, Vec<String>) {
    let mut words = Said::default();
    if said.get("state").and_then(Value::as_str) == Some("unreachable") {
        words.look(
            Health::Bad,
            "is out of reach".to_owned(),
            "out of reach".to_owned(),
        );
    }
    let over: Vec<&str> = said
        .pointer("/room/over")
        .and_then(Value::as_array)
        .map(|over| {
            over.iter()
                .filter_map(Value::as_str)
                .map(resource)
                .collect()
        })
        .unwrap_or_default();
    if !over.is_empty() {
        words.say(format!("is short of {}", and(&over)));
    }
    let free = said
        .get("disk_free")
        .and_then(Value::as_f64)
        .map(|bytes| format!("{:.0} GB free", bytes / GIGABYTE));
    let disk = |band: &str| {
        free.as_ref().map_or(format!("{band} disk"), |free| {
            format!("{band} disk, {free}")
        })
    };
    let band = match said.get("disk_band").and_then(Value::as_str) {
        Some("filling") => Some((Health::Watch, disk("filling"))),
        Some("nearly-full" | "critically-full") => Some((Health::Bad, disk("nearly full"))),
        _ => None,
    };
    if let Some((health, side)) = band {
        words.look(health, format!("has a {side}"), side);
    }
    let on_battery = said.pointer("/power/on_battery").and_then(Value::as_bool) == Some(true);
    if let Some(charge) = said.pointer("/power/charge").and_then(Value::as_u64)
        && on_battery
        && charge <= LOW_CHARGE
    {
        let side = format!("on battery at {charge}%");
        words.look(Health::Watch, format!("is {side}"), side);
    }
    // An error the firmware kept before the boot, as an abrupt reset leaves, is
    // said for a day after it (power.rs), so the morning after one in the
    // night says it.
    if let Some(ago) = said
        .pointer("/firmware/error_before_boot")
        .and_then(Value::as_u64)
    {
        // The time is the boot's: the record says nothing of when the
        // error was.
        let side = format!("booted {} after a firmware error", ago_said(ago));
        words.look(Health::Watch, side.clone(), side);
    }
    for part in ITS_OWN {
        match own.and_then(|own| own.pointer(&format!("/parts/{part}"))?.as_str()) {
            Some("watch") => words.look(
                Health::Watch,
                format!("says its {part} wants a look"),
                format!("{part} wants a look"),
            ),
            Some("bad") => words.look(
                Health::Bad,
                format!("says its {part} is failing"),
                format!("{part} failing"),
            ),
            _ => {}
        }
    }
    if let Some(backup) = backup {
        backup_looks(&mut words, backup);
    }
    let Said {
        mut line,
        mut looks,
    } = words;
    let health = looks
        .iter()
        .map(|(health, ..)| *health)
        .max()
        .unwrap_or(Health::Good);
    let named: Vec<String> = looks.iter().map(|(_, said, _)| said.clone()).collect();
    // The side says the worst first; a sort that keeps ties in place keeps
    // the rest in the order they were found.
    looks.sort_by(|(a, ..), (b, ..)| b.cmp(a));
    let side = side_of(looks.into_iter().map(|(.., side)| side).collect());
    if line.is_empty() {
        line.push("is ready".to_owned());
    }
    if let Some(sessions) = sessions {
        line.push(format!("runs {}", sessions_said(sessions)));
    }
    let thing = Thing {
        kind: "machine",
        name: name_of(said).to_owned(),
        health,
        says: line.join(", "),
        side,
        address: None,
        ties: Value::Null,
    };
    (thing, named)
}

/// How long ago, in whole hours, and under the first hour as one span.
/// The words stand still for an hour at a time: minutes changed them every
/// minute, and every watcher of the record was sent them again each time.
fn ago_said(seconds: u64) -> String {
    match seconds {
        0..3_600 => "under an hour ago".to_owned(),
        _ => format!("{} ago", count(seconds / 3_600, "hour", "hours")),
    }
}

/// A quest backup, said only once it is late, with where to read why, or
/// when when it last landed can't be told. A late one is said in whole
/// hours, so its words change once an hour rather than each minute, and
/// every watcher isn't sent them anew.
fn backup_looks(words: &mut Said, backup: Backup) {
    let age = match backup {
        Backup::Unread(why) => {
            words.look(
                Health::Watch,
                format!("can't tell when its quest backup last landed ({why})"),
                "quest backup unread".to_owned(),
            );
            return;
        }
        Backup::Landed(age) => age,
    };
    let health = match age {
        age if age > BACKUP_FAILING => Health::Bad,
        age if age > BACKUP_LATE => Health::Watch,
        _ => return,
    };
    let hours = age / HOUR;
    words.look(
        health,
        format!("hasn't landed a quest backup in {hours} h{BACKUP_HINT}"),
        format!("no quest backup in {hours} h"),
    );
}

/// A resource a node says a machine is short of, in Joel's words for it.
fn resource(said: &str) -> &str {
    match said {
        "cpu" => "processor",
        "io" => "disk time",
        other => other,
    }
}

/// A machine's sessions by how they stand: the busy and the idle, since
/// the ended ones are history.
fn sessions_said(sessions: &Value) -> String {
    let of = |key: &str| sessions.get(key).and_then(Value::as_u64).unwrap_or(0);
    let (busy, idle) = (of("busy"), of("idle"));
    match (busy, idle) {
        (0, 0) => "no session".to_owned(),
        (busy, 0) => format!("{} busy", count(busy, "session", "sessions")),
        (0, idle) => format!("{} idle", count(idle, "session", "sessions")),
        (busy, idle) => format!("{busy} busy and {idle} idle sessions"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::parts::tests::{NO_LOOK, said_health};

    /// The machines, read with no quest backup on this machine, as every
    /// case but the backup's reads them.
    fn machines(
        machines: &Answer,
        health: &Answer,
        net: Option<&Value>,
        repos: Option<&Value>,
    ) -> Part {
        super::machines(machines, health, net, repos, None)
    }

    #[test]
    fn this_machines_quest_backup_is_worth_a_look_past_two_hours_and_bad_past_six() {
        let here = laptop(&[], "healthy");
        let other = json!({"name": "desk", "state": "ready"});
        let at = |age: Option<u64>| {
            let part = super::machines(
                &Ok(realm(&[here.clone(), other.clone()])),
                &Err(String::new()),
                None,
                None,
                age.map(Backup::Landed),
            );
            let of = |name: &str| {
                part.things
                    .iter()
                    .find(|thing| thing.name == name)
                    .cloned()
                    .expect("each machine is listed")
            };
            (part.health, of("laptop"), of("desk"))
        };
        let (health, laptop, desk) = at(Some(2 * 3_600));
        assert_eq!(health, Health::Good, "two hours is on time");
        assert_eq!(laptop.says, "is ready");
        let (health, laptop, desk_late) = at(Some(2 * 3_600 + 60));
        assert_eq!(health, Health::Watch);
        assert_eq!(
            laptop.says,
            "hasn't landed a quest backup in 2 h \
             (journalctl --user -u quest-backup says why)"
        );
        assert_eq!(laptop.side, ["no quest backup in 2 h"]);
        assert_eq!(
            at(Some(2 * 3_600 + 59 * 60)).1.says,
            laptop.says,
            "its words keep to whole hours, so a watcher isn't sent them anew each minute"
        );
        assert_eq!(
            desk_late.health,
            Health::Good,
            "only this machine's backup is read"
        );
        let (health, laptop, _) = at(Some(6 * 3_600));
        assert_eq!((health, laptop.health), (Health::Watch, Health::Watch));
        let (health, laptop, _) = at(Some(6 * 3_600 + 1));
        assert_eq!((health, laptop.health), (Health::Bad, Health::Bad));
        assert_eq!(laptop.side, ["no quest backup in 6 h"]);
        let (health, laptop, _) = at(None);
        assert_eq!(
            (health, laptop.says.as_str()),
            (Health::Good, "is ready"),
            "a machine keeping no backup says nothing of one"
        );
        assert_eq!(desk.health, Health::Good);
    }

    #[test]
    fn a_landing_file_says_its_age_and_one_that_cant_be_counted_is_worth_a_look() {
        let now = 1_000_000;
        let read = |text: &str| backup(Ok(text.to_owned()), now);
        assert_eq!(read("999000\n"), Some(Backup::Landed(1_000)));
        assert_eq!(
            backup(Err(io::ErrorKind::NotFound.into()), now),
            None,
            "no file is a machine keeping no backup"
        );
        assert_eq!(
            backup(Err(io::ErrorKind::PermissionDenied.into()), now),
            Some(Backup::Unread("its landing file can't be read"))
        );
        assert_eq!(
            read("yesterday"),
            Some(Backup::Unread("its landing file says no time"))
        );
        assert_eq!(
            read("1000030"),
            Some(Backup::Landed(0)),
            "a clock's small step back"
        );
        assert_eq!(
            read("1003600"),
            Some(Backup::Unread(
                "its landing file says a time ahead of this clock"
            ))
        );
        let part = super::machines(
            &Ok(realm(&[laptop(&[], "healthy")])),
            &Err(String::new()),
            None,
            None,
            read("yesterday"),
        );
        let laptop = &part.things[0];
        assert_eq!(laptop.health, Health::Watch);
        assert_eq!(
            laptop.says,
            "can't tell when its quest backup last landed (its landing file says no time)"
        );
        assert_eq!(laptop.side, ["quest backup unread"]);
    }

    fn laptop(over: &[&str], band: &str) -> Value {
        json!({"name": "laptop", "state": "ready", "disk_free": 278_819_094_528_u64,
               "disk_band": band, "room": {"over": over},
               "power": {"on_battery": false, "charge": 76}})
    }

    /// What `mycelium machines` answers for these machines.
    fn realm(machines: &[Value]) -> Value {
        json!({"Ok": {"Machines": machines}})
    }

    #[test]
    fn each_machine_carries_its_ties_to_the_other_parts() {
        let net = json!({"devices": [
            {"id": "mac:1", "name": "laptop", "detail": {"eero": {"ethernet": true}}},
            {"id": "mac:2", "name": "studio",
             "detail": {"eero": {"ethernet": false, "band": "5 GHz"}}}]});
        let repos = json!({"machine": "studio", "repositories": []});
        let studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy",
                            "power": {"charge": 100}, "room": {"over": []}});
        let spare = json!({"name": "spare", "state": "ready", "disk_band": "healthy",
                           "room": {"over": []}});
        let part = machines(
            &Ok(realm(&[studio, laptop(&[], "filling"), spare])),
            &Ok(said_health()),
            Some(&net),
            Some(&repos),
        );
        let of = |name: &str| {
            let thing = part.things.iter().find(|thing| thing.name == name);
            &thing.expect("each machine is a thing").ties
        };
        assert_eq!(
            part.things[0].name, "laptop",
            "sorted worst first, ties kept"
        );
        assert_eq!(
            of("laptop")["cells"],
            json!({"busy": 55, "idle": 33, "ended": 27})
        );
        assert_eq!(
            of("studio")["cells"],
            json!({"busy": 2, "idle": 1, "ended": 0})
        );
        assert_eq!(
            of("spare")["cells"],
            json!({"busy": 0, "idle": 0, "ended": 0}),
            "a machine the health read lists none on says none, not unread"
        );
        assert_eq!(of("laptop")["network"][0]["by"], "wired");
        assert_eq!(of("studio")["network"][0]["by"], "5 GHz");
        assert_eq!(of("studio")["code"]["repositories"], 0);
        assert_eq!(of("laptop")["code"], Value::Null);
        assert_eq!(of("laptop")["figures"]["charge"], 76);
        assert_eq!(of("studio")["figures"]["charge"], 100);
        assert_eq!(
            of("spare")["acts"]["link"],
            "mycelium:machine/spare?from=system"
        );
    }

    #[test]
    fn a_machine_short_of_room_leads_with_what_it_lacks() {
        let studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy",
                            "room": {"over": []}});
        let part = machines(
            &Ok(realm(&[studio, laptop(&["memory", "cpu"], "filling")])),
            &Ok(said_health()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Watch);
        assert_eq!(
            part.things[0].name, "laptop",
            "the worst machine comes first"
        );
        assert_eq!(
            part.things[0].says,
            "is short of memory and processor, has a filling disk, 279 GB free, \
             runs 55 busy and 33 idle sessions"
        );
        assert_eq!(
            part.things[1].says,
            "is ready, runs 2 busy and 1 idle sessions"
        );
        assert!(
            part.says.starts_with("laptop is short of memory"),
            "the part says only the machines worth a look: {}",
            part.says
        );
        assert_eq!(
            part.side,
            ["laptop has a filling disk, 279 GB free"],
            "a side leaves out what the machine runs and what it's short of"
        );
        assert_eq!(
            part.things[0].side,
            ["filling disk, 279 GB free"],
            "a machine's own side is worded for a row's side"
        );
        assert!(part.things[1].side.is_empty(), "a ready machine has none");
    }

    #[test]
    fn a_machine_short_of_room_alone_is_ready() {
        let part = machines(
            &Ok(realm(&[laptop(&["memory", "cpu", "io"], "healthy")])),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(
            part.health,
            Health::Good,
            "agents' builds fill the processor nearly all day, which isn't Joel's to act on"
        );
        assert_eq!(part.says, "1 machine ready");
        assert_eq!(part.side, NO_LOOK);
        assert_eq!(
            part.things[0].says, "is short of memory, processor and disk time",
            "what it's short of is still said"
        );
        assert!(part.things[0].side.is_empty(), "{:?}", part.things[0].side);
    }

    #[test]
    fn a_machines_side_says_its_worst_first() {
        let mut studio = laptop(&[], "healthy");
        studio["name"] = json!("studio");
        studio["power"] = json!({"on_battery": true, "charge": 12});
        studio["health"] = json!({"parts": {"code": "bad"}});
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio])),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(part.things[0].name, "studio");
        assert_eq!(
            part.things[0].says,
            "is on battery at 12%, says its code is failing"
        );
        assert_eq!(
            part.things[0].side,
            ["code failing", "on battery at 12%"],
            "the bad before the worth a look"
        );
    }

    #[test]
    fn a_side_of_many_machines_says_how_many_more() {
        let away = |name: &str| json!({"name": name, "state": "unreachable"});
        let many: Vec<Value> = ["a", "b", "c", "d", "e"].into_iter().map(away).collect();
        let part = machines(&Ok(realm(&many)), &Err(String::new()), None, None);
        assert_eq!(
            part.side,
            [
                "a is out of reach",
                "b is out of reach",
                "and 3 more worth a look"
            ]
        );
    }

    #[test]
    fn machines_all_well_are_counted() {
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy")])),
            &Err("no node".into()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Good);
        assert_eq!(part.says, "1 machine ready");
        assert_eq!(part.holds.as_deref(), Some("1 machine"));
        assert_eq!(part.side, NO_LOOK, "nothing is worth a look");
        assert_eq!(
            part.things[0].says, "is ready",
            "with no health, sessions go unsaid"
        );
    }

    #[test]
    fn a_machine_out_of_reach_or_nearly_full_is_bad() {
        let away = json!({"name": "spare", "state": "unreachable"});
        let part = machines(
            &Ok(realm(&[away, laptop(&[], "nearly-full")])),
            &Ok(said_health()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Bad);
        assert_eq!(part.things[0].name, "laptop", "two bad machines go by name");
        assert!(
            part.things[0].says.starts_with("has a nearly full disk"),
            "the disk leads what the laptop says: {}",
            part.things[0].says
        );
        assert_eq!(part.things[1].says, "is out of reach");
    }

    #[test]
    fn a_low_battery_counts_only_off_the_charger() {
        let mut low = laptop(&[], "healthy");
        low["power"] = json!({"on_battery": true, "charge": 12});
        let part = machines(
            &Ok(realm(std::slice::from_ref(&low))),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(part.things[0].says, "is on battery at 12%");
        low["power"]["on_battery"] = json!(false);
        let charging = machines(&Ok(realm(&[low])), &Err(String::new()), None, None);
        assert_eq!(charging.health, Health::Good);
    }

    #[test]
    fn a_firmware_error_before_the_boot_is_worth_a_look() {
        let mut cut = laptop(&[], "healthy");
        cut["firmware"] = json!({"error_before_boot": 3 * 3_600 + 59});
        let part = machines(
            &Ok(realm(std::slice::from_ref(&cut))),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Watch);
        assert_eq!(
            part.things[0].says,
            "booted 3 hours ago after a firmware error"
        );
        assert_eq!(
            part.things[0].side,
            ["booted 3 hours ago after a firmware error"]
        );
        assert!(
            part.says
                .starts_with("laptop booted 3 hours ago after a firmware error"),
            "the part's line reads as a sentence: {}",
            part.says
        );
        cut["firmware"]["error_before_boot"] = json!(90);
        let part = machines(
            &Ok(realm(std::slice::from_ref(&cut))),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(
            part.things[0].says,
            "booted under an hour ago after a firmware error"
        );
        // Read again later in the same hour, the machine says the same.
        cut["firmware"]["error_before_boot"] = json!(3_500);
        let later = machines(&Ok(realm(&[cut])), &Err(String::new()), None, None);
        assert_eq!(
            later.things[0], part.things[0],
            "a later read in the same hour changed the machine"
        );
    }

    #[test]
    fn how_long_ago_is_said_in_whole_hours() {
        assert_eq!(ago_said(0), "under an hour ago");
        assert_eq!(ago_said(60), "under an hour ago");
        assert_eq!(ago_said(3_599), "under an hour ago");
        assert_eq!(ago_said(3_600), "1 hour ago");
        assert_eq!(ago_said(7_199), "1 hour ago");
        assert_eq!(ago_said(7_200), "2 hours ago");
    }

    #[test]
    fn another_machine_says_what_its_own_code_is() {
        let mut studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy"});
        studio["health"] = json!({"health": "bad", "parts": {"code": "watch", "network": "good"}});
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio.clone()])),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Watch, "its code is worth a look");
        assert_eq!(part.things[0].name, "studio");
        assert_eq!(part.things[0].says, "says its code wants a look");
        studio["health"]["parts"]["code"] = json!("bad");
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio])),
            &Err(String::new()),
            None,
            None,
        );
        assert_eq!(part.health, Health::Bad);
        assert_eq!(part.things[0].says, "says its code is failing");
    }

    #[test]
    fn the_realms_own_parts_and_this_machines_health_are_never_taken_in() {
        let mut here = laptop(&[], "healthy");
        here["health"] = json!({"health": "bad", "parts": {"code": "bad"}});
        let mut studio = json!({"name": "studio", "state": "ready"});
        studio["health"] = json!({"health": "bad", "parts": {
            "machines": "bad", "mycelium": "bad", "network": "bad", "code": "good"}});
        let part = machines(&Ok(realm(&[here, studio])), &Err(String::new()), None, None);
        assert_eq!(
            part.health,
            Health::Good,
            "this machine's code is its own part, and the rest are the same whole \
             read from every machine: {}",
            part.says
        );
    }
}
