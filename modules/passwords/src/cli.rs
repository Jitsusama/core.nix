//! The `passwords` command line's own logic, apart from the programs it
//! runs, so its tests reach it: what's typed ([`parse`]), and the keys
//! 1Password's SSH agent holds, read from what ssh-add and ssh-keygen say
//! of them ([`keys`]) and set for a person or a program ([`keys_text`]).
//!
//! It reaches no daemon, and so no entry and no secret: the daemon answers
//! the shell alone, which runs it as its own child, since every agent runs
//! as Joel and could ask whatever the command could (2026-11-10). A
//! password is filled by the manager's own browser extension, or read in
//! its own app or its own command, each of which asks Joel itself.
//!
//! A person, at a terminal, is shown columns under a header, lined up, a
//! dash where a key has nothing to say, and a column nothing fills left
//! out. A program, reading a pipe, is given the same rows tab-separated
//! without a header, every column always there, so `cut -f` finds each by
//! its place. `--json` gives each key as it was read.

use std::fmt::Write;

use serde_json::{Value, json};

/// What the command says when it's mistyped, and `passwords help`.
pub const USAGE: &str = "\
usage: passwords <command> [--json]

  keys                 the keys 1Password's SSH agent holds, which ssh and git don't use
  help                 this

Passwords are in the launcher's Passwords place, which runs the passwords daemon itself;
no command reaches it, so none can read an entry or a secret.

A terminal is shown columns under a header, and a program the same columns
tab-separated without one. --json gives each key as it was read.

Exit status:
  0   done
  1   1Password's SSH agent didn't list its keys
  64  the command was mistyped
  69  1Password's SSH agent isn't answering
";

/// 1Password's SSH agent didn't list its keys.
pub const FAILED: i32 = 1;
/// The command was mistyped (sysexits' `EX_USAGE`).
pub const MISTYPED: i32 = 64;
/// Nothing answers where it should (`EX_UNAVAILABLE`).
pub const UNANSWERED: i32 = 69;

/// One thing the command was asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// The daemon, which the shell runs as its own child.
    Daemon {
        managers: String,
        /// The launcher's vocabulary, which its rows are said in.
        words: Option<String>,
    },
    Keys,
    Help,
    /// zsh's completion itself.
    Completion,
}

/// What was typed: the command, and whether it asked for JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typed {
    pub command: Command,
    pub json: bool,
}

/// What the words after `passwords` ask, or what's wrong with them.
/// `--json` may come anywhere.
pub fn parse(args: &[String]) -> Result<Typed, String> {
    let mut json = false;
    let mut words: Vec<&str> = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" => words.insert(0, "help"),
            word => words.push(word),
        }
    }
    let Some((first, rest)) = words.split_first() else {
        return Err("name a command".to_owned());
    };
    let none = |command: Command| {
        if rest.is_empty() {
            Ok(command)
        } else {
            Err(format!("{first} takes nothing after it"))
        }
    };
    let command = match *first {
        "daemon" => match rest {
            [managers, words @ ..] if words.len() <= 1 => Command::Daemon {
                managers: (*managers).to_owned(),
                words: words.first().map(|words| (*words).to_owned()),
            },
            _ => {
                return Err(
                    "daemon needs a MANAGERS file, then the launcher's WORDS; the shell runs it"
                        .to_owned(),
                );
            }
        },
        "keys" => none(Command::Keys)?,
        "help" => Command::Help,
        "completion" => match rest {
            ["zsh"] => Command::Completion,
            _ => return Err("completion is written for zsh alone".to_owned()),
        },
        other => return Err(format!("{other:?} isn't a command")),
    };
    Ok(Typed { command, json })
}

/// What `--json` gives: the value as it was read.
pub fn as_json(answer: &Value) -> String {
    serde_json::to_string_pretty(answer).unwrap_or_default() + "\n"
}

/// A column: its header, and whether its figures line up on the right.
#[derive(Clone, Copy)]
pub struct Column {
    pub header: &'static str,
    pub right: bool,
}

const fn left(header: &'static str) -> Column {
    Column {
        header,
        right: false,
    }
}

const fn right(header: &'static str) -> Column {
    Column {
        header,
        right: true,
    }
}

/// Rows set for a person, under a header, each column as wide as its
/// widest cell, two spaces between, a dash where a cell is empty, and a
/// column no row fills left out; or for a program, tab-separated with no
/// header and every column kept. A cell's own tabs and line breaks become
/// spaces so they can't split a row.
pub fn table(columns: &[Column], rows: &[Vec<String>], terminal: bool) -> String {
    let flat = |cell: &str| cell.replace(['\t', '\n', '\r'], " ");
    if !terminal {
        return rows
            .iter()
            .map(|row| {
                let cells: Vec<String> = row.iter().map(|cell| flat(cell)).collect();
                cells.join("\t") + "\n"
            })
            .collect();
    }
    if rows.is_empty() {
        return String::new();
    }
    let kept: Vec<usize> = (0..columns.len())
        .filter(|&at| {
            rows.iter()
                .any(|row| row.get(at).is_some_and(|cell| !cell.is_empty()))
        })
        .collect();
    let shown = |row: &[String], at: usize| {
        let cell = row.get(at).map_or("", String::as_str);
        if cell.is_empty() {
            "–".to_owned()
        } else {
            flat(cell)
        }
    };
    let widths: Vec<usize> = kept
        .iter()
        .map(|&at| {
            rows.iter()
                .map(|row| shown(row, at).chars().count())
                .chain([columns[at].header.chars().count()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: Vec<String>| {
        let set: Vec<String> = cells
            .iter()
            .zip(&kept)
            .zip(&widths)
            .map(|((cell, &at), &width)| {
                if columns[at].right {
                    format!("{cell:>width$}")
                } else {
                    format!("{cell:width$}")
                }
            })
            .collect();
        set.join("  ").trim_end().to_owned() + "\n"
    };
    let mut out = line(
        kept.iter()
            .map(|&at| columns[at].header.to_owned())
            .collect(),
    );
    for row in rows {
        out += &line(kept.iter().map(|&at| shown(row, at)).collect());
    }
    out
}

fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    value[name].as_str().unwrap_or("")
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

/// The agent's keys as the command shows them: for a person, a head
/// counting them, then a row each of its type, size, fingerprint and
/// comment; for a program, those rows with each key's public half last.
pub fn keys_text(keys: &[Value], terminal: bool) -> String {
    const COLUMNS: [Column; 5] = [
        left("type"),
        right("bits"),
        left("fingerprint"),
        left("comment"),
        left("public"),
    ];
    let rows: Vec<Vec<String>> = keys
        .iter()
        .map(|key| {
            vec![
                text(key, "type").to_owned(),
                key["bits"]
                    .as_u64()
                    .map(|bits| bits.to_string())
                    .unwrap_or_default(),
                text(key, "fingerprint").to_owned(),
                text(key, "comment").to_owned(),
                if terminal { "" } else { text(key, "public") }.to_owned(),
            ]
        })
        .collect();
    if !terminal {
        return table(&COLUMNS, &rows, false);
    }
    let mut out = String::new();
    let counted = if keys.len() == 1 { "key" } else { "keys" };
    let _ = writeln!(out, "1Password's SSH agent · {} {counted}", keys.len());
    if !rows.is_empty() {
        out += "\n";
        out += &table(&COLUMNS, &rows, true);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(words: &str) -> Result<Typed, String> {
        let args: Vec<String> = words.split_whitespace().map(str::to_owned).collect();
        parse(&args)
    }

    fn command(words: &str) -> Command {
        typed(words).unwrap().command
    }

    #[test]
    fn each_command_is_read_with_what_it_takes() {
        assert_eq!(command("keys"), Command::Keys);
        assert_eq!(command("help"), Command::Help);
        assert_eq!(command("--help"), Command::Help);
        assert_eq!(command("completion zsh"), Command::Completion);
        assert_eq!(
            command("daemon managers.json"),
            Command::Daemon {
                managers: "managers.json".to_owned(),
                words: None
            }
        );
        assert_eq!(
            command("daemon managers.json words.json"),
            Command::Daemon {
                managers: "managers.json".to_owned(),
                words: Some("words.json".to_owned())
            }
        );
        assert!(typed("keys --json").unwrap().json);
    }

    #[test]
    fn no_command_reaches_an_entry_or_a_secret() {
        for words in [
            "find git",
            "list",
            "details GitHub",
            "show GitHub",
            "copy GitHub",
            "code GitHub",
            "let-in 1password",
            "health",
            "audit",
            "names entries",
        ] {
            let error = typed(words).unwrap_err();
            assert!(
                error.contains("isn't a command"),
                "{words:?} said {error:?}"
            );
        }
    }

    #[test]
    fn a_mistyped_command_says_what_it_needs() {
        for (words, said) in [
            ("", "name a command"),
            ("dance", "\"dance\" isn't a command"),
            ("keys everything", "keys takes nothing after it"),
            ("completion bash", "zsh alone"),
            ("daemon", "daemon needs"),
            ("daemon a b c", "daemon needs"),
        ] {
            let error = typed(words).unwrap_err();
            assert!(error.contains(said), "{words:?} said {error:?}");
        }
    }

    #[test]
    fn the_usage_says_every_command_and_exit_status() {
        for word in ["keys", "help", "--json", "Passwords place"] {
            assert!(USAGE.contains(word), "the usage leaves out {word}");
        }
        for status in [0, FAILED, MISTYPED, UNANSWERED] {
            assert!(
                USAGE.contains(&format!("\n  {status:<2}  ")),
                "the usage leaves out {status}"
            );
        }
        assert!(USAGE.lines().all(|line| line.chars().count() < 100));
    }

    #[test]
    fn a_column_no_row_fills_is_left_out_for_a_person_alone() {
        let rows = vec![vec!["a".to_owned(), String::new(), "1".to_owned()]];
        let columns = [left("name"), left("empty"), right("count")];
        assert_eq!(table(&columns, &rows, true), "name  count\na         1\n");
        assert_eq!(table(&columns, &rows, false), "a\t\t1\n");
    }

    #[test]
    fn an_agents_keys_are_read_from_ssh_add_and_ssh_keygen() {
        let public = "ssh-ed25519 AAAAC3Nza signing key for GitHub\n\
                      ecdsa-sha2-nistp256 AAAAE2Vj\n";
        let fingerprints = "256 SHA256:abc signing key for GitHub (ED25519)\n\
                            256 SHA256:def no comment (ECDSA)\n";
        let keys = keys(public, fingerprints);
        assert_eq!(
            keys,
            [
                json!({"type": "ED25519", "bits": 256, "fingerprint": "SHA256:abc",
                       "comment": "signing key for GitHub", "public": "ssh-ed25519 AAAAC3Nza"}),
                json!({"type": "ECDSA", "bits": 256, "fingerprint": "SHA256:def",
                       "comment": "", "public": "ecdsa-sha2-nistp256 AAAAE2Vj"}),
            ]
        );
        assert_eq!(
            keys_text(&keys, true),
            "1Password's SSH agent · 2 keys\n\
             \n\
             type     bits  fingerprint  comment\n\
             ED25519   256  SHA256:abc   signing key for GitHub\n\
             ECDSA     256  SHA256:def   –\n"
        );
        assert_eq!(
            keys_text(&keys, false).lines().next(),
            Some("ED25519\t256\tSHA256:abc\tsigning key for GitHub\tssh-ed25519 AAAAC3Nza")
        );
        assert_eq!(keys_text(&[], true), "1Password's SSH agent · 0 keys\n");
    }
}
