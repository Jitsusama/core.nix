//! Which of systemd's jobs is slow enough to name.
//!
//! A job is slow once it has been running for SLOW. Each job's step comes
//! from its signals where systemd sends one, and from a look where it
//! doesn't: a job that starts running in the same breath as it's queued is
//! announced only as new, so a job of unknown step is looked at before it's
//! named. A job waiting on another is never named, since what it waits on
//! is. The one named stays until its job ends, so the screen doesn't flit
//! between jobs, and the next slow one takes its place at once.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long a job runs before it's named: past what a quick start or stop
/// takes, and short enough to name a stall while it's happening.
pub const SLOW: Duration = Duration::from_secs(3);

/// The kinds of unit whose jobs wait on something a person would know: a
/// program, a disk, a device or a session. A target's job only waits on
/// the others.
const NAMED: [&str; 5] = [".service", ".mount", ".swap", ".device", ".scope"];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Unknown,
    Waiting,
    Running,
}

#[derive(Debug)]
struct Job {
    unit: String,
    since: Instant,
    step: Step,
}

/// What to do next.
#[derive(Debug, PartialEq)]
pub enum Next {
    /// Ask systemd whether this job is running.
    Look(u32),
    /// Name this unit's job.
    Name(String),
    /// The job named has ended and nothing else is slow.
    Clear,
    Nothing,
}

#[derive(Debug, Default)]
pub struct Jobs {
    jobs: HashMap<u32, Job>,
    named: Option<u32>,
}

impl Jobs {
    /// A job systemd has queued, its step unknown. A unit no one would
    /// know, or this program's own, is left out.
    pub fn added(&mut self, id: u32, unit: &str, now: Instant, own: &str) {
        if unit == own || !NAMED.iter().any(|kind| unit.ends_with(kind)) {
            return;
        }
        self.jobs.entry(id).or_insert(Job {
            unit: unit.to_owned(),
            since: now,
            step: Step::Unknown,
        });
    }

    /// A job's step, from its signal or a look. A job that was waiting
    /// counts its time from starting to run.
    pub fn stepped(&mut self, id: u32, running: bool, now: Instant) {
        let Some(job) = self.jobs.get_mut(&id) else {
            return;
        };
        if running {
            if job.step == Step::Waiting {
                job.since = now;
            }
            job.step = Step::Running;
        } else {
            job.step = Step::Waiting;
        }
    }

    pub fn removed(&mut self, id: u32) {
        self.jobs.remove(&id);
    }

    /// When a job next becomes slow, unless one is named already.
    pub fn due(&self) -> Option<Instant> {
        if self.named.is_some_and(|id| self.jobs.contains_key(&id)) {
            return None;
        }
        self.candidates().map(|(_, job)| job.since + SLOW).min()
    }

    pub fn next(&mut self, now: Instant) -> Next {
        if self.named.is_some_and(|id| self.jobs.contains_key(&id)) {
            return Next::Nothing;
        }
        let slow = self
            .candidates()
            .filter(|(_, job)| job.since + SLOW <= now)
            .min_by_key(|(id, job)| (job.since, **id))
            .map(|(&id, job)| (id, job.step, job.unit.clone()));
        match slow {
            Some((id, Step::Unknown, _)) => Next::Look(id),
            Some((id, _, unit)) => {
                self.named = Some(id);
                Next::Name(unit)
            }
            None if self.named.take().is_some() => Next::Clear,
            None => Next::Nothing,
        }
    }

    fn candidates(&self) -> impl Iterator<Item = (&u32, &Job)> {
        self.jobs
            .iter()
            .filter(|(_, job)| job.step != Step::Waiting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: &str = "plymouth-waits.service";

    fn later(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    #[test]
    fn a_job_running_past_the_limit_is_named() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, true, start);
        assert_eq!(jobs.due(), Some(later(start, 3)));
        assert_eq!(jobs.next(later(start, 2)), Next::Nothing);
        assert_eq!(
            jobs.next(later(start, 3)),
            Next::Name("docker.service".into())
        );
    }

    #[test]
    fn a_job_of_unknown_step_is_looked_at_first() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        assert_eq!(jobs.next(later(start, 3)), Next::Look(7));
        jobs.stepped(7, true, later(start, 3));
        assert_eq!(
            jobs.next(later(start, 3)),
            Next::Name("docker.service".into())
        );
    }

    #[test]
    fn a_job_waiting_on_another_is_never_named() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, false, start);
        assert_eq!(jobs.due(), None);
        assert_eq!(jobs.next(later(start, 60)), Next::Nothing);
    }

    #[test]
    fn a_job_that_waited_counts_from_starting_to_run() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, false, start);
        jobs.stepped(7, true, later(start, 10));
        assert_eq!(jobs.next(later(start, 12)), Next::Nothing);
        assert_eq!(jobs.due(), Some(later(start, 13)));
    }

    #[test]
    fn a_job_that_ends_in_time_is_never_named() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, true, start);
        jobs.removed(7);
        assert_eq!(jobs.due(), None);
        assert_eq!(jobs.next(later(start, 5)), Next::Nothing);
    }

    #[test]
    fn the_name_stays_until_its_job_ends_then_clears() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, true, start);
        jobs.next(later(start, 3));
        jobs.added(8, "home.mount", later(start, 1), OWN);
        jobs.stepped(8, true, later(start, 1));
        assert_eq!(jobs.due(), None);
        assert_eq!(jobs.next(later(start, 9)), Next::Nothing);
        jobs.removed(8);
        jobs.removed(7);
        assert_eq!(jobs.next(later(start, 10)), Next::Clear);
        assert_eq!(jobs.next(later(start, 11)), Next::Nothing);
    }

    #[test]
    fn the_next_slow_job_takes_the_ended_ones_place() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(7, "docker.service", start, OWN);
        jobs.stepped(7, true, start);
        jobs.added(8, "home.mount", later(start, 1), OWN);
        jobs.stepped(8, true, later(start, 1));
        assert_eq!(
            jobs.next(later(start, 5)),
            Next::Name("docker.service".into())
        );
        jobs.removed(7);
        assert_eq!(jobs.next(later(start, 6)), Next::Name("home.mount".into()));
    }

    #[test]
    fn the_longest_running_is_named_first() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(8, "home.mount", later(start, 1), OWN);
        jobs.stepped(8, true, later(start, 1));
        jobs.added(9, "docker.service", start, OWN);
        jobs.stepped(9, true, start);
        assert_eq!(
            jobs.next(later(start, 5)),
            Next::Name("docker.service".into())
        );
    }

    #[test]
    fn targets_and_its_own_job_are_left_out() {
        let start = Instant::now();
        let mut jobs = Jobs::default();
        jobs.added(1, "reboot.target", start, OWN);
        jobs.added(2, OWN, start, OWN);
        jobs.added(3, "user.slice", start, OWN);
        jobs.stepped(1, true, start);
        jobs.stepped(2, true, start);
        jobs.stepped(3, true, start);
        assert_eq!(jobs.due(), None);
        assert_eq!(jobs.next(later(start, 60)), Next::Nothing);
    }

    #[test]
    fn every_kind_a_person_would_know_is_followed() {
        let start = Instant::now();
        for unit in [
            "a.service",
            "home.mount",
            "dev-zram0.swap",
            "dev-sda.device",
            "session-2.scope",
        ] {
            let mut jobs = Jobs::default();
            jobs.added(1, unit, start, OWN);
            jobs.stepped(1, true, start);
            assert_eq!(
                jobs.next(later(start, 3)),
                Next::Name(unit.into()),
                "{unit}"
            );
        }
    }
}
