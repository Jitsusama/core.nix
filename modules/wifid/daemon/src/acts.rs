//! What the launcher's Wi-Fi rows ask wifid to do, on the rows socket:
//! an `act` ask names the act and what it needs, wifid's main loop does it
//! as it does each varlink method, and the answer comes back on the ask's
//! id, a join's with each step as the card takes it, so the shell starts
//! nothing to act and holds one connection for its rows and their acts.
//!
//! ```text
//! {"id": 7, "ask": "act", "act": "leave", "network": "Lantern"}
//! {"id": 7, "done": true}
//! {"id": 8, "ask": "act", "act": "join", "network": "Lantern", "password": "..."}
//! {"id": 8, "stage": "joining", "said": "connecting to Lantern\u{2026}", "done": false}
//! {"id": 8, "stage": "online", "said": "connected to Lantern", "done": true}
//! ```
//!
//! What wifid won't do is an error coded `refused`, its `refused` the
//! varlink interface's name for why, as `WrongPassword`. Each stage and
//! refusal is `said` in the launcher's words for it (launcher/words/wifi.json),
//! as the vocabulary the connection's rows are made in has them, so the
//! shell's foot says them as they are (2026-11-13).

use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::face::error;
use crate::service::{Account, Ask, Refused, Stage, stopping};
use crate::words::Words;

/// How far ahead of the shell reading them a join's steps may get.
const STEPS: usize = 8;

/// An act's ask of the main loop, and where its answer comes.
enum Asking {
    Once(Ask, oneshot::Receiver<Result<(), Refused>>),
    Steps(Ask, mpsc::Receiver<Result<Stage, Refused>>),
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

/// Does what one `act` ask says through wifid's main loop, telling the
/// connection each answer, until it's done or refused, worded in `words`
/// where there are any.
pub async fn act(
    asks: mpsc::Sender<Ask>,
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
        tokio::spawn(async move {
            if let Some(ask) = taken.recv().await {
                answer(ask);
            }
        });
        act(asks, json!(7), said, told, words).await;
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
    }
}
