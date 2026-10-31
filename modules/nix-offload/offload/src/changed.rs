//! `nix-offload there --changed [ARGUMENT...]`: runs changed-checks for
//! the git tree it's started in on another machine, with the same
//! arguments, so neither of its evaluations runs here.
//!
//! Each evaluation of a change's checks held about 4.8 GB and several
//! cores, and every session changing core ran its own beside the person's
//! work: on 30 October they were a steady part of a load of 92 to 96. The
//! tree goes as `there` sends one, its commit by name when GitHub has it,
//! or the commits since as a bundle of a few kilobytes. The far side keeps
//! a mirror of the repository, checks the commit out beside origin's main,
//! and runs that commit's own changed-checks, so what it names is what the
//! tree's dev shell would have named.
//!
//! Where: the lent builder while one stands, which evaluates and builds
//! in one place. Otherwise the first evaluator that answers and takes the
//! repository's owner, named in `nix-offload/evaluators` under XDG's
//! config folder, which only
//! evaluates, and the checks it names are built by `there` as any build
//! is. With neither, it exits [`NOT_TAKEN`] and changed-checks runs here.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::there::{self, Sending, Source};

/// What `there --changed` exits with when nothing elsewhere took the run,
/// so changed-checks runs it here: sysexits.h's temporary failure, which
/// no changed-checks run gives.
pub const NOT_TAKEN: u8 = 75;

/// What the far side's changed-checks is told, so it runs there and never
/// sends itself on, and so the far side's slices treat it as other work.
const HERE: &str = "CHANGED_CHECKS_HERE=1 WORK_RUNS_ELSEWHERE=1";

/// A machine that evaluates, from a line of `nix-offload/evaluators`:
/// `ADDRESS HOST OWNER,OWNER`, where the address is what a quick
/// connection is tried at, `host:port`, the host is what ssh is given, and
/// the owners are those on GitHub whose repositories it takes. A machine
/// may be one only some work may reach, so it's given only what it names.
#[derive(Debug, PartialEq, Eq)]
pub struct Evaluator {
    pub address: String,
    pub host: String,
    pub owners: Vec<String>,
}

impl Evaluator {
    /// Whether it takes the repository `github` names, `owner/name`.
    pub fn takes(&self, github: &str) -> bool {
        github
            .split('/')
            .next()
            .is_some_and(|owner| self.owners.iter().any(|taken| taken == owner))
    }
}

/// The evaluators in a file of them, a line each. Blank lines and those
/// starting `#` say nothing, and a line short of a word names none.
pub fn evaluators(text: &str) -> Vec<Evaluator> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            Some(Evaluator {
                address: words.next()?.to_owned(),
                host: words.next()?.to_owned(),
                owners: words.next()?.split(',').map(str::to_owned).collect(),
            })
        })
        .collect()
}

/// The command the far side's shell is given to run changed-checks with
/// `arguments` on the commit `source` names, and the bundle it reads from
/// its standard input, when the commit isn't on GitHub.
pub fn sending(source: Source, arguments: &[String]) -> Sending {
    let (github, base, commit, dir, bundle) = match source {
        Source::Pushed {
            github,
            commit,
            dir,
        } => (github, commit.clone(), commit, dir, None),
        Source::Bundled {
            github,
            base,
            commit,
            dir,
            bundle,
        } => (github, base, commit, dir, Some(bundle)),
    };
    // Kept between runs, so each fetches only what's new; a name per
    // repository, as `owner-name`.
    let mirror = format!(
        "\"$HOME\"/.cache/nix-offload/changed/{}.git",
        github.replace('/', "-")
    );
    // A folder named for this run, gone when its shell ends.
    let into = format!("/tmp/nix-offload-changed-{}", std::process::id());
    let unbundled = if bundle.is_some() {
        format!(
            "cat > {into}/there.bundle; \
             git -C {into}/tree fetch -q {into}/there.bundle {}:refs/heads/there; ",
            there::BUNDLED
        )
    } else {
        String::new()
    };
    let words: Vec<String> = arguments.iter().map(|word| there::quoted(word)).collect();
    let command = format!(
        "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
         command -v git >/dev/null || \
         git() {{ nix shell nixpkgs#git -c git \"$@\"; }}; \
         [ -d {mirror} ] || git init -q --bare {mirror}; \
         git -C {mirror} fetch -q https://github.com/{github} \
         +refs/heads/main:refs/heads/main {base}; \
         git -C {mirror} symbolic-ref HEAD refs/heads/main; \
         git clone -q --shared {mirror} {into}/tree; \
         {unbundled}\
         git -C {into}/tree checkout -q -B nix-offload-changed {commit}; \
         cd {into}/tree/{}; \
         {HERE} nix run .#changed-checks -- {}",
        dir.as_deref().unwrap_or("."),
        words.join(" ")
    );
    Sending { command, bundle }
}

/// What a run there asked for, read from changed-checks' own arguments:
/// whether it builds, whether VM tests are among what it builds, and the
/// system it was told.
#[derive(Debug, PartialEq, Eq)]
pub struct Wanted {
    pub build: bool,
    pub vm: bool,
    pub system: Option<String>,
}

pub fn wanted(arguments: &[String]) -> Wanted {
    let (mut whole, mut no_vm, mut name_only, mut system) = (false, false, false, None);
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "--build" => whole = true,
            "--no-vm" => no_vm = true,
            "--name-only" => name_only = true,
            "--system" => system = words.next().cloned(),
            _ => {}
        }
    }
    // As changed-checks reads them: it builds what it names unless only
    // asked to name it, and VM tests only with --build and no --no-vm.
    Wanted {
        build: !name_only,
        vm: whole && !no_vm,
        system,
    }
}

/// The checks to build from what changed-checks printed, a kind and a name
/// a line: those added and changed, VM tests only when wanted, as
/// changed-checks itself would build them.
pub fn to_build(printed: &str, vm: bool) -> Vec<String> {
    printed
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let kind = words.next()?;
            let name = words.next()?;
            (matches!(kind, "added" | "changed") && (vm || !name.starts_with("vm-")))
                .then(|| name.to_owned())
        })
        .collect()
}

/// Runs `command` on `host` by ssh, sending `bundle` on its standard
/// input, and gives its status and what it printed, which comes here as
/// it goes. Its log goes to this one's standard error.
pub fn evaluated(
    ssh: &str,
    host: &str,
    sending: &Sending,
    scratch: &Path,
) -> Result<(i32, String), String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let said = there::mine(scratch, "evaluator-ssh.log");
    let mut command = there::reaching(ssh, &said);
    command.arg(host).arg(&sending.command);
    command.stdout(Stdio::piped());
    if let Some(bundle) = &sending.bundle {
        let bundle = fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
        command.stdin(bundle);
    }
    let mut running = command
        .spawn()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let mut printed = String::new();
    if let Some(output) = running.stdout.take() {
        let mut out = std::io::stdout();
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let _ = writeln!(out, "{line}");
            printed.push_str(&line);
            printed.push('\n');
        }
    }
    let status = running
        .wait()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let _ = fs::remove_file(&said);
    Ok((status.code().unwrap_or(there::UNREACHED), printed))
}

/// The command that builds `checks` of the tree `folder` for `system`, as
/// `there` builds anything: this same program, so it waits for a builder,
/// sends the tree and journals as any `there` does.
pub fn building(folder: &Path, system: &str, checks: &[String]) -> Command {
    let mut command = Command::new("/proc/self/exe");
    command
        .args(["there", "-L", "--keep-going"])
        .arg(folder)
        .args(
            checks
                .iter()
                .map(|check| format!("checks.{system}.{check}")),
        );
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn evaluators_are_read_a_line_each() {
        let read =
            evaluators("# nearby\n\nfar.local:22 far me,us\nshort:22 short\n near:22  near  me \n");
        assert_eq!(
            read,
            [
                Evaluator {
                    address: "far.local:22".to_owned(),
                    host: "far".to_owned(),
                    owners: vec!["me".to_owned(), "us".to_owned()],
                },
                Evaluator {
                    address: "near:22".to_owned(),
                    host: "near".to_owned(),
                    owners: vec!["me".to_owned()],
                }
            ]
        );
    }

    #[test]
    fn an_evaluator_takes_only_the_owners_it_names() {
        let far = &evaluators("far:22 far me,us\n")[0];
        assert!(far.takes("me/thing"));
        assert!(far.takes("us/other"));
        assert!(!far.takes("them/thing"));
        assert!(!far.takes("mee/thing"));
    }

    #[test]
    fn a_pushed_commit_is_checked_out_from_a_kept_mirror_and_run_by_its_own_changed_checks() {
        let sent = sending(
            Source::Pushed {
                github: "someone/thing".to_owned(),
                commit: "abc123".to_owned(),
                dir: None,
            },
            &words("--build --system x86_64-linux"),
        );
        assert!(sent.bundle.is_none());
        for part in [
            "\"$HOME\"/.cache/nix-offload/changed/someone-thing.git",
            "fetch -q https://github.com/someone/thing +refs/heads/main:refs/heads/main abc123",
            "symbolic-ref HEAD refs/heads/main",
            "checkout -q -B nix-offload-changed abc123",
            "CHANGED_CHECKS_HERE=1 WORK_RUNS_ELSEWHERE=1 nix run .#changed-checks -- \
             --build --system x86_64-linux",
        ] {
            assert!(sent.command.contains(part), "{part} in {}", sent.command);
        }
        assert!(!sent.command.contains("bundle"), "{}", sent.command);
    }

    #[test]
    fn unpushed_commits_come_as_a_bundle_fetched_into_the_checkout() {
        let sent = sending(
            Source::Bundled {
                github: "someone/thing".to_owned(),
                base: "base1".to_owned(),
                commit: "mine2".to_owned(),
                dir: Some("sub".to_owned()),
                bundle: PathBuf::from("/b"),
            },
            &words("--base 'odd ref'"),
        );
        assert_eq!(sent.bundle, Some(PathBuf::from("/b")));
        for part in [
            "+refs/heads/main:refs/heads/main base1",
            "there.bundle refs/nix-offload/there:refs/heads/there",
            "checkout -q -B nix-offload-changed mine2",
            "/tree/sub;",
        ] {
            assert!(sent.command.contains(part), "{part} in {}", sent.command);
        }
    }

    #[test]
    fn what_a_run_wants_is_read_from_its_arguments() {
        assert_eq!(
            wanted(&words("--no-vm --build --system aarch64-darwin")),
            Wanted {
                build: true,
                vm: false,
                system: Some("aarch64-darwin".to_owned())
            }
        );
        assert_eq!(
            wanted(&[]),
            Wanted {
                build: true,
                vm: false,
                system: None
            }
        );
        assert_eq!(
            wanted(&words("--build")),
            Wanted {
                build: true,
                vm: true,
                system: None
            }
        );
        assert_eq!(
            wanted(&words("--name-only")),
            Wanted {
                build: false,
                vm: false,
                system: None
            }
        );
    }

    #[test]
    fn those_added_and_changed_are_built_and_vm_tests_only_when_wanted() {
        let printed = "added    a\nchanged  vm-b\nremoved  c\nbroken   d\nchanged  e\n";
        assert_eq!(to_build(printed, true), ["a", "vm-b", "e"]);
        assert_eq!(to_build(printed, false), ["a", "e"]);
    }
}
