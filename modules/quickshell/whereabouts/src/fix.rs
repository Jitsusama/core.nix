//! Where the laptop is, as the file the launcher reads says it, and whether
//! a new position is far enough from the last to be worth saying.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The file's shape. The launcher reads a file of this version alone, so
/// one of another shape draws nothing rather than something wrong. The
/// second names the place and its zone.
const VERSION: u32 = 2;

/// How far, in metres, the laptop has to move before here does. The
/// weather's models see the land in cells a few kilometres wide, so a
/// smaller move changes no forecast, and GeoClue's answers at a city's
/// accuracy wander by about as much while the laptop sits still.
pub const FAR_ENOUGH: u32 = 5_000;

/// The Earth's mean radius, in metres, for the distance between two fixes.
const EARTH: f64 = 6_371_000.0;

/// One position: where, how sure and when, and what it's called and the
/// zone it keeps where they could be asked (named.rs). GeoClue's own
/// description of a position says which of its sources found it, not
/// where it is, so it's never the name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fix {
    v: u32,
    pub latitude: f64,
    pub longitude: f64,
    /// How far from here the laptop may be, in metres.
    pub accuracy: f64,
    /// When it was found, in seconds since the epoch.
    pub when: u64,
    /// The town and the province it's in, "London, Ontario", or null while
    /// it couldn't be asked.
    #[serde(default)]
    pub place: Option<String>,
    /// The time zone kept there, "America/Toronto", or null likewise.
    #[serde(default)]
    pub zone: Option<String>,
}

impl Fix {
    /// A fix from what GeoClue says, or nothing when what it says can't be
    /// a place on the Earth.
    pub fn new(latitude: f64, longitude: f64, accuracy: f64, when: u64) -> Option<Fix> {
        let fix = Fix {
            v: VERSION,
            latitude,
            longitude,
            accuracy,
            when,
            place: None,
            zone: None,
        };
        fix.holds().then_some(fix)
    }

    /// Whether it still lacks its name or its zone, so it's asked again
    /// when GeoClue next says anything, as it does once a network comes.
    pub fn unnamed(&self) -> bool {
        self.place.is_none() || self.zone.is_none()
    }

    /// The fix with what it's called and its zone, each kept where the new
    /// asking couldn't say.
    pub fn named(mut self, place: Option<String>, zone: Option<String>) -> Fix {
        self.place = place.or(self.place);
        self.zone = zone.or(self.zone);
        self
    }

    /// The fix a file says, or nothing when it's of another version, isn't
    /// JSON or names no place on the Earth, so a file left by an older
    /// program, or a half-written one, counts as no fix at all.
    pub fn read(text: &str) -> Option<Fix> {
        let fix: Fix = serde_json::from_str(text).ok()?;
        (fix.v == VERSION && fix.holds()).then_some(fix)
    }

    fn holds(&self) -> bool {
        (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
            && self.accuracy.is_finite()
            && self.accuracy >= 0.0
    }

    /// How far this fix is from another, in metres, along the Earth's
    /// surface, by the haversine formula.
    pub fn distance(&self, other: &Fix) -> f64 {
        let (from, to) = (self.latitude.to_radians(), other.latitude.to_radians());
        let across = (other.latitude - self.latitude).to_radians();
        let along = (other.longitude - self.longitude).to_radians();
        let half =
            (across / 2.0).sin().powi(2) + from.cos() * to.cos() * (along / 2.0).sin().powi(2);
        2.0 * EARTH * half.sqrt().asin()
    }

    /// Writes the fix to `path` whole: to a name beside it first, then
    /// renamed over it, so the launcher never reads half a file.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut name = path.file_name().unwrap_or_default().to_owned();
        name.push(".new");
        let beside = path.with_file_name(name);
        fs::write(&beside, serde_json::to_vec(self)?)?;
        fs::rename(&beside, path)
    }
}

/// Whether `now` is worth saying, given what was said `last`: always when
/// nothing was, and otherwise when the laptop moved further than both the
/// least move that matters and the new fix's own doubt, so a vague fix
/// close by never moves here off a sharper one.
pub fn moved(last: Option<&Fix>, now: &Fix) -> bool {
    last.is_none_or(|last| last.distance(now) >= f64::from(FAR_ENOUGH).max(now.accuracy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(latitude: f64, longitude: f64, accuracy: f64) -> Fix {
        Fix::new(latitude, longitude, accuracy, 1_790_000_000).unwrap()
    }

    #[test]
    fn a_fix_is_written_and_read_back_whole() {
        let fix = at(10.5, -20.25, 3000.0);
        let text = serde_json::to_string(&fix).unwrap();
        assert_eq!(
            text,
            r#"{"v":2,"latitude":10.5,"longitude":-20.25,"accuracy":3000.0,"when":1790000000,"place":null,"zone":null}"#
        );
        assert_eq!(Fix::read(&text), Some(fix));
        let named = at(10.5, -20.25, 3000.0).named(
            Some("Lighthouse, Ontario".into()),
            Some("America/Toronto".into()),
        );
        let text = serde_json::to_string(&named).unwrap();
        assert!(text.ends_with(r#""place":"Lighthouse, Ontario","zone":"America/Toronto"}"#));
        assert_eq!(Fix::read(&text), Some(named));
    }

    #[test]
    fn a_name_asked_again_keeps_what_the_last_asking_found() {
        let fix = at(1.0, 2.0, 3.0).named(Some("Cove".into()), None);
        assert!(fix.unnamed());
        let fix = fix.named(None, Some("America/Halifax".into()));
        assert_eq!(fix.place.as_deref(), Some("Cove"));
        assert_eq!(fix.zone.as_deref(), Some("America/Halifax"));
        assert!(!fix.unnamed());
    }

    #[test]
    fn a_file_that_isnt_one_is_no_fix() {
        assert_eq!(Fix::read(""), None);
        assert_eq!(Fix::read(r#"{"v":1,"latitude":1"#), None);
        // A first version's file, which names no place, is no fix, so a
        // place is asked for as the program starts.
        assert_eq!(
            Fix::read(r#"{"v":1,"latitude":1,"longitude":2,"accuracy":3,"when":4}"#),
            None
        );
        assert_eq!(
            Fix::read(r#"{"latitude":1,"longitude":2,"accuracy":3,"when":4}"#),
            None
        );
        assert!(Fix::read(r#"{"v":2,"latitude":1,"longitude":2,"accuracy":3,"when":4}"#).is_some());
    }

    #[test]
    fn no_place_off_the_earth_is_a_fix() {
        assert_eq!(Fix::new(91.0, 0.0, 1.0, 0), None);
        assert_eq!(Fix::new(0.0, -181.0, 1.0, 0), None);
        assert_eq!(Fix::new(f64::NAN, 0.0, 1.0, 0), None);
        assert_eq!(Fix::new(0.0, 0.0, -1.0, 0), None);
        assert_eq!(Fix::new(0.0, 0.0, f64::INFINITY, 0), None);
        assert!(Fix::new(-90.0, 180.0, 0.0, 0).is_some());
    }

    #[test]
    fn distance_is_along_the_surface() {
        // A degree of latitude is about 111.2 km anywhere, and a degree of
        // longitude shrinks with the cosine of the latitude.
        let degree = at(0.0, 0.0, 0.0).distance(&at(1.0, 0.0, 0.0));
        assert!((degree - 111_195.0).abs() < 10.0, "{degree}");
        let north = at(60.0, 0.0, 0.0).distance(&at(60.0, 1.0, 0.0));
        assert!((north - degree / 2.0).abs() < 100.0, "{north}");
        assert!(at(5.0, 5.0, 0.0).distance(&at(5.0, 5.0, 0.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn the_first_fix_is_always_said() {
        assert!(moved(None, &at(0.0, 0.0, 100_000.0)));
    }

    #[test]
    fn a_small_move_is_not_said() {
        // About 3.3 km north, with a sharp fix.
        assert!(!moved(Some(&at(0.0, 0.0, 100.0)), &at(0.03, 0.0, 100.0)));
    }

    #[test]
    fn a_move_past_the_least_that_matters_is_said() {
        // About 5.6 km north.
        assert!(moved(Some(&at(0.0, 0.0, 100.0)), &at(0.05, 0.0, 100.0)));
    }

    #[test]
    fn a_vague_fix_inside_its_own_doubt_is_not_said() {
        // About 11 km away, but only known to within 25 km.
        assert!(!moved(Some(&at(0.0, 0.0, 100.0)), &at(0.1, 0.0, 25_000.0)));
        // About 111 km away, so further than its doubt.
        assert!(moved(Some(&at(0.0, 0.0, 100.0)), &at(1.0, 0.0, 25_000.0)));
    }

    #[test]
    fn a_sharp_fix_moves_here_off_a_vague_one() {
        assert!(moved(Some(&at(0.0, 0.0, 25_000.0)), &at(0.1, 0.0, 500.0)));
    }

    #[test]
    fn a_fix_is_replaced_whole_and_leaves_nothing_beside_it() {
        let folder = std::env::temp_dir().join(format!("whereabouts-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("whereabouts.json");
        at(1.0, 2.0, 3.0).write(&path).unwrap();
        let fix = at(4.0, 5.0, 6.0);
        fix.write(&path).unwrap();
        assert_eq!(Fix::read(&fs::read_to_string(&path).unwrap()), Some(fix));
        let left: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["whereabouts.json"]);
        fs::remove_dir_all(&folder).unwrap();
    }
}
