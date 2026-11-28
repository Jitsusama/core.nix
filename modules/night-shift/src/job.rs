//! What `night-shift run` is asked to do: one night job, read from its
//! arguments, its bars, its limits and its command.

use crate::away::Away;
use crate::cgroup::{self, Limits};
use crate::hours::Hours;
use std::path::PathBuf;
use std::time::Duration;

/// One night job, as its arguments give it.
#[derive(Debug, PartialEq)]
pub struct Job {
    pub name: String,
    pub hours: Option<Hours>,
    pub start_under: f64,
    pub stops: Vec<(String, f64)>,
    pub stop_machine: f64,
    pub stop_io: Option<f64>,
    pub rests: Duration,
    pub wait_at_most: Option<Duration>,
    pub runs_at_most: Option<Duration>,
    pub every: Duration,
    pub grace: Duration,
    pub limits: Limits,
    pub away: Away,
    pub command: Vec<String>,
}

/// The job `night-shift run NAME [OPTION VALUE]... -- COMMAND...` asks
/// for, or what's wrong with how it asks.
pub fn parse(args: &[String]) -> Result<Job, String> {
    let (Some("run"), Some(name)) = (args.first().map(String::as_str), args.get(1)) else {
        return Err("night-shift takes run or turn, and the job's name".into());
    };
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("night-shift run takes the job's command after --")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut job = Job {
        name: name.clone(),
        hours: None,
        start_under: 3.0,
        stops: Vec::new(),
        stop_machine: 25.0,
        stop_io: None,
        rests: Duration::from_hours(12),
        wait_at_most: None,
        runs_at_most: None,
        every: Duration::from_secs(5),
        grace: Duration::from_secs(60),
        limits: Limits::default(),
        away: Away::default(),
        command,
    };
    let mut options = args[2..split].iter();
    while let Some(option) = options.next() {
        let value = options
            .next()
            .ok_or_else(|| format!("{option} takes a value"))?;
        let number = |text: &str| {
            text.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0.0)
                .ok_or_else(|| format!("{option} takes a number, not {text}"))
        };
        match option.as_str() {
            "--hours" => job.hours = Some(Hours::read(value)?),
            "--start-under" => job.start_under = number(value)?,
            "--stop-machine" => job.stop_machine = number(value)?,
            "--stop-io" => job.stop_io = Some(number(value)?),
            "--rests" => job.rests = Duration::from_secs_f64(number(value)? * 3600.0),
            "--wait-at-most" => {
                job.wait_at_most = Some(Duration::from_secs_f64(number(value)?));
            }
            "--runs-at-most" => {
                job.runs_at_most = Some(Duration::from_secs_f64(number(value)?));
            }
            "--every" => job.every = Duration::from_secs_f64(number(value)?),
            "--grace" => job.grace = Duration::from_secs_f64(number(value)?),
            "--stop" => {
                let (slice, bar) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--stop takes SLICE=PCT, not {value}"))?;
                job.stops.push((slice.to_owned(), number(bar)?));
            }
            "--memory-high" => job.limits.memory_high = Some(value.clone()),
            "--memory-max" => job.limits.memory_max = Some(value.clone()),
            "--swap-max" => job.limits.swap_max = Some(value.clone()),
            "--reads-at-most" => {
                let (disk, rate) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--reads-at-most takes DISK=RATE, not {value}"))?;
                let bytes = cgroup::rate(rate).ok_or_else(|| {
                    format!("--reads-at-most takes a rate such as 50M, not {rate}")
                })?;
                job.limits.reads.push((PathBuf::from(disk), bytes));
            }
            _ if job.away.take(option, value, number)? => {}
            _ => return Err(format!("{option} isn't an option of run")),
        }
    }
    job.away.check(&job.stops, job.stop_machine)?;
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_job_takes_its_bars_and_its_command() {
        let job = parse(&args(
            "run repack --start-under 2 --stop app.slice=1 --stop agents.slice=10 --stop-machine 30 --rests 6 -- git maintenance run",
        ))
        .unwrap();
        assert_eq!(job.name, "repack");
        assert!(
            (job.start_under - 2.0).abs() < f64::EPSILON,
            "it starts under {}, not 2",
            job.start_under
        );
        assert_eq!(
            job.stops,
            vec![
                ("app.slice".to_owned(), 1.0),
                ("agents.slice".to_owned(), 10.0)
            ]
        );
        assert!(
            (job.stop_machine - 30.0).abs() < f64::EPSILON,
            "it stops past {}, not 30",
            job.stop_machine
        );
        assert_eq!(job.rests, Duration::from_hours(6));
        assert_eq!(job.stop_io, None, "an io bar it wasn't given was taken");
        assert_eq!(job.command, args("git maintenance run"));
        let io = parse(&args("run repack --stop-io 5 -- git gc")).unwrap();
        assert_eq!(io.stop_io, Some(5.0), "--stop-io 5 wasn't taken as 5");
        assert!(
            parse(&args("run repack --stop-io lots -- git gc")).is_err(),
            "an io bar of words was taken"
        );
    }

    #[test]
    fn a_job_takes_its_limits() {
        let job = parse(&args(
            "run repack --memory-high 12G --swap-max 2G --reads-at-most /dev/nvme0n1=50M -- git gc",
        ))
        .unwrap();
        assert_eq!(
            job.limits,
            Limits {
                memory_high: Some("12G".to_owned()),
                memory_max: None,
                swap_max: Some("2G".to_owned()),
                reads: vec![(PathBuf::from("/dev/nvme0n1"), 50_000_000)],
            }
        );
        assert!(
            parse(&args(
                "run repack --reads-at-most /dev/nvme0n1=quick -- true"
            ))
            .is_err(),
            "a rate of words was taken"
        );
    }

    #[test]
    fn a_job_with_no_command_is_refused() {
        assert!(
            parse(&args("run repack --")).is_err(),
            "an empty command was taken"
        );
        assert!(
            parse(&args("run repack")).is_err(),
            "a missing command was taken"
        );
    }

    #[test]
    fn a_bar_that_isnt_a_number_is_refused() {
        assert!(
            parse(&args("run repack --stop app.slice=lots -- true")).is_err(),
            "a bar of words was taken"
        );
        assert!(
            parse(&args("run repack --start-under -1 -- true")).is_err(),
            "a negative bar was taken"
        );
        assert!(
            parse(&args("run repack --stop app.slice -- true")).is_err(),
            "a slice with no bar was taken"
        );
    }
}
