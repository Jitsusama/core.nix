//! The launcher's speed over the day, from the lines the shell writes to
//! its journal as Joel uses it: how long an opening took to show and to
//! answer, and how long a key or a daemon's change took to reach the
//! screen, against the 50 ms bar docs/design.md sets.

use std::collections::BTreeMap;

use crate::day::median;

/// The day's launcher figures, from `journalctl --user -u quickshell -o
/// cat`, each keyed as `MEASURES` names it. A figure with no lines behind
/// it is left out.
#[must_use]
pub fn launcher(journal: &str) -> BTreeMap<String, f64> {
    let mut presented = Vec::new();
    let mut usable = Vec::new();
    let mut keys = Batches::default();
    let mut changes = Batches::default();
    for line in journal.lines() {
        let Some((_, said)) = line.split_once("launcher: ") else {
            continue;
        };
        if let Some(rest) = said.strip_prefix("opening presented ") {
            presented.extend(first_number(rest));
        } else if let Some(rest) = said.strip_prefix("opening usable ") {
            usable.extend(first_number(rest));
        } else if said.contains(" keys presented a median ") {
            keys.add(said);
        } else if said.contains(" in ") && said.contains(" presented a median ") {
            // "N changes in agents presented ...", or "1 change in ...".
            changes.add(said);
        }
    }
    let mut figures = BTreeMap::new();
    let mut put = |key: &str, value: Option<f64>| {
        if let Some(value) = value {
            figures.insert(key.to_owned(), value);
        }
    };
    put("launcher.opening.presented", median(&presented));
    put("launcher.opening.usable", median(&usable));
    put("launcher.keys.presented", median(&keys.medians));
    put("launcher.keys.late", keys.late());
    put("launcher.changes.presented", median(&changes.medians));
    figures
}

/// The shell's summary lines of keys or changes: each batch's median,
/// once for each thing in it, so the day's median weighs every key alike,
/// and how many were within the bar.
#[derive(Default)]
struct Batches {
    medians: Vec<f64>,
    all: u32,
    within: u32,
}

impl Batches {
    /// From "N ... presented a median M ms from the ..., the slowest S ms,
    /// W within 50 ms", with or without a figure between the two.
    fn add(&mut self, said: &str) {
        let Some(count) = first_number(said) else {
            return;
        };
        let Some(middle) = said
            .split_once(" a median ")
            .and_then(|(_, rest)| first_number(rest))
        else {
            return;
        };
        // The count just before " within ", so a figure the shell adds
        // after the slowest, such as its p99, is no count.
        let within = said
            .split_once(" within ")
            .and_then(|(before, _)| before.rsplit(' ').next())
            .and_then(|count| count.parse::<f64>().ok());
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count the shell wrote as a whole number"
        )]
        let count = count as u32;
        for _ in 0..count {
            self.medians.push(middle);
        }
        self.all += count;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count the shell wrote as a whole number"
        )]
        let within = within.map_or(0, |w| w as u32);
        self.within += within.min(count);
    }

    /// The share of them past the bar, in percent.
    fn late(&self) -> Option<f64> {
        (self.all > 0).then(|| 100.0 * f64::from(self.all - self.within) / f64::from(self.all))
    }
}

fn first_number(text: &str) -> Option<f64> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|part| !part.is_empty())?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launchers_lines_become_the_days_figures() {
        let journal = "\
  INFO qml: launcher: opening presented 65 ms from the chord
  INFO qml: launcher: opening usable 312 ms from the chord, its longest stall 105 ms
  INFO qml: launcher: opening presented 233 ms from the chord
  INFO qml: launcher: opening presented 40 ms from the chord
  INFO qml: launcher: 3 keys presented a median 60 ms from the key, the slowest 71 ms, \
0 within 50 ms: a median 2 ms before the shell heard it, 10 in the shell
  INFO qml: launcher: 1 keys presented a median 20 ms from the key, the slowest 20 ms, \
the p99 20 ms, 1 within 50 ms
  INFO qml: launcher: 3 keys landed a median 79 ms from the key, the slowest 89 ms, 0 within 50 ms
  INFO qml: launcher: 1 change in on presented a median 76 ms from the line, the slowest \
76 ms, 0 within 50 ms
  INFO qml: launcher: \"↓\" took 4 ms to its frame, 2 ms searching
";
        let figures = launcher(journal);
        let want: BTreeMap<String, f64> = [
            ("launcher.opening.presented", 65.0),
            ("launcher.opening.usable", 312.0),
            ("launcher.keys.presented", 60.0),
            ("launcher.keys.late", 75.0),
            ("launcher.changes.presented", 76.0),
        ]
        .map(|(k, v)| (k.to_owned(), v))
        .into();
        assert_eq!(
            figures, want,
            "an opening, a key or a change was misread, or a line that isn't one was read"
        );
    }

    #[test]
    fn a_day_without_the_launcher_has_no_launcher_figures() {
        assert!(
            launcher("  INFO qml: notices: shown\n").is_empty(),
            "a day with no launcher lines gave it figures"
        );
    }
}
