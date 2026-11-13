//! Where the windows of sessions moving onto pi's host sat, by pi's id for
//! each, so each one's viewer opens there once it runs hosted (OSPHT4's
//! cutover; 2QZBX9, [906a6c10]). The shell works each place out from
//! niri and says the whole of them as they change, answering the cutover
//! at once; they're kept here, served as `kept-places:agents` for the
//! shell to take as it starts, rather than in a file of the shell's own
//! (Joel, 2026-11-10: "the daemons need to be the data layer, and they
//! need to serve the launcher"). What each place holds is the shell's
//! (Viewers.js), so it's kept as said.

use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::file;
use crate::served::PLACE;

/// The kind it's served as.
pub const KIND: &str = "kept-places";

/// The places kept, by pi's id for each session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptPlaces {
    pub places: Map<String, Value>,
}

impl KeptPlaces {
    /// The places `places` says, each by a session's id and each a
    /// place, or none where it says anything else.
    pub fn said(places: &Value) -> Option<Self> {
        let places = places.as_object()?;
        let fits = |id: &String| {
            !id.is_empty()
                && id
                    .chars()
                    .all(|one| one.is_ascii_alphanumeric() || one == '-')
        };
        places
            .iter()
            .all(|(id, place)| fits(id) && place.is_object())
            .then(|| Self {
                places: places.clone(),
            })
    }

    /// What's kept at `path`, or none there.
    pub fn read(path: &Path) -> Option<Self> {
        let kept = file::read(path)?;
        if kept["v"] != 1 {
            return None;
        }
        Self::said(&kept["places"])
    }

    /// What the shell kept before the daemon did, in the newest of its own
    /// folders under `shells`, its places whole, read once while the
    /// daemon has kept none.
    pub fn from_shell(shells: &Path) -> Self {
        file::newest(shells, "kept-places.json")
            .and_then(|kept| Self::said(&file::read(&kept)?))
            .unwrap_or_default()
    }

    /// Writes it to `path` whole, renamed into place, so a daemon started
    /// again reads it as it was.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        fs::write(&writing, json!({"v": 1, "places": self.places}).to_string())?;
        fs::rename(&writing, path)
    }

    /// Its record, as the shell takes it.
    pub fn record(&self) -> Value {
        json!({"key": format!("{KIND}:{PLACE}"), "kind": KIND, "places": self.places})
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn only_places_by_session_are_kept() {
        let place = json!({"workspace": 3, "column": 2, "at": 1_794_414_593_809_u64});
        assert_eq!(
            KeptPlaces::said(&json!({"01a1-b2": place})).map(|kept| kept.places.len()),
            Some(1),
            "a place by a session's id"
        );
        for refused in [
            json!([place]),
            json!({"01a1": 3}),
            json!({"../01a1": place}),
            json!({"": place}),
        ] {
            assert_eq!(KeptPlaces::said(&refused), None, "{refused} is no places");
        }
        assert_eq!(
            KeptPlaces::said(&json!({})),
            Some(KeptPlaces::default()),
            "none at all, as once each is let go"
        );
    }

    #[test]
    fn what_the_shell_kept_is_read_from_its_newest_folder_and_kept_whole() {
        let shells =
            std::env::temp_dir().join(format!("agents-kept-places-{}", std::process::id()));
        let _ = fs::remove_dir_all(&shells);
        let older = shells.join("older");
        let newer = shells.join("newer");
        fs::create_dir_all(&older).expect("a shell's folder");
        fs::create_dir_all(&newer).expect("another's");
        let place = json!({"workspace": 8, "column": 3, "at": 1});
        fs::write(
            older.join("kept-places.json"),
            json!({"01-old": place}).to_string(),
        )
        .expect("the older is written");
        std::thread::sleep(Duration::from_millis(20));
        fs::write(
            newer.join("kept-places.json"),
            json!({"01-new": place}).to_string(),
        )
        .expect("the newer is written");
        let read = KeptPlaces::from_shell(&shells);
        assert_eq!(
            read.places.keys().collect::<Vec<_>>(),
            ["01-new"],
            "the newest one's"
        );
        let kept = shells.join("agents").join("kept-places.json");
        read.write(&kept).expect("it's kept");
        assert_eq!(
            KeptPlaces::read(&kept),
            Some(read.clone()),
            "and read back whole"
        );
        assert_eq!(
            read.record(),
            json!({"key": "kept-places:agents", "kind": "kept-places",
                "places": {"01-new": place}}),
            "served for the shell to take"
        );
        assert_eq!(
            KeptPlaces::from_shell(&shells.join("none")),
            KeptPlaces::default(),
            "no shell, nothing kept"
        );
        let _ = fs::remove_dir_all(&shells);
    }
}
