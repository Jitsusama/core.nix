//! What a `mycelium:` link opens, run by the browser on each one Joel follows
//! (home.nix's desktop entry): `mycelium:cell/<ADDRESS>` opens Agents' Talk
//! to that session, its field empty, as `qs ipc call launcher talkTo` does,
//! and `mycelium:network/<name>` opens Agents' Networks on that network's
//! row, selected, as `qs ipc call launcher network` does. The realm's map
//! links its cells and networks so (XUBK2E, 2026-11-15 [2b5c927a]).
//! `mycelium:machine/<name>` opens Machines on that machine's row,
//! selected, as `qs ipc call launcher machine` does, which the system page
//! offers as each machine's Open in the Launcher: the page can't act, so
//! the row's own acts are where Joel starts a session there or updates it.
//! `mycelium:cell/<ADDRESS>/go` goes to that session instead, its window
//! raised or one opened on it, as `qs ipc call launcher goTo` does, which a
//! notice from a session offers as its Go To. A cell may say where it runs
//! and what in it is meant, `mycelium:cell/<ADDRESS>@<machine>#<anchor>`, as
//! a notice's whole cell link does: the machine is checked and the anchor let
//! go, since the address alone names the session across the realm.
//! `mycelium:sprout?words=<words>&from=<ADDRESS>` opens Start New Work with
//! those words in its field, as `qs ipc call launcher newWork` does, which
//! the realm's map offers to start work from a cell: the words are decoded
//! once and folded to one plain line, and the cell is kept only where it's
//! an address. Nothing starts until Joel's own Return there.
//!
//! Any page can write such a link, so it's trusted with a name and one word
//! more. Only the step after a cell's address is read, and only `go` means
//! anything: any other step, a query or a fragment is let go, and a name of
//! any letter an address or a network's name never holds is refused, so
//! nothing else reaches the launcher. The launcher in turn reads only the
//! name's first word and never types or sends anything for Joel. Going to a
//! session does what its row's Switch does, no more: raises its window, or
//! opens one on it, joining it from another machine where it runs there,
//! wakes it where it's parked, and counts its answer as seen. A link followed within `GAP` of
//! the last is let go too, so a page can't open the launcher over his work
//! again and again; where when the last was followed can't be kept, none
//! is followed at all.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use terminal_kit::{Exit, Failure, Help};

mod plain;
use plain::plain;

/// How long after a link is followed another is let go: longer than a
/// double click or a page's burst of them, shorter than Joel following a
/// second one by hand.
const GAP: Duration = Duration::from_secs(3);

/// The longest name a link carries: a network's name or a machine's runs to
/// a few dozen letters, so anything past this is no name. An address has a
/// bound of its own, `LONGEST_ADDRESS`.
const LONGEST: usize = 64;

/// The most letters a mesh address runs to, as the notices daemon reads a
/// cell (notice.rs's `LONGEST_ADDRESS`): six or eight today, with room.
const LONGEST_ADDRESS: usize = 12;

/// `qs`, by the path the package was built with, or found on PATH.
const QS: &str = match option_env!("MYCELIUM_LINK_QS") {
    Some(path) => path,
    None => "qs",
};

/// What a link names: a cell to talk to or to go to, by its session's
/// address, or a network or a machine, by its name, each as the mesh says
/// it.
/// Or new work to start: the words a page offers, and the cell it came
/// from where it names one by its address.
#[derive(Debug, PartialEq, Eq)]
enum Link {
    Cell(String),
    GoTo(String),
    Network(String),
    Machine(String),
    Sprout { words: String, from: Option<String> },
}

impl Link {
    /// The launcher's ipc call opening what it names. New work's words and
    /// cell each go after a fixed word, so neither is ever a flag to `qs`,
    /// whatever the page wrote, and no cell is still an argument.
    fn call(&self) -> Vec<String> {
        let call = |function: &str, argument: &str| vec![function.to_owned(), argument.to_owned()];
        match self {
            Self::Cell(address) => call("talkTo", address),
            Self::GoTo(address) => call("goTo", address),
            Self::Network(name) => call("network", name),
            Self::Machine(name) => call("machine", name),
            Self::Sprout { words, from } => vec![
                "newWork".to_owned(),
                format!("{WORDS_SAID}{words}"),
                format!("{FROM_SAID}{}", from.as_deref().unwrap_or("")),
            ],
        }
    }
}

/// The word before new work's words in the launcher's call.
const WORDS_SAID: &str = "words:";

/// The word before the cell new work came from in the launcher's call.
const FROM_SAID: &str = "from:";

/// The most characters new work's words run to: a few sentences, as Start
/// New Work's field takes, so a page can't fill the launcher with a book.
const MOST_WORDS: usize = 1000;

/// Whether `name` is one a link may carry: letters, digits and the dot,
/// hyphen and underscore network names join words with, no more than
/// `LONGEST` of them, and a letter or a digit first, so `qs` never takes
/// it for a flag of its own.
fn named(name: &str) -> bool {
    name.len() <= LONGEST
        && name
            .bytes()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// Whether `address` is one a session has on the mesh: capital letters and
/// digits, no more than `LONGEST_ADDRESS` of them.
fn addressed(address: &str) -> bool {
    (1..=LONGEST_ADDRESS).contains(&address.len())
        && address
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

/// The address a cell's name gives, `<ADDRESS>` or `<ADDRESS>@<machine>` as
/// a notice's whole cell link says where it runs, or none where either part
/// is misshaped. The machine is only checked: the address names one session
/// across the realm, and the launcher finds it wherever it runs.
fn address_of(name: &str) -> Option<&str> {
    let (address, placed) = match name.split_once('@') {
        Some((address, machine)) => (address, named(machine)),
        None => (name, true),
    };
    (addressed(address) && placed).then_some(address)
}

/// What `link` names, or what's wrong with it: its scheme in any case, with
/// or without the slashes a page may add, then its kind and its name, and
/// for a cell `go` after it; what else follows the name, a step, a query or
/// a fragment such as a notice's `#notice:<id>`, is let go.
fn read(link: &str) -> Result<Link, Failure> {
    let wrong = |why: &str| {
        Failure::new(Exit::Usage, format!("{link:?} isn't a mycelium link"))
            .because(why.to_owned())
            .try_this("mycelium-link mycelium:cell/DAHTNJ")
    };
    let scheme = "mycelium:";
    let rest = link
        .get(..scheme.len())
        .filter(|head| head.eq_ignore_ascii_case(scheme))
        .map(|_| &link[scheme.len()..])
        .ok_or_else(|| wrong("it doesn't start with mycelium:"))?;
    let rest = rest.trim_start_matches('/');
    let path = rest.split(['?', '#']).next().unwrap_or("");
    let mut steps = path.split('/');
    let kind = steps.next().unwrap_or("");
    let name = steps.next().unwrap_or("");
    match kind {
        "cell" => {
            let address = address_of(name).ok_or_else(|| {
                wrong("its cell isn't an address in capitals, and a machine after @ if any")
            })?;
            Ok(if steps.next() == Some("go") {
                Link::GoTo(address.to_owned())
            } else {
                Link::Cell(address.to_owned())
            })
        }
        "network" if named(name) => Ok(Link::Network(name.to_owned())),
        "network" => Err(wrong(
            "it names no network in letters, digits, dots, hyphens or underscores",
        )),
        "machine" if named(name) => Ok(Link::Machine(name.to_owned())),
        "machine" => Err(wrong(
            "it names no machine in letters, digits, dots, hyphens or underscores",
        )),
        "sprout" => sprouted(query_of(rest)).map_err(|why| wrong(&why)),
        _ => Err(wrong(
            "its kind is none of cell, network, machine or sprout",
        )),
    }
}

/// What follows a link's `?`, up to its fragment, or nothing.
fn query_of(rest: &str) -> &str {
    let before_fragment = rest.split('#').next().unwrap_or("");
    before_fragment
        .split_once('?')
        .map_or("", |(_, query)| query)
}

/// New work a sprout link's query offers: its `words`, said once, decoded
/// and folded to one plain line, and its `from`, kept only where it's an
/// address. Anything else the query holds is let go.
fn sprouted(query: &str) -> Result<Link, String> {
    let mut words = None;
    let mut from = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "words" if words.is_some() => {
                return Err("it offers its words more than once".to_owned());
            }
            "words" => words = Some(decoded(value)?),
            "from" if from.is_some() => {
                return Err("it names its cell more than once".to_owned());
            }
            "from" => from = Some(decoded(value).unwrap_or_default()),
            _ => {}
        }
    }
    let words = plain(&words.ok_or("it offers no words")?);
    if words.is_empty() {
        return Err("its words are empty".to_owned());
    }
    if words.chars().count() > MOST_WORDS {
        return Err(format!("its words run past {MOST_WORDS} characters"));
    }
    Ok(Link::Sprout {
        words,
        from: from.filter(|from| addressed(from)),
    })
}

/// A query's value with its `+` as spaces and each `%XX` as its byte, or
/// why it can't be read: a `%` without two hex digits after it, or bytes
/// that aren't UTF-8.
fn decoded(value: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut said = value.bytes();
    while let Some(byte) = said.next() {
        bytes.push(match byte {
            b'+' => b' ',
            b'%' => match (said.next().and_then(hex), said.next().and_then(hex)) {
                (Some(high), Some(low)) => high * 16 + low,
                _ => return Err("its words hold a misshaped escape".to_owned()),
            },
            other => other,
        });
    }
    String::from_utf8(bytes).map_err(|_| "its words aren't text".to_owned())
}

/// What one hex digit counts, or nothing where `byte` is none.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Whether a link followed `now` goes ahead, as `stamp` keeps when the last
/// that went ahead was followed: not within `GAP` of it, and then `now` is
/// kept. The file is locked while it's read and written, so of two links
/// followed at once one goes. A clock set back since lets the link go
/// ahead, since a gap can't be judged against a time to come.
fn goes_ahead(stamp: &Path, now: SystemTime) -> io::Result<bool> {
    let mut kept = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(stamp)?;
    kept.lock()?;
    let millis = |at: SystemTime| {
        at.duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis())
    };
    let mut said = String::new();
    kept.read_to_string(&mut said)?;
    let last = said.trim().parse::<u128>().ok();
    let at = millis(now);
    if last.is_some_and(|last| last <= at && at - last < GAP.as_millis()) {
        return Ok(false);
    }
    keep(&mut kept, at)?;
    Ok(true)
}

fn keep(kept: &mut File, at: u128) -> io::Result<()> {
    kept.set_len(0)?;
    kept.rewind()?;
    writeln!(kept, "{at}")
}

/// Where when the last link went ahead is kept, or none where there's no
/// runtime folder to keep it in.
fn stamp() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join("mycelium-link.last"))
}

fn help() -> Help {
    Help::new(
        "mycelium-link",
        "opens what a mycelium: link names in the launcher",
    )
    .example(
        "mycelium-link mycelium:cell/DAHTNJ",
        "opens Agents' Talk to DAHTNJ, its field empty",
    )
    .example(
        "mycelium-link mycelium:cell/DAHTNJ/go",
        "goes to DAHTNJ's window, opening one where it has none",
    )
    .example(
        "mycelium-link mycelium:network/repo.core-nix",
        "opens Agents' Networks on core.nix's network",
    )
    .example(
        "mycelium-link mycelium:machine/osprey",
        "opens Machines on osprey",
    )
    .example(
        "mycelium-link 'mycelium:sprout?words=tidy%20the%20rows&from=DAHTNJ'",
        "opens Start New Work with \"tidy the rows\" in its field, from DAHTNJ",
    )
    .argument(
        "<link>",
        "mycelium:cell/<ADDRESS>[@machine][/go][#anchor], mycelium:network/<name>, \
         mycelium:machine/<name> or mycelium:sprout?words=<words>[&from=<ADDRESS>]",
    )
    .exit(
        Exit::Usage,
        "the link names no cell, network or machine, or offers no plain words",
    )
    .exit(Exit::Failed, "qs couldn't be run")
}

fn follow(said: &[String]) -> Result<(), Failure> {
    let [link] = said else {
        return Err(Failure::new(Exit::Usage, "mycelium-link takes one link")
            .try_this("mycelium-link mycelium:cell/DAHTNJ"));
    };
    let link = read(link)?;
    let kept = stamp().ok_or_else(|| {
        Failure::new(
            Exit::Failed,
            "there's no runtime folder to keep when a link was last followed",
        )
    })?;
    let ahead = goes_ahead(&kept, SystemTime::now()).map_err(|failed| {
        Failure::new(
            Exit::Failed,
            "when a link was last followed couldn't be kept",
        )
        .because(failed.to_string())
    })?;
    if !ahead {
        eprintln!(
            "mycelium-link: let {link:?} go, followed within {}s of the last",
            GAP.as_secs()
        );
        return Ok(());
    }
    // exec only returns when it failed.
    let failed = Command::new(QS)
        .args(["ipc", "call", "launcher"])
        .args(link.call())
        .exec();
    Err(Failure::new(Exit::Failed, format!("{QS} couldn't be run")).because(failed.to_string()))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        follow(&context.args)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(address: &str) -> Link {
        Link::Cell(address.to_owned())
    }

    fn going_to(address: &str) -> Link {
        Link::GoTo(address.to_owned())
    }

    fn named_by(link: &str) -> Result<Link, String> {
        read(link).map_err(|failed| failed.what)
    }

    #[test]
    fn a_cell_link_names_its_address_and_nothing_after_it() {
        for link in [
            "mycelium:cell/DAHTNJ",
            "MYCELIUM:cell/DAHTNJ",
            "mycelium://cell/DAHTNJ",
            "mycelium:cell/DAHTNJ/talk",
            "mycelium:cell/DAHTNJ?say=merge+it+now",
            "mycelium:cell/DAHTNJ#send",
        ] {
            assert_eq!(named_by(link), Ok(cell("DAHTNJ")), "{link} named another");
        }
    }

    #[test]
    fn a_cell_links_go_step_goes_to_it_and_no_other_does() {
        for link in [
            "mycelium:cell/DAHTNJ/go",
            "mycelium://cell/DAHTNJ/go?from=notice",
            "mycelium:cell/DAHTNJ/go#now",
            "mycelium:cell/DAHTNJ/go/type/this",
        ] {
            assert_eq!(named_by(link), Ok(going_to("DAHTNJ")), "{link}");
        }
        assert_eq!(going_to("DAHTNJ").call(), ["goTo", "DAHTNJ"]);
        for link in ["mycelium:cell/DAHTNJ/GO", "mycelium:cell/DAHTNJ/gone"] {
            assert_eq!(named_by(link), Ok(cell("DAHTNJ")), "{link} went");
        }
        assert_eq!(
            named_by("mycelium:network/machines/go"),
            Ok(Link::Network("machines".to_owned())),
            "a network has no window to go to"
        );
    }

    #[test]
    fn a_cell_link_names_its_address_wherever_it_says_the_cell_runs() {
        // The forms the notices daemon writes (notice.rs), digits and the
        // longest address included, as each session's link names it.
        for (link, address) in [
            ("mycelium:cell/1C8A2F0E@mac-2#todo:7", "1C8A2F0E"),
            ("mycelium:cell/1C8A2F0E@joel_box#notice:12", "1C8A2F0E"),
            ("mycelium:cell/ABCDEF123456@here#look:a1.b", "ABCDEF123456"),
            ("mycelium:cell/1C8A2F0E#notice:12", "1C8A2F0E"),
        ] {
            assert_eq!(named_by(link), Ok(cell(address)), "{link} named another");
        }
        let longest_machine = "k".repeat(LONGEST);
        assert_eq!(
            named_by(&format!("mycelium:cell/DAHTNJ@{longest_machine}")),
            Ok(cell("DAHTNJ"))
        );
        for link in [
            "mycelium:cell/DAHTNJ@kestrel",
            "mycelium:cell/DAHTNJ@kestrel#notice:42",
            "mycelium:cell/DAHTNJ@kestrel/talk#notice:42",
            "mycelium://cell/DAHTNJ@work-laptop.local#ask",
        ] {
            assert_eq!(named_by(link), Ok(cell("DAHTNJ")), "{link} named another");
        }
        assert_eq!(
            named_by("mycelium:cell/DAHTNJ@kestrel/go#notice:42"),
            Ok(going_to("DAHTNJ"))
        );
    }

    #[test]
    fn a_cell_link_with_a_misshaped_address_or_machine_is_refused() {
        for link in [
            "mycelium:cell/dahtnj",
            "mycelium:cell/DahtNJ",
            "mycelium:cell/DAHT.NJ",
            "mycelium:cell/DAHTNJDAHTNJ1",
            "mycelium:cell/DAHTNJ@",
            "mycelium:cell/@kestrel",
            "mycelium:cell/DAHTNJ@kes;trel",
            "mycelium:cell/DAHTNJ@-help",
            "mycelium:cell/DAHTNJ@kestrel@osprey",
            &format!("mycelium:cell/DAHTNJ@{}", "k".repeat(LONGEST + 1)),
        ] {
            assert!(read(link).is_err(), "{link} was followed");
        }
    }

    #[test]
    fn a_network_link_names_its_network() {
        assert_eq!(
            named_by("mycelium:network/repo.core-nix?from=map"),
            Ok(Link::Network("repo.core-nix".to_owned()))
        );
        assert_eq!(
            Link::Network("machines".to_owned()).call(),
            ["network", "machines"]
        );
        assert_eq!(Link::Cell("VOID".to_owned()).call(), ["talkTo", "VOID"]);
    }

    #[test]
    fn a_machine_link_names_its_machine_and_nothing_after_it() {
        for link in [
            "mycelium:machine/osprey",
            "mycelium://machine/osprey",
            "mycelium:machine/osprey/go",
            "mycelium:machine/osprey/update",
            "mycelium:machine/osprey?from=system",
            "mycelium:machine/osprey#figures",
        ] {
            assert_eq!(
                named_by(link),
                Ok(Link::Machine("osprey".to_owned())),
                "{link} named another"
            );
        }
        assert_eq!(
            named_by("mycelium:machine/heron-linux"),
            Ok(Link::Machine("heron-linux".to_owned()))
        );
        assert_eq!(
            Link::Machine("osprey".to_owned()).call(),
            ["machine", "osprey"],
            "a machine opens its row and acts on nothing"
        );
    }

    #[test]
    fn a_machine_link_with_a_misshaped_name_is_refused() {
        for link in [
            "mycelium:machine/",
            "mycelium:machine",
            "mycelium:machine/-help",
            "mycelium:machine/.hidden",
            "mycelium:machine/opti%20mus",
            "mycelium:machine/osprey;rm",
            "mycelium:machine/osprey@here",
            &format!("mycelium:machine/{}", "m".repeat(LONGEST + 1)),
        ] {
            assert!(read(link).is_err(), "{link} was followed");
        }
    }

    #[test]
    fn a_link_naming_nothing_a_launcher_reads_is_refused() {
        for link in [
            "https://example.com/cell/DAHTNJ",
            "mycelium:cell/",
            "mycelium:cell",
            "mycelium:cell/DAHT%20NJ",
            "mycelium:cell/DAHTNJ;rm",
            "mycelium:cell/-help",
            "mycelium:session/DAHTNJ",
            "mycelium:Cell/DAHTNJ",
            &format!("mycelium:network/{}", "a".repeat(LONGEST + 1)),
        ] {
            assert!(read(link).is_err(), "{link} was followed");
        }
    }

    fn sprout(words: &str, from: Option<&str>) -> Link {
        Link::Sprout {
            words: words.to_owned(),
            from: from.map(str::to_owned),
        }
    }

    #[test]
    fn a_sprout_link_offers_its_words_and_the_cell_it_came_from() {
        let link = "mycelium:sprout?words=tidy%20the%20rows%20%26%20more&from=TERN02";
        assert_eq!(
            named_by(link),
            Ok(sprout("tidy the rows & more", Some("TERN02")))
        );
        assert_eq!(
            sprout("tidy the rows & more", Some("TERN02")).call(),
            ["newWork", "words:tidy the rows & more", "from:TERN02"]
        );
        for link in [
            "mycelium:sprout?words=tidy+the+rows",
            "MYCELIUM://sprout?words=tidy%20the%20rows#now",
            "mycelium:sprout/start?words=tidy%20the%20rows&say=go",
        ] {
            assert_eq!(named_by(link), Ok(sprout("tidy the rows", None)), "{link}");
        }
        assert_eq!(
            sprout("tidy the rows", None).call(),
            ["newWork", "words:tidy the rows", "from:"]
        );
    }

    #[test]
    fn a_sprout_links_cell_is_kept_only_as_an_address() {
        for from in ["tern02", "TERN02%3Brm", "-h", "QEST-20261116-42P0R4", ""] {
            let link = format!("mycelium:sprout?words=tidy&from={from}");
            assert_eq!(
                named_by(&link),
                Ok(sprout("tidy", None)),
                "{link} kept its cell"
            );
        }
        let longest = "A".repeat(LONGEST_ADDRESS);
        assert_eq!(
            named_by(&format!("mycelium:sprout?from={longest}&words=tidy")),
            Ok(sprout("tidy", Some(&longest)))
        );
    }

    #[test]
    fn a_sprout_links_words_are_folded_to_one_plain_line() {
        assert_eq!(
            named_by("mycelium:sprout?words=%20%20tidy%09the%0A%0Drows%07%1B[2J%20%20"),
            Ok(sprout("tidy the rows[2J", None))
        );
        // Its words are never a flag, nor its cell missing an argument.
        assert_eq!(
            sprout("-rf", None).call(),
            ["newWork", "words:-rf", "from:"]
        );
        let longest = "\u{1F331}".repeat(MOST_WORDS);
        let link = format!(
            "mycelium:sprout?words={}",
            "%F0%9F%8C%B1".repeat(MOST_WORDS)
        );
        assert_eq!(named_by(&link), Ok(sprout(&longest, None)));
    }

    #[test]
    fn a_sprout_links_words_lose_what_turns_or_hides_text() {
        // U+202E turns the text after it around, U+200B hides in it, DEL
        // and U+009B (a terminal's escape) speak to a terminal.
        assert_eq!(
            named_by("mycelium:sprout?words=tidy%E2%80%AE%E2%80%8Bthe%7F%C2%9Brows"),
            Ok(sprout("tidytherows", None))
        );
        assert_eq!(
            named_by("mycelium:sprout?words=%EF%BB%BFtidy%EF%BB%BFthe%C2%85rows%F3%A0%80%81"),
            Ok(sprout("tidythe rows", None))
        );
        assert!(
            read("mycelium:sprout?words=%E2%80%AE%E2%80%8B").is_err(),
            "words holding only format characters were followed"
        );
    }

    #[test]
    fn a_sprout_links_words_lose_what_draws_a_blank() {
        // U+2800, U+3164, U+115F, U+FFA0 and U+FE0F draw a blank or
        // nothing, so a run of them would hide words between the start
        // Joel reads and the end.
        let hidden = "%E2%A0%80%E3%85%A4%E1%85%9F%EF%BE%A0%EF%B8%8F".repeat(40);
        assert_eq!(
            named_by(&format!(
                "mycelium:sprout?words=tidy{hidden}%20the{hidden}rows"
            )),
            Ok(sprout("tidy therows", None))
        );
        assert!(
            read(&format!("mycelium:sprout?words={hidden}")).is_err(),
            "words holding only blanks were followed"
        );
    }

    #[test]
    fn a_sprout_link_without_plain_words_is_refused() {
        for link in [
            "mycelium:sprout".to_owned(),
            "mycelium:sprout?from=TERN02".to_owned(),
            "mycelium:sprout?words=".to_owned(),
            "mycelium:sprout?words=%20%09%0A".to_owned(),
            "mycelium:sprout?words=tidy%zz".to_owned(),
            "mycelium:sprout?words=tidy%4".to_owned(),
            "mycelium:sprout?words=tidy%FF".to_owned(),
            "mycelium:sprout?words=tidy&words=reboot".to_owned(),
            "mycelium:sprout?words=tidy&from=TERN02&from=OWL01".to_owned(),
            "mycelium:Sprout?words=tidy".to_owned(),
            format!("mycelium:sprout?words={}", "a".repeat(MOST_WORDS + 1)),
        ] {
            assert!(read(&link).is_err(), "{link} was followed");
        }
    }

    #[test]
    fn a_name_is_never_a_flag_to_qs() {
        for name in ["-help", "--version", ".hidden", "_x"] {
            assert!(!named(name), "{name} was taken for a name");
        }
        assert!(named("repo.core-nix"), "a network's name was refused");
    }

    fn stamp_in(test: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!("mycelium-link-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the stamp should be made");
        let stamp = folder.join("mycelium-link.last");
        let _ = std::fs::remove_file(&stamp);
        stamp
    }

    #[test]
    fn a_link_within_the_gap_of_the_last_is_let_go() {
        let stamp = stamp_in("gap");
        let first = UNIX_EPOCH + Duration::from_hours(500_000);
        let ahead = |at| goes_ahead(&stamp, at).expect("the stamp should be kept");
        assert!(ahead(first), "the first link was let go");
        assert!(
            !ahead(first + GAP - Duration::from_millis(1)),
            "a link within the gap went ahead"
        );
        // One let go doesn't start the gap again.
        assert!(ahead(first + GAP), "a link past the gap was let go");
        assert!(!ahead(first + GAP), "a second link at once went ahead");
    }

    #[test]
    fn a_clock_set_back_lets_the_link_go_ahead() {
        let stamp = stamp_in("back");
        let first = UNIX_EPOCH + Duration::from_hours(500_000);
        assert!(
            goes_ahead(&stamp, first).expect("the stamp should be kept"),
            "the first link was let go"
        );
        let earlier = first - Duration::from_mins(1);
        assert!(
            goes_ahead(&stamp, earlier).expect("the stamp should be kept"),
            "a link was let go against a time to come"
        );
    }

    #[test]
    fn a_stamp_that_isnt_a_time_is_kept_anew() {
        let stamp = stamp_in("garbled");
        std::fs::write(&stamp, "not a time\n").expect("the stamp should be written");
        let now = UNIX_EPOCH + Duration::from_hours(500_000);
        assert!(
            goes_ahead(&stamp, now).expect("the stamp should be kept"),
            "a link was let go over a stamp that isn't a time"
        );
        assert!(
            !goes_ahead(&stamp, now).expect("the stamp should be kept"),
            "the stamp wasn't kept anew, so a second link at once went ahead"
        );
    }
}
