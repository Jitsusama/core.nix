//! Whether the lid is shut, from logind and the laptop's own panel. niri
//! darkens the panel whenever the lid's switch says shut, and lights it
//! again only once a key, a click or a pointer moved proves the lid open
//! (modules/niri, shut-lid-darkens-the-panel.patch). So a panel still lit a
//! moment after logind says shut means the switch is wrong, and the light
//! the sensor hears is the room's, not the lid's own.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long after logind says shut, and how often after that, the panel is
/// looked at. niri darkens it within a frame of a real close, so a look
/// this late never sees it lit on the way down.
pub const PANEL_LOOKS: Duration = Duration::from_secs(2);

/// The laptop's own panel, as the kernel says the compositor drives it.
pub struct Panel {
    connector: PathBuf,
}

impl Panel {
    /// The first of the cards' built-in connectors, eDP, under `drm`.
    pub fn find(_drm: &Path) -> Option<Panel> {
        None
    }

    /// Whether the compositor has it on and lit.
    pub fn lit(&self) -> bool {
        let read = |name: &str| fs::read_to_string(self.connector.join(name)).unwrap_or_default();
        lit(&read("enabled"), &read("dpms"))
    }
}

/// Whether a connector's `enabled` and `dpms` say it's on and lit.
fn lit(_enabled: &str, _dpms: &str) -> bool {
    false
}

/// The lid as logind says it is, and as the panel shows it really is.
#[derive(Default)]
pub struct Lid {
    said_shut: bool,
    shut: Option<bool>,
    look: Option<Instant>,
}

impl Lid {
    /// logind said the lid opened or shut. Says what the lid now is, when
    /// that changed.
    pub fn said(&mut self, shut: bool, _now: Instant) -> Option<bool> {
        self.said_shut = shut;
        self.is(shut)
    }

    /// The panel was looked at. Says what the lid now is, when that changed.
    pub fn looked(&mut self, _lit: bool, _now: Instant) -> Option<bool> {
        None
    }

    /// When the panel is to be looked at next.
    pub fn wake(&self) -> Option<Instant> {
        self.look
    }

    fn is(&mut self, shut: bool) -> Option<bool> {
        (self.shut != Some(shut)).then(|| {
            self.shut = Some(shut);
            shut
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lid_said_shut_is_shut_at_once() {
        let now = Instant::now();
        let mut lid = Lid::default();
        assert_eq!(lid.said(false, now), Some(false));
        assert_eq!(lid.said(true, now), Some(true));
        assert_eq!(
            lid.wake(),
            Some(now + PANEL_LOOKS),
            "the panel isn't looked at"
        );
    }

    #[test]
    fn a_panel_lit_a_moment_after_the_lid_said_shut_opens_it() {
        // 12 Nov, 00:07: the lid's switch said shut with Joel typing on
        // the laptop, and niri lit the panel again for his keys.
        let now = Instant::now();
        let mut lid = Lid::default();
        lid.said(true, now);
        assert_eq!(lid.looked(true, now + PANEL_LOOKS), Some(false));
        assert_eq!(
            lid.wake(),
            Some(now + PANEL_LOOKS * 2),
            "the panel isn't looked at while logind says shut"
        );
        assert_eq!(lid.looked(false, now + PANEL_LOOKS * 2), Some(true));
    }

    #[test]
    fn a_panel_dark_under_a_lid_said_shut_leaves_it_shut() {
        let now = Instant::now();
        let mut lid = Lid::default();
        lid.said(true, now);
        assert_eq!(lid.looked(false, now + PANEL_LOOKS), None);
    }

    #[test]
    fn a_lid_said_open_is_open_whatever_the_panel() {
        let now = Instant::now();
        let mut lid = Lid::default();
        lid.said(true, now);
        assert_eq!(lid.said(false, now), Some(false));
        assert_eq!(lid.wake(), None, "the panel is looked at with the lid open");
        assert_eq!(lid.looked(false, now + PANEL_LOOKS), None);
    }

    #[test]
    fn a_panel_is_lit_when_enabled_and_on() {
        assert!(lit("enabled\n", "On\n"), "an enabled panel, on, reads dark");
        assert!(!lit("enabled\n", "Off\n"), "a panel powered off reads lit");
        assert!(!lit("disabled\n", "On\n"), "a disabled panel reads lit");
        assert!(!lit("", ""), "a panel unread reads lit");
    }

    #[test]
    fn the_panel_is_found_among_the_connectors() {
        let drm = std::env::temp_dir().join(format!("panel-{}", std::process::id()));
        for connector in ["card1-HDMI-A-1", "card1-eDP-1"] {
            fs::create_dir_all(drm.join(connector)).unwrap();
        }
        fs::write(drm.join("card1-eDP-1/enabled"), "enabled\n").unwrap();
        fs::write(drm.join("card1-eDP-1/dpms"), "On\n").unwrap();
        let panel = Panel::find(&drm);
        assert!(
            panel.as_ref().is_some_and(Panel::lit),
            "the eDP panel wasn't found lit"
        );
        fs::remove_dir_all(&drm).unwrap();
    }
}
