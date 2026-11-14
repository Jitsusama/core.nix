//! The keys 1Password's SSH agent holds, read from its socket only when
//! Joel asks, through ssh-add and ssh-keygen. ssh and git sign with the
//! TPM's agent, never this one, so nothing else here reaches it. Listing
//! keys reads only their public halves; nothing is signed and no key is
//! added or taken away. As text for a person, or as JSON,
//! `passwords.keys/1`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use terminal_kit::layout::{self, Align, ITEMS};
use terminal_kit::{Context, Exit, Failure, Pen, Room};

use super::written;

/// ssh-add and ssh-keygen as the package names them, else as the PATH has
/// them.
const SSH_ADD: &str = match option_env!("PASSWORDS_SSH_ADD") {
    Some(program) => program,
    None => "ssh-add",
};
const SSH_KEYGEN: &str = match option_env!("PASSWORDS_SSH_KEYGEN") {
    Some(program) => program,
    None => "ssh-keygen",
};

/// What a cell with nothing to say shows, so a column never reads as
/// shifted along.
const NOTHING: &str = "\u{2013}";

/// The columns of the agent's keys, the public half the first to give
/// way, since the fingerprint names a key in fewer cells.
const KEYS: [(&str, Align); 5] = [
    ("type", Align::Left),
    ("bits", Align::Right),
    ("fingerprint", Align::Left),
    ("comment", Align::Left),
    ("public", Align::Left),
];
const KEYS_GIVE_WAY: [usize; 3] = [4, 3, 1];

/// The agent's keys, as Joel asked.
pub fn run(context: &Context) -> Result<(), Failure> {
    let home = std::env::var_os("HOME").unwrap_or_default();
    let socket = PathBuf::from(home).join(".1password/agent.sock");
    let unanswered = |why: String| {
        Failure::new(Exit::Unavailable, "1Password's SSH agent isn't answering")
            .because(why)
            .try_this("1Password's Settings, under Developer, turns it on")
    };
    if !socket.exists() {
        return Err(unanswered(format!(
            "nothing listens at {}",
            socket.display()
        )));
    }
    let listed = Command::new(SSH_ADD)
        .arg("-L")
        .env("SSH_AUTH_SOCK", &socket)
        .stdin(Stdio::null())
        .output()
        .map_err(|why| {
            Failure::new(Exit::Failed, "couldn't run ssh-add").because(why.to_string())
        })?;
    let public = String::from_utf8_lossy(&listed.stdout).into_owned();
    let refused = String::from_utf8_lossy(&listed.stderr).trim().to_owned();
    let keys = match listed.status.code() {
        Some(0) => keys(&public, &fingerprints(&public)?),
        // ssh-add says on stdout that an agent holds no key, and on stderr
        // why it couldn't list them.
        Some(1) if refused.is_empty() => Vec::new(),
        Some(1) => {
            return Err(
                Failure::new(Exit::Failed, "1Password's SSH agent didn't list its keys")
                    .because(refused),
            );
        }
        _ => return Err(unanswered(refused)),
    };
    if context.format().is_json() {
        return written(context.answer(&json!({"schema": "passwords.keys/1", "keys": keys})));
    }
    written(context.say(&keys_text(&keys, &context.out, context.room)))
}

/// What `ssh-keygen -l` says of each public key, a line each, in order.
fn fingerprints(public: &str) -> Result<String, Failure> {
    let failed = |why: String| {
        Failure::new(Exit::Failed, "couldn't read the keys' fingerprints").because(why)
    };
    let mut child = Command::new(SSH_KEYGEN)
        .args(["-l", "-E", "sha256", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| failed(why.to_string()))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(public.as_bytes())
            .map_err(|why| failed(why.to_string()))?;
    }
    let said = child
        .wait_with_output()
        .map_err(|why| failed(why.to_string()))?;
    if !said.status.success() {
        return Err(failed(
            String::from_utf8_lossy(&said.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&said.stdout).into_owned())
}

/// The keys an SSH agent holds, from `ssh-add -L` (each key's public half
/// and comment, a line each) and `ssh-keygen -l` over those lines (each
/// one's size, fingerprint and type, in the same order): each as
/// `{"type", "bits", "fingerprint", "comment", "public"}`.
pub fn keys(public: &str, fingerprints: &str) -> Vec<Value> {
    let public = public.lines().filter(|line| {
        let mut words = line.split_whitespace();
        words.next().is_some() && words.next().is_some()
    });
    public
        .zip(fingerprints.lines())
        .map(|(line, fingerprinted)| {
            let mut words = line.split_whitespace();
            let algorithm = words.next().unwrap_or("");
            let blob = words.next().unwrap_or("");
            let comment = words.collect::<Vec<_>>().join(" ");
            let (bits, rest) = fingerprinted.split_once(' ').unwrap_or(("", ""));
            let (fingerprint, rest) = rest.split_once(' ').unwrap_or((rest, ""));
            let kind = rest
                .rsplit_once('(')
                .map_or("", |(_, kind)| kind.trim_end_matches(')'));
            json!({
                "type": kind,
                "bits": bits.parse::<u64>().ok(),
                "fingerprint": fingerprint,
                "comment": comment,
                "public": format!("{algorithm} {blob}"),
            })
        })
        .collect()
}

/// A table's cell: its own tabs and line breaks spaces, so they can't
/// split a row, and a dash where it has nothing to say.
fn cell(words: &str) -> String {
    if words.is_empty() {
        return NOTHING.to_owned();
    }
    words.replace(['\t', '\n', '\r'], " ")
}

/// The agent's keys for a person: a head counting them, then a row each
/// of its type, size, fingerprint, comment and public half, fitted to the
/// room and drawn whole into a pipe.
pub fn keys_text(keys: &[Value], pen: &Pen, room: Room) -> String {
    let text = |key: &Value, name: &str| cell(key[name].as_str().unwrap_or(""));
    let rows: Vec<Vec<String>> = keys
        .iter()
        .map(|key| {
            vec![
                text(key, "type"),
                cell(
                    &key["bits"]
                        .as_u64()
                        .map(|bits| bits.to_string())
                        .unwrap_or_default(),
                ),
                text(key, "fingerprint"),
                text(key, "comment"),
                text(key, "public"),
            ]
        })
        .collect();
    let counted = if keys.len() == 1 { "key" } else { "keys" };
    let head = format!(
        "{}{}",
        pen.title("1Password's SSH agent"),
        pen.role("muted", &format!("{ITEMS}{} {counted}", keys.len()))
    );
    let mut blocks = vec![vec![head]];
    if !rows.is_empty() {
        blocks.push(layout::table_in(&KEYS, &rows, pen, room, &KEYS_GIVE_WAY));
    }
    layout::joined(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two() -> Vec<Value> {
        let public = "ssh-ed25519 AAAAC3Nza signing key for GitHub\n\
                      ecdsa-sha2-nistp256 AAAAE2Vj\n";
        let fingerprints = "256 SHA256:abc signing key for GitHub (ED25519)\n\
                            256 SHA256:def no comment (ECDSA)\n";
        keys(public, fingerprints)
    }

    #[test]
    fn an_agents_keys_are_read_from_ssh_add_and_ssh_keygen() {
        assert_eq!(
            two(),
            [
                json!({"type": "ED25519", "bits": 256, "fingerprint": "SHA256:abc",
                       "comment": "signing key for GitHub", "public": "ssh-ed25519 AAAAC3Nza"}),
                json!({"type": "ECDSA", "bits": 256, "fingerprint": "SHA256:def",
                       "comment": "", "public": "ecdsa-sha2-nistp256 AAAAE2Vj"}),
            ]
        );
    }

    #[test]
    fn the_keys_are_a_table_under_a_head_counting_them() {
        assert_eq!(
            keys_text(&two(), &Pen::plain(), Room::Unbounded),
            "1Password's SSH agent \u{b7} 2 keys\n\
             \n\
             TYPE     BITS  FINGERPRINT  COMMENT                 PUBLIC\n\
             ED25519   256  SHA256:abc   signing key for GitHub  ssh-ed25519 AAAAC3Nza\n\
             ECDSA     256  SHA256:def   \u{2013}                       ecdsa-sha2-nistp256 AAAAE2Vj\n"
        );
        assert_eq!(
            keys_text(&[], &Pen::plain(), Room::Unbounded),
            "1Password's SSH agent \u{b7} 0 keys\n"
        );
    }

    #[test]
    fn a_narrow_terminal_lets_the_public_half_go_first() {
        let said = keys_text(&two(), &Pen::plain(), Room::Cells(60));
        assert!(
            said.lines().all(|line| line.chars().count() <= 60),
            "a line passes 60 columns:\n{said}"
        );
        assert!(
            said.contains("SHA256:abc") && !said.contains("AAAAC3Nza"),
            "the public half didn't give way before the fingerprint:\n{said}"
        );
    }
}
