//! Who sent a notice, as its row and its card sign it: the name Joel knows
//! it by, never its program's, the mark it wears, the hue of what it
//! brings, and the desktop entry it goes to, as the shell's Notices.qml
//! `sender` said it.
//!
//! senders.json names the desktop's own senders. A person's message wears
//! the person's hue (Slack, Tuple, a call, a meeting). notify-send is the
//! tool a notice came through, not who sent it, so a notice it names no
//! sender for is the desktop's, as niri's are (a screenshot taken, its
//! settings not loading). A pi session wanting Joel wears the agents'
//! mark, For You's waiting its tray's, and each of the machine's own
//! (Capture, the camera, a timer, the battery, firmware) its own. Every
//! other sender wears its program's mark, the one its row among Programs
//! wears, from the marks file the module writes (ProgramGlyphs.js's table
//! and the machine's own, read as the windows crate's marks, which Sound's
//! programs wear too), and the bell where no program's is known.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};
use windows::entries::Entries;
pub use windows::marks::Marks;

use crate::words::Words;

/// One of senders.json's senders.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Known {
    /// The desktop entry id of the program it is, whose mark it wears.
    pub program: Option<String>,
    /// Its own mark, a glyph role or `{ code }`, where it's the desktop's.
    pub glyph: Option<Value>,
    pub hue: Option<String>,
    /// Its name, as it's signed.
    pub name: Option<String>,
    /// The vocabulary's thing it's named as, where its name is the
    /// launcher's word.
    pub named: Option<String>,
}

/// senders.json's senders, by their names in lower case.
#[derive(Debug, Default, Deserialize)]
pub struct Senders {
    #[serde(default)]
    pub senders: BTreeMap<String, Known>,
}

/// The bell, worn by a sender no program's mark is known for.
const BELL: &str = "f0f3";

/// A sender as a notice's row and card sign it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sender {
    pub glyph: Value,
    pub hue: String,
    pub name: String,
    /// The desktop entry it goes to, where one resolves.
    pub entry: Option<String>,
}

/// What a sender is told by.
pub struct Signs<'a> {
    pub senders: &'a Senders,
    pub marks: &'a Marks,
    pub entries: &'a Entries,
    pub words: &'a Words,
}

impl Signs<'_> {
    /// Who sent a notice, by the name it gave and the desktop entry it
    /// named, if any: one the desktop knows by name as it's listed; any
    /// other is signed by its program's own name, as its row among
    /// Programs is, or else as it named itself. Its program is the one
    /// senders.json names, else the entry it named, else the one its name
    /// finds as the shell's `heuristicLookup` would.
    pub fn sender(&self, app: &str, entry: &str) -> Sender {
        let known = self.senders.senders.get(&app.to_lowercase());
        let program = known
            .and_then(|known| known.program.as_deref())
            .and_then(|id| self.entries.lookup(id))
            .or_else(|| {
                (!entry.is_empty())
                    .then(|| self.entries.lookup(entry))
                    .flatten()
            })
            .or_else(|| self.entries.lookup(app));
        let named = known.and_then(|known| {
            known
                .named
                .as_deref()
                .map(|thing| self.words.name(thing, "").to_owned())
                .or_else(|| known.name.clone())
        });
        let name = named
            .or_else(|| program.map(|entry| entry.name.clone()))
            .unwrap_or_else(|| app.to_owned());
        let glyph = known
            .and_then(|known| known.glyph.clone())
            .or_else(|| {
                let id = known
                    .and_then(|known| known.program.as_deref())
                    .or_else(|| program.map(|entry| entry.id.as_str()))
                    .unwrap_or(app);
                self.marks.glyph(id)
            })
            .unwrap_or_else(|| json!({ "code": BELL }));
        Sender {
            glyph,
            hue: known
                .and_then(|known| known.hue.clone())
                .unwrap_or_else(|| "program".to_owned()),
            name,
            entry: program.map(|entry| entry.id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn senders() -> Senders {
        serde_json::from_str(include_str!("../senders.json")).expect("senders.json reads")
    }

    fn entries() -> Entries {
        Entries::from_texts([
            (
                "slack",
                "[Desktop Entry]\nType=Application\nName=Slack\nStartupWMClass=Slack\n",
            ),
            (
                "org.gnome.Fractal",
                "[Desktop Entry]\nType=Application\nName=Fractal\n",
            ),
        ])
    }

    fn signed(app: &str, entry: &str) -> Sender {
        let (senders, entries) = (senders(), entries());
        let marks = Marks::of(&[("slack", "f198"), ("pi", "mark:agent")]);
        let words = crate::words::tests::core();
        Signs {
            senders: &senders,
            marks: &marks,
            entries: &entries,
            words: &words,
        }
        .sender(app, entry)
    }

    #[test]
    fn a_known_sender_is_signed_as_the_desktop_knows_it() {
        let slack = signed("Slack", "");
        assert_eq!(
            slack.glyph,
            json!({ "code": "f198" }),
            "its program's own mark"
        );
        assert_eq!(
            (slack.hue.as_str(), slack.name.as_str()),
            ("person", "Slack")
        );
        assert_eq!(slack.entry.as_deref(), Some("slack"));
        let pi = signed("pi", "");
        assert_eq!(pi.glyph, json!("agent"), "the desktop's own, as a role");
        assert_eq!(pi.name, "Agents", "named in the launcher's word");
        assert_eq!(pi.entry, None, "no program, so nowhere to go");
        assert_eq!(signed("notify-send", "").glyph, json!({ "code": "f0e51" }));
    }

    #[test]
    fn any_other_is_signed_by_its_program_else_itself() {
        let fractal = signed("fractal", "org.gnome.Fractal");
        assert_eq!(fractal.name, "Fractal", "its entry's name");
        assert_eq!(fractal.entry.as_deref(), Some("org.gnome.Fractal"));
        assert_eq!(
            fractal.glyph,
            json!({ "code": BELL }),
            "the bell, its mark unknown"
        );
        let stranger = signed("some-tool", "");
        assert_eq!(
            (stranger.name.as_str(), stranger.hue.as_str()),
            ("some-tool", "program")
        );
        assert_eq!(stranger.entry, None);
    }
}
