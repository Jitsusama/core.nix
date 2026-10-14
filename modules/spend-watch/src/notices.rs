//! Notices, through the desktop's notification server, which in this
//! shell is the shell itself, so its quiet while sharing holds them.
//!
//! Only a warning or an alert asks, once per finding, and its notice is
//! withdrawn when the finding no longer holds or is marked expected. A
//! cap's always may; the rest share a budget a week, so the watch can't
//! nag. Once a day, at the first pass after four in the morning that finds
//! the screen unlocked, one quiet line says how yesterday went.
//!
//! A notice's summary never holds an amount, and neither does a critical
//! one's body, since a critical notice shows while the screen is shared;
//! the rest keep their amounts in the body, which sharing holds back.

use crate::findings::{Finding, Severity};
use crate::history::History;
use crate::money::{percent, whole};
use crate::picture::Picture;
use crate::time::{DAY, Zone};
use rusqlite::params;
use std::collections::HashMap;
use std::time::Duration;
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Show {
        key: String,
        summary: String,
        body: String,
        critical: bool,
        cap: bool,
    },
    Withdraw {
        key: String,
        id: u32,
    },
}

/// What the desktop does with a change.
pub trait Desk {
    /// Shows a notice, giving the id the server gave it.
    fn show(&mut self, summary: &str, body: &str, critical: bool) -> Result<u32, String>;
    fn close(&mut self, id: u32) -> Result<(), String>;
    /// Whether the person's screen is unlocked, as far as can be told.
    fn unlocked(&mut self) -> bool;
}

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

/// The summary a finding's notice carries: its title without an amount.
fn summary(finding: &Finding) -> String {
    let title = &finding.title;
    if title.contains('$') {
        match finding.detector.as_str() {
            "caps" => "A cap needs you".into(),
            "burn" => "Spend is burning fast".into(),
            _ => "AI spend needs a look".into(),
        }
    } else {
        let mut chars = title.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(chars).collect()
        })
    }
}

/// What should change, given what holds and what's been noticed.
pub fn plan(
    history: &History,
    findings: &[Finding],
    budget: u32,
    now: f64,
) -> Result<Vec<Change>, String> {
    let shown: Vec<(String, u32)> = {
        let mut statement = history
            .db
            .prepare(
                "SELECT key, id FROM notices WHERE withdrawn IS NULL AND key NOT LIKE 'daily:%'",
            )
            .map_err(fail)?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(fail)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(fail)?
    };
    let spent: u32 = history
        .db
        .query_row(
            "SELECT COUNT(*) FROM notices WHERE cap = 0 AND at >= ?1 AND key NOT LIKE 'daily:%'",
            [now - 7.0 * DAY],
            |row| row.get(0),
        )
        .map_err(fail)?;
    let asking: HashMap<&str, &Finding> = findings
        .iter()
        .filter(|finding| !finding.marked && finding.severity >= Severity::Warning)
        .map(|finding| (finding.key.as_str(), finding))
        .collect();
    let mut changes = Vec::new();
    for (key, id) in &shown {
        if !asking.contains_key(key.as_str()) {
            changes.push(Change::Withdraw {
                key: key.clone(),
                id: *id,
            });
        }
    }
    let mut left = budget.saturating_sub(spent);
    // When each key was last noticed, so a finding that holds on is told
    // of once, and one that cleared and came back, starting anew, again.
    let told: HashMap<String, f64> = {
        let mut statement = history
            .db
            .prepare("SELECT key, MAX(at) FROM notices GROUP BY key")
            .map_err(fail)?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(fail)?
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(fail)?
    };
    for finding in findings {
        let told_now = told
            .get(&finding.key)
            .is_some_and(|&at| at >= finding.started);
        if !asking.contains_key(finding.key.as_str()) || told_now {
            continue;
        }
        if !finding.cap {
            if left == 0 {
                continue;
            }
            left -= 1;
        }
        let critical = finding.cap && finding.severity == Severity::Alert;
        let body = if critical && finding.detail.contains('$') {
            "Super+U shows where it went.".to_owned()
        } else {
            finding.detail.clone()
        };
        changes.push(Change::Show {
            key: finding.key.clone(),
            summary: summary(finding),
            body,
            critical,
            cap: finding.cap,
        });
    }
    Ok(changes)
}

/// The day's line, once a day after four in the morning: yesterday's
/// spend against a usual day, its cache writes, and the week.
pub fn daily(history: &History, picture: &Picture, now: f64) -> Result<Option<Change>, String> {
    let zone = &picture.zone;
    let today = zone.day(now);
    if zone.local_hour(now) < 4 {
        return Ok(None);
    }
    let key = format!("daily:{}", Zone::iso_day(today));
    let sent: bool = history
        .db
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM notices WHERE key = ?1)",
            [&key],
            |row| row.get(0),
        )
        .map_err(fail)?;
    if sent || picture.yesterday.dollars < 1.0 {
        return Ok(None);
    }
    let weekend = |day: i64| Zone::weekday(day) >= 5;
    let yesterday = today - 1;
    let n = picture.days.dollars.len() as i64;
    let at = |day: i64| {
        let index = day - picture.days.first;
        (0..n)
            .contains(&index)
            .then(|| picture.days.dollars[index as usize])
    };
    let comparable: Vec<f64> = (2..=29)
        .map(|back| today - back)
        .filter(|&day| weekend(day) == weekend(yesterday))
        .filter_map(at)
        .collect();
    let spent = picture.yesterday.dollars;
    let word = if comparable.len() >= 5 {
        let below = comparable.iter().filter(|value| **value < spent).count() as f64
            / comparable.len() as f64;
        if below <= 0.1 {
            ", quiet"
        } else if below <= 0.9 {
            ", usual"
        } else {
            ", more than usual"
        }
    } else {
        ""
    };
    let (cache, all): (f64, f64) = history
        .db
        .query_row(
            "SELECT COALESCE(SUM(cost_cache_write), 0), COALESCE(SUM(cost), 0) FROM turns
             WHERE at >= ?1 AND at < ?2",
            [zone.day_start(yesterday), zone.day_start(today)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(fail)?;
    let mut body = format!(
        "Yesterday {}{word}",
        crate::money::maybe_about(whole(spent), picture.yesterday.estimated)
    );
    if all > 0.0 {
        body.push_str(&format!("; cache writes {}", percent(cache / all)));
    }
    body.push_str(&format!(
        "; the last 7 days {}.",
        whole(picture.week.dollars)
    ));
    Ok(Some(Change::Show {
        key,
        summary: "Yesterday's AI spend".into(),
        body,
        critical: false,
        cap: false,
    }))
}

/// Carries changes out and records them.
pub fn apply(
    history: &History,
    desk: &mut dyn Desk,
    changes: Vec<Change>,
    now: f64,
) -> Result<(), String> {
    for change in changes {
        match change {
            Change::Show {
                key,
                summary,
                body,
                critical,
                cap,
            } => {
                let id = desk.show(&summary, &body, critical)?;
                history
                    .db
                    .execute(
                        "INSERT INTO notices VALUES (?1, ?2, ?3, ?4, NULL)",
                        params![key, now, cap, id],
                    )
                    .map_err(fail)?;
            }
            Change::Withdraw { key, id } => {
                // A server that has forgotten the id has nothing to close.
                let _ = desk.close(id);
                history
                    .db
                    .execute(
                        "UPDATE notices SET withdrawn = ?1 WHERE key = ?2 AND withdrawn IS NULL",
                        params![now, key],
                    )
                    .map_err(fail)?;
            }
        }
    }
    Ok(())
}

/// How long any call waits for its answer, so a wedged server can't stop
/// the daemon's loop.
const PATIENCE: Duration = Duration::from_secs(2);

fn patient(builder: Builder<'_>) -> zbus::Result<Connection> {
    builder.method_timeout(PATIENCE).build()
}

const NOTIFICATIONS: &str = "org.freedesktop.Notifications";

/// The desktop over the real buses, connected afresh when a call fails.
#[derive(Default)]
pub struct Buses {
    session: Option<Connection>,
    system: Option<Connection>,
}

impl Buses {
    fn session(&mut self) -> Result<&Connection, String> {
        if self.session.is_none() {
            self.session = Some(
                Builder::session()
                    .and_then(patient)
                    .map_err(|error| error.to_string())?,
            );
        }
        self.session.as_ref().ok_or_else(|| "no session bus".into())
    }
}

impl Desk for Buses {
    fn show(&mut self, summary: &str, body: &str, critical: bool) -> Result<u32, String> {
        let hints = HashMap::from([("urgency", Value::U8(if critical { 2 } else { 1 }))]);
        let reply = self
            .session()?
            .call_method(
                Some(NOTIFICATIONS),
                "/org/freedesktop/Notifications",
                Some(NOTIFICATIONS),
                "Notify",
                &(
                    "AI spend",
                    0u32,
                    "",
                    summary,
                    body,
                    Vec::<&str>::new(),
                    hints,
                    -1i32,
                ),
            )
            .map_err(|error| {
                self.session = None;
                error.to_string()
            })?;
        reply
            .body()
            .deserialize()
            .map_err(|error| error.to_string())
    }

    fn close(&mut self, id: u32) -> Result<(), String> {
        self.session()?
            .call_method(
                Some(NOTIFICATIONS),
                "/org/freedesktop/Notifications",
                Some(NOTIFICATIONS),
                "CloseNotification",
                &id,
            )
            .map(|_| ())
            .map_err(|error| {
                self.session = None;
                error.to_string()
            })
    }

    /// From logind: the user's display session's locked hint. Anything
    /// that can't be read counts as unlocked, so the line still comes.
    fn unlocked(&mut self) -> bool {
        if self.system.is_none() {
            self.system = Builder::system().and_then(patient).ok();
        }
        let Some(system) = &self.system else {
            return true;
        };
        let get = |path: &str, interface: &str, property: &str| -> Option<OwnedValue> {
            let reply = system
                .call_method(
                    Some("org.freedesktop.login1"),
                    path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &(interface, property),
                )
                .ok()?;
            reply.body().deserialize::<OwnedValue>().ok()
        };
        let display = get(
            "/org/freedesktop/login1/user/self",
            "org.freedesktop.login1.User",
            "Display",
        )
        .and_then(|value| <(String, OwnedObjectPath)>::try_from(value).ok());
        let Some((_, path)) = display else {
            return true;
        };
        let locked = get(
            path.as_str(),
            "org.freedesktop.login1.Session",
            "LockedHint",
        )
        .and_then(|value| bool::try_from(value).ok());
        locked != Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(key: &str, severity: Severity, cap: bool) -> Finding {
        Finding {
            key: key.into(),
            detector: if cap { "caps".into() } else { "burn".into() },
            severity,
            title: format!("{key} at $100"),
            detail: "detail".into(),
            dollars: 1.0,
            surprise: 1.0,
            badge: String::new(),
            note: String::new(),
            session: None,
            quest: None,
            action: None,
            cap,
            started: 0.0,
            marked: false,
        }
    }

    struct Fake {
        shown: Vec<(u32, String)>,
    }

    impl Desk for Fake {
        fn show(&mut self, summary: &str, _: &str, _: bool) -> Result<u32, String> {
            self.shown
                .push((self.shown.len() as u32 + 1, summary.into()));
            Ok(self.shown.len() as u32)
        }
        fn close(&mut self, id: u32) -> Result<(), String> {
            self.shown.retain(|(shown, _)| *shown != id);
            Ok(())
        }
        fn unlocked(&mut self) -> bool {
            true
        }
    }

    #[test]
    fn the_budget_holds_but_a_cap_always_asks() {
        let history = History::memory();
        let mut desk = Fake { shown: Vec::new() };
        let findings: Vec<Finding> = (0..5)
            .map(|i| finding(&format!("burn:{i}"), Severity::Alert, false))
            .chain([finding("cap:over", Severity::Alert, true)])
            .collect();
        let changes = plan(&history, &findings, 3, 0.0).unwrap();
        apply(&history, &mut desk, changes, 0.0).unwrap();
        assert_eq!(desk.shown.len(), 4);
        assert!(
            desk.shown.iter().all(|(_, summary)| !summary.contains('$')),
            "no amount in a summary"
        );
        // Told once: the same findings again change nothing.
        assert!(plan(&history, &findings, 3, 60.0).unwrap().is_empty());
    }

    #[test]
    fn a_finding_gone_or_marked_is_withdrawn() {
        let history = History::memory();
        let mut desk = Fake { shown: Vec::new() };
        let mut held = vec![finding("cap:near", Severity::Warning, true)];
        apply(
            &history,
            &mut desk,
            plan(&history, &held, 3, 0.0).unwrap(),
            0.0,
        )
        .unwrap();
        held[0].marked = true;
        let changes = plan(&history, &held, 3, 60.0).unwrap();
        assert!(matches!(changes.as_slice(), [Change::Withdraw { .. }]));
        apply(&history, &mut desk, changes, 60.0).unwrap();
        assert!(desk.shown.is_empty());
    }

    // The same cap over in week one and again in week six is two
    // episodes, each told of.
    #[test]
    fn a_finding_that_comes_back_is_told_again() {
        let history = History::memory();
        let mut desk = Fake { shown: Vec::new() };
        let mut over = finding("cap:over", Severity::Alert, true);
        apply(
            &history,
            &mut desk,
            plan(&history, &[over.clone()], 3, 0.0).unwrap(),
            0.0,
        )
        .unwrap();
        apply(
            &history,
            &mut desk,
            plan(&history, &[], 3, DAY).unwrap(),
            DAY,
        )
        .unwrap();
        assert!(desk.shown.is_empty());
        over.started = 35.0 * DAY;
        let changes = plan(&history, &[over], 3, 35.0 * DAY).unwrap();
        assert!(
            matches!(changes.as_slice(), [Change::Show { .. }]),
            "{changes:?}"
        );
    }

    #[test]
    fn a_critical_body_holds_no_amount() {
        let history = History::memory();
        let mut over = finding("cap:over", Severity::Alert, true);
        over.detail = "$4,100 of $4,000 in the last 168 hours.".into();
        let changes = plan(&history, &[over], 3, 0.0).unwrap();
        let [Change::Show { body, critical, .. }] = changes.as_slice() else {
            panic!("{changes:?}")
        };
        assert!(*critical && !body.contains('$'), "{body}");
    }

    #[test]
    fn a_note_never_asks() {
        let history = History::memory();
        assert!(
            plan(
                &history,
                &[finding("regime", Severity::Note, false)],
                3,
                0.0
            )
            .unwrap()
            .is_empty()
        );
    }
}
