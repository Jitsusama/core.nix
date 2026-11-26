//! What whoever is signed in is told: the words for a hibernation that
//! rolled back, and the way a notice reaches each user at a seat, through
//! their own systemd.

use crate::said;
use crate::why::Why;
use serde_json::Value;
use std::env;
use std::path::PathBuf;

/// What a notice says: who, why, then what to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Notice {
    pub(crate) title: String,
    pub(crate) body: String,
}

/// The notice for a rollback, said with the laptop still on, or once it
/// has gone to sleep instead.
pub(crate) fn notice(why: &Why, slept: bool) -> Notice {
    let kept = if slept {
        "Nothing was lost: with the lid shut, it went to sleep instead."
    } else {
        "Nothing was lost; the laptop is still on."
    };
    let again = "Hibernate again in a moment; if it fails twice, put the laptop to sleep instead.";
    let close = "Close something you don't need, then hibernate again.";
    let (title, said, todo) = match why {
        Why::Memory => (
            "Not enough memory to hibernate".to_owned(),
            "Memory was too full to copy before writing it to the swap file.".to_owned(),
            close.to_owned(),
        ),
        Why::SwapGone => (
            "No swap file to hibernate to".to_owned(),
            "The swap file that memory is written to wasn't there.".to_owned(),
            "Put it to sleep instead: hibernating again would fail the same way.".to_owned(),
        ),
        Why::SwapFull => (
            "Not enough swap to hibernate".to_owned(),
            "The swap file had no room left for memory.".to_owned(),
            close.to_owned(),
        ),
        Why::Device(driver) => (
            format!("{driver} wouldn't let it hibernate"),
            format!("The driver {driver} refused to stop its device."),
            again.to_owned(),
        ),
        Why::Program(program) => (
            format!("{program} wouldn't let it hibernate"),
            format!("{program} wouldn't pause."),
            format!("Close {program}, then hibernate again."),
        ),
        Why::Refused => (
            "A device wouldn't let it hibernate".to_owned(),
            "Something refused to stop, and the kernel didn't say what.".to_owned(),
            again.to_owned(),
        ),
        Why::Unwritten => (
            "Hibernation couldn't write memory".to_owned(),
            "Memory couldn't be written to the swap file.".to_owned(),
            again.to_owned(),
        ),
        Why::Woken => (
            "Woken while hibernating".to_owned(),
            "Something woke the laptop as it went down: a key, the lid or a cable.".to_owned(),
            "Hibernate again, and leave it be until it's off.".to_owned(),
        ),
        Why::Unsaid => (
            "Hibernation didn't finish".to_owned(),
            "The kernel didn't say why.".to_owned(),
            again.to_owned(),
        ),
    };
    Notice {
        title,
        body: format!("{said} {kept} {todo}"),
    }
}

/// The users signed in at a seat, from `loginctl list-sessions --json=short`.
fn seated(sessions: &Value) -> Vec<String> {
    let mut users: Vec<String> = sessions
        .as_array()
        .into_iter()
        .flatten()
        .filter(|session| {
            session["seat"]
                .as_str()
                .is_some_and(|seat| !seat.is_empty())
        })
        .filter_map(|session| session["user"].as_str().map(str::to_owned))
        .collect();
    users.sort();
    users.dedup();
    users
}

fn on_path(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// Puts a notice in each seated user's session, through their own systemd.
pub(crate) fn say_to_each_session(notice: &Notice) {
    eprintln!(
        "hibernate-unlock: it says: {}: {}",
        notice.title, notice.body
    );
    let Some(notify) = on_path("notify-send") else {
        eprintln!("hibernate-unlock: no notify-send to say the hibernation didn't finish");
        return;
    };
    let notify = notify.to_string_lossy();
    let sessions = said("loginctl", &["list-sessions", "--json=short"])
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null);
    for user in seated(&sessions) {
        let machine = format!("--machine={user}@.host");
        let args = [
            machine.as_str(),
            "--user",
            "--collect",
            "--quiet",
            &notify,
            "--urgency=critical",
            "--app-name=Hibernation",
            &notice.title,
            &notice.body,
        ];
        if said("systemd-run", &args).is_none() {
            eprintln!("hibernate-unlock: couldn't tell {user} the hibernation didn't finish");
        }
    }
}

/// Whether a user at a seat has a graphical session up.
pub(crate) fn signed_in() -> bool {
    let sessions = said("loginctl", &["list-sessions", "--json=short"])
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null);
    seated(&sessions).iter().any(|user| {
        let machine = format!("--machine={user}@.host");
        said(
            "systemctl",
            &[
                &machine,
                "--user",
                "is-active",
                "--quiet",
                "graphical-session.target",
            ],
        )
        .is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::{Notice, notice, seated};
    use crate::why::Why;
    use crate::why::tests::cases;
    use serde_json::{Value, json};

    /// Every notice, word for word, with the laptop still on.
    #[test]
    fn each_notice_says_why_that_nothing_was_lost_and_what_to_do() {
        let kept = "Nothing was lost; the laptop is still on.";
        let again = "Hibernate again in a moment; if it fails twice, put the laptop to sleep \
                     instead.";
        let close = "Close something you don't need, then hibernate again.";
        let said = [
            (
                Why::Memory,
                "Not enough memory to hibernate",
                format!(
                    "Memory was too full to copy before writing it to the swap file. {kept} \
                     {close}"
                ),
            ),
            (
                Why::SwapGone,
                "No swap file to hibernate to",
                format!(
                    "The swap file that memory is written to wasn't there. {kept} Put it to \
                     sleep instead: hibernating again would fail the same way."
                ),
            ),
            (
                Why::SwapFull,
                "Not enough swap to hibernate",
                format!("The swap file had no room left for memory. {kept} {close}"),
            ),
            (
                Why::Device("amdgpu".to_owned()),
                "amdgpu wouldn't let it hibernate",
                format!("The driver amdgpu refused to stop its device. {kept} {again}"),
            ),
            (
                Why::Program("fuse-thing".to_owned()),
                "fuse-thing wouldn't let it hibernate",
                format!(
                    "fuse-thing wouldn't pause. {kept} Close fuse-thing, then hibernate again."
                ),
            ),
            (
                Why::Refused,
                "A device wouldn't let it hibernate",
                format!(
                    "Something refused to stop, and the kernel didn't say what. {kept} {again}"
                ),
            ),
            (
                Why::Unwritten,
                "Hibernation couldn't write memory",
                format!("Memory couldn't be written to the swap file. {kept} {again}"),
            ),
            (
                Why::Woken,
                "Woken while hibernating",
                format!(
                    "Something woke the laptop as it went down: a key, the lid or a cable. \
                     {kept} Hibernate again, and leave it be until it's off."
                ),
            ),
            (
                Why::Unsaid,
                "Hibernation didn't finish",
                format!("The kernel didn't say why. {kept} {again}"),
            ),
        ];
        for (why, title, body) in said {
            assert_eq!(
                notice(&why, false),
                Notice {
                    title: title.to_owned(),
                    body
                }
            );
        }
    }

    #[test]
    fn a_notice_said_as_it_sleeps_says_it_slept() {
        assert_eq!(
            notice(&Why::Memory, true).body,
            "Memory was too full to copy before writing it to the swap file. Nothing was \
             lost: with the lid shut, it went to sleep instead. Close something you don't \
             need, then hibernate again."
        );
    }

    #[test]
    fn with_no_swap_file_it_never_says_to_hibernate_again() {
        let Notice { body, .. } = notice(&Why::SwapGone, false);
        assert!(!body.contains("then hibernate again"), "{body}");
    }

    #[test]
    fn every_notice_is_plain_text() {
        for (why, _) in cases() {
            for slept in [false, true] {
                let Notice { title, body } = notice(&why, slept);
                for text in [title, body] {
                    assert!(text.is_ascii(), "{text}");
                    assert!(!text.contains("--"), "{text}");
                }
            }
        }
    }

    #[test]
    fn only_users_at_a_seat_are_told_once_each() {
        let sessions = json!([
            {"session": "1", "user": "joel", "seat": "seat0"},
            {"session": "2", "user": "joel", "seat": "seat0"},
            {"session": "3", "user": "ssh", "seat": null},
            {"session": "4", "user": "cron", "seat": ""}
        ]);
        assert_eq!(seated(&sessions), vec!["joel".to_owned()]);
        assert_eq!(seated(&Value::Null), [] as [std::string::String; 0]);
    }
}
