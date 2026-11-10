//! `there --run`: a command run on a lent builder, on the commit checked
//! out here, for a suite that isn't a Nix check.
//!
//! A test suite run as a command, as the harness's vitest and its host
//! tests are, can't be sent as a build, so until 10 November each ran on
//! this machine: at a load of 99 to 229, with the person waiting 7 s on
//! each launcher opening. Sent with `--run`, it runs where changed-checks
//! does, in a checkout of the same commit, with the store paths it names
//! by `--copy` sent first and `WORK_RUNS_ELSEWHERE=1` set, so it knows it
//! runs on a builder. Its output streams back and its status is this
//! one's.
//!
//! It goes only to the two lent builders, never a nearby machine, an
//! evaluator or a VM builder, so work code stays on the account's own
//! cloud. Its ssh names the builder for as long as it runs, which is what
//! the lender counts a builder in use by, so it isn't given back idle
//! mid-run. Its paths go as held outputs do, signed or content-addressed,
//! since the builder trusts no account. Once its caller has been gone for
//! 15 seconds (`NIX_OFFLOAD_RUN_GRACE`), or once it has run for
//! `--longest`, everything it started there ends with it.

use std::path::Path;
use std::process::Command;

use crate::there::{self, Sending};

/// How long a command runs for at most, unless `--longest` says: the
/// harness's host tests took 56 minutes on 10 November, and one past three
/// hours is one that hangs.
pub const LONGEST: u64 = 3 * 3600;

/// What `there --run` was asked for.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    /// The store paths sent ahead of it.
    pub copies: Vec<String>,
    /// The seconds it runs for at most.
    pub longest: u64,
    /// The command, a word each.
    pub words: Vec<String>,
}

/// Reads `--run`'s arguments: `--copy PATH` as often as wanted, each a
/// store path, `--longest SECONDS`, then `--` and the command.
pub fn asked(arguments: &[String]) -> Result<Asked, String> {
    let mut copies = Vec::new();
    let mut longest = LONGEST;
    let mut each = arguments.iter();
    while let Some(word) = each.next() {
        match word.as_str() {
            "--copy" => {
                let path = each.next().ok_or("--copy takes a store path")?;
                if !path.starts_with("/nix/store/") {
                    return Err(format!("--copy takes a store path, not {path}"));
                }
                copies.push(path.clone());
            }
            "--longest" => {
                longest = each
                    .next()
                    .and_then(|seconds| seconds.parse().ok())
                    .filter(|seconds| *seconds > 0)
                    .ok_or("--longest takes a number of seconds")?;
            }
            "--" => {
                let words: Vec<String> = each.cloned().collect();
                if words.is_empty() {
                    return Err("--run takes a command after --".to_owned());
                }
                return Ok(Asked {
                    copies,
                    longest,
                    words,
                });
            }
            other => return Err(format!("--run doesn't take {other}")),
        }
    }
    Err("--run takes a command after --".to_owned())
}

/// What the builder's shell runs: `checkout`, which goes into the commit's
/// folder there, then the command. A clock beside it ends the shell's whole
/// group once it has run `longest` seconds, saying so, and is ended with
/// it otherwise. The command's own status is the shell's. The clock's
/// sleep holds no output open, since one left behind by a TERM landing
/// before its pid is known would keep ssh, and so the builder, until it
/// woke. A command's 255 comes back as 254, since ssh's 255 means the
/// builder was lost, and the run would be sent again.
pub fn command(checkout: &str, asked: &Asked) -> String {
    let words: Vec<String> = asked.words.iter().map(|word| there::quoted(word)).collect();
    let longest = asked.longest;
    format!(
        "{checkout}set +e; \
         (trap 'kill $sleeping 2>/dev/null; exit 0' TERM; \
         sleep {longest} 2>/dev/null & sleeping=$!; wait $sleeping; \
         echo \"nix-offload: the command ran past {longest} s, so it ends\" >&2; \
         kill -TERM 0) </dev/null >/dev/null & clock=$!; \
         WORK_RUNS_ELSEWHERE=1 {}; \
         ended=$?; kill $clock 2>/dev/null; \
         if [ $ended = 255 ]; then \
         echo 'nix-offload: the command ended 255, which ssh says of a lost builder, \
         so it ends 254' >&2; ended=254; fi; (exit $ended)",
        words.join(" ")
    )
}

/// The bytes `paths` and all they reference hold, as Nix says, or none
/// where it can't say.
fn closure_of(nix: &str, paths: &[String]) -> Option<u64> {
    let said = Command::new(nix)
        .args(["path-info", "--closure-size"])
        .args(paths)
        .output()
        .ok()
        .filter(|said| said.status.success())?;
    String::from_utf8_lossy(&said.stdout)
        .lines()
        .map(|line| line.split_whitespace().nth(1)?.parse::<u64>().ok())
        .sum()
}

/// Runs `sending` for `asked` on the lent builder `store` names, its paths
/// sent first, and gives its status, or none when ssh couldn't reach it or
/// its paths couldn't go. `[ssh, nix]` are the programs run.
pub fn on(
    store: &str,
    sending: &Sending,
    asked: &Asked,
    scratch: &Path,
    [ssh, nix]: [&str; 2],
) -> Option<i32> {
    let lent = there::builder(store.lines().next()?.trim())?;
    if !asked.copies.is_empty() {
        eprintln!(
            "nix-offload: sending {} to {}",
            asked.copies.join(", "),
            lent.host
        );
        let size = closure_of(nix, &asked.copies).unwrap_or(0);
        if let Err(why) = there::paths_up(&asked.copies, &lent, nix, there::send_within(size)) {
            eprintln!("nix-offload: {why}");
            return None;
        }
    }
    match there::on_with(
        &lent,
        sending,
        there::Bring::Nothing,
        scratch,
        ssh,
        there::far_brief,
    ) {
        Ok(ran) if ran.status != there::UNREACHED => Some(ran.status),
        Ok(_) => {
            eprintln!("nix-offload: ssh couldn't reach {}", lent.host);
            None
        }
        Err(why) => {
            eprintln!("nix-offload: {why}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn words(said: &str) -> Vec<String> {
        said.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_command_is_read_after_its_copies_and_its_clock() {
        assert_eq!(
            asked(&words(
                "--copy /nix/store/a-runtime --longest 60 --copy /nix/store/b-bin -- pnpm test"
            )),
            Ok(Asked {
                copies: vec![
                    "/nix/store/a-runtime".to_owned(),
                    "/nix/store/b-bin".to_owned()
                ],
                longest: 60,
                words: words("pnpm test"),
            })
        );
        assert_eq!(
            asked(&words("-- sh -c --copy")).map(|asked| (asked.longest, asked.words)),
            Ok((LONGEST, words("sh -c --copy"))),
            "what follows -- is the command's"
        );
    }

    #[test]
    fn a_command_asked_wrongly_is_refused() {
        for wrong in [
            "",
            "--",
            "pnpm test",
            "--copy ./result -- true",
            "--copy",
            "--longest 0 -- true",
            "--longest soon -- true",
        ] {
            assert!(asked(&words(wrong)).is_err(), "{wrong:?} was taken");
        }
    }

    /// The shell's status and what it said on each stream, running the
    /// command `asked` as a builder's shell would, from `folder`.
    fn ran(asked: &Asked, folder: &Path) -> (Option<i32>, String, String) {
        let said = Command::new("sh")
            .args(["-c", &command("", asked)])
            .current_dir(folder)
            .process_group(0)
            .output()
            .unwrap();
        (
            said.status.code(),
            String::from_utf8_lossy(&said.stdout).into_owned(),
            String::from_utf8_lossy(&said.stderr).into_owned(),
        )
    }

    #[test]
    fn a_command_ends_with_its_own_status_knowing_it_runs_elsewhere() {
        let folder = tempfile::tempdir().unwrap();
        let asked = Asked {
            copies: Vec::new(),
            longest: 60,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                "echo \"elsewhere=$WORK_RUNS_ELSEWHERE\"; exit 3".to_owned(),
            ],
        };
        let began = Instant::now();
        let (status, stdout, _) = ran(&asked, folder.path());
        assert_eq!(status, Some(3), "its status was lost");
        assert_eq!(stdout, "elsewhere=1\n");
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "its clock held the shell open"
        );
    }

    #[test]
    fn a_command_ending_as_ssh_does_for_a_lost_builder_ends_otherwise() {
        let folder = tempfile::tempdir().unwrap();
        let asked = Asked {
            copies: Vec::new(),
            longest: 60,
            words: vec!["sh".to_owned(), "-c".to_owned(), "exit 255".to_owned()],
        };
        let (status, _, stderr) = ran(&asked, folder.path());
        assert_eq!(status, Some(254), "it would be sent again: {stderr}");
        assert!(stderr.contains("so it ends 254"), "{stderr}");
    }

    #[test]
    fn a_command_past_its_longest_ends_with_all_it_started() {
        let folder = tempfile::tempdir().unwrap();
        let left = folder.path().join("left");
        let asked = Asked {
            copies: Vec::new(),
            longest: 1,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                format!("(sleep 4; touch {}) & sleep 60", left.display()),
            ],
        };
        let began = Instant::now();
        let (status, _, stderr) = ran(&asked, folder.path());
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "it ran on past its longest"
        );
        assert_ne!(status, Some(0), "one cut off passed");
        assert!(stderr.contains("ran past 1 s"), "{stderr}");
        std::thread::sleep(Duration::from_secs(5));
        assert!(!left.exists(), "what it started went on");
    }

    #[test]
    fn a_command_whose_caller_is_gone_ends_with_all_it_started() {
        // The caller stands in for an ssh session, starting the run in a
        // process group of its own, as sshd does, and ending once told.
        let folder = tempfile::tempdir().unwrap();
        let left = folder.path().join("left");
        let asked = Asked {
            copies: Vec::new(),
            longest: 600,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                format!(
                    "echo started; (sleep 8; touch {}) & sleep 120",
                    left.display()
                ),
            ],
        };
        let run = there::far_brief(&command("", &asked));
        let mut caller = Command::new("sh")
            .args(["-c", "sh -c \"$1\" & read -r line", "caller", &run])
            .env("TMPDIR", folder.path())
            .env("NIX_OFFLOAD_RUN_GRACE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap();
        let mut said = BufReader::new(caller.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        assert_eq!(line, "started\n");
        drop(caller.stdin.take());
        caller.wait().unwrap();
        let (ended, closed) = mpsc::channel();
        std::thread::spawn(move || {
            let mut rest = String::new();
            let _ = std::io::Read::read_to_string(&mut said, &mut rest);
            let _ = ended.send(());
        });
        assert!(
            closed.recv_timeout(Duration::from_secs(30)).is_ok(),
            "the command outlived its caller"
        );
        std::thread::sleep(Duration::from_secs(9));
        assert!(!left.exists(), "what it started went on");
    }
}
