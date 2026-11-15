//! What the launcher's Wi-Fi rows ask wifid to do, on the rows socket:
//! an `act` ask names the act and what it needs, wifid's main loop does it
//! as it does each varlink method, and the answer comes back on the ask's
//! id, a join's with each step as the card takes it, so the shell starts
//! nothing to act and holds one connection for its rows and their acts.
//!
//! ```text
//! {"id": 7, "ask": "act", "act": "leave", "network": "Lantern"}
//! {"id": 7, "done": true}
//! {"id": 9, "ask": "act", "act": "roam", "bssid": "6c:ae:f6:c1:84:26"}
//! {"id": 9, "done": true}
//! {"id": 8, "ask": "act", "act": "join", "network": "Lantern", "password": "..."}
//! {"id": 8, "stage": "joining", "said": "connecting to Lantern\u{2026}", "done": false}
//! {"id": 8, "stage": "online", "said": "connected to Lantern", "done": true}
//! ```
//!
//! A roam readies the access point first, looking over its channel so the
//! move needn't, and one wifid no longer hears looks for it once before
//! refusing it as `NotNearby`.
//!
//! What wifid won't do is an error coded `refused`, its `refused` the
//! varlink interface's name for why, as `WrongPassword`. Each stage and
//! refusal is `said` in the launcher's words for it (launcher/words/wifi.json),
//! as the vocabulary the connection's rows are made in has them, so the
//! shell's foot says them as they are (2026-11-13).

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot, watch};

use crate::face::error;
use crate::service::{Account, Ask, Refused, Stage, stopping};
use crate::state::State;
use crate::words::Words;

/// How far ahead of the shell reading them a join's steps may get.
const STEPS: usize = 8;

/// An act's ask of the main loop, and where its answer comes.
enum Asking {
    Once(Ask, oneshot::Receiver<Result<(), Refused>>),
    Steps(Ask, mpsc::Receiver<Result<Stage, Refused>>),
    /// A move to the access point at that address, which may look first.
    Roam(String),
}

/// What an `act` ask asks the main loop, or its error's code and why it
/// isn't one.
fn asking(said: &Value) -> Result<Asking, (&'static str, String)> {
    let network = || {
        said.get("network")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(("bad-ask", "say the network by its name".to_owned()))
    };
    let flag = |name: &str| {
        said.get(name)
            .and_then(Value::as_bool)
            .ok_or(("bad-ask", format!("say {name} as true or false")))
    };
    let act = said.get("act").and_then(Value::as_str).unwrap_or("");
    if act == "roam" {
        return Ok(Asking::Roam(address(said)?));
    }
    if act == "join" {
        let account = match said.get("account") {
            None | Some(Value::Null) => None,
            Some(account) => Some(
                serde_json::from_value::<Account>(account.clone())
                    .map_err(|why| ("bad-ask", format!("that account isn't one: {why}")))?,
            ),
        };
        let password = said
            .get("password")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let (progress, steps) = mpsc::channel(STEPS);
        let join = Ask::Join {
            name: network()?,
            password,
            account,
            progress,
        };
        return Ok(Asking::Steps(join, steps));
    }
    let (done, answer) = oneshot::channel();
    let ask = match act {
        "radio" => Ask::Radio {
            on: flag("on")?,
            done,
        },
        "scan" => Ask::Scan { done },
        "leave" => Ask::Leave {
            name: network()?,
            done,
        },
        "forget" => Ask::Forget {
            name: network()?,
            done,
        },
        "trust" => Ask::Trust {
            name: network()?,
            trusted: flag("trusted")?,
            done,
        },
        other => return Err(("unknown-act", format!("no act {other:?} here"))),
    };
    Ok(Asking::Once(ask, answer))
}

/// The access point a `roam` ask names, by its address, in lower case as
/// wpa_supplicant writes it.
fn address(said: &Value) -> Result<String, (&'static str, String)> {
    let bssid = said.get("bssid").and_then(Value::as_str).unwrap_or("");
    let parts: Vec<&str> = bssid.split(':').collect();
    let hex = |part: &&str| part.len() == 2 && part.chars().all(|digit| digit.is_ascii_hexdigit());
    if parts.len() == 6 && parts.iter().all(hex) {
        Ok(bssid.to_ascii_lowercase())
    } else {
        let why = "say the access point by its address, as 6c:ae:f6:c1:84:26";
        Err(("bad-ask", why.to_owned()))
    }
}

/// Does what one `act` ask says through wifid's main loop, telling the
/// connection each answer, until it's done or refused, worded in `words`
/// where there are any.
pub async fn act(
    asks: mpsc::Sender<Ask>,
    air: watch::Receiver<State>,
    id: Value,
    said: Value,
    told: mpsc::Sender<String>,
    words: Option<Words>,
) {
    let tell = |answer: Value| told.try_send(answer.to_string()).is_ok();
    let network = said.get("network").and_then(Value::as_str).unwrap_or("");
    let refused = |why: &Refused| refusal(&id, &said, why, words.as_ref());
    let step = |stage: Stage, last: bool| {
        let mut step = json!({"id": id, "stage": stage, "done": last});
        let staged = step["stage"].as_str().and_then(|stage| {
            words
                .as_ref()
                .and_then(|words| words.staged(stage, network))
        });
        if let Some(staged) = staged {
            step["said"] = json!(staged);
        }
        step
    };
    match asking(&said) {
        Err((code, why)) => {
            tell(error(&id, code, &why));
        }
        Ok(Asking::Once(ask, answer)) => {
            let answered = match asks.send(ask).await {
                Ok(()) => answer.await.unwrap_or_else(|_| Err(stopping())),
                Err(_) => Err(stopping()),
            };
            tell(match answered {
                Ok(()) => json!({"id": id, "done": true}),
                Err(why) => refused(&why),
            });
        }
        Ok(Asking::Roam(bssid)) => {
            tell(match roam(&asks, air, &bssid).await {
                Ok(()) => json!({"id": id, "done": true}),
                Err(why) => refused(&why),
            });
        }
        Ok(Asking::Steps(ask, mut steps)) => {
            if asks.send(ask).await.is_err() {
                tell(refused(&stopping()));
                return;
            }
            loop {
                match steps.recv().await {
                    Some(Ok(stage)) => {
                        let last = stage.is_final();
                        if !tell(step(stage, last)) || last {
                            return;
                        }
                    }
                    Some(Err(why)) => {
                        tell(refused(&why));
                        return;
                    }
                    None => {
                        tell(refused(&stopping()));
                        return;
                    }
                }
            }
        }
    }
}

/// How long a roam waits for the look it asked for: one over every
/// channel took 5 to 6 s (2026-11-14).
const LOOKING: Duration = Duration::from_secs(15);

/// How long a move waits for the look over its access point's channel
/// alone, which the kernel took 145 ms over as a move began (2026-11-15).
/// A look that doesn't end by then holds up no move.
const READYING: Duration = Duration::from_secs(2);

/// How long a move waits once its look has ended, for what the look held
/// up to reach the card before it leaves.
const CATCHING_UP: Duration = Duration::from_millis(200);

/// Moves the card to the access point at `bssid`, looking for it first
/// where it isn't heard. wpa_supplicant forgets an access point 180 s
/// after it last heard it, and with nothing looking more than hourly most
/// of the air is forgotten by the time a roam's asked: Main Floor's own
/// radio 5 m away was refused (2026-11-14). The look is waited for here,
/// apart from the main loop, which hears it end. The chooser moves the
/// card this way too (moves.rs).
///
/// One that's heard is readied first, its channel alone looked over for
/// the kernel, which forgets a point sooner, and moved to once that look
/// ends or `READYING` passes.
pub(crate) async fn roam(
    asks: &mpsc::Sender<Ask>,
    mut air: watch::Receiver<State>,
    bssid: &str,
) -> Result<(), Refused> {
    let readying = |done| Ask::Ready {
        bssid: bssid.to_owned(),
        done,
    };
    let roaming = |done| Ask::Roam {
        bssid: bssid.to_owned(),
        done,
    };
    let before = air.borrow_and_update().looks();
    match once(asks, readying).await {
        Ok(()) => {
            if looked(&mut air, before, READYING).await == Ok(false) {
                return Err(stopping());
            }
            return once(asks, roaming).await;
        }
        Err(Refused::NotNearby) => {}
        Err(refused) => return Err(refused),
    }
    let before = air.borrow_and_update().looks();
    once(asks, |done| Ask::Scan { done }).await?;
    match looked(&mut air, before, LOOKING).await {
        Ok(true) => once(asks, roaming).await,
        Ok(false) => Err(stopping()),
        Err(_) => Err(Refused::NotNearby),
    }
}

/// Waits as long as `within` for a look to end after `before` had, saying
/// whether one did, or `false` where wifid is stopping.
async fn looked(
    air: &mut watch::Receiver<State>,
    before: u32,
    within: Duration,
) -> Result<bool, tokio::time::error::Elapsed> {
    // Only whether it ended is kept, since the state it's read from is
    // held locked for as long as it's borrowed.
    let looked = air.wait_for(|now| now.looks() != before);
    tokio::time::timeout(within, looked)
        .await
        .map(|ended| ended.is_ok())
}

/// Has every card look, as the scan act does; the chooser asks this of a
/// card hearing no other access point of its network (moves.rs).
pub(crate) async fn look(asks: &mpsc::Sender<Ask>) -> Result<(), Refused> {
    once(asks, |done| Ask::Scan { done }).await
}

/// Asks the main loop one thing and waits for its answer.
async fn once(
    asks: &mpsc::Sender<Ask>,
    ask: impl FnOnce(oneshot::Sender<Result<(), Refused>>) -> Ask,
) -> Result<(), Refused> {
    let (done, answer) = oneshot::channel();
    match asks.send(ask(done)).await {
        Ok(()) => answer.await.unwrap_or_else(|_| Err(stopping())),
        Err(_) => Err(stopping()),
    }
}

/// Why wifid won't, as an error naming it as the varlink interface does,
/// and saying it in the launcher's words where they have it. A password
/// that didn't work is said by what the ask gave: an account's, one Joel
/// typed, or none, when the one saved has changed, as the launcher said it
/// before wifid served its acts (2026-11-13).
fn refusal(id: &Value, asked: &Value, refused: &Refused, words: Option<&Words>) -> Value {
    let (why, message) = match refused {
        Refused::Failed { reason } => ("Failed".to_owned(), reason.clone()),
        other => {
            let why = format!("{other:?}");
            (why.clone(), why)
        }
    };
    let worded_as = match refused {
        Refused::WrongPassword if asked.get("account").is_some_and(|one| !one.is_null()) => {
            "AccountPassword"
        }
        Refused::WrongPassword if asked.get("password").is_none() => "PasswordChanged",
        _ => why.as_str(),
    };
    let mut said = error(id, "refused", &message);
    if let Some(worded) = words.and_then(|words| words.refused(worded_as)) {
        said["error"]["said"] = json!(worded);
    }
    said["error"]["refused"] = json!(why);
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Acts on one ask, the main loop answering each with `answer`, and
    /// says what the connection was told.
    async fn acted(said: Value, answer: impl FnOnce(Ask) + Send + 'static) -> Vec<Value> {
        acted_in(None, said, answer).await
    }

    /// The same, its answers worded in the launcher's vocabulary.
    async fn acted_in(
        words: Option<Words>,
        said: Value,
        answer: impl FnOnce(Ask) + Send + 'static,
    ) -> Vec<Value> {
        let (asks, mut taken) = mpsc::channel(1);
        let (told, mut telling) = mpsc::channel(16);
        let (_state, watching) = watch::channel(State::default());
        tokio::spawn(async move {
            if let Some(ask) = taken.recv().await {
                answer(ask);
            }
        });
        act(asks, watching, json!(7), said, told, words).await;
        let mut heard = Vec::new();
        while let Ok(line) = telling.try_recv() {
            heard.push(serde_json::from_str(&line).expect("JSON"));
        }
        heard
    }

    #[tokio::test]
    async fn leaving_asks_the_main_loop_and_says_it_s_done() {
        let heard = acted(
            json!({"ask": "act", "act": "leave", "network": "Lantern"}),
            |ask| {
                let Ask::Leave { name, done } = ask else {
                    panic!("asked {ask:?}, not to leave");
                };
                assert_eq!(name, "Lantern");
                done.send(Ok(())).expect("answered");
            },
        )
        .await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
    }

    #[tokio::test]
    async fn turning_the_radio_on_and_trusting_say_which_way() {
        let heard = acted(json!({"act": "radio", "on": true}), |ask| {
            let Ask::Radio { on, done } = ask else {
                panic!("asked {ask:?}, not the radio");
            };
            assert!(on, "asked to turn the radio off, not on");
            done.send(Ok(())).expect("answered");
        })
        .await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
        let heard = acted(
            json!({"act": "trust", "network": "Lantern", "trusted": false}),
            |ask| {
                let Ask::Trust {
                    name,
                    trusted,
                    done,
                } = ask
                else {
                    panic!("asked {ask:?}, not to trust");
                };
                assert_eq!((name.as_str(), trusted), ("Lantern", false));
                done.send(Ok(())).expect("answered");
            },
        )
        .await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
    }

    /// Roams to an access point the main loop hears only once it's looked
    /// everywhere `near_after` times, if ever, each look it's asked for
    /// ending where `ending`, and says what the connection was told, what
    /// the main loop was asked, in turn, and how long after the readying
    /// look ended the move was asked.
    async fn roamed(
        near_after: Option<u32>,
        ending: bool,
    ) -> (Vec<Value>, Vec<&'static str>, Option<Duration>) {
        let (asks, mut taken) = mpsc::channel(1);
        let (told, mut telling) = mpsc::channel(16);
        let (state, watching) = watch::channel(State::default());
        let main_loop = tokio::spawn(async move {
            let mut asked = Vec::new();
            let (mut ended, mut waited) = (None, None);
            let mut looks = 0;
            let near = |looks| {
                if near_after.is_some_and(|after| looks >= after) {
                    Ok(())
                } else {
                    Err(Refused::NotNearby)
                }
            };
            let address = "6c:ae:f6:c1:84:26";
            while let Some(ask) = taken.recv().await {
                match ask {
                    Ask::Ready { bssid, done } => {
                        assert_eq!(bssid, address, "written as wpa_supplicant writes it");
                        asked.push("ready");
                        let answer = near(looks);
                        let heard = answer.is_ok();
                        done.send(answer).expect("answered");
                        // Its look over the one channel ends, as the main
                        // loop hears it.
                        if heard && ending {
                            state.send_modify(State::look_ended);
                            ended = Some(tokio::time::Instant::now());
                        }
                    }
                    Ask::Roam { bssid, done } => {
                        assert_eq!(bssid, address, "written as wpa_supplicant writes it");
                        asked.push("roam");
                        waited = ended.map(|ended| ended.elapsed());
                        done.send(near(looks)).expect("answered");
                    }
                    Ask::Scan { done } => {
                        looks += 1;
                        asked.push("scan");
                        done.send(Ok(())).expect("answered");
                        if ending {
                            state.send_modify(State::look_ended);
                        }
                    }
                    other => panic!("asked {other:?}, not to roam or look"),
                }
            }
            (asked, waited)
        });
        let asked = json!({"act": "roam", "bssid": "6C:AE:F6:C1:84:26"});
        act(asks, watching, json!(7), asked, told, None).await;
        let (asked, waited) = main_loop.await.expect("the main loop answered");
        let mut heard = Vec::new();
        while let Ok(line) = telling.try_recv() {
            heard.push(serde_json::from_str(&line).expect("JSON"));
        }
        (heard, asked, waited)
    }

    #[tokio::test]
    async fn roaming_readies_the_access_point_and_moves_once_its_channel_is_looked_over() {
        let (heard, asked, _) = roamed(Some(0), true).await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
        assert_eq!(asked, ["ready", "roam"], "readied, then moved");
    }

    #[tokio::test(start_paused = true)]
    async fn a_move_waits_for_what_its_look_held_up_to_reach_the_card() {
        let (_, asked, waited) = roamed(Some(0), true).await;
        assert_eq!(asked, ["ready", "roam"], "readied, then moved");
        assert!(
            waited.is_some_and(|waited| waited >= CATCHING_UP),
            "moved {waited:?} after the look ended, not {CATCHING_UP:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn roaming_moves_anyway_once_its_look_is_waited_for_long_enough() {
        let (heard, asked, _) = roamed(Some(0), false).await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
        assert_eq!(asked, ["ready", "roam"], "readied, then moved");
    }

    #[tokio::test]
    async fn roaming_to_an_access_point_forgotten_looks_for_it_first() {
        let (heard, asked, _) = roamed(Some(1), true).await;
        assert_eq!(heard, vec![json!({"id": 7, "done": true})]);
        assert_eq!(
            asked,
            ["ready", "scan", "roam"],
            "one look everywhere asked for before moving"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_access_point_unheard_after_one_look_is_refused() {
        for ending in [true, false] {
            let (heard, asked, _) = roamed(None, ending).await;
            assert_eq!(
                heard[0]["error"]["refused"], "NotNearby",
                "the look ending: {ending}"
            );
            let moved = if ending { &["roam"][..] } else { &[] };
            assert_eq!(
                asked,
                [&["ready", "scan"][..], moved].concat(),
                "one look asked for, the look ending: {ending}"
            );
        }
    }

    #[tokio::test]
    async fn a_refused_act_says_why_as_varlink_names_it() {
        let heard = acted(json!({"act": "forget", "network": "Lantern"}), |ask| {
            let Ask::Forget { done, .. } = ask else {
                panic!("asked {ask:?}, not to forget");
            };
            done.send(Err(Refused::Declared)).expect("answered");
        })
        .await;
        assert_eq!(heard[0]["error"]["code"], "refused");
        assert_eq!(heard[0]["error"]["refused"], "Declared");
        let heard = acted(json!({"act": "scan"}), |ask| {
            let Ask::Scan { done } = ask else {
                panic!("asked {ask:?}, not to scan");
            };
            let reason = "no card".to_owned();
            done.send(Err(Refused::Failed { reason }))
                .expect("answered");
        })
        .await;
        assert_eq!(
            (&heard[0]["error"]["refused"], &heard[0]["error"]["message"]),
            (&json!("Failed"), &json!("no card"))
        );
    }

    #[tokio::test]
    async fn a_join_says_each_step_until_the_last() {
        let heard = acted(
            json!({"act": "join", "network": "Lantern", "password": "lit"}),
            |ask| {
                let Ask::Join {
                    name,
                    password,
                    account,
                    progress,
                } = ask
                else {
                    panic!("asked {ask:?}, not to join");
                };
                assert_eq!(
                    (name.as_str(), password.as_deref(), account),
                    ("Lantern", Some("lit"), None)
                );
                for stage in [Stage::Joining, Stage::GettingAddress, Stage::Online] {
                    progress.try_send(Ok(stage)).expect("told");
                }
            },
        )
        .await;
        assert_eq!(
            heard,
            vec![
                json!({"id": 7, "stage": "joining", "done": false}),
                json!({"id": 7, "stage": "gettingAddress", "done": false}),
                json!({"id": 7, "stage": "online", "done": true}),
            ]
        );
    }

    #[tokio::test]
    async fn a_join_refused_partway_ends_with_why() {
        let heard = acted(json!({"act": "join", "network": "Lantern"}), |ask| {
            let Ask::Join { progress, .. } = ask else {
                panic!("asked {ask:?}, not to join");
            };
            progress.try_send(Ok(Stage::Joining)).expect("told");
            progress
                .try_send(Err(Refused::WrongPassword))
                .expect("told");
        })
        .await;
        assert_eq!(heard.len(), 2, "heard {heard:?}");
        assert_eq!(heard[1]["error"]["refused"], "WrongPassword");
    }

    fn words() -> Words {
        Words::read(&crate::words::launchers_words()).expect("Wi-Fi's words")
    }

    /// Joins Lantern, the main loop telling `told` back.
    async fn joined(asked: Value, told: Vec<Result<Stage, Refused>>) -> Vec<Value> {
        acted_in(Some(words()), asked, |ask| {
            let Ask::Join { progress, .. } = ask else {
                panic!("asked {ask:?}, not to join");
            };
            for one in told {
                progress.try_send(one).expect("told");
            }
        })
        .await
    }

    #[tokio::test]
    async fn a_join_says_each_step_in_the_launcher_s_words() {
        let lantern = json!({"act": "join", "network": "Lantern"});
        let heard = joined(lantern, vec![Ok(Stage::Joining), Ok(Stage::Online)]).await;
        let said: Vec<&Value> = heard.iter().map(|one| &one["said"]).collect();
        assert_eq!(
            said,
            [
                &json!("connecting to Lantern\u{2026}"),
                &json!("connected to Lantern")
            ]
        );
    }

    #[tokio::test]
    async fn a_refusal_says_why_in_the_launcher_s_words() {
        let lantern = json!({"act": "join", "network": "Lantern"});
        let heard = joined(
            lantern.clone(),
            vec![Err(Refused::Failed {
                reason: "no card".to_owned(),
            })],
        )
        .await;
        assert_eq!(heard[0]["error"]["said"], "it couldn't be connected");
        for (asked, said) in [
            (lantern, "its password has changed"),
            (
                json!({"act": "join", "network": "Lantern", "password": "typed"}),
                "that password didn't work",
            ),
            (
                json!({"act": "join", "network": "Campus", "password": "typed",
                    "account": {"method": "peap", "identity": "joel",
                        "serverDomain": "campus.example"}}),
                "that account's password didn't work",
            ),
        ] {
            let heard = joined(asked, vec![Err(Refused::WrongPassword)]).await;
            assert_eq!(heard[0]["error"]["said"], said, "heard {heard:?}");
            assert_eq!(
                heard[0]["error"]["refused"], "WrongPassword",
                "coded as varlink says"
            );
        }
    }

    #[tokio::test]
    async fn an_act_wifid_has_stopped_for_says_so() {
        let heard = acted(json!({"act": "leave", "network": "Lantern"}), drop).await;
        assert_eq!(heard[0]["error"]["message"], "wifid is stopping");
    }

    #[tokio::test]
    async fn an_act_it_doesn_t_know_or_short_of_what_it_needs_is_refused() {
        let heard = acted(json!({"act": "dance"}), drop).await;
        assert_eq!(heard[0]["error"]["code"], "unknown-act");
        let heard = acted(json!({"act": "leave"}), drop).await;
        assert_eq!(heard[0]["error"]["code"], "bad-ask");
        let heard = acted(json!({"act": "radio"}), drop).await;
        assert_eq!(heard[0]["error"]["code"], "bad-ask");
        for bssid in [json!(null), json!("next door"), json!("6c:ae:f6:c1:84")] {
            let heard = acted(json!({"act": "roam", "bssid": bssid}), drop).await;
            assert_eq!(heard[0]["error"]["code"], "bad-ask", "roaming to {bssid}");
        }
    }
}
