//! The rescue of a hibernation that rolled back, with the machine never
//! off: systemd runs it ten seconds after the sleep, and by the lid and
//! whether it has tried once already it says so, hibernates again, sleeps
//! or powers off, so a laptop is never left on in silence.

use crate::notice::{notice, say_to_each_session};
use crate::said;
use crate::why::Why;
use std::env;
use std::fs;
use std::path::Path;

/// Kept while a rolled-back hibernation is tried again, so a second
/// rollback powers off rather than trying forever.
pub(crate) const TRIED_AGAIN: &str = "/run/hibernate-unlock-tried-again";

/// What a rescue does once a hibernation rolled back.
#[derive(Debug, PartialEq, Eq)]
enum Rescue {
    HibernateAgain,
    PowerOff,
    Sleep,
    Say,
}

/// With the lid open someone is there to be told. With it shut, sleep
/// where a second try would fail the same way; otherwise one more try,
/// then off.
fn rescue(lid_shut: bool, tried_again: bool, why: &Why) -> Rescue {
    match (lid_shut, tried_again) {
        (false, _) => Rescue::Say,
        (true, _) if why.fails_again() => Rescue::Sleep,
        (true, false) => Rescue::HibernateAgain,
        (true, true) => Rescue::PowerOff,
    }
}

/// What follows the rescue asking the laptop to sleep: nothing once it
/// sleeps, and off when it couldn't, so a laptop with its lid shut is never
/// left on in a bag.
fn after_asking_to_sleep(slept: bool) -> Option<Rescue> {
    (!slept).then_some(Rescue::PowerOff)
}

/// Whether a lid's `/proc/acpi/button/lid/*/state` says it's shut.
fn says_shut(state: &str) -> bool {
    state.split_whitespace().last() == Some("closed")
}

/// Asks systemd to run the rescue in ten seconds, once this sleep is over
/// and the sessions are thawed, with the same programs on its `PATH`.
pub(crate) fn rolled_back(why: &Why) {
    let words = why.args().join(" ");
    eprintln!("hibernate-unlock: the hibernation rolled back, so the machine is still on: {words}");
    let Ok(me) = env::current_exe() else {
        eprintln!("hibernate-unlock: couldn't find itself to rescue the machine");
        return;
    };
    let path = format!("--setenv=PATH={}", env::var("PATH").unwrap_or_default());
    let me = me.to_string_lossy();
    let why = why.args();
    let mut args = vec![
        "--on-active=10s",
        "--collect",
        "--quiet",
        "--description=Rescues a hibernation that rolled back",
        &path,
        &me,
        "rescue",
    ];
    args.extend(why.iter().map(String::as_str));
    if said("systemd-run", &args).is_none() {
        eprintln!("hibernate-unlock: systemd-run couldn't start the rescue");
    }
}

fn lid_shut() -> bool {
    fs::read_dir("/proc/acpi/button/lid")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|lid| fs::read_to_string(lid.path().join("state")).ok())
        .any(|state| says_shut(&state))
}

/// What the rescue ten seconds after a rollback does.
pub(crate) fn rescue_now(why: &Why) {
    let tried_again = Path::new(TRIED_AGAIN).exists();
    match rescue(lid_shut(), tried_again, why) {
        Rescue::HibernateAgain => {
            eprintln!("hibernate-unlock: the lid is shut, so it hibernates again");
            let _ = fs::write(TRIED_AGAIN, "");
            let _ = said("systemctl", &["hibernate", "--check-inhibitors=no"]);
        }
        Rescue::PowerOff => {
            eprintln!(
                "hibernate-unlock: the lid is shut and it rolled back twice, so it powers off"
            );
            power_off();
        }
        Rescue::Sleep => {
            eprintln!(
                "hibernate-unlock: the lid is shut and hibernating again would fail the same \
                 way, so it sleeps"
            );
            let _ = fs::remove_file(TRIED_AGAIN);
            say_to_each_session(&notice(why, true));
            let slept = said("systemctl", &["suspend", "--check-inhibitors=no"]).is_some();
            if after_asking_to_sleep(slept) == Some(Rescue::PowerOff) {
                eprintln!(
                    "hibernate-unlock: the lid is shut and it couldn't sleep, so it powers off"
                );
                power_off();
            }
        }
        Rescue::Say => {
            eprintln!("hibernate-unlock: the lid is open, so it says so");
            let _ = fs::remove_file(TRIED_AGAIN);
            say_to_each_session(&notice(why, false));
        }
    }
}

fn power_off() {
    let _ = fs::remove_file(TRIED_AGAIN);
    let _ = said("systemctl", &["poweroff", "--check-inhibitors=no"]);
}

#[cfg(test)]
mod tests {
    use super::{Rescue, after_asking_to_sleep, rescue, says_shut};
    use crate::why::Why;
    use crate::why::tests::cases;

    #[test]
    fn a_rollback_with_the_lid_open_is_said() {
        for (why, _) in cases() {
            assert_eq!(rescue(false, false, &why), Rescue::Say);
            assert_eq!(rescue(false, true, &why), Rescue::Say);
        }
    }

    #[test]
    fn short_of_memory_or_swap_with_the_lid_shut_it_sleeps() {
        for why in [Why::Memory, Why::SwapGone, Why::SwapFull] {
            assert_eq!(rescue(true, false, &why), Rescue::Sleep, "{why:?}");
            assert_eq!(rescue(true, true, &why), Rescue::Sleep, "{why:?}");
        }
    }

    #[test]
    fn a_laptop_that_cannot_sleep_powers_off() {
        assert_eq!(after_asking_to_sleep(true), None);
        assert_eq!(after_asking_to_sleep(false), Some(Rescue::PowerOff));
    }

    #[test]
    fn otherwise_with_the_lid_shut_it_tries_once_then_powers_off() {
        for (why, _) in cases().into_iter().filter(|(why, _)| !why.fails_again()) {
            assert_eq!(rescue(true, false, &why), Rescue::HibernateAgain, "{why:?}");
            assert_eq!(rescue(true, true, &why), Rescue::PowerOff, "{why:?}");
        }
    }

    #[test]
    fn the_lid_reads_shut_only_when_it_says_closed() {
        assert_eq!(
            [
                says_shut("state:      closed\n"),
                says_shut("state:      open\n"),
                says_shut("")
            ],
            [true, false, false],
            "a lid saying closed wasn't read as shut, or one saying open or nothing was"
        );
    }
}
