//! `cadence axes`: the nine axes Joel holds the machine to (resources,
//! time to done, quality, collaboration, security, organization, scale,
//! polish and his stress), each read now against its bar, so he sees in
//! one page which are out of spec and who has each. `cadence axes <axis>`
//! steps into one and says where each reading comes from.
//!
//! Every reading is a script's, from a record the machine already keeps:
//! cadence's card and day, the daily measure, the journal, the mesh, the
//! notices and niri. No model runs, and the page draws in about a second.
//! A reading nothing measures yet says so and names its owner, rather than
//! leave the axis looking fine. The bars are first guesses from
//! docs/delivery.md and the research behind this view
//! (RSCH-20261111-I1QC91); they only ever tighten.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::Value;

use crate::day::Day;
use crate::mesh::{self, Mesh};
use crate::{DAY, command, folders, now_ms};

/// `cadence axes [AXIS]`: prints the nine axes, or the one named.
pub fn run(mut args: impl Iterator<Item = String>) -> ExitCode {
    let asked = args.next();
    if args.next().is_some() {
        eprintln!("usage: cadence axes [AXIS]");
        return ExitCode::from(2);
    }
    let (home, state_home, folder) = match folders() {
        Ok(folders) => folders,
        Err(err) => {
            eprintln!("cadence: {err}");
            return ExitCode::FAILURE;
        }
    };
    let now = now_ms() / 1000;
    let axes = read(&gather(&folder, &state_home, now));
    let ink = ink(&home);
    let zoned = Timestamp::from_second(now).map(|t| t.to_zoned(TimeZone::system()));
    let read_at = zoned
        .as_ref()
        .map(|z| format!("read {}", z.strftime("%H:%M")))
        .unwrap_or_default();
    match asked {
        None => {
            let day = zoned
                .map(|z| z.strftime("%A %-d %B").to_string())
                .unwrap_or_default();
            let heading = format!("{day} {} {read_at}", ink.separator);
            print!("{}", page(&axes, &heading, &ink));
        }
        Some(asked) => {
            let Some(one) = find(&axes, &asked) else {
                let names: Vec<&str> = axes.iter().map(|a| a.name).collect();
                eprintln!(
                    "cadence: no axis is {asked}; the axes are {}",
                    names.join(", ")
                );
                return ExitCode::from(2);
            };
            print!("{}", axis_page(one, &read_at, &ink));
        }
    }
    ExitCode::SUCCESS
}

/// Where a reading's bar is: a figure past it is out of spec.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bar {
    AtMost(f64),
    AtLeast(f64),
}

/// One figure an axis is judged by.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// What it is, with `{}` where its figure goes: "{} pi windows".
    pub says: &'static str,
    /// The figure, or none when nothing measures it yet.
    pub now: Option<f64>,
    /// What it's counted in: "%", " min", "", or "$", which goes before.
    pub unit: &'static str,
    pub bar: Option<Bar>,
    /// The mesh address that owns its bar and its fix.
    pub owner: &'static str,
    /// What more there is to say of it, on the axis's own page.
    pub detail: Option<String>,
    /// Where it's read from, so it can be read again by hand.
    pub from: &'static str,
}

impl Reading {
    fn new(says: &'static str, now: Option<f64>, unit: &'static str, owner: &'static str) -> Self {
        Self {
            says,
            now,
            unit,
            bar: None,
            owner,
            detail: None,
            from: "",
        }
    }

    fn bar(mut self, bar: Bar) -> Self {
        self.bar = Some(bar);
        self
    }

    fn from(mut self, from: &'static str) -> Self {
        self.from = from;
        self
    }

    fn detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }

    /// Whether its figure is past its bar; a reading with no figure or no
    /// bar is never out.
    #[must_use]
    pub fn out(&self) -> bool {
        match (self.now, self.bar) {
            (Some(now), Some(Bar::AtMost(bar))) => now > bar,
            (Some(now), Some(Bar::AtLeast(bar))) => now < bar,
            _ => false,
        }
    }

    /// The reading as a line says it: "85 pi windows, at most 12".
    #[must_use]
    pub fn words(&self) -> String {
        let Some(now) = self.now else {
            let named = self.says.replace(" {}", "").replace("{} ", "");
            return format!("{named}: no reading yet");
        };
        let said = self.says.replacen("{}", &shown(now, self.unit), 1);
        match self.bar {
            Some(Bar::AtMost(bar)) => format!("{said}, at most {}", shown(bar, self.unit)),
            Some(Bar::AtLeast(bar)) => format!("{said}, at least {}", shown(bar, self.unit)),
            None => said,
        }
    }
}

/// One of the nine axes and what it's read by.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub name: &'static str,
    pub readings: Vec<Reading>,
}

impl Axis {
    fn out(&self) -> Vec<&Reading> {
        self.readings.iter().filter(|r| r.out()).collect()
    }

    fn unread(&self) -> bool {
        self.readings.iter().all(|r| r.now.is_none())
    }
}

/// A figure as a line says it: whole from ten, else to one place, with
/// thousands set apart.
#[must_use]
pub fn shown(x: f64, unit: &str) -> String {
    let number = if x.abs() >= 10.0 || x.fract() == 0.0 {
        let whole = format!("{:.0}", x.abs());
        let mut grouped = String::new();
        for (at, digit) in whole.chars().enumerate() {
            if at > 0 && (whole.len() - at) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        format!("{}{grouped}", if x < 0.0 { "-" } else { "" })
    } else {
        format!("{x:.1}")
    };
    if unit == "$" {
        format!("${number}")
    } else {
        format!("{number}{unit}")
    }
}

/// What the page is drawn with: the theme's marks, and the SGR colour the
/// failed mark takes, when it's drawn on a terminal.
#[derive(Debug, Clone, PartialEq)]
pub struct Ink {
    pub failed: String,
    pub separator: String,
    pub alert: Option<String>,
}

impl Default for Ink {
    /// The theme module's own marks (modules/theme/glyphs.nix), uncoloured.
    fn default() -> Self {
        Self {
            failed: "✕".into(),
            separator: "·".into(),
            alert: None,
        }
    }
}

impl Ink {
    /// The marks and alert colour of a theme.json, for a terminal when
    /// `colour` is set.
    #[must_use]
    pub fn of(theme: &Value, colour: bool) -> Self {
        let defaults = Self::default();
        let mark = |name: &str, fallback: String| {
            theme["marks"][name]["glyph"]
                .as_str()
                .map_or(fallback, str::to_owned)
        };
        Self {
            failed: mark("failed", defaults.failed),
            separator: mark("separator", defaults.separator),
            alert: colour.then(|| alert(theme)).flatten(),
        }
    }

    fn failed(&self) -> String {
        match &self.alert {
            Some(sgr) => format!("\x1b[{sgr}m{}\x1b[0m", self.failed),
            None => self.failed.clone(),
        }
    }
}

/// The terminal's sixteen colour names, in their SGR order.
const SIXTEEN: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright_black",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "bright_white",
];

/// The SGR colour of the terminal slot holding the theme's alert pigment
/// or nearest it, as eero's views find a role (modules/eero/src/views/
/// theme.rs): a theme may put any pigment in any slot, so the slot named
/// red is never assumed to be it.
fn alert(theme: &Value) -> Option<String> {
    let channels = |hex: &str| -> Option<[i64; 3]> {
        let hex = hex.strip_prefix('#')?;
        let at = |i: usize| i64::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        Some([at(0)?, at(2)?, at(4)?])
    };
    let want = channels(theme["colours"]["alert"].as_str()?)?;
    let (at, apart) = SIXTEEN
        .iter()
        .enumerate()
        .filter_map(|(at, name)| {
            let have = channels(theme["terminal"][*name].as_str()?)?;
            Some((at, (0..3).map(|i| (want[i] - have[i]).pow(2)).sum::<i64>()))
        })
        .min_by_key(|(_, apart)| *apart)?;
    // Past this it's no longer the alert's hue, and the mark goes plain.
    (apart <= 48 * 48).then(|| (if at < 8 { 30 + at } else { 82 + at }).to_string())
}

/// The axis `asked` names, by the start of its name: `time` is time to
/// done.
#[must_use]
pub fn find<'a>(axes: &'a [Axis], asked: &str) -> Option<&'a Axis> {
    let asked = asked.to_lowercase();
    axes.iter()
        .find(|a| !asked.is_empty() && a.name.starts_with(&asked))
}

/// The page of all nine: each axis's one line, marked when out of spec,
/// saying its first reading out with its owner, else its first reading.
#[must_use]
pub fn page(axes: &[Axis], heading: &str, ink: &Ink) -> String {
    let sep = format!(" {} ", ink.separator);
    let width = axes.iter().map(|a| a.name.len()).max().unwrap_or(0);
    let mut out = format!("axes{sep}{heading}\n\n");
    for axis in axes {
        let off = axis.out();
        let (mark, said) = if let Some(first) = off.first() {
            let more = match off.len() {
                1 => String::new(),
                n => format!("{sep}{} more out", n - 1),
            };
            (
                ink.failed(),
                format!("{}{sep}{}{more}", first.words(), first.owner),
            )
        } else {
            let said = axis.readings.iter().find(|r| r.now.is_some()).map_or_else(
                || {
                    let owner = axis.readings.first().map_or("", |r| r.owner);
                    format!("no reading yet{sep}{owner}")
                },
                Reading::words,
            );
            (" ".to_owned(), said)
        };
        let _ = writeln!(out, "{mark} {:width$}  {said}", axis.name);
    }
    let off = axes.iter().filter(|a| !a.out().is_empty()).count();
    let unread = axes.iter().filter(|a| a.unread()).count();
    let _ = write!(out, "\n{off} of {} out of spec", axes.len());
    if unread > 0 {
        let _ = write!(out, "{sep}{unread} not read yet");
    }
    let _ = writeln!(out, "{sep}cadence axes <axis> steps into one");
    out
}

/// One axis's page: every reading, marked when out of spec, with its
/// owner, what more there is to say and where it's read from.
#[must_use]
pub fn axis_page(axis: &Axis, heading: &str, ink: &Ink) -> String {
    let sep = format!(" {} ", ink.separator);
    let mut out = format!("{}{sep}{heading}\n\n", axis.name);
    for reading in &axis.readings {
        let mark = if reading.out() {
            ink.failed()
        } else {
            " ".to_owned()
        };
        let _ = writeln!(out, "{mark} {}{sep}{}", reading.words(), reading.owner);
        let more: Vec<String> = reading
            .detail
            .iter()
            .cloned()
            .chain((!reading.from.is_empty()).then(|| format!("from {}", reading.from)))
            .collect();
        if !more.is_empty() {
            let _ = writeln!(out, "    {}", more.join(&sep));
        }
    }
    out
}

/// Everything the axes are read from, as text, so reading them is tested
/// without the machine.
#[derive(Debug, Default)]
pub struct Sources {
    /// The daily measures, oldest first.
    pub measures: Vec<String>,
    pub card: String,
    /// cadence's newest day, when one has been kept.
    pub day: Option<Day>,
    /// The systemd units failed now, one a line.
    pub failed: String,
    /// The mesh's messages of the last day, and its presences.
    pub messages: Vec<Note>,
    pub ghosts: Option<usize>,
    /// `$XDG_RUNTIME_DIR/jobs/quests.json`.
    pub quests: String,
    /// Each file under `mission/waiting/`.
    pub waiting: Vec<String>,
    /// `niri msg -j windows`, when niri answered.
    pub windows: Option<String>,
    /// The shell's launcher lines since midnight.
    pub launcher: String,
    /// `notices/notices.json`.
    pub notices: String,
    /// `/proc/pressure/cpu` and `/proc/meminfo`.
    pub pressure: String,
    pub meminfo: String,
    /// Now, in seconds, and the moment the reading's day began.
    pub now: i64,
    pub midnight: i64,
}

/// A mesh message, as far as the axes read one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub intent: String,
    #[serde(default)]
    pub re: Option<String>,
}

/// The nine axes, read from `s`.
#[must_use]
pub fn read(s: &Sources) -> Vec<Axis> {
    vec![
        resources(s),
        time(s),
        quality(s),
        collaboration(s),
        Axis {
            name: "security",
            readings: vec![
                Reading::new("ISXEG8's musts past their date {}", None, "", "ISXEG8")
                    .bar(Bar::AtMost(0.0)),
            ],
        },
        organization(s),
        scale(s),
        Axis {
            name: "polish",
            readings: vec![{
                let (over, keys) = launcher(&s.launcher);
                Reading::new(
                    "{} launcher keys over 50 ms today",
                    Some(count(over)),
                    "",
                    "Y2YBTS",
                )
                .bar(Bar::AtMost(0.0))
                .detail(Some(format!("of {keys} keys")))
                .from("the shell's journal")
            }],
        },
        stress(s),
    ]
}

fn resources(s: &Sources) -> Axis {
    let spends: Vec<f64> = s.measures.iter().filter_map(|m| spend(m)).collect();
    let week = |skip: usize| -> Option<f64> {
        let days: Vec<f64> = spends.iter().rev().skip(skip).take(7).copied().collect();
        (days.len() == 7).then(|| days.iter().sum::<f64>() / 7.0)
    };
    let mut spend = Reading::new("{} a day this week", week(0), "$", "80ECO2")
        .from("the daily measure's Spend lines")
        .detail(
            spends
                .last()
                .map(|d| format!("yesterday {}, its bar the week before", shown(*d, "$"))),
        );
    // Spend is in spec while it falls week on week.
    if let Some(before) = week(7) {
        spend = spend.bar(Bar::AtMost(before));
    }
    let jobs = s
        .day
        .as_ref()
        .and_then(|d| d.figures.get("pressure.cpu.jobs").copied());
    let (_, swap) = crate::records::memory(&s.meminfo);
    let now = crate::records::avg300(&s.pressure);
    vec_axis(
        "resources",
        vec![
            spend,
            Reading::new(
                "{} on cache rewrites after compaction",
                s.measures.last().and_then(|m| after_compaction(m)),
                "$",
                "QCXZEB",
            )
            .bar(Bar::AtMost(100.0))
            .from("the daily measure's cache rewrites"),
            Reading::new("agents' CPU pressure {} over the day", jobs, "%", "5FWIUS")
                .bar(Bar::AtMost(15.0))
                .detail(now.map(|n| {
                    format!(
                        "the machine's {} now, {} GiB of swap",
                        shown(n, "%"),
                        shown(swap, "")
                    )
                }))
                .from("cadence's day, lag-watch's figures"),
            Reading::new(
                "{} machine-wide out-of-memory kills over the day",
                s.day
                    .as_ref()
                    .and_then(|d| d.figures.get("oom.machine").copied()),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(0.0))
            .from("cadence's day, the kernel's journal"),
            Reading::new("turns making one tool call {}", None, "%", "QCXZEB")
                .bar(Bar::AtMost(60.0)),
        ],
    )
}

fn time(s: &Sources) -> Axis {
    let waiting = card_waiting(&s.card);
    let on = card_median(&s.card, "Merge to on").or_else(|| {
        // None landed in the hour: the oldest still waiting is how long
        // merge to on is running.
        waiting.iter().copied().reduce(f64::max)
    });
    vec_axis(
        "time to done",
        vec![
            Reading::new("merge to on {}", on, " min", "80ECO2")
                .bar(Bar::AtMost(18.0))
                .detail(
                    (!waiting.is_empty()).then(|| format!("{} merges not on yet", waiting.len())),
                )
                .from("cadence's card"),
            Reading::new(
                "last commit to merge {}",
                card_median(&s.card, "Last commit to merge"),
                " min",
                "80ECO2",
            )
            .bar(Bar::AtMost(20.0))
            .from("cadence's card"),
            Reading::new(
                "VM wait {}",
                card_median(&s.card, "VM wait"),
                " min",
                "5FWIUS",
            )
            .bar(Bar::AtMost(2.0))
            .from("cadence's card"),
        ],
    )
}

fn quality(s: &Sources) -> Axis {
    let crashes = s.day.as_ref().and_then(|d| d.named.get("crashes"));
    let most = crashes.map(|c| {
        let mut named: Vec<(&String, &u32)> = c.iter().collect();
        named.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        named
            .iter()
            .take(3)
            .map(|(name, n)| format!("{name} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let failed: Vec<&str> = s.failed.lines().filter(|l| !l.trim().is_empty()).collect();
    vec_axis(
        "quality",
        vec![
            Reading::new(
                "{} crashes over the day",
                crashes.map(|c| f64::from(c.values().sum::<u32>())),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(5.0))
            .detail(most.filter(|m| !m.is_empty()).map(|m| format!("most {m}")))
            .from("cadence's day, coredumpctl"),
            Reading::new(
                "{} units failed now",
                Some(count(failed.len())),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(0.0))
            .detail((!failed.is_empty()).then(|| {
                failed
                    .iter()
                    .filter_map(|l| l.split_whitespace().next())
                    .collect::<Vec<_>>()
                    .join(", ")
            }))
            .from("systemctl --failed, the system's and the account's"),
            Reading::new(
                "the net {} behind main",
                card_net(&s.card),
                " min",
                "D5QEKR",
            )
            .bar(Bar::AtMost(60.0))
            .from("cadence's card"),
            // Y9XISP's daily code-health read lands by Thu 12 18:00.
            Reading::new("{} code-health regressions", None, "", "Y9XISP").bar(Bar::AtMost(0.0)),
        ],
    )
}

fn collaboration(s: &Sources) -> Axis {
    let all = s.messages.len();
    let fyi = s.messages.iter().filter(|m| m.intent == "fyi").count();
    let answered: BTreeSet<&str> = s.messages.iter().filter_map(|m| m.re.as_deref()).collect();
    let asked_by = (s.now - 30 * 60) * 1000;
    let unanswered = s
        .messages
        .iter()
        .filter(|m| m.intent == "question" && !answered.contains(m.id.as_str()))
        .filter(|m| mesh::stamped_at(&m.id).is_some_and(|at| at < asked_by))
        .count();
    let read = all > 0;
    vec_axis(
        "collaboration",
        vec![
            Reading::new(
                "{} of the mesh's messages are fyi",
                read.then(|| percent(fyi, all)),
                "%",
                "85PKB2",
            )
            .bar(Bar::AtMost(40.0))
            .detail(read.then(|| {
                format!(
                    "{} of {} in the last day",
                    shown(count(fyi), ""),
                    shown(count(all), "")
                )
            }))
            .from("the mesh's messages"),
            Reading::new(
                "{} questions with no answer after 30 min",
                read.then_some(count(unanswered)),
                "",
                "85PKB2",
            )
            .bar(Bar::AtMost(0.0))
            .detail(Some(
                "over the last day, an answer from any peer counting".into(),
            ))
            .from("the mesh's messages"),
        ],
    )
}

fn organization(s: &Sources) -> Axis {
    let day_ago = (s.now - DAY) * 1000;
    let mut old = 0;
    let mut minutes = 0.0;
    for file in &s.waiting {
        let Ok(waiting) = serde_json::from_str::<Value>(file) else {
            continue;
        };
        for item in waiting["items"].as_array().into_iter().flatten() {
            minutes += item["minutes"].as_f64().unwrap_or(0.0);
            let raised = item["raisedAt"]
                .as_str()
                .and_then(|at| at.parse::<Timestamp>().ok());
            if raised.is_some_and(|at| at.as_millisecond() < day_ago) {
                old += 1;
            }
        }
    }
    vec_axis(
        "organization",
        vec![
            Reading::new("{} quests at driving", driving(&s.quests), "", "WCJQSG")
                .bar(Bar::AtMost(5.0))
                .from("the jobs sorter's quests.json"),
            Reading::new(
                "{} asks waiting on Joel over a day",
                Some(f64::from(old)),
                "",
                "WCJQSG",
            )
            .bar(Bar::AtMost(0.0))
            .detail(Some(format!(
                "{} min of his time asked in all",
                shown(minutes, "")
            )))
            .from("mission's waiting files"),
            Reading::new("{} active quests with no live plan", None, "", "WCJQSG")
                .bar(Bar::AtMost(0.0)),
        ],
    )
}

fn scale(s: &Sources) -> Axis {
    vec_axis(
        "scale",
        vec![
            Reading::new(
                "{} pi windows",
                s.windows.as_deref().and_then(pi_windows),
                "",
                "OSPHT4",
            )
            .bar(Bar::AtMost(12.0))
            .from("niri's windows"),
            Reading::new(
                "{} sessions listed live whose pi is gone",
                s.ghosts.map(count),
                "",
                "85PKB2",
            )
            .bar(Bar::AtMost(0.0))
            .from("the mesh's presences"),
        ],
    )
}

fn stress(s: &Sources) -> Axis {
    let (cleared, by_hand) = notices(&s.notices, s.midnight * 1000);
    let left = cleared + by_hand;
    vec_axis(
        "stress",
        vec![
            Reading::new(
                "{} of notices cleared themselves today",
                (left > 0).then(|| percent(cleared, left)),
                "%",
                "2RCBOJ",
            )
            .bar(Bar::AtLeast(70.0))
            .detail(Some(format!("{by_hand} he cleared by hand")))
            .from("the notices' record"),
            Reading::new("his typed lines that complain {}", None, "%", "1JQPPC")
                .bar(Bar::AtMost(5.0)),
        ],
    )
}

fn vec_axis(name: &'static str, readings: Vec<Reading>) -> Axis {
    Axis { name, readings }
}

fn percent(part: usize, all: usize) -> f64 {
    100.0 * count(part) / count(all)
}

/// A count as a figure. No count here comes near `u32`'s top, which
/// `f64` holds exactly.
fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn money(text: &str) -> Option<f64> {
    let digits: String = text
        .trim_start_matches('$')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .filter(|c| *c != ',')
        .collect();
    digits.parse().ok()
}

/// The day's spend, from the measure's "Spend: $7,293 over ..." line.
fn spend(measure: &str) -> Option<f64> {
    money(measure.lines().find_map(|l| l.strip_prefix("Spend: "))?)
}

/// What cache rewrites after a compaction cost, from the measure's
/// "- after compaction: 1183 times, $474" line.
fn after_compaction(measure: &str) -> Option<f64> {
    let line = measure
        .lines()
        .find_map(|l| l.strip_prefix("- after compaction: "))?;
    money(line.split(", ").nth(1)?)
}

/// The median the card's line starting `what` says, none when it says
/// none.
fn card_median(card: &str, what: &str) -> Option<f64> {
    let line = card.lines().find(|l| l.starts_with(what))?;
    let rest = line.split("): ").nth(1)?.strip_prefix("median ")?;
    rest.split([',', ' ']).next()?.parse().ok()
}

/// How long each merge not on yet has waited, in minutes, from the card.
fn card_waiting(card: &str) -> Vec<f64> {
    card.lines()
        .find_map(|l| l.trim_start().strip_prefix("not on yet: "))
        .map(|list| {
            list.split(", ")
                .filter_map(|m| m.split_whitespace().nth(1)?.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// How long ago the merge the net tests was made, in minutes: zero when
/// it's caught up with main.
fn card_net(card: &str) -> Option<f64> {
    let line = card.lines().find(|l| l.starts_with("The net "))?;
    if line.starts_with("The net is caught up") {
        return Some(0.0);
    }
    let rest = line.rsplit("merged ").next()?;
    rest.split_whitespace().next()?.parse().ok()
}

/// How many quests are at driving, which the jobs sorter weighs at 400.
fn driving(quests: &str) -> Option<f64> {
    let weights: BTreeMap<String, f64> = serde_json::from_str(quests).ok()?;
    Some(count(weights.values().filter(|w| **w >= 400.0).count()))
}

/// How many of niri's windows are pi's: a session's, a spawned one's or
/// a host's.
fn pi_windows(windows: &str) -> Option<f64> {
    let windows: Vec<Value> = serde_json::from_str(windows).ok()?;
    Some(count(
        windows
            .iter()
            .filter(|w| w["app_id"].as_str().is_some_and(|a| a.starts_with("pi-")))
            .count(),
    ))
}

/// Of the launcher's keys, how many took past 50 ms to their frame, and
/// how many there were, from lines like `launcher: "fi" took 31 ms to its
/// frame`.
fn launcher(journal: &str) -> (usize, usize) {
    let took: Vec<u32> = journal
        .lines()
        .filter_map(|line| {
            let after = line.split("launcher: \"").nth(1)?;
            let ms = after.rsplit_once("\" took ")?.1;
            ms.split_whitespace().next()?.parse().ok()
        })
        .collect();
    (took.iter().filter(|ms| **ms > 50).count(), took.len())
}

/// Of the notices that left since `since` (in milliseconds), how many
/// cleared themselves and how many were dismissed by hand.
fn notices(record: &str, since: i64) -> (usize, usize) {
    let record: Value = serde_json::from_str(record).unwrap_or_default();
    let left = |how: &str| {
        record["notices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| n["at"].as_i64().is_some_and(|at| at >= since))
            .filter(|n| n["left"].as_str() == Some(how))
            .count()
    };
    (left("expired"), left("dismissed"))
}

/// The mesh's messages stamped since `since`, in seconds, from the day
/// folders the harness keeps them in.
fn messages(root: &Path, since: i64) -> Vec<Note> {
    let folder = root.join("mesh-workflow/messages");
    [since, since + DAY]
        .iter()
        .filter_map(|at| Timestamp::from_second(*at).ok())
        .map(|at| folder.join(at.strftime("%Y-%m-%d").to_string()))
        .collect::<BTreeSet<PathBuf>>()
        .iter()
        .flat_map(|day| fs::read_dir(day).into_iter().flatten().flatten())
        .filter_map(|entry| serde_json::from_slice::<Note>(&fs::read(entry.path()).ok()?).ok())
        .filter(|m| mesh::stamped_at(&m.id).is_some_and(|ms| ms >= since * 1000))
        .collect()
}

/// How many presences say busy or listening for a pi that isn't running
/// this boot.
fn ghosts(mesh: &Mesh) -> usize {
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|b| b.trim().to_owned())
        .unwrap_or_default();
    mesh.all()
        .iter()
        .filter(|p| p.state == "busy" || p.state == "listening")
        .filter(|p| {
            let this_boot = p
                .owner
                .as_ref()
                .and_then(|o| o.boot_id.as_deref())
                .is_some_and(|b| b == boot);
            !(this_boot && mesh::alive(Some(p), |pid| Path::new(&format!("/proc/{pid}")).exists()))
        })
        .count()
}

/// The newest of cadence's kept days.
fn newest_day(folder: &Path) -> Option<Day> {
    fs::read_dir(folder.join("day"))
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice::<Day>(&fs::read(e.path()).ok()?).ok())
        .max_by_key(|d| d.to)
}

/// Reads every source from the machine, each cheaply: files, and four
/// commands that answer at once.
#[must_use]
pub fn gather(folder: &Path, state_home: &Path, now: i64) -> Sources {
    let text = |path: PathBuf| fs::read_to_string(path).unwrap_or_default();
    let mut measures: Vec<PathBuf> = fs::read_dir(state_home.join("pi/agentic-harness.pi/measure"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    measures.sort();
    let measures = measures
        .into_iter()
        .rev()
        .take(14)
        .rev()
        .map(text)
        .collect();
    let midnight = Timestamp::from_second(now)
        .ok()
        .and_then(|t| t.to_zoned(TimeZone::system()).start_of_day().ok())
        .map_or(now - DAY, |z| z.timestamp().as_second());
    let failed = ["--system", "--user"]
        .iter()
        .filter_map(|scope| {
            command("systemctl", &[scope, "--failed", "--plain", "--no-legend"]).ok()
        })
        .collect::<String>();
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || {
            PathBuf::from(format!(
                "/run/user/{}",
                fs::metadata("/proc/self").map_or(0, |m| {
                    use std::os::unix::fs::MetadataExt;
                    m.uid()
                })
            ))
        },
        PathBuf::from,
    );
    Sources {
        measures,
        card: text(folder.join("card.txt")),
        day: newest_day(folder),
        failed,
        messages: messages(&mesh.root, now - DAY),
        ghosts: Some(ghosts(&mesh)),
        quests: text(runtime.join("jobs/quests.json")),
        waiting: fs::read_dir(state_home.join("mission/waiting"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| text(e.path()))
            .collect(),
        windows: command("niri", &["msg", "-j", "windows"]).ok(),
        launcher: command(
            "journalctl",
            &[
                "--user",
                "-u",
                "quickshell",
                &format!("--since=@{midnight}"),
                "-o",
                "cat",
                "--no-pager",
                "--grep=launcher: \".*took",
            ],
        )
        .unwrap_or_default(),
        notices: text(state_home.join("notices/notices.json")),
        pressure: text(PathBuf::from("/proc/pressure/cpu")),
        meminfo: text(PathBuf::from("/proc/meminfo")),
        now,
        midnight,
    }
}

/// The ink for standard output: the account's theme on a terminal, unless
/// `NO_COLOR` asks for none.
#[must_use]
pub fn ink(home: &Path) -> Ink {
    let config =
        std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
    let theme: Value = fs::read_to_string(config.join("jitsusama/theme.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let colour = std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty());
    Ink::of(&theme, colour)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = "Card at 19:20 (the hour since 18:20)
Core: 18 merged (0 tests or docs only).
Last commit to merge, outside boot (target median 20): median 0.9, p90 1.3, max 1.8 min (n 16).
Merge to on (target median 18): none.
  not on yet: #3438 0 min, #3433 3 min, #3363 177 min
  switches: 0, last at none
VM runs: 71, 61 passed.
VM wait for a builder (target median 2): median 0.0, p90 0.0, max 0.4 min (n 71).
The net tests 6f8501688 (9 merges behind main), merged 28 min ago.
";

    fn measure(spend: &str) -> String {
        format!(
            "# Daily Measure\n\n## Audit\n\nSpend: ${spend} over 115278 turns, up 14% on the day \
             before. Cache reads $3,893.\n\nCache rewrites, by cause:\n- after compaction: 1183 \
             times, $474\n- idle over an hour: 139 times, $192\n"
        )
    }

    #[test]
    fn the_card_says_its_medians_and_what_waits() {
        assert_eq!(card_median(CARD, "Last commit to merge"), Some(0.9));
        assert_eq!(card_median(CARD, "VM wait"), Some(0.0));
        assert_eq!(card_median(CARD, "Merge to on"), None);
        assert_eq!(card_waiting(CARD), vec![0.0, 3.0, 177.0]);
        assert_eq!(card_net(CARD), Some(28.0));
        assert_eq!(card_net("The net is caught up with main.\n"), Some(0.0));
    }

    #[test]
    fn with_none_on_in_the_hour_the_oldest_waiting_is_merge_to_on() {
        let s = Sources {
            card: CARD.into(),
            ..Sources::default()
        };
        let on = &time(&s).readings[0];
        assert_eq!(on.now, Some(177.0));
        assert!(on.out(), "a merge waiting 177 min was in spec");
    }

    #[test]
    fn the_measure_says_spend_and_rewrites() {
        assert_eq!(spend(&measure("7,293")), Some(7293.0));
        assert_eq!(after_compaction(&measure("7,293")), Some(474.0));
        assert_eq!(spend("no spend here"), None);
    }

    #[test]
    fn spend_is_out_when_this_week_costs_more_than_the_one_before() {
        let mut measures: Vec<String> = (0..7).map(|_| measure("1,000")).collect();
        measures.extend((0..7).map(|_| measure("2,000")));
        let s = Sources {
            measures,
            ..Sources::default()
        };
        let spend = &resources(&s).readings[0];
        assert_eq!(spend.bar, Some(Bar::AtMost(1000.0)));
        assert!(spend.out());
        assert_eq!(spend.words(), "$2,000 a day this week, at most $1,000");
    }

    #[test]
    fn under_two_weeks_of_measures_spend_has_no_bar() {
        let s = Sources {
            measures: (0..9).map(|_| measure("1,000")).collect(),
            ..Sources::default()
        };
        let spend = &resources(&s).readings[0];
        assert_eq!(spend.bar, None);
        assert!(!spend.out());
    }

    #[test]
    fn launcher_keys_past_fifty_ms_are_counted() {
        let journal = r#"qml: launcher: "fi" took 31 ms to its frame, 4 ms searching
qml: launcher: "fir" took 74 ms to its frame, 60 ms searching
qml: launcher: "a \" took 9" took 120 ms to its frame
qml: launcher: drawer for "Firefox" took 80 ms to its frame
"#;
        assert_eq!(launcher(journal), (2, 3));
    }

    #[test]
    fn notices_that_left_today_are_split_by_how() {
        let record = r#"{"next":4,"notices":[
            {"at":5000,"left":"expired"},{"at":6000,"left":"dismissed"},
            {"at":7000,"left":"dismissed"},{"at":100,"left":"expired"},
            {"at":8000,"left":null}]}"#;
        assert_eq!(notices(record, 1000), (1, 2));
    }

    #[test]
    fn driving_quests_and_pi_windows_are_counted() {
        assert_eq!(driving(r#"{"A":400,"B":200,"C":400}"#), Some(2.0));
        let windows = r#"[{"app_id":"pi-session.1"},{"app_id":"slack"},{"app_id":"pi-hosted.x"}]"#;
        assert_eq!(pi_windows(windows), Some(2.0));
    }

    #[test]
    fn a_question_answered_by_anyone_is_not_counted() {
        let old = (10_000 - 3600) * 1000;
        let note = |id: String, intent: &str, re: Option<&str>| Note {
            id,
            intent: intent.into(),
            re: re.map(str::to_owned),
        };
        let asked = format!("{old}-A-1-aa");
        let s = Sources {
            now: 10_000,
            messages: vec![
                note(asked.clone(), "question", None),
                note(format!("{old}-B-1-bb"), "question", None),
                note(format!("{}-C-1-cc", 9_999_000), "question", None),
                note(format!("{old}-D-1-dd"), "fyi", Some(&asked)),
            ],
            ..Sources::default()
        };
        let readings = collaboration(&s).readings;
        assert_eq!(readings[0].now, Some(25.0));
        // B alone: A was answered, and C was asked under 30 min ago.
        assert_eq!(readings[1].now, Some(1.0));
    }

    #[test]
    fn an_old_ask_waiting_on_joel_is_counted() {
        let file = r#"{"v":2,"items":[
            {"id":"a","raisedAt":"2026-11-08T03:41:59.182Z","minutes":5},
            {"id":"b","raisedAt":"2026-11-12T00:00:00Z","minutes":20}]}"#;
        let s = Sources {
            now: 1_794_444_000,
            waiting: vec![file.into()],
            ..Sources::default()
        };
        let asks = &organization(&s).readings[1];
        assert_eq!(asks.now, Some(1.0));
        assert_eq!(
            asks.detail.as_deref(),
            Some("25 min of his time asked in all")
        );
    }

    #[test]
    fn figures_are_said_plainly() {
        assert_eq!(shown(7293.0, "$"), "$7,293");
        assert_eq!(shown(1_234_567.0, ""), "1,234,567");
        assert_eq!(shown(0.9, " min"), "0.9 min");
        assert_eq!(shown(56.4, "%"), "56%");
        assert_eq!(shown(3.0, ""), "3");
        let unread = Reading::new("turns making one tool call {}", None, "%", "QCXZEB");
        assert_eq!(unread.words(), "turns making one tool call: no reading yet");
    }

    #[test]
    fn an_axis_is_found_by_the_start_of_its_name() {
        let axes = read(&Sources::default());
        assert_eq!(find(&axes, "time").map(|a| a.name), Some("time to done"));
        assert_eq!(find(&axes, "Scale").map(|a| a.name), Some("scale"));
        assert!(find(&axes, "").is_none());
        assert!(find(&axes, "nope").is_none());
    }

    fn axes() -> Vec<Axis> {
        vec![
            vec_axis(
                "scale",
                vec![
                    Reading::new("{} pi windows", Some(85.0), "", "OSPHT4")
                        .bar(Bar::AtMost(12.0))
                        .from("niri's windows"),
                    Reading::new("{} ghosts", Some(3.0), "", "85PKB2").bar(Bar::AtMost(0.0)),
                ],
            ),
            vec_axis(
                "time to done",
                vec![
                    Reading::new("VM wait {}", Some(0.4), " min", "5FWIUS")
                        .bar(Bar::AtMost(2.0))
                        .detail(Some("71 runs".into())),
                ],
            ),
            vec_axis(
                "security",
                vec![Reading::new("{} musts past their date", None, "", "ISXEG8")],
            ),
        ]
    }

    #[test]
    fn the_page_marks_each_axis_out_and_names_its_owner() {
        let page = page(
            &axes(),
            "Wednesday 11 November · read 19:45",
            &Ink::default(),
        );
        assert_eq!(
            page,
            "axes · Wednesday 11 November · read 19:45

✕ scale         85 pi windows, at most 12 · OSPHT4 · 1 more out
  time to done  VM wait 0.4 min, at most 2 min
  security      no reading yet · ISXEG8

1 of 3 out of spec · 1 not read yet · cadence axes <axis> steps into one
"
        );
    }

    #[test]
    fn an_axis_page_says_every_reading_and_where_it_comes_from() {
        let axes = axes();
        assert_eq!(
            axis_page(&axes[0], "read 19:45", &Ink::default()),
            "scale · read 19:45

✕ 85 pi windows, at most 12 · OSPHT4
    from niri's windows
✕ 3 ghosts, at most 0 · 85PKB2
"
        );
        assert_eq!(
            axis_page(&axes[2], "read 19:45", &Ink::default()),
            "security · read 19:45

  musts past their date: no reading yet · ISXEG8
"
        );
    }

    #[test]
    fn the_failed_mark_takes_the_slot_holding_the_alert_colour() {
        let theme: Value = serde_json::from_str(
            r##"{"colours":{"alert":"#FF6F5F"},
                "terminal":{"red":"#00FF00","bright_red":"#FF705F"},
                "marks":{"failed":{"glyph":"x"}}}"##,
        )
        .unwrap();
        let ink = Ink::of(&theme, true);
        assert_eq!(ink.failed, "x");
        assert_eq!(ink.alert.as_deref(), Some("91"));
        assert_eq!(Ink::of(&theme, false).alert, None);
    }
}
