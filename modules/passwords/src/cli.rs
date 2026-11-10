//! The `passwords` command line's own logic, apart from its socket and the
//! programs it runs, so its tests reach it: what's typed ([`parse`]), each
//! answer set as text for a person or for a program, what zsh is offered
//! ([`entry_names`] and the rest), and the keys 1Password's SSH agent
//! holds, read from what ssh-add and ssh-keygen say of them ([`keys`]).
//!
//! A person, at a terminal, is shown columns under a header, lined up, a
//! dash where an entry has nothing to say, and a column nothing fills left
//! out. A program, reading a pipe, is given the same rows tab-separated
//! without a header, every column always there, so `cut -f` finds each by
//! its place. `--json` gives what the daemon answered as it is, which holds
//! no secret but the one `show` asked for.

use std::fmt::Write;

use serde_json::{Value, json};

use crate::named;

/// What the command says when it's mistyped, and `passwords help`.
pub const USAGE: &str = "\
usage: passwords <command> [--json]

The entries every password manager keeps, as the passwords daemon holds them:
  find WORDS           the entries the words find, those titled so first
  list                 every entry
  details ENTRY        an entry's details, with no hidden detail's value
  show ENTRY [DETAIL]  a detail's value, its secret included: the password, unless named
  copy ENTRY [DETAIL]  the same, onto the clipboard, marked sensitive and cleared after 90 s
  code ENTRY           its one-time code now, and the seconds left of its period
  let-in MANAGER       ask a password manager to let passwords in, as 1Password asks you
  health               how each password manager stands, and how fast the daemon answers
  audit                every password checked for reuse and breaches; each with a problem
  keys                 the keys 1Password's SSH agent holds, which ssh and git don't use
  help                 this

ENTRY is an entry's key, its title, or words that find it alone; DETAIL is a
detail's label or its id. A terminal is shown columns under a header, and a
program the same columns tab-separated without one. --json gives what the
daemon answers as it is, which holds no secret but the one show asks for.

audit reads every password from its manager, so 1Password may ask you first;
each is hashed on this machine, and only the first five characters of its
hash go to Have I Been Pwned, whose answer is matched here.

Exit status:
  0   done
  1   nothing was found, more than one was, or the password manager refused
  64  the command was mistyped
  69  the passwords daemon, or 1Password's SSH agent, isn't answering
  75  the password manager isn't ready yet; try again
  77  the password manager is signed out; passwords let-in MANAGER lets it in
";

/// Nothing was found, more than one was, or the manager refused.
pub const FAILED: i32 = 1;
/// The command was mistyped (sysexits' `EX_USAGE`).
pub const MISTYPED: i32 = 64;
/// Nothing answers where it should (`EX_UNAVAILABLE`).
pub const UNANSWERED: i32 = 69;
/// The manager isn't ready yet (`EX_TEMPFAIL`).
pub const NOT_READY: i32 = 75;
/// The manager is signed out (`EX_NOPERM`).
pub const SIGNED_OUT: i32 = 77;

/// What a search finds an entry by when the command resolves its words: as
/// many as are held, so the one titled so is never left past a limit.
pub const EVERY: usize = 100_000;

/// One thing the command was asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Daemon {
        socket: String,
        managers: String,
        /// The launcher's vocabulary, which its rows are said in.
        words: Option<String>,
    },
    Find(String),
    List,
    Details(String),
    Show {
        entry: String,
        detail: Option<String>,
    },
    Copy {
        entry: String,
        detail: Option<String>,
    },
    Code(String),
    LetIn(String),
    Health,
    Audit,
    Keys,
    Help,
    /// What zsh's completion offers of a kind.
    Names(Names),
    /// zsh's completion itself.
    Completion,
}

/// The kinds zsh's completion asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Names {
    Entries,
    Details(String),
    Managers,
}

/// What was typed: the command, and whether it asked for JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typed {
    pub command: Command,
    pub json: bool,
}

/// What the words after `passwords` ask, or what's wrong with them.
/// `--json` may come anywhere before a `--`, after which every word is
/// taken as it is, for a title that starts with a dash.
pub fn parse(args: &[String]) -> Result<Typed, String> {
    let mut json = false;
    let mut words: Vec<&str> = Vec::new();
    let mut literal = false;
    for arg in args {
        match arg.as_str() {
            "--" if !literal => literal = true,
            "--json" if !literal => json = true,
            "-h" | "--help" if !literal => words.insert(0, "help"),
            word => words.push(word),
        }
    }
    let Some((first, rest)) = words.split_first() else {
        return Err("name a command".to_owned());
    };
    let joined = |what: &str| {
        if rest.is_empty() {
            Err(format!("{first} needs {what}"))
        } else {
            Ok(rest.join(" "))
        }
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
            [socket, managers, words @ ..] if words.len() <= 1 => Command::Daemon {
                socket: (*socket).to_owned(),
                managers: (*managers).to_owned(),
                words: words.first().map(|words| (*words).to_owned()),
            },
            _ => {
                return Err(
                    "daemon needs a SOCKET and a MANAGERS file, then the launcher's WORDS"
                        .to_owned(),
                );
            }
        },
        "find" => Command::Find(joined("words to find by")?),
        "list" => none(Command::List)?,
        "details" => Command::Details(joined("an ENTRY")?),
        "show" | "copy" => {
            let (entry, detail) = match rest {
                [entry] => ((*entry).to_owned(), None),
                [entry, detail] => ((*entry).to_owned(), Some((*detail).to_owned())),
                [] => return Err(format!("{first} needs an ENTRY")),
                _ => {
                    return Err(format!(
                        "{first} takes an ENTRY and a DETAIL; quote a title with spaces"
                    ));
                }
            };
            if *first == "show" {
                Command::Show { entry, detail }
            } else {
                Command::Copy { entry, detail }
            }
        }
        "code" => Command::Code(joined("an ENTRY")?),
        "let-in" => match rest {
            [manager] => Command::LetIn((*manager).to_owned()),
            _ => return Err("let-in needs one MANAGER".to_owned()),
        },
        "health" => none(Command::Health)?,
        "audit" => none(Command::Audit)?,
        "keys" => none(Command::Keys)?,
        "help" => Command::Help,
        "names" => Command::Names(match rest {
            ["entries"] => Names::Entries,
            ["managers"] => Names::Managers,
            ["details", entry @ ..] if !entry.is_empty() => Names::Details(entry.join(" ")),
            _ => return Err("names needs entries, managers or details ENTRY".to_owned()),
        }),
        "completion" => match rest {
            ["zsh"] => Command::Completion,
            _ => return Err("completion is written for zsh alone".to_owned()),
        },
        other => return Err(format!("{other:?} isn't a command")),
    };
    Ok(Typed { command, json })
}

/// The exit status an error the daemon answered stands for.
pub fn status(error: &Value) -> i32 {
    match error["code"].as_str() {
        Some("signed-out") => SIGNED_OUT,
        Some("not-ready") => NOT_READY,
        _ => FAILED,
    }
}

/// An answer as `--json` gives it: as the daemon said it, less the id that
/// only tied it to its ask.
pub fn as_json(answer: &Value) -> String {
    let mut answer = answer.clone();
    if let Some(answer) = answer.as_object_mut() {
        answer.remove("id");
    }
    serde_json::to_string_pretty(&answer).unwrap_or_default() + "\n"
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

/// A count of things, in words: "1 entry", "3 entries".
pub fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// A manager's state as a person reads it.
fn state_words(state: &str) -> String {
    state.replace('-', " ")
}

/// Each manager that isn't answering, with why and what to do, one a line:
/// what a person reads under a head, and a program on standard error.
pub fn standings(sources: &Value) -> Vec<String> {
    let Some(sources) = sources.as_object() else {
        return Vec::new();
    };
    sources
        .iter()
        .filter(|(_, standing)| standing["state"] != "answering")
        .map(|(manager, standing)| {
            let state = standing["state"].as_str().unwrap_or("");
            let reason = standing["reason"].as_str().unwrap_or("");
            let mut line = format!("{manager} is {}", state_words(state));
            if !reason.is_empty() {
                let _ = write!(line, ": {reason}");
            }
            if state == "signed-out" {
                let _ = write!(line, "; passwords let-in {manager} lets it in");
            }
            line + "."
        })
        .collect()
}

/// The host of a website's address, as a person reads it among others:
/// `https://github.com/login` is `github.com`.
pub fn host(address: &str) -> &str {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    rest.rsplit_once('@').map_or(rest, |(_, host)| host)
}

/// An entry's first website: the one its manager calls primary, else the
/// first it gives.
fn website(record: &Value) -> &str {
    let websites = record["websites"].as_array().map_or(&[][..], Vec::as_slice);
    websites
        .iter()
        .find(|website| website["primary"] == true)
        .or_else(|| websites.first())
        .and_then(|website| website["address"].as_str())
        .unwrap_or("")
}

fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    value[name].as_str().unwrap_or("")
}

/// Entries as the command shows them, from a list or a search's answer:
/// for a person, a head counting them with how each manager stands, then
/// a row each of its title, kind, username, website, collection and key;
/// for a program, those rows alone, each website whole.
pub fn entries(answer: &Value, terminal: bool) -> String {
    const COLUMNS: [Column; 6] = [
        left("title"),
        left("kind"),
        left("username"),
        left("website"),
        left("collection"),
        left("key"),
    ];
    let records = answer["records"].as_array().map_or(&[][..], Vec::as_slice);
    let rows: Vec<Vec<String>> = records
        .iter()
        .map(|record| {
            let address = website(record);
            vec![
                text(record, "title").to_owned(),
                text(record, "category").to_owned(),
                text(record, "username").to_owned(),
                if terminal { host(address) } else { address }.to_owned(),
                record["collection"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
                text(record, "key").to_owned(),
            ]
        })
        .collect();
    if !terminal {
        return table(&COLUMNS, &rows, false);
    }
    let mut out = head(records.len(), &answer["sources"]);
    if !rows.is_empty() {
        out += "\n";
        out += &table(&COLUMNS, &rows, true);
    }
    out
}

/// A person's head over entries: how many, each manager and its state,
/// then a line for each that isn't answering.
fn head(count: usize, sources: &Value) -> String {
    let mut line = counted(count, "entry", "entries");
    if let Some(sources) = sources.as_object() {
        for (manager, standing) in sources {
            let state = standing["state"].as_str().unwrap_or("");
            let _ = write!(line, " · {manager} {}", state_words(state));
        }
    }
    let mut out = line + "\n";
    for said in standings(sources) {
        out += &said;
        out += "\n";
    }
    out
}

/// One entry in full, as the command shows it: for a person, a head of its
/// title, kind, collection and manager, a row for each detail, its hidden
/// ones saying only that they're hidden and how strong a password is, and
/// then every other fact it has, its websites, tags and times among them;
/// for a program, a row for each detail alone, ending with its id.
pub fn details(record: &Value, terminal: bool) -> String {
    const FOR_A_PROGRAM: [Column; 7] = [
        left("detail"),
        left("section"),
        left("kind"),
        left("hidden"),
        left("value"),
        left("strength"),
        left("id"),
    ];
    const FOR_A_PERSON: [Column; 5] = [
        left("detail"),
        left("section"),
        left("kind"),
        left("value"),
        left("strength"),
    ];
    let none = Vec::new();
    let details = record["details"].as_array().unwrap_or(&none);
    let hidden = |detail: &Value| detail["hidden"] == true;
    if !terminal {
        let rows: Vec<Vec<String>> = details
            .iter()
            .map(|detail| {
                vec![
                    text(detail, "label").to_owned(),
                    text(detail, "section").to_owned(),
                    text(detail, "type").to_owned(),
                    if hidden(detail) { "hidden" } else { "" }.to_owned(),
                    text(detail, "value").to_owned(),
                    text(detail, "strength").to_owned(),
                    text(detail, "id").to_owned(),
                ]
            })
            .collect();
        return table(&FOR_A_PROGRAM, &rows, false);
    }
    let rows: Vec<Vec<String>> = details
        .iter()
        .map(|detail| {
            vec![
                text(detail, "label").to_owned(),
                text(detail, "section").to_owned(),
                text(detail, "type").to_owned(),
                if hidden(detail) {
                    "hidden".to_owned()
                } else {
                    text(detail, "value").to_owned()
                },
                text(detail, "strength").to_owned(),
            ]
        })
        .collect();
    let mut title = vec![text(record, "title"), text(record, "category")];
    let collection = record["collection"]["name"].as_str().unwrap_or("");
    let manager = text(record, "manager");
    let within = if collection.is_empty() {
        String::new()
    } else {
        format!("in {collection}")
    };
    title.push(&within);
    title.push(manager);
    if record["favourite"] == true {
        title.push("favourite");
    }
    title.retain(|part| !part.is_empty());
    let mut out = title.join(" · ") + "\n";
    if rows.is_empty() {
        out += "No details are held of it yet.\n";
    } else {
        out += "\n";
        out += &table(&FOR_A_PERSON, &rows, true);
    }
    let facts = facts(record);
    if !facts.is_empty() {
        out += "\n";
        let width = facts
            .iter()
            .map(|(name, _)| name.chars().count())
            .max()
            .unwrap_or(0);
        for (name, value) in facts {
            out += format!("{name:width$}  {value}").trim_end();
            out += "\n";
        }
    }
    out
}

/// Every fact of an entry past its details, each named, for a person: each
/// website with its label, its tags, when it was made and changed, what
/// its manager says of it alone, and its key.
fn facts(record: &Value) -> Vec<(String, String)> {
    let mut facts = Vec::new();
    for website in record["websites"].as_array().into_iter().flatten() {
        let mut said = text(website, "address").to_owned();
        let mut about: Vec<&str> = vec![text(website, "label")];
        if website["primary"] == true {
            about.push("primary");
        }
        about.retain(|part| !part.is_empty());
        if !about.is_empty() {
            let _ = write!(said, " ({})", about.join(", "));
        }
        facts.push(("website".to_owned(), said));
    }
    let tags: Vec<&str> = record["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if !tags.is_empty() {
        facts.push(("tags".to_owned(), tags.join(", ")));
    }
    for (name, said) in [("created", "made"), ("changed", "changed")] {
        if let Some(at) = record[name].as_u64() {
            facts.push((said.to_owned(), when(at)));
        }
    }
    for (name, value) in record["own"].as_object().into_iter().flatten() {
        let value = match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        facts.push((name.clone(), value));
    }
    facts.push(("key".to_owned(), text(record, "key").to_owned()));
    facts
}

/// A time in milliseconds since the epoch as a person reads it, to the
/// minute, in UTC and saying so, since a manager says its times in UTC.
pub fn when(millis: u64) -> String {
    let seconds = millis / 1000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let of_day = seconds % 86_400;
    // The civil date of a day count (Howard Hinnant's), as 1Password's
    // manager reads one the other way.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        of_day / 3600,
        of_day % 3600 / 60
    )
}

/// How the daemon stands, from its health: for a person, a head of its
/// state, since when and how many entries it holds, how fast its last asks
/// were answered, then a row for each manager; for a program, the rows.
pub fn health(answer: &Value, terminal: bool) -> String {
    const COLUMNS: [Column; 3] = [left("manager"), left("state"), left("reason")];
    let rows: Vec<Vec<String>> = answer["sources"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(manager, standing)| {
            let state = standing["state"].as_str().unwrap_or("");
            vec![
                manager.clone(),
                if terminal {
                    state_words(state)
                } else {
                    state.to_owned()
                },
                text(standing, "reason").to_owned(),
            ]
        })
        .collect();
    if !terminal {
        return table(&COLUMNS, &rows, false);
    }
    let held = usize::try_from(answer["entries"].as_u64().unwrap_or(0)).unwrap_or(0);
    let mut parts = vec![
        format!("passwords {}", state_words(text(answer, "state"))),
        counted(held, "entry held", "entries held"),
    ];
    if let Some(since) = answer["since"].as_u64() {
        parts.insert(1, format!("since {}", when(since)));
    }
    let mut out = parts.join(" · ") + "\n";
    let mut took: Vec<u64> = answer["took"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    took.sort_unstable();
    if let (Some(median), Some(most)) = (took.get(took.len() / 2), took.last()) {
        let _ = writeln!(
            out,
            "The last {} took {median} ms at the median and {most} ms at the most.",
            counted(took.len(), "ask", "asks")
        );
    }
    out += "\n";
    out + &table(&COLUMNS, &rows, true)
}

/// A count grouped in threes as a person reads it: 1204 as "1,204".
pub fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What an audit found, each entry with a problem: for a person, a head
/// of how many passwords were audited and how many entries are reused and
/// breached, how each manager stands, then a row each of its title, how
/// many other entries share its password, how many times it was seen in
/// a breach, and its key; for a program, those rows, every count a
/// figure, breached empty where Have I Been Pwned couldn't be asked.
pub fn audit(answer: &Value, terminal: bool) -> String {
    const COLUMNS: [Column; 4] = [
        left("title"),
        right("reused with"),
        right("breached"),
        left("key"),
    ];
    let records = answer["records"].as_array().map_or(&[][..], Vec::as_slice);
    let rows: Vec<Vec<String>> = records
        .iter()
        .map(|record| {
            let health = &record["health"];
            let reused = health["reused"].as_u64().unwrap_or(0);
            let breached = health["breached"].as_u64();
            let (reused, breached) = if terminal {
                (
                    if reused == 0 {
                        String::new()
                    } else {
                        counted(
                            usize::try_from(reused).unwrap_or(usize::MAX),
                            "other",
                            "others",
                        )
                    },
                    breached
                        .filter(|times| *times > 0)
                        .map(grouped)
                        .unwrap_or_default(),
                )
            } else {
                (
                    reused.to_string(),
                    breached.map(|times| times.to_string()).unwrap_or_default(),
                )
            };
            vec![
                text(record, "title").to_owned(),
                reused,
                breached,
                text(record, "key").to_owned(),
            ]
        })
        .collect();
    if !terminal {
        return table(&COLUMNS, &rows, false);
    }
    let audited = usize::try_from(answer["audited"].as_u64().unwrap_or(0)).unwrap_or(0);
    let reused = usize::try_from(answer["reused"].as_u64().unwrap_or(0)).unwrap_or(0);
    let mut parts = vec![
        counted(audited, "password audited", "passwords audited"),
        format!("{} reused", counted(reused, "entry", "entries")),
    ];
    parts.push(match answer["breached"].as_u64() {
        Some(breached) => format!(
            "{} breached",
            counted(usize::try_from(breached).unwrap_or(0), "entry", "entries")
        ),
        None => "breaches not checked".to_owned(),
    });
    let mut out = parts.join(" · ") + "\n";
    for said in standings(&answer["sources"]) {
        out += &said;
        out += "\n";
    }
    if !rows.is_empty() {
        out += "\n";
        out += &table(&COLUMNS, &rows, true);
    }
    out
}

/// A one-time code: for a person, the code and the seconds left of its
/// period; for a program, the code, the seconds left and the period.
pub fn code(answer: &Value, terminal: bool) -> String {
    let code = text(answer, "code");
    let left = answer["left"].as_u64().unwrap_or(0);
    let period = answer["period"].as_u64().unwrap_or(0);
    if terminal {
        format!("{code}  {left} s left of {period}\n")
    } else {
        format!("{code}\t{left}\t{period}\n")
    }
}

/// What a copy did: for a person, which detail went and when the clipboard
/// clears; for a program, the detail and when it clears, in milliseconds
/// since the epoch.
pub fn copied(answer: &Value, now: u64, terminal: bool) -> String {
    let detail = answer["detail"].as_str().unwrap_or("password");
    let clears = answer["clears"].as_u64().unwrap_or(0);
    if terminal {
        let left = clears.saturating_sub(now).div_ceil(1000);
        format!("Copied its {detail}; the clipboard clears in {left} s.\n")
    } else {
        format!("{detail}\t{clears}\n")
    }
}

/// Whether two titles are the same to a person typing one, as the command
/// finds an entry by its title: letter for letter, whatever their case.
fn same_title(one: &str, other: &str) -> bool {
    one.to_lowercase() == other.to_lowercase()
}

/// What some words find of the entries a search answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Resolved {
    /// The one entry, by its key.
    One(String),
    None,
    /// More than one, none titled just so alone.
    Many(Vec<Value>),
}

/// The entry these words name, of those a search by them found: the only
/// one, else the only one titled just so.
pub fn resolve(words: &str, found: &[Value]) -> Resolved {
    let key = |record: &Value| text(record, "key").to_owned();
    match found {
        [] => Resolved::None,
        [one] => Resolved::One(key(one)),
        many => {
            let titled: Vec<&Value> = many
                .iter()
                .filter(|record| same_title(text(record, "title"), words))
                .collect();
            match titled.as_slice() {
                [one] => Resolved::One(key(one)),
                _ => Resolved::Many(many.to_vec()),
            }
        }
    }
}

/// The id of the detail of an entry a person named, by its id or else by
/// its label, whatever its case; or what to say when none is, or several.
pub fn detail_id(record: &Value, wanted: &str) -> Result<String, String> {
    let title = text(record, "title");
    let details = record["details"].as_array().map_or(&[][..], Vec::as_slice);
    if details.is_empty() {
        return Err(format!("{title}'s details couldn't be read."));
    }
    if details.iter().any(|detail| text(detail, "id") == wanted) {
        return Ok(wanted.to_owned());
    }
    let labelled: Vec<&Value> = details
        .iter()
        .filter(|detail| same_title(text(detail, "label"), wanted))
        .collect();
    match labelled.as_slice() {
        [one] => Ok(text(one, "id").to_owned()),
        [] => {
            let labels: Vec<&str> = details.iter().map(|detail| text(detail, "label")).collect();
            Err(format!(
                "{title} has no detail {wanted:?}; it has {}.",
                labels.join(", ")
            ))
        }
        many => {
            let ids: Vec<String> = many
                .iter()
                .map(|detail| {
                    let section = text(detail, "section");
                    if section.is_empty() {
                        text(detail, "id").to_owned()
                    } else {
                        format!("{} (in {section})", text(detail, "id"))
                    }
                })
                .collect();
            Err(format!(
                "{} of {title}'s details are labelled {wanted:?}; name one by its id: {}.",
                many.len(),
                ids.join(", ")
            ))
        }
    }
}

/// Names as zsh's `_describe` reads them, one a line: the name, its colons
/// escaped, since a colon ends it, then a colon and what it is. A line
/// break in either would start another, so it's a space.
pub fn for_zsh(names: &[(String, String)]) -> String {
    let flat = |text: &str| text.replace(['\n', '\r'], " ");
    names.iter().fold(String::new(), |mut out, (name, what)| {
        let _ = writeln!(out, "{}:{}", flat(name).replace(':', "\\:"), flat(what));
        out
    })
}

/// Whether a title finds the entry it names alone, as [`resolve`] would:
/// one with a word in it, that no other entry shares whatever its case,
/// and that can't be read as a key.
fn finds_alone(title: &str, records: &[Value]) -> bool {
    title.chars().any(char::is_alphanumeric)
        && named(title).is_none()
        && records
            .iter()
            .filter(|record| same_title(text(record, "title"), title))
            .count()
            == 1
}

/// Each entry as zsh offers it, by the title that finds it, or its key
/// where another shares the title, each with its kind, username and
/// website, and its manager where there are more than one.
pub fn entry_names(records: &[Value]) -> Vec<(String, String)> {
    let first = records.first().map(|record| text(record, "manager"));
    let several = records
        .iter()
        .any(|record| Some(text(record, "manager")) != first);
    records
        .iter()
        .map(|record| {
            let title = text(record, "title");
            let mut what = vec![text(record, "category"), text(record, "username")];
            let address = host(website(record));
            what.push(address);
            if several {
                what.push(text(record, "manager"));
            }
            let name = if finds_alone(title, records) {
                title.to_owned()
            } else {
                what.insert(0, title);
                text(record, "key").to_owned()
            };
            what.retain(|part| !part.is_empty());
            (name, what.join(" · "))
        })
        .collect()
}

/// Each detail of an entry as zsh offers it, by its label, or its id where
/// another shares the label, each with its kind and section; never its
/// value, hidden or not.
pub fn detail_names(record: &Value) -> Vec<(String, String)> {
    let details = record["details"].as_array().map_or(&[][..], Vec::as_slice);
    details
        .iter()
        .map(|detail| {
            let label = text(detail, "label");
            let shared = details
                .iter()
                .filter(|other| same_title(text(other, "label"), label))
                .count()
                > 1;
            let kind = if detail["hidden"] == true && text(detail, "type") == "text" {
                "hidden"
            } else {
                text(detail, "type")
            };
            let mut what = vec![kind, text(detail, "section")];
            let name = if shared || label.is_empty() {
                what.insert(0, label);
                text(detail, "id")
            } else {
                label
            };
            what.retain(|part| !part.is_empty());
            (name.to_owned(), what.join(" · "))
        })
        .collect()
}

/// Each password manager as zsh offers it, with how it stands.
pub fn manager_names(sources: &Value) -> Vec<(String, String)> {
    sources
        .as_object()
        .into_iter()
        .flatten()
        .map(|(manager, standing)| {
            (
                manager.clone(),
                state_words(standing["state"].as_str().unwrap_or("")),
            )
        })
        .collect()
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
    let mut out = format!(
        "1Password's SSH agent · {}\n",
        counted(keys.len(), "key", "keys")
    );
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

    fn record(key: &str, title: &str) -> Value {
        json!({
            "key": format!("entry:1password:{key}"), "manager": "1password", "id": key,
            "title": title, "category": "login", "username": "ada",
            "collection": {"id": "v1", "name": "Private"},
            "websites": [
                {"address": "https://example.org/", "label": "", "primary": false},
                {"address": "https://github.com/login", "label": "website", "primary": true},
            ],
        })
    }

    fn github() -> Value {
        let mut github = record("gh", "GitHub");
        github["favourite"] = json!(true);
        github["tags"] = json!(["work", "code"]);
        github["created"] = json!(1_709_208_000_000_u64);
        github["changed"] = json!(1_794_082_895_500_u64);
        github["own"] = json!({"files": 1});
        github["details"] = json!([
            {"id": "username", "label": "username", "type": "text", "hidden": false,
             "purpose": "username", "value": "ada"},
            {"id": "password", "label": "password", "type": "hidden", "hidden": true,
             "purpose": "password", "strength": "fantastic"},
            {"id": "TOTP_1", "label": "one-time password", "type": "code", "hidden": true,
             "section": "Security"},
            {"id": "bank", "label": "PIN", "type": "text", "hidden": true, "section": "Bank"},
            {"id": "door", "label": "pin", "type": "text", "hidden": true, "section": "Door"},
        ]);
        github
    }

    #[test]
    fn each_command_is_read_with_what_it_takes() {
        assert_eq!(command("find git hub"), Command::Find("git hub".to_owned()));
        assert_eq!(command("list"), Command::List);
        assert_eq!(
            command("show GitHub"),
            Command::Show {
                entry: "GitHub".to_owned(),
                detail: None
            }
        );
        assert_eq!(
            command("copy GitHub username"),
            Command::Copy {
                entry: "GitHub".to_owned(),
                detail: Some("username".to_owned())
            }
        );
        assert_eq!(command("code Git Hub"), Command::Code("Git Hub".to_owned()));
        assert_eq!(
            command("details GitHub"),
            Command::Details("GitHub".to_owned())
        );
        assert_eq!(
            command("let-in 1password"),
            Command::LetIn("1password".to_owned())
        );
        assert_eq!(command("health"), Command::Health);
        assert_eq!(command("audit"), Command::Audit);
        assert_eq!(command("keys"), Command::Keys);
        assert_eq!(command("help"), Command::Help);
        assert_eq!(command("--help"), Command::Help);
        assert_eq!(command("names entries"), Command::Names(Names::Entries));
        assert_eq!(
            command("names details Git Hub"),
            Command::Names(Names::Details("Git Hub".to_owned()))
        );
        assert_eq!(command("names managers"), Command::Names(Names::Managers));
        assert_eq!(command("completion zsh"), Command::Completion);
        assert_eq!(
            command("daemon /run/passwords.sock managers.json"),
            Command::Daemon {
                socket: "/run/passwords.sock".to_owned(),
                managers: "managers.json".to_owned(),
                words: None
            }
        );
        assert_eq!(
            command("daemon /run/passwords.sock managers.json words.json"),
            Command::Daemon {
                socket: "/run/passwords.sock".to_owned(),
                managers: "managers.json".to_owned(),
                words: Some("words.json".to_owned())
            }
        );
    }

    #[test]
    fn json_is_asked_anywhere_before_the_words_end() {
        let asked = typed("--json list").unwrap();
        assert!(asked.json);
        assert_eq!(asked.command, Command::List);
        assert!(typed("find git --json").unwrap().json);
        // After --, it's a title like any other.
        let literal = typed("show -- --json").unwrap();
        assert!(!literal.json);
        assert_eq!(
            literal.command,
            Command::Show {
                entry: "--json".to_owned(),
                detail: None
            }
        );
    }

    #[test]
    fn a_mistyped_command_says_what_it_needs() {
        for (words, said) in [
            ("", "name a command"),
            ("dance", "\"dance\" isn't a command"),
            ("find", "find needs words to find by"),
            ("show", "show needs an ENTRY"),
            ("show a b c", "quote a title"),
            ("list everything", "list takes nothing after it"),
            ("let-in", "let-in needs one MANAGER"),
            ("names people", "names needs"),
            ("completion bash", "zsh alone"),
            ("daemon socket", "daemon needs"),
        ] {
            let error = typed(words).unwrap_err();
            assert!(error.contains(said), "{words:?} said {error:?}");
        }
    }

    #[test]
    fn the_usage_says_every_command_and_exit_status() {
        for word in [
            "find", "list", "details", "show", "copy", "code", "let-in", "health", "audit", "keys",
            "help", "--json",
        ] {
            assert!(USAGE.contains(word), "the usage leaves out {word}");
        }
        for status in [0, FAILED, MISTYPED, UNANSWERED, NOT_READY, SIGNED_OUT] {
            assert!(
                USAGE.contains(&format!("\n  {status:<2}  ")),
                "the usage leaves out {status}"
            );
        }
        assert!(USAGE.lines().all(|line| line.chars().count() < 100));
    }

    #[test]
    fn an_error_answered_gives_its_exit_status() {
        assert_eq!(status(&json!({"code": "signed-out"})), SIGNED_OUT);
        assert_eq!(status(&json!({"code": "not-ready"})), NOT_READY);
        assert_eq!(status(&json!({"code": "refused"})), FAILED);
        assert_eq!(status(&json!({})), FAILED);
    }

    #[test]
    fn json_is_the_answer_as_it_is_less_its_id() {
        let said = as_json(&json!({"id": 3, "code": "123456", "period": 30}));
        let read: Value = serde_json::from_str(&said).unwrap();
        assert_eq!(read, json!({"code": "123456", "period": 30}));
    }

    #[test]
    fn a_person_reads_columns_under_a_header() {
        let answer = json!({
            "records": [record("gh", "GitHub"), {"key": "entry:1password:n", "title": "Door",
                         "category": "note"}],
            "sources": {"1password": {"state": "answering"}},
        });
        assert_eq!(
            entries(&answer, true),
            "2 entries · 1password answering\n\
             \n\
             title   kind   username  website     collection  key\n\
             GitHub  login  ada       github.com  Private     entry:1password:gh\n\
             Door    note   –         –           –           entry:1password:n\n"
        );
    }

    #[test]
    fn a_program_reads_every_column_tab_separated_without_a_header() {
        let mut tabbed = record("gh", "Git\tHub");
        tabbed["collection"] = Value::Null;
        let answer = json!({"records": [tabbed], "sources": {}});
        assert_eq!(
            entries(&answer, false),
            "Git Hub\tlogin\tada\thttps://github.com/login\t\tentry:1password:gh\n"
        );
        assert_eq!(entries(&json!({"records": []}), false), "");
    }

    #[test]
    fn a_column_no_row_fills_is_left_out_for_a_person_alone() {
        let rows = vec![vec!["a".to_owned(), String::new(), "1".to_owned()]];
        let columns = [left("name"), left("empty"), right("count")];
        assert_eq!(table(&columns, &rows, true), "name  count\na         1\n");
        assert_eq!(table(&columns, &rows, false), "a\t\t1\n");
    }

    #[test]
    fn a_manager_not_answering_is_said_with_what_to_do() {
        let answer = json!({
            "records": [],
            "sources": {"1password": {"state": "signed-out",
                                      "reason": "1Password asks you to let the launcher in"}},
        });
        assert_eq!(
            entries(&answer, true),
            "0 entries · 1password signed out\n\
             1password is signed out: 1Password asks you to let the launcher in; \
             passwords let-in 1password lets it in.\n"
        );
        assert_eq!(
            standings(
                &json!({"other": {"state": "refused", "reason": "it stopped"},
                              "fine": {"state": "answering"}})
            ),
            ["other is refused: it stopped."]
        );
    }

    #[test]
    fn an_entry_shows_its_details_without_a_secret_and_every_other_fact() {
        let shown = details(&github(), true);
        assert_eq!(
            shown,
            "GitHub · login · in Private · 1password · favourite\n\
             \n\
             detail             section   kind    value   strength\n\
             username           –         text    ada     –\n\
             password           –         hidden  hidden  fantastic\n\
             one-time password  Security  code    hidden  –\n\
             PIN                Bank      text    hidden  –\n\
             pin                Door      text    hidden  –\n\
             \n\
             website  https://example.org/\n\
             website  https://github.com/login (website, primary)\n\
             tags     work, code\n\
             made     2024-02-29 12:00 UTC\n\
             changed  2026-11-07 20:21 UTC\n\
             files    1\n\
             key      entry:1password:gh\n"
        );
        assert_eq!(
            details(&github(), false).lines().nth(1),
            Some("password\t\thidden\thidden\t\tfantastic\tpassword")
        );
        let summary = details(&record("b", "Bank"), true);
        assert!(
            summary.contains("No details are held of it yet."),
            "{summary}"
        );
    }

    #[test]
    fn times_are_read_to_the_minute_in_utc() {
        assert_eq!(when(0), "1970-01-01 00:00 UTC");
        assert_eq!(when(951_868_801_000), "2000-03-01 00:00 UTC");
        assert_eq!(when(1_709_208_000_000), "2024-02-29 12:00 UTC");
        assert_eq!(when(1_794_082_895_500), "2026-11-07 20:21 UTC");
    }

    #[test]
    fn health_says_how_the_daemon_and_each_manager_stand() {
        let answer = json!({
            "id": 1, "state": "signed-out", "since": 1_794_082_895_500_u64, "entries": 1,
            "took": [40, 2, 900, 3, 5],
            "sources": {"1password": {"state": "signed-out", "reason": "Let it in"}},
        });
        assert_eq!(
            health(&answer, true),
            "passwords signed out · since 2026-11-07 20:21 UTC · 1 entry held\n\
             The last 5 asks took 5 ms at the median and 900 ms at the most.\n\
             \n\
             manager    state       reason\n\
             1password  signed out  Let it in\n"
        );
        assert_eq!(health(&answer, false), "1password\tsigned-out\tLet it in\n");
    }

    #[test]
    fn an_audit_lists_each_entry_with_a_problem() {
        let mut github = record("gh", "GitHub");
        github["health"] = json!({"reused": 2, "breached": 1204});
        let mut bank = record("bank", "Bank");
        bank["health"] = json!({"reused": 0, "breached": 1});
        let answer = json!({
            "audited": 3, "reused": 1, "breached": 2, "records": [github, bank],
            "sources": {"1password": {"state": "answering"}},
        });
        assert_eq!(
            audit(&answer, true),
            "3 passwords audited \u{b7} 1 entry reused \u{b7} 2 entries breached\n\
             \n\
             title   reused with  breached  key\n\
             GitHub     2 others     1,204  entry:1password:gh\n\
             Bank              \u{2013}         1  entry:1password:bank\n"
        );
        assert_eq!(
            audit(&answer, false),
            "GitHub\t2\t1204\tentry:1password:gh\nBank\t0\t1\tentry:1password:bank\n"
        );
        let unreached = json!({"audited": 1, "reused": 0, "breached": null, "records": []});
        assert_eq!(
            audit(&unreached, true),
            "1 password audited \u{b7} 0 entries reused \u{b7} breaches not checked\n"
        );
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }

    #[test]
    fn a_code_and_a_copy_say_how_long_they_last() {
        let answer = json!({"code": "123456", "period": 30, "left": 12});
        assert_eq!(code(&answer, true), "123456  12 s left of 30\n");
        assert_eq!(code(&answer, false), "123456\t12\t30\n");
        let answer = json!({"copied": true, "detail": "password", "clears": 91_000});
        assert_eq!(
            copied(&answer, 1_000, true),
            "Copied its password; the clipboard clears in 90 s.\n"
        );
        assert_eq!(copied(&answer, 1_000, false), "password\t91000\n");
    }

    #[test]
    fn words_name_the_one_entry_found_or_the_one_titled_so() {
        let github = record("gh", "GitHub");
        let gist = record("gist", "GitHub Gist");
        assert_eq!(
            resolve("github", &[github.clone(), gist.clone()]),
            Resolved::One("entry:1password:gh".to_owned())
        );
        assert_eq!(
            resolve("gist", std::slice::from_ref(&gist)),
            Resolved::One("entry:1password:gist".to_owned())
        );
        assert_eq!(resolve("bank", &[]), Resolved::None);
        let again = record("gh2", "github");
        assert!(
            matches!(resolve("GitHub", &[github, again]), Resolved::Many(many) if many.len() == 2)
        );
    }

    #[test]
    fn a_detail_is_named_by_its_id_or_its_label() {
        let github = github();
        assert_eq!(detail_id(&github, "TOTP_1").unwrap(), "TOTP_1");
        assert_eq!(detail_id(&github, "One-Time Password").unwrap(), "TOTP_1");
        let none = detail_id(&github, "cvv").unwrap_err();
        assert!(none.contains("username, password"), "{none}");
        let two = detail_id(&github, "pin").unwrap_err();
        assert!(two.contains("bank (in Bank), door (in Door)"), "{two}");
        assert!(detail_id(&record("b", "Bank"), "password").is_err());
    }

    #[test]
    fn zsh_is_offered_each_entry_by_the_title_that_finds_it() {
        let mut card = record("card", "Bank: Card");
        card["category"] = json!("card");
        card["username"] = Value::Null;
        card["websites"] = json!([]);
        let records = [
            record("gh", "GitHub"),
            record("a", "Mail"),
            record("b", "mail"),
            card,
            record("stars", "***"),
        ];
        let names = entry_names(&records);
        let offered: Vec<&str> = names.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            offered,
            [
                "GitHub",
                "entry:1password:a",
                "entry:1password:b",
                "Bank: Card",
                "entry:1password:stars"
            ]
        );
        assert_eq!(names[0].1, "login · ada · github.com");
        assert_eq!(names[1].1, "Mail · login · ada · github.com");
        assert_eq!(names[3].1, "card");
        assert_eq!(
            for_zsh(&names[..2]),
            "GitHub:login · ada · github.com\n\
             entry\\:1password\\:a:Mail · login · ada · github.com\n"
        );
    }

    #[test]
    fn zsh_is_told_each_entrys_manager_where_there_are_several() {
        let mut other = record("x", "Bank");
        other["manager"] = json!("keepass");
        let names = entry_names(&[record("gh", "GitHub"), other]);
        assert_eq!(names[1].1, "login · ada · github.com · keepass");
    }

    #[test]
    fn zsh_is_offered_each_detail_by_its_label_and_never_its_value() {
        let names = detail_names(&github());
        assert_eq!(
            names,
            [
                ("username".to_owned(), "text".to_owned()),
                ("password".to_owned(), "hidden".to_owned()),
                ("one-time password".to_owned(), "code · Security".to_owned()),
                ("bank".to_owned(), "PIN · hidden · Bank".to_owned()),
                ("door".to_owned(), "pin · hidden · Door".to_owned()),
            ]
        );
        assert!(!for_zsh(&names).contains("ada"));
        assert_eq!(
            manager_names(&json!({"1password": {"state": "signed-out"}})),
            [("1password".to_owned(), "signed out".to_owned())]
        );
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
