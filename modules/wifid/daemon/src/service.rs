//! io.jitsusama.Wifi, which the shell and varlinkctl talk to wifid through,
//! over varlink on the socket systemd holds for wifid. systemd keeps the
//! socket while wifid starts again, so nobody talking to it has to wait and
//! try again; they only ask again on the connection they had.
//!
//! ```sh
//! varlinkctl introspect /run/wifid/io.jitsusama.Wifi
//! varlinkctl call --more /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi.Describe '{}'
//! ```

use std::env;
use std::io;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process;

use futures_util::Stream;
use futures_util::stream;
use tokio::sync::watch;
use zlink::tokio::unix::Listener;
use zlink::{Reply, service};

use crate::state::{Card, Described, Reach, Saved, Security, State};

pub struct Wifi {
    pub state: watch::Receiver<State>,
    /// Counted up to have every card's network checked again now.
    pub asked: watch::Sender<u64>,
}

#[service(
    interface = "io.jitsusama.Wifi",
    types = [Card, Reach, Saved, Security],
    vendor = "jitsusama",
    product = "wifid",
    version = env!("CARGO_PKG_VERSION"),
    url = "https://github.com/Jitsusama/core.nix"
)]
impl Wifi {
    /// Every card and every saved network. Called with more, it answers
    /// again with all of it each time any of it changes.
    #[zlink(more)]
    async fn describe(&self, more: bool) -> impl Stream<Item = Reply<Described>> + Unpin {
        let mut state = self.state.clone();
        let now = state.borrow_and_update().described();
        let replies = stream::unfold((state, Some(now)), move |(mut state, next)| async move {
            let described = match next {
                Some(described) => described,
                None if more => {
                    state.changed().await.ok()?;
                    state.borrow_and_update().described()
                }
                None => return None,
            };
            let reply = Reply::new(Some(described)).set_continues(Some(more));
            Some((reply, (state, None)))
        });
        Box::pin(replies)
    }

    /// Checks every card's network again now. Describe says what it found
    /// once it has.
    async fn check(&self) {
        self.asked.send_modify(|asked| *asked += 1);
    }
}

/// The socket systemd holds for wifid, if it passed one, as the first of
/// the files it hands a service.
pub fn listener() -> io::Result<Option<Listener>> {
    let ours = env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Ok(None);
    }
    // SAFETY: systemd handed this process the socket as file 3, its first,
    // and nothing else in wifid takes it.
    let socket = unsafe { OwnedFd::from_raw_fd(3) };
    let listener =
        Listener::try_from(socket).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(Some(listener))
}

#[cfg(test)]
mod tests {
    use futures_util::{FutureExt, StreamExt};

    use super::*;
    use crate::link::Link;

    fn wifi() -> (Wifi, watch::Sender<State>, watch::Receiver<u64>) {
        let (state, shown) = watch::channel(State::default());
        let (asked, checks) = watch::channel(0);
        (
            Wifi {
                state: shown,
                asked,
            },
            state,
            checks,
        )
    }

    fn inn() -> Link {
        Link {
            index: 2,
            name: "wlan2".into(),
            ssid: Some("Roadside Inn".into()),
            routable: true,
            dns: Vec::new(),
            portal: None,
        }
    }

    #[tokio::test]
    async fn describing_once_answers_with_everything_as_it_is() {
        let (wifi, state, _) = wifi();
        state.send_modify(|state| state.follow(&inn()));

        let mut replies = wifi.describe(false).await;
        let reply = replies.next().await.unwrap();

        assert_eq!(reply.continues(), Some(false));
        assert_eq!(reply.parameters(), Some(&state.borrow().described()));
        assert!(replies.next().await.is_none());
    }

    #[tokio::test]
    async fn describing_with_more_answers_again_each_time_anything_changes() {
        let (wifi, state, _) = wifi();

        let mut replies = wifi.describe(true).await;
        let first = replies.next().await.unwrap();
        state.send_modify(|state| state.follow(&inn()));
        let second = replies.next().await.unwrap();
        let unchanged = replies.next().now_or_never();

        assert_eq!(first.continues(), Some(true));
        assert_eq!(first.parameters().unwrap().cards, []);
        assert_eq!(second.continues(), Some(true));
        assert_eq!(second.parameters().unwrap().cards[0].name, "wlan2");
        assert!(unchanged.is_none(), "answered again with nothing changed");
    }

    #[tokio::test]
    async fn checking_asks_for_every_card_s_network_to_be_checked_again() {
        let (wifi, _, mut checks) = wifi();

        wifi.check().await;

        assert!(checks.has_changed().unwrap());
        assert_eq!(*checks.borrow_and_update(), 1);
    }
}
