//! A TV's own acts: pressing one of its remote's keys, and waking it from
//! standby. Each is asked of a device the record holds as a television,
//! at the address eero gives it, so net reaches only a TV its own answer
//! named one (sources/samsung.rs).
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

use serde_json::json;
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

/// The TV called `device`: its hardware address and its address, refused
/// where the record doesn't hold it as one.
fn reached(device: &Device) -> Result<(&str, IpAddr), Failure> {
    if !device.roles.contains(&Role::Television) {
        return Err(Failure::new(
            Exit::Usage,
            format!("{} isn't a TV net has heard from", device.name),
        )
        .because("only a TV answering as one on its own page takes its keys")
        .try_this("net devices"));
    }
    let mac = device.macs.first().ok_or_else(|| {
        Failure::new(
            Exit::Unavailable,
            format!("eero gives {} no hardware address", device.name),
        )
    })?;
    let address = device
        .addresses
        .iter()
        .find_map(|address| address.parse::<IpAddr>().ok().filter(IpAddr::is_ipv4))
        .ok_or_else(|| {
            Failure::new(
                Exit::Unavailable,
                format!("eero gives {} no address now", device.name),
            )
            .because("a TV switched off at the wall leaves the network")
        })?;
    Ok((mac, address))
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
    let (mac, address) = reached(device)?;
    let kept = kept::open(mac).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("what {} allowed can't be opened", device.name),
        )
        .because(error.to_string())
    })?;
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
                let _ = waiting.failed(&format!("{} didn't allow this machine", device.name));
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
    let answer = json!({
        "schema": "net.pressed/1",
        "device": device.id,
        "key": key_word(word),
        "allowed_now": first,
    });
    said(
        context,
        &answer,
        &format!("Pressed {} on {}.\n", spoken(word), device.name),
    )
}

/// Where what the TV allowed is kept, as a word for the shell.
fn kept_file(device: &Device) -> Option<String> {
    let mac = device.macs.first()?;
    let file = kept::file_in(&kept::folder().ok()?, mac);
    Some(terminal_kit::failure::shell_word(&file.display().to_string()).into_owned())
}

/// Why a key wasn't pressed, in the three parts every failure has.
fn refused(device: &Device, word: &str, why: remote::Why) -> Failure {
    let name = &device.name;
    let again = format!("net devices {} press {}", word_for(device), key_word(word));
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
                 a new one",
            );
            match kept_file(device) {
                Some(file) => failure.try_this(format!("rm {file} && {again}")),
                None => failure,
            }
        }
        remote::Why::NotAllowed => {
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
    }
}

/// `net devices <name> wake`: wakes the TV from standby, waiting until it
/// answers as awake or `WOKEN` passes.
pub fn wake(context: &Context, device: &Device) -> Result<(), Failure> {
    let (mac, address) = reached(device)?;
    wake::send(mac, Some(address)).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("{} couldn't be sent its wake", device.name),
        )
        .because(error.to_string())
    })?;
    let waiting = context.wait(&format!("waking {}", device.name)).ok();
    let until = Instant::now() + WOKEN;
    let woke = loop {
        if awake(address) {
            break true;
        }
        if Instant::now() >= until {
            break false;
        }
        std::thread::sleep(Duration::from_secs(1));
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
    let answer = json!({"schema": "net.woken/1", "device": device.id});
    said(context, &answer, &format!("{} is awake.\n", device.name))
}

/// Whether the TV at `address` says it's on, asked as the daemon asks it,
/// for two seconds at most and following no redirect.
fn awake(address: IpAddr) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
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
fn said(context: &Context, answer: &serde_json::Value, line: &str) -> Result<(), Failure> {
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
}
