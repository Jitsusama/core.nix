//! zathura's commands through the launcher's command line, as mpv's and swayimg's are.
//!
//! zathura's `:` runs `zathura-command line BUS`, given the window's bus name (`$DBUS`),
//! which asks that zathura what it shows and opens the launcher's command line over it,
//! named for the document. What's chosen there comes back as `zathura-command run BUS
//! COMMAND [REST]`: a page, `:41`, as `goto` and its number, which goes there through zathura's
//! `GotoPage`, `$` the last, a number past the last going to it as Vim's does to a line;
//! anything else is run as zathura's own command line would run it, through its
//! `ExecuteCommand`, so every command zathura has still runs, saying what went wrong
//! in zathura's window as its own line does.
//!
//! Usage: `zathura-command line BUS` | `zathura-command run BUS COMMAND [REST]`
//!
//! The older `zathura-command BUS COMMAND [REST]`, from before `run` had its word,
//! still runs, unlisted, for a caller not yet moved.
//!
//! It takes its help, exits and failures from the terminal kit
//! (docs/commands.md); it answers nothing, since what it does shows in
//! zathura's window or the launcher.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use terminal_kit::{Exit, Failure, Help};

/// zathura's object on its bus.
const OBJECT: &str = "/org/pwmt/zathura";

/// Where a page goes, as the launcher's `:{n}` and `:$` give it.
#[derive(Debug, PartialEq, Eq)]
enum Goto {
    /// A page by its number, counted from one.
    Page(u32),
    /// The last page.
    Last,
}

/// What this was asked to do.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    /// Open the launcher's command line over the zathura on this bus.
    Line(String),
    /// Go to a page in the zathura on this bus.
    Goto(String, Goto),
    /// Run a line of zathura's own commands in the zathura on this bus.
    Run(String, String),
}

/// The help the older form, a bus name first, is read under: no subcommands, so
/// the kit leaves the bus name to `asked`.
fn older_help() -> Help {
    Help::new(
        "zathura-command",
        "carries the launcher's command line to a zathura",
    )
    .example("zathura-command line :1.42", "what zathura's colon runs")
    .example("zathura-command run :1.42 goto 41", "goes to page 41")
    .exit(
        Exit::Failed,
        "zathura or the launcher couldn't be asked, or said no",
    )
}

fn help() -> Help {
    older_help()
        .subcommand(
            Help::new("line", "opens the launcher's line over that zathura")
                .argument("<bus>", "the zathura's bus name, its $DBUS")
                .example("zathura-command line :1.42", "what zathura's colon runs")
                .exit(Exit::Failed, "the launcher couldn't be asked, or said no"),
        )
        .subcommand(
            Help::new("run", "runs one of zathura's commands there")
                .argument("<bus>", "the zathura's bus name, its $DBUS")
                .argument("<command>", "zathura's command; goto, a page")
                .argument("[rest]", "what follows it, as one word")
                .example("zathura-command run :1.42 goto 41", "goes to page 41")
                .example("zathura-command run :1.42 goto '$'", "to the last")
                .exit(Exit::Failed, "zathura couldn't be asked, or said no"),
        )
}

/// Whether a word is a bus name, a unique one as `:1.42` or a well-known one as
/// `org.pwmt.zathura.PID-7`, as the older form gives first.
fn is_bus(word: &str) -> bool {
    word.starts_with(':') || word.contains('.')
}

fn main() -> ExitCode {
    let older = std::env::args().nth(1).is_some_and(|first| is_bus(&first));
    let help = if older { older_help() } else { help() };
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let asked = asked(&context.args)
            .map_err(|said| Failure::new(Exit::Usage, said).try_this("zathura-command --help"))?;
        match &asked {
            Asked::Line(bus) => line(bus),
            Asked::Goto(bus, to) => went(bus, to),
            Asked::Run(bus, typed) => ran(bus, typed),
        }
        .map_err(|said| Failure::new(Exit::Failed, said))
    })
}

/// What the words given ask for.
fn asked(arguments: &[String]) -> Result<Asked, String> {
    match arguments {
        [first, bus] if first == "line" => Ok(Asked::Line(bus.clone())),
        [first, rest @ ..] if first == "run" => ran_of(rest),
        older => ran_of(older),
    }
}

/// What `run`'s words ask for: a page, or a line of zathura's own.
fn ran_of(arguments: &[String]) -> Result<Asked, String> {
    match arguments {
        [bus, command, rest @ ..] if command == "goto" => {
            let to = rest.first().map_or("", |to| to.trim());
            goto_of(to).map(|to| Asked::Goto(bus.clone(), to))
        }
        [bus, command, rest @ ..] if !command.is_empty() && rest.len() <= 1 => {
            let rest = rest.first().map_or("", |rest| rest.trim());
            Ok(Asked::Run(bus.clone(), typed(command, rest)))
        }
        _ => Err("it takes line BUS, or run BUS COMMAND [REST]".into()),
    }
}

/// The page `:{n}` or `:$` names; a time, as a player takes, names none.
fn goto_of(to: &str) -> Result<Goto, String> {
    if to == "$" {
        return Ok(Goto::Last);
    }
    to.parse()
        .map(Goto::Page)
        .map_err(|_| format!("zathura goes to a page by its number, not {to}"))
}

/// The line zathura's own command line would have been typed: the command, then what
/// followed it, if anything.
fn typed(command: &str, rest: &str) -> String {
    if rest.is_empty() {
        command.to_owned()
    } else {
        format!("{command} {rest}")
    }
}

/// The page `to` lands on in a document of `pages`, counted from nought as `GotoPage`
/// takes it: :0 and :1 the first, as in Vim, and one past the last the last. None in a
/// document with no pages.
fn page_index(to: &Goto, pages: u32) -> Option<u32> {
    let last = pages.checked_sub(1)?;
    Some(match to {
        Goto::Page(page) => page.saturating_sub(1).min(last),
        Goto::Last => last,
    })
}

/// Opens the launcher's command line over the zathura on `bus`, named for the document
/// it shows, or for nothing while it shows none.
fn line(bus: &str) -> Result<(), String> {
    let shown = property(bus, "filename").and_then(|said| string_in(&said));
    let name = shown
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let status = Command::new("launcher-say")
        .args(["command-line", "zathura", bus, &name])
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("the launcher couldn't be asked: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("the launcher didn't open its command line".into())
    }
}

/// Goes to the page `to` names in the zathura on `bus`.
fn went(bus: &str, to: &Goto) -> Result<(), String> {
    let pages = property(bus, "numberofpages")
        .and_then(|said| number_in(&said))
        .ok_or_else(|| format!("zathura on {bus} said no document"))?;
    let page =
        page_index(to, pages).ok_or_else(|| format!("zathura on {bus} has no page to go to"))?;
    called(bus, "GotoPage", &page.to_string())
}

/// Runs a line of zathura's own commands in the zathura on `bus`.
fn ran(bus: &str, typed: &str) -> Result<(), String> {
    called(bus, "ExecuteCommand", &quoted(typed))
}

/// Calls zathura's `method` on `bus` with its one argument, as `GVariant`'s text, and
/// whether it said it did.
fn called(bus: &str, method: &str, argument: &str) -> Result<(), String> {
    let answer = Command::new(env!("GDBUS"))
        .args(["call", "--session", "--dest", bus, "--object-path", OBJECT])
        .args(["--method", &format!("org.pwmt.zathura.{method}"), "--"])
        .arg(argument)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("gdbus couldn't be run: {error}"))?;
    let said = String::from_utf8_lossy(&answer.stdout);
    if answer.status.success() && said.contains("true") {
        Ok(())
    } else {
        Err(format!("zathura on {bus} didn't {method} {argument}"))
    }
}

/// gdbus's answer for one of zathura's properties on `bus`, or none when it can't be read.
fn property(bus: &str, name: &str) -> Option<String> {
    let answer = Command::new(env!("GDBUS"))
        .args(["call", "--session", "--dest", bus, "--object-path", OBJECT])
        .args(["--method", "org.freedesktop.DBus.Properties.Get"])
        .args(["org.pwmt.zathura", name])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    answer
        .status
        .success()
        .then(|| String::from_utf8_lossy(&answer.stdout).into_owned())
}

/// The number in gdbus's answer for a `u` property, as `(<uint32 212>,)`.
fn number_in(answer: &str) -> Option<u32> {
    let after = &answer[answer.find("uint32 ")? + "uint32 ".len()..];
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The string in gdbus's answer for an `s` property, as `(<'/d/a.pdf'>,)`, or none when
/// it's empty. `GVariant` quotes a string holding `'` in `"` instead.
fn string_in(answer: &str) -> Option<String> {
    let after = &answer[answer.find('<')? + 1..];
    let mut characters = after.chars();
    let quote = characters
        .next()
        .filter(|first| *first == '\'' || *first == '"')?;
    let mut text = String::new();
    while let Some(character) = characters.next() {
        match character {
            '\\' => text.push(characters.next()?),
            ended if ended == quote => return Some(text).filter(|text| !text.is_empty()),
            _ => text.push(character),
        }
    }
    None
}

/// `text` as a string in `GVariant`'s text, which gdbus reads its arguments as.
fn quoted(text: &str) -> String {
    let mut written = String::from("'");
    for character in text.chars() {
        if character == '\'' || character == '\\' {
            written.push('\\');
        }
        written.push(character);
    }
    written.push('\'');
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUS: &str = "org.pwmt.zathura.PID-42";

    fn words(said: &[&str]) -> Vec<String> {
        said.iter().map(|&word| word.to_owned()).collect()
    }

    #[test]
    fn colon_opens_the_launchers_line_over_its_window() {
        assert_eq!(asked(&words(&["line", BUS])), Ok(Asked::Line(BUS.into())));
    }

    #[test]
    fn a_page_and_the_last_are_gone_to() {
        assert_eq!(
            asked(&words(&["run", BUS, "goto", "41"])),
            Ok(Asked::Goto(BUS.into(), Goto::Page(41)))
        );
        assert_eq!(
            asked(&words(&["run", BUS, "goto", "$"])),
            Ok(Asked::Goto(BUS.into(), Goto::Last))
        );
    }

    #[test]
    fn a_time_is_no_page() {
        assert!(asked(&words(&["run", BUS, "goto", "12:30"])).is_err());
        assert!(asked(&words(&["run", BUS, "goto"])).is_err());
    }

    #[test]
    fn any_other_command_runs_as_zathuras_own_line_would() {
        assert_eq!(
            asked(&words(&["run", BUS, "set", "recolor false"])),
            Ok(Asked::Run(BUS.into(), "set recolor false".into()))
        );
        assert_eq!(
            asked(&words(&["run", BUS, "blist", ""])),
            Ok(Asked::Run(BUS.into(), "blist".into()))
        );
        assert_eq!(
            asked(&words(&["run", BUS, "write!", " ~/kept.pdf "])),
            Ok(Asked::Run(BUS.into(), "write! ~/kept.pdf".into()))
        );
    }

    #[test]
    fn too_little_or_too_much_is_misused() {
        assert!(asked(&words(&[])).is_err());
        assert!(asked(&words(&["run", BUS])).is_err());
        assert!(asked(&words(&["run", BUS, "set", "a", "b"])).is_err());
        assert!(asked(&words(&["line"])).is_err());
    }

    #[test]
    fn the_older_form_asks_as_run_does() {
        for said in [
            &["goto", "41"][..],
            &["goto", "$"],
            &["set", "recolor false"],
            &["blist", ""],
            &["goto", "12:30"],
        ] {
            let older = [&[BUS][..], said].concat();
            let run = [&["run", BUS][..], said].concat();
            assert_eq!(
                asked(&words(&older)),
                asked(&words(&run)),
                "{older:?} should ask what {run:?} does"
            );
        }
    }

    #[test]
    fn a_bus_name_is_told_from_a_command() {
        assert!(is_bus(BUS), "a well-known name is a bus");
        assert!(is_bus(":1.42"), "a unique name is a bus");
        for word in ["line", "run", "lien"] {
            assert!(!is_bus(word), "{word} should be read as a command");
        }
    }

    #[test]
    fn pages_are_counted_from_nought_and_kept_inside_the_document() {
        assert_eq!(page_index(&Goto::Page(41), 212), Some(40));
        assert_eq!(page_index(&Goto::Page(1), 212), Some(0));
        assert_eq!(page_index(&Goto::Page(0), 212), Some(0));
        assert_eq!(page_index(&Goto::Page(500), 212), Some(211));
        assert_eq!(page_index(&Goto::Last, 212), Some(211));
        assert_eq!(page_index(&Goto::Last, 0), None);
    }

    #[test]
    fn a_count_is_read_from_gdbus() {
        assert_eq!(number_in("(<uint32 212>,)\n"), Some(212));
        assert_eq!(number_in("(<'a'>,)\n"), None);
    }

    #[test]
    fn the_name_shown_is_read_from_gdbus() {
        assert_eq!(string_in("(<'/d/a.pdf'>,)\n"), Some("/d/a.pdf".to_owned()));
        assert_eq!(
            string_in("(<\"/d/it's.pdf\">,)\n"),
            Some("/d/it's.pdf".to_owned())
        );
        assert_eq!(string_in("(<''>,)\n"), None);
    }

    #[test]
    fn a_line_is_quoted_for_gdbus() {
        assert_eq!(quoted(r"exec echo 'a\b'"), r"'exec echo \'a\\b\''");
    }
}
