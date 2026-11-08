//! Joel's say over the built-in screen's warmth: whether it follows the
//! room, and whether the night light warms it after sunset, each a property
//! on the session's bus, which the shell's Warmth sets.
//!
//! ```sh
//! busctl --user set-property io.jitsusama.Backlight /io/jitsusama/Backlight \
//!   io.jitsusama.Backlight FollowsTheRoom b false
//! busctl --user set-property io.jitsusama.Backlight /io/jitsusama/Backlight \
//!   io.jitsusama.Backlight NightLight b false
//! ```
//!
//! What he says is kept in the daemon's state (main.rs), so it outlasts a
//! restart and a sign-in, as a setting does.

use tokio::sync::watch;
use zbus::{Connection, connection, interface};

const NAME: &str = "io.jitsusama.Backlight";
const PATH: &str = "/io/jitsusama/Backlight";

/// Whether a kept choice says on. Only "off" says not, so a missing or
/// spoiled file leaves it on, as it starts.
pub fn says(kept: &str) -> bool {
    kept.trim() != "off"
}

/// What a choice is kept as.
pub fn kept(on: bool) -> &'static str {
    if on { "on\n" } else { "off\n" }
}

struct Switch {
    follows: watch::Sender<bool>,
    night: watch::Sender<bool>,
}

#[interface(name = "io.jitsusama.Backlight")]
impl Switch {
    /// Whether the built-in screen's warmth follows the room's light.
    #[zbus(property)]
    fn follows_the_room(&self) -> bool {
        *self.follows.borrow()
    }

    #[zbus(property)]
    fn set_follows_the_room(&mut self, follows: bool) {
        self.follows.send_replace(follows);
    }

    /// Whether the night light warms the built-in screen after sunset.
    #[zbus(property)]
    fn night_light(&self) -> bool {
        *self.night.borrow()
    }

    #[zbus(property)]
    fn set_night_light(&mut self, on: bool) {
        self.night.send_replace(on);
    }
}

/// What Joel says through the switch: whether the warmth follows the room,
/// and whether the night light is on.
pub struct Said {
    pub follows: watch::Receiver<bool>,
    pub night: watch::Receiver<bool>,
}

/// Serves the switch, starting at `follows` and `night`. Gives the bus it
/// holds its name on, to be kept for as long as it serves, and what Joel
/// says through it.
pub async fn serve(follows: bool, night: bool) -> zbus::Result<(Connection, Said)> {
    let (follows, follows_heard) = watch::channel(follows);
    let (night, night_heard) = watch::channel(night);
    let bus = connection::Builder::session()?
        .name(NAME)?
        .serve_at(PATH, Switch { follows, night })?
        .build()
        .await?;
    Ok((
        bus,
        Said {
            follows: follows_heard,
            night: night_heard,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_off_turns_it_off() {
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
