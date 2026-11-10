//! What key-counts has tallied of each keyboard (modules/key-counts), as it
//! writes it to `/var/lib/key-counts/counts.tsv` a minute after a key is
//! pressed, beside it and renamed over it: a header, then a row for each
//! board, layer, modifiers held and key, with its presses and holds. Read
//! here as it's written, each board is a record (`counts:<board>`) with
//! its sums ready: every press and hold, the keys pressed most, and each
//! layer's and each set of modifiers' presses, the most first, so the
//! shell reads no file and sums nothing to fill a keyboard's drawer. When
//! counting began, the seconds a hand may write to `counts.since` beside
//! them, is said with each.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Root;

/// The folder key-counts keeps its counts in.
pub const FOLDER: &str = "/var/lib/key-counts";
const COUNTS: &str = "counts.tsv";
const SINCE: &str = "counts.since";
/// How many keys a board's drawer names as most pressed.
const MOST: usize = 5;

/// Whether a file of this name in the folder is one the counts are read
/// from, rather than one being written.
pub fn names(name: &str) -> bool {
    name == COUNTS || name == SINCE
}

/// A key as the key map spells it, from the kernel's name for it: one name
/// for a key there are two of, a number pad's digit as what it types, and
/// the keys the counter knows only by their number, code193 the Copilot
/// key's F23 and code70 Scroll Lock.
fn spelled(key: &str) -> &str {
    match key {
        "leftshift" | "rightshift" => "shift",
        "leftctrl" | "rightctrl" => "ctrl",
        "leftalt" | "rightalt" => "alt",
        "leftmeta" | "rightmeta" => "super",
        "kpenter" => "enter",
        "capslock" => "caps_lock",
        "numlock" => "num_lock",
        "scrolllock" | "code70" => "scroll_lock",
        "code193" => "f23",
        _ => key
            .strip_prefix("kp")
            .filter(|digit| digit.len() == 1 && digit.bytes().all(|byte| byte.is_ascii_digit()))
            .unwrap_or(key),
    }
}

/// What one board has tallied.
#[derive(Default)]
struct Tally {
    presses: u64,
    holds: u64,
    modified: u64,
    keys: BTreeMap<String, u64>,
    layers: BTreeMap<String, u64>,
    modifiers: BTreeMap<String, u64>,
}

/// Each part's presses, the most first and those alike A to Z, leaving out
/// any never pressed.
fn ranked(sums: BTreeMap<String, u64>) -> Vec<Value> {
    let mut ranked: Vec<(String, u64)> = sums
        .into_iter()
        .filter(|(_, presses)| *presses > 0)
        .collect();
    ranked.sort_by(|(one, more), (other, fewer)| fewer.cmp(more).then_with(|| one.cmp(other)));
    ranked
        .into_iter()
        .map(|(name, presses)| json!({"name": name, "presses": presses}))
        .collect()
}

/// Each board that has counted a press, by its name; none when key-counts
/// has written nothing, as on a machine without it. A line that isn't six
/// fields is left out, and a count that isn't a number counts none.
pub fn boards(root: &Root) -> Vec<Value> {
    let Some(text) = root.read(&format!("{FOLDER}/{COUNTS}")) else {
        return Vec::new();
    };
    let began = root
        .read(&format!("{FOLDER}/{SINCE}"))
        .and_then(|said| said.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0);
    let mut boards: BTreeMap<String, Tally> = BTreeMap::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let [board, layer, modifiers, key, presses, holds] = fields[..] else {
            continue;
        };
        if board == "board" {
            continue;
        }
        let presses = presses.parse::<u64>().unwrap_or(0);
        let tally = boards.entry(board.to_owned()).or_default();
        tally.presses += presses;
        tally.holds += holds.parse::<u64>().unwrap_or(0);
        *tally.keys.entry(spelled(key).to_owned()).or_default() += presses;
        *tally.layers.entry(layer.to_owned()).or_default() += presses;
        if modifiers != "-" {
            tally.modified += presses;
            *tally.modifiers.entry(modifiers.to_owned()).or_default() += presses;
        }
    }
    boards
        .into_iter()
        .filter(|(_, tally)| tally.presses > 0)
        .map(|(name, tally)| {
            let mut most = ranked(tally.keys);
            most.truncate(MOST);
            json!({
                "key": format!("counts:{name}"),
                "kind": "counts",
                "board": name,
                "presses": tally.presses,
                "holds": tally.holds,
                "most": most,
                "layers": ranked(tally.layers),
                "modified": tally.modified,
                "modifiers": ranked(tally.modifiers),
                "began_s": began,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn machine(name: &str) -> Root {
        let root =
            std::env::temp_dir().join(format!("machine-counts-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Root::new(root)
    }

    fn write(root: &Root, name: &str, text: &str) {
        let folder = root.path(FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(name), text).unwrap();
    }

    #[test]
    fn only_the_counts_and_their_start_are_read() {
        assert!(
            names("counts.tsv") && names("counts.since"),
            "the counts' files are read"
        );
        assert!(!names("counts.new"), "a file being written isn't");
    }

    #[test]
    fn a_boards_keys_layers_and_modifiers_are_summed_the_most_first() {
        let root = machine("summed");
        assert_eq!(
            boards(&root),
            Vec::<Value>::new(),
            "nothing written, no board"
        );
        // The board's keys on two layers, some under modifiers, the two
        // shifts one key, a pad's digit and the Copilot key spelled as the
        // key map does, and the laptop's apart.
        write(
            &root,
            COUNTS,
            "board\tlayer\tmodifiers\tkey\tpresses\tholds\n\
             ergodox\tcoal\t-\te\t9100\t0\n\
             ergodox\tcoal\t-\tspace\t5000\t40\n\
             ergodox\tcoal\tctrl\ta\t900\t0\n\
             ergodox\tnumb\t-\tkp1\t800\t0\n\
             ergodox\tcoal\t-\tleftshift\t2000\t2000\n\
             ergodox\tcoal\t-\trightshift\t1200\t1200\n\
             ergodox\tnumb\tshift\tcode193\t1000\t0\n\
             laptop\tbase\t-\tj\t70\t0\n\
             not a line\n\
             other\tbase\t-\tj\t0\t0\n",
        );
        write(&root, SINCE, "1793898549\n");
        let said = boards(&root);
        assert_eq!(
            said.iter()
                .map(|board| board["key"].clone())
                .collect::<Vec<_>>(),
            [json!("counts:ergodox"), json!("counts:laptop")],
            "a board that counted nothing is left out"
        );
        assert_eq!(
            said[0],
            json!({
                "key": "counts:ergodox", "kind": "counts", "board": "ergodox",
                "presses": 20000, "holds": 3240,
                "most": [{"name": "e", "presses": 9100}, {"name": "space", "presses": 5000},
                         {"name": "shift", "presses": 3200}, {"name": "f23", "presses": 1000},
                         {"name": "a", "presses": 900}],
                "layers": [{"name": "coal", "presses": 18200}, {"name": "numb", "presses": 1800}],
                "modified": 1900,
                "modifiers": [{"name": "shift", "presses": 1000}, {"name": "ctrl", "presses": 900}],
                "began_s": 1_793_898_549_u64,
            })
        );
        write(&root, SINCE, "");
        assert_eq!(boards(&root)[1]["began_s"], Value::Null, "no start written");
    }
}
