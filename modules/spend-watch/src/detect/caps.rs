//! What the caps find: an allowance over, near or likely reached within a
//! day, an override ending or with no end known, the cap's changes of the
//! last day, and pi's requests refused for being over one.

use super::{changed_note, ending_note, finding, offered, refused_note};
use crate::findings::{Action, Finding, Severity};
use crate::money::{percent, whole};
use crate::picture::{CapView, Picture};
use crate::time::{DAY, HOUR, Zone};

pub fn caps(picture: &Picture) -> Vec<Finding> {
    let mut out = Vec::new();
    for view in &picture.caps {
        out.extend(cap_findings(picture, view));
    }
    out.extend(cap_events(picture));
    out
}

/// A finding's key for one of a cap's states.
fn cap_key(view: &CapView, what: &str) -> String {
    format!("cap:{}:{}:{what}", view.owner, view.cap.name)
}

/// A cap as a sentence names it: its window's name, then the glossary's
/// word for what it is, as in "near the Premium Day allowance".
fn allowance_named(cap: &crate::caps::Cap) -> String {
    format!("the {} allowance", cap.label)
}

/// What one cap with an allowance says now: over it, likely to reach it
/// within a day, or near it, and its override ending.
fn cap_findings(picture: &Picture, view: &CapView) -> Vec<Finding> {
    let cap = &view.cap;
    let now = picture.now;
    let ask = offered(picture, "cap");
    if let Some(used) = view.used {
        return share_finding(picture, view, used, ask)
            .into_iter()
            .collect();
    }
    let Some(allowance) = cap.allowance else {
        return Vec::new();
    };
    let share = view.spent / allowance;
    let over = view.spent >= allowance || cap.state == "refused";
    // Where the spend is counted: a rolling window, or a period that
    // resets whatever is spent.
    let counted = match view.resets {
        Some(at) => format!(", resetting at {}", picture.zone.clock(at, now)),
        None => format!(" in the last {} hours", cap.window_hours),
    };
    // A feed's figure is the provider's own count at its last run, which
    // is what it refuses by, so the time it ran is said with it.
    let counted = if cap.feed.is_empty() {
        counted
    } else {
        format!(
            "{counted}, by the {} count",
            picture.zone.clock(cap.at, now)
        )
    };
    if over {
        return vec![over_finding(picture, view, allowance, &counted, ask)];
    }
    // An end nobody knows needs no forecast to be warned of.
    let unending = unending_finding(picture, view, view.clock.as_ref(), allowance, ask.clone());
    // An end nobody knows is said once, by the unknown-end warning.
    let unwarned = unending.is_none();
    let mut out: Vec<Finding> = unending.into_iter().collect();
    let Some(clock) = &view.clock else {
        return out;
    };
    let chance = clock.cap_within_day;
    if chance >= 0.2 {
        let mut f = finding(
            cap_key(view, "close"),
            "caps",
            if chance >= 0.5 {
                Severity::Alert
            } else {
                Severity::Warning
            },
            format!("may reach {} within a day", allowance_named(cap)),
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
        f.badge = format!("{} chance", percent(chance));
        f.note = format!("{} of {} spent", whole(view.spent), whole(allowance));
        f.dollars = allowance - view.spent;
        f.surprise = 2.0;
        f.cap = true;
        f.action.clone_from(&ask);
        out.push(f);
    } else if share >= cap.warn {
        let mut f = finding(
            cap_key(view, "near"),
            "caps",
            Severity::Warning,
            format!("near {}", allowance_named(cap)),
            format!(
                "{} of {}{counted}.{}",
                whole(view.spent),
                whole(allowance),
                reach(picture, view, clock).map_or_else(String::new, |at| format!(
                    " At {}'s pace it's reached by {}.",
                    paced(picture),
                    dated(&picture.zone, at)
                ))
            ),
        );
        f.badge = percent(share);
        f.note = format!("{} of {}", whole(view.spent), whole(allowance));
        f.dollars = allowance - view.spent;
        f.cap = true;
        f.action.clone_from(&ask);
        out.push(f);
    }
    if unwarned {
        out.extend(ending_finding(picture, view, clock, allowance, ask));
    }
    out
}

/// When the allowance is reached at this pace, unless that's past a known
/// end still ahead, when the allowance will be another figure.
fn reach(picture: &Picture, view: &CapView, clock: &crate::caps::Clock) -> Option<f64> {
    let ends = view
        .ends_at
        .filter(|end| view.cap.ends_known && *end > picture.now);
    clock
        .reaches_at
        .filter(|at| ends.is_none_or(|end| *at <= end))
}

/// An override whose end the feed can't say, warned while the spend is
/// already over what follows it: the cap refuses the moment it ends, and
/// nothing can warn ahead of a day nobody knows. On 19 November the
/// proxy's weekly cap stood at a $40,000 raise, ending who knew when,
/// over a $4,000 fallback, with $33,671 spent; on 31 October and 2
/// November the cap refused 107 requests. A guessed end is no end known,
/// and nor is one already past while the override still stands; but a
/// feed that could know the end and gives none says there is none.
fn unending_finding(
    picture: &Picture,
    view: &CapView,
    clock: Option<&crate::caps::Clock>,
    allowance: f64,
    ask: Option<Action>,
) -> Option<Finding> {
    let fallback = view.cap.fallback?;
    let now = picture.now;
    let ahead = view.ends_at.filter(|end| *end > now);
    if (view.cap.ends_known && (ahead.is_some() || view.cap.ends.is_none()))
        || view.spent < fallback
    {
        return None;
    }
    // Known by now means given and past, which the margin shows first.
    let passed = view.cap.ends_known;
    let reaches = clock
        .and_then(|clock| reach(picture, view, clock))
        .map_or_else(String::new, |at| {
            format!(
                " At {}'s pace it reaches {} by {}.",
                paced(picture),
                whole(allowance),
                dated(&picture.zone, at)
            )
        });
    let mut f = finding(
        cap_key(view, "unending"),
        "caps",
        Severity::Warning,
        format!(
            "{} override's end {}",
            allowance_named(&view.cap),
            if passed { "has passed" } else { "is unknown" }
        ),
        unending_detail(view, ahead, allowance, fallback, &reaches),
    );
    f.badge = if passed { "end passed" } else { "end unknown" }.into();
    f.note = format!("over the {} after it", whole(fallback));
    f.dollars = view.spent - fallback;
    f.surprise = 2.5;
    f.cap = true;
    f.action = ask;
    Some(f)
}

/// What the unknown-end warning says: why the end the feed gives, if any,
/// isn't one to warn ahead of, and what follows, with `reaches` after it.
fn unending_detail(
    view: &CapView,
    ahead: Option<f64>,
    allowance: f64,
    fallback: f64,
    reaches: &str,
) -> String {
    let (spent, after) = (whole(view.spent), whole(fallback));
    // A known end past while the feed still reads the override may be one
    // the feed hasn't run since, so the cap may already refuse.
    if let (Some(day), true) = (view.cap.ends, view.cap.ends_known) {
        return format!(
            "The {} override's end, {}, has passed, but the feed still reads it and may \
             not have run since, and the {spent} spent is already over the {after} that \
             follows it, so the cap may already refuse.{reaches}",
            whole(allowance),
            Zone::day_name(day)
        );
    }
    let given = match (view.cap.ends, ahead) {
        (Some(day), Some(_)) => format!(" (its end, {}, is a guess)", Zone::day_name(day)),
        (Some(day), None) => format!(" (its end, {}, a guess, has passed)", Zone::day_name(day)),
        (None, _) => String::new(),
    };
    format!(
        "Nobody knows when the {} override ends{given}, and the {spent} spent is already \
         over the {after} that follows it, so the cap refuses the moment it ends.{reaches}",
        whole(allowance)
    )
}

/// A moment by its day and time, "Fri 20 Nov 23:00", for a finding told
/// once and read later, when "tomorrow" would have moved.
fn dated(zone: &Zone, at: f64) -> String {
    format!("{} {}", Zone::day_name(zone.day(at)), zone.clock(at, at))
}

/// The hour a pace was judged in, said beside a reach: a notice keeps its
/// first telling while the pace moves on, so it says when it was true.
fn paced(picture: &Picture) -> String {
    dated(
        &picture.zone,
        crate::time::hour_start(crate::time::hour_of(picture.now)),
    )
}

/// A share of a plan's window used up, or past its warning: said
/// in shares, since a share is never turned into dollars, and with when
/// it resets.
fn share_finding(
    picture: &Picture,
    view: &CapView,
    used: f64,
    ask: Option<Action>,
) -> Option<Finding> {
    let cap = &view.cap;
    let over = used >= 1.0 || cap.state == "refused";
    if !over && used < cap.warn {
        return None;
    }
    let resets = view.resets.map_or_else(String::new, |at| {
        format!(", resetting at {}", picture.zone.clock(at, picture.now))
    });
    let (what, severity, title) = if over {
        (
            "over",
            Severity::Alert,
            format!("over {}", allowance_named(cap)),
        )
    } else {
        (
            "near",
            Severity::Warning,
            format!("near {}", allowance_named(cap)),
        )
    };
    let mut f = finding(
        cap_key(view, what),
        "caps",
        severity,
        title,
        format!(
            "{} of {} used{resets}.",
            percent(used),
            allowance_named(cap)
        ),
    );
    f.badge = view.resets.map_or_else(
        || percent(used),
        |at| format!("resets {}", picture.zone.clock(at, picture.now)),
    );
    f.note = format!("{} used", percent(used));
    f.surprise = if over { 3.0 } else { 1.0 };
    f.cap = true;
    f.action = ask;
    Some(f)
}

/// A cap that's over: by how much, and when it's back under.
fn over_finding(
    picture: &Picture,
    view: &CapView,
    allowance: f64,
    counted: &str,
    ask: Option<Action>,
) -> Finding {
    let now = picture.now;
    let frees_at = view.clock.as_ref().and_then(|clock| clock.frees_at);
    let frees = match view.resets {
        Some(_) => String::new(),
        None => frees_at
            .map(|at| {
                format!(
                    " It's back under by {} if nothing more is spent.",
                    picture.zone.clock(at, now)
                )
            })
            .unwrap_or_default(),
    };
    let mut f = finding(
        cap_key(view, "over"),
        "caps",
        Severity::Alert,
        format!("over {}", allowance_named(&view.cap)),
        format!(
            "{} of {}{counted}.{frees}",
            whole(view.spent),
            whole(allowance),
        ),
    );
    f.badge = frees_at.map_or_else(
        || percent(view.spent / allowance),
        |at| format!("back {}", picture.zone.clock(at, now)),
    );
    f.note = format!("{} of {}", whole(view.spent), whole(allowance));
    f.dollars = view.spent - allowance;
    f.surprise = 3.0;
    f.cap = true;
    f.action = ask;
    f
}

/// An override ending, warned ahead when what follows it is likely to be
/// over at once.
fn ending_finding(
    picture: &Picture,
    view: &CapView,
    clock: &crate::caps::Clock,
    allowance: f64,
    ask: Option<Action>,
) -> Option<Finding> {
    let (Some(ends_at), Some(after), Some(fallback)) =
        (view.ends_at, clock.over_after_end, view.cap.fallback)
    else {
        return None;
    };
    let days_left = (ends_at - picture.now) / DAY;
    let lead = picture.lead;
    let due = days_left > 0.0 && days_left <= f64::from(lead.0) && after >= 0.5;
    if !due {
        return None;
    }
    let day = Zone::day_name(picture.zone.day(ends_at - 1.0));
    // A guessed end is still warned as it nears, but said to be a guess.
    let known = view.cap.ends_known;
    let mut f = finding(
        cap_key(view, "ending"),
        "caps",
        if days_left <= f64::from(lead.1) {
            Severity::Alert
        } else {
            Severity::Warning
        },
        format!(
            "{} override {}",
            allowance_named(&view.cap),
            if known { "is ending" } else { "may be ending" }
        ),
        format!(
            "The {} override ends {day}{}. At {}'s pace, a {} chance the spend \
             is over the {} that follows the moment it does.",
            whole(allowance),
            if known { "" } else { ", by the feed's guess" },
            paced(picture),
            percent(after),
            whole(fallback)
        ),
    );
    f.badge = if known {
        format!("ends {day}")
    } else {
        format!("ends {day}, a guess")
    };
    f.note = ending_note(after, fallback);
    f.dollars = view.spent;
    f.surprise = 2.5;
    f.cap = true;
    f.action = ask;
    Some(f)
}

/// The cap changes of the last day worth a look: a fall or a lapse not yet
/// undone, and a rise.
fn cap_events(picture: &Picture) -> Vec<Finding> {
    let mut out = Vec::new();
    let now = picture.now;
    // A fall is past once a later change takes the cap back to where it
    // was, so it stops asking for a look; the return is told instead.
    // No limit is at least any limit.
    let at_least = |one: Option<f64>, other: Option<f64>| match (one, other) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(one), Some(other)) => one >= other,
    };
    let back = |event: &crate::caps::Event| {
        picture.events.iter().any(|later| {
            later.feed == event.feed
                && later.budget == event.budget
                && later.name == event.name
                && later.at > event.at
                && later.kind == "raised"
                && at_least(later.after, event.before)
        })
    };
    for event in picture.events.iter().filter(|event| now - event.at < DAY) {
        if matches!(event.kind.as_str(), "lowered" | "lapsed") && back(event) {
            continue;
        }
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
                if event.budget.is_empty() {
                    event.feed.clone()
                } else {
                    format!("{}/{}", event.feed, event.budget)
                },
                event.name,
                event.kind,
                crate::time::whole_seconds(event.at)
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
        // The title says what it is now; the side, when.
        f.badge = format!("at {}", picture.zone.clock(event.at, now));
        f.note = changed_note(event.before, event.spent);
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
    f.badge = format!("last {}", picture.zone.clock(last.0, picture.now));
    f.note = refused_note(recent.len(), &picture.zone.clock(recent[0].0, picture.now));
    f.surprise = 3.0;
    f.cap = true;
    f.action = offered(picture, "cap");
    vec![f]
}

#[cfg(test)]
#[path = "caps_tests.rs"]
mod tests;
