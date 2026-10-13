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
use std::net::IpAddr;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process;

use futures_util::stream;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};
use zlink::introspect::{CustomType, Type};
use zlink::tokio::unix::Listener;
use zlink::{Reply, introspect, service};

use crate::state::{Card, Described, Nearby, Reach, Saved, Security, State, Trouble};

pub struct Wifi {
    pub state: watch::Receiver<State>,
    /// Counted up to have every card's network checked again now.
    pub asked: watch::Sender<u64>,
    /// Where wifid's main loop takes what only it can do.
    pub asks: mpsc::Sender<Ask>,
}

/// A change to the saved networks, which wifid's main loop makes, since
/// it gives wpa_supplicant and networkd every network, with where to say
/// how it went.
#[derive(Debug)]
pub enum Ask {
    /// Joining a network, with where to say each step and how it ended.
    Join {
        name: String,
        password: Option<String>,
        progress: mpsc::Sender<Result<Stage, Refused>>,
    },
    Scan {
        done: oneshot::Sender<Result<(), Refused>>,
    },
    Leave {
        name: String,
        done: oneshot::Sender<Result<(), Refused>>,
    },
    Lookup {
        card: String,
        name: String,
        done: oneshot::Sender<Result<Vec<IpAddr>, Refused>>,
    },
    Save {
        saving: Saving,
        done: oneshot::Sender<Result<(), Refused>>,
    },
    Radio {
        on: bool,
        done: oneshot::Sender<Result<(), Refused>>,
    },
    Forget {
        name: String,
        done: oneshot::Sender<Result<(), Refused>>,
    },
    Trust {
        name: String,
        trusted: bool,
        done: oneshot::Sender<Result<(), Refused>>,
    },
}

/// Why wifid won't, or couldn't.
#[derive(Debug, Clone, PartialEq, zlink::ReplyError, introspect::ReplyError)]
#[zlink(interface = "io.jitsusama.Wifi")]
pub enum Refused {
    /// Nix declares the network, so only a rebuild changes or forgets it.
    Declared,
    /// No network of that name is saved.
    NotSaved,
    /// The network asks for a password, and none was given.
    NeedsPassword,
    /// The network isn't saved, and no card hears it.
    NotNearby,
    /// The password didn't work.
    WrongPassword,
    /// The access point turned the card away.
    TurnedAway,
    /// The card didn't join the network in time.
    TimedOut,
    /// No card is on the network.
    NotJoined,
    /// wifid tried and couldn't, for this reason.
    Failed { reason: String },
}

/// A name's addresses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Found {
    pub addresses: Vec<String>,
}

/// A network to save as the shell asks, with anything left out as it was.
#[derive(Debug, PartialEq)]
pub struct Saving {
    pub name: String,
    pub security: Security,
    pub password: Option<String>,
    pub hidden: Option<bool>,
    pub priority: Option<i32>,
    pub address: Option<Address>,
}

/// The address a network sees of the card.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Address {
    /// One of the card's own for that network, the same each time.
    Network,
    /// The card's own, as it came.
    Card,
    /// A new one each time the card joins it.
    Random,
}

/// How far joining has got.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Joining {
    pub stage: Stage,
}

/// A step in joining a network. The last is how far the network reaches.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Stage {
    /// Finding the network and agreeing how to join it.
    Joining,
    /// On the network and waiting for an address.
    GettingAddress,
    /// Finding out how far the network reaches.
    Checking,
    /// Beyond itself.
    Online,
    /// Nowhere until someone signs in.
    SignIn,
    /// Nowhere, or nowhere sensible.
    Limited,
}

impl Stage {
    /// Whether joining is done.
    pub fn is_final(self) -> bool {
        matches!(self, Stage::Online | Stage::SignIn | Stage::Limited)
    }
}

#[service(
    interface = "io.jitsusama.Wifi",
    types = [Address, Card, Nearby, Reach, Saved, Security, Stage, Trouble],
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

    /// Joins a network, saving it first when it's new or given a password.
    /// Called with more, it says each step as the card takes it. Either way
    /// it ends with how far the network reaches, or why the card couldn't
    /// join it.
    #[zlink(more)]
    async fn join(
        &self,
        more: bool,
        name: String,
        password: Option<String>,
    ) -> impl Stream<Item = Result<Reply<Joining>, Refused>> + Unpin {
        let (progress, steps) = mpsc::channel(8);
        let join = Ask::Join {
            name,
            password,
            progress,
        };
        let asked = self.asks.send(join).await.is_ok();
        let replies = stream::unfold((steps, asked), move |(mut steps, going)| async move {
            if !going {
                return None;
            }
            loop {
                let stage = match steps.recv().await {
                    Some(Ok(stage)) => stage,
                    Some(Err(refused)) => return Some((Err(refused), (steps, false))),
                    None => return Some((Err(stopping()), (steps, false))),
                };
                let last = stage.is_final();
                if more || last {
                    let reply = Reply::new(Some(Joining { stage })).set_continues(Some(!last));
                    return Some((Ok(reply), (steps, !last)));
                }
            }
        });
        // wifid stopping before it was even asked ends the stream at once.
        let unasked = (!asked).then(|| Err(stopping()));
        Box::pin(stream::iter(unasked).chain(replies))
    }

    /// Has every card look for networks now. Describe says what they hear.
    async fn scan(&self) -> Result<(), Refused> {
        self.ask(|done| Ask::Scan { done }).await
    }

    /// Saves a network without joining it, as one that hides its name or one
    /// brought from another machine, or changes one saved from the launcher.
    /// A new one is shown, of priority 0, and sees an address of the card's
    /// own for it, unless asked otherwise.
    async fn save(
        &self,
        name: String,
        security: Security,
        password: Option<String>,
        hidden: Option<bool>,
        priority: Option<i32>,
        address: Option<Address>,
    ) -> Result<(), Refused> {
        let saving = Saving {
            name,
            security,
            password,
            hidden,
            priority,
            address,
        };
        self.ask(|done| Ask::Save { saving, done }).await
    }

    /// The addresses a card's network says a name has, asked of the servers
    /// the network handed out, through the card, whatever a VPN has put in
    /// their place: the sign-in window reaches a network's sign-in page so.
    async fn lookup(&self, card: String, name: String) -> Result<Found, Refused> {
        let (done, answer) = oneshot::channel();
        let asked = Ask::Lookup { card, name, done };
        self.asks.send(asked).await.map_err(|_| stopping())?;
        let found = answer.await.map_err(|_| stopping())??;
        Ok(Found {
            addresses: found.iter().map(IpAddr::to_string).collect(),
        })
    }

    /// Takes every card on the network off it. They join nothing more
    /// until asked to.
    async fn leave(&self, name: String) -> Result<(), Refused> {
        self.ask(|done| Ask::Leave { name, done }).await
    }

    /// Turns Wi-Fi's radios on or off, every one at once.
    async fn radio(&self, on: bool) -> Result<(), Refused> {
        self.ask(|done| Ask::Radio { on, done }).await
    }

    /// Forgets a network saved from the launcher, with its password.
    async fn forget(&self, name: String) -> Result<(), Refused> {
        self.ask(|done| Ask::Forget { name, done }).await
    }

    /// Trusts a network saved from the launcher, so the machine can be
    /// found on it and find others there, or stops trusting it.
    async fn trust(&self, name: String, trusted: bool) -> Result<(), Refused> {
        self.ask(|done| Ask::Trust {
            name,
            trusted,
            done,
        })
        .await
    }
}

impl Wifi {
    /// Asks wifid's main loop and waits for its answer.
    async fn ask(
        &self,
        ask: impl FnOnce(oneshot::Sender<Result<(), Refused>>) -> Ask,
    ) -> Result<(), Refused> {
        let (done, answer) = oneshot::channel();
        self.asks.send(ask(done)).await.map_err(|_| stopping())?;
        answer.await.map_err(|_| stopping())?
    }
}

fn stopping() -> Refused {
    Refused::Failed {
        reason: "wifid is stopping".into(),
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
        let (wifi, state, checks, _) = asking();
        (wifi, state, checks)
    }

    fn asking() -> (
        Wifi,
        watch::Sender<State>,
        watch::Receiver<u64>,
        mpsc::Receiver<Ask>,
    ) {
        let (state, shown) = watch::channel(State::default());
        let (asked, checks) = watch::channel(0);
        let (asks, taken) = mpsc::channel(1);
        let wifi = Wifi {
            state: shown,
            asked,
            asks,
        };
        (wifi, state, checks, taken)
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

    /// Joins as the shell would, with wifid's main loop saying these.
    async fn joined(
        more: bool,
        said: Vec<Result<Stage, Refused>>,
    ) -> (
        Vec<Result<Reply<Joining>, Refused>>,
        (String, Option<String>),
    ) {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Join {
                name,
                password,
                progress,
            }) = taken.recv().await
            else {
                panic!("wasn't asked to join");
            };
            for step in said {
                progress.send(step).await.unwrap();
            }
            (name, password)
        });

        let replies = wifi.join(more, "Lantern".into(), Some("pass".into())).await;
        drop(wifi);
        let replies = replies.collect().await;

        (replies, main_loop.await.unwrap())
    }

    fn stages(replies: &[Result<Reply<Joining>, Refused>]) -> Vec<(Stage, Option<bool>)> {
        replies
            .iter()
            .map(|reply| {
                let reply = reply.as_ref().unwrap();
                (reply.parameters().unwrap().stage, reply.continues())
            })
            .collect()
    }

    #[tokio::test]
    async fn joining_with_more_says_each_step_until_how_far_the_network_reaches() {
        let said = vec![
            Ok(Stage::Joining),
            Ok(Stage::GettingAddress),
            Ok(Stage::Checking),
            Ok(Stage::SignIn),
        ];

        let (replies, asked) = joined(true, said).await;

        assert_eq!(asked, ("Lantern".into(), Some("pass".into())));
        assert_eq!(
            stages(&replies),
            [
                (Stage::Joining, Some(true)),
                (Stage::GettingAddress, Some(true)),
                (Stage::Checking, Some(true)),
                (Stage::SignIn, Some(false)),
            ]
        );
    }

    #[tokio::test]
    async fn joining_without_more_answers_once_with_how_far_it_reaches() {
        let said = vec![Ok(Stage::Joining), Ok(Stage::Checking), Ok(Stage::Online)];

        let (replies, _) = joined(false, said).await;

        assert_eq!(stages(&replies), [(Stage::Online, Some(false))]);
    }

    #[tokio::test]
    async fn joining_ends_with_why_the_card_couldn_t() {
        let said = vec![Ok(Stage::Joining), Err(Refused::WrongPassword)];

        let (replies, _) = joined(true, said).await;

        assert_eq!(stages(&replies[..1]), [(Stage::Joining, Some(true))]);
        assert!(
            matches!(replies[1..], [Err(Refused::WrongPassword)]),
            "{replies:?}"
        );
    }

    #[tokio::test]
    async fn joining_cut_short_by_wifid_stopping_says_so() {
        let (replies, _) = joined(true, vec![Ok(Stage::Joining)]).await;

        assert!(
            matches!(replies.last(), Some(Err(Refused::Failed { .. }))),
            "{replies:?}"
        );
    }

    #[tokio::test]
    async fn scanning_asks_wifid_to_have_every_card_look() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Scan { done }) = taken.recv().await else {
                panic!("wasn't asked to scan");
            };
            done.send(Ok(())).unwrap();
        });

        let answer = wifi.scan().await;
        drop(wifi);

        assert_eq!(answer, Ok(()));
        main_loop.await.unwrap();
    }

    #[tokio::test]
    async fn saving_asks_wifid_to_keep_what_was_given() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Save { saving, done }) = taken.recv().await else {
                panic!("wasn't asked to save");
            };
            done.send(Ok(())).unwrap();
            saving
        });

        let answer = wifi
            .save(
                "Hidden Hall".into(),
                Security::Personal,
                Some("under the stairs".into()),
                Some(true),
                None,
                Some(Address::Card),
            )
            .await;
        drop(wifi);

        assert_eq!(answer, Ok(()));
        assert_eq!(
            main_loop.await.unwrap(),
            Saving {
                name: "Hidden Hall".into(),
                security: Security::Personal,
                password: Some("under the stairs".into()),
                hidden: Some(true),
                priority: None,
                address: Some(Address::Card),
            }
        );
    }

    #[tokio::test]
    async fn looking_a_name_up_asks_wifid_through_which_card() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Lookup { card, name, done }) = taken.recv().await else {
                panic!("wasn't asked to look a name up");
            };
            let found = vec!["10.43.0.1".parse().unwrap(), "fd00::1".parse().unwrap()];
            done.send(Ok(found)).unwrap();
            (card, name)
        });

        let answer = wifi.lookup("wlan2".into(), "front-desk.inn".into()).await;
        drop(wifi);

        assert_eq!(
            answer,
            Ok(Found {
                addresses: vec!["10.43.0.1".into(), "fd00::1".into()]
            })
        );
        assert_eq!(
            main_loop.await.unwrap(),
            ("wlan2".into(), "front-desk.inn".into())
        );
    }

    #[tokio::test]
    async fn leaving_says_why_wifid_couldn_t() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Leave { name, done }) = taken.recv().await else {
                panic!("wasn't asked to leave");
            };
            done.send(Err(Refused::NotJoined)).unwrap();
            name
        });

        let answer = wifi.leave("Lantern".into()).await;
        drop(wifi);

        assert_eq!(answer, Err(Refused::NotJoined));
        assert_eq!(main_loop.await.unwrap(), "Lantern");
    }

    #[tokio::test]
    async fn turning_wifi_off_asks_wifid_to() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Radio { on, done }) = taken.recv().await else {
                panic!("wasn't asked to turn the radios");
            };
            done.send(Ok(())).unwrap();
            on
        });

        let answer = wifi.radio(false).await;
        drop(wifi);

        assert_eq!(answer, Ok(()));
        assert!(!main_loop.await.unwrap());
    }

    #[tokio::test]
    async fn forgetting_asks_wifid_and_says_it_s_done_once_it_is() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Forget { name, done }) = taken.recv().await else {
                panic!("wasn't asked to forget");
            };
            done.send(Ok(())).unwrap();
            name
        });

        let answer = wifi.forget("Lantern".into()).await;
        drop(wifi);

        assert_eq!(answer, Ok(()));
        assert_eq!(main_loop.await.unwrap(), "Lantern");
    }

    #[tokio::test]
    async fn trusting_answers_with_why_wifid_won_t() {
        let (wifi, _, _, mut taken) = asking();
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Trust {
                name,
                trusted,
                done,
            }) = taken.recv().await
            else {
                panic!("wasn't asked to trust");
            };
            done.send(Err(Refused::Declared)).unwrap();
            (name, trusted)
        });

        let answer = wifi.trust("Roadside Inn".into(), true).await;
        drop(wifi);

        assert_eq!(answer, Err(Refused::Declared));
        assert_eq!(main_loop.await.unwrap(), ("Roadside Inn".into(), true));
    }

    #[tokio::test]
    async fn asking_once_wifid_has_stopped_says_it_couldn_t() {
        let (wifi, _, _, taken) = asking();
        drop(taken);

        let answer = wifi.forget("Lantern".into()).await;

        assert!(matches!(answer, Err(Refused::Failed { .. })), "{answer:?}");
    }
}
