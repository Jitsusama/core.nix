//! Each quest's work ranks by the tier its quest sits in.
//!
//! The machine's config names its tiers in order and the quests that head
//! each one, in a file mapping each head's id to its tier's name. The
//! harness keeps a second file mapping each open quest's id to its
//! parent's, top-level quests left out. A quest is in the tier of the
//! nearest quest, itself first and then up through its parents, that the
//! first file names, so a quest under one tier's head that heads another
//! tier itself ranks by its own. A quest no tier claims is unranked.
//!
//! Under a slice sorted by quest, a tier is a slice of its own,
//! jobs-work-builds.slice under jobs-work.slice, and each quest's slice
//! sits inside its tier's, so the tiers share what agents' work gets by
//! the weights the config gives them, however many quests each has open.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

/// The tier of a quest no tier claims.
pub const UNRANKED: &str = "unranked";

/// The most tiers a machine names, as tiers.nix holds it to, so every
/// tier's work stays above every pi for the OOM killer (`src/oom.rs`).
pub const MOST: usize = 8;

/// Reads a flat JSON object of strings to strings, as both files are.
/// Anything else is no file at all, so a half-written one never moves a
/// quest's work to the wrong tier.
pub fn parse(text: &str) -> Option<HashMap<String, String>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value
        .as_object()?
        .iter()
        .map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
        .collect()
}

/// Whether a tier's name makes a slice name of its own: lowercase letters
/// and digits, since systemd reads a dash as nesting.
pub fn named_plainly(tier: &str) -> bool {
    !tier.is_empty()
        && tier
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// The slice a tier's work goes in under `base`: jobs-work-builds.slice
/// for builds under jobs-work.slice.
pub fn slice_for(base: &str, tier: &str) -> Option<String> {
    let base = base.strip_suffix(".slice")?;
    named_plainly(tier).then(|| format!("{base}-{tier}.slice"))
}

/// The tier `cgroup`'s work sits in under `base`, from the path
/// /proc/PID/cgroup gives: builds for
/// `.../jobs-work.slice/jobs-work-builds.slice/...`.
pub fn tier_in<'a>(cgroup: &'a str, base: &str) -> Option<&'a str> {
    let stem = base.strip_suffix(".slice")?;
    let mut parts = cgroup.trim_end().split('/');
    parts.find(|part| *part == base)?;
    let tier = parts
        .next()?
        .strip_prefix(stem)?
        .strip_prefix('-')?
        .strip_suffix(".slice")?;
    named_plainly(tier).then_some(tier)
}

/// The tiers in the order the machine ranks them, first to last, so the
/// OOM killer can take what's lowest first.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Order(Vec<String>);

impl Order {
    /// Reads `--tier-order`'s names, first to last, split at commas,
    /// refusing more than [`MOST`].
    pub fn new(names: &str) -> Result<Self, String> {
        let tiers: Vec<String> = names
            .split(',')
            .filter(|name| named_plainly(name) && *name != UNRANKED)
            .map(str::to_owned)
            .collect();
        if tiers.len() > MOST {
            return Err(format!(
                "--tier-order names {} tiers, past the {MOST} that keep work above every pi",
                tiers.len()
            ));
        }
        Ok(Self(tiers))
    }

    /// How many ranks `tier` sits below the first: none for the first,
    /// and one past the last for the unranked tier and any it doesn't
    /// name. With no order, every tier is the first.
    pub fn rank(&self, tier: &str) -> usize {
        self.0
            .iter()
            .position(|named| named == tier)
            .unwrap_or(self.0.len())
    }

    /// How many ranks `tier` sits above the unranked tier.
    pub fn above_last(&self, tier: &str) -> usize {
        self.0.len() - self.rank(tier)
    }
}

/// One file as last read.
struct Read {
    path: PathBuf,
    modified: Option<SystemTime>,
    map: HashMap<String, String>,
}

impl Read {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            modified: None,
            map: HashMap::new(),
        }
    }

    /// Takes a file's text, keeping what it had for one it can't wholly
    /// read: that one is most likely mid-write.
    fn take(&mut self, text: &str) {
        if let Some(map) = parse(text) {
            self.map = map;
        }
    }

    fn refresh(&mut self) {
        let modified = std::fs::metadata(&self.path)
            .and_then(|meta| meta.modified())
            .ok();
        if modified.is_none() || modified == self.modified {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&self.path) {
            self.modified = modified;
            self.take(&text);
        }
    }
}

/// The tiers' heads and the quests' parents, as last read, and the tier
/// each quest was last found in, so a move between tiers is said once.
pub struct Tiers {
    heads: Read,
    parents: Read,
    last: HashMap<String, String>,
}

impl Tiers {
    pub fn new(heads: PathBuf, parents: PathBuf) -> Self {
        Self {
            heads: Read::new(heads),
            parents: Read::new(parents),
            last: HashMap::new(),
        }
    }

    /// Takes the heads file's text.
    #[cfg(test)]
    pub fn take_heads(&mut self, text: &str) {
        self.heads.take(text);
    }

    /// Takes the parents file's text.
    #[cfg(test)]
    pub fn take_parents(&mut self, text: &str) {
        self.parents.take(text);
    }

    /// Reads either file again if it changed since it was last read.
    pub fn refresh(&mut self) {
        self.heads.refresh();
        self.parents.refresh();
    }

    /// The tier `quest` is in: that of the nearest quest, itself first and
    /// then each parent up, that heads one. A parent seen twice ends the
    /// walk, so a loop in the harness's file ranks the quest unranked
    /// rather than hanging the sorter.
    pub fn tier_of(&self, quest: &str) -> &str {
        let mut seen = HashSet::new();
        let mut at = quest;
        while seen.insert(at) {
            if let Some(tier) = self.heads.map.get(at).filter(|t| named_plainly(t)) {
                return tier;
            }
            match self.parents.map.get(at) {
                Some(parent) => at = parent,
                None => break,
            }
        }
        UNRANKED
    }

    /// Notes `quest` as found in `tier`, answering the tier it was in
    /// before when that differs, so the move can be said.
    pub fn moved(&mut self, quest: &str, tier: &str) -> Option<String> {
        let before = self.last.insert(quest.to_string(), tier.to_string())?;
        (before != tier).then_some(before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Joel's order as the zone first set it, cut to a few heads.
    const HEADS: &str = r#"{"QEST-B": "builds", "QEST-R": "reviews", "QEST-F": "fleet",
        "QEST-H": "home", "QEST-Z": "lab"}"#;

    /// fleet's head sits under home's, and a subquest under each.
    const PARENTS: &str = r#"{"QEST-F": "QEST-H", "QEST-F1": "QEST-F", "QEST-F2": "QEST-F1",
        "QEST-B1": "QEST-B", "QEST-X1": "QEST-X"}"#;

    fn tiers() -> Tiers {
        let mut tiers = Tiers::new(PathBuf::new(), PathBuf::new());
        tiers.take_heads(HEADS);
        tiers.take_parents(PARENTS);
        tiers
    }

    #[test]
    fn a_head_is_in_its_own_tier() {
        assert_eq!(tiers().tier_of("QEST-B"), "builds");
        assert_eq!(tiers().tier_of("QEST-Z"), "lab");
    }

    #[test]
    fn a_quest_takes_its_nearest_heads_tier() {
        let tiers = tiers();
        assert_eq!(tiers.tier_of("QEST-B1"), "builds");
        assert_eq!(tiers.tier_of("QEST-F2"), "fleet");
        // fleet heads its own tier, though it sits under home's head.
        assert_eq!(tiers.tier_of("QEST-F"), "fleet");
    }

    #[test]
    fn a_quest_no_tier_claims_is_unranked() {
        let tiers = tiers();
        assert_eq!(tiers.tier_of("QEST-X1"), UNRANKED);
        assert_eq!(tiers.tier_of("QEST-NEW"), UNRANKED);
    }

    #[test]
    fn a_loop_among_the_parents_ends_unranked() {
        let mut tiers = tiers();
        tiers.take_parents(r#"{"QEST-L1": "QEST-L2", "QEST-L2": "QEST-L1"}"#);
        assert_eq!(tiers.tier_of("QEST-L1"), UNRANKED);
    }

    #[test]
    fn a_tier_whose_name_makes_no_slice_claims_nothing() {
        let mut tiers = tiers();
        tiers.take_heads(r#"{"QEST-B": "bui-lds", "QEST-B1": "builds"}"#);
        assert_eq!(tiers.tier_of("QEST-B"), UNRANKED);
        assert_eq!(tiers.tier_of("QEST-B1"), "builds");
    }

    #[test]
    fn a_file_it_cannot_wholly_read_keeps_what_it_had() {
        let mut tiers = tiers();
        tiers.take_parents(r#"{"QEST-F1": "QEST-"#);
        tiers.take_heads(r#"{"QEST-B": 3}"#);
        assert_eq!(tiers.tier_of("QEST-F2"), "fleet");
        assert_eq!(tiers.tier_of("QEST-B"), "builds");
    }

    #[test]
    fn only_flat_objects_of_strings_are_read() {
        for text in [
            "",
            "[]",
            "{\"a\": 1}",
            "{\"a\": {\"b\": \"c\"}}",
            "{\"a\": \"b\"} x",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
        assert_eq!(parse("{}").map(|m| m.len()), Some(0));
    }

    #[test]
    fn an_order_ranks_its_tiers_first_to_last_and_every_other_after() {
        let order = Order::new("builds,reviews,fleet").unwrap();
        assert_eq!(order.rank("builds"), 0);
        assert_eq!(order.rank("fleet"), 2);
        assert_eq!(order.rank(UNRANKED), 3, "unranked comes after the last");
        assert_eq!(order.rank("lab"), 3, "a tier it doesn't name is unranked");
        assert_eq!(order.above_last("builds"), 3);
        assert_eq!(order.above_last(UNRANKED), 0);
        let none = Order::default();
        assert_eq!(
            (none.rank(UNRANKED), none.above_last("builds")),
            (0, 0),
            "with no tiers, all work ranks alike"
        );
        assert_eq!(
            Order::new("builds,,Re-views,unranked,fleet"),
            Order::new("builds,fleet"),
            "a name that makes no slice, or unranked's, ranks nothing"
        );
        let too_many = "a,b,c,d,e,f,g,h,i";
        assert!(
            Order::new(too_many).is_err(),
            "nine tiers would put work under a pi"
        );
        assert!(Order::new(&too_many[..15]).is_ok(), "eight is the most");
    }

    #[test]
    fn the_tier_work_sits_in_is_read_from_its_cgroup() {
        let account = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice";
        let work = format!(
            "{account}/jobs-work.slice/jobs-work-builds.slice/\
             jobs-work-builds-qestb.slice/work-7-8.scope\n"
        );
        assert_eq!(tier_in(&work, "jobs-work.slice"), Some("builds"));
        let unranked = format!("{account}/jobs-work.slice/jobs-work-unranked.slice/work-7-8.scope");
        assert_eq!(tier_in(&unranked, "jobs-work.slice"), Some(UNRANKED));
        let untiered = format!("{account}/jobs-work.slice/jobs-work-qestb.slice/work-7-8.scope");
        assert_eq!(
            tier_in(&untiered, "jobs-work.slice"),
            Some("qestb"),
            "a quest's slice reads as a tier the order doesn't name, so unranked"
        );
        assert_eq!(
            tier_in(&format!("{account}/jobs.service"), "jobs-work.slice"),
            None
        );
    }

    #[test]
    fn a_tiers_slice_sits_under_its_base() {
        assert_eq!(
            slice_for("jobs-work.slice", "builds").as_deref(),
            Some("jobs-work-builds.slice")
        );
        assert_eq!(slice_for("jobs-work.slice", "bui-lds"), None);
        assert_eq!(slice_for("jobs-work.slice", "Builds"), None);
        assert_eq!(slice_for("jobs-work.slice", ""), None);
        assert_eq!(slice_for("jobs-work", "builds"), None);
    }

    #[test]
    fn a_move_between_tiers_is_said_once() {
        let mut tiers = tiers();
        assert_eq!(tiers.moved("QEST-F1", "fleet"), None);
        assert_eq!(tiers.moved("QEST-F1", "fleet"), None);
        assert_eq!(tiers.moved("QEST-F1", "builds").as_deref(), Some("fleet"));
        assert_eq!(tiers.moved("QEST-F1", "builds"), None);
    }
}
