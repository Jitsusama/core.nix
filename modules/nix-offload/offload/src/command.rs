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
//! since the builder trusts no account. Once it has run for `--longest`,
//! everything it started there ends with it.
//!
//! Long because its tests, over half of it, stay beside the private link
//! they drive, running the scripts it writes through real shells.
//!
//! The command runs apart from the ssh that started it, into files there,
//! which that ssh follows. A link lost mid-run, as every tunnel's was at
//! 02:18 on 14 November, is reached again through the same host key and
//! followed on from what had come back, rather than sent again from the
//! start. A run nothing has followed for 120 s (`NIX_OFFLOAD_UNWATCHED`)
//! ends with everything it started, so one whose caller is gone holds the
//! builder no longer than that.
//!
//! It runs as the person, let out of the fence a session works in, so it
//! sends only HEAD's objects, never the index or the files a fenced
//! session can write, and refuses a tree with changes not committed or
//! with objects borrowed from another repository (`there::Sends`).

use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::there::{self, Builder, Sending};

/// What a run says on its standard error as it starts there, before its
/// name: from then, a link lost is reattached to.
const STARTED: &str = "nix-offload: this runs there as ";

/// What a run reattached to says once the builder has no such run.
const GONE: &str = "nix-offload: the builder has no run ";

/// How long a lost link is tried again for: under the 120 s a run there
/// waits unwatched before it ends.
const REATTACH_WITHIN: Duration = Duration::from_secs(90);

/// How long apart a lost link is tried again.
const REATTACH_EVERY: Duration = Duration::from_secs(5);

/// How long a link stays up before its loss counts afresh.
const LINKED: Duration = Duration::from_secs(30);

/// A name for a run there no other run has: this process and the moment.
pub fn run_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{}-{nanos}", std::process::id())
}

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
/// folder there, then the command, started apart as the run `run` names
/// and followed (`follow`). A clock beside it ends the run's whole group
/// once it has run `longest` seconds, saying so, and is ended with it
/// otherwise. A watch beside it ends the group once nothing has followed
/// it for `NIX_OFFLOAD_UNWATCHED` seconds, and takes its files away then
/// too, whether it ended or not. The command's own status is the shell's. The clock's
/// sleep holds no output open, since one left behind by a TERM landing
/// before its pid is known would keep ssh, and so the builder, until it
/// woke. Once it has run out, the clock ends the group before saying so,
/// deaf to the group's TERM and to a broken pipe: said first, over a link
/// gone dead, the saying killed the clock, and the command ran on to its
/// end and passed (E2ZIAQ, 12 November). A command's 255 comes back as
/// 254, since ssh's 255 means the builder was lost, and the run would be
/// sent again.
pub fn command(checkout: &str, asked: &Asked, run: &str) -> String {
    let words: Vec<String> = asked.words.iter().map(|word| there::quoted(word)).collect();
    let longest = asked.longest;
    // The run's own shell, in a session of its own, so neither the end of
    // the ssh that started it nor `far`'s watch ends it. It ignores the
    // TERM its clock sends its group, to write the status it ended with.
    let apart = format!(
        "run=$1; trap : TERM; \
         (trap '' TERM; while sleep 2; do [ -d \"$run\" ] || exit 0; \
         beat=$(stat -c %Y \"$run/beat\" 2>/dev/null) || exit 0; \
         [ $(($(date +%s) - beat)) -gt \"${{NIX_OFFLOAD_UNWATCHED:-120}}\" ] || continue; \
         [ -e \"$run/status\" ] || kill -TERM 0; rm -rf \"$run\"; exit 0; done) \
         </dev/null >/dev/null 2>&1 & \
         (trap 'kill $sleeping 2>/dev/null; exit 0' TERM; \
         sleep {longest} 2>/dev/null & sleeping=$!; wait $sleeping; \
         trap '' PIPE TERM; kill -TERM 0; \
         echo \"nix-offload: the command ran past {longest} s, so it ends\" >&2) \
         </dev/null >/dev/null & clock=$!; \
         WORK_RUNS_ELSEWHERE=1 {}; \
         ended=$?; kill $clock 2>/dev/null; wait $clock 2>/dev/null; \
         if [ $ended = 255 ]; then \
         echo 'nix-offload: the command ended 255, which ssh says of a lost builder, \
         so it ends 254' >&2; ended=254; fi; \
         echo $ended > \"$run/status.new\" && mv -f \"$run/status.new\" \"$run/status\"",
        words.join(" ")
    );
    format!(
        "{checkout}set +e; {RUN_DIR} \
         mkdir -p \"$run\" && : > \"$run/out\" && : > \"$run/err\" && : > \"$run/beat\" \
         || {{ echo \"nix-offload: the builder can't keep the run's files\" >&2; exit 254; }}; \
         setsid sh -c {} nix-offload-run \"$run\" </dev/null >\"$run/out\" 2>\"$run/err\" & \
         echo $! > \"$run/pid\"; \
         echo \"{STARTED}{run}, so a lost link reattaches to it\" >&2; {}",
        there::quoted(&apart),
        follow(run, 0, 0),
    )
    .replace("{RUN_NAME}", run)
}

/// Where a run keeps its files there, named `{RUN_NAME}`.
const RUN_DIR: &str = "run=\"${TMPDIR:-/tmp}/nix-offload-runs/{RUN_NAME}\";";

/// What follows the run `run` there, from the `out` bytes of its output and
/// the `err` of its errors already come back: each is printed on from
/// there as it comes, and the run's status is this shell's, once it ends.
/// While it follows, it says so to the run's watch every 2 s. A run there's
/// no sign of says so, `GONE`, ending as ssh does for a lost builder.
pub fn follow(run: &str, out: u64, err: u64) -> String {
    let (out, err) = (out + 1, err + 1);
    format!(
        "{RUN_DIR} \
         if [ ! -s \"$run/pid\" ]; then echo \"{GONE}{run}\" >&2; exit 255; fi; \
         pid=$(cat \"$run/pid\"); \
         (while kill -0 $$ 2>/dev/null; do touch \"$run/beat\" 2>/dev/null; sleep 2; done) \
         </dev/null >/dev/null 2>&1 & \
         tail -s 0.5 -c +{err} --pid=\"$pid\" -f \"$run/err\" >&2 & erring=$!; \
         tail -s 0.5 -c +{out} --pid=\"$pid\" -f \"$run/out\"; wait $erring; \
         if [ -e \"$run/status\" ]; then ended=$(cat \"$run/status\"); rm -rf \"$run\"; \
         exit \"$ended\"; fi; \
         echo 'nix-offload: the run there ended with no status' >&2; rm -rf \"$run\"; exit 254"
    )
    .replace("{RUN_NAME}", run)
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

/// Runs `sending` for `asked` on the lent builder `store` names, as the run
/// `run`, its paths sent first, and gives its status, or none when ssh
/// couldn't reach it or its paths couldn't go. `[ssh, nix]` are the
/// programs run.
pub fn on(
    store: &str,
    sending: &Sending,
    asked: &Asked,
    run: &str,
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
    eprintln!("nix-offload: building on the lent builder {}", lent.host);
    let link = Link {
        lent: &lent,
        scratch,
        ssh,
        pause: REATTACH_EVERY,
    };
    let out = || Box::new(std::io::stdout()) as Box<dyn Write + Send>;
    match link.followed(sending, run, &out, &mut std::io::stderr()) {
        Ok(status) if status != there::UNREACHED => Some(status),
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

/// How a run reaches its builder: the builder, the scratch folder ssh's
/// files go in, the ssh run, and how long apart a lost link is tried again.
struct Link<'a> {
    lent: &'a Builder,
    scratch: &'a Path,
    ssh: &'a str,
    pause: Duration,
}

/// What a run has said here so far: the bytes of its output and of its
/// errors, and whether it said it started there, or that it's gone.
#[derive(Default)]
struct Followed {
    out: u64,
    err: u64,
    started: bool,
    gone: bool,
}

impl Link<'_> {
    /// Sends `sending`, the run `run`, and follows it, its output going to
    /// what `out` gives and its errors to `err`, giving its status. A link
    /// lost once it started is reached again every `pause` for
    /// `REATTACH_WITHIN`, and followed on from what had come back; ssh's
    /// own 255 comes back once it can't be, or before the run started.
    fn followed(
        &self,
        sending: &Sending,
        run: &str,
        out: &dyn Fn() -> Box<dyn Write + Send>,
        err: &mut dyn Write,
    ) -> Result<i32, String> {
        let mut seen = Followed::default();
        let mut script = sending.command.clone();
        let mut bundle = sending.bundle.as_deref();
        let mut lost: Option<Instant> = None;
        loop {
            let began = Instant::now();
            let status = self.attached(&script, bundle.take(), out(), err, &mut seen)?;
            if status != there::UNREACHED || !seen.started || seen.gone {
                return Ok(status);
            }
            if began.elapsed() > LINKED {
                lost = None;
            }
            if lost.get_or_insert_with(Instant::now).elapsed() > REATTACH_WITHIN {
                let _ = writeln!(
                    err,
                    "nix-offload: {} wasn't reached again within {} s, so its run is lost",
                    self.lent.host,
                    REATTACH_WITHIN.as_secs()
                );
                return Ok(status);
            }
            let _ = writeln!(
                err,
                "nix-offload: the link to {} was lost, so this reattaches to its run",
                self.lent.host
            );
            std::thread::sleep(self.pause);
            script = follow(run, seen.out, seen.err);
        }
    }

    /// Runs `script` there once, `bundle` on its standard input, passing on
    /// what it prints and counting it in `seen`, and gives ssh's status.
    /// Its errors count but for nix-offload's own lines, which come from
    /// here and the builder's gate rather than the run's file.
    fn attached(
        &self,
        script: &str,
        bundle: Option<&Path>,
        mut out: Box<dyn Write + Send>,
        err: &mut dyn Write,
        seen: &mut Followed,
    ) -> Result<i32, String> {
        let (mut command, known, said) = there::reach(self.lent, self.scratch, self.ssh)?;
        command
            .arg(&self.lent.host)
            .arg(there::far_brief(script))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(bundle) = bundle {
            let bundle =
                std::fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
            command.stdin(bundle);
        }
        let ended = command.spawn().and_then(|mut running| {
            let printed = running.stdout.take();
            let copying = std::thread::spawn(move || {
                let mut count = 0;
                let mut chunk = [0u8; 8192];
                if let Some(mut printed) = printed {
                    while let Ok(read) = printed.read(&mut chunk) {
                        if read == 0 {
                            break;
                        }
                        let _ = out.write_all(&chunk[..read]).and_then(|()| out.flush());
                        count += read as u64;
                    }
                }
                count
            });
            if let Some(said) = running.stderr.take() {
                let mut said = std::io::BufReader::new(said);
                let mut line = Vec::new();
                while said.read_until(b'\n', &mut line)? > 0 {
                    let _ = err.write_all(&line);
                    if line.starts_with(STARTED.as_bytes()) {
                        seen.started = true;
                    } else if line.starts_with(GONE.as_bytes()) {
                        seen.gone = true;
                    } else if !line.starts_with(b"nix-offload: ") {
                        seen.err += line.len() as u64;
                    }
                    line.clear();
                }
            }
            let status = running.wait()?;
            seen.out += copying.join().unwrap_or(0);
            Ok(status)
        });
        let _ = std::fs::remove_file(&known);
        let _ = std::fs::remove_file(&said);
        let status = ended.map_err(|error| format!("couldn't run {}: {error}", self.ssh))?;
        // A run killed by a signal there comes back as ssh's status.
        Ok(status.code().unwrap_or(there::UNREACHED))
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
            .args(["-c", &command("", asked, "a-run")])
            .current_dir(folder)
            .env("TMPDIR", folder)
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

    /// What `asked` ends with, sent far as a lent run is, while its caller
    /// reads nothing it says: the stand-in for a link gone dead.
    fn ran_far_unread(asked: &Asked, folder: &Path) -> Option<i32> {
        let run = there::far_brief(&command("", asked, "a-run"));
        Command::new("sh")
            .args([
                "-c",
                "{ sh -c \"$1\" 2>&1 >/dev/null; echo $? >&3; } 3>ended | true; \
                 read -r ended < ended; exit \"$ended\"",
                "caller",
                &run,
            ])
            .current_dir(folder)
            .env("TMPDIR", folder)
            .process_group(0)
            .status()
            .unwrap()
            .code()
    }

    #[test]
    fn a_command_past_its_longest_ends_though_nothing_reads_it() {
        let folder = tempfile::tempdir().unwrap();
        let left = folder.path().join("left");
        let asked = Asked {
            copies: Vec::new(),
            longest: 1,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                format!("(sleep 4; touch {}) & sleep 8", left.display()),
            ],
        };
        let began = Instant::now();
        let ended = ran_far_unread(&asked, folder.path());
        assert!(
            began.elapsed() < Duration::from_secs(6),
            "its clock didn't end it, so it ran its 8 s"
        );
        assert_ne!(ended, Some(0), "one cut off passed");
        std::thread::sleep(Duration::from_secs(5));
        assert!(!left.exists(), "what it started went on");
    }

    #[test]
    fn a_command_its_clock_ends_is_never_sent_again() {
        let folder = tempfile::tempdir().unwrap();
        let asked = Asked {
            copies: Vec::new(),
            longest: 1,
            words: vec!["sleep".to_owned(), "30".to_owned()],
        };
        let ended = ran_far_unread(&asked, folder.path());
        assert_ne!(ended, Some(there::UNREACHED), "it would be sent again");
        assert_ne!(ended, Some(0), "one cut off passed");
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
        let run = there::far_brief(&command("", &asked, "a-run"));
        let mut caller = Command::new("sh")
            .args(["-c", "sh -c \"$1\" & read -r line", "caller", &run])
            .env("TMPDIR", folder.path())
            .env("NIX_OFFLOAD_RUN_GRACE", "1")
            .env("NIX_OFFLOAD_UNWATCHED", "1")
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

    /// A command that says `one` on each stream, waits, then says `two` on
    /// each and ends 4.
    fn says_twice() -> Asked {
        Asked {
            copies: Vec::new(),
            longest: 60,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                "echo one; echo err-one >&2; sleep 3; echo two; echo err-two >&2; exit 4"
                    .to_owned(),
            ],
        }
    }

    /// `script` run as a builder's shell would, in `folder`.
    fn far_side(script: &str, folder: &Path) -> Command {
        let mut far = Command::new("sh");
        far.args(["-c", script])
            .current_dir(folder)
            .env("TMPDIR", folder)
            .process_group(0);
        far
    }

    /// Starts `asked` as the run `run` and ends what follows it, as a lost
    /// link does, once it has said its first line.
    fn started_then_lost(asked: &Asked, run: &str, folder: &Path, unwatched: &str) {
        let mut far = far_side(&command("", asked, run), folder)
            .env("NIX_OFFLOAD_UNWATCHED", unwatched)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut said = BufReader::new(far.stdout.take().unwrap());
        let mut line = String::new();
        said.read_line(&mut line).unwrap();
        assert!(
            line.starts_with("one") || line.starts_with("started"),
            "{line}"
        );
        let group = libc::pid_t::try_from(far.id()).unwrap();
        assert_eq!(unsafe { libc::kill(-group, libc::SIGKILL) }, 0);
        far.wait().unwrap();
    }

    #[test]
    fn a_run_whose_link_was_lost_is_followed_on_from_what_came_back() {
        let folder = tempfile::tempdir().unwrap();
        started_then_lost(&says_twice(), "lost", folder.path(), "120");
        let followed = far_side(&follow("lost", 4, 8), folder.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&followed.stderr);
        assert_eq!(
            followed.status.code(),
            Some(4),
            "its status was lost: {stderr}"
        );
        assert_eq!(String::from_utf8_lossy(&followed.stdout), "two\n");
        assert!(stderr.contains("err-two"), "{stderr}");
        assert!(!stderr.contains("err-one"), "said again: {stderr}");
        assert!(
            !folder.path().join("nix-offload-runs/lost").exists(),
            "its files stayed"
        );
    }

    #[test]
    fn a_run_nothing_follows_ends_with_all_it_started() {
        let folder = tempfile::tempdir().unwrap();
        let left = folder.path().join("left");
        let asked = Asked {
            copies: Vec::new(),
            longest: 600,
            words: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                format!(
                    "echo started; (sleep 8; touch {0}) & sleep 8; touch {0}",
                    left.display()
                ),
            ],
        };
        started_then_lost(&asked, "unwatched", folder.path(), "1");
        std::thread::sleep(Duration::from_secs(10));
        assert!(!left.exists(), "it ran on unwatched");
        assert!(
            !folder.path().join("nix-offload-runs/unwatched").exists(),
            "its files stayed"
        );
    }

    #[test]
    fn a_run_the_builder_has_no_sign_of_ends_as_a_lost_builder_does() {
        let folder = tempfile::tempdir().unwrap();
        let followed = far_side(&follow("never", 0, 0), folder.path())
            .output()
            .unwrap();
        assert_eq!(followed.status.code(), Some(there::UNREACHED));
        let said = String::from_utf8_lossy(&followed.stderr);
        assert!(said.contains(GONE), "it never said the run was gone: {said}");
    }

    /// What's written to it, kept where a test reads it.
    #[derive(Clone, Default)]
    struct Kept(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl Write for Kept {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Kept {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    /// ssh as a builder answers whose first link is lost 1.5 s in, once
    /// what it ran has started: it ends what followed the run and exits
    /// 255, leaving the run itself going. Every link after holds. Each
    /// runs in a session of its own, as sshd gives one, since what `far`
    /// wraps a command in ends its own group as it goes; and each is
    /// waited on, as sshd does, since a shell left a zombie still answers
    /// the loop beside it that keeps its errors open while it lives.
    const LOSES_FIRST_LINK: &str = r#"#!/bin/sh
here=$(dirname "$0")
export TMPDIR="$here"
for last; do :; done
echo "$last" >> "$here/sent"
if [ ! -e "$here/lost-once" ]; then
  : > "$here/lost-once"
  setsid sh -c "$last" & follower=$!
  sleep 1.5
  kill -KILL -"$follower"
  exit 255
fi
setsid sh -c "$last"
exit $?
"#;

    /// What's left running of a test in `folder`, a process a line with its
    /// state, parent, group and session, and the pipes it holds, to say
    /// what a run that never ended waits on.
    fn left_running(folder: &Path) -> String {
        use std::fmt::Write as _;
        let folder = folder.display().to_string();
        let mut left = String::new();
        for each in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
            let place = each.path();
            let Ok(cmdline) = std::fs::read(place.join("cmdline")) else {
                continue;
            };
            let cmdline = String::from_utf8_lossy(&cmdline).replace('\0', " ");
            if !cmdline.contains(&folder) && !cmdline.contains("nix-offload-run") {
                continue;
            }
            let stat = std::fs::read_to_string(place.join("stat")).unwrap_or_default();
            let fields: Vec<&str> = stat
                .rsplit_once(") ")
                .map_or("", |(_, rest)| rest)
                .split(' ')
                .take(4)
                .collect();
            let pipes: Vec<String> = std::fs::read_dir(place.join("fd"))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|fd| {
                    let target = std::fs::read_link(fd.path()).ok()?;
                    let target = target.display().to_string();
                    target
                        .starts_with("pipe:")
                        .then(|| format!("{}={target}", fd.file_name().to_string_lossy()))
                })
                .collect();
            let short: String = cmdline.chars().take(100).collect();
            let _ = writeln!(
                left,
                "{} {fields:?} {pipes:?} {short}",
                each.file_name().to_string_lossy()
            );
        }
        left
    }

    #[test]
    fn a_lost_link_is_reattached_to_and_the_run_ends_as_it_did() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().unwrap();
        let ssh = folder.path().join("ssh");
        std::fs::write(&ssh, LOSES_FIRST_LINK).unwrap();
        std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (out, err) = (Kept::default(), Kept::default());
        let (printed, mut erred) = (out.clone(), err.clone());
        let scratch = folder.path().join("scratch");
        let (ended, status) = mpsc::channel();
        // Followed apart, so a run that never ends fails the test with what
        // it left running rather than holding the whole suite.
        std::thread::spawn(move || {
            let lent = Builder {
                host: "far".to_owned(),
                key: None,
                host_key: None,
                store: "ssh-ng://far".to_owned(),
            };
            let link = Link {
                lent: &lent,
                scratch: &scratch,
                ssh: ssh.to_str().unwrap(),
                pause: Duration::from_millis(100),
            };
            let sending = Sending {
                command: command("", &says_twice(), "relinked"),
                bundle: None,
            };
            let to_out = move || Box::new(printed.clone()) as Box<dyn Write + Send>;
            let _ = ended.send(link.followed(&sending, "relinked", &to_out, &mut erred));
        });
        let status = status
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| {
                panic!(
                    "the run never ended; said {:?}, printed {:?}, left running:\n{}",
                    err.text(),
                    out.text(),
                    left_running(folder.path())
                )
            });
        let said = err.text();
        assert_eq!(status, Ok(4), "{said}");
        assert_eq!(out.text(), "one\ntwo\n", "{said}");
        assert_eq!(said.matches("err-one").count(), 1, "{said}");
        assert!(said.contains("err-two"), "{said}");
        assert!(said.contains("so this reattaches to its run"), "{said}");
        let sent = std::fs::read_to_string(folder.path().join("sent")).unwrap();
        assert_eq!(
            sent.matches("WORK_RUNS_ELSEWHERE=1").count(),
            1,
            "sent again: {sent}"
        );
    }
}
