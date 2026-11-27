//! A TV's own acts: pressing one of its remote's keys, and waking it from
//! standby. Each is asked of a device the record holds as a television,
//! at the address eero gives it, so net reaches only a TV its own answer
//! named one (sources/samsung.rs), or one that has allowed this machine.
//!
//! - `remote`: its keys, over the TV's own channel, after one Allow.
//! - `kept`: what the TV allowed, sealed for this account.
//! - `wake`: Wake-on-LAN to its hardware address.
//!
//! Only the keys below are sent; any other word is refused before the TV
//! is reached, so nothing else of the TV's (its apps, its settings, its
//! text input) goes through this door.

pub mod kept;
pub mod remote;
pub mod wake;

use std::net::IpAddr;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure};

use crate::model::{Device, Role};
use crate::sources::samsung;

/// Each key Joel can press, by the word he types for it, and what Samsung
/// calls it.
pub const KEYS: [(&str, &str); 14] = [
    ("power", "KEY_POWER"),
    ("volume-up", "KEY_VOLUP"),
    ("volume-down", "KEY_VOLDOWN"),
    ("mute", "KEY_MUTE"),
    ("home", "KEY_HOME"),
    ("source", "KEY_SOURCE"),
    ("back", "KEY_RETURN"),
    ("ok", "KEY_ENTER"),
    ("up", "KEY_UP"),
    ("down", "KEY_DOWN"),
    ("left", "KEY_LEFT"),
    ("right", "KEY_RIGHT"),
    ("play", "KEY_PLAY"),
    ("pause", "KEY_PAUSE"),
];

/// What Samsung calls the key Joel calls `word`, in any case.
pub fn code(word: &str) -> Option<&'static str> {
    KEYS.iter()
        .find(|(said, _)| said.eq_ignore_ascii_case(word))
        .map(|(_, code)| *code)
}

/// The words for every key, as one line.
fn words() -> String {
    KEYS.iter()
        .map(|(word, _)| *word)
        .collect::<Vec<_>>()
        .join(", ")
}

/// How long a woken TV has to answer before the wake is said not heard:
/// Joel's Q70C answers its page about 5 s after leaving standby.
const WOKEN: Duration = Duration::from_secs(15);

/// How long one ask of a waking TV's page may take, and how long apart
/// the asks are; neither runs past `WOKEN`.
const ASKED: Duration = Duration::from_secs(2);
const APART: Duration = Duration::from_secs(1);

/// The TV called `device`, by its hardware address: one the record holds
/// as a television, or one that has allowed this machine, since a TV in
/// standby says nothing on its page and the daemon may have started
/// since it last did. `allowed` says whether a TV with a hardware address
/// has allowed this machine.
fn television(device: &Device, allowed: impl Fn(&str) -> bool) -> Result<&str, Failure> {
    let mac = device.macs.first().map(String::as_str);
    let known = device.roles.contains(&Role::Television) || mac.is_some_and(&allowed);
    if !known {
        return Err(Failure::new(
            Exit::Usage,
            format!("{} isn't a TV net has heard from", device.name),
        )
        .because(
            "net knows a TV once it answers as one on its own page or allows this machine, \
             and one in standby since net's daemon started says nothing",
        )
        .try_this("net devices"));
    }
    mac.ok_or_else(|| {
        Failure::new(
            Exit::Unavailable,
            format!("eero gives {} no hardware address", device.name),
        )
    })
}

/// Whether the TV with hardware address `mac` has allowed this machine:
/// what it allowed is kept.
fn allowed_before(mac: &str) -> bool {
    kept::folder().is_ok_and(|folder| kept::file_in(&folder, mac).exists())
}

/// The address eero gives `device` now, if any.
fn address(device: &Device) -> Option<IpAddr> {
    device
        .addresses
        .iter()
        .find_map(|address| address.parse::<IpAddr>().ok().filter(IpAddr::is_ipv4))
}

/// The address a key goes to, refused where eero gives none.
fn reached(device: &Device) -> Result<IpAddr, Failure> {
    address(device).ok_or_else(|| {
        Failure::new(
            Exit::Unavailable,
            format!("eero gives {} no address now", device.name),
        )
        .because("a TV switched off at the wall leaves the network")
    })
}

/// The name the TV's Allow prompt shows: net, and the machine asking,
/// read as it runs.
fn asker() -> String {
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|host| host.trim().to_owned())
        .filter(|host| !host.is_empty());
    match host {
        Some(host) => format!("net on {host}"),
        None => "net".to_owned(),
    }
}

/// `net devices <name> press <key>`: presses the key on the TV, asking it
/// to allow this machine the first time.
pub fn press(context: &Context, device: &Device, word: &str) -> Result<(), Failure> {
    let code = code(word).ok_or_else(|| {
        Failure::new(Exit::Usage, format!("a TV has no key net calls {word}"))
            .because(format!("the keys are {}", words()))
            .try_this(format!("net devices {} press volume-up", word_for(device)))
    })?;
    let mac = television(device, allowed_before)?;
    let address = reached(device)?;
    let kept = kept::open(mac).map_err(|error| unopened(device, word, &error))?;
    let first = kept.as_ref().is_none_or(|kept| kept.token.is_none());
    let mut waiting = first
        .then(|| {
            context
                .wait(&format!("press Allow on {}'s screen", device.name))
                .ok()
        })
        .flatten();
    let pressed = remote::press(address, &asker(), code, kept.as_ref());
    let kept_now = match pressed {
        Ok(kept_now) => {
            if let Some(waiting) = waiting.take() {
                let _ = waiting.done(&format!("{} allowed this machine", device.name));
            }
            kept_now
        }
        Err(why) => {
            if let Some(waiting) = waiting.take() {
                let _ = waiting.failed(&unallowed(device, &why));
            }
            return Err(refused(device, word, why));
        }
    };
    if kept.as_ref() != Some(&kept_now)
        && let Err(error) = kept::keep(mac, &kept_now)
    {
        let _ = context.tell(
            "!",
            "warning",
            &format!(
                "the key went, but what {} allowed couldn't be kept, so it will ask again: {error}",
                device.name
            ),
        );
    }
    said(
        context,
        &pressed_answer(device, word, first),
        &format!(
            "Pressed {} on {}.\n",
            spoken(word),
            terminal_kit::layout::spaced(&device.name)
        ),
    )
}

/// `net.pressed/1`: which key went to which TV, and whether the TV was
/// asked to allow this machine for it.
fn pressed_answer(device: &Device, word: &str, first: bool) -> Value {
    json!({
        "schema": "net.pressed/1",
        "device": device.id,
        "key": key_word(word),
        "allowed_now": first,
    })
}

/// What the wait for Allow ends on, by why the key didn't go: only a TV
/// that refused or stopped asking didn't allow this machine.
fn unallowed(device: &Device, why: &remote::Why) -> String {
    let name = &device.name;
    match why {
        remote::Why::Refused | remote::Why::Unanswered => {
            format!("{name} didn't allow this machine")
        }
        remote::Why::Unreachable(_) => format!("{name} didn't answer"),
        remote::Why::NotTheTv => format!("{name}'s address answered as another box"),
        remote::Why::Broke(_) | remote::Why::Unlike(_) => format!("{name} broke off"),
    }
}

/// Why what the TV allowed can't be opened, with the way out: forgetting
/// it, so the TV asks to allow this machine again.
fn unopened(device: &Device, word: &str, error: &std::io::Error) -> Failure {
    let failure = Failure::new(
        Exit::Failed,
        format!("what {} allowed can't be opened", device.name),
    )
    .because(format!(
        "{error}; forgetting it means pressing Allow on the TV again"
    ));
    match kept_file(device) {
        Some(file) => failure.try_this(format!("rm {file} && {}", again(device, word))),
        None => failure,
    }
}

/// Where what the TV allowed is kept, as a word for the shell.
fn kept_file(device: &Device) -> Option<String> {
    let mac = device.macs.first()?;
    let file = kept::file_in(&kept::folder().ok()?, mac);
    Some(terminal_kit::failure::shell_word(&file.display().to_string()).into_owned())
}

/// Pressing the same key again, as a try line.
fn again(device: &Device, word: &str) -> String {
    format!("net devices {} press {}", word_for(device), key_word(word))
}

/// Why a key wasn't pressed, in the three parts every failure has.
fn refused(device: &Device, word: &str, why: remote::Why) -> Failure {
    let name = &device.name;
    let again = again(device, word);
    match why {
        remote::Why::Unreachable(error) => {
            Failure::new(Exit::Unavailable, format!("{name} didn't answer"))
                .because(format!("it's asleep or off ({error})"))
                .try_this(format!("net devices {} wake", word_for(device)))
        }
        remote::Why::NotTheTv => {
            let what = format!("what answered at {name}'s address isn't the TV that allowed this");
            let failure = Failure::new(Exit::Refused, what).because(
                "its certificate isn't the one kept; a TV reset to its factory settings makes \
                 a new one, and forgetting the old pins whatever answers next, so do it only \
                 once the TV was reset",
            );
            match kept_file(device) {
                Some(file) => failure.try_this(format!("rm {file} && {again}")),
                None => failure,
            }
        }
        remote::Why::Refused => Failure::new(Exit::Refused, format!("{name} refused this machine"))
            .because(
                "it was told Deny, or it forgot this machine; it asks again once this \
                     machine is taken off its Device Connection Manager's list",
            )
            .try_this(again),
        remote::Why::Unanswered => {
            Failure::new(Exit::Refused, format!("{name} didn't allow this machine"))
                .because(
                    "it asks on its own screen, and nobody pressed Allow before it stopped \
                     asking",
                )
                .try_this(again)
        }
        remote::Why::Broke(error) => Failure::new(Exit::TryAgain, format!("{name} broke off"))
            .because(error)
            .try_this(again),
        remote::Why::Unlike(error) => Failure::new(
            Exit::Failed,
            format!("what answered at {name}'s address doesn't speak as a Samsung TV does"),
        )
        .because(error),
    }
}

/// `net devices <name> wake`: wakes the TV from standby, waiting until it
/// answers as awake or `WOKEN` passes. A TV eero gives no address is sent
/// its wake all the same, by its hardware address alone, but can't be
/// heard waking.
pub fn wake(context: &Context, device: &Device) -> Result<(), Failure> {
    let mac = television(device, allowed_before)?;
    let address = address(device);
    wake::send(mac, address).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("{} couldn't be sent its wake", device.name),
        )
        .because(error.to_string())
    })?;
    let name = terminal_kit::layout::spaced(&device.name);
    let Some(address) = address else {
        return said(
            context,
            &woken_answer(device, false),
            &format!(
                "Sent {name} its wake; eero gives it no address, so net can't hear it wake.\n"
            ),
        );
    };
    let waiting = context.wait(&format!("waking {}", device.name)).ok();
    let until = Instant::now() + WOKEN;
    let woke = loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break false;
        }
        if awake(address, left.min(ASKED)) {
            break true;
        }
        std::thread::sleep(until.saturating_duration_since(Instant::now()).min(APART));
    };
    if !woke {
        if let Some(waiting) = waiting {
            let _ = waiting.failed(&format!("{} didn't wake", device.name));
        }
        return Err(
            Failure::new(Exit::Unavailable, format!("{} didn't wake", device.name))
                .because(format!(
                    "it answered nothing for {} s; a TV on Wi-Fi hears a wake only with \
                 Power On with Mobile on, in its network settings",
                    WOKEN.as_secs()
                ))
                .try_this(format!("net devices {} wake", word_for(device))),
        );
    }
    if let Some(waiting) = waiting {
        let _ = waiting.done(&format!("{} is awake", device.name));
    }
    said(
        context,
        &woken_answer(device, true),
        &format!("{name} is awake.\n"),
    )
}

/// `net.woken/1`: which TV was sent its wake, and whether it was heard
/// waking.
fn woken_answer(device: &Device, heard: bool) -> Value {
    json!({"schema": "net.woken/1", "device": device.id, "heard": heard})
}

/// Whether the TV at `address` says it's on, asked as the daemon asks it,
/// for `longest` at most and following no redirect.
fn awake(address: IpAddr, longest: Duration) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(longest))
        .max_redirects(0)
        .build()
        .into();
    agent
        .get(&samsung::url(&address.to_string()))
        .call()
        .and_then(|mut answer| {
            answer
                .body_mut()
                .with_config()
                .limit(samsung::LONGEST)
                .read_to_string()
        })
        .is_ok_and(|page| samsung::on_now(&page))
}

/// The answer as JSON when asked for, and otherwise the line.
fn said(context: &Context, answer: &Value, line: &str) -> Result<(), Failure> {
    let written = if context.format().is_json() {
        context.answer(answer)
    } else {
        context.say(line)
    };
    written.map_err(|error| {
        Failure::new(Exit::Failed, "net couldn't write its answer").because(error.to_string())
    })
}

/// The key's own word, as the list has it, for a word typed in any case.
fn key_word(word: &str) -> String {
    word.to_ascii_lowercase()
}

/// The key as a sentence says it: volume up, not volume-up.
fn spoken(word: &str) -> String {
    key_word(word).replace('-', " ")
}

/// The device's name as a shell word, for a try line.
fn word_for(device: &Device) -> String {
    terminal_kit::failure::shell_word(&device.name).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device called `name`, with `roles`, the hardware address
    /// 02:00:00:00:00:90 and `addresses`.
    fn device(name: &str, roles: &[&str], addresses: &[&str]) -> Device {
        serde_json::from_value(json!({
            "id": "mac:02:00:00:00:00:90",
            "name": name,
            "roles": roles,
            "source": "eero",
            "macs": ["02:00:00:00:00:90"],
            "addresses": addresses,
            "ports": [],
            "radios": [],
        }))
        .expect("a device")
    }

    fn never(_mac: &str) -> bool {
        false
    }

    #[test]
    fn each_key_is_found_by_its_word_in_any_case() {
        for (word, said) in KEYS {
            assert_eq!(code(word), Some(said));
            assert_eq!(code(&word.to_uppercase()), Some(said));
        }
    }

    #[test]
    fn a_word_no_key_goes_by_is_no_key() {
        for word in ["", "KEY_POWER", "factory-reset", "volume up", "power "] {
            assert_eq!(code(word), None, "{word:?}");
        }
    }

    #[test]
    fn every_key_goes_by_one_word_and_one_code() {
        let mut words: Vec<&str> = KEYS.iter().map(|(word, _)| *word).collect();
        let mut codes: Vec<&str> = KEYS.iter().map(|(_, code)| *code).collect();
        words.sort_unstable();
        words.dedup();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!((words.len(), codes.len()), (KEYS.len(), KEYS.len()));
    }

    #[test]
    fn a_key_is_said_in_plain_words() {
        assert_eq!(spoken("Volume-Up"), "volume up");
    }

    #[test]
    fn a_tv_heard_on_its_page_or_one_that_allowed_this_machine_is_a_tv() {
        let heard = device("Living", &["television"], &[]);
        assert_eq!(
            television(&heard, never).map_err(|failure| failure.json(false)),
            Ok("02:00:00:00:00:90")
        );
        let silent = device("Living", &["client"], &[]);
        assert_eq!(
            television(&silent, |mac| mac == "02:00:00:00:00:90")
                .map_err(|failure| failure.json(false)),
            Ok("02:00:00:00:00:90"),
            "a TV in standby that allowed this machine isn't taken"
        );
        let refused = television(&silent, never).expect_err("a client is taken as a TV");
        assert_eq!(refused.code(), Exit::Usage.code());
    }

    #[test]
    fn a_key_goes_only_to_an_address_eero_gives_now() {
        let tv = device("Living", &["television"], &["fe80::1", "10.0.0.90"]);
        assert_eq!(
            reached(&tv).map_err(|failure| failure.json(false)),
            Ok("10.0.0.90".parse::<IpAddr>().expect("an address"))
        );
        let gone = device("Living", &["television"], &[]);
        let failure = reached(&gone).expect_err("a TV with no address is reached");
        assert_eq!(failure.code(), Exit::Unavailable.code());
        assert_eq!(address(&gone), None, "a wake has an address to ask");
    }

    #[test]
    fn each_reason_a_key_didnt_go_has_its_own_exit_and_way_out() {
        let tv = device("Living", &["television"], &["10.0.0.90"]);
        let cases = [
            (
                remote::Why::Unreachable("timed out".into()),
                Exit::Unavailable,
                Some("wake"),
            ),
            (remote::Why::Refused, Exit::Refused, Some("press mute")),
            (remote::Why::Unanswered, Exit::Refused, Some("press mute")),
            (
                remote::Why::Broke("reset".into()),
                Exit::TryAgain,
                Some("press mute"),
            ),
            (remote::Why::Unlike("bad frame".into()), Exit::Failed, None),
        ];
        for (why, exit, then) in cases {
            let said = format!("{why:?}");
            let failure = refused(&tv, "Mute", why).json(false);
            assert_eq!(failure["exit"], exit.code(), "{said}");
            match then {
                Some(then) => assert!(
                    failure["try"]
                        .as_str()
                        .is_some_and(|tried| tried.contains(then)),
                    "{said} offers {}",
                    failure["try"]
                ),
                None => assert!(failure["try"].is_null(), "{said} offers a retry"),
            }
        }
    }

    #[test]
    fn only_a_tv_that_refused_or_stopped_asking_is_said_not_to_allow() {
        let tv = device("Living", &["television"], &[]);
        for why in [remote::Why::Refused, remote::Why::Unanswered] {
            assert_eq!(unallowed(&tv, &why), "Living didn't allow this machine");
        }
        for why in [
            remote::Why::Unreachable("timed out".into()),
            remote::Why::NotTheTv,
            remote::Why::Broke("reset".into()),
            remote::Why::Unlike("bad frame".into()),
        ] {
            let said = unallowed(&tv, &why);
            assert!(!said.contains("allow"), "{why:?} blames Allow: {said}");
        }
    }

    #[test]
    fn the_answers_say_the_device_the_key_and_what_was_heard() {
        let tv = device("Living", &["television"], &[]);
        assert_eq!(
            pressed_answer(&tv, "Volume-Up", true),
            json!({
                "schema": "net.pressed/1",
                "device": "mac:02:00:00:00:00:90",
                "key": "volume-up",
                "allowed_now": true,
            })
        );
        assert_eq!(
            woken_answer(&tv, false),
            json!({"schema": "net.woken/1", "device": "mac:02:00:00:00:00:90", "heard": false})
        );
    }
}
