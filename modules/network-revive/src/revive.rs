//! What each unit gets once the network has changed, or once the daemon
//! starts. A unit that failed, or waits to restart, starts again; one that
//! runs reloads, if it's meant to; one stopped on purpose, or one systemd
//! doesn't know, is left. A unit's check has to pass first, so nothing
//! starts only to fail. The network often answers names a moment after
//! networkd calls it routable, so a refused check is tried again for half a
//! minute before the unit is left down. systemd leaves the bus for a moment
//! whenever it executes itself again, as a switch to a new configuration has
//! it do, so what it didn't answer then is asked again once it's back.

use std::future::Future;

use tokio::time::{Duration, sleep};

use crate::config::Unit;

/// Why the units are looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The daemon started, with the account or again: anything already
    /// down is started, but nothing running has missed a change.
    Started,
    /// What the machine can reach changed.
    Changed,
}

/// A unit as systemd sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub load: String,
    pub active: String,
    pub sub: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Clear what systemd counts against it, then start it.
    Start,
    Reload,
}

/// What a unit got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing: it runs, it was stopped on purpose, or systemd queued its
    /// restart already.
    Left,
    /// Started, from the state it was in.
    Started(State),
    Reloaded,
    /// It runs, but its check failed, so reloading would only fail too.
    Kept {
        why: String,
    },
    /// Down, and its check never passed.
    Refused {
        why: String,
    },
    /// systemd wouldn't do what was asked.
    Failed {
        action: Action,
        error: String,
    },
}

/// Why something asked of systemd came back without what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unanswered {
    /// systemd left the bus before answering, or isn't on it yet.
    Away(String),
    /// systemd refused, or the check failed.
    Said(String),
}

/// The account's units, as the daemon reaches them.
pub trait Units {
    async fn state(&self, name: &str) -> Result<State, Unanswered>;
    /// Runs the unit's program with the check's arguments added.
    async fn check(&self, name: &str, arguments: &[String]) -> Result<(), Unanswered>;
    /// Clears what systemd counts against the unit and starts it, both or
    /// neither, even if what asked is dropped in between.
    async fn start(&self, name: &str) -> Result<(), Unanswered>;
    async fn reload(&self, name: &str) -> Result<(), Unanswered>;
}

/// How long systemd may be away before what it wasn't there to answer is
/// given up on. An account's systemd is back on the bus well within half a
/// second of executing itself again.
pub const AWAY: Duration = Duration::from_secs(10);

/// How often systemd is asked again while it's away.
const AWAY_AGAIN: Duration = Duration::from_millis(250);

/// Asks systemd, and asks again while it's away, until it answers or has
/// been away too long.
async fn asked<T, Answer: Future<Output = Result<T, Unanswered>>>(
    mut ask: impl FnMut() -> Answer,
) -> Result<T, String> {
    let mut away = Duration::ZERO;
    loop {
        match ask().await {
            Ok(answer) => return Ok(answer),
            Err(Unanswered::Away(_)) if away < AWAY => {
                sleep(AWAY_AGAIN).await;
                away += AWAY_AGAIN;
            }
            Err(Unanswered::Away(error) | Unanswered::Said(error)) => return Err(error),
        }
    }
}

/// How long to wait before each further look at a unit whose check failed.
pub const AGAIN: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
];

pub fn decide(state: &State, why: Why, reload: bool) -> Option<Action> {
    if state.load != "loaded" {
        return None;
    }
    match (state.active.as_str(), state.sub.as_str()) {
        // A moment from a restart systemd queued already.
        ("failed", "failed-before-auto-restart") => None,
        ("failed", _) | ("activating", "auto-restart") => Some(Action::Start),
        ("active", _) if reload && why == Why::Changed => Some(Action::Reload),
        _ => None,
    }
}

pub async fn revive(units: &impl Units, unit: &Unit, why: Why) -> Outcome {
    let mut refused = None;
    for wait in [Duration::ZERO].into_iter().chain(AGAIN) {
        sleep(wait).await;
        // Looked at afresh each time, since anything may have started or
        // stopped it meanwhile.
        let state = match asked(|| units.state(&unit.name)).await {
            Ok(state) => state,
            Err(error) => {
                return Outcome::Failed {
                    action: Action::Start,
                    error,
                };
            }
        };
        let action = decide(&state, why, unit.reload);
        // Started by something else meanwhile, it has the network as it
        // is now, and needs no reloading.
        let Some(action) = action.filter(|action| refused.is_none() || *action == Action::Start)
        else {
            return Outcome::Left;
        };
        if let Some(arguments) = &unit.check
            && let Err(why) = asked(|| units.check(&unit.name, arguments)).await
        {
            if action == Action::Reload {
                return Outcome::Kept { why };
            }
            refused = Some(why);
            continue;
        }
        return match action {
            Action::Start => match asked(|| units.start(&unit.name)).await {
                Ok(()) => Outcome::Started(state),
                Err(error) => Outcome::Failed { action, error },
            },
            Action::Reload => match asked(|| units.reload(&unit.name)).await {
                Ok(()) => Outcome::Reloaded,
                Err(error) => Outcome::Failed { action, error },
            },
        };
    }
    Outcome::Refused {
        why: refused.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use tokio::time::Instant;

    use super::*;

    fn state(active: &str, sub: &str) -> State {
        State {
            load: "loaded".into(),
            active: active.into(),
            sub: sub.into(),
        }
    }

    fn unit(check: bool, reload: bool) -> Unit {
        Unit {
            name: "proxy.service".into(),
            check: check.then(|| vec!["-t".into()]),
            reload,
        }
    }

    /// Units that say what they're told to, and keep what was asked of them
    /// and when.
    struct Told {
        states: RefCell<VecDeque<State>>,
        checks: RefCell<VecDeque<Result<(), String>>>,
        answers: RefCell<VecDeque<Result<(), Unanswered>>>,
        began: Instant,
        asked: RefCell<Vec<(Duration, String)>>,
    }

    impl Told {
        fn new(states: &[State], checks: &[Result<(), &str>]) -> Told {
            Told {
                states: RefCell::new(states.iter().cloned().collect()),
                checks: RefCell::new(
                    checks
                        .iter()
                        .map(|check| check.map_err(str::to_string))
                        .collect(),
                ),
                answers: RefCell::new(VecDeque::new()),
                began: Instant::now(),
                asked: RefCell::new(Vec::new()),
            }
        }

        fn asked(&self, what: &str) {
            self.asked
                .borrow_mut()
                .push((self.began.elapsed(), what.to_string()));
        }

        /// Has starts and reloads answered with these in turn, the last for
        /// every one after it.
        fn answering(self, answers: &[Result<(), Unanswered>]) -> Told {
            *self.answers.borrow_mut() = answers.iter().cloned().collect();
            self
        }

        fn answer(&self) -> Result<(), Unanswered> {
            let mut answers = self.answers.borrow_mut();
            let answer = if answers.len() > 1 {
                answers.pop_front()
            } else {
                answers.front().cloned()
            };
            answer.unwrap_or(Ok(()))
        }

        fn acted_at(&self) -> Vec<Duration> {
            let asked = self.asked.borrow();
            asked
                .iter()
                .filter(|(_, what)| ["start", "reload"].contains(&what.as_str()))
                .map(|(at, _)| *at)
                .collect()
        }

        fn actions(&self) -> Vec<String> {
            let asked = self.asked.borrow();
            let actions = asked
                .iter()
                .filter(|(_, what)| ["start", "reload"].contains(&what.as_str()));
            actions.map(|(_, what)| what.clone()).collect()
        }

        fn checked_at(&self) -> Vec<Duration> {
            let asked = self.asked.borrow();
            asked
                .iter()
                .filter(|(_, what)| what == "check")
                .map(|(at, _)| *at)
                .collect()
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the stand-in answers at once, which an async trait allows"
    )]
    impl Units for Told {
        async fn state(&self, _: &str) -> Result<State, Unanswered> {
            let mut states = self.states.borrow_mut();
            let state = if states.len() > 1 {
                states.pop_front()
            } else {
                states.front().cloned()
            };
            state.ok_or_else(|| Unanswered::Said("no state".to_string()))
        }

        async fn check(&self, _: &str, arguments: &[String]) -> Result<(), Unanswered> {
            assert_eq!(arguments, ["-t"]);
            self.asked("check");
            let mut checks = self.checks.borrow_mut();
            let check = if checks.len() > 1 {
                checks.pop_front()
            } else {
                checks.front().cloned()
            };
            check.unwrap_or(Ok(())).map_err(Unanswered::Said)
        }

        async fn start(&self, _: &str) -> Result<(), Unanswered> {
            self.asked("start");
            self.answer()
        }

        async fn reload(&self, _: &str) -> Result<(), Unanswered> {
            self.asked("reload");
            self.answer()
        }
    }

    fn away() -> Result<(), Unanswered> {
        Err(Unanswered::Away("Remote peer disconnected".into()))
    }

    #[test]
    fn a_failed_unit_starts_again() {
        let failed = state("failed", "failed");

        assert_eq!(decide(&failed, Why::Changed, false), Some(Action::Start));
        assert_eq!(decide(&failed, Why::Started, false), Some(Action::Start));
    }

    #[test]
    fn a_unit_waiting_to_restart_starts_now() {
        let waiting = state("activating", "auto-restart");

        assert_eq!(decide(&waiting, Why::Changed, false), Some(Action::Start));
    }

    #[test]
    fn a_unit_a_moment_from_its_restart_is_left_to_it() {
        let queued = state("activating", "auto-restart-queued");
        let failing = state("failed", "failed-before-auto-restart");

        assert_eq!(decide(&queued, Why::Changed, false), None);
        assert_eq!(decide(&failing, Why::Changed, false), None);
    }

    #[test]
    fn a_unit_stopped_on_purpose_stays_stopped() {
        assert_eq!(decide(&state("inactive", "dead"), Why::Changed, true), None);
    }

    #[test]
    fn a_unit_already_starting_is_left_to_start() {
        assert_eq!(
            decide(&state("activating", "start"), Why::Changed, true),
            None
        );
    }

    #[test]
    fn a_unit_systemd_doesnt_know_is_left() {
        let missing = State {
            load: "not-found".into(),
            ..state("failed", "failed")
        };

        assert_eq!(decide(&missing, Why::Changed, false), None);
    }

    #[test]
    fn a_running_unit_reloads_only_when_meant_to_and_the_network_changed() {
        let running = state("active", "running");

        assert_eq!(decide(&running, Why::Changed, true), Some(Action::Reload));
        assert_eq!(decide(&running, Why::Changed, false), None);
        assert_eq!(decide(&running, Why::Started, true), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_unit_whose_check_passes_is_started_at_once() {
        let units = Told::new(&[state("failed", "failed")], &[Ok(())]);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(outcome, Outcome::Started(state("failed", "failed")));
        assert_eq!(units.actions(), ["start"]);
        assert_eq!(units.checked_at(), [Duration::ZERO]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_refused_check_is_tried_again_until_it_passes() {
        let checks = [Err("host not found"), Err("host not found"), Ok(())];
        let units = Told::new(&[state("failed", "failed")], &checks);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(outcome, Outcome::Started(state("failed", "failed")));
        assert_eq!(units.actions(), ["start"]);
        let seconds = |seconds| Duration::from_secs(seconds);
        assert_eq!(units.checked_at(), [seconds(0), seconds(1), seconds(3)]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_check_that_never_passes_leaves_the_unit_down_and_says_why() {
        let units = Told::new(&[state("failed", "failed")], &[Err("host not found")]);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(
            outcome,
            Outcome::Refused {
                why: "host not found".into()
            }
        );
        assert!(units.actions().is_empty());
        assert_eq!(units.checked_at().len(), 1 + AGAIN.len());
        assert_eq!(units.checked_at().last(), Some(&Duration::from_secs(31)));
    }

    #[tokio::test(start_paused = true)]
    async fn a_unit_started_by_something_else_meanwhile_is_left() {
        let states = [state("failed", "failed"), state("active", "running")];
        let units = Told::new(&states, &[Err("host not found")]);

        let outcome = revive(&units, &unit(true, true), Why::Changed).await;

        assert_eq!(outcome, Outcome::Left);
        assert!(units.actions().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_running_unit_whose_check_fails_is_kept_as_it_is_without_trying_again() {
        let units = Told::new(&[state("active", "running")], &[Err("host not found")]);

        let outcome = revive(&units, &unit(true, true), Why::Changed).await;

        assert_eq!(
            outcome,
            Outcome::Kept {
                why: "host not found".into()
            }
        );
        assert!(units.actions().is_empty());
        assert_eq!(units.checked_at(), [Duration::ZERO]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_running_unit_whose_check_passes_reloads() {
        let units = Told::new(&[state("active", "running")], &[Ok(())]);

        let outcome = revive(&units, &unit(true, true), Why::Changed).await;

        assert_eq!(outcome, Outcome::Reloaded);
        assert_eq!(units.actions(), ["reload"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_start_systemd_was_away_for_is_asked_again_once_it_is_back() {
        let units = Told::new(&[state("failed", "failed")], &[Ok(())]).answering(&[
            away(),
            away(),
            away(),
            Ok(()),
        ]);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(outcome, Outcome::Started(state("failed", "failed")));
        let millis = |millis| Duration::from_millis(millis);
        assert_eq!(
            units.acted_at(),
            [millis(0), millis(250), millis(500), millis(750)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_reload_systemd_was_away_for_is_asked_again_once_it_is_back() {
        let units =
            Told::new(&[state("active", "running")], &[Ok(())]).answering(&[away(), Ok(())]);

        let outcome = revive(&units, &unit(true, true), Why::Changed).await;

        assert_eq!(outcome, Outcome::Reloaded);
        assert_eq!(units.actions(), ["reload", "reload"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_start_systemd_stays_away_for_is_given_up_and_says_why() {
        let units = Told::new(&[state("failed", "failed")], &[Ok(())]).answering(&[away()]);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(
            outcome,
            Outcome::Failed {
                action: Action::Start,
                error: "Remote peer disconnected".into()
            }
        );
        assert_eq!(units.acted_at().last(), Some(&AWAY));
    }

    #[tokio::test(start_paused = true)]
    async fn a_start_systemd_refuses_is_not_asked_again() {
        let refused = Err(Unanswered::Said("Unit proxy.service is masked.".into()));
        let units = Told::new(&[state("failed", "failed")], &[Ok(())]).answering(&[refused]);

        let outcome = revive(&units, &unit(true, false), Why::Changed).await;

        assert_eq!(
            outcome,
            Outcome::Failed {
                action: Action::Start,
                error: "Unit proxy.service is masked.".into()
            }
        );
        assert_eq!(units.actions(), ["start"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_unit_with_no_check_is_started_unchecked() {
        let units = Told::new(&[state("failed", "failed")], &[Err("never asked")]);

        let outcome = revive(&units, &unit(false, false), Why::Changed).await;

        assert_eq!(outcome, Outcome::Started(state("failed", "failed")));
        assert!(units.checked_at().is_empty());
    }
}
