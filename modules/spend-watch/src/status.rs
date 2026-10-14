//! What the shell draws, written to the runtime directory: `status.json`,
//! small, for the overview's row and the margin, with a heartbeat so the
//! shell knows the watch still runs; and `collection.json` (the
//! collection module), larger, for the launcher, written only when it
//! changes. Every string is ready to draw, each with a quiet form for
//! while the screen is shared, so the shell does no arithmetic and formats
//! no money.

use crate::collection::Collection;
use crate::findings::{Finding, Severity};
use crate::money::{dollars, maybe_about, whole};
use crate::picture::{Figure, Picture};
use crate::series::Source;
use serde::Serialize;
use std::path::Path;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Said {
    pub value: String,
    pub badge: String,
    pub hint: String,
    /// The margin's footer: one short phrase, so it never runs out of room.
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellAction {
    /// What kind of thing it does, for its glyph: go, ask, sign-in,
    /// report, mark.
    pub kind: String,
    pub name: String,
    pub run: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub v: u32,
    pub heartbeat: f64,
    pub active: bool,
    pub role: String,
    pub wants: bool,
    pub title: String,
    /// As it's said, and as it's said while the screen is shared.
    pub open: Said,
    pub quiet: Said,
    pub actions: Vec<ShellAction>,
}

fn figure_text(figure: &Figure) -> String {
    maybe_about(dollars(figure.dollars), figure.estimated)
}

/// The findings that ask for the person: unmarked, past a note.
fn asking(findings: &[Finding]) -> Vec<&Finding> {
    findings
        .iter()
        .filter(|finding| !finding.marked && finding.severity > Severity::Note)
        .collect()
}

/// Words fit to show while the screen is shared: none that hold an amount.
pub(crate) fn unpriced(text: &str) -> Option<String> {
    (!text.contains('$')).then(|| text.to_owned())
}

pub(crate) fn action_kind(action: &crate::findings::Action) -> &'static str {
    match action.run.get(1).map(String::as_str) {
        Some("go") => "go",
        _ if action.name.to_lowercase().contains("sign in") => "sign-in",
        _ => "ask",
    }
}

pub fn status(picture: &Picture, findings: &[Finding]) -> Status {
    let asking = asking(findings);
    let top = asking.first().copied();
    let word = &picture.standing.word;
    let today = figure_text(&picture.today);
    let value = if word.is_empty() {
        format!("{today} today")
    } else {
        format!("{today} today · {word}")
    };
    let quiet_value = if word.is_empty() {
        "amounts hidden while sharing".to_owned()
    } else {
        format!("today · {word}")
    };
    // What the margin says under its actions: why the finding holds, else
    // that a feed's bill has stopped coming so the figures are only list
    // prices, else where today is headed. Without a feed, list prices are
    // all there is, so saying so would say nothing. Everything else has
    // its home in the collection.
    let degraded = match crate::series::weakest(&picture.pieces_today) {
        Some(Source::List) if picture.fed => "at list prices: no bill read lately",
        _ => "",
    };
    let (context, quiet_context) = match top {
        Some(finding) => (
            finding.note.clone(),
            unpriced(&finding.note).unwrap_or_default(),
        ),
        None if !degraded.is_empty() => (degraded.to_owned(), degraded.to_owned()),
        None => match picture.today_ahead() {
            Some((low, mid, high)) => (
                format!(
                    "today should end near {} ({} to {})",
                    whole(mid),
                    whole(low),
                    whole(high)
                ),
                degraded.to_owned(),
            ),
            None => (degraded.to_owned(), degraded.to_owned()),
        },
    };
    let hint = top.map_or_else(|| value.clone(), |finding| capitalized(&finding.title));
    let quiet_hint = match top {
        Some(finding) => unpriced(&finding.title)
            .map(|title| capitalized(&title))
            .unwrap_or_else(|| "AI spend wants a look".into()),
        None => quiet_value.clone(),
    };
    let mut actions: Vec<ShellAction> = Vec::new();
    if let Some(action) = top.and_then(|finding| finding.action.as_ref()) {
        actions.push(ShellAction {
            kind: action_kind(action).into(),
            name: action.name.clone(),
            run: action.run.clone(),
        });
    }
    if let Some(top) = top {
        actions.push(ShellAction {
            kind: "mark".into(),
            name: "It's expected".into(),
            run: vec![
                "spend-watch".into(),
                "findings".into(),
                "expect".into(),
                top.key.clone(),
            ],
        });
    }
    for source in &picture.sources {
        if source.state == "signed-out"
            && let Some(id) = &source.sign_in
            && let Some(action) = picture
                .actions
                .iter()
                .find(|a| &a.id == id && a.feed == source.feed)
            && !actions.iter().any(|a| a.run == action.run)
        {
            actions.push(ShellAction {
                kind: "sign-in".into(),
                name: action.name.clone(),
                run: action.run.clone(),
            });
        }
    }
    actions.push(ShellAction {
        kind: "report".into(),
        name: "Read this week's report".into(),
        run: ["kitty", "--hold", "spend-watch", "report"]
            .map(str::to_owned)
            .to_vec(),
    });
    let (role, wants) = match top.map(|finding| finding.severity) {
        Some(Severity::Alert) => ("alert", true),
        Some(Severity::Warning) => ("warning", false),
        _ => ("text", false),
    };
    Status {
        v: VERSION,
        heartbeat: picture.now,
        active: top.is_some(),
        role: role.into(),
        wants,
        title: "AI spend".into(),
        open: Said {
            value,
            badge: top.map_or_else(String::new, |finding| finding.badge.clone()),
            hint,
            context,
        },
        quiet: Said {
            value: quiet_value,
            // A badge that's a percentage or a clock says no amount.
            badge: top
                .map(|finding| finding.badge.clone())
                .filter(|badge| !badge.contains('$'))
                .unwrap_or_default(),
            hint: quiet_hint,
            context: quiet_context,
        },
        actions,
    }
}

/// What the shell shows while a newer spend-watch keeps the history and
/// this one leaves it be: nothing is watched, so it wants a look, but
/// there's nothing to sign in to or mark, only the newer one to run.
pub fn held(now: f64) -> Status {
    let said = Said {
        value: "on hold".into(),
        badge: String::new(),
        hint: "A newer spend-watch keeps the history".into(),
        context: "run the newer spend-watch".into(),
    };
    Status {
        v: VERSION,
        heartbeat: now,
        active: true,
        role: "warning".into(),
        wants: false,
        title: "AI spend".into(),
        open: said.clone(),
        quiet: said,
        actions: Vec::new(),
    }
}

pub(crate) fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Writes a file whole or not at all: to a temporary name beside it,
/// then renamed over it, so the shell never reads half of one.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let dir = path.parent().ok_or("a status file needs a directory")?;
    std::fs::create_dir_all(dir).map_err(|error| format!("making {}: {error}", dir.display()))?;
    let temporary = dir.join(format!(
        ".{}.new",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("status")
    ));
    std::fs::write(&temporary, text)
        .map_err(|error| format!("writing {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("writing {}: {error}", path.display()))
}

/// Writes what changed: the status always (its heartbeat moves), the
/// collection only when it differs from what was last written.
pub struct Writer {
    last_collection: String,
}

impl Writer {
    pub fn new() -> Writer {
        Writer {
            last_collection: String::new(),
        }
    }

    pub fn write(
        &mut self,
        runtime: &Path,
        status: &Status,
        collection: &Collection,
    ) -> Result<(), String> {
        let text = serde_json::to_string(collection).map_err(|error| error.to_string())?;
        if text != self.last_collection {
            write_atomic(&runtime.join("collection.json"), &text)?;
            self.last_collection = text;
        }
        write_atomic(
            &runtime.join("status.json"),
            &serde_json::to_string(status).map_err(|error| error.to_string())?,
        )
    }
}

impl Default for Writer {
    fn default() -> Writer {
        Writer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::History;

    fn config() -> crate::config::Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap()
    }

    // Over a cap and refused: the open status says how much, and the quiet
    // one, drawn while the screen is shared, says nothing of it. The
    // margin's footer is one short phrase either way.
    #[test]
    fn a_shared_screen_shows_no_amount() {
        let history = History::memory();
        let now = crate::time::parse("2026-10-14T12:00:00Z").unwrap();
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "weekly", "label": "weekly cap",
            "allowance": 4000, "windowHours": 168, "spent": 4100, "warn": 0.9,
            "state": "refused", "at": "2026-10-14T11:55:00Z",
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, now).unwrap();
        let config = config();
        let picture = Picture::read(&history, &config, now).unwrap();
        let mut found = crate::detect::all(&picture, &history, &config).unwrap();
        crate::findings::sort(&mut found);
        let status = status(&picture, &found);
        assert!(status.wants, "{status:?}");
        assert_eq!(status.role, "alert");
        assert_eq!(status.open.context, "$4,100 of $4,000");
        let quiet = &status.quiet;
        for said in [&quiet.value, &quiet.badge, &quiet.hint, &quiet.context] {
            assert!(!said.contains('$'), "{said}");
        }
        assert!(!quiet.hint.is_empty());
    }

    // Held, nothing is watched: the margin shows it as it shows a cap
    // near, and its footer says what mends it.
    #[test]
    fn a_held_watch_wants_a_look_and_beats() {
        let held = held(1_791_988_200.0);
        assert!(held.active && !held.wants, "{held:?}");
        assert_eq!(
            (held.role.as_str(), held.heartbeat),
            ("warning", 1_791_988_200.0)
        );
        assert_eq!(held.open, held.quiet);
        // Beside the margin's one hint, its footer holds about 28.
        assert!(held.open.context.len() <= 28, "{}", held.open.context);
        assert!(held.actions.is_empty());
    }

    // A cap a feed read this very hour, as the meter's premium day is on
    // every reading, is drawn with nothing billed after it.
    #[test]
    fn a_cap_read_this_hour_is_drawn() {
        let history = History::memory();
        let now = crate::time::parse("2026-10-14T12:30:00Z").unwrap();
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "premium", "label": "premium day",
            "allowance": 1000, "windowHours": 24, "spent": 600, "warn": 0.5,
            "state": "notified", "at": "2026-10-14T12:29:00Z",
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, now).unwrap();
        let picture = Picture::read(&history, &config(), now).unwrap();
        assert_eq!(picture.caps.len(), 1);
        assert_eq!(picture.caps[0].spent, 600.0);
    }

    fn premium_day(spent: f64, at: &str, resets: &str) -> History {
        let history = History::memory();
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "premium", "label": "premium day",
            "allowance": 1000, "windowHours": 24, "spent": spent, "warn": 0.5,
            "state": "refused", "resets": resets, "at": at,
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(&history, "proxy", &record, 0.0).unwrap();
        history
    }

    // Over a day's cap, it's back when the day resets, not when spend
    // rolls out of the last 24 hours.
    #[test]
    fn a_day_s_cap_is_back_when_the_day_resets() {
        let now = crate::time::parse("2026-10-14T12:30:00Z").unwrap();
        let history = premium_day(1100.0, "2026-10-14T12:29:00Z", "2026-10-15T04:00:00Z");
        let config = config();
        let picture = Picture::read(&history, &config, now).unwrap();
        let found = crate::detect::caps(&picture);
        let over = found.iter().find(|f| f.key.ends_with(":over")).unwrap();
        assert_eq!(
            over.detail,
            "$1,100 of $1,000, resetting at 04:00 tomorrow."
        );
        assert_eq!(over.badge, "back 04:00 tomorrow");
    }

    // Read in a day that's since reset: none of it counts, and the next
    // reset is the one after.
    #[test]
    fn a_day_s_cap_read_yesterday_has_reset() {
        let now = crate::time::parse("2026-10-14T12:30:00Z").unwrap();
        let history = premium_day(1100.0, "2026-10-13T12:00:00Z", "2026-10-14T04:00:00Z");
        let picture = Picture::read(&history, &config(), now).unwrap();
        assert_eq!(picture.caps[0].spent, 0.0);
        assert_eq!(
            picture.caps[0].resets,
            crate::time::parse("2026-10-15T04:00:00Z")
        );
    }
}
