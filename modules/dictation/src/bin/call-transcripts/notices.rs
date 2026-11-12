//! The notices call-transcripts raises, every one from Calls, in
//! VKCF3Z's words: a call being transcribed, its line for the call's chat
//! offered to copy; its summary ready, or its transcript alone when
//! there's no summarizer; and what couldn't be done, and why.
//!
//! Each answers a call Joel is on or just had, so none is ever counted
//! missed (the shell's `x-answers`). Each waits on a thread of its own for
//! the act Joel chooses, which it's given by its name. One that says what
//! has passed is withdrawn once it has (`Shown`), as a call's being
//! transcribed once the call ends.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};

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

/// The notice once a call is written down with no summarizer: its
/// transcript alone, and why there's no summary.
pub fn written_down(title: &str) -> Notice {
    Notice {
        heading: format!("{} Is Written Down", named(title, true)),
        body: "Its transcript is in its folder. It isn't summed up, since the \
               summarizer can't yet sum a call up truly."
            .to_owned(),
        acts: vec![
            ("transcript", "Open the Transcript"),
            ("forget", "Forget"),
            ("default", "Open the Transcript"),
        ],
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
        // Its number, said as it's shown, so it can be withdrawn.
        "--print-id".to_owned(),
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

/// Where a notice raised stands, for withdrawing it: its number comes
/// only once the server has it, and one closed by Joel or its server is
/// gone already.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Showing {
    #[default]
    Coming,
    Up(u32),
    /// Withdrawn before its number came, so it's closed once it does.
    Withdrawn,
    Gone,
}

/// A notice raised, which can be withdrawn once what it says has passed,
/// so it never stays saying so: on 2026-11-11 a call's "Is Being
/// Transcribed" stayed on Joel's desktop after the call ended and its
/// summary came.
#[derive(Clone, Debug, Default)]
pub struct Shown(Arc<Mutex<Showing>>);

impl Shown {
    fn moved(&self, to: impl FnOnce(Showing) -> (Showing, Option<u32>)) -> Option<u32> {
        let mut showing = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let (now, closing) = to(*showing);
        *showing = now;
        closing
    }

    /// Its number has come: the number to close at once when it was
    /// withdrawn meanwhile.
    fn up(&self, id: u32) -> Option<u32> {
        self.moved(|was| match was {
            Showing::Coming => (Showing::Up(id), None),
            Showing::Withdrawn => (Showing::Gone, Some(id)),
            other => (other, None),
        })
    }

    /// Closed, by Joel or its server: nothing is left to withdraw.
    fn gone(&self) {
        self.moved(|_| (Showing::Gone, None));
    }

    /// The number to close to withdraw it, none while it's yet to come,
    /// when it's closed once it does, or once it's gone.
    fn withdrawing(&self) -> Option<u32> {
        self.moved(|was| match was {
            Showing::Coming => (Showing::Withdrawn, None),
            Showing::Up(id) => (Showing::Gone, Some(id)),
            other => (other, None),
        })
    }

    /// Withdraws it, as what it says has passed.
    pub fn withdraw(&self) {
        if let Some(id) = self.withdrawing() {
            close(id);
        }
    }
}

/// busctl's arguments to close notice `id`, as the specification's
/// `CloseNotification` withdraws one its sender raised.
fn closing(id: u32) -> Vec<String> {
    [
        "--user",
        "call",
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
        "CloseNotification",
        "u",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([id.to_string()])
    .collect()
}

fn close(id: u32) {
    match Command::new("busctl").args(closing(id)).output() {
        Ok(done) if done.status.success() => {}
        Ok(done) => eprintln!(
            "call-transcripts: notice {id} wasn't withdrawn: {}",
            String::from_utf8_lossy(&done.stderr).trim()
        ),
        Err(why) => eprintln!("call-transcripts: notice {id} wasn't withdrawn: {why}"),
    }
}

/// Raises a notice, and gives `then` the act Joel chose, on a thread of
/// its own; a notice let go without one gives nothing. notify-send says
/// the notice's number first, at once, then the act chosen once it's
/// closed.
pub fn show(notice: &Notice, then: impl FnOnce(&str) + Send + 'static) -> Shown {
    let args = arguments(notice);
    let shown = Shown::default();
    let showing = shown.clone();
    std::thread::spawn(move || {
        let raised = Command::new("notify-send")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match raised {
            Ok(child) => child,
            Err(why) => {
                eprintln!("call-transcripts: a notice wasn't shown: {why}");
                return;
            }
        };
        let mut chosen = String::new();
        if let Some(out) = child.stdout.take() {
            let mut lines = BufReader::new(out).lines().map_while(Result::ok);
            if let Some(id) = lines.next().and_then(|line| line.trim().parse().ok())
                && let Some(closing) = showing.up(id)
            {
                close(closing);
            }
            chosen = lines
                .find(|line| !line.trim().is_empty())
                .unwrap_or_default();
        }
        showing.gone();
        match child.wait_with_output() {
            Ok(done) if done.status.success() => {
                if !chosen.trim().is_empty() {
                    then(chosen.trim());
                }
            }
            Ok(done) => eprintln!(
                "call-transcripts: a notice wasn't shown: {}",
                String::from_utf8_lossy(&done.stderr).trim()
            ),
            Err(why) => eprintln!("call-transcripts: a notice wasn't shown: {why}"),
        }
    });
    shown
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

/// systemd-run's arguments to open a call's file as any file is opened,
/// in a unit of its own among the session's apps. The service started
/// before the session said where its screen is, so a program it starts
/// itself finds none, and xdg-open falls through to text browsers; a
/// unit started now has the session's environment as it is. It's given
/// the service's PATH, which holds xdg-open and the profile's programs,
/// and it outlives the service's restart at a switch.
fn opening(file: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = [
        "--user",
        "--collect",
        "--quiet",
        "--slice=app.slice",
        "--setenv=PATH",
        "--",
        "xdg-open",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.push(file.as_os_str().to_owned());
    args
}

/// Opens a call's file as any file is opened, in the session's reader.
pub fn open(file: &std::path::Path) {
    match Command::new("systemd-run").args(opening(file)).status() {
        Ok(done) if done.success() => {}
        Ok(done) => eprintln!("call-transcripts: {} wasn't opened: {done}", file.display()),
        Err(why) => eprintln!("call-transcripts: {} wasn't opened: {why}", file.display()),
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
    fn a_notice_shown_is_withdrawn_by_its_number_once() {
        let shown = Shown::default();
        assert_eq!(shown.up(7), None, "nothing closes as it's shown");
        assert_eq!(shown.withdrawing(), Some(7), "withdrawn, it's closed");
        assert_eq!(shown.withdrawing(), None, "and only once");
    }

    #[test]
    fn a_notice_withdrawn_before_its_number_comes_closes_once_it_does() {
        let shown = Shown::default();
        assert_eq!(shown.withdrawing(), None, "no number to close yet");
        assert_eq!(shown.up(7), Some(7), "closed as its number comes");
    }

    #[test]
    fn a_notice_joel_closed_isnt_withdrawn() {
        let shown = Shown::default();
        shown.up(7);
        shown.gone();
        assert_eq!(shown.withdrawing(), None, "nothing is left to close");
    }

    #[test]
    fn a_notice_is_raised_saying_its_number_and_withdrawn_by_it() {
        assert!(
            arguments(&being_transcribed("")).contains(&"--print-id".to_owned()),
            "notify-send says the number it's withdrawn by"
        );
        assert_eq!(
            closing(7)[5..],
            ["CloseNotification", "u", "7"],
            "closed as its sender withdraws it"
        );
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
    fn a_call_written_down_alone_opens_its_transcript() {
        let notice = written_down("Standup");
        assert_eq!(notice.heading, "Standup Is Written Down");
        assert!(notice.body.contains("isn't summed up"));
        let names: Vec<&str> = notice.acts.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["transcript", "forget", "default"]);
    }

    #[test]
    fn a_file_opens_in_a_unit_of_its_own_with_the_services_path() {
        let args = opening(std::path::Path::new("/calls/a b/summary.md"));
        let end = args.iter().position(|arg| arg == "--").unwrap();
        assert!(args[..end].contains(&"--user".into()));
        assert!(args[..end].contains(&"--setenv=PATH".into()));
        assert_eq!(args[end + 1..], ["xdg-open", "/calls/a b/summary.md"]);
    }

    #[test]
    fn a_body_opening_with_a_dash_comes_after_the_options_end() {
        let args = arguments(&summed_up("Standup", "- Thursday it is"));
        let end = args.iter().position(|arg| arg == "--").unwrap();
        assert_eq!(args[end + 2], "- Thursday it is");
        assert!(!args.contains(&"--hint=boolean:suppress-sound:true".to_owned()));
    }
}
