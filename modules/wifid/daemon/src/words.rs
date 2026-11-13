//! The words Wi-Fi's rows and a join's foot are said in, read from the
//! launcher's vocabulary (launcher/words.json) as the shell hands it over,
//! so wifid says no word of its own (rows.rs, acts.rs).

use std::collections::BTreeMap;

use serde_json::Value;

/// The words Wi-Fi's rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Wi-Fi Off, as the radio's row reads while it's on, and its glyph.
    pub radio_on: Said,
    /// Wi-Fi On, while it's off.
    pub radio_off: Said,
    /// "Sign in to {network}", and its glyph.
    pub sign_in: Said,
    /// How a network stands beside its name, and how it's joined after it.
    pub connected: String,
    pub saved: String,
    /// "{n} of 4 bars".
    pub bars: String,
    pub open: String,
    pub password: String,
    pub account: String,
    /// How far a join has got, by its stage as the varlink interface names it, as the launcher's
    /// foot says it while the join goes on: "connecting to {network}\u{2026}".
    pub stages: BTreeMap<String, String>,
    /// Why wifid won't, by the varlink interface's name for it: "that password didn't work".
    pub refusals: BTreeMap<String, String>,
}

/// A row's name and glyph role.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Said {
    pub name: String,
    pub glyph: String,
}

impl Words {
    /// Wi-Fi's words from the whole vocabulary, or the first it lacks, so a
    /// vocabulary that lost one is said rather than shown as blanks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let thing = |id: &str| {
            vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let word = |within: &Value, id: &str, at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(within, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let said_all = |id: &str| -> Result<BTreeMap<String, String>, String> {
            thing(id)?["said"]
                .as_object()
                .map(|said| {
                    said.iter()
                        .filter_map(|(key, word)| Some((key.clone(), word.as_str()?.to_owned())))
                        .collect()
                })
                .ok_or_else(|| format!("the vocabulary's {id} has no said"))
        };
        let radio = thing("wifi:radio")?;
        let sign_in = thing("wifi:sign-in")?;
        let network = thing("wifi:network")?;
        let said = |at: &str| word(network, "wifi:network", &["said", at]);
        Ok(Words {
            radio_on: Said {
                name: word(radio, "wifi:radio", &["name"])?,
                glyph: word(radio, "wifi:radio", &["glyph"])?,
            },
            radio_off: Said {
                name: word(radio, "wifi:radio", &["states", "off", "name"])?,
                glyph: word(radio, "wifi:radio", &["states", "off", "glyph"])?,
            },
            sign_in: Said {
                name: word(sign_in, "wifi:sign-in", &["name"])?,
                glyph: word(sign_in, "wifi:sign-in", &["glyph"])?,
            },
            connected: said("connected")?,
            saved: said("saved")?,
            bars: said("bars")?,
            open: said("open")?,
            password: said("password")?,
            account: said("account")?,
            stages: said_all("wifi:join")?,
            refusals: said_all("wifi:refused")?,
        })
    }

    /// A join's stage on a network as the foot says it, or none for a stage it has no word for.
    pub fn staged(&self, stage: &str, network: &str) -> Option<String> {
        self.stages
            .get(stage)
            .map(|said| said.replace("{network}", network))
    }

    /// Why wifid won't, in the launcher's words, or none for a reason it has no word for.
    pub fn refused(&self, why: &str) -> Option<&str> {
        self.refusals.get(why).map(String::as_str)
    }
}

#[cfg(test)]
pub(crate) fn launchers_words() -> Value {
    serde_json::from_str(include_str!("../tests/words.json")).expect("the launcher's words parse")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_vocabulary_without_a_word_says_which() {
        let said = Words::read(&json!({"things": [{"id": "wifi:radio"}]}));
        assert_eq!(said, Err("the vocabulary has no wifi:sign-in".into()));
    }
}
