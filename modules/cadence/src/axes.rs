//! `cadence axes`: the nine axes Joel holds the machine to (resources,
//! time to done, quality, collaboration, security, organization, scale,
//! polish and his stress), each read now against its bar, so he sees in
//! one page which are out of spec and who has each. `cadence axes <axis>`
//! steps into one and says where each reading comes from.
//!
//! Every reading is a script's, from a record the machine already keeps
//! (readings.rs): cadence's card and day, the daily measure, the journal,
//! the mesh, the notices and niri. No model runs, and the page draws in
//! under a second. A reading nothing measures yet says so and names its
//! owner, rather than leave the axis looking fine. The bars are first
//! guesses from docs/delivery.md and the research behind this view
//! (RSCH-20261111-I1QC91); they only ever tighten.

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
                readings: vec![
                    Reading::new("{} pi windows", Some(85.0), "", "OSPHT4")
                        .bar(Bar::AtMost(12.0))
                        .from("niri's windows"),
                    Reading::new("{} ghosts", Some(3.0), "", "85PKB2").bar(Bar::AtMost(0.0)),
                ],
            },
            Axis {
                name: "time to done",
                readings: vec![
                    Reading::new("VM wait {}", Some(0.4), " min", "5FWIUS")
                        .bar(Bar::AtMost(2.0))
                        .detail(Some("71 runs".into())),
                ],
            },
            Axis {
                name: "security",
                readings: vec![Reading::new("{} musts past their date", None, "", "ISXEG8")],
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
