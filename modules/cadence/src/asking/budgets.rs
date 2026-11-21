//! What cadence remembers of a budget's alarms between ticks, so a level is
//! told to the orchestrator once while it stands, and no budget watched is
//! told only once it has truly stood.

use crate::watch::{Breach, Snapshot, breaches};

use super::State;

/// How long a level's alarm is remembered once it stops standing: one is
/// told again only once it has stood clear this long, so a share hovering
/// at a level isn't told at each crossing, while a rolling window that
/// eased overnight is told when it climbs back. It is remembered however long no
/// budget can be read, as through a sleep, so a level standing on either
/// side isn't told again; one that eased just before and climbed back
/// meanwhile isn't told either, as cadence can't tell the two apart.
const BUDGET_REST: i64 = 6 * 3600;
/// How long no budget may go watched before the orchestrator is told:
/// spend-watch times its passes by a clock that stops while the machine
/// sleeps, so after a wake it may take five minutes to write again.
const UNWATCHED_FOR: i64 = 10 * 60;
/// Sleep between ticks past this, read from the kernel's clocks, starts
/// again how long no budget was watched, as a boot does: a wake too brief
/// for spend-watch to write isn't counted into the next, and a nap is never
/// counted as watched time. A tick skipped or failed while awake slept
/// none, so it never restarts the count, and slow ticks only delay a stop,
/// however slow. A stretch awake that cadence didn't run, as logged out,
/// counts too: spend-watch runs beside it, so nothing was watched.
const SLEPT: i64 = 60;
/// Where the clocks can't be read, a gap between ticks past this starts the
/// count again instead, so one tick missed is absorbed, and a nap under it
/// is counted as watched time.
const MISSED: i64 = 15 * 60;

/// Whether an alarm's key says a budget passed a share of an allowance,
/// which only the orchestrator can act on, by moving work to another
/// budget or asking Joel, as it asked to be told on 20 November.
pub(super) fn budgeting(key: &str) -> bool {
    key.starts_with("alarm:budget:")
}

/// Whether a budget's alarm at `key`, last standing at `at`, is still
/// remembered. No budget watched is forgotten once spend-watch writes
/// again, as a stop doesn't hover the way a share does, so each stop is
/// told, however soon after the last.
pub(super) fn rests(s: &Snapshot, key: &str, at: i64) -> bool {
    let stop = key.strip_prefix("alarm:") == Some(crate::watch::budgets::UNWATCHED);
    s.budgets_unwatched.is_some() || (!stop && s.now - at < BUDGET_REST)
}

/// Remembers a budget's alarm while it stands, a level with those beneath
/// it, so a week past 90% is told once however long it stays, and isn't
/// told as new past 80% as it eases.
pub(super) fn remember(state: &mut State, key: &str, now: i64) {
    for key in crate::watch::budgets::stands_for(key) {
        state.seen.insert(format!("alarm:{key}"), now);
    }
}

/// The breaches standing now, but for no budget watched until it has stood
/// for `UNWATCHED_FOR` of waking time, so a machine just woken isn't told
/// of it.
pub(super) fn standing(s: &Snapshot, state: &mut State) -> Vec<Breach> {
    let slept = match (&state.asleep, &s.asleep) {
        (Some(before), Some(now)) => {
            before.boot != now.boot || now.millis - before.millis > SLEPT * 1000
        }
        _ => state.ticked.is_some_and(|at| s.now - at > MISSED),
    };
    state.ticked = Some(s.now);
    state.asleep.clone_from(&s.asleep);
    let unwatched_for = if s.budgets_unwatched.is_some() {
        if slept {
            state.unwatched_since = None;
        }
        s.now - *state.unwatched_since.get_or_insert(s.now)
    } else {
        state.unwatched_since = None;
        0
    };
    breaches(s)
        .into_iter()
        .filter(|b| b.key != crate::watch::budgets::UNWATCHED || unwatched_for >= UNWATCHED_FOR)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asking::tests::told_whom;
    use crate::asking::{Question, REMEMBER, To, tick};
    use crate::watch::asleep::Asleep;
    use crate::watch::tests::calm;

    /// The machine's sleep in `boot`, `seconds` of it.
    fn slept(boot: &str, seconds: i64) -> Asleep {
        Asleep {
            boot: boot.into(),
            millis: seconds * 1000,
        }
    }

    /// The proxy token's week at `share`, standing as `standing`.
    fn week(share: f64, standing: &str) -> crate::watch::budgets::Allowance {
        crate::watch::budgets::Allowance {
            budget: "proxy".into(),
            label: "Your AI proxy token".into(),
            name: "weekly".into(),
            window: "Week".into(),
            share,
            standing: standing.into(),
            resets: None,
        }
    }

    #[test]
    fn a_budget_past_a_level_is_told_to_the_orchestrator_once_and_waits_on_no_answer() {
        let mut s = calm(100_000);
        s.budgets = vec![week(0.91, "$36,261 of $40,000")];
        let mut state = State::default();
        let told = tick(&s, &mut state);
        assert_eq!(
            told,
            [Question {
                to: To::Orchestrator,
                key: None,
                text: "Cadence alarm: Your AI proxy token's Week allowance stands at \
                       $36,261 of $40,000, 91%: past 90%, the last level it's told."
                    .into(),
            }],
            "a budget at 91% wasn't told to the orchestrator, or was waited on"
        );
        s.now += 5 * 60;
        assert!(
            tick(&s, &mut state).is_empty(),
            "one standing was told twice"
        );
        // It stands past 90% all week, then eases past 80%.
        for _ in 0..4 {
            s.now += REMEMBER;
            assert!(
                tick(&s, &mut state).is_empty(),
                "a week standing past 90% was told again as it stood"
            );
        }
        s.budgets[0].share = 0.85;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a budget easing from past 90% to past 80% was told as nearing its allowance"
        );
        // Hovering at a level isn't told at each crossing, but a rolling
        // window that eased for longer than BUDGET_REST is told as it
        // climbs back.
        s.budgets[0].share = 0.92;
        s.now += 3600;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a share back past 90% an hour after easing was told again"
        );
        s.budgets[0].share = 0.85;
        s.now += BUDGET_REST + 60;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a share standing past 80% was told"
        );
        s.budgets[0].share = 0.92;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "a rolling window climbing back past 90% after a night's easing went untold"
        );
    }

    #[test]
    fn no_budget_watched_for_ten_minutes_is_told_once_a_stop() {
        let mut s = calm(100_000);
        let mut state = State::default();
        for stop in 1..=2 {
            s.budgets_unwatched = Some("is missing".into());
            // spend-watch may take five minutes after a wake to write.
            for _ in 0..2 {
                assert!(
                    tick(&s, &mut state).is_empty(),
                    "a spend-watch not yet written after a wake was told as stopped"
                );
                s.now += 5 * 60;
            }
            assert_eq!(
                told_whom(&tick(&s, &mut state)),
                [(To::Orchestrator, false)],
                "stop {stop}: no budget watched for ten minutes wasn't told to the \
                 orchestrator, or was waited on"
            );
            // It stands awake past BUDGET_REST, then through a weekend's
            // sleep and on waking.
            for _ in 0..=(BUDGET_REST / 300) {
                s.now += 5 * 60;
                assert!(
                    tick(&s, &mut state).is_empty(),
                    "stop {stop}: a standing stop was told again while awake"
                );
            }
            s.now += REMEMBER;
            for _ in 0..4 {
                assert!(
                    tick(&s, &mut state).is_empty(),
                    "stop {stop}: a standing stop was told again after a sleep"
                );
                s.now += 5 * 60;
            }
            // spend-watch writes again for an hour awake, and stops.
            s.budgets_unwatched = None;
            for _ in 0..12 {
                s.now += 5 * 60;
                assert!(
                    tick(&s, &mut state).is_empty(),
                    "a watched hour told something"
                );
            }
        }
    }

    #[test]
    fn a_wake_too_brief_for_spend_watch_to_write_isnt_counted_into_the_next() {
        let mut s = calm(100_000);
        s.asleep = Some(slept("one", 0));
        let mut state = State::default();
        assert!(tick(&s, &mut state).is_empty(), "nothing stands");
        // A dark wake three hours on reads the file from before the sleep,
        // and the machine sleeps again before spend-watch writes.
        s.now += 3 * 3600;
        s.asleep = Some(slept("one", 3 * 3600 - 300));
        s.budgets_unwatched = Some("was last written 180 min ago".into());
        assert!(
            tick(&s, &mut state).is_empty(),
            "a stale file on a brief wake was told at once"
        );
        s.now += 8 * 3600;
        s.asleep = Some(slept("one", 11 * 3600 - 300));
        s.budgets_unwatched = Some("was last written 660 min ago".into());
        assert!(
            tick(&s, &mut state).is_empty(),
            "the sleep after a brief wake was counted as unwatched time"
        );
        // A wake, a nap of nine minutes, and a wake again.
        s.now += 10 * 60;
        s.asleep = Some(slept("one", 11 * 3600 - 300 + 9 * 60));
        assert!(
            tick(&s, &mut state).is_empty(),
            "a nap was counted as unwatched time"
        );
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "five minutes awake");
        s.now += 5 * 60;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "no budget watched ten minutes after waking went untold"
        );
    }

    #[test]
    fn ticks_skipped_while_awake_never_hide_a_stop() {
        // Ticks run late every other mark, awake throughout.
        let mut s = calm(100_000);
        s.asleep = Some(slept("one", 0));
        s.budgets_unwatched = Some("is missing".into());
        let mut state = State::default();
        assert!(tick(&s, &mut state).is_empty(), "a stop told at once");
        s.now += 10 * 60;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "a tick skipped while awake restarted the stop's count"
        );
        // A boot starts the count again, though neither boot slept.
        s.budgets_unwatched = None;
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "spend-watch writes");
        s.budgets_unwatched = Some("is missing".into());
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "a stop told at once");
        s.now += 5 * 60;
        s.asleep = Some(slept("two", 0));
        assert!(
            tick(&s, &mut state).is_empty(),
            "time before a boot was counted into a stop"
        );
        // Ticks slowed past two marks apiece while awake, as by a stalled
        // fetch, still tell it.
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "five minutes awake");
        s.now += 16 * 60;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "ticks slowed while awake hid a stop"
        );
        // Where sleep can't be read, one tick missed is absorbed.
        s.budgets_unwatched = None;
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "spend-watch writes");
        s.budgets_unwatched = Some("is missing".into());
        s.asleep = None;
        s.now += 5 * 60;
        assert!(tick(&s, &mut state).is_empty(), "a stop told at once");
        s.now += 10 * 60;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "with no clocks, one tick missed restarted the stop's count"
        );
    }

    #[test]
    fn a_level_standing_through_a_sleep_isnt_told_again_on_waking() {
        let mut s = calm(100_000);
        s.budgets = vec![week(0.92, "$36,800 of $40,000")];
        let mut state = State::default();
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "a budget at 92% wasn't told"
        );
        // A night, then a weekend past REMEMBER. The first tick on waking
        // reads the file from before the sleep, and spend-watch writes
        // again minutes later.
        for hours in [8, 64] {
            let standing = std::mem::take(&mut s.budgets);
            s.now += hours * 3600;
            s.budgets_unwatched = Some(format!("was last written {} min ago", hours * 60));
            assert!(
                tick(&s, &mut state).is_empty(),
                "a stale file on waking after {hours} h was told at once"
            );
            s.now += 5 * 60;
            s.budgets_unwatched = None;
            s.budgets = standing;
            assert!(
                tick(&s, &mut state).is_empty(),
                "a level standing through {hours} h asleep was told again on waking"
            );
        }
    }
}
