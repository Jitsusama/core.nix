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
//! jobs-work-walgit.slice under jobs-work.slice, and each quest's slice
//! sits inside its tier's, so the tiers share what agents' work gets by
//! the weights the config gives them, however many quests each has open.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

/// The tier of a quest no tier claims.
pub const UNRANKED: &str = "unranked";

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

/// The slice a tier's work goes in under `base`: jobs-work-walgit.slice
/// for walgit under jobs-work.slice.
pub fn slice_for(base: &str, tier: &str) -> Option<String> {
    let base = base.strip_suffix(".slice")?;
    named_plainly(tier).then(|| format!("{base}-{tier}.slice"))
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
    const HEADS: &str = r#"{"QEST-W": "walgit", "QEST-D": "delta", "QEST-S": "swarm",
        "QEST-O": "optimus", "QEST-A": "aquifer"}"#;

    /// swarm's head sits under optimus's, and a subquest under each.
    const PARENTS: &str = r#"{"QEST-S": "QEST-O", "QEST-S1": "QEST-S", "QEST-S2": "QEST-S1",
        "QEST-W1": "QEST-W", "QEST-X1": "QEST-X"}"#;

    fn tiers() -> Tiers {
        let mut tiers = Tiers::new(PathBuf::new(), PathBuf::new());
        tiers.take_heads(HEADS);
        tiers.take_parents(PARENTS);
        tiers
    }

    #[test]
    fn a_head_is_in_its_own_tier() {
        assert_eq!(tiers().tier_of("QEST-W"), "walgit");
        assert_eq!(tiers().tier_of("QEST-A"), "aquifer");
    }

    #[test]
    fn a_quest_takes_its_nearest_heads_tier() {
        let tiers = tiers();
        assert_eq!(tiers.tier_of("QEST-W1"), "walgit");
        assert_eq!(tiers.tier_of("QEST-S2"), "swarm");
        // swarm heads its own tier, though it sits under optimus's head.
        assert_eq!(tiers.tier_of("QEST-S"), "swarm");
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
        tiers.take_heads(r#"{"QEST-W": "wal-git", "QEST-W1": "walgit"}"#);
        assert_eq!(tiers.tier_of("QEST-W"), UNRANKED);
        assert_eq!(tiers.tier_of("QEST-W1"), "walgit");
    }

    #[test]
    fn a_file_it_cannot_wholly_read_keeps_what_it_had() {
        let mut tiers = tiers();
        tiers.take_parents(r#"{"QEST-S1": "QEST-"#);
        tiers.take_heads(r#"{"QEST-W": 3}"#);
        assert_eq!(tiers.tier_of("QEST-S2"), "swarm");
        assert_eq!(tiers.tier_of("QEST-W"), "walgit");
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
    fn a_tiers_slice_sits_under_its_base() {
        assert_eq!(
            slice_for("jobs-work.slice", "walgit").as_deref(),
            Some("jobs-work-walgit.slice")
        );
        assert_eq!(slice_for("jobs-work.slice", "wal-git"), None);
        assert_eq!(slice_for("jobs-work.slice", "Walgit"), None);
        assert_eq!(slice_for("jobs-work.slice", ""), None);
        assert_eq!(slice_for("jobs-work", "walgit"), None);
    }

    #[test]
    fn a_move_between_tiers_is_said_once() {
        let mut tiers = tiers();
        assert_eq!(tiers.moved("QEST-S1", "swarm"), None);
        assert_eq!(tiers.moved("QEST-S1", "swarm"), None);
        assert_eq!(tiers.moved("QEST-S1", "walgit").as_deref(), Some("swarm"));
        assert_eq!(tiers.moved("QEST-S1", "walgit"), None);
    }
}
