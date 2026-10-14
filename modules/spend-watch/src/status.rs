//! What the shell draws, written to the runtime directory: `status.json`,
//! small, for the overview's row and the margin, with a heartbeat so the
//! shell knows the watch still runs; and `collection.json`, larger, for
//! the launcher, written only when it changes. Every string is ready to
//! draw, each with a quiet form for while the screen is shared, so the
//! shell does no arithmetic and formats no money.

use crate::findings::{Finding, Severity};
use crate::money::{dollars, maybe_about, percent, whole};
use crate::picture::{Figure, Picture};
use crate::report::{self, Cut, Format, Period};
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
fn unpriced(text: &str) -> Option<String> {
    (!text.contains('$')).then(|| text.to_owned())
}

fn action_kind(action: &crate::findings::Action) -> &'static str {
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
    let mut context = Vec::new();
    let mut quiet_context = Vec::new();
    if let Some(top) = top {
        context.push(top.detail.clone());
        quiet_context.extend(unpriced(&top.title));
    }
    if let Some((low, mid, high)) = picture.today_ahead() {
        context.push(format!(
            "today should end near {} ({} to {})",
            whole(mid),
            whole(low),
            whole(high)
        ));
    }
    for view in &picture.caps {
        if let Some(allowance) = view.cap.allowance {
            let share = view.spent / allowance;
            context.push(format!(
                "{} of the {} {}",
                percent(share),
                whole(allowance),
                view.cap.label
            ));
            quiet_context.push(format!("{} of the {}", percent(share), view.cap.label));
        }
    }
    context.push(format!("{} this week", figure_text(&picture.week)));
    match crate::series::weakest(&picture.pieces_today) {
        Some(Source::List) => {
            let line = if picture.fed {
                "at pi's list prices: no bill read lately"
            } else {
                "at pi's list prices, from its logs"
            };
            context.push(line.into());
            quiet_context.push(line.into());
        }
        Some(Source::Estimated) => {
            context.push("estimated from pi's logs where the bill hasn't come".into())
        }
        _ => {}
    }
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
            context: context.join("; "),
        },
        quiet: Said {
            value: quiet_value,
            // A badge that's a percentage or a clock says no amount.
            badge: top
                .map(|finding| finding.badge.clone())
                .filter(|badge| !badge.contains('$'))
                .unwrap_or_default(),
            hint: quiet_hint,
            context: quiet_context.join("; "),
        },
        actions,
    }
}

/// One row of the collection, as the launcher draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub key: String,
    pub name: String,
    pub side: String,
    /// What Return copies.
    pub copy: String,
    /// The theme's role for its glyph: text, warning or alert.
    pub role: String,
    /// What it runs instead of copying, when it has something to do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Vec<String>>,
    /// What marks a finding expected, so it never asks again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    pub id: String,
    pub title: String,
    pub headline: String,
    pub rows: Vec<Line>,
    /// The whole report, for Return to copy.
    pub markdown: String,
    /// What else finds it in the collection: its period's and cut's words,
    /// or what it's about, beside its title.
    pub words: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    pub id: String,
    pub words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub v: u32,
    /// How things stand, then what's been found.
    pub now: Vec<Line>,
    pub findings: Vec<Line>,
    /// What the collection shows while the screen is shared: how today
    /// stands, each cap's share and each finding, with no amount in any.
    pub quiet: Vec<Line>,
    /// Each report by its id, "<period>/<cut>" or a focused one's name.
    pub reports: std::collections::BTreeMap<String, Shown>,
    pub periods: Vec<Words>,
    pub cuts: Vec<Words>,
    pub examples: Vec<String>,
}

fn line(key: &str, name: String, side: String) -> Line {
    Line {
        key: key.into(),
        copy: format!("{name}: {side}"),
        name,
        side,
        role: "text".into(),
        run: None,
        expect: None,
    }
}

fn shown(id: String, words: &str, report: &report::Report) -> Shown {
    let rows = report
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let side = if row.share > 0.0 {
                format!("{} · {}", row.shown, percent(row.share))
            } else {
                row.shown.clone()
            };
            line(&format!("{id}/{index}"), row.name.clone(), side)
        })
        .collect();
    Shown {
        id,
        title: report.title.clone(),
        headline: if report.shown.is_empty() {
            report.notes.first().cloned().unwrap_or_default()
        } else {
            format!("{}, {}", report.shown, report.source)
        },
        rows,
        markdown: report::render(report, Format::Markdown),
        words: words.to_owned(),
    }
}

pub fn collection(
    picture: &Picture,
    findings: &[Finding],
    config: &crate::config::Config,
) -> Collection {
    let mut now = Vec::new();
    let standing = if picture.standing.word.is_empty() {
        String::new()
    } else {
        format!(" · {}", picture.standing.word)
    };
    now.push(line(
        "now:today",
        "Today".into(),
        format!("{}{standing}", figure_text(&picture.today)),
    ));
    if let Some((low, mid, high)) = picture.today_ahead() {
        now.push(line(
            "now:today-ahead",
            "Today should end near".into(),
            format!("{} ({} to {})", whole(mid), whole(low), whole(high)),
        ));
    }
    now.push(line(
        "now:yesterday",
        "Yesterday".into(),
        figure_text(&picture.yesterday),
    ));
    now.push(line(
        "now:week",
        "The last 7 days".into(),
        figure_text(&picture.week),
    ));
    if let Some((low, mid, high)) = picture.week_ahead() {
        now.push(line(
            "now:week-ahead",
            "The next 7 days".into(),
            format!("{} ({} to {})", whole(mid), whole(low), whole(high)),
        ));
    }
    now.push(line(
        "now:month",
        "The last 30 days".into(),
        figure_text(&picture.month),
    ));
    for view in &picture.caps {
        let mut row = line(
            &format!("now:cap:{}", view.cap.name),
            view.cap.label.clone(),
            report::cap_line(picture, view),
        );
        row.name = capitalized(&view.cap.label);
        row.copy = row.side.clone();
        now.push(row);
    }
    for source in &picture.sources {
        if source.state != "fine" && source.state != "optional" {
            let mut row = line(
                &format!("now:source:{}", source.name),
                capitalized(&source.name),
                format!("{}: {}", source.state, source.message),
            );
            row.role = "warning".into();
            now.push(row);
        }
    }
    let mut quiet = vec![line(
        "quiet:today",
        "Today".into(),
        if picture.standing.word.is_empty() {
            "amounts hidden while sharing".into()
        } else {
            picture.standing.word.clone()
        },
    )];
    for view in &picture.caps {
        if let Some(allowance) = view.cap.allowance {
            quiet.push(line(
                &format!("quiet:cap:{}", view.cap.name),
                capitalized(&view.cap.label),
                format!("{} spent", percent(view.spent / allowance)),
            ));
        }
    }
    for finding in findings {
        let name = unpriced(&capitalized(&finding.title))
            .unwrap_or_else(|| "Something wants a look".into());
        quiet.push(Line {
            key: format!("quiet:finding:{}", finding.key),
            copy: name.clone(),
            name,
            side: if finding.marked {
                "expected".into()
            } else {
                finding.severity.name().into()
            },
            role: "text".into(),
            run: finding.action.as_ref().map(|action| action.run.clone()),
            expect: None,
        });
    }
    let findings = findings
        .iter()
        .map(|finding| Line {
            key: format!("finding:{}", finding.key),
            name: capitalized(&finding.title),
            side: if finding.marked {
                "expected".into()
            } else {
                finding.severity.name().into()
            },
            copy: format!("{}. {}", capitalized(&finding.title), finding.detail),
            role: match (finding.marked, finding.severity) {
                (false, Severity::Alert) => "alert".into(),
                (false, Severity::Warning) => "warning".into(),
                _ => "text".into(),
            },
            run: finding.action.as_ref().map(|action| action.run.clone()),
            expect: (!finding.marked).then(|| {
                ["spend-watch", "findings", "expect", &finding.key]
                    .map(String::from)
                    .to_vec()
            }),
        })
        .collect();
    let mut reports = std::collections::BTreeMap::new();
    for period in Period::ALL {
        for cut in Cut::ALL {
            let id = format!("{}/{}", period.id(), cut.id());
            let words = format!("{} by {}", period.words().join(" "), cut.words().join(" "));
            reports.insert(
                id.clone(),
                shown(id, &words, &report::cut(picture, period, cut, 12)),
            );
        }
    }
    let summary = report::summary(picture, &[]);
    reports.insert(
        "summary".into(),
        shown("summary".into(), "today summary", &summary),
    );
    reports.insert(
        "waste".into(),
        shown(
            "waste".into(),
            "waste cache context rebuilt",
            &report::waste(picture),
        ),
    );
    reports.insert(
        "caps".into(),
        shown("caps".into(), "cap caps limit", &report::caps(picture)),
    );
    reports.insert(
        "forecast".into(),
        shown(
            "forecast".into(),
            "forecast ahead",
            &report::forecast(picture),
        ),
    );
    reports.insert(
        "quests".into(),
        shown(
            "quests".into(),
            "quests outcomes pull requests",
            &report::quests(picture, Period::Days30),
        ),
    );
    reports.insert(
        "outliers".into(),
        shown(
            "outliers".into(),
            "outliers costly sessions",
            &report::outliers(
                picture,
                config.detectors.false_discovery,
                config.detectors.floor,
            ),
        ),
    );
    Collection {
        v: VERSION,
        now,
        findings,
        quiet,
        reports,
        periods: Period::ALL
            .iter()
            .map(|period| Words {
                id: period.id().into(),
                words: period.words().iter().map(|w| (*w).to_owned()).collect(),
            })
            .collect(),
        cuts: Cut::ALL
            .iter()
            .map(|cut| Words {
                id: cut.id().into(),
                words: cut.words().iter().map(|w| (*w).to_owned()).collect(),
            })
            .collect(),
        examples: [
            "spend last week by quest",
            "spend today by model",
            "spend this month by agent",
            "spend waste",
            "spend caps",
        ]
        .map(str::to_owned)
        .to_vec(),
    }
}

fn capitalized(text: &str) -> String {
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
    // one, drawn while the screen is shared, says nothing of it.
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
        assert!(status.open.context.contains('$'), "{status:?}");
        let quiet = &status.quiet;
        for said in [&quiet.value, &quiet.badge, &quiet.hint, &quiet.context] {
            assert!(!said.contains('$'), "{said}");
        }
        assert!(!quiet.hint.is_empty());
        // The collection's quiet lines likewise, while its others say it.
        let collected = collection(&picture, &found, &config);
        assert!(
            collected
                .findings
                .iter()
                .any(|line| line.copy.contains('$'))
        );
        assert!(collected.quiet.len() >= 2, "{:?}", collected.quiet);
        for line in &collected.quiet {
            for said in [&line.name, &line.side, &line.copy] {
                assert!(!said.contains('$'), "{said}");
            }
        }
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
