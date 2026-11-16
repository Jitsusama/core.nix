//! Each part judged from its owner's own answer, as that owner's JSON
//! says it. An answer is a `Result`: the owner's record, or why it couldn't
//! be had, which makes the part not read. Every field is read leniently,
//! since an owner adds fields freely and a newer one may say a word this
//! build doesn't know: an unknown word costs the part that word, never the
//! part.

use serde_json::Value;

use crate::record::{Deeper, Health, Part};

mod machines;

pub use machines::machines;

/// An owner's answer: its record, or why it couldn't be had.
pub type Answer = Result<Value, String>;

/// The most items a side says of a part, so thousands of machines worth a
/// look still make a short list: past it, the last says how many more.
const SIDE_ITEMS: usize = 3;

/// The most machines a side names in its links down; past it, it counts
/// them, so no name is ever cut.
const LINKS_NAMED: usize = 2;

/// The mycelium, from `mycelium health`: its links, its sessions, and the
/// presences left behind by sessions whose process has gone, which it
/// holds rather than judges.
pub fn mycelium(health: &Answer) -> Part {
    let deeper = Deeper {
        command: "mycelium networks",
        site: "mycelium/",
        launcher: "Agents",
    };
    let said = match health {
        Ok(said) => said,
        Err(why) => return unread("mycelium", why, deeper),
    };
    let mut worst = Health::Good;
    let mut words = Vec::new();
    let mut side = Vec::new();
    let links = said
        .get("links")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let down: Vec<&str> = links
        .iter()
        .filter(|link| link.get("state").and_then(Value::as_str) != Some("up"))
        .filter_map(|link| link.get("machine").and_then(Value::as_str))
        .collect();
    if !down.is_empty() {
        worst = Health::Bad;
        words.push(format!("the link to {} is down", and(&down)));
        side.push(match down.len() {
            1 => format!("link to {} down", down[0]),
            named if named <= LINKS_NAMED => format!("links to {} down", and(&down)),
            many => format!("{many} links down"),
        });
    }
    let dead = said
        .get("deadPresences")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    // What it holds, its sessions and links, is for the drawer.
    let machines = said.get("sessions").and_then(Value::as_object);
    let of = |key: &str| {
        machines.map_or(0, |machines| {
            machines
                .values()
                .map(|one| one.get(key).and_then(Value::as_u64).unwrap_or(0))
                .sum::<u64>()
        })
    };
    let (busy, idle) = (of("busy"), of("idle"));
    let across = machines.map_or(0, serde_json::Map::len);
    let mut held = vec![format!(
        "{busy} busy and {idle} idle sessions on {}",
        count(across as u64, "machine", "machines")
    )];
    // A session whose process has gone is an agent's leftover, as a gone
    // tree is, which Joel can't act on from Agents: it's held, never a
    // look.
    if dead > 0 {
        held.push(count(dead, "session gone", "sessions gone"));
    }
    if down.is_empty() && !links.is_empty() {
        held.push(format!("{} up", count(links.len() as u64, "link", "links")));
    }
    let holds = held.join("; ");
    words.extend(held);
    Part {
        name: "mycelium",
        health: worst,
        says: words.join("; "),
        side: side_of(side),
        holds: Some(holds),
        things: Vec::new(),
        deeper,
    }
}

/// The home network, from `net --json`: its sources, then what net finds
/// worth a look, in net's own words. The internet as the narrowest point
/// is how the network is, not something wrong, so it's left to net.
pub fn network(net: &Answer) -> Part {
    let deeper = Deeper {
        command: "net",
        site: "network/",
        launcher: "Network",
    };
    let said = match net {
        Ok(said) => said,
        Err(why) => return unread("network", why, deeper),
    };
    let mut worst = Health::Good;
    let mut words = Vec::new();
    let silent: Vec<&str> = said
        .get("sources")
        .and_then(Value::as_array)
        .map(|sources| {
            sources
                .iter()
                .filter(|source| source.get("state").and_then(Value::as_str) != Some("answering"))
                .filter_map(|source| source.get("name").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if !silent.is_empty() {
        worst = Health::Bad;
        let isnt = if silent.len() == 1 { "isn't" } else { "aren't" };
        words.push(format!("{} {isnt} answering", and(&silent)));
    }
    let looks: Vec<&str> = said
        .get("concerns")
        .and_then(Value::as_array)
        .map(|concerns| {
            concerns
                .iter()
                .filter(|concern| concern.get("kind").and_then(Value::as_str) != Some("ceiling"))
                .filter_map(|concern| concern.get("says").and_then(Value::as_str))
                .map(unstopped)
                .collect()
        })
        .unwrap_or_default();
    let mut side = words.clone();
    if let Some(first) = looks.first() {
        worst = worst.max(Health::Watch);
        words.push(match looks.len() {
            1 => (*first).to_owned(),
            more => format!("{first}; and {} more worth a look", more - 1),
        });
        // Net words its first look best, and orders them so.
        side.push(lowered(first));
        if looks.len() > 1 {
            side.push(format!("and {} more worth a look", looks.len() - 1));
        }
    }
    let devices = said
        .get("devices")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let holds = count(devices as u64, "device", "devices");
    words.push(holds.clone());
    Part {
        name: "network",
        health: worst,
        says: words.join("; "),
        side: side_of(side),
        holds: Some(holds),
        things: Vec::new(),
        deeper,
    }
}

/// The code, from `repos --json`: its repositories and trees, and the trees
/// still listed whose folders have gone. Those are agents' leftovers,
/// which the work tool's tidy clears and Joel can't act on from the code,
/// so they are a fact of what the code holds and never worth his look.
pub fn code(repos: &Answer) -> Part {
    let deeper = Deeper {
        command: "repos",
        site: "code/",
        launcher: "Code",
    };
    let said = match repos {
        Ok(said) => said,
        Err(why) => return unread("code", why, deeper),
    };
    let repositories = said
        .get("repositories")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let trees: Vec<Value> = repositories
        .iter()
        .flat_map(|repository| array(repository, "checkouts"))
        .flat_map(|checkout| array(&checkout, "trees"))
        .collect();
    let gone = trees
        .iter()
        .filter(|tree| tree.get("present").and_then(Value::as_bool) == Some(false))
        .count();
    let gone = match gone {
        0 => String::new(),
        1 => ", 1 with its folder gone".to_owned(),
        many => format!(", {many} with their folders gone"),
    };
    let holds = format!(
        "{} with {}{gone}",
        count(repositories.len() as u64, "repository", "repositories"),
        count(trees.len() as u64, "tree", "trees")
    );
    Part {
        name: "code",
        health: Health::Good,
        says: holds.clone(),
        side: Vec::new(),
        holds: Some(holds),
        things: Vec::new(),
        deeper,
    }
}

fn array(said: &Value, key: &str) -> Vec<Value> {
    said.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn unread(name: &'static str, why: &str, deeper: Deeper) -> Part {
    Part {
        name,
        health: Health::Unread,
        says: why.to_owned(),
        side: vec![lowered(why)],
        holds: None,
        things: Vec::new(),
        deeper,
    }
}

/// A side's items, at most `SIDE_ITEMS` of them, the last saying how many
/// more there were when there were.
fn side_of(mut items: Vec<String>) -> Vec<String> {
    if items.len() > SIDE_ITEMS {
        let more = items.len() - (SIDE_ITEMS - 1);
        items.truncate(SIDE_ITEMS - 1);
        items.push(format!("and {more} more worth a look"));
    }
    items
}

/// An owner's words as an item of a side, which starts lower case unless
/// a name leads. An owner starts a sentence with "The", which is no name;
/// any other capital is taken as one, since only the owner knows.
fn lowered(said: &str) -> String {
    match said.strip_prefix("The ") {
        Some(rest) => format!("the {rest}"),
        None => said.to_owned(),
    }
}

/// An owner's sentence without its one closing stop, so it joins others.
/// Net words each look as a sentence, and the join supplies the stop.
fn unstopped(said: &str) -> &str {
    said.strip_suffix('.').unwrap_or(said)
}

/// A count and its noun, in the singular for one.
fn count(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Words joined as a sentence lists them: a, b and c.
fn and(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A side with nothing worth a look on it.
    pub(super) const NO_LOOK: [&str; 0] = [];

    /// What `mycelium health` answers for a laptop linked to a studio.
    pub(super) fn said_health() -> Value {
        json!({"links": [{"machine": "studio", "state": "up"}],
               "sessions": {"laptop": {"busy": 55, "idle": 33, "ended": 27},
                            "studio": {"busy": 2, "idle": 1, "ended": 0}},
               "deadPresences": 0})
    }

    #[test]
    fn an_owner_that_fails_leaves_its_part_not_read_with_why() {
        let part = machines(&Err("mycelium isn't installed".into()), &Ok(said_health()));
        assert_eq!(part.health, Health::Unread);
        assert_eq!(part.says, "mycelium isn't installed");
        assert_eq!(
            part.side,
            ["mycelium isn't installed"],
            "why is worth a look"
        );
        assert_eq!(
            network(&Err("net said nothing".into())).health,
            Health::Unread
        );
        assert_eq!(code(&Err("repos timed out".into())).says, "repos timed out");
        assert_eq!(part.holds, None, "a part not read holds nothing it can say");
    }

    #[test]
    fn the_mycelium_says_its_sessions_and_its_links() {
        let part = mycelium(&Ok(said_health()));
        assert_eq!(part.health, Health::Good);
        assert_eq!(
            part.says,
            "57 busy and 34 idle sessions on 2 machines; 1 link up"
        );
        assert_eq!(part.side, NO_LOOK, "what it holds is no look");
        assert_eq!(
            part.holds.as_deref(),
            Some("57 busy and 34 idle sessions on 2 machines; 1 link up"),
            "what it holds is said apart"
        );
    }

    #[test]
    fn a_link_down_is_bad_and_a_dead_session_only_held() {
        let mut said = said_health();
        said["deadPresences"] = json!(7);
        let held = mycelium(&Ok(said.clone()));
        assert_eq!(
            held.health,
            Health::Good,
            "a dead session is an agent's leftover, which Joel can't act on from Agents"
        );
        assert_eq!(held.side, NO_LOOK);
        assert_eq!(
            held.holds.as_deref(),
            Some("57 busy and 34 idle sessions on 2 machines; 7 sessions gone; 1 link up")
        );
        said["links"][0]["state"] = json!("unreachable");
        let part = mycelium(&Ok(said));
        assert_eq!(part.health, Health::Bad);
        assert!(
            part.says.starts_with("the link to studio is down; 57 busy"),
            "{}",
            part.says
        );
        assert_eq!(
            part.side,
            ["link to studio down"],
            "a side is worded for a row, its verb dropped"
        );
    }

    #[test]
    fn links_down_are_named_up_to_two_and_counted_past_them() {
        let down = |machines: &[&str]| {
            let links: Vec<Value> = machines
                .iter()
                .map(|machine| json!({"machine": machine, "state": "unreachable"}))
                .collect();
            mycelium(&Ok(json!({"links": links}))).side
        };
        assert_eq!(
            down(&["studio", "skiff"]),
            ["links to studio and skiff down"]
        );
        assert_eq!(
            down(&["studio", "skiff", "spare"]),
            ["3 links down"],
            "no name is ever cut"
        );
    }

    fn net(states: &[(&str, &str)], concerns: &[(&str, &str)]) -> Value {
        json!({
            "sources": states.iter().map(|(name, state)| json!({"name": name, "state": state}))
                .collect::<Vec<_>>(),
            "concerns": concerns.iter().map(|(kind, says)| json!({"kind": kind, "says": says}))
                .collect::<Vec<_>>(),
            "devices": [{}, {}, {}],
        })
    }

    #[test]
    fn the_network_says_nets_first_look_in_its_words() {
        let part = network(&Ok(net(
            &[("eero", "answering")],
            &[
                ("ceiling", "The internet is the narrowest point"),
                ("busy", "Main Floor's radio is busy"),
                ("weak", "iphone has a weak signal"),
            ],
        )));
        assert_eq!(part.health, Health::Watch);
        assert_eq!(
            part.says, "Main Floor's radio is busy; and 1 more worth a look; 3 devices",
            "the internet's ceiling is how the network is, not a look"
        );
        assert_eq!(
            part.side,
            ["Main Floor's radio is busy", "and 1 more worth a look"],
            "a name leading keeps its capital, and the devices are left out"
        );
    }

    #[test]
    fn a_look_nets_words_end_is_joined_without_its_stop() {
        let part = network(&Ok(net(
            &[("eero", "answering")],
            &[
                ("stale", "The eero last answered 20 hours ago."),
                ("weak", "iphone has a weak signal."),
            ],
        )));
        assert_eq!(
            part.says, "The eero last answered 20 hours ago; and 1 more worth a look; 3 devices",
            "net's sentence loses its stop where it joins the others"
        );
        assert_eq!(
            part.side,
            [
                "the eero last answered 20 hours ago",
                "and 1 more worth a look"
            ],
            "an item of a side carries no stop"
        );
    }

    #[test]
    fn an_owners_leading_the_starts_a_side_lower_case() {
        let part = network(&Ok(net(
            &[("modem", "unreachable")],
            &[("loss", "The modem's downstream channel 16 has lost 878")],
        )));
        assert_eq!(
            part.side,
            [
                "modem isn't answering",
                "the modem's downstream channel 16 has lost 878"
            ]
        );
    }

    #[test]
    fn a_source_not_answering_is_bad() {
        let part = network(&Ok(net(
            &[
                ("eero", "answering"),
                ("modem", "unreachable"),
                ("hp", "signed-out"),
            ],
            &[],
        )));
        assert_eq!(part.health, Health::Bad);
        assert_eq!(part.says, "modem and hp aren't answering; 3 devices");
        let calm = network(&Ok(net(&[("eero", "answering")], &[("ceiling", "")])));
        assert_eq!(
            (calm.health, calm.says.as_str()),
            (Health::Good, "3 devices")
        );
        assert_eq!(calm.holds.as_deref(), Some("3 devices"));
    }

    #[test]
    fn the_code_counts_its_trees_and_the_gone_ones() {
        let tree = |present: bool| json!({"present": present});
        let said = json!({"repositories": [
            {"checkouts": [{"trees": [tree(true), tree(false)]}, {"trees": [tree(true)]}]},
            {"checkouts": [{"trees": []}]},
        ]});
        let part = code(&Ok(said));
        assert_eq!(
            part.health,
            Health::Good,
            "a gone tree isn't the code's fault"
        );
        assert_eq!(
            part.side, NO_LOOK,
            "a gone tree is an agent's leftover, which Joel can't act on from the code"
        );
        assert_eq!(
            part.holds.as_deref(),
            Some("2 repositories with 3 trees, 1 with its folder gone"),
            "the gone trees are a fact of what it holds, for whoever tidies"
        );
        assert_eq!(
            part.says,
            "2 repositories with 3 trees, 1 with its folder gone"
        );
        let none = code(&Ok(
            json!({"repositories": [{"checkouts": [{"trees": []}]}]}),
        ));
        assert_eq!(none.says, "1 repository with 0 trees");
        assert_eq!(none.side, NO_LOOK, "trees it holds are no look");
    }
}
