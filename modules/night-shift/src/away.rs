//! Whether Joel is away from the machine, for a job whose bars ease while
//! he is.
//!
//! The apps' and the desktop's slices stop a job at 1%, since a stall
//! there is one he feels at his keys. With nobody at them, a browser left
//! open still stalls a little now and then: on the night of 23 November,
//! with no one at the machine from 23:20 until morning, app.slice stopped
//! a large repository's repack 19 times, each at 1.1 to 2.3%. So a job can give a slice
//! an eased bar with `--away-stop SLICE=PCT`, and the machine one with
//! `--away-stop-machine PCT`, which hold only while he is away; the moment
//! he's back, the bars it was given with `--stop` hold again, so a stall
//! then stops it as before.
//!
//! He's at the machine while `--away-mark` is there, the mark swayidle-away
//! keeps for two minutes after a key or a touch, and while a call is on,
//! as `--away-call` says: a program and its arguments, split at spaces,
//! exiting 0 during a call and 1 without one; a call it can't tell about
//! counts as one, and so does a program that won't start or doesn't
//! answer within `CALL_ANSWERS_WITHIN`, which is stopped. The watch never
//! waits on it longer than `CALL_WAITED_ON_AT_MOST`: while it hasn't
//! answered, its last answer holds, or a call before any. Each time a call,
//! or a probe that can't tell, starts keeping him counted as here, the
//! run says so once. He's away once neither
//! has held for `--away-after` seconds. At the start, the last minute he
//! was there is the last line of `--away-seen`, rhythm's minutes, in
//! seconds since 1970; with none to read, the run counts him there as it
//! starts, so he must be gone `--away-after` within it before a bar eases.
//!
//! The mark is the account's own, so it says nothing of an account Joel
//! isn't signed into: there he always reads as away. A slice's eased bar
//! is that account's alone, but the machine's is the whole machine's, so
//! only the account he works in should ease it.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often a call is asked about while nothing else says he's there:
/// a program run each sample would cost more than the answer is worth.
const CALL_ASKED_EVERY: Duration = Duration::from_secs(30);

/// How long the call program may take before it counts as a call and is
/// stopped, so one that hangs holds no bar off for longer.
pub const CALL_ANSWERS_WITHIN: Duration = Duration::from_secs(10);

/// How long one sample waits for the call program before it reads the
/// slices; a quick program answers inside it, and a slow one is looked
/// at again on the next sample rather than holding the stops up.
const CALL_WAITED_ON_AT_MOST: Duration = Duration::from_secs(2);

/// How often a sample looks whether the call program has answered.
const CALL_LOOKED_AT_EVERY: Duration = Duration::from_millis(20);

/// How a job tells Joel is away, and the bars it has while he is.
#[derive(Debug, Clone, PartialEq)]
pub struct Away {
    pub mark: Option<PathBuf>,
    pub seen: Option<PathBuf>,
    pub call: Vec<String>,
    pub after: Duration,
    pub stops: Vec<(String, f64)>,
    pub stop_machine: Option<f64>,
}

impl Default for Away {
    fn default() -> Self {
        Away {
            mark: None,
            seen: None,
            call: Vec::new(),
            after: Duration::from_mins(30),
            stops: Vec::new(),
            stop_machine: None,
        }
    }
}

impl Away {
    /// Takes `option`'s `value` when it's one of these, saying whether it
    /// was; `number` reads a bar or a count of seconds as `run` does.
    pub fn take(
        &mut self,
        option: &str,
        value: &str,
        number: impl Fn(&str) -> Result<f64, String>,
    ) -> Result<bool, String> {
        match option {
            "--away-mark" => self.mark = Some(PathBuf::from(value)),
            "--away-seen" => self.seen = Some(PathBuf::from(value)),
            "--away-call" => {
                self.call = value.split_whitespace().map(str::to_owned).collect();
            }
            "--away-after" => self.after = Duration::from_secs_f64(number(value)?),
            "--away-stop-machine" => self.stop_machine = Some(number(value)?),
            "--away-stop" => {
                let (slice, bar) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--away-stop takes SLICE=PCT, not {value}"))?;
                self.stops.push((slice.to_owned(), number(bar)?));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Whether these bars can hold as given: each eases a slice the job
    /// watches, and some mark says when he's there.
    pub fn check(&self, watched: &[(String, f64)]) -> Result<(), String> {
        if self.stops.is_empty() && self.stop_machine.is_none() {
            return Ok(());
        }
        if self.mark.is_none() {
            return Err("an eased bar needs --away-mark to tell when Joel is away".into());
        }
        for (slice, _) in &self.stops {
            if !watched.iter().any(|(stopped, _)| stopped == slice) {
                return Err(format!(
                    "--away-stop {slice} eases a bar it hasn't, so give it a --stop too"
                ));
            }
        }
        Ok(())
    }

    /// Whether any bar eases, so the run need look for him at all.
    pub fn eases(&self) -> bool {
        !self.stops.is_empty() || self.stop_machine.is_some()
    }

    /// The bar `slice` has, `bar` unless he's away and it eases.
    pub fn slice_bar(&self, slice: &str, bar: f64, away: bool) -> f64 {
        if !away {
            return bar;
        }
        self.stops
            .iter()
            .find(|(eased, _)| eased == slice)
            .map_or(bar, |(_, eased)| *eased)
    }

    /// The machine's bar, `bar` unless he's away and it eases.
    pub fn machine_bar(&self, bar: f64, away: bool) -> f64 {
        if away {
            self.stop_machine.unwrap_or(bar)
        } else {
            bar
        }
    }

    /// The watch for a run starting now, from the last minute he was seen.
    pub fn watch(&self) -> Watch {
        let now = SystemTime::now();
        let seen = self
            .seen
            .as_ref()
            .and_then(|file| fs::read_to_string(file).ok())
            .and_then(|text| last_seen(&text));
        Watch {
            away: self.clone(),
            here_at: seen.unwrap_or(now).min(now),
            call_asked: None,
            asking: None,
            told: None,
            news: None,
        }
    }
}

/// The last minute a file of rhythm's minutes holds.
fn last_seen(text: &str) -> Option<SystemTime> {
    let seconds = text
        .lines()
        .rev()
        .find_map(|line| line.trim().parse().ok())?;
    UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

/// A run's look at whether he's away, kept from one sample to the next.
pub struct Watch {
    away: Away,
    here_at: SystemTime,
    call_asked: Option<(Instant, bool)>,
    asking: Option<(Child, Instant)>,
    told: Option<String>,
    news: Option<String>,
}

/// What the call program's status says: whether a call counts as on,
/// and, where it does, why.
fn call_answer(status: ExitStatus) -> (bool, String) {
    match status.code() {
        Some(0) => (true, "a call is on, so Joel counts as here".into()),
        Some(1) => (false, String::new()),
        Some(code) => (
            true,
            format!("the call program can't tell (it exited {code}), so it counts as a call"),
        ),
        None => (
            true,
            "the call program was killed, so it counts as a call".into(),
        ),
    }
}

impl Watch {
    /// Whether he's away now.
    pub fn is_away(&mut self) -> bool {
        let now = SystemTime::now();
        if self.away.mark.as_ref().is_some_and(|mark| mark.exists()) {
            self.here_at = now;
            return false;
        }
        let gone = now.duration_since(self.here_at).unwrap_or_default();
        if gone < self.away.after {
            return false;
        }
        if self.on_a_call() {
            self.here_at = now;
            return false;
        }
        true
    }

    /// What the run should say of why he counts as here, once each time
    /// a call, or a call program that can't tell, starts keeping him so.
    pub fn news(&mut self) -> Option<String> {
        self.news.take()
    }

    /// Whether a call is on, asked at most every `CALL_ASKED_EVERY`; a
    /// call it can't ask about, or can't tell about, counts as one.
    fn on_a_call(&mut self) -> bool {
        let Some((program, args)) = self.away.call.split_first() else {
            return false;
        };
        if self.asking.is_none() {
            if let Some((at, on)) = self.call_asked
                && at.elapsed() < CALL_ASKED_EVERY
            {
                return on;
            }
            match Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => self.asking = Some((child, Instant::now())),
                Err(error) => {
                    return self.answer(
                        true,
                        format!("the call program won't start ({error}), so it counts as a call"),
                    );
                }
            }
        }
        let waited_from = Instant::now();
        loop {
            let Some((child, began)) = self.asking.as_mut() else {
                return self.last_answer();
            };
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.asking = None;
                    let (on, why) = call_answer(status);
                    return self.answer(on, why);
                }
                Ok(None) if began.elapsed() < CALL_ANSWERS_WITHIN => {}
                Ok(None) | Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    self.asking = None;
                    let why = format!(
                        "the call program didn't answer within {}s, so it counts as a call",
                        CALL_ANSWERS_WITHIN.as_secs()
                    );
                    return self.answer(true, why);
                }
            }
            if waited_from.elapsed() >= CALL_WAITED_ON_AT_MOST {
                return self.last_answer();
            }
            // A short look again for the program's exit: try_wait doesn't
            // block, and a quick program answers within a few of these.
            thread::sleep(CALL_LOOKED_AT_EVERY);
        }
    }

    /// The answer before this one, while the program hasn't answered: a
    /// call, before any.
    fn last_answer(&self) -> bool {
        self.call_asked.is_none_or(|(_, on)| on)
    }

    /// Keeps the program's answer, with news where why he's here changed.
    fn answer(&mut self, on: bool, why: String) -> bool {
        self.call_asked = Some((Instant::now(), on));
        if !on {
            self.told = None;
        } else if self.told.as_deref() != Some(why.as_str()) {
            self.news = Some(why.clone());
            self.told = Some(why);
        }
        on
    }
}

impl Drop for Watch {
    /// A call program still running when the run ends goes with it.
    fn drop(&mut self) {
        if let Some((child, _)) = self.asking.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Result<f64, String> {
        text.parse().map_err(|_| format!("not a number: {text}"))
    }

    fn away(options: &[(&str, &str)]) -> Away {
        let mut away = Away::default();
        for (option, value) in options {
            assert!(
                away.take(option, value, number).unwrap(),
                "{option} wasn't taken"
            );
        }
        away
    }

    #[test]
    fn its_options_are_taken_and_others_left() {
        let taken = away(&[
            ("--away-mark", "/run/here"),
            ("--away-after", "60"),
            ("--away-call", "a-call-is-on  --here"),
            ("--away-stop", "app.slice=50"),
            ("--away-stop-machine", "12"),
        ]);
        assert_eq!(taken.mark, Some(PathBuf::from("/run/here")));
        assert_eq!(taken.after, Duration::from_secs(60));
        assert_eq!(taken.call, ["a-call-is-on", "--here"]);
        assert_eq!(taken.stops, vec![("app.slice".to_owned(), 50.0)]);
        assert_eq!(taken.stop_machine, Some(12.0));
        let mut other = Away::default();
        assert!(
            !other.take("--stop", "app.slice=1", number).unwrap(),
            "it took run's own --stop"
        );
    }

    #[test]
    fn an_eased_bar_needs_a_mark_and_a_bar_to_ease() {
        let watched = vec![("app.slice".to_owned(), 1.0)];
        let no_mark = away(&[("--away-stop", "app.slice=50")]);
        assert!(no_mark.check(&watched).is_err(), "it eased with no mark");
        let unwatched = away(&[
            ("--away-mark", "/run/here"),
            ("--away-stop", "session.slice=50"),
        ]);
        let refused = unwatched.check(&watched).unwrap_err();
        assert!(refused.contains("session.slice"), "{refused}");
        assert!(
            Away::default().check(&watched).is_ok(),
            "a job easing nothing was refused"
        );
    }

    #[test]
    fn a_bar_eases_only_while_away() {
        let eased = away(&[
            ("--away-mark", "/run/here"),
            ("--away-stop", "app.slice=50"),
            ("--away-stop-machine", "12"),
        ]);
        let here = eased.slice_bar("app.slice", 1.0, false);
        assert!(
            (here - 1.0).abs() < f64::EPSILON,
            "with him here the slice's bar was {here}, not its own 1"
        );
        let gone = eased.slice_bar("app.slice", 1.0, true);
        assert!(
            (gone - 50.0).abs() < f64::EPSILON,
            "with him away the slice's bar was {gone}, not the eased 50"
        );
        assert!(
            (eased.slice_bar("session.slice", 1.0, true) - 1.0).abs() < f64::EPSILON,
            "a slice it doesn't ease eased"
        );
        let here = eased.machine_bar(25.0, false);
        assert!(
            (here - 25.0).abs() < f64::EPSILON,
            "with him here the machine's bar was {here}, not its own 25"
        );
        let gone = eased.machine_bar(25.0, true);
        assert!(
            (gone - 12.0).abs() < f64::EPSILON,
            "with him away the machine's bar was {gone}, not the eased 12"
        );
    }

    #[test]
    fn why_a_call_keeps_him_here_is_news_once_each_time_it_starts() {
        let mut watch = away(&[("--away-mark", "/run/here")]).watch();
        assert!(watch.last_answer(), "no answer yet didn't count as a call");
        assert!(
            watch.answer(true, "a call".into()),
            "a call didn't keep him here"
        );
        assert_eq!(watch.news().as_deref(), Some("a call"));
        assert!(
            watch.answer(true, "a call".into()),
            "a call going on didn't keep him here"
        );
        assert_eq!(watch.news(), None, "the same reason was said twice");
        assert!(!watch.answer(false, String::new()), "no call kept him here");
        assert!(!watch.last_answer(), "no call didn't hold until the next");
        assert!(
            watch.answer(true, "a call".into()),
            "a call after none didn't keep him here"
        );
        assert_eq!(
            watch.news().as_deref(),
            Some("a call"),
            "a call after none wasn't said again"
        );
    }

    #[test]
    fn the_last_minute_seen_is_the_last_number() {
        let seen = last_seen("1700000000\n1700000060\n\n").unwrap();
        assert_eq!(seen, UNIX_EPOCH + Duration::from_secs(1_700_000_060));
        assert!(last_seen("").is_none(), "an empty file had a minute");
    }
}
