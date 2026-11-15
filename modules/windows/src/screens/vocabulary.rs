//! The files Screens is said by, read once as the daemon starts: the
//! launcher's words.json and the account's number settings, each one it
//! can't read said once in the journal, Screens keeping its rows; and
//! the words read from the vocabulary, each where its row says it.

use std::collections::BTreeMap;
use std::path::Path;

use ready_rows::numbers::Numbers;
use serde_json::Value;

use super::{Counted, FACTS, PRIMARIES, Sleep, TRANSFORMS, Told, Words, acts, line, tells};
use crate::light;

impl Words {
    /// The words a screen's row is said in, and what the vocabulary
    /// lacks. A word it lacks is said as its id, and a glyph it lacks is
    /// none, so Screens keeps its rows and what they do rather than
    /// showing none for want of a word, as the sound daemon does.
    pub fn read(vocabulary: &Value) -> (Words, Vec<String>) {
        let things: BTreeMap<&str, &Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thing| Some((thing["id"].as_str()?, thing)))
            .collect();
        let lacked = std::cell::RefCell::new(Vec::new());
        let found = |id: &str, path: &[&str]| -> Option<String> {
            let mut at = *things.get(id)?;
            for step in path {
                at = &at[*step];
            }
            at.as_str().map(str::to_owned)
        };
        let word = |id: &str, path: &[&str], unsaid: &str| {
            found(id, path).unwrap_or_else(|| {
                lacked.borrow_mut().push(format!("{id} {}", path.join(".")));
                unsaid.to_owned()
            })
        };
        let said = |key: &str| word("displays", &["said", key], key);
        let words = Words {
            built_in: word("displays:built-in", &["name"], "displays:built-in"),
            dark: said("dark"),
            off: said("off"),
            focused: said("focused"),
            hub_on: said("hubOn"),
            scale_at: said("scaleAt"),
            not_on: said("not-on"),
            not_off: said("not-off"),
            not_unplugged: said("not-unplugged"),
            not_set: said("not-set"),
            chosen: said("chosen"),
            not_offered: said("not-offered"),
            none_given: said("none-given"),
            offers_range: said("offers-range"),
            is_unplugged: said("isUnplugged"),
            counted: Counted::read(&said),
            stood: line::Stood::read(&word),
            wakes: word("verb:display-off", &["said", "wakes"], "wakes"),
            moves: word("verb:display-unplug", &["said", "moves"], "moves"),
            ways: acts::Way::read(&word),
            tells: tells::Tells::read(&word),
            sleep: Sleep {
                name: word("displays:sleep", &["name"], "displays:sleep"),
                glyph: word("displays:sleep", &["glyph"], ""),
                note: word("displays:sleep", &["said", "note"], "note"),
            },
            told: Told {
                named: FACTS
                    .iter()
                    .map(|id| {
                        let part = format!("displays:{id}");
                        (*id, word(&part, &["name"], &part))
                    })
                    .collect(),
                turned: TRANSFORMS
                    .iter()
                    .map(|turn| (*turn, word("displays:rotation", &["said", turn], turn)))
                    .collect(),
                built_in: said("built-in"),
                screen: said("screen"),
                preferred: word("displays:modes", &["said", "preferred"], "preferred"),
                custom: word("displays:modes", &["said", "custom"], "custom"),
                preferred_mode: word(
                    "displays:modes",
                    &["said", "preferred-mode"],
                    "preferred-mode",
                ),
                at: word("displays:modes", &["said", "at"], "at"),
                unsupported: word("displays:variable", &["said", "unsupported"], "unsupported"),
                variable_on: word("displays:variable", &["said", "on"], "on"),
                variable_off: word("displays:variable", &["said", "off"], "off"),
                typed: TRANSFORMS
                    .iter()
                    .map(|turn| {
                        let typed = format!("typed-{turn}");
                        (*turn, word("displays:rotation", &["said", &typed], &typed))
                    })
                    .collect(),
                parts: things
                    .iter()
                    .filter_map(|(id, thing)| {
                        let part = id.strip_prefix("displays:")?;
                        Some((part.to_owned(), thing["name"].as_str()?.to_owned()))
                    })
                    .collect(),
                white: word("displays:white", &["said", "point"], "point"),
                primaries: PRIMARIES
                    .map(|colour| word("displays:primaries", &["said", colour], colour)),
            },
            light: light::Words::read(
                &word,
                things
                    .get("displays")
                    .map_or(&Value::Null, |screens| &screens["said"]),
            ),
            numbers: Numbers::default(),
        };
        (words, lacked.into_inner())
    }

    /// The words with the account's number settings, which a screen's
    /// density is said in.
    #[must_use]
    pub fn numbered(self, numbers: Numbers) -> Words {
        Words { numbers, ..self }
    }
}

impl Counted {
    /// How many screens there are, in the place's own words.
    fn read(said: &impl Fn(&str) -> String) -> Counted {
        Counted {
            one: said("counted-one"),
            many: said("counted-many"),
            on: said("counted-on"),
        }
    }
}

/// The words a screen's row is said in, from the launcher's words.json,
/// each it lacks said by its id and warned of once, here.
#[must_use]
pub fn worded(path: &Path) -> Words {
    let read = std::fs::read_to_string(path).map_err(|why| why.to_string());
    let vocabulary =
        read.and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()));
    let (words, lacked) = Words::read(vocabulary.as_ref().unwrap_or(&Value::Null));
    if let Err(why) = &vocabulary {
        eprintln!(
            "<4>couldn't read the words in {}, so Screens says ids: {why}",
            path.display()
        );
    } else if !lacked.is_empty() {
        eprintln!(
            "<4>the words in {} lack {}, said by id",
            path.display(),
            lacked.join(", ")
        );
    }
    words
}

/// The account's number settings, or the comma and the full stop where
/// it's handed none, or none it can read, which it says once, here.
#[must_use]
pub fn numbered(path: Option<&Path>) -> Numbers {
    let Some(path) = path else {
        return Numbers::default();
    };
    let read = std::fs::read_to_string(path).map_err(|why| why.to_string());
    match read.and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string())) {
        Ok(settings) => Numbers::read(&settings),
        Err(why) => {
            eprintln!(
                "<4>couldn't read the number settings in {}, so Screens says the usual: {why}",
                path.display()
            );
            Numbers::default()
        }
    }
}
