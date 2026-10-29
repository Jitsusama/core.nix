//! Joel's say over whether the built-in screen's warmth follows the room: a
//! property on the session's bus, which the shell's Warmth part sets.
//!
//! ```sh
//! busctl --user set-property io.jitsusama.Backlight /io/jitsusama/Backlight \
//!   io.jitsusama.Backlight FollowsTheRoom b false
//! ```
//!
//! What he says is kept in the daemon's state (main.rs), so it outlasts a
//! restart and a sign-in, as a setting does.

use tokio::sync::watch;
use zbus::{Connection, connection, interface};

const NAME: &str = "io.jitsusama.Backlight";
const PATH: &str = "/io/jitsusama/Backlight";

/// Whether a kept choice says to follow the room. Only "off" says not to,
/// so a missing or spoiled file leaves the warmth following, as it starts.
pub fn says(kept: &str) -> bool {
    kept.trim() != "off"
}

/// What a choice is kept as.
pub fn kept(follows: bool) -> &'static str {
    if follows { "on\n" } else { "off\n" }
}

struct Switch {
    said: watch::Sender<bool>,
}

#[interface(name = "io.jitsusama.Backlight")]
impl Switch {
    /// Whether the built-in screen's warmth follows the room's light.
    #[zbus(property)]
    fn follows_the_room(&self) -> bool {
        *self.said.borrow()
    }

    #[zbus(property)]
    fn set_follows_the_room(&mut self, follows: bool) {
        self.said.send_replace(follows);
    }
}

/// Serves the switch, starting at `follows`. Gives the bus it holds its name
/// on, to be kept for as long as it serves, and what Joel says through it.
pub async fn serve(follows: bool) -> zbus::Result<(Connection, watch::Receiver<bool>)> {
    let (said, heard) = watch::channel(follows);
    let bus = connection::Builder::session()?
        .name(NAME)?
        .serve_at(PATH, Switch { said })?
        .build()
        .await?;
    Ok((bus, heard))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_off_stops_the_warmth_following() {
        assert!(!says("off\n"));
        assert!(says("on\n"));
        assert!(says(""), "nothing kept yet");
        assert!(says("of"), "a spoiled file");
    }

    #[test]
    fn a_choice_is_read_back_as_kept() {
        assert!(says(kept(true)));
        assert!(!says(kept(false)));
    }
}
