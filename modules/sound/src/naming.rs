//! How the daemon names a program, as Sound.qml's `programOf` did: by the
//! desktop entry the name its stream gives finds, as the shell's
//! `heuristicLookup` would (`windows::entries`), named as the entry names
//! it, else as it names itself, "pw-cat" as "pw-cat", nothing capitalised
//! (VKCF3Z, 2026-11-09); and wearing its program's own mark
//! (`windows::marks`), else the launcher's mark for a program.

use std::fmt::Display;
use std::path::Path;

use serde_json::json;
use windows::entries::{self, Entries};
use windows::marks::Marks;

use crate::programs::Named;

/// The launcher's mark for a program whose own isn't known, as the
/// theme's `program` glyph.
const PROGRAM: &str = "program";

/// The desktop entries a program is named by, read again as they change,
/// and each program's mark.
#[derive(Default)]
pub struct Naming {
    pub entries: Entries,
    pub marks: Marks,
}

impl Naming {
    /// A program by the name its stream gives.
    pub fn named(&self, given: &str) -> Named {
        let entry = (!given.is_empty())
            .then(|| self.entries.lookup(given))
            .flatten();
        let id = entry.map_or(given, |entry| entry.id.as_str());
        Named {
            name: entry.map_or(given, |entry| entry.name.as_str()).to_owned(),
            glyph: self.marks.glyph(id).unwrap_or_else(|| json!(PROGRAM)),
        }
    }
}

/// What a file the daemon is given holds, as `made` reads it, or none
/// where it can't be read, said once, so the daemon says less rather than
/// not starting: without its words a row says the vocabulary's ids, and
/// without marks a program wears the launcher's.
pub fn read_or_none<T: Default, E: Display>(path: &str, made: impl Fn(&str) -> Result<T, E>) -> T {
    let read = std::fs::read_to_string(Path::new(path)).map_err(|why| why.to_string());
    match read.and_then(|text| made(&text).map_err(|why| why.to_string())) {
        Ok(made) => made,
        Err(why) => {
            eprintln!("<4>couldn't read {path}: {why}");
            T::default()
        }
    }
}

/// The desktop entries programs are named by, read now, and again each
/// time they change, as a switch changes them, handed to `changed`.
pub fn followed(mut changed: impl FnMut(Entries) + Send + 'static) -> Entries {
    let folders = programs::xdg::folders_here();
    let (read, looked) = Entries::read(&folders);
    let reading = folders.clone();
    let following = entries::follow(&folders, looked, move || {
        changed(Entries::read(&reading).0);
    });
    if let Err(why) = following {
        eprintln!("<4>inotify isn't there, so entries changing aren't heard: {why}");
    }
    read
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_named_and_marked_by_its_entry() {
        let naming = Naming {
            entries: Entries::from_texts([(
                "firefox",
                "[Desktop Entry]\nType=Application\nName=Firefox\n",
            )]),
            marks: Marks::of(&[("firefox", "e745")]),
        };
        let firefox = naming.named("Firefox");
        assert_eq!(
            (firefox.name.as_str(), firefox.glyph),
            ("Firefox", json!({"code": "e745"}))
        );
        let cat = naming.named("pw-cat");
        assert_eq!((cat.name.as_str(), cat.glyph), ("pw-cat", json!("program")));
    }
}
