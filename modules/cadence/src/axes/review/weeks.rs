//! Week on week: the newest day cadence kept, each of its figures against
//! the day kept a week before it, by the same fifth and floor the day is
//! judged by against its median. A figure only one of them has is left
//! out, since a week can't move what wasn't read.

use crate::DAY;
use crate::day::{Day, measures};
use crate::judge::round;

/// The newest day against the one a week before it.
#[derive(Debug, Default, PartialEq)]
pub struct Weeks {
    /// The newest day's date.
    pub newest: String,
    /// The date of the day a week before it, when one was kept.
    pub before: Option<String>,
    /// The oldest day kept, to say when a comparison first can be.
    pub oldest: String,
    pub worse: Vec<String>,
    pub better: Vec<String>,
    /// How many figures both days read.
    pub held: usize,
}

/// The newest of `days` against the one that ended a week before it, give
/// or take half a day; `None` with no day kept at all.
pub fn weeks(days: &[Day]) -> Option<Weeks> {
    let newest = days.iter().max_by_key(|d| d.to)?;
    let oldest = days.iter().min_by_key(|d| d.to)?;
    let before = days
        .iter()
        .find(|d| (d.to - (newest.to - 7 * DAY)).abs() <= DAY / 2);
    let mut weeks = Weeks {
        newest: newest.date.clone(),
        before: before.map(|d| d.date.clone()),
        oldest: oldest.date.clone(),
        ..Weeks::default()
    };
    let Some(before) = before else {
        return Some(weeks);
    };
    for measure in measures() {
        for (key, whose, now) in measure.read(&newest.figures) {
            let Some(&then) = before.figures.get(key) else {
                continue;
            };
            weeks.held += 1;
            let says = whose.map_or_else(
                || measure.says.to_owned(),
                |whose| format!("{whose}'s {}", measure.says),
            );
            let unit = measure.unit;
            let words = format!(
                "{says} went from {}{unit} to {}{unit}",
                round(then),
                round(now)
            );
            let worse = measure.worse();
            if now > then * worse && now - then >= measure.floor {
                weeks.worse.push(words);
            } else if now * worse < then && then - now >= measure.floor {
                weeks.better.push(words);
            }
        }
    }
    Some(weeks)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TO: i64 = 1_794_610_800;

    fn day(date: &str, to: i64, figures: &[(&str, f64)]) -> Day {
        Day {
            date: date.to_owned(),
            from: to - DAY,
            to,
            figures: figures.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
            ..Day::default()
        }
    }

    #[test]
    fn each_figure_is_held_against_the_day_a_week_before() {
        let days = [
            day(
                "2026-11-06",
                TO - 7 * DAY + 600,
                &[
                    ("oom.cgroup", 2.0),
                    ("pressure.cpu.jobs", 35.0),
                    ("pressure.cpu.app", 0.5),
                ],
            ),
            day("2026-11-12", TO - DAY, &[("oom.cgroup", 40.0)]),
            day(
                "2026-11-13",
                TO,
                &[
                    ("oom.cgroup", 10.0),
                    ("pressure.cpu.jobs", 20.0),
                    ("pressure.cpu.app", 0.5),
                    ("oom.machine", 0.0),
                ],
            ),
        ];
        assert_eq!(
            weeks(&days),
            Some(Weeks {
                newest: "2026-11-13".to_owned(),
                before: Some("2026-11-06".to_owned()),
                oldest: "2026-11-06".to_owned(),
                worse: vec!["kills inside a slice's own memory limit went from 2 to 10".to_owned()],
                better: vec!["the agents' jobs' CPU pressure went from 35% to 20%".to_owned()],
                held: 3,
            }),
            "the newest day is held against the week before it, not the day before; \
             a figure only one of them read is left out"
        );
    }

    #[test]
    fn with_no_day_a_week_before_nothing_is_compared() {
        let days = [
            day("2026-11-11", TO - 2 * DAY, &[("oom.cgroup", 2.0)]),
            day("2026-11-13", TO, &[("oom.cgroup", 10.0)]),
        ];
        assert_eq!(
            weeks(&days),
            Some(Weeks {
                newest: "2026-11-13".to_owned(),
                before: None,
                oldest: "2026-11-11".to_owned(),
                ..Weeks::default()
            }),
            "two days apart is no week"
        );
        assert_eq!(weeks(&[]), None, "no day kept is no comparison");
    }
}
