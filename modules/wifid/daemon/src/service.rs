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

use crate::detail::{Best, Count, Detail, Joined, Point};
use crate::radio::{Band, Load, Management, Protection};
use crate::state::{Card, Described, Nearby, Reach, Saved, Security, State, Trouble};

pub struct Wifi {
    pub state: watch::Receiver<State>,
    /// Counted up to have every card's network checked again now.
    pub asked: watch::Sender<u64>,
    /// Where wifid's main loop takes what only it can do.
    pub asks: mpsc::Sender<Ask>,
    /// How many are asking for a network in detail as it changes, while
    /// which the cards on networks read their links.
    pub watching: watch::Sender<u32>,
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
        account: Option<Account>,
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
    /// The network's server couldn't prove it's the network's, so it was
    /// given no password.
    ServerUnproven,
    /// The network asks for an account, and none was given.
    NeedsAccount,
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Saving {
    pub name: String,
    pub security: Security,
    pub password: Option<String>,
    pub hidden: Option<bool>,
    pub priority: Option<i32>,
    pub address: Option<Address>,
    pub account: Option<Account>,
}

/// An account an enterprise network is joined with. Its server proves
/// itself with a certificate the machine's authorities issued that names
/// the server's domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Account {
    pub method: Method,
    pub identity: String,
    /// The name said outside the tunnel, where anyone can hear it.
    pub anonymous_identity: Option<String>,
    pub server_domain: String,
    /// How the password is sent inside the tunnel: MSCHAPv2 for PEAP and
    /// PAP for TTLS, unless given.
    pub inner: Option<Inner>,
}

/// The tunnel an enterprise network's password goes through.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Method {
    Peap,
    Ttls,
}

/// How the password is sent inside the tunnel.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Inner {
    Mschapv2,
    Pap,
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
    types = [
        Account, Address, Band, Best, Card, Count, Inner, Joined, Load, Management, Method, Nearby,
        Point, Protection, Reach, Saved, Saving, Security, Stage, Trouble
    ],
    vendor = "jitsusama",
    product = "wifid",
    version = env!("CARGO_PKG_VERSION"),
    url = "https://github.com/Jitsusama/core.nix"
)]
impl Wifi {
    /// Every card and every saved network. Called with more, it answers
    /// again with all of it each time any of it changes.
    #[zlink(more)]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn describe(&self, more: bool) -> impl Stream<Item = Reply<Described>> + Unpin {
        let mut state = self.state.clone();
        let now = state.borrow_and_update().described();
        // What was sent last, so a change wifid makes that the shell can't
        // see, as a signal wandering within a bar, sends nothing.
        let replies = stream::unfold(
            (state, Some(now), None),
            move |(mut state, next, sent): (_, _, Option<Described>)| async move {
                let described = match next {
                    Some(described) => described,
                    None if more => loop {
                        state.changed().await.ok()?;
                        let described = state.borrow_and_update().described();
                        if sent.as_ref() != Some(&described) {
                            break described;
                        }
                    },
                    None => return None,
                };
                let reply = Reply::new(Some(described.clone())).set_continues(Some(more));
                Some((reply, (state, None, Some(described))))
            },
        );
        Box::pin(replies)
    }

    /// One network in the air in detail: each of its access points, how
    /// many in each band, what each card on it reads of its link, and the
    /// access point wifid would pick, with why. Called with more, it
    /// answers again each time any of it changes, and the cards on
    /// networks read their links every two seconds until the call ends.
    #[zlink(more)]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn detail(&self, more: bool, name: String) -> impl Stream<Item = Reply<Detail>> + Unpin {
        // Kept in the stream, so the count goes down as the call ends.
        let watching = more.then(|| Watching::new(&self.watching));
        let mut state = self.state.clone();
        let now = state.borrow_and_update().detail(&name);
        let replies = stream::unfold(
            (state, Some(now), None, watching),
            move |(mut state, next, sent, watching): (_, _, Option<Detail>, _)| {
                let name = name.clone();
                async move {
                    let detail = match next {
                        Some(detail) => detail,
                        // zlink learns a caller has gone only as it next
                        // answers them, so the detail is said again now and
                        // then unchanged, and the cards stop reading their
                        // links soon after the launcher closes.
                        None if more => loop {
                            tokio::select! {
                                changed = state.changed() => changed.ok()?,
                                () = tokio::time::sleep(AGAIN) => {
                                    break state.borrow_and_update().detail(&name);
                                }
                            }
                            let detail = state.borrow_and_update().detail(&name);
                            if sent.as_ref() != Some(&detail) {
                                break detail;
                            }
                        },
                        None => return None,
                    };
                    let reply = Reply::new(Some(detail.clone())).set_continues(Some(more));
                    Some((reply, (state, None, Some(detail), watching)))
                }
            },
        );
        Box::pin(replies)
    }

    /// Answers once the machine is as online as it's going to get for now:
    /// a card or cable reaches beyond the machine, Wi-Fi is off, or every
    /// card has looked and heard no saved network it can join. The boot
    /// waits on this rather than on a timeout.
    #[zlink(more)]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn settled(&self, _more: bool) -> impl Stream<Item = Reply<()>> + Unpin {
        // Waiting in a stream rather than in the call leaves wifid free to
        // answer every other call meanwhile.
        let mut state = self.state.clone();
        let settled = async move {
            // Once wifid stops, nothing will settle, and nothing need wait.
            let _ = state.wait_for(State::settled).await;
            Reply::new(None).set_continues(Some(false))
        };
        Box::pin(stream::once(settled))
    }

    /// Checks every card's network again now. Describe says what it found
    /// once it has.
    #[expect(
        clippy::unused_async,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn check(&self) {
        self.asked.send_modify(|asked| *asked += 1);
    }

    /// Joins a network, saving it first when it's new or given a password,
    /// or an account where it asks for one. Called with more, it says each
    /// step as the card takes it. Either way it ends with how far the
    /// network reaches, or why the card couldn't join it.
    #[zlink(more)]
    async fn join(
        &self,
        more: bool,
        name: String,
        password: Option<String>,
        account: Option<Account>,
    ) -> impl Stream<Item = Result<Reply<Joining>, Refused>> + Unpin {
        let (progress, steps) = mpsc::channel(8);
        let join = Ask::Join {
            name,
            password,
            account,
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
    async fn save(&self, network: Saving) -> Result<(), Refused> {
        self.ask(|done| Ask::Save {
            saving: network,
            done,
        })
        .await
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

/// How long a network in detail goes unsaid while nothing of it changes.
const AGAIN: tokio::time::Duration = tokio::time::Duration::from_secs(5);

/// One caller asking for a network in detail as it changes, counted for as
/// long as it's kept.
struct Watching(watch::Sender<u32>);

impl Watching {
    fn new(watching: &watch::Sender<u32>) -> Watching {
        watching.send_modify(|watching| *watching += 1);
        Watching(watching.clone())
    }
}

impl Drop for Watching {
    fn drop(&mut self) {
        self.0
            .send_modify(|watching| *watching = watching.saturating_sub(1));
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
    use std::time::Duration;

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
            watching: watch::Sender::new(0),
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
    async fn describing_with_more_says_nothing_of_a_change_it_doesn_t_show() {
        let (wifi, state, _) = wifi();
        state.send_modify(|state| state.follow(&inn()));

        let mut replies = wifi.describe(true).await;
        let first = replies.next().await.unwrap();
        // A card beginning to join changes wifid's state, not what it shows.
        state.send_modify(|state| state.attempting("wlan2"));
        let unshown = replies.next().now_or_never();

        assert_eq!(first.parameters().unwrap().cards[0].name, "wlan2");
        assert!(unshown.is_none(), "answered again with nothing to show");
    }

    #[tokio::test]
    async fn settling_answers_once_a_card_reaches_beyond_its_network() {
        let (wifi, state, _) = wifi();
        state.send_modify(|state| state.follow(&inn()));

        let mut settling = wifi.settled(false).await;
        let before = settling.next().now_or_never();
        state.send_modify(|state| state.checking(2));
        let after = tokio::time::timeout(Duration::from_secs(3), settling.next()).await;

        assert!(before.is_none(), "settled before the card reached beyond");
        let reply = after
            .expect("never settled")
            .expect("ended without answering");
        assert_eq!(reply.continues(), Some(false));
        assert!(settling.next().await.is_none());
    }

    #[tokio::test]
    async fn settling_answers_at_once_when_already_settled() {
        let (wifi, state, _) = wifi();
        state.send_modify(|state| state.wired(true));

        let settled = wifi.settled(false).await.next().now_or_never();

        assert!(matches!(settled, Some(Some(_))), "{settled:?}");
    }

    fn inn_heard() -> crate::air::Heard {
        crate::air::Heard {
            name: "Roadside Inn".into(),
            security: Security::Open,
            signal: -52,
            bssid: "02:00:00:00:01:00".into(),
            frequency: 2462,
            age: 0,
            radio: crate::radio::Radio::default(),
        }
    }

    #[tokio::test]
    async fn a_network_in_detail_is_answered_once_as_it_is() {
        let (wifi, state, _) = wifi();
        state.send_modify(|state| state.heard("/BSSs/0", Some(inn_heard())));

        let mut replies = wifi.detail(false, "Roadside Inn".into()).await;
        let reply = replies.next().await.unwrap();

        assert_eq!(reply.continues(), Some(false));
        assert_eq!(
            reply.parameters().unwrap().points[0].bssid,
            "02:00:00:00:01:00"
        );
        assert!(replies.next().await.is_none());
    }

    #[tokio::test]
    async fn a_network_in_detail_with_more_is_answered_again_as_it_changes() {
        let (wifi, state, _) = wifi();

        let mut replies = wifi.detail(true, "Roadside Inn".into()).await;
        let first = replies.next().await.unwrap();
        state.send_modify(|state| state.heard("/BSSs/0", Some(inn_heard())));
        let second = replies.next().await.unwrap();
        state.send_modify(|state| state.follow(&inn()));
        let unchanged = replies.next().now_or_never();

        assert_eq!(first.parameters().unwrap().points, []);
        assert_eq!(second.continues(), Some(true));
        assert_eq!(second.parameters().unwrap().points.len(), 1);
        assert!(
            unchanged.is_none(),
            "answered again with its detail the same"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_network_in_detail_with_more_is_answered_again_now_and_then_unchanged() {
        let (wifi, _state, _) = wifi();
        let mut replies = wifi.detail(true, "Roadside Inn".into()).await;
        replies.next().await.unwrap();

        let again = tokio::time::timeout(Duration::from_secs(6), replies.next()).await;

        let again = again.expect("not answered again unchanged").unwrap();
        assert_eq!(again.continues(), Some(true));
    }

    #[tokio::test]
    async fn asking_for_detail_as_it_changes_has_the_cards_read_their_links_until_done() {
        let (wifi, _state, _) = wifi();
        let watching = wifi.watching.subscribe();

        let once = wifi.detail(false, "Roadside Inn".into()).await;
        let unwatched = *watching.borrow();
        let replies = wifi.detail(true, "Roadside Inn".into()).await;
        let watched = *watching.borrow();
        drop((once, replies));

        assert_eq!((unwatched, watched, *watching.borrow()), (0, 1, 0));
    }

    #[tokio::test]
    async fn settling_answers_once_wifid_stops() {
        let (wifi, state, _) = wifi();

        drop(state);
        let mut settling = wifi.settled(false).await;
        let settled = tokio::time::timeout(Duration::from_secs(3), settling.next()).await;

        assert!(
            matches!(settled, Ok(Some(_))),
            "still waiting on a wifid that stopped"
        );
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
                ..
            }) = taken.recv().await
            else {
                panic!("wasn't asked to join");
            };
            for step in said {
                progress.send(step).await.unwrap();
            }
            (name, password)
        });

        let replies = wifi
            .join(more, "Lantern".into(), Some("pass".into()), None)
            .await;
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
        let hall = Saving {
            name: "Hidden Hall".into(),
            security: Security::Personal,
            password: Some("under the stairs".into()),
            hidden: Some(true),
            priority: None,
            address: Some(Address::Card),
            account: None,
        };
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Save { saving, done }) = taken.recv().await else {
                panic!("wasn't asked to save");
            };
            done.send(Ok(())).unwrap();
            saving
        });

        let answer = wifi.save(hall.clone()).await;
        drop(wifi);

        assert_eq!(answer, Ok(()));
        assert_eq!(main_loop.await.unwrap(), hall);
    }

    #[tokio::test]
    async fn joining_with_an_account_hands_it_to_wifid() {
        let (wifi, _, _, mut taken) = asking();
        let account = Account {
            method: Method::Peap,
            identity: "joel".into(),
            anonymous_identity: None,
            server_domain: "radius.office.test".into(),
            inner: None,
        };
        let main_loop = tokio::spawn(async move {
            let Some(Ask::Join { account, .. }) = taken.recv().await else {
                panic!("wasn't asked to join");
            };
            account
        });

        let replies = wifi
            .join(
                false,
                "Office".into(),
                Some("hunter22".into()),
                Some(account.clone()),
            )
            .await;
        drop((wifi, replies));

        assert_eq!(main_loop.await.unwrap(), Some(account));
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
