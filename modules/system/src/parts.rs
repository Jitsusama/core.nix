//! Each part judged from its owner's own answer, as that owner's JSON
//! says it. An answer is a `Result`: the owner's record, or why it couldn't
//! be had, which makes the part not read. Every field is read leniently,
//! since an owner adds fields freely and a newer one may say a word this
//! build doesn't know: an unknown word costs the part that word, never the
//! part.

use serde_json::Value;

use crate::record::{Deeper, Health, Part, Thing};

/// An owner's answer: its record, or why it couldn't be had.
pub type Answer = Result<Value, String>;

/// A gigabyte, as a disk's free room is said.
const GIGABYTE: f64 = 1e9;
/// A battery at or under this, off the charger, is worth a look.
const LOW_CHARGE: u64 = 20;

/// The parts of another machine's own health this takes in: those only
/// that machine reads. Its machines, mycelium and network are the same
/// whole read from every machine, each already a part here, and taking
/// them in would spread one machine's trouble to all. The first two would
/// also latch, as each machine's word fed the other's.
const ITS_OWN: [&str; 1] = ["code"];

/// The most items a side says of a part, so thousands of machines worth a
/// look still make a short list: past it, the last says how many more.
const SIDE_ITEMS: usize = 3;

/// The machines of the realm, from `mycelium machines`, each with its
/// sessions from `mycelium health` where that was read.
pub fn machines(machines: &Answer, health: &Answer) -> Part {
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
            machine(
                said,
                sessions.and_then(|all| all.get(name_of(said))),
                others,
            )
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

/// One machine: whether a link reaches it, what it's short of, its disk,
/// its battery, what its own health says of its own parts and its
/// sessions, each said only when it's worth saying, with the words of
/// what is worth a look in it.
fn machine(said: &Value, sessions: Option<&Value>, own: Option<&Value>) -> (Thing, Vec<String>) {
    let mut health = Health::Good;
    let mut words = Vec::new();
    if said.get("state").and_then(Value::as_str) == Some("unreachable") {
        health = Health::Bad;
        words.push("is out of reach".to_owned());
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
        health = health.max(Health::Watch);
        words.push(format!("is short of {}", and(&over)));
    }
    let free = said
        .get("disk_free")
        .and_then(Value::as_f64)
        .map(|bytes| format!("{:.0} GB free", bytes / GIGABYTE));
    match said.get("disk_band").and_then(Value::as_str) {
        Some("filling") => {
            health = health.max(Health::Watch);
            words.push(free.map_or("has a filling disk".to_owned(), |free| {
                format!("has a filling disk, {free}")
            }));
        }
        Some("nearly-full" | "critically-full") => {
            health = Health::Bad;
            words.push(free.map_or("has a nearly full disk".to_owned(), |free| {
                format!("has a nearly full disk, {free}")
            }));
        }
        _ => {}
    }
    let on_battery = said.pointer("/power/on_battery").and_then(Value::as_bool) == Some(true);
    if let Some(charge) = said.pointer("/power/charge").and_then(Value::as_u64)
        && on_battery
        && charge <= LOW_CHARGE
    {
        health = health.max(Health::Watch);
        words.push(format!("is on battery at {charge}%"));
    }
    for part in ITS_OWN {
        match own.and_then(|own| own.pointer(&format!("/parts/{part}"))?.as_str()) {
            Some("watch") => {
                health = health.max(Health::Watch);
                words.push(format!("says its {part} wants a look"));
            }
            Some("bad") => {
                health = Health::Bad;
                words.push(format!("says its {part} is failing"));
            }
            _ => {}
        }
    }
    let looks = words.clone();
    if words.is_empty() {
        words.push("is ready".to_owned());
    }
    if let Some(sessions) = sessions {
        words.push(format!("runs {}", sessions_said(sessions)));
    }
    let thing = Thing {
        kind: "machine",
        name: name_of(said).to_owned(),
        health,
        says: words.join(", "),
        address: None,
    };
    (thing, looks)
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

/// The mycelium, from `mycelium health`: its links, its sessions, and the
/// presences left behind by sessions whose process has gone.
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
    }
    let dead = said
        .get("deadPresences")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if dead > 0 {
        worst = worst.max(Health::Watch);
        words.push(count(
            dead,
            "session says it runs with its process gone",
            "sessions say they run with their processes gone",
        ));
    }
    // What it holds, its sessions and links, is for the drawer.
    let side = words.clone();
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
    const NO_LOOK: [&str; 0] = [];

    fn laptop(over: &[&str], band: &str) -> Value {
        json!({"name": "laptop", "state": "ready", "disk_free": 278_819_094_528_u64,
               "disk_band": band, "room": {"over": over},
               "power": {"on_battery": false, "charge": 76}})
    }

    /// What `mycelium machines` answers for these machines.
    fn realm(machines: &[Value]) -> Value {
        json!({"Ok": {"Machines": machines}})
    }

    /// What `mycelium health` answers for a laptop linked to a studio.
    fn said_health() -> Value {
        json!({"links": [{"machine": "studio", "state": "up"}],
               "sessions": {"laptop": {"busy": 55, "idle": 33, "ended": 27},
                            "studio": {"busy": 2, "idle": 1, "ended": 0}},
               "deadPresences": 0})
    }

    #[test]
    fn a_machine_short_of_room_leads_with_what_it_lacks() {
        let studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy",
                            "room": {"over": []}});
        let part = machines(
            &Ok(realm(&[studio, laptop(&["memory", "cpu"], "filling")])),
            &Ok(said_health()),
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
            ["laptop is short of memory and processor, has a filling disk, 279 GB free"],
            "a side leaves out what the machine runs"
        );
    }

    #[test]
    fn a_side_of_many_machines_says_how_many_more() {
        let away = |name: &str| json!({"name": name, "state": "unreachable"});
        let many: Vec<Value> = ["a", "b", "c", "d", "e"].into_iter().map(away).collect();
        let part = machines(&Ok(realm(&many)), &Err(String::new()));
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
        let part = machines(&Ok(realm(std::slice::from_ref(&low))), &Err(String::new()));
        assert_eq!(part.things[0].says, "is on battery at 12%");
        low["power"]["on_battery"] = json!(false);
        let charging = machines(&Ok(realm(&[low])), &Err(String::new()));
        assert_eq!(charging.health, Health::Good);
    }

    #[test]
    fn another_machine_says_what_its_own_code_is() {
        let mut studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy"});
        studio["health"] = json!({"health": "bad", "parts": {"code": "watch", "network": "good"}});
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio.clone()])),
            &Err(String::new()),
        );
        assert_eq!(part.health, Health::Watch, "its code is worth a look");
        assert_eq!(part.things[0].name, "studio");
        assert_eq!(part.things[0].says, "says its code wants a look");
        studio["health"]["parts"]["code"] = json!("bad");
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio])),
            &Err(String::new()),
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
        let part = machines(&Ok(realm(&[here, studio])), &Err(String::new()));
        assert_eq!(
            part.health,
            Health::Good,
            "this machine's code is its own part, and the rest are the same whole \
             read from every machine: {}",
            part.says
        );
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
    fn a_link_down_is_bad_and_a_dead_presence_worth_a_look() {
        let mut said = said_health();
        said["deadPresences"] = json!(7);
        assert_eq!(mycelium(&Ok(said.clone())).health, Health::Watch);
        said["links"][0]["state"] = json!("unreachable");
        let part = mycelium(&Ok(said));
        assert_eq!(part.health, Health::Bad);
        assert!(
            part.says
                .starts_with("the link to studio is down; 7 sessions say"),
            "{}",
            part.says
        );
        assert_eq!(
            part.side,
            [
                "the link to studio is down",
                "7 sessions say they run with their processes gone"
            ]
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
