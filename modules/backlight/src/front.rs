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
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;

/// How long after niri's stream ends, or can't be reached, it's asked
/// again.
pub const AGAIN: Duration = Duration::from_secs(2);

/// The app ids of programs that keep their own colours, as their windows
/// give them, lower case, besides GIMP's, which begin `gimp`: GIMP is the
/// one installed, and the others are the image editors a machine might
/// add, by the ids their own packages give.
const KEEP_THEIR_COLOURS: [&str; 7] = [
    "org.gimp.gimp",
    "darktable",
    "org.darktable.darktable",
    "krita",
    "org.kde.krita",
    "inkscape",
    "org.inkscape.inkscape",
];

/// Whether a window's app id is a program that keeps its own colours.
#[must_use]
pub fn keeps_its_colours(app_id: &str) -> bool {
    let id = app_id.to_ascii_lowercase();
    id.starts_with("gimp") || KEEP_THEIR_COLOURS.contains(&id.as_str())
}

/// Hears niri for as long as the daemon runs, and gives what it says is in
/// front; none where `NIRI_SOCKET` isn't set, as outside niri.
pub fn listen() -> Option<watch::Receiver<Option<String>>> {
    let Some(socket) = env::var_os("NIRI_SOCKET") else {
        eprintln!("<5>NIRI_SOCKET isn't set, so no program in front holds the screens' colours");
        return None;
    };
    let (tell, heard) = watch::channel(None);
    tokio::spawn(follow(PathBuf::from(socket), tell));
    Some(heard)
}

/// Waits for what's in front to change, or forever once nothing says.
pub async fn changed(heard: &mut Option<watch::Receiver<Option<String>>>) -> Option<String> {
    let Some(receiver) = heard else {
        return std::future::pending().await;
    };
    if receiver.changed().await.is_ok() {
        receiver.borrow_and_update().clone()
    } else {
        *heard = None;
        std::future::pending().await
    }
}

/// Hears niri's event stream, and asks again every [`AGAIN`] while niri's
/// gone, saying so once each time it goes.
async fn follow(socket: PathBuf, tell: watch::Sender<Option<String>>) {
    let mut front = Front::default();
    let mut said_gone = false;
    while !tell.is_closed() {
        match UnixStream::connect(&socket).await {
            Ok(stream) => {
                said_gone = false;
                hear(stream, &mut front, &tell).await;
            }
            Err(why) if !said_gone => {
                eprintln!("<4>niri isn't answering on {}: {why}", socket.display());
                said_gone = true;
            }
            Err(_) => {}
        }
        if front.lost() {
            tell.send_replace(None);
        }
        tokio::time::sleep(AGAIN).await;
    }
}

/// Hears each event of one stream until it ends.
async fn hear(stream: UnixStream, front: &mut Front, tell: &watch::Sender<Option<String>>) {
    // The writing half is held until the stream ends, since dropping it
    // would shut the socket's writing side.
    let (reading, mut writing) = stream.into_split();
    if writing.write_all(b"\"EventStream\"\n").await.is_err() {
        return;
    }
    let mut lines = BufReader::new(reading).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(why) = event.get("Err") {
            eprintln!("<4>niri refused its event stream: {why}");
            return;
        }
        if front.hear(&event) {
            tell.send_replace(front.in_front().map(str::to_owned));
        }
    }
}

/// The windows niri has said, and which has the focus.
#[derive(Default)]
pub struct Front {
    windows: HashMap<u64, String>,
    focused: Option<u64>,
    said: Option<String>,
}

impl Front {
    /// The program keeping its own colours in front, as last said.
    pub fn in_front(&self) -> Option<&str> {
        self.said.as_deref()
    }

    /// Hears one of niri's events, and says whether the program keeping
    /// its own colours in front changed.
    pub fn hear(&mut self, event: &Value) -> bool {
        if let Some(all) = event.get("WindowsChanged") {
            self.windows.clear();
            self.focused = None;
            let windows = all.get("windows").and_then(Value::as_array);
            for window in windows.into_iter().flatten() {
                self.window(window);
            }
        } else if let Some(window) = event.get("WindowOpenedOrChanged") {
            if let Some(window) = window.get("window") {
                self.window(window);
            }
        } else if let Some(closed) = event.get("WindowClosed") {
            if let Some(id) = closed.get("id").and_then(Value::as_u64) {
                self.windows.remove(&id);
                if self.focused == Some(id) {
                    self.focused = None;
                }
            }
        } else if let Some(focus) = event.get("WindowFocusChanged") {
            self.focused = focus.get("id").and_then(Value::as_u64);
        } else {
            return false;
        }
        self.said()
    }

    /// niri's stream ended, so nothing is known to be in front. Says
    /// whether that changed what was.
    pub fn lost(&mut self) -> bool {
        self.windows.clear();
        self.focused = None;
        self.said()
    }

    /// Keeps a window niri described, and whether it has the focus.
    fn window(&mut self, window: &Value) {
        let Some(id) = window.get("id").and_then(Value::as_u64) else {
            return;
        };
        let app_id = window.get("app_id").and_then(Value::as_str);
        self.windows
            .insert(id, app_id.unwrap_or_default().to_owned());
        match window.get("is_focused").and_then(Value::as_bool) {
            Some(true) => self.focused = Some(id),
            Some(false) if self.focused == Some(id) => self.focused = None,
            _ => {}
        }
    }

    /// Takes the program keeping its own colours in front now, and says
    /// whether that's not what was last said.
    fn said(&mut self) -> bool {
        let now = self
            .focused
            .and_then(|id| self.windows.get(&id))
            .filter(|app_id| keeps_its_colours(app_id))
            .cloned();
        let changed = now != self.said;
        self.said = now;
        changed
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
        assert!(!begun, "a terminal in front was said to keep its colours");
        assert_eq!(
            front.in_front(),
            None,
            "a terminal was taken for an image editor"
        );
        front
    }

    /// Hears an event, and gives whether it changed what's in front and
    /// what's in front after it.
    fn heard<'a>(front: &'a mut Front, event: &Value) -> (bool, Option<&'a str>) {
        let changed = front.hear(event);
        (changed, front.in_front())
    }

    /// Its stream's end, as `heard`.
    fn lost(front: &mut Front) -> (bool, Option<&str>) {
        let changed = front.lost();
        (changed, front.in_front())
    }

    const GIMP: Option<&str> = Some("gimp");

    #[test]
    fn gimp_given_the_focus_is_in_front_until_it_loses_it() {
        let mut front = desktop();
        assert_eq!(
            heard(&mut front, &json!({"WindowFocusChanged": {"id": 2}})),
            (true, GIMP),
            "GIMP given the focus wasn't said"
        );
        assert_eq!(
            heard(&mut front, &json!({"WindowFocusChanged": {"id": 2}})),
            (false, GIMP),
            "the same focus was said again"
        );
        assert_eq!(
            heard(&mut front, &json!({"WindowFocusChanged": {"id": 1}})),
            (true, None),
            "GIMP losing the focus wasn't said"
        );
    }

    #[test]
    fn gimp_opening_with_the_focus_is_in_front() {
        let mut front = desktop();
        assert_eq!(
            heard(
                &mut front,
                &json!({"WindowOpenedOrChanged": {"window": window(3, "gimp", true)}})
            ),
            (true, GIMP),
            "a GIMP window opened in front wasn't said"
        );
    }

    #[test]
    fn gimp_in_front_closing_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            heard(&mut front, &json!({"WindowClosed": {"id": 2}})),
            (true, None),
            "GIMP closing left it in front"
        );
    }

    #[test]
    fn the_focus_on_nothing_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            heard(&mut front, &json!({"WindowFocusChanged": {"id": null}})),
            (true, None),
            "an empty workspace left GIMP in front"
        );
    }

    #[test]
    fn niri_gone_leaves_nothing_in_front() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            lost(&mut front),
            (true, None),
            "GIMP stayed in front of no niri"
        );
        assert_eq!(
            lost(&mut front),
            (false, None),
            "nothing in front was said twice"
        );
    }

    #[test]
    fn events_about_anything_else_say_nothing() {
        let mut front = desktop();
        let _ = front.hear(&json!({"WindowFocusChanged": {"id": 2}}));
        assert_eq!(
            heard(
                &mut front,
                &json!({"WorkspacesChanged": {"workspaces": []}})
            ),
            (false, GIMP),
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
