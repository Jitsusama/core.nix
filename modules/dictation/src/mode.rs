//! How what's said is typed, by the window it lands in. A shell takes a
//! command, which Whisper writes as a sentence ("Git status."), so there
//! it loses the capital and the full stop; everything else, pi's prompt
//! included, takes the words as Whisper writes them. No model is asked,
//! so nothing waits: Joel asked for no delayed voice. The window is
//! niri's focused one as the words are typed, asked over niri's socket in
//! about 2 ms, so it's where the words land, not where the chord went down.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// What the focused window takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Sentences, as Whisper writes them.
    Prose,
    /// A command line: no capital to start, no full stop to end.
    Shell,
}

/// The terminals a shell runs in, by the app id niri knows them by.
const TERMINALS: &[&str] = &[
    "kitty",
    "foot",
    "Alacritty",
    "com.mitchellh.ghostty",
    "org.wezfurlong.wezterm",
];

/// The mode for a window with this app id and title. A pi session runs
/// in a terminal too, but its prompt takes sentences: its app id says so
/// when the harness opened it, and its title, π, when not.
pub fn mode(app_id: &str, title: &str) -> Mode {
    let pi = app_id.starts_with("pi-session.")
        || app_id.starts_with("pi-spawned.")
        || title.starts_with('π');
    if !pi && TERMINALS.contains(&app_id) {
        Mode::Shell
    } else {
        Mode::Prose
    }
}

/// `text` as `mode` takes it.
pub fn shaped(text: &str, mode: Mode) -> String {
    let text = text.trim();
    match mode {
        Mode::Prose => text.to_owned(),
        Mode::Shell => {
            let text = text.strip_suffix(['.', '!', '?']).unwrap_or(text);
            let mut chars = text.chars();
            match (chars.next(), chars.clone().next()) {
                // A capital before a lower-case letter is a sentence's;
                // one before another capital is a name's, such as NIX_PATH.
                (Some(first), Some(second)) if first.is_uppercase() && second.is_lowercase() => {
                    first.to_lowercase().chain(chars).collect()
                }
                _ => text.to_owned(),
            }
        }
    }
}

/// The focused window's mode, or prose when niri can't be asked, since
/// words typed as heard are never worse than they were before modes.
pub fn focused() -> Mode {
    ask_niri().map_or(Mode::Prose, |reply| from_reply(&reply))
}

/// The mode niri's answer to `"FocusedWindow"` names, prose for no window.
fn from_reply(reply: &str) -> Mode {
    let reply: serde_json::Value = serde_json::from_str(reply).unwrap_or_default();
    let window = &reply["Ok"]["FocusedWindow"];
    mode(
        window["app_id"].as_str().unwrap_or_default(),
        window["title"].as_str().unwrap_or_default(),
    )
}

fn ask_niri() -> Option<String> {
    let stream = UnixStream::connect(socket()?).ok()?;
    let wait = Some(Duration::from_millis(200));
    stream.set_read_timeout(wait).ok()?;
    stream.set_write_timeout(wait).ok()?;
    (&stream).write_all(b"\"FocusedWindow\"\n").ok()?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply).ok()?;
    Some(reply)
}

/// niri's socket: the one the user manager was told of, or, since the
/// service can start from its own socket before niri says, the one niri
/// keeps in the runtime folder.
fn socket() -> Option<PathBuf> {
    if let Some(socket) = std::env::var_os("NIRI_SOCKET") {
        return Some(socket.into());
    }
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    std::fs::read_dir(&runtime)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let niris = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("niri."));
            niris
                && path
                    .extension()
                    .is_some_and(|extension| extension == "sock")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_takes_a_command_not_a_sentence() {
        assert_eq!(shaped(" Git status.", Mode::Shell), "git status");
        assert_eq!(shaped("Ls dash la?", Mode::Shell), "ls dash la");
    }

    #[test]
    fn a_shell_keeps_a_name_in_capitals() {
        assert_eq!(shaped("NIX_PATH is set.", Mode::Shell), "NIX_PATH is set");
        assert_eq!(shaped("I", Mode::Shell), "I");
    }

    #[test]
    fn prose_is_typed_as_heard() {
        assert_eq!(
            shaped(" Hello, this is a test.", Mode::Prose),
            "Hello, this is a test."
        );
    }

    #[test]
    fn pi_takes_sentences_though_it_runs_in_a_terminal() {
        assert_eq!(mode("pi-session.1794413231989", "π"), Mode::Prose);
        assert_eq!(mode("pi-spawned.1794413365886", "π core.nix"), Mode::Prose);
        assert_eq!(mode("kitty", "π"), Mode::Prose);
    }

    #[test]
    fn a_terminal_takes_commands_and_anything_else_prose() {
        assert_eq!(mode("kitty", "~/src/core.nix"), Mode::Shell);
        assert_eq!(mode("foot", "zsh"), Mode::Shell);
        assert_eq!(mode("google-chrome", "Inbox"), Mode::Prose);
        assert_eq!(mode("", ""), Mode::Prose);
    }

    #[test]
    fn niris_answer_names_the_window() {
        let kitty = r#"{"Ok":{"FocusedWindow":{"id":7,"title":"zsh","app_id":"kitty"}}}"#;
        assert_eq!(from_reply(kitty), Mode::Shell);
        assert_eq!(from_reply(r#"{"Ok":{"FocusedWindow":null}}"#), Mode::Prose);
        assert_eq!(from_reply("not json"), Mode::Prose);
    }
}
