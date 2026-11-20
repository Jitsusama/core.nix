//! `nix-offload outside [--git-dir DIR --work-tree DIR] [--lent-only] --
//! there WORD...`: runs a `there` call in the environment the agent
//! fence's exemption gives one, and nothing else.
//!
//! The fence lets one `nix-offload there` call out, since its ssh reaches
//! the lent builders with a key the fence hides, and gives it an
//! environment of its own: the home and runtime folders, the account's
//! name, the system's own `PATH`, git told which folder to read so it
//! never reads a `.git` a fenced command wrote, `core.fsmonitor` off, and
//! `NIX_OFFLOAD_LENT_ONLY` for a tree's commit, so it reaches a lent
//! builder or nothing. Those terms were the harness's to build; here they
//! are built once, for the harness and a Mac's gate (`gate.rs`) alike, so
//! the exemption ends for both on [`ENDS`] and any tightening reaches both
//! (security review, 15 November).
//!
//! It grants nothing by itself. A fenced command that runs it stays
//! fenced, without the key, and reaches no builder: the exemption is the
//! harness running it outside the fence, or mycelium starting a gate's
//! `answer` as the person. This only says how such a call is run.

use std::env;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use terminal_kit::{Exit, Failure};

/// The day the exemption is due out, as the harness's
/// `OFFLOAD_EXEMPTION_ENDS`: a broker holds the builders' key by then, so
/// nothing goes outside the fence to reach them. From this day `outside`
/// runs nothing.
pub const ENDS: (i64, u32, u32) = (2026, 11, 28);

/// A git folder no account can make, for a call outside any tree, so no
/// git it runs reads a `.git` someone else wrote.
pub const NO_GIT_DIR: &str = "/var/empty/no-git";

/// What `outside` exits with past [`ENDS`]: the table's refused, 77.
const ENDED: Exit = Exit::Refused;

/// A tree whose commit a call sends, by its git folder and its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree<'a> {
    /// Its git folder, the only one git reads.
    pub git_dir: &'a str,
    /// The tree itself, where the call runs.
    pub work_tree: &'a str,
}

/// Who runs the call, from the environment it's started in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account<'a> {
    /// The home folder.
    pub home: &'a str,
    /// The account's name, when there is one.
    pub user: Option<&'a str>,
    /// `XDG_RUNTIME_DIR`, where the Nix daemon's doors are.
    pub runtime: Option<&'a str>,
}

/// The `PATH` of the exemption: the system's own programs, the account's
/// profile before them, which only root writes.
fn system_path(user: Option<&str>) -> String {
    let own = user
        .filter(|user| plain_account(user))
        .map(|user| format!("/etc/profiles/per-user/{user}/bin"));
    own.into_iter()
        .chain(std::iter::once("/run/current-system/sw/bin".to_owned()))
        .collect::<Vec<_>>()
        .join(":")
}

/// Whether an account's name is one a folder may be named by.
fn plain_account(user: &str) -> bool {
    let mut bytes = user.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == b'_')
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

/// The whole environment a call outside is given, in order, and nothing
/// from the shell that asked. `NIX_CONFIG` turns off a flake's own
/// settings, so a commit's `nixConfig` adds no substituter or setting for
/// an account Nix may trust (security review, 15 November).
#[must_use]
pub fn environment(
    account: &Account,
    tree: Option<&Tree>,
    lent_only: bool,
) -> Vec<(String, String)> {
    let pair = |key: &str, value: &str| (key.to_owned(), value.to_owned());
    let mut set = vec![
        pair("HOME", account.home),
        pair("PATH", &system_path(account.user)),
    ];
    if let Some(user) = account.user {
        set.extend([pair("USER", user), pair("LOGNAME", user)]);
    }
    if let Some(runtime) = account.runtime {
        set.push(pair("XDG_RUNTIME_DIR", runtime));
    }
    match tree {
        Some(tree) => set.extend([
            pair("GIT_DIR", tree.git_dir),
            pair("GIT_WORK_TREE", tree.work_tree),
        ]),
        None => set.extend([
            pair("GIT_DIR", NO_GIT_DIR),
            pair("GIT_CEILING_DIRECTORIES", "/"),
        ]),
    }
    set.extend([
        pair("GIT_CONFIG_COUNT", "1"),
        pair("GIT_CONFIG_KEY_0", "core.fsmonitor"),
        pair("GIT_CONFIG_VALUE_0", "false"),
        pair("NIX_CONFIG", "accept-flake-config = false"),
    ]);
    if lent_only {
        set.push(pair("NIX_OFFLOAD_LENT_ONLY", "1"));
    }
    set
}

/// The civil date of a day counted from 1970-01-01, as (year, month, day).
#[must_use]
pub fn civil(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's days_from_civil, run backwards.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * month_shifted + 2) / 5 + 1;
    let month = if month_shifted < 10 {
        month_shifted + 3
    } else {
        month_shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        year,
        u32::try_from(month).expect("a month is 1 to 12"),
        u32::try_from(day).expect("a day is 1 to 31"),
    )
}

/// Whether the exemption has ended on `today`, its last day the one
/// before [`ENDS`].
#[must_use]
pub fn ended(today: (i64, u32, u32)) -> bool {
    today >= ENDS
}

/// Today in UTC.
fn today() -> (i64, u32, u32) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    civil(seconds.div_euclid(86_400))
}

/// What a call names: its tree, whether it stays on the lent builders,
/// and the `there` words it runs.
#[derive(Debug, PartialEq, Eq)]
struct Asked<'a> {
    tree: Option<Tree<'a>>,
    lent_only: bool,
    words: &'a [String],
}

/// Reads `outside`'s arguments, or says what's wrong with them.
fn asked(arguments: &[String]) -> Result<Asked<'_>, String> {
    let (mut git_dir, mut work_tree, mut lent_only) = (None, None, false);
    let mut at = 0;
    while let Some(word) = arguments.get(at) {
        match word.as_str() {
            "--" => break,
            "--lent-only" => lent_only = true,
            "--git-dir" | "--work-tree" => {
                let value = arguments
                    .get(at + 1)
                    .ok_or_else(|| format!("{word} needs a folder"))?;
                if !Path::new(value).is_absolute() {
                    return Err(format!("{word} takes a folder from the root, not {value}"));
                }
                if word == "--git-dir" {
                    git_dir = Some(value.as_str());
                } else {
                    work_tree = Some(value.as_str());
                }
                at += 1;
            }
            other => return Err(format!("it doesn't take {other}")),
        }
        at += 1;
    }
    let words = arguments.get(at + 1..).unwrap_or_default();
    if words.first().map(String::as_str) != Some("there") {
        return Err("it runs only a `there` call, after `--`".to_owned());
    }
    let tree = match (git_dir, work_tree) {
        (Some(git_dir), Some(work_tree)) => Some(Tree { git_dir, work_tree }),
        (None, None) => None,
        _ => return Err("--git-dir and --work-tree go together".to_owned()),
    };
    Ok(Asked {
        tree,
        lent_only,
        words,
    })
}

/// `outside`: runs the call, or says why it can't, as docs/commands.md's
/// three parts. It builds the environment and grants nothing: a fenced
/// command that runs it stays fenced. The exemption is whoever runs it
/// outside the fence: the harness, or mycelium starting a Mac's gate.
/// Its help is the kit's (src/help.rs).
pub fn outside(arguments: &[String]) -> Result<(), Failure> {
    let asked = asked(arguments).map_err(|why| {
        crate::ended::usage(format!("outside can't run that: {why}"))
            .try_this("nix-offload outside --help")
    })?;
    if ended(today()) {
        return Err(Failure::new(
            ENDED,
            format!(
                "outside runs nothing from {}-{:02}-{:02}, when the fence's exemption ended",
                ENDS.0, ENDS.1, ENDS.2
            ),
        )
        .because("A broker holds the builders' key now, and reaches them inside the fence.")
        .try_this("nix-offload there"));
    }
    let (Some(home), Ok(program)) = (env::var_os("HOME"), env::current_exe()) else {
        return Err(Failure::new(
            Exit::Config,
            "outside needs HOME, and its own program's path, to run a call",
        ));
    };
    let home = home.to_string_lossy().into_owned();
    let (user, runtime) = (env::var("USER").ok(), env::var("XDG_RUNTIME_DIR").ok());
    let account = Account {
        home: &home,
        user: user.as_deref(),
        runtime: runtime.as_deref(),
    };
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(environment(&account, asked.tree.as_ref(), asked.lent_only))
        .args(asked.words);
    if let Some(tree) = &asked.tree {
        command.current_dir(tree.work_tree);
    }
    let error = command.exec();
    Err(Failure::new(Exit::Bug, "outside couldn't start its call").because(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split(' ').map(str::to_owned).collect()
    }

    #[test]
    fn a_trees_call_gets_the_exemptions_environment_and_no_more() {
        // The same list, in the same order, as the harness's
        // tests/lib/internal/offload-exemption.test.ts pins for a held
        // tree, with NIX_CONFIG beside it, which the harness takes up when
        // it runs its calls through `outside` (by 17 November).
        // Whoever changes one changes the other.
        let account = Account {
            home: "/home/someone",
            user: Some("someone"),
            runtime: Some("/run/user/1000"),
        };
        let tree = Tree {
            git_dir: "/src/a/.git",
            work_tree: "/src/a",
        };
        let set: Vec<String> = environment(&account, Some(&tree), true)
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        assert_eq!(
            set,
            [
                "HOME=/home/someone",
                "PATH=/etc/profiles/per-user/someone/bin:/run/current-system/sw/bin",
                "USER=someone",
                "LOGNAME=someone",
                "XDG_RUNTIME_DIR=/run/user/1000",
                "GIT_DIR=/src/a/.git",
                "GIT_WORK_TREE=/src/a",
                "GIT_CONFIG_COUNT=1",
                "GIT_CONFIG_KEY_0=core.fsmonitor",
                "GIT_CONFIG_VALUE_0=false",
                "NIX_CONFIG=accept-flake-config = false",
                "NIX_OFFLOAD_LENT_ONLY=1",
            ],
            "a tree's call should get the exemption's environment, in its order"
        );
    }

    #[test]
    fn a_call_outside_any_tree_reads_a_git_folder_nobody_can_make() {
        let account = Account {
            home: "/home/someone",
            user: Some("some one"),
            runtime: None,
        };
        let set = environment(&account, None, false);
        assert!(
            set.contains(&("GIT_DIR".to_owned(), NO_GIT_DIR.to_owned()))
                && set.contains(&("GIT_CEILING_DIRECTORIES".to_owned(), "/".to_owned())),
            "a call outside a tree should read no git folder, got {set:?}"
        );
        assert!(
            set.contains(&("PATH".to_owned(), "/run/current-system/sw/bin".to_owned())),
            "an account name no folder can have should give the system's PATH alone, got {set:?}"
        );
        assert!(
            !set.iter().any(|(key, _)| key == "NIX_OFFLOAD_LENT_ONLY"),
            "derivations alone needn't be kept to the lent builders, got {set:?}"
        );
    }

    #[test]
    fn days_count_to_the_dates_a_calendar_gives() {
        assert_eq!(civil(0), (1970, 1, 1), "day 0 should be 1970-01-01");
        assert_eq!(
            civil(20_785),
            (2026, 11, 28),
            "day 20785 should be 2026-11-28"
        );
        assert_eq!(
            civil(19_782),
            (2024, 2, 29),
            "day 19782 should be the leap day of 2024"
        );
    }

    #[test]
    fn the_exemption_ends_on_its_day_and_not_before() {
        assert!(!ended((2026, 11, 27)), "the day before should still run");
        assert!(ended((2026, 11, 28)), "the day it ends should run nothing");
        assert!(ended((2027, 1, 1)), "a day past it should run nothing");
    }

    #[test]
    fn only_a_there_call_runs_outside() {
        assert!(
            asked(&words("-- sh -c true")).is_err(),
            "a shell should be refused"
        );
        assert!(
            asked(&words("--lent-only")).is_err(),
            "no call at all should be refused"
        );
        assert!(
            asked(&words("--git-dir /a/.git -- there --changed")).is_err(),
            "a git folder without its tree should be refused"
        );
        assert!(
            asked(&words("--git-dir a/.git --work-tree a -- there --changed")).is_err(),
            "a folder named from here should be refused"
        );
        let arguments = words("--git-dir /a/.git --work-tree /a --lent-only -- there --changed");
        let read = asked(&arguments).expect("a tree's call should be read");
        assert_eq!(
            read,
            Asked {
                tree: Some(Tree {
                    git_dir: "/a/.git",
                    work_tree: "/a"
                }),
                lent_only: true,
                words: &arguments[6..],
            },
            "a tree's call should be read whole"
        );
    }
}
