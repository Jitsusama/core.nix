//! The detectors, each a function from how things stand to what it finds.
//! A threshold is set for a stated false-alarm rate on Joel's own history
//! where there's one to set (the burn's, by its run length between false
//! alarms), and many cuts tested at once are held to a false discovery
//! rate (the peers'). Each finding says what drove it and what to do.

use crate::config::Config;
use crate::findings::{Action, Finding, Severity};
use crate::history::History;
use crate::money::{dollars, percent, whole};
use crate::picture::Picture;
use crate::stats;
use crate::time::{DAY, HOUR, Zone};
use augurs_changepoint::{Detector, NormalGammaDetector};
use std::collections::{BTreeMap, HashMap, HashSet};

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

fn finding(
    key: String,
    detector: &str,
    severity: Severity,
    title: String,
    detail: String,
) -> Finding {
    Finding {
        key,
        detector: detector.into(),
        severity,
        title,
        detail,
        dollars: 0.0,
        surprise: 1.0,
        badge: String::new(),
        session: None,
        quest: None,
        action: None,
        cap: false,
        started: 0.0,
        marked: false,
    }
}

/// A short, stable name for something too long or too private to be a
/// key itself: a credential, a sentence a provider said. FNV-1a, 64 bits.
fn digest(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// What a provider said to the person in the last quarter hour: a warning
/// when it was one of its own, a note otherwise. Its text is drawn as
/// plain words, never markup.
pub fn said(picture: &Picture) -> Vec<Finding> {
    picture
        .said
        .iter()
        .map(|(severity, text)| {
            let plain: String = text.chars().filter(|c| !matches!(c, '<' | '>')).collect();
            let severity = match severity.as_str() {
                "critical" | "error" | "alert" | "warning" => Severity::Warning,
                _ => Severity::Note,
            };
            let mut f = finding(
                format!("said:{}", digest(text)),
                "said",
                severity,
                format!("the provider says: {plain}"),
                "Said by the provider's own meter.".into(),
            );
            f.surprise = 0.5;
            f
        })
        .collect()
}

/// What a feed offers for a kind of problem, as the finding's action.
fn offered(picture: &Picture, fits: &str) -> Option<Action> {
    picture
        .actions
        .iter()
        .find(|action| action.fits.iter().any(|fit| fit == fits))
        .map(|action| Action {
            name: action.name.clone(),
            run: action.run.clone(),
        })
}

fn go_to(session: &str) -> Action {
    Action {
        name: "Go to the session".into(),
        run: vec!["spend-watch".into(), "go".into(), session.into()],
    }
}

pub fn all(picture: &Picture, history: &History, config: &Config) -> Result<Vec<Finding>, String> {
    let mut found = Vec::new();
    found.extend(caps(picture));
    found.extend(refusals(picture));
    found.extend(burn(picture, history, config)?);
    found.extend(waste(picture, config));
    found.extend(peers(picture, config));
    found.extend(stalled(picture, config));
    found.extend(regimes(picture));
    found.extend(mix(picture, config));
    found.extend(novelty(picture, history, config)?);
    found.extend(spikes(picture));
    found.extend(health(picture));
    found.extend(said(picture));
    let quiet: HashSet<&str> = config.detectors.quiet.iter().map(String::as_str).collect();
    for finding in &mut found {
        if quiet.contains(finding.detector.as_str()) && !finding.cap {
            finding.severity = finding.severity.min(Severity::Note);
        }
    }
    Ok(found)
}

/// The days of history a burn's threshold is judged on: long enough that
/// its quantile is more than the largest day of a short window.
pub const BURN_DAYS: usize = 56;

pub fn caps(picture: &Picture) -> Vec<Finding> {
    let mut out = Vec::new();
    let now = picture.now;
    for view in &picture.caps {
        let cap = &view.cap;
        let key = |what: &str| format!("cap:{}:{}:{what}", cap.feed, cap.name);
        let ask = offered(picture, "cap");
        let Some(allowance) = cap.allowance else {
            continue;
        };
        let share = view.spent / allowance;
        let over = view.spent >= allowance || cap.state == "refused";
        // Where the spend is counted: a rolling window, or a period that
        // resets whatever is spent.
        let counted = match view.resets {
            Some(at) => format!(", resetting at {}", picture.zone.clock(at, now)),
            None => format!(" in the last {} hours", cap.window_hours),
        };
        if over {
            let frees = match view.resets {
                Some(_) => String::new(),
                None => view
                    .clock
                    .as_ref()
                    .and_then(|clock| clock.frees_at)
                    .map(|at| {
                        format!(
                            " It's back under by {} if nothing more is spent.",
                            picture.zone.clock(at, now)
                        )
                    })
                    .unwrap_or_default(),
            };
            let mut f = finding(
                key("over"),
                "caps",
                Severity::Alert,
                format!("over the {}", cap.label),
                format!(
                    "{} of {}{counted}.{frees}",
                    whole(view.spent),
                    whole(allowance),
                ),
            );
            f.badge = view
                .clock
                .as_ref()
                .and_then(|clock| clock.frees_at)
                .map(|at| format!("back {}", picture.zone.clock(at, now)))
                .unwrap_or_else(|| percent(share));
            f.dollars = view.spent - allowance;
            f.surprise = 3.0;
            f.cap = true;
            f.action = ask.clone();
            out.push(f);
            continue;
        }
        if let Some(clock) = &view.clock {
            let chance = clock.cap_within_day;
            if chance >= 0.2 {
                let mut f = finding(
                    key("close"),
                    "caps",
                    if chance >= 0.5 {
                        Severity::Alert
                    } else {
                        Severity::Warning
                    },
                    format!(
                        "{} chance of the {} within a day",
                        percent(chance),
                        cap.label
                    ),
                    format!(
                        "{} of {} spent{}; by the forecast's paths.",
                        whole(view.spent),
                        whole(allowance),
                        match view.resets {
                            Some(_) => counted.clone(),
                            None => format!(
                                ", {} rolling out in the next day",
                                whole(clock.freeing_in_day)
                            ),
                        }
                    ),
                );
                f.badge = percent(share);
                f.dollars = allowance - view.spent;
                f.surprise = 2.0;
                f.cap = true;
                f.action = ask.clone();
                out.push(f);
            } else if share >= cap.warn {
                let mut f = finding(
                    key("near"),
                    "caps",
                    Severity::Warning,
                    format!("{} of the {}", percent(share), cap.label),
                    format!("{} of {}{counted}.", whole(view.spent), whole(allowance)),
                );
                f.badge = percent(share);
                f.dollars = allowance - view.spent;
                f.cap = true;
                f.action = ask.clone();
                out.push(f);
            }
            // An override ending, warned ahead when what follows it is likely
            // to be over at once.
            if let (Some(ends_at), Some(after), Some(fallback)) =
                (view.ends_at, clock.over_after_end, cap.fallback)
            {
                let days_left = (ends_at - now) / DAY;
                let lead = picture.lead;
                if days_left > 0.0 && days_left <= lead.0 as f64 && after >= 0.5 {
                    let day = Zone::day_name(picture.zone.day(ends_at - 1.0));
                    let weekday = &day[..3];
                    let mut f = finding(
                        key("ending"),
                        "caps",
                        if days_left <= lead.1 as f64 {
                            Severity::Alert
                        } else {
                            Severity::Warning
                        },
                        format!("the {} override ends {day}", whole(allowance)),
                        format!(
                            "At this pace, a {} chance the spend is over the {} that follows \
                             the moment it ends.",
                            percent(after),
                            whole(fallback)
                        ),
                    );
                    f.badge = format!("ends {weekday}");
                    f.dollars = view.spent;
                    f.surprise = 2.5;
                    f.cap = true;
                    f.action = ask.clone();
                    out.push(f);
                }
            }
        }
    }
    for event in picture.events.iter().filter(|event| now - event.at < DAY) {
        let (severity, title) = match event.kind.as_str() {
            "lapsed" => (
                Severity::Alert,
                format!(
                    "an override lapsed: the cap is {}",
                    event.after.map_or("none".into(), whole)
                ),
            ),
            "lowered" => (
                Severity::Alert,
                format!(
                    "the cap fell to {}",
                    event.after.map_or("none".into(), whole)
                ),
            ),
            "raised" => (
                Severity::Note,
                format!(
                    "the cap rose to {}",
                    event.after.map_or("no limit".into(), whole)
                ),
            ),
            _ => continue,
        };
        let mut f = finding(
            format!(
                "cap-event:{}:{}:{}:{}",
                event.feed, event.name, event.kind, event.at as i64
            ),
            "caps",
            severity,
            title,
            format!(
                "At {}, with {} spent in the window; it was {}.",
                picture.zone.clock(event.at, now),
                whole(event.spent),
                event.before.map_or("no limit".into(), whole)
            ),
        );
        f.badge = event.after.map_or(String::new(), whole);
        f.dollars = event.spent;
        f.surprise = 3.0;
        f.cap = true;
        f.action = if severity == Severity::Note {
            None
        } else {
            offered(picture, "cap")
        };
        out.push(f);
    }
    out
}

/// A refusal in pi's logs in the last six hours: cut off, whatever a feed
/// says or whether there is one.
pub fn refusals(picture: &Picture) -> Vec<Finding> {
    let recent: Vec<&(f64, String)> = picture
        .refusals
        .iter()
        .filter(|(at, _)| picture.now - at < 6.0 * HOUR)
        .collect();
    let Some(last) = recent.last() else {
        return Vec::new();
    };
    let mut f = finding(
        format!("refused:{}", picture.zone.day(last.0)),
        "caps",
        Severity::Alert,
        "refused for being over a cap".into(),
        format!(
            "{} of pi's requests refused since {}, the last at {}. Your spend is still readable.",
            recent.len(),
            picture.zone.clock(recent[0].0, picture.now),
            picture.zone.clock(last.0, picture.now)
        ),
    );
    f.badge = "refused".into();
    f.surprise = 3.0;
    f.cap = true;
    f.action = offered(picture, "cap");
    vec![f]
}

/// Spend by minute from the logs, at list prices, since `from`.
fn minutes(history: &History, from: f64) -> Result<BTreeMap<i64, (f64, String)>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT CAST(at / 60 AS INTEGER), session, SUM(cost) FROM turns WHERE at >= ?1
             GROUP BY 1, 2",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([from], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(fail)?;
    // Each minute's spend, and the session that spent most of it.
    let mut out: BTreeMap<i64, (f64, String)> = BTreeMap::new();
    let mut most: HashMap<i64, f64> = HashMap::new();
    for row in rows {
        let (minute, session, spent) = row.map_err(fail)?;
        let entry = out.entry(minute).or_insert((0.0, String::new()));
        entry.0 += spent;
        let top = most.entry(minute).or_insert(f64::NEG_INFINITY);
        if spent > *top {
            *top = spent;
            entry.1 = session;
        }
    }
    Ok(out)
}

/// A burn: spend running far above the rate a usual hour like this one
/// spends at, found by CUSUM minute by minute. Its allowance is twice the
/// expected rate; its threshold is set so that on the last `BURN_DAYS`
/// days it would have alarmed once in the configured run length.
pub fn burn(picture: &Picture, history: &History, config: &Config) -> Result<Vec<Finding>, String> {
    let now = picture.now;
    let from = now - BURN_DAYS as f64 * DAY;
    let by_minute = minutes(history, from)?;
    if by_minute.is_empty() {
        return Ok(Vec::new());
    }
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let zone = &picture.zone;
    // A usual active day's spend, and the hour shape, give each minute's
    // expected rate.
    let usual_day = stats::median(
        &picture
            .days
            .dollars
            .iter()
            .rev()
            .skip(1)
            .take(28)
            .copied()
            .filter(|d| *d >= 5.0)
            .collect::<Vec<_>>(),
    );
    if !usual_day.is_finite() || usual_day <= 0.0 {
        return Ok(Vec::new());
    }
    let allowance = |minute: i64| {
        let at = minute as f64 * 60.0;
        let share = picture.shape.shares[Zone::weekday(zone.day(at))][zone.local_hour(at) as usize];
        // Twice the usual rate, and never less than a dollar an hour.
        (2.0 * usual_day * share / 60.0).max(1.0 / 60.0)
    };
    let first = (from / 60.0) as i64;
    let last = (now / 60.0) as i64;
    let mut score = 0.0;
    let mut daily_max: BTreeMap<i64, f64> = BTreeMap::new();
    let mut last_hour: Vec<(f64, String)> = Vec::new();
    for minute in first..=last {
        let (spent, session) = by_minute
            .get(&minute)
            .cloned()
            .unwrap_or((0.0, String::new()));
        let spent = spent * ratio;
        score = f64::max(0.0, score + spent - allowance(minute));
        let day = zone.day(minute as f64 * 60.0);
        let entry = daily_max.entry(day).or_default();
        *entry = entry.max(score);
        if minute > last - 60 {
            last_hour.push((spent, session));
        }
    }
    let today = zone.day(now);
    let past: Vec<f64> = daily_max
        .iter()
        .filter(|(day, _)| **day < today)
        .map(|(_, max)| *max)
        .collect();
    if past.len() < 14 {
        return Ok(Vec::new());
    }
    let rate = 1.0 / config.detectors.burn_run_length_days;
    // The threshold: what the daily maximum passed on that share of days,
    // and never below an hour of twice the usual rate.
    let threshold = stats::quantile(&past, 1.0 - rate).max(usual_day / 12.0);
    if score <= threshold {
        return Ok(Vec::new());
    }
    // The score falls slowly once spending stops, so a burn is also the
    // last hour past what it allows, from a session that spent it.
    let spent_hour: f64 = last_hour.iter().map(|(spent, _)| spent).sum();
    let allowed_hour: f64 = (last - 59..=last).map(allowance).sum();
    if spent_hour <= allowed_hour {
        return Ok(Vec::new());
    }
    let mut by_session: HashMap<&str, f64> = HashMap::new();
    for (spent, session) in &last_hour {
        if !session.is_empty() && *spent > 0.0 {
            *by_session.entry(session.as_str()).or_default() += spent;
        }
    }
    let Some(top) = by_session
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(s, _)| s.to_owned())
    else {
        return Ok(Vec::new());
    };
    let quest = history
        .db
        .query_row(
            "SELECT quest FROM turns WHERE session = ?1 ORDER BY at DESC LIMIT 1",
            [&top],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    let title_of = quest
        .as_ref()
        .and_then(|id| picture.quests.iter().find(|q| &q.id == id))
        .map(|q| q.title.clone());
    let mut f = finding(
        format!("burn:{top}"),
        "burn",
        Severity::Alert,
        match &title_of {
            Some(title) => format!("a burn in {title}"),
            None => "a burn".into(),
        },
        format!(
            "{} in the last hour against a usual {} for this hour; past what eight weeks crossed \
             once a {}.",
            dollars(spent_hour),
            dollars(picture.expected_rate()),
            if config.detectors.burn_run_length_days >= 28.0 {
                "month".to_owned()
            } else {
                format!("{} days", config.detectors.burn_run_length_days)
            }
        ),
    );
    f.badge = format!("{}/h", whole(spent_hour));
    f.dollars = spent_hour;
    f.surprise = -(rate.log10());
    f.session = Some(top.clone());
    f.quest = quest;
    f.action = Some(go_to(&top));
    Ok(vec![f])
}

pub fn waste(picture: &Picture, config: &Config) -> Vec<Finding> {
    let waste = &picture.waste;
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let rebuilt = waste.rebuilt * ratio;
    if rebuilt < config.detectors.floor.max(100.0) {
        return Vec::new();
    }
    let Some((cause, amount)) = waste.causes.first() else {
        return Vec::new();
    };
    let mut detail = format!(
        "{} of this week's {} went on rebuilding context a session already had, the most of it \
         ({}) {}.",
        dollars(rebuilt),
        dollars(waste.spent * ratio),
        dollars(amount * ratio),
        crate::logs::Cause::words(cause)
    );
    let mut f = finding(
        "waste:week".into(),
        "waste",
        Severity::Note,
        format!("{} of context rebuilt this week", whole(rebuilt)),
        String::new(),
    );
    if let Some((session, worst)) = &waste.worst
        && worst * ratio >= config.detectors.floor
    {
        detail.push_str(&format!(
            " {} of it after pauses in one session.",
            dollars(worst * ratio)
        ));
        f.session = Some(session.clone());
        f.action = Some(go_to(session));
    }
    f.detail = detail;
    f.dollars = rebuilt;
    f.surprise = 1.0;
    vec![f]
}

pub fn peers(picture: &Picture, config: &Config) -> Vec<Finding> {
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let recent: Vec<crate::waste::Work> = picture.work.clone();
    crate::waste::outliers(
        &recent,
        config.detectors.false_discovery,
        config.detectors.floor / ratio.max(0.01),
    )
    .into_iter()
    .take(3)
    .map(|outlier| {
        let work = &outlier.work;
        let mut f = finding(
            format!("peer:{}", work.session),
            "peers",
            Severity::Note,
            match work
                .quest
                .as_deref()
                .map(|quest| picture.title(picture.root(quest)))
            {
                Some(title) => {
                    format!(
                        "a session of {title} at {:.1} times the usual per turn",
                        outlier.times
                    )
                }
                None => format!("a session at {:.1} times the usual per turn", outlier.times),
            },
            format!(
                "{} over {} turns; {} of its input from cache against {} usual. Held to a {} \
                     false discovery rate across {} sessions.",
                dollars(work.dollars * ratio),
                work.turns,
                percent(work.cached),
                percent(outlier.usual_cached),
                percent(config.detectors.false_discovery),
                recent.len()
            ),
        );
        f.dollars = outlier.excess * ratio;
        f.surprise = -outlier.p.max(1e-12).log10();
        f.session = Some(work.session.clone());
        f.quest = work.quest.clone();
        f.action = Some(go_to(&work.session));
        f
    })
    .collect()
}

/// A quest spending while it doesn't move: real money over the last week
/// and no update to its record in five days.
pub fn stalled(picture: &Picture, config: &Config) -> Vec<Finding> {
    let mut spend: HashMap<&str, f64> = HashMap::new();
    for piece in &picture.pieces_week {
        if let Some(quest) = &piece.quest {
            *spend.entry(picture.root(quest)).or_default() += piece.dollars;
        }
    }
    let today = picture.zone.day(picture.now);
    let mut out = Vec::new();
    for quest in &picture.quests {
        if quest.status != "active" {
            continue;
        }
        let spent = spend.get(quest.id.as_str()).copied().unwrap_or(0.0);
        let updated = quest.updated.as_deref().and_then(crate::time::parse_day);
        let quiet_days = updated.map_or(0, |day| today - day);
        if spent >= config.detectors.floor.max(100.0) && quiet_days >= 5 {
            let mut f = finding(
                format!("stalled:{}", quest.id),
                "outcomes",
                Severity::Note,
                format!(
                    "{} spent on {} while it hasn't moved",
                    whole(spent),
                    quest.title
                ),
                format!(
                    "{} this week; its record last changed {quiet_days} days ago, with {} pull \
                     requests.",
                    dollars(spent),
                    quest.prs
                ),
            );
            f.dollars = spent;
            f.quest = Some(quest.id.clone());
            out.push(f);
        }
    }
    out
}

/// A change of regime in daily spend: Bayesian online changepoints on the
/// standardized log of each day, a change counted once a week has passed
/// after it and the days since differ from the days before by a quarter.
pub fn regimes(picture: &Picture) -> Vec<Finding> {
    let daily = &picture.days.dollars;
    let n = daily.len().saturating_sub(1); // today is still going
    if n < 56 {
        return Vec::new();
    }
    let logs: Vec<f64> = daily[..n].iter().map(|d| d.ln_1p()).collect();
    let centre = stats::mean(&logs);
    let spread = (logs.iter().map(|x| (x - centre).powi(2)).sum::<f64>() / n as f64)
        .sqrt()
        .max(1e-6);
    let z: Vec<f64> = logs.iter().map(|x| (x - centre) / spread).collect();
    let changes = NormalGammaDetector::default().detect_changepoints(&z);
    let Some(&at) = changes.iter().rev().find(|&&at| at + 7 <= n && at > 0) else {
        return Vec::new();
    };
    if n - at > 21 {
        return Vec::new();
    }
    let before_from = changes
        .iter()
        .rev()
        .find(|&&c| c < at)
        .copied()
        .unwrap_or(0);
    let usual = |days: &[f64]| {
        stats::median(
            &days
                .iter()
                .copied()
                .filter(|d| *d >= 5.0)
                .collect::<Vec<_>>(),
        )
    };
    let (before, after) = (usual(&daily[before_from..at]), usual(&daily[at..n]));
    if !before.is_finite() || !after.is_finite() || (after / before - 1.0).abs() < 0.25 {
        return Vec::new();
    }
    let day = picture.days.first + at as i64;
    let mut f = finding(
        format!("regime:{day}"),
        "regimes",
        Severity::Note,
        format!(
            "since {} a usual day is {}",
            Zone::day_name(day),
            whole(after)
        ),
        format!(
            "{} from {}; found by online changepoints on daily spend.",
            if after > before { "up" } else { "down" },
            whole(before)
        ),
    );
    f.dollars = (after - before).abs() * 7.0;
    f.surprise = 2.0;
    vec![f]
}

/// Today's split by model against its usual, by Jensen-Shannon divergence,
/// judged against how far each of the last four weeks' days strayed.
pub fn mix(picture: &Picture, config: &Config) -> Vec<Finding> {
    let zone = &picture.zone;
    let today = zone.day(picture.now);
    let mut by_day: BTreeMap<i64, HashMap<String, f64>> = BTreeMap::new();
    for piece in &picture.pieces_month {
        let day = zone.day(crate::time::hour_start(piece.hour));
        *by_day
            .entry(day)
            .or_default()
            .entry(piece.model.clone())
            .or_default() += piece.dollars;
    }
    let models: Vec<String> = {
        let mut all: HashSet<String> = HashSet::new();
        for day in by_day.values() {
            all.extend(day.keys().cloned());
        }
        let mut all: Vec<String> = all.into_iter().collect();
        all.sort();
        all
    };
    let vector = |day: &HashMap<String, f64>| {
        models
            .iter()
            .map(|m| day.get(m).copied().unwrap_or(0.0))
            .collect::<Vec<_>>()
    };
    let usual: Vec<f64> = models
        .iter()
        .map(|m| {
            by_day
                .iter()
                .filter(|(d, _)| **d < today)
                .map(|(_, day)| day.get(m).copied().unwrap_or(0.0))
                .sum()
        })
        .collect();
    let Some(now) = by_day.get(&today) else {
        return Vec::new();
    };
    let spent: f64 = now.values().sum();
    if spent < config.detectors.floor.max(50.0) {
        return Vec::new();
    }
    let strays: Vec<f64> = by_day
        .iter()
        .filter(|(d, day)| **d < today && day.values().sum::<f64>() >= 50.0)
        .map(|(_, day)| stats::jensen_shannon(&vector(day), &usual))
        .collect();
    if strays.len() < 10 {
        return Vec::new();
    }
    let divergence = stats::jensen_shannon(&vector(now), &usual);
    if divergence <= stats::quantile(&strays, 0.99).max(0.1) {
        return Vec::new();
    }
    let top = now
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(m, d)| (m.clone(), *d))
        .unwrap_or_default();
    let total_usual: f64 = usual.iter().sum();
    let usual_share = models
        .iter()
        .position(|m| *m == top.0)
        .map_or(0.0, |i| usual[i] / total_usual.max(1e-9));
    let mut f = finding(
        format!("mix:{today}"),
        "mix",
        Severity::Note,
        format!("today leans on {}", top.0),
        format!(
            "{} of today's spend against {} usual.",
            percent(top.1 / spent),
            percent(usual_share)
        ),
    );
    f.dollars = spent;
    f.surprise = 2.0;
    vec![f]
}

/// Something new spending under the account's name: a tool, model or
/// credential billed in the last day that wasn't in the thirty before.
pub fn novelty(
    picture: &Picture,
    history: &History,
    config: &Config,
) -> Result<Vec<Finding>, String> {
    let now = picture.now;
    let recent_from = crate::time::hour_of(now - DAY);
    let before_from = crate::time::hour_of(now - 31.0 * DAY);
    let mut out = Vec::new();
    for (column, what) in [
        ("tool", "tool"),
        ("model", "model"),
        ("token", "credential"),
    ] {
        let query = format!(
            "SELECT {column}, SUM(dollars) FROM billed
             WHERE hour >= ?1 AND {column} != '' AND {column} != 'all'
             AND {column} NOT IN (SELECT {column} FROM billed WHERE hour >= ?2 AND hour < ?1)
             GROUP BY 1 HAVING SUM(dollars) >= 1"
        );
        let seen_before: bool = history
            .db
            .query_row(
                &format!("SELECT EXISTS (SELECT 1 FROM billed WHERE hour < ?1 AND {column} != '')"),
                [recent_from],
                |row| row.get(0),
            )
            .map_err(fail)?;
        if !seen_before {
            continue;
        }
        let mut statement = history.db.prepare(&query).map_err(fail)?;
        let rows = statement
            .query_map([recent_from, before_from], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(fail)?;
        for row in rows {
            let (name, spent) = row.map_err(fail)?;
            let shown = if what == "credential" {
                format!("{}...", name.chars().take(8).collect::<String>())
            } else {
                name.clone()
            };
            // Something new that's spent little is only noted: a model
            // tried once isn't worth a look in the margin.
            let severity = if spent >= config.detectors.floor {
                Severity::Warning
            } else {
                Severity::Note
            };
            // A credential's key is its digest, never the credential.
            let named = if what == "credential" {
                digest(&name)
            } else {
                name.clone()
            };
            let mut f = finding(
                format!("new:{column}:{named}"),
                "novelty",
                severity,
                format!("a new {what} spending under your name: {shown}"),
                format!(
                    "{} in the last day; not billed in the thirty days before. Expected, or worth \
                     a look?",
                    dollars(spent)
                ),
            );
            f.dollars = spent;
            f.surprise = 2.0;
            out.push(f);
        }
    }
    // Spend billed for sessions no log here knows, past its usual share.
    let share = |pieces: &[crate::series::Piece]| {
        let pi: f64 = pieces
            .iter()
            .filter(|p| p.tool == "pi")
            .map(|p| p.dollars)
            .sum();
        let elsewhere: f64 = pieces
            .iter()
            .filter(|p| p.session == "elsewhere")
            .map(|p| p.dollars)
            .sum();
        (elsewhere, if pi > 0.0 { elsewhere / pi } else { 0.0 })
    };
    let (week, week_share) = share(&picture.pieces_week);
    let (_, month_share) = share(&picture.pieces_month);
    if week >= 50.0 && week_share > (2.0 * month_share).max(0.1) {
        let mut f = finding(
            "new:elsewhere".into(),
            "novelty",
            Severity::Warning,
            "pi spending somewhere this machine doesn't know".into(),
            format!(
                "{} this week from sessions with no log here, {} of pi's spend against {} usual.",
                dollars(week),
                percent(week_share),
                percent(month_share)
            ),
        );
        f.dollars = week;
        f.surprise = 2.0;
        out.push(f);
    }
    Ok(out)
}

/// On course for a day far past any of the last eight weeks'.
pub fn spikes(picture: &Picture) -> Vec<Finding> {
    let Some((_, median, _)) = picture.today_ahead() else {
        return Vec::new();
    };
    let recent: Vec<f64> = picture
        .days
        .dollars
        .iter()
        .rev()
        .skip(1)
        .take(56)
        .copied()
        .collect();
    if recent.len() < 28 {
        return Vec::new();
    }
    let costliest = recent.iter().copied().fold(0.0, f64::max);
    if median <= costliest || median < 300.0 {
        return Vec::new();
    }
    let mut f = finding(
        format!("spike:{}", picture.zone.day(picture.now)),
        "spikes",
        // A long day is a note unless the money is large.
        if median >= 2.0 * costliest {
            Severity::Warning
        } else {
            Severity::Note
        },
        format!("on course for {} today", whole(median)),
        format!(
            "The most of any day in eight weeks was {}.",
            whole(costliest)
        ),
    );
    f.dollars = median - costliest;
    f.surprise = 1.5;
    vec![f]
}

/// A feed's source that can't give its figures. Optional sources aren't
/// findings; signing in follows the sign-in door.
pub fn health(picture: &Picture) -> Vec<Finding> {
    let mut out = Vec::new();
    for source in &picture.sources {
        let (severity, title) = match source.state.as_str() {
            "signed-out" => (
                Severity::Warning,
                format!("{} wants a sign-in", source.name),
            ),
            "error" => (Severity::Warning, format!("{} can't be read", source.name)),
            "stalled" => (Severity::Note, format!("{} has gone quiet", source.name)),
            _ => continue,
        };
        let mut f = finding(
            format!("health:{}:{}", source.feed, source.name),
            "health",
            severity,
            title,
            if source.message.is_empty() {
                "Other sources carry the figures meanwhile.".into()
            } else {
                format!(
                    "{}. Other sources carry the figures meanwhile.",
                    source.message
                )
            },
        );
        f.action = source
            .sign_in
            .as_ref()
            .and_then(|id| {
                picture
                    .actions
                    .iter()
                    .find(|a| &a.id == id && a.feed == source.feed)
            })
            .map(|a| Action {
                name: a.name.clone(),
                run: a.run.clone(),
            });
        f.surprise = 0.5;
        out.push(f);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::{Cost, Event, Kind, Tokens, Turn};

    fn config() -> Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap()
    }

    fn turn(history: &History, session: &str, at: f64, dollars: f64) {
        history
            .add_event(&Event::Turn(Turn {
                session: session.into(),
                at,
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: dollars,
                    ..Cost::default()
                },
                quest: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z, a Wednesday

    // Eight weeks of the same working days, a dollar every ten minutes
    // from nine to five, in a session a day.
    fn ordinary() -> History {
        let history = History::memory();
        let today = (NOW / DAY).floor() * DAY;
        for back in 0..56 {
            let day = today - back as f64 * DAY;
            for minute in (9 * 60..17 * 60).step_by(10) {
                let at = day + minute as f64 * 60.0;
                if at < NOW - 3600.0 {
                    turn(&history, &format!("s{back}"), at, 1.0);
                }
            }
        }
        history
    }

    fn burns_at(history: &History, now: f64) -> Vec<Finding> {
        let config = config();
        let picture = Picture::read(history, &config, now).unwrap();
        burn(&picture, history, &config).unwrap()
    }

    fn burns(history: &History) -> Vec<Finding> {
        burns_at(history, NOW)
    }

    #[test]
    fn an_ordinary_hour_is_no_burn() {
        let history = ordinary();
        for minute in (0..60).step_by(10) {
            turn(&history, "s0", NOW - 3600.0 + minute as f64 * 60.0, 1.0);
        }
        assert!(burns(&history).is_empty());
    }

    // Ten dollars a minute for the last hour is one burn, in the session
    // that spent it.
    #[test]
    fn a_burning_hour_names_its_session() {
        let history = ordinary();
        for minute in 0..60 {
            turn(&history, "hot", NOW - 3600.0 + minute as f64 * 60.0, 10.0);
        }
        let found = burns(&history);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].key, "burn:hot");
        assert_eq!(found[0].session.as_deref(), Some("hot"));
    }

    // A burn from six to seven in the evening, when a usual hour allows a
    // dollar, still scores high at half past eight; but nothing is burning.
    #[test]
    fn a_burn_that_stopped_clears() {
        let history = ordinary();
        let evening = (NOW / DAY).floor() * DAY + 20.5 * 3600.0;
        for minute in 0..60 {
            turn(
                &history,
                "hot",
                evening - 9000.0 + minute as f64 * 60.0,
                10.0,
            );
        }
        assert!(burns_at(&history, evening).is_empty());
    }

    fn billed(history: &History, hour: &str, model: &str, dollars: f64) {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "warehouse", "hour": hour, "tool": "pi",
            "model": model, "vendor": "v", "dollars": dollars,
            "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(history, "proxy", &record, NOW).unwrap();
    }

    // A model new this day is a warning past the floor and a note below it;
    // one billed the month before is nothing new.
    #[test]
    fn something_new_is_weighed_by_what_it_spent() {
        let history = History::memory();
        billed(&history, "2026-10-04T12:00:00Z", "old", 50.0);
        billed(&history, "2026-10-14T12:00:00Z", "old", 50.0);
        billed(&history, "2026-10-14T12:00:00Z", "big", 30.0);
        billed(&history, "2026-10-14T13:00:00Z", "small", 3.0);
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let found = novelty(&picture, &history, &config).unwrap();
        let severity = |key: &str| found.iter().find(|f| f.key == key).map(|f| f.severity);
        assert_eq!(severity("new:model:big"), Some(Severity::Warning));
        assert_eq!(severity("new:model:small"), Some(Severity::Note));
        assert_eq!(severity("new:model:old"), None);
    }
}
