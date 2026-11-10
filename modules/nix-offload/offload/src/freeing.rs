//! Freeing a full builder beside: one whose proof finds its store under
//! the floor (src/proof.rs) takes no builds until it has room, and a
//! builder that collects only at Nix's own min-free may sit there for
//! good. On 9 November the work Mac's builder sat at 6 GB free until a
//! person freed it by hand. So a builder beside may name a program that
//! frees its store, such as an ssh to a key it lets only collect, and the
//! keeper runs it while the builder is full: at once, then at most every
//! `AGAIN`, ending one that runs past `LONGEST`. It's never waited on,
//! since the keeper names who answers every few seconds.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// How long after one run starts the next may, while the builder stays
/// full: long enough for its next proofs to say what the last freed.
pub const AGAIN: Duration = Duration::from_mins(15);

/// How long a run may take before it's ended: on 9 November collecting
/// 19.8 GiB from the work Mac's builder took 25 s.
pub const LONGEST: Duration = Duration::from_mins(10);

/// The programs that free builders beside, from a file of them, a line
/// each: the builder's address as the file of builders beside names it,
/// then the program. Blank lines and those starting `#` say nothing.
pub fn programs(text: &str) -> Vec<(String, PathBuf)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (address, program) = line.split_once(char::is_whitespace)?;
            Some((address.to_owned(), PathBuf::from(program.trim())))
        })
        .collect()
}

/// One builder's program, and its run.
pub struct Freeing {
    program: PathBuf,
    running: Option<Child>,
    began: Option<Instant>,
}

impl Freeing {
    pub fn of(program: &Path) -> Self {
        Self {
            program: program.to_owned(),
            running: None,
            began: None,
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Run the program at `now` while `builder` is `full` and none ran
    /// within `AGAIN`, saying how the last run ended once it has.
    pub fn tend(&mut self, builder: &str, full: bool, now: Instant) {
        let program = self.program.display();
        if let Some(running) = &mut self.running {
            match running.try_wait() {
                Ok(Some(status)) if status.success() => {
                    eprintln!(
                        "nix-offload: {program} freed what it could of {builder}'s store, and \
                         its next proofs say how much is free"
                    );
                }
                Ok(Some(status)) => eprintln!(
                    "nix-offload: {program} couldn't free {builder}'s store ({status}), so \
                     this tries again in {} minutes while it stays full",
                    AGAIN.as_secs() / 60
                ),
                Ok(None) if self.began.is_some_and(|began| now >= began + LONGEST) => {
                    let group = i32::try_from(running.id()).unwrap_or(i32::MAX);
                    // SAFETY: killpg only signals the group the run leads.
                    unsafe { libc::killpg(group, libc::SIGKILL) };
                    let _ = running.wait();
                    eprintln!(
                        "nix-offload: {program} ran past {} minutes freeing {builder}'s \
                         store, so this ended it",
                        LONGEST.as_secs() / 60
                    );
                }
                Ok(None) => return,
                Err(error) => eprintln!("nix-offload: couldn't hear from {program}: {error}"),
            }
            self.running = None;
        }
        if !full || self.began.is_some_and(|began| now < began + AGAIN) {
            return;
        }
        eprintln!("nix-offload: {builder} is full, so this runs {program} to free its store");
        self.began = Some(now);
        match Command::new(&self.program)
            .stdin(Stdio::null())
            .process_group(0)
            .spawn()
        {
            Ok(child) => self.running = Some(child),
            Err(error) => eprintln!("nix-offload: couldn't run {program}: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn program(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("free");
        crate::testing::executable(&path, &format!("#!/bin/sh\n{body}\n"));
        path
    }

    fn runs(dir: &Path) -> usize {
        fs::read_to_string(dir.join("ran")).map_or(0, |ran| ran.lines().count())
    }

    /// Tend at `now` until the run started then has ended, waiting at most
    /// 5 s.
    fn tend_until_ended(freeing: &mut Freeing, full: bool, now: Instant) {
        freeing.tend("work", full, now);
        let ending = Instant::now() + Duration::from_secs(5);
        while freeing.running.is_some() && Instant::now() < ending {
            std::thread::sleep(Duration::from_millis(10));
            freeing.tend("work", full, now);
        }
        assert!(freeing.running.is_none(), "the run never ended");
    }

    #[test]
    fn programs_are_read_a_line_each_by_the_builders_address() {
        assert_eq!(
            programs("# who frees whom\n\nwork.local:22 /bin/free-work\n"),
            vec![("work.local:22".to_owned(), PathBuf::from("/bin/free-work"))]
        );
    }

    #[test]
    fn a_full_builder_is_freed_at_once_and_again_only_after_a_while() {
        let dir = tempfile::tempdir().expect("a folder");
        let ran = dir.path().join("ran");
        let mut freeing = Freeing::of(&program(
            dir.path(),
            &format!("echo ran >> '{}'", ran.display()),
        ));
        let began = Instant::now();
        tend_until_ended(&mut freeing, true, began);
        assert_eq!(runs(dir.path()), 1, "a full builder is freed at once");
        let soon = began + AGAIN.saturating_sub(Duration::from_secs(1));
        tend_until_ended(&mut freeing, true, soon);
        assert_eq!(runs(dir.path()), 1, "not again so soon");
        tend_until_ended(&mut freeing, true, began + AGAIN);
        assert_eq!(runs(dir.path()), 2, "again once a while has passed");
    }

    #[test]
    fn a_builder_with_room_is_never_freed() {
        let dir = tempfile::tempdir().expect("a folder");
        let ran = dir.path().join("ran");
        let mut freeing = Freeing::of(&program(
            dir.path(),
            &format!("echo ran >> '{}'", ran.display()),
        ));
        tend_until_ended(&mut freeing, false, Instant::now());
        assert_eq!(runs(dir.path()), 0, "a builder with room is left alone");
    }

    #[test]
    fn a_run_past_its_longest_is_ended_with_what_it_started() {
        let dir = tempfile::tempdir().expect("a folder");
        let pid = dir.path().join("pid");
        let mut freeing = Freeing::of(&program(
            dir.path(),
            &format!("sleep 600 & echo $! > '{}'; wait", pid.display()),
        ));
        let began = Instant::now();
        freeing.tend("work", true, began);
        let ending = Instant::now() + Duration::from_secs(5);
        while !pid.exists() && Instant::now() < ending {
            std::thread::sleep(Duration::from_millis(10));
        }
        let child: i32 = fs::read_to_string(&pid)
            .expect("the run says what it started")
            .trim()
            .parse()
            .expect("a pid");
        freeing.tend(
            "work",
            true,
            began + LONGEST.saturating_sub(Duration::from_secs(1)),
        );
        assert!(freeing.running.is_some(), "a run within its time goes on");
        tend_until_ended(&mut freeing, true, began + LONGEST);
        assert!(
            crate::proof::ends(child),
            "what the run started ends with it"
        );
    }
}
