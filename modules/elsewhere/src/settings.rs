//! What elsewhere is told: the machines it may send to, the first taken
//! by `--there`, the folder under each machine's home its copies run in, and
//! each kind of job it may send. A machine repository writes them, through
//! home.nix, to `$XDG_CONFIG_HOME/elsewhere/settings.json`, since which
//! machines a realm holds and what each needs to run a kind are its
//! business, never core's.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How much of a job's tree goes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Copies {
    /// The folder it runs in, and what's under it.
    Folder,
    /// The repository holding that folder, from its top.
    Repository,
}

/// One kind of job, opted in by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub description: String,
    /// The programs a job of this kind may start, by name: on the far
    /// machine each is found on its own path, since a path here names a
    /// build for this machine's processor.
    pub programs: Vec<String>,
    /// A shell line run on the far machine before the job, such as one
    /// putting the kind's programs on its path.
    pub setup: String,
    pub copies: Copies,
    /// What of the tracked files doesn't travel: a path from the tree's
    /// top when it starts with `/`, else a name at any depth.
    pub excludes: Vec<String>,
    /// What's copied back once the job ends, relative to its folder.
    pub brings_back: Vec<String>,
    /// What a machine must be to take a job of it, as `mycelium place
    /// --need` reads it, such as `system=darwin`.
    pub needs: Vec<String>,
    /// How long its jobs run, as `mycelium place --lasts` reads it, so
    /// none goes to a machine leaving sooner.
    pub lasts: Option<String>,
}

/// Everything elsewhere is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The machines it may send to; a placement on any other stays here.
    /// `--there` takes the first.
    pub machines: Vec<String>,
    /// The folder within the quest's scratch on a far machine that its
    /// copies run in: a strict fence there writes only the quest's
    /// scratch, so a copy anywhere else in its home can't be made. Hidden
    /// by default, so no share carries a copy home.
    pub scratch: String,
    /// The realm mycelium is asked about, or the one it's set to.
    pub realm: Option<String>,
    pub kinds: BTreeMap<String, Kind>,
    /// The mycelium it asks and sends by, on its path unless named.
    pub mycelium: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            machines: Vec::new(),
            scratch: ".elsewhere-runs".to_owned(),
            realm: None,
            kinds: BTreeMap::new(),
            mycelium: "mycelium".to_owned(),
        }
    }
}

/// Where the settings are read from, given the environment's variables.
pub fn file(config_home: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match (config_home.filter(|it| !it.is_empty()), home) {
        (Some(config), _) => PathBuf::from(config),
        (None, Some(home)) => Path::new(home).join(".config"),
        (None, None) => return None,
    };
    Some(base.join("elsewhere").join("settings.json"))
}

/// The settings a JSON text gives, or what's wrong with it. A setting
/// left out takes its default, so a file naming one kind is enough.
pub fn read(text: &str) -> Result<Settings, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut settings = Settings::default();
    if let Some(machines) = value.get("machines") {
        settings.machines = strings(machines, "machines")?;
    }
    for (key, slot) in [
        ("scratch", &mut settings.scratch),
        ("mycelium", &mut settings.mycelium),
    ] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            text.clone_into(slot);
        }
    }
    if !one_folder(&settings.scratch) {
        return Err(format!(
            "scratch is one folder's name in the quest's scratch, other than the handover, not {:?}",
            settings.scratch
        ));
    }
    settings.realm = value
        .get("realm")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(kinds) = value.get("kinds").and_then(Value::as_object) {
        for (name, kind) in kinds {
            settings.kinds.insert(name.clone(), self::kind(name, kind)?);
        }
    }
    Ok(settings)
}

fn kind(name: &str, value: &Value) -> Result<Kind, String> {
    let list = |key: &str| {
        value.get(key).map_or(Ok(Vec::new()), |list| {
            strings(list, &format!("{name}'s {key}"))
        })
    };
    let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default();
    let copies = match text("copies") {
        "" | "folder" => Copies::Folder,
        "repository" => Copies::Repository,
        other => {
            return Err(format!(
                "{name}'s copies is folder or repository, not {other}"
            ));
        }
    };
    let programs = list("programs")?;
    if programs.is_empty() {
        return Err(format!("{name} names no program a job of it may start"));
    }
    Ok(Kind {
        description: text("description").to_owned(),
        programs,
        setup: text("setup").to_owned(),
        copies,
        excludes: list("excludes")?,
        brings_back: list("brings_back")?,
        needs: list("needs")?,
        lasts: Some(text("lasts"))
            .filter(|it| !it.is_empty())
            .map(str::to_owned),
    })
}

impl Kind {
    /// The first path this kind brings back that names nothing that could
    /// come home, since the far side passes over hidden files and the near
    /// side lands nothing outside the folder; or one the far side's `find`
    /// would read as part of its expression. Asked as a job goes far, so
    /// one kind set wrongly leaves the others, and its own jobs here, to
    /// run.
    pub fn odd_brings_back(&self) -> Option<&str> {
        self.brings_back
            .iter()
            .map(String::as_str)
            .find(|path| !plain(path))
    }
}

/// Whether `path`, with its slashes at either end taken off, names some
/// place within a folder by plain names alone, which `find` reads as a
/// path.
fn plain(path: &str) -> bool {
    let path = path.trim_matches('/');
    !path.starts_with(['-', '!', '(', ')', ','])
        && !path.is_empty()
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

/// Whether `name` names one folder within the quest's scratch, and not
/// the handover: a job's copy is emptied before it's made, so it must
/// never land on the files that crossed for the job, or outside the quest.
fn one_folder(name: &str) -> bool {
    let handover = crate::far::HANDOVER
        .split('/')
        .next()
        .unwrap_or_default();
    !name.is_empty()
        && !name.contains('/')
        && !name.starts_with('-')
        && !matches!(name, "." | "..")
        && name != handover
}

fn strings(value: &Value, what: &str) -> Result<Vec<String>, String> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|it| it.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| format!("{what} is a list of words"))
}

#[cfg(test)]
mod tests {
    use super::{Copies, file, read};
    use std::path::PathBuf;

    #[test]
    fn a_kind_left_short_takes_the_defaults() {
        let settings =
            read(r#"{"machines": ["far"], "kinds": {"check": {"programs": ["tla-rs"]}}}"#)
                .expect("a kind naming its program alone was refused");
        let check = &settings.kinds["check"];
        assert_eq!(
            check.copies,
            Copies::Folder,
            "a kind didn't copy its folder by default"
        );
        assert!(check.setup.is_empty(), "a kind had a setup it wasn't given");
        assert!(
            check.needs.is_empty() && check.lasts.is_none(),
            "a kind asked mycelium for what it wasn't given"
        );
        assert_eq!(
            settings.scratch, ".elsewhere-runs",
            "the scratch didn't default"
        );
        assert_eq!(
            settings.mycelium, "mycelium",
            "mycelium wasn't the one on the path"
        );
    }

    #[test]
    fn a_kind_that_starts_nothing_or_copies_oddly_is_refused() {
        for (text, says) in [
            (r#"{"kinds": {"check": {}}}"#, "names no program"),
            (
                r#"{"kinds": {"check": {"programs": ["x"], "copies": "disk"}}}"#,
                "not disk",
            ),
            (r#"{"machines": "far"}"#, "machines is a list"),
            ("{", "EOF"),
        ] {
            let problem = read(text).expect_err(&format!("{text} was taken"));
            assert!(
                problem.contains(says),
                "{text} was refused as {problem}, not {says}"
            );
        }
    }

    #[test]
    fn a_scratch_that_isn_t_one_folder_of_the_quest_s_own_is_refused() {
        for scratch in ["", ".", "..", "../x", "a/b", "/runs/", "-x", "handover", "handover/elsewhere"] {
            let text = serde_json::json!({ "scratch": scratch }).to_string();
            let problem = read(&text).expect_err(&format!("the scratch {scratch:?} was taken"));
            assert!(
                problem.contains("one folder's name"),
                "the scratch {scratch:?} was refused as {problem}"
            );
        }
        for scratch in [".elsewhere-runs", "runs"] {
            let text = serde_json::json!({ "scratch": scratch }).to_string();
            assert_eq!(
                read(&text).map(|settings| settings.scratch),
                Ok(scratch.to_owned()),
                "the scratch {scratch:?} wasn't taken"
            );
        }
    }

    #[test]
    fn a_kind_bringing_back_what_can_t_come_home_is_refused_alone() {
        let settings = read(
            r#"{"kinds": {
                "hidden": {"programs": ["x"], "brings_back": ["out", "out/.cache"]},
                "up": {"programs": ["x"], "brings_back": ["../up"]},
                "option": {"programs": ["x"], "brings_back": ["-delete"]},
                "not": {"programs": ["x"], "brings_back": ["!"]},
                "open": {"programs": ["x"], "brings_back": ["(out"]},
                "shut": {"programs": ["x"], "brings_back": [")"]},
                "comma": {"programs": ["x"], "brings_back": [",out"]},
                "empty": {"programs": ["x"], "brings_back": ["/"]},
                "fine": {"programs": ["x"], "brings_back": ["/out/", "a/b"]}
            }}"#,
        )
        .expect("one kind set wrongly stopped the settings loading");
        for (name, odd) in [
            ("hidden", Some("out/.cache")),
            ("up", Some("../up")),
            ("option", Some("-delete")),
            ("not", Some("!")),
            ("open", Some("(out")),
            ("shut", Some(")")),
            ("comma", Some(",out")),
            ("empty", Some("/")),
            ("fine", None),
        ] {
            assert_eq!(
                settings.kinds[name].odd_brings_back(),
                odd,
                "{name}'s brings_back was judged wrongly"
            );
        }
    }

    #[test]
    fn the_settings_live_in_the_config_home() {
        assert_eq!(
            file(Some("/c"), Some("/h")),
            Some(PathBuf::from("/c/elsewhere/settings.json")),
            "XDG_CONFIG_HOME wasn't read first"
        );
        assert_eq!(
            file(Some(""), Some("/h")),
            Some(PathBuf::from("/h/.config/elsewhere/settings.json")),
            "an empty XDG_CONFIG_HOME wasn't passed over"
        );
    }
}
