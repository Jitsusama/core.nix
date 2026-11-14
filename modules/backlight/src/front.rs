//! Which program is in front, as niri says, when it's one that keeps its
//! own colours, such as GIMP. Such a program sets each colour exactly, so
//! a screen warmed for the night or the room, or brightened and dimmed
//! with the light, shows Joel colours other than those he's choosing: the
//! light research asks that the screens hold their own white and their
//! brightness while one is in front (answering.rs does).
//!
//! niri's event stream, on `NIRI_SOCKET`, describes every window as it
//! begins, then says each window opened, changed or closed, and each move
//! of the focus. While niri is gone, as between one niri and the next,
//! nothing is in front, and it's asked again every two seconds.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::Value;

/// How long after niri's stream ends, or can't be reached, it's asked
/// again.
pub const AGAIN: Duration = Duration::from_secs(2);

/// Whether a window's app id is a program that keeps its own colours.
#[must_use]
pub fn keeps_its_colours(app_id: &str) -> bool {
    let _ = app_id;
    false
}

/// The windows niri has said, and which has the focus.
#[derive(Default)]
pub struct Front {
    windows: HashMap<u64, String>,
    focused: Option<u64>,
    said: Option<String>,
}

impl Front {
    /// Hears one of niri's events. Gives the program keeping its own
    /// colours now in front, or none, when that changed.
    pub fn hear(&mut self, event: &Value) -> Option<Option<String>> {
        let _ = event;
        None
    }

    /// niri's stream ended, so nothing is known to be in front. Gives none
    /// when that changed.
    pub fn lost(&mut self) -> Option<Option<String>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// niri's description of a window, as its stream gives one.
    fn window(id: u64, app_id: &str, is_focused: bool) -> Value {
        json!({"id": id, "app_id": app_id, "title": "", "is_focused": is_focused})
    }

    /// A desktop with a terminal in front and GIMP behind it.
    fn desktop() -> Front {
        let mut front = Front::default();
        let begun = front.hear(&json!({"WindowsChanged": {"windows": [
            window(1, "kitty", true),
            window(2, "gimp", false),
        ]}}));
        assert_eq!(
            begun, None,
            "a terminal in front was said to keep its colours"
        );
        front
    }

    #[test]
    fn gimp_given_the_focus_is_in_front_until_it_loses_it() {
        let mut front = desktop();
        assert_eq!(
            front.hear(&json!({"WindowFocusChanged": {"id": 2}})),
            Some(Some("gimp".to_owned())),
            "GIMP given the focus wasn't said"
        );
        assert_eq!(
            front.hear(&json!({"WindowFocusChanged": {"id": 2}})),
            None,
            "the same focus was said again"
        );
        assert_eq!(
            front.hear(&json!({"WindowFocusChanged": {"id": 1}})),
            Some(None),
            "GIMP losing the focus wasn't said"
        );
    }

    #[test]
    fn gimp_opening_with_the_focus_is_in_front() {
        let mut front = desktop();
        assert_eq!(
            front.hear(&json!({"WindowOpenedOrChanged": {"window": window(3, "gimp", true)}})),
            Some(Some("gimp".to_owned())),
            "a GIMP window opened in front wasn't said"
        );
    }

    #[test]
    fn gimp_in_front_closing_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            front.hear(&json!({"WindowClosed": {"id": 2}})),
            Some(None),
            "GIMP closing left it in front"
        );
    }

    #[test]
    fn the_focus_on_nothing_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            front.hear(&json!({"WindowFocusChanged": {"id": null}})),
            Some(None),
            "an empty workspace left GIMP in front"
        );
    }

    #[test]
    fn niri_gone_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(front.lost(), Some(None), "GIMP stayed in front of no niri");
        assert_eq!(front.lost(), None, "nothing in front was said twice");
    }

    #[test]
    fn events_about_anything_else_say_nothing() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            front.hear(&json!({"WorkspacesChanged": {"workspaces": []}})),
            None,
            "a workspace's change moved what's in front"
        );
    }

    #[test]
    fn image_editors_keep_their_colours_and_others_do_not() {
        for app_id in [
            "gimp",
            "gimp-3.2",
            "org.gimp.GIMP",
            "darktable",
            "org.kde.krita",
        ] {
            assert!(
                keeps_its_colours(app_id),
                "{app_id} was taken for any program"
            );
        }
        for app_id in ["kitty", "firefox", "google-chrome", ""] {
            assert!(
                !keeps_its_colours(app_id),
                "{app_id} was taken for an image editor"
            );
        }
    }
}
