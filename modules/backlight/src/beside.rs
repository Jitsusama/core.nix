//! The screens beside the laptop's own panel, such as a monitor on its
//! dock, which the night light warms as it warms the panel. Only the night
//! reaches them, never the room's warmth: the sensor sits beside the panel,
//! and another screen's white is its own until the evening. At a desk a
//! monitor gives nearly twice the panel's light at the same nits, so a
//! night that warmed only the panel would leave most of the light white.
//!
//! wl-gammarelay-rs serves an object for each screen niri shows but lists
//! none of them, so they're found from the kernel's DRM connectors that
//! niri has on, looked at every few seconds, which costs a folder's read.
//! A screen just on takes the night's white at once, since it's lighting
//! up anyway; after that the night's own small steps are its glide.
//!
//! While a program that keeps its own colours is in front (front.rs), the
//! screens take their own white at once, and the night's once it leaves.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// How often the connectors are looked at for a screen come or gone.
pub const LOOKS: Duration = Duration::from_secs(5);

/// How long a screen the relay has no object for is let be, once it has
/// missed `MISSES` times in a row: niri shows a screen a moment after the
/// kernel has it on, but one it never shows shouldn't be asked every look.
pub const MISSED_RESTS: Duration = Duration::from_mins(1);

/// How many looks in a row a screen may miss before it's let be a while.
pub const MISSES: u8 = 3;

/// The connectors besides the panel's that niri has on, under `drm`, named
/// as niri and the relay name them, such as DP-5, in order.
#[must_use]
pub fn shown(drm: &Path, panel: Option<&str>) -> Vec<String> {
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut shown: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (card, connector) = name.split_once('-')?;
            let enabled = fs::read_to_string(entry.path().join("enabled")).ok()?;
            (card.starts_with("card") && Some(connector) != panel && enabled.trim() == "enabled")
                .then(|| connector.to_owned())
        })
        .collect();
    shown.sort();
    shown
}

/// What each screen beside the panel was last set to, which have missed,
/// and when they're next looked at.
#[derive(Default)]
pub struct Beside {
    set: BTreeMap<String, u16>,
    missed: BTreeMap<String, (u8, Instant)>,
    due: Option<Instant>,
    rests: Option<Instant>,
    /// Whether a program that keeps its own colours is in front.
    kept: bool,
}

impl Beside {
    /// Knowing no screen yet, and due to look at once.
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Beside {
            due: Some(now),
            ..Beside::default()
        }
    }

    /// When the screens are next looked at.
    #[must_use]
    pub fn wake(&self) -> Option<Instant> {
        self.due
    }

    /// The screens of `shown` that are owed `kelvin` now, forgetting those
    /// no longer shown, so one shown again is set again.
    #[must_use]
    pub fn owed(&mut self, shown: &[String], kelvin: u16, now: Instant) -> Vec<String> {
        self.set.retain(|screen, _| shown.contains(screen));
        self.missed.retain(|screen, _| shown.contains(screen));
        shown
            .iter()
            .filter(|screen| {
                self.set.get(*screen) != Some(&kelvin)
                    && self
                        .missed
                        .get(*screen)
                        .is_none_or(|&(_, until)| until <= now)
            })
            .cloned()
            .collect()
    }

    /// The screens were looked at.
    pub fn looked(&mut self, now: Instant) {
        let next = now + LOOKS;
        self.due = Some(self.rests.map_or(next, |rests| rests.max(next)));
    }

    /// A screen took `kelvin`. Says whether it's newly warmed by the night.
    pub fn took(&mut self, screen: &str, kelvin: u16) -> bool {
        self.missed.remove(screen);
        self.set.insert(screen.to_owned(), kelvin).is_none()
    }

    /// The relay had no object for a screen: it's asked again at the next
    /// look, until it has missed `MISSES` times in a row.
    pub fn missed(&mut self, screen: &str, now: Instant) {
        let tries = self
            .missed
            .get(screen)
            .map_or(0, |&(tries, _)| tries)
            .saturating_add(1);
        let until = if tries >= MISSES {
            now + MISSED_RESTS
        } else {
            now
        };
        self.missed.insert(screen.to_owned(), (tries, until));
    }

    /// The relay refused otherwise, or didn't answer, so the screens are
    /// let be until a moment.
    pub fn rest(&mut self, until: Instant) {
        self.rests = Some(until);
        self.due = Some(until);
    }

    /// The night moved, so the screens are looked at once, unless they're
    /// let be.
    pub fn look_now(&mut self, now: Instant) {
        self.due = Some(self.rests.map_or(now, |rests| rests.max(now)));
    }

    /// Whether a program that keeps its own colours is in front
    /// (front.rs), when the screens hold their own white whatever the
    /// night; they're looked at once when that changes.
    pub fn keep_white(&mut self, kept: bool, now: Instant) {
        if self.kept != kept {
            self.kept = kept;
            self.look_now(now);
        }
    }

    /// Whether the screens hold their own white whatever the night.
    #[must_use]
    pub fn keeps_white(&self) -> bool {
        self.kept
    }

    /// The relay started again at its own white, so every screen is set
    /// again, at once.
    pub fn forget(&mut self, now: Instant) {
        self.set.clear();
        self.missed.clear();
        self.rests = None;
        self.due = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn names(screens: &[&str]) -> Vec<String> {
        screens.iter().map(|&screen| screen.to_owned()).collect()
    }

    #[test]
    fn the_screens_beside_the_panel_are_those_niri_has_on() {
        let drm = std::env::temp_dir().join(format!("beside-{}", std::process::id()));
        for (connector, enabled) in [
            ("card0-eDP-1", "enabled"),
            ("card0-DP-1", "disabled"),
            ("card0-DP-5", "enabled"),
            ("card0-DP-4", "enabled"),
        ] {
            fs::create_dir_all(drm.join(connector)).unwrap();
            fs::write(drm.join(connector).join("enabled"), format!("{enabled}\n")).unwrap();
        }
        fs::create_dir_all(drm.join("card0")).unwrap();
        fs::create_dir_all(drm.join("renderD128")).unwrap();
        assert_eq!(
            shown(&drm, Some("eDP-1")),
            names(&["DP-4", "DP-5"]),
            "the panel or a screen niri has off was taken for one beside it"
        );
        fs::remove_dir_all(&drm).unwrap();
    }

    #[test]
    fn a_missing_drm_folder_shows_no_screen() {
        assert!(
            shown(Path::new("/nonexistent/drm"), Some("eDP-1")).is_empty(),
            "a screen was found where there's no DRM"
        );
    }

    #[test]
    fn the_screens_are_looked_at_once_then_every_few_seconds() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        assert_eq!(beside.wake(), Some(now), "the first look waits");
        beside.looked(now);
        assert_eq!(beside.wake(), Some(now + LOOKS), "the next look isn't due");
    }

    #[test]
    fn a_screen_just_shown_takes_the_night_at_once_and_its_steps_after() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        let shown = names(&["DP-5"]);
        assert_eq!(beside.owed(&shown, 4000, now), shown, "a new screen waits");
        assert!(beside.took("DP-5", 4000), "a first warming isn't said");
        assert!(
            beside.owed(&shown, 4000, now).is_empty(),
            "a screen already at the night's white is set again"
        );
        assert_eq!(
            beside.owed(&shown, 3950, now),
            shown,
            "the night's step doesn't reach the screen"
        );
        assert!(!beside.took("DP-5", 3950), "each step is said as new");
    }

    #[test]
    fn a_screen_gone_and_back_is_set_again() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        let shown = names(&["DP-5"]);
        let _ = beside.owed(&shown, 4000, now);
        beside.took("DP-5", 4000);
        assert!(
            beside.owed(&[], 4000, now).is_empty(),
            "a gone screen is owed"
        );
        assert_eq!(
            beside.owed(&shown, 4000, now),
            shown,
            "a screen plugged in again keeps the white it had"
        );
    }

    #[test]
    fn a_relay_started_again_sets_every_screen_again_at_once() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        let shown = names(&["DP-4", "DP-5"]);
        let _ = beside.owed(&shown, 4000, now);
        beside.took("DP-4", 4000);
        beside.took("DP-5", 4000);
        beside.looked(now);
        let later = now + Duration::from_secs(1);
        beside.forget(later);
        assert_eq!(beside.wake(), Some(later), "the restart waits for a look");
        assert_eq!(
            beside.owed(&shown, 4000, later),
            shown,
            "a restarted relay's screens are left at its white"
        );
    }

    #[test]
    fn a_screen_not_yet_shown_is_asked_each_look_then_let_be() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        let shown = names(&["DP-5"]);
        for miss in 0..MISSES {
            let at = now + LOOKS * u32::from(miss);
            assert_eq!(
                beside.owed(&shown, 4000, at),
                shown,
                "miss {miss} isn't asked again"
            );
            beside.missed("DP-5", at);
        }
        let last = now + LOOKS * u32::from(MISSES - 1);
        assert!(
            beside.owed(&shown, 4000, last + LOOKS).is_empty(),
            "a screen the relay never shows is asked every look"
        );
        assert_eq!(
            beside.owed(&shown, 4000, last + MISSED_RESTS),
            shown,
            "a screen let be is never asked again"
        );
    }

    #[test]
    fn a_screen_that_took_the_night_after_missing_starts_afresh() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        let shown = names(&["DP-5"]);
        for _ in 0..MISSES {
            beside.missed("DP-5", now);
        }
        beside.took("DP-5", 4000);
        beside.missed("DP-5", now);
        assert_eq!(
            beside.owed(&shown, 3950, now + LOOKS),
            shown,
            "old misses count against a screen that took a white since"
        );
    }

    #[test]
    fn a_refusing_relay_is_let_be_even_when_the_night_moves() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        beside.looked(now);
        let until = now + Duration::from_mins(10);
        beside.rest(until);
        assert_eq!(
            beside.wake(),
            Some(until),
            "a refusing relay is asked again"
        );
        beside.look_now(now + Duration::from_secs(30));
        assert_eq!(
            beside.wake(),
            Some(until),
            "the night's step asks a refusing relay"
        );
        beside.look_now(until + Duration::from_secs(1));
        assert_eq!(
            beside.wake(),
            Some(until + Duration::from_secs(1)),
            "the night's step after the rest waits"
        );
    }

    #[test]
    fn a_program_keeping_its_colours_gives_the_screens_their_white_at_once() {
        let now = Instant::now();
        let mut beside = Beside::new(now);
        beside.looked(now);
        let later = now + Duration::from_secs(1);
        beside.keep_white(true, later);
        assert!(
            beside.keeps_white(),
            "the screens took the night in front of GIMP"
        );
        assert_eq!(beside.wake(), Some(later), "GIMP's white waits for a look");
        beside.keep_white(false, later);
        assert!(
            !beside.keeps_white(),
            "the night didn't come back once GIMP left"
        );
    }
}
