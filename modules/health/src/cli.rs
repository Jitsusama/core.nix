//! health's command line: its help, and what its words ask for once the
//! terminal kit has walked them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use terminal_kit::{Exit, Failure, Help, flags, time};

use crate::day::Day;
use crate::follow;
use crate::show::KINDS;

/// What the words ask for.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    /// Where the days are kept, if not the account's state folder.
    pub into: Option<PathBuf>,
    /// The day to read, if not today.
    pub day: Option<String>,
    pub view: View,
}

/// What's shown, or done.
#[derive(Debug, PartialEq, Eq)]
pub enum View {
    /// The daemon, keeping the day's file current.
    Follow,
    /// The day: its most painful interactions, then a line a kind.
    Day,
    /// A view by its name, or the things whose names hold the words.
    Words(String),
}

/// The flags health reads itself, each with its value.
const FLAGS: [&str; 2] = ["--day", "--into"];

/// health's help, with each of its views as a subcommand.
pub fn help() -> Help {
    let help = Help::new("health", "what the machine's journals said today")
        .example(
            "health",
            "the three most painful interactions, then a line a kind",
        )
        .example("health services", "the services that said most")
        .example("health chrome", "what each thing called chrome kept saying")
        .example(
            "health -- -x",
            "words that open with a dash, looked for too",
        )
        .example(
            "health boot --day 2026-11-12",
            "that day's boots, stage by stage",
        )
        .argument("[words]...", "what to look for in the names of what spoke")
        .subcommand(view(
            "boot",
            "each boot's stages, and each shutdown's seconds",
        ))
        .subcommand(view(
            "memory",
            "what pi and the shell hold, by each copy's age",
        ))
        .subcommand(view(
            "died",
            "the agents that died, and those gone before today",
        ))
        .subcommand(view(
            "launcher",
            "each part of the launcher against its budget",
        ))
        .subcommand(view(
            "launches",
            "each program's openings and ends, slowest first",
        ))
        .subcommand(view("figures", "each figure cadence holds the day to"))
        .subcommand(view(
            "busy",
            "whose work kept the processors busy, by the hour",
        ));
    KINDS
        .iter()
        .fold(help, |help, kind| {
            help.subcommand(view(kind.name(), kind_words(kind.name())))
        })
        .subcommand(
            Help::new("follow", "the daemon, which keeps the day's file current")
                .example("health follow", "as its unit runs it")
                .option("--into FOLDER", "where the days are kept")
                .exit(
                    Exit::Failed,
                    "a journal couldn't be followed, or the day kept",
                ),
        )
        .option("--day DATE", "another day, as 2026-11-12")
        .option(
            "--into FOLDER",
            "where the days are kept, ~/.local/state/health",
        )
        .exit(Exit::NotFound, "nothing was kept for the day")
        .exit(Exit::BadInput, "the day's file can't be read")
        .exit(Exit::Failed, "the day's file couldn't be opened")
}

/// One of the day's views, read from its file as the day is. Words after
/// its name are looked for with it, as `health launcher shell` always
/// was, rather than refused.
fn view(name: &str, words: &str) -> Help {
    Help::new(name, words)
        .example(&format!("health {name}"), "today's")
        .example(&format!("health {name} --day 2026-11-12"), "that day's")
        .argument(
            "[more]...",
            "looked for with its name in the names of what spoke, not shown as the view",
        )
        .option("--day DATE", "another day, as 2026-11-12")
        .exit(Exit::NotFound, "nothing was kept for the day")
        .exit(Exit::BadInput, "the day's file can't be read")
        .exit(Exit::Failed, "the day's file couldn't be opened")
}

/// What a kind's view lists.
fn kind_words(kind: &str) -> &'static str {
    match kind {
        "words" => "what Joel typed that sounds like something hurt",
        "programs" => "the programs that said most, crashes among them",
        "services" => "the services that said most",
        "devices" => "the machine's parts that said most",
        "security" => "what sudo, polkit and the logins said most",
        _ => "the agents' tools that failed, were refused or repeated",
    }
}

/// What the walked words ask for.
///
/// # Errors
///
/// Asked wrongly, 64, for a flag health doesn't read or one without its
/// value.
pub fn read(words: &[String]) -> Result<Asked, Failure> {
    let mut asked = Asked {
        into: None,
        day: None,
        view: View::Day,
    };
    let mut rest = Vec::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        // Past `--`, every word is looked for, a dash before it or not.
        if word == "--" {
            rest.extend(words.by_ref().cloned());
            break;
        }
        // The kit reads these itself, to say a failure in full.
        if word == "-v" || word == "--verbose" {
            continue;
        }
        // A lone `-` is a word, as the kit's walk takes it.
        if !word.starts_with('-') || word.len() == 1 {
            rest.push(word.clone());
            continue;
        }
        let (flag, joined) = word
            .split_once('=')
            .map_or((word.as_str(), None), |(flag, value)| {
                (flag, Some(value.to_owned()))
            });
        if !FLAGS.contains(&flag) {
            return Err(flags::unknown("flag", flag, &FLAGS).try_this("health --help"));
        }
        let value = joined.or_else(|| words.next().cloned()).ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{flag} needs a value")).try_this("health --help")
        })?;
        if flag == "--day" {
            asked.day = Some(value);
        } else {
            asked.into = Some(PathBuf::from(value));
        }
    }
    asked.view = match rest.as_slice() {
        [] => View::Day,
        // The walk took it as the daemon's subcommand, which takes no
        // more words, so it's alone.
        [only] if only == "follow" => View::Follow,
        _ => View::Words(rest.join(" ")),
    };
    Ok(asked)
}

/// The day kept in `into` for `date`.
///
/// # Errors
///
/// 66 when nothing was kept for it, and 65 when its file can't be read.
pub fn day_in(into: &Path, date: &str) -> Result<Day, Failure> {
    let file = fs::read(follow::file_of(into, date)).map_err(|fault| {
        if fault.kind() == io::ErrorKind::NotFound {
            Failure::new(Exit::NotFound, format!("nothing was kept for {date}"))
                .because("its daemon may not be running")
                .try_this("systemctl --user status health")
        } else {
            Failure::new(Exit::Failed, format!("{date}'s file couldn't be opened"))
                .because(fault.to_string())
        }
    })?;
    serde_json::from_slice(&file).map_err(|fault| {
        Failure::new(Exit::BadInput, format!("{date}'s file can't be read"))
            .because(fault.to_string())
    })
}

/// How long today's file may go unwritten before `health` says so. A
/// running daemon writes it every half minute, since it minds memory at
/// each look, so this long means its writes fail, as on a full disk, or
/// it isn't running. A machine just woken is said so too until health's
/// first look, within half a minute, since the file's clock went on.
const STALE_AFTER: Duration = Duration::from_mins(3);

/// What `health` says of the day `date` kept in `into` when it's `today`
/// and its file was last written `STALE_AFTER` or more before `now`, so
/// what it shows has stood still: nothing for another day, which is done,
/// or a file it can't time. A file timed past `now`, as after the clock
/// went back, is said nothing of, since how long it stood can't be told;
/// a running health writes it again within half a minute. Across a
/// change of time zone, before health starts again in the new one, today
/// here may be a day it wrote in the old, which is then said to stand.
pub fn stale(into: &Path, date: &str, today: &str, now: SystemTime) -> Option<String> {
    if date != today {
        return None;
    }
    let written = fs::metadata(follow::file_of(into, date))
        .ok()?
        .modified()
        .ok()?;
    let since = now.duration_since(written).ok()?;
    (since >= STALE_AFTER).then(|| {
        format!(
            "{date} was last written {} ago; health may not be running, or can't write it",
            time::span(since.as_secs())
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::tree::walk;

    /// The words a line holds, walked as the kit walks them.
    fn asked(line: &str) -> Result<Asked, Failure> {
        let words: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        let help = help();
        let walked = walk(&help, &words)?;
        read(&walked.words)
    }

    fn view(line: &str) -> View {
        asked(line).map_or_else(
            |failure| panic!("{line} was refused: {}", failure.what),
            |asked| asked.view,
        )
    }

    /// The command the kit's walk took a line to, as `health boot`.
    fn path(line: &str) -> String {
        let words: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        let help = help();
        walk(&help, &words).map_or_else(
            |failure| format!("refused: {}", failure.what),
            |walked| walked.path(),
        )
    }

    #[test]
    fn each_view_is_walked_to_by_its_name_and_its_help_is_its_own() {
        for (line, to) in [
            ("boot", "health boot"),
            ("services", "health services"),
            ("follow", "health follow"),
            ("--into /kept boot", "health boot"),
            ("boot --day 2026-11-12", "health boot"),
            ("chrome", "health"),
            ("-- boot", "health"),
        ] {
            assert_eq!(path(line), to, "{line} was walked elsewhere");
        }
    }

    #[test]
    fn each_view_answers_to_its_name_and_other_words_are_looked_for() {
        assert_eq!(view(""), View::Day, "a bare health didn't show the day");
        assert_eq!(
            view("boot"),
            View::Words("boot".into()),
            "boot wasn't its view"
        );
        assert_eq!(
            view("services"),
            View::Words("services".into()),
            "a kind wasn't its view"
        );
        assert_eq!(
            view("chrome"),
            View::Words("chrome".into()),
            "a name wasn't looked for"
        );
        assert_eq!(
            view("pi agent"),
            View::Words("pi agent".into()),
            "words weren't looked for together"
        );
        assert_eq!(view("follow"), View::Follow, "follow wasn't the daemon");
        assert_eq!(
            view("launcher shell"),
            View::Words("launcher shell".into()),
            "words after a view's name weren't looked for with it"
        );
        assert_eq!(
            view("-- -x boot"),
            View::Words("-x boot".into()),
            "words past -- weren't looked for whole"
        );
    }

    #[test]
    fn the_day_and_the_folder_are_read_wherever_they_are_typed() {
        let both = asked("--into /kept boot --day 2026-11-12").expect("the flags were taken");
        assert_eq!(
            both.into,
            Some(PathBuf::from("/kept")),
            "the folder was lost"
        );
        assert_eq!(both.day.as_deref(), Some("2026-11-12"), "the day was lost");
        assert_eq!(both.view, View::Words("boot".into()), "the view was lost");
        let loud = asked("boot -v").expect("the kit's -v was refused");
        assert_eq!(loud.view, View::Words("boot".into()), "-v was looked for");
        let joined = asked("--day=2026-11-11").expect("a flag with = was taken");
        assert_eq!(
            joined.day.as_deref(),
            Some("2026-11-11"),
            "the day with = was lost"
        );
    }

    #[test]
    fn a_flag_it_doesnt_read_or_without_its_value_is_asked_wrongly() {
        for line in ["--dya 2026-11-12", "boot --day", "follow again"] {
            let failure = asked(line)
                .err()
                .unwrap_or_else(|| panic!("{line} was taken"));
            assert_eq!(failure.exit, Exit::Usage, "{line} ended {:?}", failure.exit);
        }
        let failure = asked("--dya 2026-11-12").expect_err("a wrong flag was taken");
        assert!(
            failure.why.as_deref() == Some("did you mean --day?"),
            "a wrong flag didn't name the nearest: {failure:?}"
        );
    }

    #[test]
    fn a_day_never_kept_is_not_found_and_a_broken_one_is_bad_input() {
        let kept = tempfile::tempdir().expect("a folder for the days");
        let gone = day_in(kept.path(), "2026-11-12").expect_err("a missing day was read");
        assert_eq!(
            gone.exit,
            Exit::NotFound,
            "a missing day ended {:?}",
            gone.exit
        );
        assert!(
            gone.what.contains("2026-11-12"),
            "it didn't name the day: {}",
            gone.what
        );
        fs::write(follow::file_of(kept.path(), "2026-11-12"), "{").expect("a broken day");
        let broken = day_in(kept.path(), "2026-11-12").expect_err("a broken day was read");
        assert_eq!(
            broken.exit,
            Exit::BadInput,
            "a broken day ended {:?}",
            broken.exit
        );
        fs::write(
            follow::file_of(kept.path(), "2026-11-11"),
            serde_json::to_vec(&Day::default()).expect("a day as JSON"),
        )
        .expect("a kept day");
        assert!(
            day_in(kept.path(), "2026-11-11").is_ok(),
            "a kept day wasn't read"
        );
        fs::create_dir(follow::file_of(kept.path(), "2026-11-10")).expect("a folder for a day");
        let unopened = day_in(kept.path(), "2026-11-10").expect_err("a folder was read as a day");
        assert_eq!(
            unopened.exit,
            Exit::Failed,
            "a day that couldn't be opened ended {:?}",
            unopened.exit
        );
    }

    #[test]
    fn only_today_written_minutes_ago_is_said_to_have_stood_still() {
        let kept = tempfile::tempdir().expect("a folder for the days");
        let written = SystemTime::UNIX_EPOCH + Duration::from_hours(498_468);
        // The day before is kept too, written as long ago, so only its
        // date can keep it from being said to stand still.
        for date in ["2026-11-11", "2026-11-12"] {
            let file = follow::file_of(kept.path(), date);
            fs::write(&file, "{}").expect("a kept day");
            fs::File::options()
                .write(true)
                .open(&file)
                .and_then(|open| open.set_modified(written))
                .expect("the day's time set");
        }
        let after = |seconds| written + Duration::from_secs(seconds);
        let today = |date, seconds| stale(kept.path(), date, "2026-11-12", after(seconds));
        assert_eq!(
            [
                today("2026-11-12", 179),
                today("2026-11-12", 180),
                today("2026-11-12", 540),
                today("2026-11-11", 540),
            ],
            [
                None,
                Some(
                    "2026-11-12 was last written 3 m ago; health may not be running, or can't \
                     write it"
                        .to_owned()
                ),
                Some(
                    "2026-11-12 was last written 9 m ago; health may not be running, or can't \
                     write it"
                        .to_owned()
                ),
                None,
            ],
            "a day written within three minutes, or a day done, was said to stand still, or \
             today written nine minutes ago wasn't"
        );
        assert_eq!(
            [
                stale(kept.path(), "2026-11-13", "2026-11-13", after(540)),
                stale(
                    kept.path(),
                    "2026-11-12",
                    "2026-11-12",
                    written - STALE_AFTER * 2
                ),
            ],
            [None, None],
            "a day never kept, or one timed past now, was said to stand still"
        );
    }
}
