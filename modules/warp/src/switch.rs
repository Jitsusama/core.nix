//! WARP turned on and off: `warp connect` and `warp disconnect`, as the
//! launcher's `:connect` and `:disconnect` run them, each asking warp-cli,
//! which asks WARP's service. What warp-cli needs no one's leave for, it
//! does as anyone can; what it decides here is only what it must say
//! first, from WARP's own settings:
//!
//! - the organisation holding the switch (`switch_locked`) holds it for
//!   both, so neither is asked and its name says who holds it;
//! - WARP already standing as asked is said, and warp-cli isn't asked;
//! - WARP always on (`always_on`) comes back by itself once disconnected,
//!   after `auto_connect_after_min` minutes, which is said with it.
//!
//! warp-cli says it did a switch by ending well, as `Success`; how the
//! link then goes, connecting and connected, is its status's to say.

use serde::Serialize;
use serde_json::Value;

use crate::ask::{Ask, Unasked};
use crate::read::{self, Failure};

/// Which way WARP is switched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Connect,
    Disconnect,
}

impl Act {
    /// The act's word, warp-cli's and the command line's.
    pub fn word(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Disconnect => "disconnect",
        }
    }

    /// warp-cli's state once it's done.
    fn state(self) -> &'static str {
        match self {
            Self::Connect => "Connected",
            Self::Disconnect => "Disconnected",
        }
    }
}

/// What a switch came to, as `--json` prints it.
#[derive(Debug, PartialEq, Serialize)]
pub struct Switched {
    /// `connect` or `disconnect`.
    pub act: &'static str,
    /// Whether WARP stands as asked, or warp-cli said it's on its way.
    pub done: bool,
    /// Why it isn't: `held`, the organisation holds the switch, or
    /// `refused`, warp-cli wouldn't, in `said`.
    pub refused: Option<&'static str>,
    /// Who holds the switch, where it's held.
    pub organisation: Option<String>,
    /// The minutes WARP comes back by itself after, once disconnected
    /// while always on, or 0 for at once.
    pub comes_back_after_min: Option<u64>,
    /// What happened, in a sentence.
    pub said: String,
}

/// Switches WARP as `act` asks, or says why not; a failure to ask at all
/// is read's, said as any reading's is.
pub fn switch(cli: &impl Ask, act: Act) -> Result<Switched, Failure> {
    let warp = read::read(cli)?;
    let settings = warp.settings.as_ref();
    let organisation = settings.and_then(|settings| settings.organisation.clone());
    let standing = warp.status.state.to_lowercase();
    let switched = |done, refused, comes_back_after_min, said: String| Switched {
        act: act.word(),
        done,
        refused,
        organisation: organisation.clone(),
        comes_back_after_min,
        said,
    };
    if settings.and_then(|settings| settings.switch_locked) == Some(true) {
        let holder = organisation.as_deref().unwrap_or("The organisation");
        return Ok(switched(
            false,
            Some("held"),
            None,
            format!("{holder} holds WARP's switch, so it's left {standing}"),
        ));
    }
    if warp.status.state == act.state() {
        return Ok(switched(
            true,
            None,
            None,
            format!("WARP is already {standing}"),
        ));
    }
    let said = cli.ask(&[act.word()]).map_err(|unasked| match unasked {
        Unasked::Missing(at) => Failure::NotInstalled(at),
        Unasked::Stuck(why) => Failure::NotAnswering(why),
    })?;
    if let Some(why) = refusal(said.ok, &said.text) {
        return Ok(switched(
            false,
            Some("refused"),
            None,
            format!("warp-cli wouldn't {}: {why}", act.word()),
        ));
    }
    let always_on = settings.and_then(|settings| settings.always_on) == Some(true);
    Ok(match act {
        Act::Connect => switched(true, None, None, "WARP is connecting".into()),
        Act::Disconnect if always_on => {
            let after = settings
                .and_then(|settings| settings.auto_connect_after_min)
                .unwrap_or(0);
            let when = if after == 0 {
                "at once".to_string()
            } else {
                format!("in {after} min")
            };
            switched(
                true,
                None,
                Some(after),
                format!("WARP is disconnected, and comes back by itself {when}"),
            )
        }
        Act::Disconnect => switched(true, None, None, "WARP is disconnected".into()),
    })
}

/// Why warp-cli wouldn't, from what it said, or none where it did: an
/// error as warp-cli says one, { code, error }, or a run that failed,
/// in whatever words it printed.
fn refusal(ok: bool, text: &str) -> Option<String> {
    let said: Option<Value> = serde_json::from_str(text).ok();
    let error = said
        .as_ref()
        .filter(|said| said["code"].is_string())
        .map(|said| said["error"].as_str().unwrap_or_default().to_string());
    match error {
        Some(why) => Some(why),
        None if ok => None,
        None => Some(text.trim().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ask::Said;
    use crate::contract::{Contract, file};

    /// A warp-cli that remembers what it was asked.
    struct Heard<A> {
        cli: A,
        asked: Mutex<Vec<String>>,
    }

    impl<A: Ask> Ask for Heard<A> {
        fn ask(&self, question: &[&str]) -> Result<Said, Unasked> {
            self.asked
                .lock()
                .expect("nothing panics holding it")
                .push(question.join(" "));
            self.cli.ask(question)
        }
    }

    impl<A> Heard<A> {
        fn asked(&self, question: &str) -> bool {
            self.asked
                .lock()
                .expect("nothing panics holding it")
                .iter()
                .any(|asked| asked == question)
        }
    }

    fn heard(cli: Contract) -> Heard<Contract> {
        Heard {
            cli,
            asked: Mutex::new(Vec::new()),
        }
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn done() -> Result<Said, Unasked> {
        Ok(Said {
            ok: true,
            text: "Success\n".into(),
        })
    }

    /// The contract's settings, some of them changed.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn settings(changes: &Value) -> Result<Said, Unasked> {
        let mut said: Value = serde_json::from_str(&file("settings.json")).expect("it's JSON");
        for (key, value) in changes.as_object().expect("changes are an object") {
            said["settings"][key] = value.clone();
        }
        Ok(Said {
            ok: true,
            text: said.to_string(),
        })
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn status(state: &str) -> Result<Said, Unasked> {
        Ok(Said {
            ok: true,
            text: serde_json::json!({ "status": state, "reason": "ManualDisconnection" })
                .to_string(),
        })
    }

    #[test]
    fn a_switch_the_organisation_holds_is_left_as_it_is_and_says_who_holds_it() {
        for act in [Act::Connect, Act::Disconnect] {
            let cli = heard(
                Contract::whole()
                    .with(
                        "settings",
                        settings(&serde_json::json!({ "switch_locked": true })),
                    )
                    .with("status", status("Connecting"))
                    .with(act.word(), done()),
            );
            let switched = switch(&cli, act).expect("it reads");
            assert_eq!(
                (
                    switched.done,
                    switched.refused,
                    switched.organisation.as_deref()
                ),
                (false, Some("held"), Some("example"))
            );
            assert_eq!(
                switched.said,
                "example holds WARP's switch, so it's left connecting"
            );
            assert!(!cli.asked(act.word()), "{} was asked", act.word());
        }
    }

    #[test]
    fn a_held_switch_with_no_organisation_named_says_the_organisation_holds_it() {
        let cli = heard(
            Contract::whole()
                .with(
                    "settings",
                    settings(&serde_json::json!({ "switch_locked": true, "organization": null })),
                )
                .with("disconnect", done()),
        );
        let switched = switch(&cli, Act::Disconnect).expect("it reads");
        assert_eq!(
            switched.said,
            "The organisation holds WARP's switch, so it's left connected"
        );
        assert!(!cli.asked("disconnect"));
    }

    #[test]
    fn disconnecting_while_always_on_says_when_it_comes_back() {
        let cli = heard(Contract::whole().with("disconnect", done()));
        let switched = switch(&cli, Act::Disconnect).expect("it reads");
        assert!(cli.asked("disconnect"));
        assert_eq!(
            switched,
            Switched {
                act: "disconnect",
                done: true,
                refused: None,
                organisation: Some("example".into()),
                comes_back_after_min: Some(2),
                said: "WARP is disconnected, and comes back by itself in 2 min".into(),
            }
        );
    }

    #[test]
    fn always_on_with_no_wait_comes_back_at_once() {
        let cli = heard(
            Contract::whole()
                .with(
                    "settings",
                    settings(&serde_json::json!({ "auto_connect_after_min": 0 })),
                )
                .with("disconnect", done()),
        );
        let switched = switch(&cli, Act::Disconnect).expect("it reads");
        assert_eq!(switched.comes_back_after_min, Some(0));
        assert_eq!(
            switched.said,
            "WARP is disconnected, and comes back by itself at once"
        );
    }

    #[test]
    fn disconnecting_a_warp_not_always_on_says_nothing_of_coming_back() {
        let cli = heard(
            Contract::whole()
                .with(
                    "settings",
                    settings(&serde_json::json!({ "always_on": false })),
                )
                .with("disconnect", done()),
        );
        let switched = switch(&cli, Act::Disconnect).expect("it reads");
        assert_eq!(
            (
                switched.done,
                switched.comes_back_after_min,
                switched.said.as_str()
            ),
            (true, None, "WARP is disconnected")
        );
    }

    #[test]
    fn connecting_asks_warp_cli_and_leaves_the_rest_to_its_status() {
        let cli = heard(
            Contract::whole()
                .with("status", status("Disconnected"))
                .with("connect", done()),
        );
        let switched = switch(&cli, Act::Connect).expect("it reads");
        assert!(cli.asked("connect"));
        assert_eq!(
            (switched.done, switched.refused, switched.said.as_str()),
            (true, None, "WARP is connecting")
        );
    }

    #[test]
    fn warp_already_standing_as_asked_is_said_and_warp_cli_isnt_asked() {
        let cli = heard(Contract::whole().with("connect", done()));
        let switched = switch(&cli, Act::Connect).expect("it reads");
        assert!(!cli.asked("connect"));
        assert_eq!(
            (switched.done, switched.said.as_str()),
            (true, "WARP is already connected")
        );
        let off = heard(
            Contract::whole()
                .with("status", status("Disconnected"))
                .with("disconnect", done()),
        );
        let switched = switch(&off, Act::Disconnect).expect("it reads");
        assert!(!off.asked("disconnect"));
        assert_eq!(switched.said, "WARP is already disconnected");
    }

    #[test]
    fn a_switch_warp_cli_refuses_is_said_in_its_words() {
        // The contract refuses a question it isn't given, as warp-cli
        // refuses one it won't answer.
        let cli = heard(Contract::whole().with("status", status("Disconnected")));
        let switched = switch(&cli, Act::Connect).expect("it reads");
        assert!(cli.asked("connect"));
        assert_eq!(
            (switched.done, switched.refused, switched.said.as_str()),
            (
                false,
                Some("refused"),
                "warp-cli wouldn't connect: Invalid account type."
            )
        );
    }

    #[test]
    fn a_warp_cli_that_fails_without_saying_why_is_refused_all_the_same() {
        let cli = heard(Contract::whole().with(
            "disconnect",
            Ok(Said {
                ok: false,
                text: "Error: it broke\n".into(),
            }),
        ));
        let switched = switch(&cli, Act::Disconnect).expect("it reads");
        assert_eq!(
            (switched.refused, switched.said.as_str()),
            (
                Some("refused"),
                "warp-cli wouldn't disconnect: Error: it broke"
            )
        );
    }

    #[test]
    fn no_warp_cli_or_a_service_not_answering_is_reads_failure() {
        let missing = Contract::none().with("status", Err(Unasked::Missing("warp-cli".into())));
        assert_eq!(
            switch(&missing, Act::Connect),
            Err(Failure::NotInstalled("warp-cli".into()))
        );
        let stuck = Contract::whole()
            .with("status", status("Disconnected"))
            .with("connect", Err(Unasked::Stuck("too slow".into())));
        assert_eq!(
            switch(&stuck, Act::Connect),
            Err(Failure::NotAnswering("too slow".into()))
        );
    }
}
