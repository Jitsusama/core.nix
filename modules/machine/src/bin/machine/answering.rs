//! The answers to what's asked over the socket: what a get, a list or a
//! watch names read first where it isn't fresh, a refresh, how the
//! sources stand and how long reads took, and the acts.

use std::collections::HashSet;
use std::time::Duration;

use crate::making::{makes, sources_of};
use crate::reading::{read, read_making};
use crate::{Daemon, FIRMWARE, PLACES, Place, Shared, lock};
use daemon_serve::serve::Wanted;
use machine::interface::{MADE, act_of, error, kind_of, refused};
use machine::{acts, firmware_rows, guard, screen_acts, supply, units};
use serde_json::{Value, json};

/// How long a firmware act that says no patience of its own is waited
/// for: a polkit prompt left unanswered, not a change still going in.
const ACT_PATIENCE: Duration = Duration::from_secs(120);
/// What a get answers from what's held, rather than reading again.
const FRESH: Duration = Duration::from_millis(1500);
/// What a get, a list or a watch beginning names, read first where it
/// isn't fresh or a key isn't held, and a place's rows made where they're
/// asked for and not yet made, so the answer holds them.
pub(crate) fn read_for(shared: &Shared, wanted: &Wanted<'_>) {
    refresh(shared, wanted.kinds, wanted.keys, wanted.places);
}

/// The kinds these kinds or keys are of.
fn wanted_kinds(kinds: &[String], keys: Option<&[String]>) -> HashSet<String> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| kind_of(key))
            .map(str::to_owned)
            .collect(),
        None => kinds.iter().cloned().collect(),
    }
}

/// The places whose rows these kinds or keys ask for, of these places or
/// every place's.
fn asked_places(
    kinds: &[String],
    keys: Option<&[String]>,
    places: Option<&[String]>,
) -> Vec<Place> {
    if !wanted_kinds(kinds, keys)
        .iter()
        .any(|kind| MADE.contains(&kind.as_str()))
    {
        return Vec::new();
    }
    PLACES
        .into_iter()
        .filter(|place| match keys {
            Some(keys) => keys.iter().any(|key| place.holds(key)),
            None => places.is_none_or(|places| places.iter().any(|p| p == place.name)),
        })
        .collect()
}

/// Reads the kinds of these kinds or keys that weren't read lately, and
/// what a place's rows are made of where they're asked for and not yet
/// made.
fn refresh(shared: &Shared, kinds: &[String], keys: Option<&[String]>, places: Option<&[String]>) {
    let asked = wanted_kinds(kinds, keys);
    let mut made = asked_places(kinds, keys, places);
    let stale: HashSet<String> = {
        let daemon = lock(&shared.daemon);
        if daemon.words.is_none() {
            made.clear();
        }
        let wanted = sources_of(&asked, &made);
        // A place whose headline isn't held has never been made.
        let unmade: Vec<&str> = made
            .iter()
            .filter(|place| !daemon.held.contains_key(&place.head()))
            .flat_map(|place| place.sources.iter().copied())
            .collect();
        wanted
            .iter()
            .filter(|kind| {
                // A key not yet held is read for, as a tree first asked is.
                let unheld = keys.is_some_and(|keys| {
                    keys.iter().any(|key| {
                        kind_of(key) == Some(kind.as_str()) && !daemon.held.contains_key(key)
                    })
                });
                unheld
                    || unmade.contains(&kind.as_str())
                    || daemon
                        .read_at
                        .get(*kind)
                        .is_none_or(|at| at.elapsed() > FRESH)
            })
            .cloned()
            .collect()
    };
    // Read for rows not yet watched, they're made though no watch holds
    // them yet.
    if !stale.is_empty() {
        read_making(shared, &stale, keys.unwrap_or_default(), &made);
    }
}

pub(crate) fn sources(daemon: &Daemon) -> Value {
    json!({"kernel": {"state": "answering"}, "udisks": daemon.udisks})
}

/// What health says beside what every daemon's says: how long the last
/// reads took, and what each kind of each took, so a slow read says
/// what held it.
pub(crate) fn health(shared: &Shared) -> Value {
    let daemon = lock(&shared.daemon);
    json!({
        "took_us": daemon.took.iter().collect::<Vec<_>>(),
        "kinds_took_us": daemon.kinds_took.iter().collect::<Vec<_>>(),
    })
}

/// The answer to an act, the daemon's own ask, on a thread of its own;
/// none for an ask it doesn't know.
pub(crate) fn own(shared: &Shared, said: &Value) -> Option<Value> {
    let act = act_of(said)?;
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    Some(if act.starts_with("firmware-") {
        firmware_act(shared, &id, act, said)
    } else {
        acted(shared, &id, act, said)
    })
}

/// Does what an act asks and says it's done, or why not; the kinds it
/// moved are read again first, so a watch hears of it before the answer.
fn acted(shared: &Shared, id: &Value, act: &str, said: &Value) -> Value {
    match act {
        "charge-limit" => {
            let name = said["supply"].as_str().unwrap_or("");
            let Some(wanted) = said["stop_pct"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
            else {
                return error(id, "bad-act", "a charge limit needs its stop_pct");
            };
            let set = {
                let mut reader = lock(&shared.reader);
                // Whatever came of it, the limit is read again at once.
                reader.charging.forget(name);
                supply::set_limit(&reader.root, name, wanted)
            };
            if let Err(unset) = set {
                return error(id, unset.code, &unset.message);
            }
            read(shared, &HashSet::from(["supply".to_owned()]), &[]);
            json!({"id": id, "done": true})
        }
        "brightness" => {
            let aimed = screen_acts::aimed(&lock(&shared.reader).root, said);
            if let Err(unlit) =
                aimed.and_then(|(device, steps)| screen_acts::brighten(&device, steps))
            {
                return error(id, unlit.code, &unlit.message);
            }
            read(shared, &HashSet::from(["backlight".to_owned()]), &[]);
            json!({"id": id, "done": true})
        }
        // The backlight program writes its warmth, or steers the screen
        // again, as a switch moves, which is heard as its files are.
        "night-light" | "follow-the-room" => match screen_acts::switched(act, said)
            .and_then(|(property, on)| screen_acts::switch(property, on))
        {
            Ok(()) => json!({"id": id, "done": true}),
            Err(unswitched) => error(id, unswitched.code, &unswitched.message),
        },
        "restart-unit" => match units::restart(said["unit"].as_str().unwrap_or("")) {
            Ok(()) => json!({"id": id, "done": true}),
            Err(unrestarted) => error(id, unrestarted.code, &unrestarted.message),
        },
        // The guard rewrites its status as it ends, which is heard as any
        // write of it is.
        "disk-keep" | "disk-unkeep" | "disk-clear" | "disk-thaw" => match guard::told(act, said) {
            Ok(told) => json!({"id": id, "done": true, "said": told}),
            Err(untold) => refused(id, untold.code, &untold.message, &untold.said),
        },
        _ => error(id, "unknown-act", &format!("no act {act:?} here")),
    }
}

/// Does a firmware act, on the thread daemon-serve answers it on, and
/// says how it went once it's done, waiting at most the `patience_s` it's
/// given; another asked while one goes is refused as `busy`, since fwupd
/// puts in one thing at a time. fwupd is asked again as it ends, whatever
/// it said.
fn firmware_act(shared: &Shared, id: &Value, act: &str, said: &Value) -> Value {
    let text = |name: &str| said[name].as_str().unwrap_or("").to_owned();
    let key = match act {
        "firmware-install" => format!("install:{}", text("device")),
        "firmware-fix" => format!("fix:{}", text("attribute")),
        "firmware-undo" => format!("undo:{}", text("attribute")),
        _ => "setup".to_owned(),
    };
    {
        let mut daemon = lock(&shared.daemon);
        if daemon.acting.going.is_some() {
            return error(id, "busy", "another firmware act is going");
        }
        daemon.acting.going = Some(key.clone());
        daemon.acting.ended.remove(&key);
    }
    // Firmware's rows say it's going before anything else is said.
    let firmware = HashSet::from(["firmware".to_owned()]);
    read(shared, &firmware, &[]);
    let patience = said["patience_s"]
        .as_u64()
        .map_or(ACT_PATIENCE, Duration::from_secs);
    let text = |name: &str| said[name].as_str().unwrap_or("");
    let done = match act {
        "firmware-install" => {
            let fwupdmgr = std::env::var(acts::FWUPDMGR).unwrap_or_default();
            acts::install(&fwupdmgr, text("device"), patience)
        }
        "firmware-fix" => acts::change(text("attribute"), false, patience),
        "firmware-undo" => acts::change(text("attribute"), true, patience),
        "firmware-setup" => acts::setup(patience),
        _ => Err(acts::refused("unknown-act", format!("no act {act:?} here"))),
    };
    {
        let mut daemon = lock(&shared.daemon);
        daemon.acting.going = None;
        // How it ended is said while Firmware's rows are watched, and a
        // launcher opened after has nothing to be told of it.
        if makes(&daemon, FIRMWARE) {
            let ended = match &done {
                Ok(()) => firmware_rows::Ended::Done,
                Err(refused) if refused.code == "stopped" => {
                    firmware_rows::Ended::Stopped(patience.as_secs().div_ceil(60))
                }
                Err(refused) => firmware_rows::Ended::Refused(refused.message.clone()),
            };
            daemon.acting.ended.insert(key, ended);
        }
        daemon.hurried.insert("firmware".to_owned());
    }
    shared.woken.notify_all();
    read(shared, &firmware, &[]);
    match done {
        Ok(()) => json!({"id": id, "done": true}),
        Err(refused) => error(id, refused.code, &refused.message),
    }
}
