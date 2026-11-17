//! What a `mycelium:` link opens, run by the browser on each one Joel follows
//! (home.nix's desktop entry): `mycelium:cell/<ADDRESS>` opens Agents' Talk
//! to that session, its field empty, as `qs ipc call launcher talkTo` does,
//! and `mycelium:network/<name>` opens Agents' Networks on that network's
//! row, selected, as `qs ipc call launcher network` does. The realm's map
//! links its cells and networks so (XUBK2E, 2026-11-15 [2b5c927a]).
//! `mycelium:cell/<ADDRESS>/go` goes to that session instead, its window
//! raised or one opened on it, as `qs ipc call launcher goTo` does, which a
//! notice from a session offers as its Go To.
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

/// How long after a link is followed another is let go: longer than a
/// double click or a page's burst of them, shorter than Joel following a
/// second one by hand.
const GAP: Duration = Duration::from_secs(3);

/// The longest name a link carries: an address is eight letters at most and
/// a network's name a few dozen, so anything past this is no name.
const LONGEST: usize = 64;

/// `qs`, by the path the package was built with, or found on PATH.
const QS: &str = match option_env!("MYCELIUM_LINK_QS") {
    Some(path) => path,
    None => "qs",
};

/// What a link names: a cell to talk to or to go to, by its session's
/// address, or a network, by its name, each as the mesh says it.
#[derive(Debug, PartialEq, Eq)]
enum Link {
    Cell(String),
    GoTo(String),
    Network(String),
}

impl Link {
    /// The launcher's ipc call opening what it names.
    fn call(&self) -> [&str; 2] {
        match self {
            Self::Cell(address) => ["talkTo", address],
            Self::GoTo(address) => ["goTo", address],
            Self::Network(name) => ["network", name],
        }
    }
}

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

/// What `link` names, or what's wrong with it: its scheme in any case, with
/// or without the slashes a page may add, then its kind and its name, and
/// for a cell `go` after it; what else follows the name, a step, a query or
/// a fragment, is let go.
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
    let path = rest.trim_start_matches('/');
    let path = path.split(['?', '#']).next().unwrap_or("");
    let mut steps = path.split('/');
    let kind = steps.next().unwrap_or("");
    let name = steps.next().unwrap_or("");
    if !named(name) {
        return Err(wrong(
            "it names no cell or network in letters, digits, dots, hyphens or underscores",
        ));
    }
    match kind {
        "cell" if steps.next() == Some("go") => Ok(Link::GoTo(name.to_owned())),
        "cell" => Ok(Link::Cell(name.to_owned())),
        "network" => Ok(Link::Network(name.to_owned())),
        _ => Err(wrong("its kind is neither cell nor network")),
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
    .argument(
        "<link>",
        "mycelium:cell/<address>[/go] or mycelium:network/<name>",
    )
    .exit(Exit::Usage, "the link names no cell or network")
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
