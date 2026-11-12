//! Where the broker may put a process, and what it may be asked about:
//! the rules alone, with nothing read, so each case is proven in a unit
//! test.
//!
//! A scope goes only in agents' own work, `jobs-work.slice` or a quest's
//! slice under it, since polkit refuses a prompt to a process there and
//! would ask Joel for one anywhere he works (core's agents-ask-first.rules).
//! It's taken only for a process already an agent's: in `agents.slice`,
//! where every pi runs; in `jobs.slice/jobs.service`, where pi writes each
//! command it starts before the sorter moves it; or already in agents'
//! work.

/// The slice every agent's work runs under, its quests' slices within it.
pub const WORK: &str = "jobs-work.slice";

/// The longest quest a slice is named for, as pi names them
/// (`qest20261025abcdef`, a quest's id lowercased, its dashes gone).
const QUEST_MOST: usize = 40;

/// The longest unit name systemd takes (`UNIT_NAME_MAX`).
const UNIT_MOST: usize = 255;

/// The kinds of unit a status may be asked of, by their names' ends.
const UNIT_KINDS: &[&str] = &[
    ".service", ".socket", ".timer", ".scope", ".slice", ".target", ".path", ".mount",
];

/// Where a process asking for a scope runs, among the places the broker
/// takes one from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// In `agents.slice`, where pi itself runs and a command it started
    /// sits until it's moved.
    Agent,
    /// In the holder pi writes each command into, `jobs.slice/jobs.service`.
    Holder,
    /// Already in agents' work, in a quest's slice when it names one.
    Work { quest: Option<String> },
}

/// Where a process runs, from its `/proc/<pid>/cgroup`, when it's a place
/// a scope may be made from; none for anywhere else, Joel's own programs
/// and terminals above all, or a process under no user manager.
pub fn origin(cgroup: &str) -> Option<Origin> {
    let path = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim();
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let manager = parts
        .iter()
        .position(|part| part.starts_with("user@") && part.ends_with(".service"))?;
    match &parts[manager + 1..] {
        ["agents.slice", _, ..] => Some(Origin::Agent),
        ["jobs.slice", "jobs.service", ..] => Some(Origin::Holder),
        ["jobs.slice", "jobs-work.slice", rest @ ..] => Some(Origin::Work {
            quest: rest.first().and_then(|slice| {
                let quest = slice.strip_prefix("jobs-work-")?.strip_suffix(".slice")?;
                is_quest(quest).then(|| quest.to_owned())
            }),
        }),
        _ => None,
    }
}

/// Whether a quest is named as its slice may be: lowercase letters and
/// digits only, so a quest can't name another slice or climb out of
/// agents' work.
pub fn is_quest(quest: &str) -> bool {
    !quest.is_empty()
        && quest.len() <= QUEST_MOST
        && quest
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

/// The quest a scope is for: the one the process already runs under, which
/// it can't choose, or else the one it asks for.
pub fn quest_for(origin: &Origin, asked: Option<&str>) -> Option<String> {
    match origin {
        Origin::Work { quest: Some(quest) } => Some(quest.clone()),
        _ => asked.map(str::to_owned),
    }
}

/// The slice a scope for this quest goes in.
pub fn slice_for(quest: Option<&str>) -> String {
    quest.map_or_else(|| WORK.to_owned(), |quest| format!("jobs-work-{quest}.slice"))
}

/// The scope made for this process, named so polkit's rules know it as an
/// agent's wherever it is, and by its pid, which is in no other scope
/// while it lives.
pub fn scope_for(quest: Option<&str>, pid: u32) -> String {
    format!("pi-jobs-{}-{pid}.scope", quest.unwrap_or("work"))
}

/// Whether a name is a unit's, of a kind a status may be asked of, and so
/// can't be taken as anything else on the bus.
pub fn is_unit(name: &str) -> bool {
    let named = name.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.' | b'\\' | b'@' | b'-')
    });
    let kind = UNIT_KINDS.iter().find(|kind| name.ends_with(*kind));
    named
        && name.len() <= UNIT_MOST
        && !name.starts_with(['-', '.'])
        && kind.is_some_and(|kind| name.len() > kind.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANAGER: &str = "0::/user.slice/user-1000.slice/user@1000.service";

    fn at(path: &str) -> String {
        format!("{MANAGER}/{path}\n")
    }

    #[test]
    fn an_agent_its_holder_and_its_work_may_ask() {
        assert_eq!(
            origin(&at("agents.slice/pi-85pkb2.scope")),
            Some(Origin::Agent)
        );
        assert_eq!(
            origin(&at("jobs.slice/jobs.service")),
            Some(Origin::Holder)
        );
        assert_eq!(
            origin(&at(
                "jobs.slice/jobs-work.slice/jobs-work-qest2026102585pkb2.slice/work-1.scope"
            )),
            Some(Origin::Work {
                quest: Some("qest2026102585pkb2".to_owned())
            })
        );
        assert_eq!(
            origin(&at("jobs.slice/jobs-work.slice/work-2.scope")),
            Some(Origin::Work { quest: None })
        );
    }

    #[test]
    fn joels_own_places_may_not() {
        for path in [
            "app.slice/app-kitty-1.scope",
            "session.slice/niri.service",
            "jobs.slice/jobs-awaited.slice/awaited.service",
            "jobs.slice/jobs-timed.slice/timed.service",
            "shared.slice/run-1.scope",
            "agents.slice",
            "jobs.slice",
        ] {
            assert_eq!(origin(&at(path)), None, "{path} isn't an agent's");
        }
        assert_eq!(origin("0::/system.slice/sshd.service\n"), None, "no user manager");
        assert_eq!(origin(""), None, "no cgroup at all");
    }

    #[test]
    fn a_quest_names_only_a_slice_of_agents_work() {
        assert!(is_quest("qest2026102585pkb2"));
        for quest in ["", "QEST", "a-b", "../x", "a.slice", "a b", &"a".repeat(41)] {
            assert!(!is_quest(quest), "{quest:?} could name another slice");
        }
        assert_eq!(slice_for(Some("q1")), "jobs-work-q1.slice");
        assert_eq!(slice_for(None), WORK);
    }

    #[test]
    fn work_already_under_a_quest_keeps_it_whatever_it_asks() {
        let under = Origin::Work {
            quest: Some("kept".to_owned()),
        };
        assert_eq!(quest_for(&under, Some("other")), Some("kept".to_owned()));
        assert_eq!(quest_for(&Origin::Agent, Some("asked")), Some("asked".to_owned()));
        assert_eq!(quest_for(&Origin::Holder, None), None);
    }

    #[test]
    fn a_scope_is_named_as_polkit_knows_an_agents() {
        assert_eq!(scope_for(Some("q1"), 42), "pi-jobs-q1-42.scope");
        assert_eq!(scope_for(None, 42), "pi-jobs-work-42.scope");
    }

    #[test]
    fn a_status_is_asked_only_of_a_units_name() {
        for name in ["jobs-sorter.service", "pi-host@a.service", "x-2d.slice", "a\\x2db.scope"] {
            assert!(is_unit(name), "{name} is a unit");
        }
        for name in [
            "",
            ".service",
            "-a.service",
            "a",
            "a.exe",
            "a b.service",
            "/org/freedesktop.service",
            "a;rm.service",
        ] {
            assert!(!is_unit(name), "{name:?} isn't a unit");
        }
        assert!(!is_unit(&format!("{}.service", "a".repeat(250))));
    }
}
