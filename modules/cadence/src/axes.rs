//! `cadence axes`: the ten axes Joel holds the machine to (resources,
//! time to done, quality, collaboration, security, organization, scale,
//! polish, responsiveness and his stress), each read now against its bar, so he sees in
//! one page which are out of spec and who has each. `cadence axes <axis>`
//! steps into one and says where each reading comes from.
//!
//! Every reading is a script's, from a record the machine already keeps
//! (readings.rs): cadence's card and day, the daily measure, the journal,
//! the mesh, the notices and niri. No model runs, and the page draws in
//! under a second. A reading nothing measures yet says so and names its
//! owner, rather than leave the axis looking fine. Only a reading taken
//! from Joel's side (his key, his screen, his time) can show an axis in
//! spec: one read from ours is a stand-in, and an axis read on one is
//! marked not judged, since what we measure is what we fix. The bars are first
//! guesses from docs/delivery.md and the research behind this view
//! (RSCH-20261111-I1QC91); they only ever tighten.

mod counted;
mod landings;
mod open;
mod plans;
mod readings;

use std::fmt::Write;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::Value;

use crate::{folders, now_ms};
use readings::{gather, read};

/// `cadence axes [AXIS]`: prints the ten axes, or the one named.
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
    let mut axes = read(&gather(&folder, &state_home, now));
    let ink = ink(&home);
    let sep = format!(" {} ", ink.separator);
    open::attach(
        &mut axes,
        &open::gather(&folder),
        now,
        &TimeZone::system(),
        &sep,
    );
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
    /// Whether it's read from Joel's side rather than from what we built.
    pub his: bool,
    /// Why its figure is held back, when its sources disagree on it.
    pub held: Option<&'static str>,
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
            his: false,
            held: None,
        }
    }

    fn his(mut self) -> Self {
        self.his = true;
        self
    }

    fn held(mut self, why: &'static str) -> Self {
        self.held = Some(why);
        self
    }

    /// The figure as it may be shown: none while it's held back.
    fn figure(&self) -> Option<f64> {
        self.now.filter(|_| self.held.is_none())
    }

    /// Whether it can say its axis is in spec: read, shown, and from his
    /// side.
    fn judges(&self) -> bool {
        self.his && self.figure().is_some()
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

    /// Whether its figure is past its bar; a reading with no figure, a
    /// held one or no bar is never out.
    #[must_use]
    pub fn out(&self) -> bool {
        match (self.figure(), self.bar) {
            (Some(now), Some(Bar::AtMost(bar))) => now > bar,
            (Some(now), Some(Bar::AtLeast(bar))) => now < bar,
            _ => false,
        }
    }

    /// The reading as a line says it: "85 pi windows, at most 12".
    #[must_use]
    pub fn words(&self) -> String {
        let Some(now) = self.figure() else {
            let named = self.says.replace(" {}", "").replace("{} ", "");
            let why = if self.held.is_some() {
                "not agreed yet"
            } else {
                "no reading yet"
            };
            return format!("{named}: {why}");
        };
        let says = if (now - 1.0).abs() < f64::EPSILON && self.unit.is_empty() {
            one(self.says)
        } else {
            self.says.to_owned()
        };
        let said = says.replacen("{}", &shown(now, self.unit), 1);
        match self.bar {
            Some(Bar::AtMost(bar)) => format!("{said}, at most {}", shown(bar, self.unit)),
            Some(Bar::AtLeast(bar)) => format!("{said}, at least {}", shown(bar, self.unit)),
            None => said,
        }
    }
}

/// One of the ten axes and what it's read by.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub name: &'static str,
    pub readings: Vec<Reading>,
    /// What the daily run found worse on it and hasn't seen fixed.
    pub findings: Vec<open::Open>,
}

/// Where an axis stands: past a bar, in spec on his side's readings
/// alone, read only on stand-ins or in part, or not read at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Out,
    InSpec,
    StandIn,
    Unread,
}

impl Axis {
    fn out(&self) -> Vec<&Reading> {
        self.readings.iter().filter(|r| r.out()).collect()
    }

    fn standing(&self) -> Standing {
        if self.readings.iter().any(Reading::out) {
            Standing::Out
        } else if self.readings.iter().all(|r| r.figure().is_none()) {
            Standing::Unread
        } else if self.readings.iter().all(Reading::judges) {
            Standing::InSpec
        } else {
            Standing::StandIn
        }
    }
}

/// A figure as a line says it: whole from ten, else to one place, with
/// thousands set apart.
#[must_use]
/// A count's words for one of it: the first plural among the three words
/// after its figure goes singular, so "{} core changes" says "1 core
/// change" and "{} crashes" says "1 crash". Words with no plural there,
/// like "{} a day", stay as they are.
fn one(says: &str) -> String {
    let Some((before, after)) = says.split_once("{}") else {
        return says.to_owned();
    };
    let mut words: Vec<String> = after.split(' ').map(str::to_owned).collect();
    if let Some(word) = words
        .iter_mut()
        .skip(1)
        .take(3)
        .find(|w| w.len() > 2 && w.ends_with('s'))
    {
        let cut = if ["ches", "shes", "sses", "xes"]
            .iter()
            .any(|end| word.ends_with(end))
        {
            2
        } else {
            1
        };
        word.truncate(word.len() - cut);
    }
    format!("{before}{{}}{}", words.join(" "))
}

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
    /// The mark of an axis not yet judged from his side.
    pub pending: String,
    pub separator: String,
    pub alert: Option<String>,
}

impl Default for Ink {
    /// The theme module's own marks (modules/theme/glyphs.nix), uncoloured.
    fn default() -> Self {
        Self {
            failed: "✕".into(),
            pending: "◇".into(),
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
            pending: mark("pending", defaults.pending),
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

/// The ink for standard output: the account's theme on a terminal, unless
/// `NO_COLOR` asks for none.
fn ink(home: &Path) -> Ink {
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

/// The axis `asked` names, by the start of its name: `time` is time to
/// done.
#[must_use]
pub fn find<'a>(axes: &'a [Axis], asked: &str) -> Option<&'a Axis> {
    let asked = asked.to_lowercase();
    axes.iter()
        .find(|a| !asked.is_empty() && a.name.starts_with(&asked))
}

/// The page of all ten: each axis's one line, marked when out of spec,
/// saying its first reading out with its owner; marked pending when it's
/// read only on stand-ins or in part, saying so; else its first reading.
#[must_use]
pub fn page(axes: &[Axis], heading: &str, ink: &Ink) -> String {
    let sep = format!(" {} ", ink.separator);
    let width = axes.iter().map(|a| a.name.len()).max().unwrap_or(0);
    let mut out = format!("axes{sep}{heading}\n\n");
    for axis in axes {
        let off = axis.out();
        let shown = axis.readings.iter().find(|r| r.figure().is_some());
        let (mark, said) = match (axis.standing(), off.first(), shown) {
            (Standing::Out, Some(first), _) => {
                let more = match off.len() {
                    1 => String::new(),
                    n => format!("{sep}{} more out", n - 1),
                };
                (
                    ink.failed(),
                    format!("{}{sep}{}{more}", first.words(), first.owner),
                )
            }
            (Standing::InSpec, _, Some(first)) => (" ".to_owned(), first.words()),
            (Standing::StandIn, _, Some(first)) => {
                let why = if first.his {
                    let unread = axis.readings.iter().filter(|r| !r.judges()).count();
                    format!("{unread} more not read from your side")
                } else {
                    "read on a stand-in".to_owned()
                };
                (ink.pending.clone(), format!("{}{sep}{why}", first.words()))
            }
            _ => {
                let first = axis.readings.first();
                let said = match first.and_then(|r| r.held) {
                    Some(_) => "not agreed yet",
                    None => "no reading yet",
                };
                let owner = first.map_or("", |r| r.owner);
                (ink.pending.clone(), format!("{said}{sep}{owner}"))
            }
        };
        let findings = match axis.findings.len() {
            0 => String::new(),
            1 => format!("{sep}1 finding open"),
            n => format!("{sep}{n} findings open"),
        };
        let _ = writeln!(out, "{mark} {:width$}  {said}{findings}", axis.name);
    }
    let tally = |standing: Standing| axes.iter().filter(|a| a.standing() == standing).count();
    let _ = write!(
        out,
        "\n{} of {} out of spec",
        tally(Standing::Out),
        axes.len()
    );
    for (standing, said) in [
        (Standing::InSpec, "in spec"),
        (Standing::StandIn, "read on a stand-in"),
        (Standing::Unread, "not read yet"),
    ] {
        let n = tally(standing);
        if n > 0 {
            let _ = write!(out, "{sep}{n} {said}");
        }
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
        } else if reading.judges() {
            " ".to_owned()
        } else {
            ink.pending.clone()
        };
        let _ = writeln!(out, "{mark} {}{sep}{}", reading.words(), reading.owner);
        // A held figure's detail says it too, so only why it's held shows.
        let detail = reading
            .held
            .map_or_else(|| reading.detail.clone(), |why| Some(why.into()));
        let side = (!reading.his && reading.figure().is_some())
            .then(|| "a stand-in, read from our side rather than yours".to_owned());
        let more: Vec<String> = detail
            .into_iter()
            .chain(side)
            .chain((!reading.from.is_empty()).then(|| format!("from {}", reading.from)))
            .collect();
        if !more.is_empty() {
            let _ = writeln!(out, "    {}", more.join(&sep));
        }
    }
    if !axis.findings.is_empty() {
        out.push_str("\nfound worse and not yet fixed\n");
    }
    for finding in &axis.findings {
        let mark = if finding.late {
            ink.failed()
        } else {
            ink.pending.clone()
        };
        let _ = writeln!(out, "{mark} {}", finding.said);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::readings::Sources;
    use super::*;

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
    fn a_count_of_one_says_its_thing_once() {
        let said = |says, n: f64| Reading::new(says, Some(n), "", "WCJQSG").words();
        assert_eq!(
            said("{} quests worked on with no live plan", 1.0),
            "1 quest worked on with no live plan"
        );
        assert_eq!(said("{} crashes over the day", 1.0), "1 crash over the day");
        assert_eq!(
            said("{} core changes reached you", 1.0),
            "1 core change reached you"
        );
        assert_eq!(
            said("{} machine-wide out-of-memory kills", 1.0),
            "1 machine-wide out-of-memory kill"
        );
        assert_eq!(said("{} a day", 1.0), "1 a day");
        assert_eq!(
            said("{} crashes over the day", 2.0),
            "2 crashes over the day"
        );
        let share = Reading::new("{} of notices cleared", Some(1.0), "%", "2RCBOJ");
        assert_eq!(share.words(), "1% of notices cleared");
    }

    #[test]
    fn an_axis_is_found_by_the_start_of_its_name() {
        let axes = read(&Sources::default());
        let found = |asked: &str| find(&axes, asked).map(|a| a.name);
        assert_eq!(found("time"), Some("time to done"));
        assert_eq!(found("Scale"), Some("scale"));
        assert_eq!(found(""), None);
        assert_eq!(found("nope"), None);
    }

    fn axes() -> Vec<Axis> {
        vec![
            Axis {
                name: "scale",
                findings: vec![open::Open {
                    late: true,
                    said: "ghosts rose · 85PKB2 · found 2026-11-11 · late since Thu 12 18:00"
                        .into(),
                }],
                readings: vec![
                    Reading::new("{} pi windows", Some(85.0), "", "OSPHT4")
                        .bar(Bar::AtMost(12.0))
                        .from("niri's windows")
                        .his(),
                    Reading::new("{} ghosts", Some(3.0), "", "85PKB2").bar(Bar::AtMost(0.0)),
                ],
            },
            Axis {
                name: "time to done",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("VM wait {}", Some(0.4), " min", "5FWIUS")
                        .bar(Bar::AtMost(2.0))
                        .detail(Some("71 runs".into())),
                ],
            },
            Axis {
                name: "security",
                findings: Vec::new(),
                readings: vec![Reading::new("{} musts past their date", None, "", "ISXEG8")],
            },
            Axis {
                name: "polish",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("{} slow keys", Some(0.0), "", "Y2YBTS")
                        .bar(Bar::AtMost(0.0))
                        .his(),
                ],
            },
            Axis {
                name: "stress",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("{} asks waiting", Some(0.0), "", "WCJQSG")
                        .bar(Bar::AtMost(0.0))
                        .his(),
                    Reading::new("{} complaints", None, "%", "1JQPPC").his(),
                ],
            },
            Axis {
                name: "resources",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("{} a day", Some(9000.0), "$", "QCXZEB")
                        .bar(Bar::AtMost(1.0))
                        .detail(Some("yesterday $9,000".into()))
                        .held("its sources disagree"),
                ],
            },
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

✕ scale         85 pi windows, at most 12 · OSPHT4 · 1 more out · 1 finding open
◇ time to done  VM wait 0.4 min, at most 2 min · read on a stand-in
◇ security      no reading yet · ISXEG8
  polish        0 slow keys, at most 0
◇ stress        0 asks waiting, at most 0 · 1 more not read from your side
◇ resources     not agreed yet · QCXZEB

1 of 6 out of spec · 1 in spec · 2 read on a stand-in · 2 not read yet · cadence axes <axis> steps into one
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
    a stand-in, read from our side rather than yours

found worse and not yet fixed
✕ ghosts rose · 85PKB2 · found 2026-11-11 · late since Thu 12 18:00
"
        );
        assert_eq!(
            axis_page(&axes[5], "read 19:45", &Ink::default()),
            "resources · read 19:45

◇ a day: not agreed yet · QCXZEB
    its sources disagree
",
            "a held figure showed on its axis's page"
        );
        assert_eq!(
            axis_page(&axes[2], "read 19:45", &Ink::default()),
            "security · read 19:45

◇ musts past their date: no reading yet · ISXEG8
"
        );
    }

    #[test]
    fn the_failed_mark_takes_the_slot_holding_the_alert_colour() {
        // The one theme cadence's tests name colours in, kept in a file of
        // its own since its colours are what's tested, not a look.
        let theme: Value = serde_json::from_str(include_str!("../tests/theme.json")).unwrap();
        let ink = Ink::of(&theme, true);
        assert_eq!(ink.failed, "x");
        // Its alert is nearest bright red's pigment, not red's slot.
        assert_eq!(ink.alert.as_deref(), Some("91"));
        assert_eq!(Ink::of(&theme, false).alert, None);
    }
}
