//! What rhythm keeps, where, and the loop that keeps it.
//!
//! The minutes Joel was at the machine go in `$XDG_STATE_HOME/rhythm/seen`,
//! one per line in seconds since 1970, kept for 120 days: swayidle-away's
//! mark (`$XDG_RUNTIME_DIR/someone-at-the-machine`), there for two minutes
//! after a key or a touch, is looked at each minute. The first run seeds
//! the file from what the logs hold (seed.rs), and keeps the minute it
//! began watching in `$XDG_STATE_HOME/rhythm/watched`, since a seeded
//! minute is read differently (nights.rs); seen minutes kept without it
//! were seeded by an older rhythm that read the harness's words as Joel's,
//! so they're seeded afresh. The profile is written to
//! `$XDG_STATE_HOME/rhythm/profile.json` whenever it changes, whole, by a
//! rename, so a reader never sees half of it. Each look also asks whether
//! a walk is due (walk.rs), and suggests it with a notice from Daylight;
//! the day's suggestions are kept in `$XDG_STATE_HOME/rhythm/walks`, so a
//! restart doesn't make a fourth.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::clock::{days_from_civil, local};
use crate::nights::{Profile, evening_of, nights, weekday_of};
use crate::read::{Sun, profile_json, said};
use crate::seed;
use crate::walk::{self, Look, Weather};

/// How long the minutes seen are kept, in days: a season's worth, which
/// the weekday and four-week readings need well under.
const KEEP_DAYS: i64 = 120;

/// Where rhythm reads and writes.
pub struct Paths {
    pub seen: PathBuf,
    pub watched: PathBuf,
    pub profile: PathBuf,
    pub walks: PathBuf,
    pub mark: PathBuf,
    pub quiet: PathBuf,
    pub whereabouts: PathBuf,
    pub zsh: PathBuf,
    pub pi: PathBuf,
}

impl Paths {
    /// The paths from the session's environment, or none without a home
    /// or a runtime folder.
    pub fn from_env() -> Option<Paths> {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
        let state = std::env::var_os("XDG_STATE_HOME")
            .map_or_else(|| home.join(".local/state"), PathBuf::from)
            .join("rhythm");
        Some(Paths {
            seen: state.join("seen"),
            watched: state.join("watched"),
            profile: state.join("profile.json"),
            walks: state.join("walks"),
            mark: runtime.join("someone-at-the-machine"),
            quiet: runtime.join("quiet"),
            whereabouts: runtime.join("whereabouts/here.json"),
            zsh: std::env::var_os("HISTFILE")
                .map_or_else(|| home.join(".zsh_history"), PathBuf::from),
            pi: home.join(".pi/agent/sessions"),
        })
    }
}

/// The start of the minute holding a moment.
fn minute(seconds: i64) -> i64 {
    seconds - seconds.rem_euclid(60)
}

/// Seconds since 1970 now.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// Minutes, sorted, each once, from the last `KEEP_DAYS` before `now`.
fn kept(mut seen: Vec<i64>, now: i64) -> Vec<i64> {
    let since = now - KEEP_DAYS * 86_400;
    seen.retain(|moment| (since..=now).contains(moment));
    for moment in &mut seen {
        *moment = minute(*moment);
    }
    seen.sort_unstable();
    seen.dedup();
    seen
}

/// The minutes the file holds, or none where it can't be read.
fn read_seen(path: &Path) -> Option<Vec<i64>> {
    let text = fs::read_to_string(path).ok()?;
    Some(
        text.lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect(),
    )
}

/// Writes a file whole, by a rename.
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let fresh = path.with_extension("new");
    fs::write(&fresh, text)?;
    fs::rename(&fresh, path)
}

/// The minutes the logs hold from before rhythm ran.
fn seeded(paths: &Paths, now: i64) -> Vec<i64> {
    let mut found = fs::read(&paths.zsh)
        .map(|history| seed::zsh(&history))
        .unwrap_or_default();
    let since = SystemTime::now() - Duration::from_secs(KEEP_DAYS.unsigned_abs() * 86_400);
    found.extend(seed::pi_sessions(&paths.pi, since));
    kept(found, now)
}

/// Where whereabouts says the laptop is, in degrees north and east.
fn place(text: &str) -> Option<(f64, f64)> {
    let figure = |key: &str| -> Option<f64> {
        let after = text.split_once(&format!("\"{key}\""))?.1;
        let value = after.trim_start().strip_prefix(':')?.trim_start();
        let end = value
            .find(|letter: char| !(letter.is_ascii_digit() || "+-.eE".contains(letter)))
            .unwrap_or(value.len());
        value[..end]
            .parse()
            .ok()
            .filter(|number: &f64| number.is_finite())
    };
    let (latitude, longitude) = (figure("latitude")?, figure("longitude")?);
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude))
        .then_some((latitude, longitude))
}

/// Today's sunrise and sunset by the local clock where whereabouts says
/// the laptop is, or none where it says nowhere or the sun doesn't cross.
fn sun_today(paths: &Paths, now: i64) -> Option<Sun> {
    let (latitude, longitude) = place(&fs::read_to_string(&paths.whereabouts).ok()?)?;
    let today = local(now)?;
    let at = |rising: bool| -> Option<u32> {
        let sun::Crossing::At(minutes) = sun::crosses(
            today.year,
            today.month,
            today.day,
            latitude,
            longitude,
            rising,
        ) else {
            return None;
        };
        // Whole minutes after the day's midnight in UTC, which the local
        // clock then says.
        #[expect(clippy::cast_possible_truncation, reason = "a day's minutes fit")]
        let moment = days_from_civil(today.year, today.month, today.day) * 86_400
            + minutes.round() as i64 * 60;
        Some(local(moment)?.minute)
    };
    Some(Sun {
        rise: at(true)?,
        set: at(false)?,
    })
}

/// How long the weather asked for is trusted, in seconds.
const WEATHER_KEPT: i64 = 30 * 60;

/// The walks suggested today, and the weather last asked for.
struct Walker {
    walks: Vec<i64>,
    weather: Option<(i64, Option<Weather>)>,
}

impl Walker {
    fn new(paths: &Paths) -> Walker {
        Walker {
            walks: read_seen(&paths.walks).unwrap_or_default(),
            weather: None,
        }
    }

    /// Suggests a walk when one is due and nothing stands in its way.
    fn look(&mut self, paths: &Paths, seen: &[i64], now: i64, sun: Option<Sun>) {
        let (Some(today), Some(sun)) = (local(now), sun) else {
            return;
        };
        let midnight = minute(now) - i64::from(today.minute) * 60;
        self.walks.retain(|walk| *walk >= midnight);
        let look = Look {
            now,
            minute: today.minute,
            at_keys: paths.mark.exists(),
            seen,
            walks: &self.walks,
            sun: Some(&sun),
        };
        let quiet = fs::read_to_string(&paths.quiet).ok();
        if !walk::due(&look) || walk::quiet(quiet.as_deref(), now * 1000) || call_on() {
            return;
        }
        let weather = self.weather(paths, now);
        if !walk::room(self.walks.len(), weather.as_ref()) {
            return;
        }
        let (question, out) = walk::said(weather.as_ref(), sun.set);
        match Command::new("notify-send")
            .args(["--app-name=daylight", &question, &out])
            .status()
        {
            Ok(status) if status.success() => {
                eprintln!("<6>suggested a walk: {question} {out}");
                self.walks.push(now);
                let lines: Vec<String> = self.walks.iter().map(ToString::to_string).collect();
                if let Err(error) = write_whole(&paths.walks, &(lines.join("\n") + "\n")) {
                    eprintln!("<4>rhythm can't keep today's walks: {error}");
                }
            }
            Ok(status) => eprintln!("<4>notify-send couldn't suggest a walk: {status}"),
            Err(error) => eprintln!("<4>can't run notify-send to suggest a walk: {error}"),
        }
    }

    /// The weather where the laptop is, asked of Open-Meteo at most each
    /// half hour, and only once a walk is due; none where it can't be had.
    fn weather(&mut self, paths: &Paths, now: i64) -> Option<Weather> {
        if let Some((at, weather)) = &self.weather
            && now - at < WEATHER_KEPT
        {
            return weather.clone();
        }
        let weather = fs::read_to_string(&paths.whereabouts)
            .ok()
            .as_deref()
            .and_then(place)
            .and_then(|(latitude, longitude)| {
                Command::new("curl")
                    .args([
                        "-fsS",
                        "--max-time",
                        "10",
                        &walk::asked(latitude, longitude),
                    ])
                    .output()
                    .ok()
            })
            .filter(|answer| answer.status.success())
            .and_then(|answer| Weather::read(&String::from_utf8_lossy(&answer.stdout)));
        self.weather = Some((now, weather.clone()));
        weather
    }
}

/// Whether a call is on, as a-call-is-on says by exiting 0; a call it
/// can't tell of holds nothing back.
fn call_on() -> bool {
    Command::new("a-call-is-on")
        .status()
        .is_ok_and(|status| status.code() == Some(0))
}

/// The profile, the evening's weekday it's for, and the sun, as of now.
fn reading(paths: &Paths, seen: &[i64], watched: i64, now: i64) -> (Profile, u32, Option<Sun>) {
    let evening = local(now).map_or(now.div_euclid(86_400), evening_of);
    let profile = Profile::of(&nights(seen, watched, local), evening);
    (profile, weekday_of(evening), sun_today(paths, now))
}

/// What Joel reads with `rhythm`, from what's kept, or the logs where
/// nothing is kept yet.
pub fn read(paths: &Paths) -> String {
    let now = now();
    let (seen, watched) = match (read_seen(&paths.seen), watched(paths)) {
        (Some(seen), Some(watched)) => (seen, watched),
        _ => (seeded(paths, now), now),
    };
    let (profile, weekday, sun) = reading(paths, &seen, watched, now);
    said(&profile, weekday, sun)
}

/// The minute rhythm began watching, where it's kept.
fn watched(paths: &Paths) -> Option<i64> {
    read_seen(&paths.watched)?.first().copied()
}

/// Keeps the minutes Joel is at the machine, and the profile they make,
/// for as long as the session runs.
pub fn keep(paths: &Paths) -> std::io::Result<()> {
    let (mut seen, watched) =
        if let (Some(seen), Some(watched)) = (read_seen(&paths.seen), watched(paths)) {
            (kept(seen, now()), watched)
        } else {
            let seen = seeded(paths, now());
            eprintln!(
                "<6>rhythm seeded {} minutes at the machine from the logs",
                seen.len()
            );
            (seen, minute(now()))
        };
    let lines: Vec<String> = seen.iter().map(ToString::to_string).collect();
    write_whole(&paths.seen, &(lines.join("\n") + "\n"))?;
    write_whole(&paths.watched, &format!("{watched}\n"))?;
    let mut written = fs::read_to_string(&paths.profile).unwrap_or_default();
    let mut walker = Walker::new(paths);
    loop {
        let now = now();
        if paths.mark.exists() && seen.last() != Some(&minute(now)) {
            seen.push(minute(now));
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&paths.seen)?;
            writeln!(file, "{}", minute(now))?;
        }
        let (profile, weekday, sun) = reading(paths, &seen, watched, now);
        let json = profile_json(&profile, weekday, sun);
        if json != written {
            write_whole(&paths.profile, &json)?;
            written = json;
        }
        walker.look(paths, &seen, now, sun);
        // The mark lasts two minutes after a key, so a look each minute
        // sees every minute Joel is there, and costs nothing between.
        std::thread::sleep(Duration::from_mins(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_are_kept_once_sorted_and_for_a_season() {
        let now = 200 * 86_400;
        let seen = vec![now - 30, now - 59, now - 200 * 86_400, now - 90, now + 600];
        assert_eq!(
            kept(seen, now),
            vec![now - 120, now - 60],
            "each minute once, none from the future or past a season"
        );
    }

    #[test]
    fn whereabouts_says_where_the_laptop_is() {
        let said = r#"{"v":2,"latitude":43.6748,"longitude":-79.3794,"accuracy":26000}"#;
        assert_eq!(place(said), Some((43.6748, -79.3794)), "Toronto");
        assert_eq!(
            place(r#"{"latitude": 91, "longitude": 0}"#),
            None,
            "past the pole"
        );
    }
}
