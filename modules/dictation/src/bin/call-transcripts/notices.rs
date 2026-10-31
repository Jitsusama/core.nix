//! The notices call-transcripts raises, every one from Calls, in
//! VKCF3Z's words: a call being transcribed, its line for the call's chat
//! offered to copy; its summary ready; and what couldn't be done, and why.
//!
//! Each answers a call Joel is on or just had, so none is ever counted
//! missed (the shell's `x-answers`). Each waits on a thread of its own for
//! the act Joel chooses, which it's given by its name.

use std::io::Write;
use std::process::{Command, Stdio};

/// The line offered for the call's chat, telling the others, for Joel to
/// paste; never posted by itself, since that would be posting in his name.
pub const LINE: &str = "Heads up: I keep a transcript of my calls on my laptop, for my own \
                        notes. Nothing is uploaded. Say if you'd rather I didn't.";

/// A notice, and the acts it offers, by name and label in the order shown.
/// An act named `default` is the card's own: clicking it or pressing
/// Return, never shown as a button of its own.
pub struct Notice {
    pub heading: String,
    pub body: String,
    pub acts: Vec<(&'static str, &'static str)>,
    /// Raised with no sound, for a call that's on.
    pub quiet: bool,
}

/// A call's name in a heading: its title, or "the Call" with none.
fn named(title: &str, start: bool) -> String {
    match (title.is_empty(), start) {
        (false, _) => title.to_owned(),
        (true, true) => "The Call".to_owned(),
        (true, false) => "the Call".to_owned(),
    }
}

/// The notice as a call starts: the line offered to copy, and the way to
/// honour an objection to it in one key.
pub fn being_transcribed(title: &str) -> Notice {
    Notice {
        heading: format!("{} Is Being Transcribed", named(title, true)),
        body: LINE.to_owned(),
        acts: vec![
            ("stop", "Stop Transcribing"),
            ("forget", "Forget"),
            ("copy", "Copy the Line"),
            ("default", "Copy the Line"),
        ],
        quiet: true,
    }
}

/// The notice once a call's summary is written: its first line, and Read
/// It to open it.
pub fn summed_up(title: &str, first: &str) -> Notice {
    Notice {
        heading: format!("Summary of {} Is Ready", named(title, false)),
        body: first.to_owned(),
        acts: vec![("read", "Read It"), ("forget", "Forget")],
        quiet: false,
    }
}

/// The notice when a call couldn't be heard at all.
pub fn not_transcribed(title: &str, why: &str) -> Notice {
    Notice {
        heading: format!("{} Couldn't Be Transcribed", named(title, true)),
        body: why.to_owned(),
        acts: vec![("forget", "Forget")],
        quiet: true,
    }
}

/// The notice when a call written down couldn't be summed up: Try Again
/// first when the cause can pass, such as a busy NPU or a full disk.
pub fn not_summed_up(title: &str, why: &str, can_pass: bool) -> Notice {
    let mut acts = Vec::new();
    if can_pass {
        acts.push(("again", "Try Again"));
    }
    acts.push(("transcript", "Open the Transcript"));
    acts.push(("forget", "Forget"));
    Notice {
        heading: format!("Summary of {} Couldn't Be Made", named(title, false)),
        body: why.to_owned(),
        acts,
        quiet: false,
    }
}

/// notify-send's arguments for a notice, the heading and body after `--`,
/// since a body that opens with a dash is never an option.
fn arguments(notice: &Notice) -> Vec<String> {
    let mut args = vec![
        "--app-name=Calls".to_owned(),
        "--hint=string:x-answers:call".to_owned(),
    ];
    if notice.quiet {
        args.push("--hint=boolean:suppress-sound:true".to_owned());
    }
    args.extend(
        notice
            .acts
            .iter()
            .map(|(name, label)| format!("--action={name}={label}")),
    );
    args.extend([
        "--wait".to_owned(),
        "--".to_owned(),
        notice.heading.clone(),
        notice.body.clone(),
    ]);
    args
}

/// Raises a notice, and gives `then` the act Joel chose, on a thread of
/// its own; a notice let go without one gives nothing.
pub fn show(notice: &Notice, then: impl FnOnce(&str) + Send + 'static) {
    let args = arguments(notice);
    std::thread::spawn(
        move || match Command::new("notify-send").args(&args).output() {
            Ok(done) if done.status.success() => {
                let chosen = String::from_utf8_lossy(&done.stdout);
                let chosen = chosen.trim();
                if !chosen.is_empty() {
                    then(chosen);
                }
            }
            Ok(done) => eprintln!(
                "call-transcripts: a notice wasn't shown: {}",
                String::from_utf8_lossy(&done.stderr).trim()
            ),
            Err(why) => eprintln!("call-transcripts: a notice wasn't shown: {why}"),
        },
    );
}

/// Puts the chat's line on the clipboard, for Joel to paste.
pub fn copy_line() {
    let copied = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut input) = child.stdin.take() {
                input.write_all(LINE.as_bytes())?;
            }
            child.wait()
        });
    if let Err(why) = copied {
        eprintln!("call-transcripts: the line wasn't copied: {why}");
    }
}

/// Opens a call's file as any file is opened.
pub fn open(file: &std::path::Path) {
    if let Err(why) = Command::new("xdg-open").arg(file).status() {
        eprintln!("call-transcripts: {} wasn't opened: {why}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_starting_offers_its_line_quietly_and_a_way_to_stop() {
        let notice = being_transcribed("Review Planning");
        assert_eq!(notice.heading, "Review Planning Is Being Transcribed");
        assert_eq!(notice.body, LINE);
        assert!(notice.quiet);
        let names: Vec<&str> = notice.acts.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["stop", "forget", "copy", "default"]);
    }

    #[test]
    fn a_call_with_no_title_is_named_the_call() {
        assert_eq!(
            being_transcribed("").heading,
            "The Call Is Being Transcribed"
        );
        assert_eq!(summed_up("", "x").heading, "Summary of the Call Is Ready");
        assert_eq!(
            not_transcribed("", "x").heading,
            "The Call Couldn't Be Transcribed"
        );
    }

    #[test]
    fn try_again_comes_first_only_when_the_cause_can_pass() {
        let passing = not_summed_up("Standup", "the NPU was busy", true);
        assert_eq!(passing.heading, "Summary of Standup Couldn't Be Made");
        assert_eq!(passing.acts[0], ("again", "Try Again"));
        let lasting = not_summed_up("Standup", "its transcript can't be read", false);
        assert!(lasting.acts.iter().all(|(name, _)| *name != "again"));
    }

    #[test]
    fn a_body_opening_with_a_dash_comes_after_the_options_end() {
        let args = arguments(&summed_up("Standup", "- Thursday it is"));
        let end = args.iter().position(|arg| arg == "--").unwrap();
        assert_eq!(args[end + 2], "- Thursday it is");
        assert!(!args.contains(&"--hint=boolean:suppress-sound:true".to_owned()));
    }
}
