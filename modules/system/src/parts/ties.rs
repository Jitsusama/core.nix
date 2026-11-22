//! What ties a machine to the other three parts, so a view can draw each
//! machine whole: the mycelium's cells on it, the network it reaches
//! through, the code it holds, its own figures, and what Joel can do with
//! it. The machine is the bridge between the others (Joel, 2026-11-17),
//! and its own module because each tie changes for its owner's reasons
//! and none of them changes what makes a machine worth a look.

use serde_json::{Value, json};

/// A gigabyte, the finest a machine's bytes are said in.
const GIGABYTE: f64 = 1e9;

/// The finest a share of a processor is said in: a tenth.
const TENTHS: f64 = 10.0;

/// The longest name mycelium-link takes for a machine.
const LONGEST_NAME: usize = 64;

/// The most characters of an update's why said, so a long error stays
/// one clause of a machine's line.
const WHY_AT_MOST: usize = 240;

/// The ties of one machine, as the node says it (`said`), with its
/// sessions from `mycelium health` (`Null` for a health read that lists
/// none on it), the network's `net --json` and this machine's
/// `repos --json`, each where it was read. A figure is said only as finely
/// as a view says it, so a watch is told a machine again only when what
/// it would say of it moves, not at every reading's change.
pub fn ties(
    said: &Value,
    sessions: Option<&Value>,
    net: Option<&Value>,
    repos: Option<&Value>,
) -> Value {
    let name = said.get("name").and_then(Value::as_str).unwrap_or("");
    // Whether the node reaches it, so a view can set one out of reach
    // apart rather than read its words, and when it was last heard, said
    // only then: a machine in reach is measured every reading, and saying
    // that would tell every watch of it again each time.
    let reached = said.get("state").and_then(Value::as_str) != Some("unreachable");
    let heard = if reached {
        Value::Null
    } else {
        said.get("measured_at").cloned().unwrap_or(Value::Null)
    };
    json!({
        "reached": reached,
        "heard_at": heard,
        "cells": sessions.map(cells),
        "network": net.map(|net| places(name, net)),
        "code": repos.and_then(|repos| code(name, repos)),
        "figures": figures(said),
        "acts": acts(name),
    })
}

/// What Joel can do with a machine: the launcher's link to its row, and
/// the commands, for a name mycelium-link would take (its `named`); none
/// for any other, since the name is the node's and a page offers both.
fn acts(name: &str) -> Value {
    let shaped = name.len() <= LONGEST_NAME
        && name.starts_with(|first: char| first.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || matches!(each, '.' | '_' | '-'));
    if !shaped {
        return Value::Null;
    }
    json!({
        "link": format!("mycelium:machine/{name}?from=system"),
        "commands": [
            format!("mycelium sprout --on {name}"),
            format!("mycelium update {name}"),
        ],
    })
}

/// `value` said in whole `unit`s, or as it was where it isn't a number.
fn coarse(value: Option<&Value>, unit: f64) -> Value {
    match value.and_then(Value::as_f64) {
        // A figure here is a count of bytes, a share or a rate, all well
        // inside what an f64 carries whole.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a machine's bytes and rates are positive and far below 2^53"
        )]
        Some(number) if unit >= 1.0 => json!(((number / unit).round() * unit) as u64),
        Some(number) => json!((number / unit).round() * unit),
        None => value.cloned().unwrap_or(Value::Null),
    }
}

/// The mycelium's cells on a machine: the busy, the idle and the ended,
/// as the node's health counts them.
fn cells(sessions: &Value) -> Value {
    let of = |key: &str| sessions.get(key).and_then(Value::as_u64).unwrap_or(0);
    json!({"busy": of("busy"), "idle": of("idle"), "ended": of("ended")})
}

/// Where a machine meets the network: each device the network names for
/// it, by its band or a wire, through the first point on its way out.
fn places(name: &str, net: &Value) -> Value {
    let devices = net
        .get("devices")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let point = |id: &str| {
        devices
            .iter()
            .find(|device| device.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|device| device.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_owned()
    };
    devices
        .iter()
        .filter(|device| {
            device
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|said| !name.is_empty() && said.eq_ignore_ascii_case(name))
        })
        .map(|device| {
            let eero = device.pointer("/detail/eero");
            let wired = eero.and_then(|eero| eero.get("ethernet")?.as_bool()) == Some(true);
            let radio = |key: &str| {
                (!wired)
                    .then(|| eero?.get(key).cloned())
                    .flatten()
                    .unwrap_or(Value::Null)
            };
            let by = if wired { json!("wired") } else { radio("band") };
            json!({
                "device": device.get("id"),
                "by": by,
                "point": device
                    .pointer("/path/through/0")
                    .and_then(Value::as_str)
                    .map(point),
                "bars": radio("bars"),
                "room_mbps": coarse(device.pointer("/path/down/room_mbps"), 1.0),
            })
        })
        .collect()
}

/// The code a machine holds, counted, when `repos` was read on it; none
/// for another machine, since repos reads only the one it runs on.
fn code(name: &str, repos: &Value) -> Option<Value> {
    if repos.get("machine").and_then(Value::as_str) != Some(name) {
        return None;
    }
    let repositories = repos
        .get("repositories")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let checkouts: Vec<&Value> = repositories
        .iter()
        .flat_map(|repository| listed(repository, "checkouts"))
        .collect();
    let trees = checkouts
        .iter()
        .flat_map(|checkout| listed(checkout, "trees"))
        .filter(|tree| tree.get("present").and_then(Value::as_bool) != Some(false))
        .count();
    let behind = checkouts
        .iter()
        .filter(|checkout| {
            checkout
                .pointer("/head/upstream/count/behind")
                .and_then(Value::as_u64)
                .is_some_and(|behind| behind > 0)
        })
        .count();
    let changes: usize = repositories
        .iter()
        .map(|repository| listed(repository, "changes").len())
        .sum();
    Some(json!({
        "repositories": repositories.len(),
        "checkouts": checkouts.len(),
        "trees": trees,
        "behind": behind,
        "changes": changes,
    }))
}

fn listed<'a>(said: &'a Value, key: &str) -> &'a [Value] {
    said.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A machine's own figures, as the node says them, its bytes in whole
/// gigabytes and its processor's share in tenths, and its names plain, as
/// they reach a terminal; what it didn't say is said as unknown.
fn figures(said: &Value) -> Value {
    let at = |pointer: &str| said.pointer(pointer).cloned().unwrap_or(Value::Null);
    let named = |pointer: &str| match said.pointer(pointer) {
        Some(Value::String(name)) => Value::from(crate::plain::words(name)),
        other => other.cloned().unwrap_or(Value::Null),
    };
    let bytes = |pointer: &str| coarse(said.pointer(pointer), GIGABYTE);
    json!({
        "model": named("/model_name"),
        "system": named("/system"),
        "os": named("/os"),
        "disk": bytes("/disk"),
        "disk_free": bytes("/disk_free"),
        "cores": at("/cpu/cores"),
        "cpu_busy": coarse(said.pointer("/cpu/busy"), 1.0 / TENTHS),
        "memory": bytes("/memory"),
        "memory_available": bytes("/room/memory_available"),
        "charge": at("/power/charge"),
        "on_battery": at("/power/on_battery"),
        "updated_at": at("/updated/at"),
        "update_step": step(said.pointer("/updated/step")),
        "update_why": update_why(said.get("updated").unwrap_or(&Value::Null)),
    })
}

/// Why a machine's last update didn't all go ahead, as its node says it:
/// a refusal's reason; a failure's first error, else its reason, since a
/// failure's reason names only the step that failed and the error is what
/// says why (cadence reads it so); or why a done update's system isn't the
/// one it boots. None for one that went ahead or is still under way, so a
/// reason an earlier attempt left can't stand for this one.
fn update_why(updated: &Value) -> Value {
    let reason = updated.get("why").and_then(Value::as_str).map(uncommanded);
    let reason = reason.as_deref();
    let why = match updated.get("step").and_then(Value::as_str) {
        Some("refused") => reason.and_then(refused_why),
        Some("failed") => failed_why(updated, reason),
        Some("done") => updated
            .get("boots")
            .and_then(Value::as_str)
            .and_then(|boots| plain(&uncommanded(boots))),
        _ => None,
    };
    why.map_or(Value::Null, Value::from)
}

/// Why an update was refused: its reason, whose last clause, after its
/// last colon, is what a person does about it, as a hold's is
/// (update-to-main's update.rs `held`), so it's kept whole beside a long
/// reason's cut.
fn refused_why(reason: &str) -> Option<String> {
    match reason.rsplit_once(": ") {
        Some((refusal, lever)) => beside(refusal, ": ", lever),
        None => plain(reason),
    }
}

/// Why an update failed: its first error with words in it, else its
/// reason. When going back failed too, or there was nothing to go back to,
/// that is said beside the error, and kept whole however long the error,
/// since only then is the machine left off the commit it ran and waiting
/// on a person.
fn failed_why(updated: &Value, reason: Option<&str>) -> Option<String> {
    let stranded = reason.and_then(|reason| {
        STRANDED
            .iter()
            .find_map(|left| reason.find(left).map(|at| reason.split_at(at)))
    });
    match (first_error(updated), stranded) {
        (Some(error), Some((_, left))) => beside(error, "; ", left),
        (Some(error), None) => plain(error),
        (None, Some((failure, left))) => beside(failure, "; ", left),
        (None, None) => reason.and_then(plain),
    }
}

/// How update-to-main says a failed switch left the machine on no system
/// it ran, which a reason carries after the failure itself.
const STRANDED: [&str; 2] = ["going back failed too", "there was no system to go back to"];

/// The first error in a failed update's lines with words in it, passing
/// over each that is only a piece waiting on one that failed: the lines
/// start at the first error that isn't (update-to-main's run.rs `tail`),
/// but when every error is one they are the last lines, and none says why.
fn first_error(updated: &Value) -> Option<&str> {
    let lines: Vec<&str> = updated
        .get("lines")
        .and_then(Value::as_array)
        .map(|lines| lines.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let under = |line: &str| line.starts_with(char::is_whitespace);
    // Whether the error the line at hand belongs to is a waiting piece's:
    // its own line, or one of those indented under it, says so.
    let mut waits = false;
    for (at, &line) in lines.iter().enumerate() {
        if !under(line) {
            waits = is_waiting(line)
                || lines[at + 1..]
                    .iter()
                    .take_while(|next| under(next))
                    .any(|next| is_waiting(next));
        }
        if let Some(error) = line.trim().strip_prefix("error:")
            && !waits
            && plain(error).is_some()
        {
            return Some(error);
        }
    }
    None
}

/// Whether a line of a Nix error says its piece failed only because one
/// it needs did, in the words of Nix before 2.29 and after, as
/// update-to-main's run.rs `is_waiting` tells it.
fn is_waiting(line: &str) -> bool {
    let line = line.trim();
    (line.starts_with("Reason:") && line.contains(" failed") && line.contains("dependenc"))
        || (line.starts_with("error:") && line.contains("dependencies of derivation"))
}

/// How update-to-main names a check by the command it ran (system.rs
/// `boot_once_checked` and `healthy`): a store path, which would spend the
/// why before it reached what the check said.
const CHECKS: [&str; 2] = ["the boot check `", "the health check `"];

/// `why` with each check it names said by its name alone, not its command.
fn uncommanded(why: &str) -> String {
    let mut said = why.to_owned();
    for check in CHECKS {
        let named = check.trim_end_matches([' ', '`']);
        if let Some(at) = said.find(check)
            && let Some(command) = said[at + check.len()..].find('`')
        {
            let after = said[at + check.len() + command + 1..].trim_start();
            let between = if after.is_empty() || after.starts_with([':', ',', ';', '.']) {
                ""
            } else {
                " "
            };
            said = format!("{}{named}{between}{after}", &said[..at]);
        }
    }
    said
}

/// `head` then `tail` as one plain line, the tail whole where it takes no
/// more than half the why and the head cut to leave it room, since the
/// tail says what to do or where the machine was left; a longer tail is
/// cut with the rest, from the end.
fn beside(head: &str, between: &str, tail: &str) -> Option<String> {
    let whole = || plain(&format!("{head}{between}{tail}"));
    let Some(tail) = plain(tail) else {
        return whole();
    };
    let tail_length = tail.chars().count();
    if tail_length > WHY_AT_MOST / 2 {
        return whole();
    }
    // Its clauses are joined as plain joins the rest's.
    let between = between.replace("; ", ", ");
    match cut_to(head, WHY_AT_MOST - tail_length - between.chars().count()) {
        Some(head) => Some(format!("{head}{between}{tail}")),
        None => Some(tail),
    }
}

/// `why` as one plain line of at most `WHY_AT_MOST` characters, or none
/// when nothing is left of it (`cut_to`).
fn plain(why: &str) -> Option<String> {
    cut_to(why, WHY_AT_MOST)
}

/// `why` as one plain line, or none when nothing is left of it: its first
/// line, made plain as every owner's words are (src/plain.rs), since it
/// reaches a terminal; its own clauses joined by commas, since the command
/// joins a machine's clauses with "; "; no closing mark, which the page
/// gives it once; and at most `at_most` characters, cut at a word and
/// marked so.
fn cut_to(why: &str, at_most: usize) -> Option<String> {
    let first = crate::plain::words(why.lines().next().unwrap_or_default()).replace("; ", ", ");
    let said = unended(&first);
    if said.chars().count() <= at_most {
        return (!said.is_empty()).then(|| said.to_owned());
    }
    let cut: String = said.chars().take(at_most.saturating_sub(1)).collect();
    let word = cut.rfind(' ').map_or(cut.as_str(), |at| &cut[..at]);
    Some(format!("{}{CUT}", unended(word)))
}

/// What marks a why cut short.
const CUT: char = '\u{2026}';

/// `words` trimmed of space and of the marks a clause can end on.
fn unended(words: &str) -> &str {
    words.trim().trim_end_matches(['.', ';', ':', ',', ' '])
}

/// The step a machine's last update reached, when it reads as one: a short
/// word of lowercase letters, since it reaches a terminal as it is.
fn step(said: Option<&Value>) -> Value {
    said.and_then(Value::as_str)
        .filter(|step| {
            (1..=STEP_AT_MOST).contains(&step.len())
                && step
                    .chars()
                    .all(|each| each.is_ascii_lowercase() || each == '-')
        })
        .map_or(Value::Null, Value::from)
}

/// The longest a step's name may be.
const STEP_AT_MOST: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    fn laptop() -> Value {
        json!({"name": "laptop", "state": "ready", "system": "linux",
               "os": "NixOS 26.11 (Zokor)", "model_name": "Example 14",
               "cpu": {"cores": 16, "busy": 0.89}, "memory": 66_898_141_184_u64,
               "disk": 1_023_117_623_296_u64, "disk_free": 169_270_915_072_u64,
               "power": {"on_battery": false, "charge": 79},
               "room": {"memory_available": 30_279_519_437_u64, "over": ["memory"]},
               "updated": {"at": "2026-11-17T22:45:18Z", "step": "done"}})
    }

    fn net() -> Value {
        let point = |id: &str, name: &str| json!({"id": id, "name": name, "roles": ["node"]});
        json!({"devices": [
            point("eero:1", "Hall"),
            point("eero:2", "Den"),
            {"id": "mac:a8", "name": "laptop",
             "path": {"through": ["eero:1", "eero:2", "modem"],
                      "down": {"room_mbps": 183.7}},
             "detail": {"eero": {"ethernet": false, "band": "2.4 GHz", "bars": 5,
                                 "signal_dbm": -36}}},
            {"id": "mac:d0", "name": "desk",
             "path": {"through": ["eero:2", "modem"], "down": {"room_mbps": 369.85}},
             "detail": {"eero": {"ethernet": true, "band": null, "bars": 5,
                                 "signal_dbm": 0}}},
            {"id": "mac:ff", "name": "kettle", "path": {"through": ["eero:2"]}},
            {"id": "mac:3c", "name": "Laptop",
             "path": {"through": ["switch:9"], "down": {"room_mbps": 941.2}},
             "detail": {"eero": {"ethernet": true}}},
        ]})
    }

    fn repos() -> Value {
        let tree = |present: bool| json!({"present": present});
        let checkout = |behind: u64, trees: Vec<Value>| {
            json!({"head": {"upstream": {"count": {"ahead": 0, "behind": behind}}},
                   "trees": trees})
        };
        json!({"machine": "laptop", "repositories": [
            {"checkouts": [checkout(100, vec![tree(true), tree(false)]),
                           checkout(0, vec![tree(true)])],
             "changes": [{}, {}]},
            {"checkouts": [{"head": {"upstream": null}, "trees": []}], "changes": [{}]},
        ]})
    }

    #[test]
    fn a_machine_carries_its_cells_as_busy_idle_and_ended() {
        let sessions = json!({"busy": 45, "idle": 27, "ended": 50, "unreachable": 0});
        let ties = ties(&laptop(), Some(&sessions), None, None);
        assert_eq!(ties["cells"], json!({"busy": 45, "idle": 27, "ended": 50}));
        let unread = super::ties(&laptop(), None, None, None);
        assert_eq!(unread["cells"], Value::Null, "unread health says no cells");
        let unlisted = super::ties(&laptop(), Some(&Value::Null), None, None);
        assert_eq!(
            unlisted["cells"],
            json!({"busy": 0, "idle": 0, "ended": 0}),
            "a health read that lists none on it says none"
        );
    }

    #[test]
    fn a_machine_reaches_the_network_through_each_device_named_for_it() {
        let ties = ties(&laptop(), None, Some(&net()), None);
        assert_eq!(
            ties["network"],
            json!([{"device": "mac:a8", "by": "2.4 GHz", "point": "Hall",
                    "bars": 5, "room_mbps": 184},
                   {"device": "mac:3c", "by": "wired", "point": "switch:9",
                    "bars": null, "room_mbps": 941}]),
            "each device named for it, whatever its case, in the network's order, a point \
             the network doesn't list said by its id"
        );
        let mut desk = laptop();
        desk["name"] = json!("desk");
        let wired = super::ties(&desk, None, Some(&net()), None);
        assert_eq!(
            wired["network"],
            json!([{"device": "mac:d0", "by": "wired", "point": "Den",
                    "bars": null, "room_mbps": 370}]),
            "a wire has no bars to say"
        );
        let unread = super::ties(&laptop(), None, None, None);
        assert_eq!(unread["network"], Value::Null, "unread net says no network");
    }

    #[test]
    fn this_machines_code_is_counted_and_anothers_is_not_read_from_here() {
        let ties = ties(&laptop(), None, None, Some(&repos()));
        assert_eq!(
            ties["code"],
            json!({"repositories": 2, "checkouts": 3, "trees": 2, "behind": 1,
                   "changes": 3}),
            "a tree whose folder is gone isn't counted as one"
        );
        let mut studio = laptop();
        studio["name"] = json!("studio");
        let other = super::ties(&studio, None, None, Some(&repos()));
        assert_eq!(
            other["code"],
            Value::Null,
            "repos reads only its own machine"
        );
    }

    fn why_of(updated: &Value) -> Value {
        let mut machine = laptop();
        machine["updated"] = updated.clone();
        super::ties(&machine, None, None, None)["figures"]["update_why"].clone()
    }

    #[test]
    fn an_update_carries_why_only_where_it_didnt_go_ahead() {
        let paused = "updates are paused for the follower, since uid 501 asked at \
                      1763755200; a person's ask still updates, and \
                      `update-to-main resume` lets the follower's run, and no commit is held";
        let joined = "updates are paused for the follower, since uid 501 asked at \
                      1763755200, a person's ask still updates, and \
                      `update-to-main resume` lets the follower's run, and no commit is held";
        assert_eq!(
            why_of(&json!({"step": "refused", "why": paused})),
            json!(joined),
            "a reason's own clauses are kept, joined so they can't read as the machine's"
        );
        let full = "0123456789abcdef0123456789abcdef01234567";
        let held = format!(
            "0123456 is held, and main at 89abcde holds it; `update-to-main lift {full}` lets \
             it through"
        );
        let lever = why_of(&json!({"step": "refused", "why": held}));
        assert!(
            lever.as_str().is_some_and(|why| why.contains(full)),
            "a held commit's lever keeps the full id it needs: {lever}"
        );
        let failed = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                            "lines": ["error: Cannot build '/nix/store/x.drv'.",
                                      "       Reason: builder failed with exit code 1.",
                                      "error: Cannot build '/nix/store/y.drv'.",
                                      "       Reason: 1 dependency failed."]});
        assert_eq!(
            why_of(&failed),
            json!("Cannot build '/nix/store/x.drv'"),
            "a failure says its first error, not the step that failed, and stops once"
        );
        let bare = json!({"step": "failed", "lines": ["switching"],
                          "why": "switching failed (exit status: 2); the machine went back to \
                                  what it ran"});
        assert_eq!(
            why_of(&bare),
            json!("switching failed (exit status: 2), the machine went back to what it ran"),
            "a failure with no error says its reason, and where it left the machine"
        );
        let traced = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                            "lines": ["error:", "       \u{2026} while evaluating 'x'",
                                      "       error: attribute 'x' missing"]});
        assert_eq!(
            why_of(&traced),
            json!("attribute 'x' missing"),
            "an error with nothing on its line gives way to the next that says something"
        );
        let nothing = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                             "lines": ["error:"]});
        assert_eq!(
            why_of(&nothing),
            json!("building the system failed (exit status: 1)"),
            "a failure whose errors say nothing says its reason"
        );
        let stranded = json!({"step": "failed",
                              "why": "switching failed (exit status: 4); going back failed too: \
                                      switching back failed (exit status: 4)",
                              "lines": ["error: unit nix-daemon.service failed",
                                        "error: unit nix-daemon.service failed again"]});
        assert_eq!(
            why_of(&stranded),
            json!(
                "unit nix-daemon.service failed, going back failed too: switching back failed \
                 (exit status: 4)"
            ),
            "a machine left off what it ran says so beside its error"
        );
        let gone = json!({"step": "failed", "lines": ["error: the switch was cut off"],
                          "why": "switching failed (exit status: 4); there was no system to go \
                                  back to"});
        assert_eq!(
            why_of(&gone),
            json!("the switch was cut off, there was no system to go back to"),
            "a machine with nothing to go back to says so beside its error"
        );
        let old = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                         "lines": ["error: builder for '/nix/store/x.drv' failed with exit \
                                    code 1;"]});
        assert_eq!(
            why_of(&old),
            json!("builder for '/nix/store/x.drv' failed with exit code 1"),
            "an error's own closing mark is dropped, so the page's isn't doubled"
        );
    }

    #[test]
    fn a_failed_update_keeps_where_it_left_the_machine_and_passes_over_waiting_pieces() {
        let waiting = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                             "lines": ["error: Cannot build '/nix/store/y.drv'.",
                                       "       Reason: 1 dependency failed.",
                                       "error: 1 dependencies of derivation '/nix/store/z.drv' \
                                        failed to build"]});
        assert_eq!(
            why_of(&waiting),
            json!("building the system failed (exit status: 1)"),
            "an error that is only a piece waiting on another, in either Nix's words, says no \
             why, so the reason is said"
        );
        let long = "x ".repeat(115);
        let lost = json!({"step": "failed", "lines": [format!("error: {long}")],
                          "why": "switching failed (exit status: 4); there was no system to go \
                                  back to"});
        let lost = why_of(&lost);
        let lost = lost.as_str().expect("a long error still says something");
        assert!(
            lost.chars().count() <= WHY_AT_MOST
                && lost.ends_with("x\u{2026}, there was no system to go back to"),
            "a long error is cut before where the machine was left, which is kept: {lost}"
        );
        let unsaid = json!({"step": "failed", "lines": [],
                            "why": format!("switching failed: {long}; going back failed too: \
                                            switching back failed (exit status: 4)")});
        let unsaid = why_of(&unsaid);
        let unsaid = unsaid.as_str().expect("a long reason still says something");
        assert!(
            unsaid.chars().count() <= WHY_AT_MOST
                && unsaid.ends_with(
                    "x\u{2026}, going back failed too: switching back failed (exit status: 4)"
                ),
            "a long reason with no error keeps where the machine was left too: {unsaid}"
        );
        let checked = json!({"step": "failed", "lines": [],
                             "why": "the health check `/nix/store/0123-check/bin/check --now` \
                                     failed (exit status: 1); the machine went back to what it \
                                     ran"});
        assert_eq!(
            why_of(&checked),
            json!("the health check failed (exit status: 1), the machine went back to what it ran"),
            "a health check is said by its name, not the command it ran"
        );
    }

    #[test]
    fn a_boot_check_says_its_cause_and_an_update_with_nothing_to_say_carries_none() {
        let boots = json!({"step": "done", "boots": "the boot check \
                           `/nix/store/0123456789abcdefghijklmnopqrstuv-check/bin/check --now` \
                           failed (exit status: 1), so the machine boots what it booted before: \
                           no loader; bad entry"});
        assert_eq!(
            why_of(&boots),
            json!(
                "the boot check failed (exit status: 1), so the machine boots what it booted \
                 before: no loader, bad entry"
            ),
            "a done update that boots its old system says why, without the check's command"
        );
        let quiet = json!({"step": "done", "boots": "the boot check `check` failed (exit \
                           status: 1), so the machine boots what it booted before: "});
        assert_eq!(
            why_of(&quiet),
            json!(
                "the boot check failed (exit status: 1), so the machine boots what it booted \
                 before"
            ),
            "a check that printed nothing ends its why without a colon"
        );
        let unrun = json!({"step": "done", "boots": "the boot check `x`: No such file or \
                           directory (os error 2), so the machine boots what it booted before: "});
        assert_eq!(
            why_of(&unrun),
            json!(
                "the boot check: No such file or directory (os error 2), so the machine boots \
                 what it booted before"
            ),
            "a check that couldn't run keeps its error beside its name"
        );
        let unbooted = json!({"step": "done", "boots": "making it what the machine boots failed \
                              (exit status: 1), so the machine boots what it booted before: \
                              no `loader` entry"});
        assert_eq!(
            why_of(&unbooted),
            json!(
                "making it what the machine boots failed (exit status: 1), so the machine boots \
                 what it booted before: no `loader` entry"
            ),
            "only a check's command is left out, never words in what it said"
        );
        for (updated, what) in [
            (json!({"step": "done"}), "an update that went ahead"),
            (
                json!({"step": "unchanged", "why": "left over"}),
                "an update with nothing to do",
            ),
            (
                json!({"step": "building", "why": "the Mac is over its room"}),
                "one under way",
            ),
            (
                json!({"step": "refused", "why": " . "}),
                "a reason with nothing in it",
            ),
            (
                json!({"step": "refused", "why": 4}),
                "a reason that isn't words",
            ),
        ] {
            assert_eq!(why_of(&updated), Value::Null, "{what} carries no why");
        }
    }

    #[test]
    fn an_update_step_is_carried_only_when_it_reads_as_one() {
        let step_of = |step: Value| {
            let mut machine = laptop();
            machine["updated"] = json!({"step": step});
            super::ties(&machine, None, None, None)["figures"]["update_step"].clone()
        };
        assert_eq!(
            step_of(json!("refused")),
            json!("refused"),
            "a step is carried"
        );
        for (step, what) in [
            (
                json!("\u{1b}[2Jdone"),
                "a step that would act on a terminal",
            ),
            (json!("Done"), "a step not in update-to-main's form"),
            (
                json!("x".repeat(STEP_AT_MOST + 1)),
                "a step too long to be one",
            ),
            (json!(4), "a step that isn't words"),
        ] {
            assert_eq!(step_of(step), Value::Null, "{what} is not carried");
        }
    }

    #[test]
    fn an_update_why_is_one_plain_line_of_bounded_length() {
        let why = "busy, saying \u{1b}]8;;x\u{7}ok\rdone \u{202e}evil\u{2069}\nmore";
        assert_eq!(
            why_of(&json!({"step": "refused", "why": why})),
            json!("busy, saying ok done evil"),
            "its first line is said, a link's escape gone whole, a return as a space and what \
             would turn the words around gone"
        );
        assert_eq!(
            why_of(&json!({"step": "refused", "why": "busy;\tsaying so"})),
            json!("busy, saying so"),
            "a clause ended by a tab is joined as one ended by a space"
        );
        let path = "/run/update-to-main-hold/given";
        let units = ["a-rather-long-unit-name.service"; 10].join(", ");
        let hold = format!(
            "switching would change {units}, which waits for a hold: once the machine's work is \
             held, make {path} and ask again"
        );
        let cut = why_of(&json!({"step": "refused", "why": hold}));
        let cut = cut.as_str().expect("a long reason still says something");
        assert!(
            cut.chars().count() <= WHY_AT_MOST,
            "a long reason is cut to its bound: {cut}"
        );
        assert!(
            cut.contains(
                "service\u{2026}: once the machine's work is held, make \
                          /run/update-to-main-hold/given and ask again"
            ),
            "a cut is made at a word and marked, before the lever, which is kept: {cut}"
        );
        let short = why_of(&json!({"step": "refused", "why": hold.replace(&units, "x.service")}));
        assert!(
            short.as_str().is_some_and(|why| why.ends_with(
                "make /run/update-to-main-hold/given \
                                                             and ask again"
            )),
            "a hold's lever under the bound is said whole: {short}"
        );
    }

    #[test]
    fn a_machine_carries_its_own_figures() {
        let ties = ties(&laptop(), None, None, None);
        assert_eq!(
            ties["figures"],
            json!({"model": "Example 14", "system": "linux",
                   "os": "NixOS 26.11 (Zokor)",
                   "disk": 1_023_000_000_000_u64, "disk_free": 169_000_000_000_u64,
                   "cores": 16, "cpu_busy": 0.9,
                   "memory": 67_000_000_000_u64, "memory_available": 30_000_000_000_u64,
                   "charge": 79, "on_battery": false,
                   "updated_at": "2026-11-17T22:45:18Z", "update_step": "done",
                   "update_why": null})
        );
        let mut odd = laptop();
        odd["model_name"] = json!("\u{1b}[2JExample\u{202e} 14");
        odd["os"] = json!("NixOS\t26.11");
        let odd = super::ties(&odd, None, None, None);
        assert_eq!(
            (&odd["figures"]["model"], &odd["figures"]["os"]),
            (&json!("Example 14"), &json!("NixOS 26.11")),
            "a machine's names reach a terminal plain"
        );
        let mut mac = laptop();
        mac["updated"] = json!({"at": "2026-11-17T23:00:00Z", "step": "refused",
                                "why": "the Mac is over its room"});
        assert_eq!(
            super::ties(&mac, None, None, None)["figures"]["update_why"],
            json!("the Mac is over its room"),
            "an update put off carries why"
        );
        let bare = super::ties(
            &json!({"name": "spare", "state": "unreachable"}),
            None,
            None,
            None,
        );
        assert_eq!(
            bare["figures"]["disk"],
            Value::Null,
            "what the node didn't say is said as unknown"
        );
    }

    #[test]
    fn a_machine_out_of_reach_says_so_and_when_it_was_last_heard() {
        let mut here = laptop();
        here["measured_at"] = json!("2026-11-23T22:45:09Z");
        let ties = ties(&here, None, None, None);
        assert_eq!(ties["reached"], json!(true));
        assert_eq!(
            ties["heard_at"],
            Value::Null,
            "one in reach is measured every reading, so when isn't said"
        );
        let away = super::ties(
            &json!({"name": "spare", "state": "unreachable",
                    "measured_at": "2026-11-23T19:02:00Z"}),
            None,
            None,
            None,
        );
        assert_eq!(away["reached"], json!(false));
        assert_eq!(away["heard_at"], json!("2026-11-23T19:02:00Z"));
        let never = super::ties(
            &json!({"name": "spare", "state": "unreachable"}),
            None,
            None,
            None,
        );
        assert_eq!(never["reached"], json!(false));
        assert_eq!(
            never["heard_at"],
            Value::Null,
            "one never heard has no time to say"
        );
    }

    #[test]
    fn a_figure_moving_less_than_it_is_said_leaves_the_ties_as_they_were() {
        let mut later = laptop();
        later["disk_free"] = json!(169_301_000_000_u64);
        later["cpu"]["busy"] = json!(0.91);
        later["room"]["memory_available"] = json!(30_112_000_000_u64);
        let mut moved = net();
        moved["devices"][2]["path"]["down"]["room_mbps"] = json!(183.9);
        moved["devices"][2]["detail"]["eero"]["signal_dbm"] = json!(-41);
        assert_eq!(
            ties(&laptop(), None, Some(&net()), None),
            ties(&later, None, Some(&moved), None),
            "a watch is told a machine again only when what it says of it moves"
        );
    }

    #[test]
    fn a_machines_acts_go_through_the_launcher_and_its_commands() {
        let ties = ties(&laptop(), None, None, None);
        assert_eq!(
            ties["acts"],
            json!({"link": "mycelium:machine/laptop?from=system",
                   "commands": ["mycelium sprout --on laptop", "mycelium update laptop"]})
        );
        for misshaped in [
            "",
            "-h",
            ".hidden",
            "lap top",
            "lap;rm",
            "lap/top",
            "\u{e9}t\u{e9}",
        ] {
            let mut machine = laptop();
            machine["name"] = json!(misshaped);
            let refused = super::ties(&machine, None, None, None);
            assert_eq!(
                refused["acts"],
                Value::Null,
                "{misshaped:?} names no machine"
            );
        }
        let long = "a".repeat(65);
        let mut machine = laptop();
        machine["name"] = json!(long);
        assert_eq!(super::ties(&machine, None, None, None)["acts"], Value::Null);
        let nameless = super::ties(&json!({"state": "ready"}), None, None, None);
        assert_eq!(
            nameless["acts"],
            Value::Null,
            "a machine with no name has no acts"
        );
    }
}
