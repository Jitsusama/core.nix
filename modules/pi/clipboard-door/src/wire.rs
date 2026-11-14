//! What the shim and the door say to each other over the door's socket.
//!
//! The shim sends one line, the request, and for a copy the bytes to copy
//! after it, then shuts its half of the socket. The door answers one
//! line, `ok`, with the type it read for a paste, or `no` and why as a
//! word from a fixed set, then for a paste the bytes it read, or for a
//! listing the types offered, a line apiece:
//!
//! ```text
//! copy clipboard text/plain;charset=utf-8 always   ok
//! paste primary text                               ok text/plain;charset=utf-8
//! list clipboard                                   no nothing-copied
//! clear primary                                    ok
//! ```
//!
//! The door never sends text a client could have put there, so what it
//! says is only ever one of its own words or a type the compositor
//! offered.

use std::fmt;

/// The longest line either end sends, far longer than any request.
pub const LONGEST_LINE: usize = 1024;

/// The longest type a request names, longer than any in use.
pub const LONGEST_KIND: usize = 255;

/// Which of the desktop's two selections: the clipboard, or the text last
/// highlighted, which a middle click pastes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Clipboard,
    Primary,
}

impl Selection {
    /// The word for this selection, in a request and in the journal.
    pub fn word(self) -> &'static str {
        match self {
            Selection::Clipboard => "clipboard",
            Selection::Primary => "primary",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        match word {
            "clipboard" => Some(Selection::Clipboard),
            "primary" => Some(Selection::Primary),
            _ => None,
        }
    }
}

/// The type a paste asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// Text if it's offered as text, else whatever is offered first.
    Any,
    /// A type exactly, or with no `/`, any type of that family, as
    /// wl-paste's `--type` takes it: `text` for any text, `image` for any
    /// picture.
    Kind(String),
}

/// What a client asks of the door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Put the bytes that follow on a selection, offered as `kind`, or as
    /// what they look like when it's `None`; served to one paste only
    /// when `once`.
    Copy {
        selection: Selection,
        kind: Option<String>,
        once: bool,
    },
    /// What a selection holds, as the type wanted.
    Paste {
        selection: Selection,
        wanted: Wanted,
    },
    /// The types a selection is offered as.
    List { selection: Selection },
    /// Empty a selection.
    Clear { selection: Selection },
}

impl Request {
    /// The selection this asks about.
    pub fn selection(&self) -> Selection {
        match self {
            Request::Copy { selection, .. }
            | Request::Paste { selection, .. }
            | Request::List { selection }
            | Request::Clear { selection } => *selection,
        }
    }

    /// This request as its line, without the newline.
    pub fn line(&self) -> String {
        match self {
            Request::Copy {
                selection,
                kind,
                once,
            } => format!(
                "copy {} {} {}",
                selection.word(),
                kind.as_deref().unwrap_or("-"),
                if *once { "once" } else { "always" }
            ),
            Request::Paste { selection, wanted } => format!(
                "paste {} {}",
                selection.word(),
                match wanted {
                    Wanted::Any => "-",
                    Wanted::Kind(kind) => kind,
                }
            ),
            Request::List { selection } => format!("list {}", selection.word()),
            Request::Clear { selection } => format!("clear {}", selection.word()),
        }
    }

    /// The request a line says, or nothing when it says none.
    pub fn read(line: &str) -> Option<Self> {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["copy", selection, kind, once] => Some(Request::Copy {
                selection: Selection::from_word(selection)?,
                kind: match *kind {
                    "-" => None,
                    kind => Some(plain_kind(kind)?.to_owned()),
                },
                once: match *once {
                    "once" => true,
                    "always" => false,
                    _ => return None,
                },
            }),
            ["paste", selection, wanted] => Some(Request::Paste {
                selection: Selection::from_word(selection)?,
                wanted: match *wanted {
                    "-" => Wanted::Any,
                    kind => Wanted::Kind(plain_kind(kind)?.to_owned()),
                },
            }),
            ["list", selection] => Some(Request::List {
                selection: Selection::from_word(selection)?,
            }),
            ["clear", selection] => Some(Request::Clear {
                selection: Selection::from_word(selection)?,
            }),
            _ => None,
        }
    }
}

/// A type as a request may name one: printable ASCII with no space, and
/// not the word that means none.
pub fn plain_kind(kind: &str) -> Option<&str> {
    let plain = !kind.is_empty()
        && kind != "-"
        && kind.len() <= LONGEST_KIND
        && kind.bytes().all(|byte| byte.is_ascii_graphic());
    plain.then_some(kind)
}

/// Why the door refused, as the one word its answer and its journal line
/// carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The client isn't a command in a fence of this account.
    Unfenced,
    /// The request couldn't be read.
    Malformed,
    /// The selection holds nothing.
    NothingCopied,
    /// The selection isn't offered as the type asked for.
    NoSuchType,
    /// A password manager marked what the selection holds as secret.
    Secret,
    /// More was sent, or offered, than the door carries.
    TooLarge,
    /// The program holding the selection didn't hand it over in time.
    Unanswered,
    /// The door couldn't reach the compositor, or lost it.
    NoDisplay,
    /// The door is answering as many clients as it does at once.
    Busy,
}

impl Why {
    /// Every reason, for reading one back from its word.
    const ALL: [Why; 9] = [
        Why::Unfenced,
        Why::Malformed,
        Why::NothingCopied,
        Why::NoSuchType,
        Why::Secret,
        Why::TooLarge,
        Why::Unanswered,
        Why::NoDisplay,
        Why::Busy,
    ];

    /// The word this reason is sent and journalled as.
    pub fn word(self) -> &'static str {
        match self {
            Why::Unfenced => "unfenced",
            Why::Malformed => "malformed",
            Why::NothingCopied => "nothing-copied",
            Why::NoSuchType => "no-such-type",
            Why::Secret => "secret",
            Why::TooLarge => "too-large",
            Why::Unanswered => "unanswered",
            Why::NoDisplay => "no-display",
            Why::Busy => "busy",
        }
    }

    /// The reason a word names.
    pub fn from_word(word: &str) -> Option<Self> {
        Why::ALL.into_iter().find(|why| why.word() == word)
    }

    /// What wl-copy or wl-paste says for this reason, as the terminal
    /// kit says what failed, worded as wl-clipboard words the ones it has.
    pub fn sentence(self) -> &'static str {
        match self {
            Why::Unfenced => "the clipboard door answers only commands in pi's fence",
            Why::Malformed => "the clipboard door couldn't read what was asked",
            Why::NothingCopied => "nothing is copied",
            Why::NoSuchType => "no suitable type of content copied",
            Why::Secret => {
                "a password manager marked what's copied as secret, \
                 which the fence's clipboard door never reads"
            }
            Why::TooLarge => "that is more than the clipboard door carries (64 MiB)",
            Why::Unanswered => "the program holding the clipboard didn't hand it over",
            Why::NoDisplay => "the clipboard door can't reach the display",
            Why::Busy => "the clipboard door is answering too many commands; try again",
        }
    }
}

/// The door's answer: done, with the type read for a paste, or refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Ok(Option<String>),
    No(Why),
}

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Answer::Ok(None) => write!(f, "ok"),
            Answer::Ok(Some(kind)) => write!(f, "ok {kind}"),
            Answer::No(why) => write!(f, "no {}", why.word()),
        }
    }
}

impl Answer {
    /// The answer a line says, or nothing when it says none.
    pub fn read(line: &str) -> Option<Self> {
        match line.split_once(' ') {
            None if line == "ok" => Some(Answer::Ok(None)),
            Some(("ok", kind)) => Some(Answer::Ok(Some(plain_kind(kind)?.to_owned()))),
            Some(("no", word)) => Some(Answer::No(Why::from_word(word)?)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_request_reads_back_as_itself() {
        for request in [
            Request::Copy {
                selection: Selection::Primary,
                kind: Some("text/plain;charset=utf-8".into()),
                once: true,
            },
            Request::Copy {
                selection: Selection::Clipboard,
                kind: None,
                once: false,
            },
            Request::Paste {
                selection: Selection::Clipboard,
                wanted: Wanted::Any,
            },
            Request::Paste {
                selection: Selection::Primary,
                wanted: Wanted::Kind("image".into()),
            },
            Request::List {
                selection: Selection::Primary,
            },
            Request::Clear {
                selection: Selection::Clipboard,
            },
        ] {
            assert_eq!(Request::read(&request.line()), Some(request));
        }
    }

    #[test]
    fn a_line_saying_no_request_is_none() {
        for line in [
            "",
            "copy",
            "copy clipboard",
            "copy secondary - always",
            "copy clipboard - twice",
            "paste clipboard text extra",
            "paste clipboard  text",
            "list",
            "type text",
            "clear clipboard\n",
        ] {
            assert_eq!(Request::read(line), None, "{line:?}");
        }
    }

    #[test]
    fn a_type_with_a_space_or_a_control_is_refused() {
        assert_eq!(plain_kind("text/plain"), Some("text/plain"));
        for kind in ["", "-", "a b", "a\tb", "a\u{7f}", "é", &"a".repeat(256)] {
            assert_eq!(plain_kind(kind), None, "{kind:?}");
        }
    }

    #[test]
    fn every_answer_reads_back_as_itself() {
        let mut answers = vec![Answer::Ok(None), Answer::Ok(Some("image/png".into()))];
        answers.extend(Why::ALL.into_iter().map(Answer::No));
        for answer in answers {
            assert_eq!(Answer::read(&answer.to_string()), Some(answer));
        }
        assert_eq!(Answer::read("no such"), None);
        assert_eq!(Answer::read("okay"), None);
    }
}
