//! Each model's ratio of bill to list price, learned from hours a feed's
//! warehouse settled, so that once the bill goes quiet (signed out, cut
//! off, a personal machine after a work one) pi's logs still give a close
//! figure. Learned over the last thirty days the bill covered, by model,
//! with the spread of its daily ratios so an estimate can say how close it
//! is; a model never billed takes the ratio of everything together.

use std::collections::{BTreeMap, HashMap};

/// The least list-priced spend a model's ratio is learned from.
const LEARN_FLOOR: f64 = 20.0;
pub const LEARN_DAYS: i64 = 30;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ratio {
    pub ratio: f64,
    /// The tenth and ninetieth percentiles of its daily ratios.
    pub low: f64,
    pub high: f64,
    pub list: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ratios {
    pub models: HashMap<String, Ratio>,
    pub overall: Option<Ratio>,
}

impl Ratios {
    pub fn fixed(models: &[(&str, f64)]) -> Ratios {
        Ratios {
            models: models
                .iter()
                .map(|(model, ratio)| {
                    (
                        (*model).to_owned(),
                        Ratio {
                            ratio: *ratio,
                            low: *ratio,
                            high: *ratio,
                            list: 0.0,
                        },
                    )
                })
                .collect(),
            overall: None,
        }
    }

    pub fn of(&self, model: &str) -> Option<f64> {
        self.models
            .get(model)
            .or(self.overall.as_ref())
            .map(|ratio| ratio.ratio)
    }

    /// From (hour, model, list, billed) for each settled hour.
    pub fn learn(rows: &[(i64, String, f64, f64)]) -> Ratios {
        let Some(last) = rows.iter().map(|row| row.0).max() else {
            return Ratios::default();
        };
        let since = last - LEARN_DAYS * 24;
        let mut daily: HashMap<&str, BTreeMap<i64, (f64, f64)>> = HashMap::new();
        let mut all: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for (hour, model, list, billed) in rows.iter().filter(|row| row.0 >= since) {
            let day = hour.div_euclid(24);
            let entry = daily.entry(model).or_default().entry(day).or_default();
            entry.0 += list;
            entry.1 += billed;
            let entry = all.entry(day).or_default();
            entry.0 += list;
            entry.1 += billed;
        }
        let ratio = |days: &BTreeMap<i64, (f64, f64)>| -> Option<Ratio> {
            let list: f64 = days.values().map(|day| day.0).sum();
            let billed: f64 = days.values().map(|day| day.1).sum();
            if list < LEARN_FLOOR || billed <= 0.0 {
                return None;
            }
            let mut each: Vec<f64> = days
                .values()
                .filter(|day| day.0 >= 1.0)
                .map(|day| day.1 / day.0)
                .collect();
            each.sort_by(f64::total_cmp);
            let at = |q: f64| crate::stats::quantile_sorted(&each, q);
            Some(Ratio {
                ratio: billed / list,
                low: at(0.1),
                high: at(0.9),
                list,
            })
        };
        Ratios {
            models: daily
                .iter()
                .filter_map(|(model, days)| ratio(days).map(|r| ((*model).to_owned(), r)))
                .collect(),
            overall: ratio(&all),
        }
    }
}

/// The settled days a price's change is judged on, newest last.
pub const SHIFT_DAYS: usize = 3;

/// A model the bill now prices apart from how it did: its ratio of bill to
/// list over the last few settled days against the month before.
#[derive(Debug, Clone, PartialEq)]
pub struct Shift {
    pub model: String,
    pub before: f64,
    pub now: f64,
    /// The first day of the change, as `day` numbers it.
    pub since: i64,
    /// List-priced spend a day in those days, to say what it's worth.
    pub list_a_day: f64,
}

/// Each model whose last `SHIFT_DAYS` settled days, ending on the last day
/// anything settled, all fall outside the tenth to ninetieth percentiles
/// of the days before on the same side, agree with each other within a
/// tenth, and move its ratio by a tenth or more. A price moves in a step
/// and stays; spend some other machine made, with no log here, comes and
/// goes by the day. `day` says which day an hour is in.
pub fn shifts(rows: &[(i64, String, f64, f64)], day: impl Fn(i64) -> i64) -> Vec<Shift> {
    let mut daily: BTreeMap<&str, BTreeMap<i64, (f64, f64)>> = BTreeMap::new();
    for (hour, model, list, billed) in rows {
        let entry = daily
            .entry(model)
            .or_default()
            .entry(day(*hour))
            .or_default();
        entry.0 += list;
        entry.1 += billed;
    }
    let last = daily
        .values()
        .filter_map(|days| days.keys().next_back().copied())
        .max();
    let mut out = Vec::new();
    for (model, days) in daily {
        if days.keys().next_back().copied() != last {
            continue;
        }
        // Days with too little spend say little of a price.
        let days: Vec<(i64, f64, f64)> = days
            .into_iter()
            .filter(|(_, (list, _))| *list >= 1.0)
            .map(|(day, (list, billed))| (day, list, billed))
            .collect();
        if days.len() < SHIFT_DAYS + 7 {
            continue;
        }
        let (earlier, recent) = days.split_at(days.len() - SHIFT_DAYS);
        let earlier = &earlier[earlier.len().saturating_sub(LEARN_DAYS as usize)..];
        let sum = |days: &[(i64, f64, f64)]| {
            days.iter().fold((0.0, 0.0), |(list, billed), day| {
                (list + day.1, billed + day.2)
            })
        };
        let ((before_list, before_billed), (now_list, now_billed)) = (sum(earlier), sum(recent));
        if before_list < LEARN_FLOOR || now_list < LEARN_FLOOR {
            continue;
        }
        let mut each: Vec<f64> = earlier.iter().map(|day| day.2 / day.1).collect();
        each.sort_by(f64::total_cmp);
        let (low, high) = (
            crate::stats::quantile_sorted(&each, 0.1),
            crate::stats::quantile_sorted(&each, 0.9),
        );
        let ratios: Vec<f64> = recent.iter().map(|day| day.2 / day.1).collect();
        let all_below = ratios.iter().all(|ratio| *ratio < low);
        let all_above = ratios.iter().all(|ratio| *ratio > high);
        let (before, now) = (before_billed / before_list, now_billed / now_list);
        let steady = ratios.iter().all(|ratio| (ratio / now - 1.0).abs() <= 0.1);
        if (all_below || all_above) && steady && before > 0.0 && (now / before - 1.0).abs() >= 0.1 {
            out.push(Shift {
                model: model.to_owned(),
                before,
                now,
                since: recent[0].0,
                list_a_day: now_list / SHIFT_DAYS as f64,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(model: &str, ratios: &[f64]) -> Vec<(i64, String, f64, f64)> {
        ratios
            .iter()
            .enumerate()
            .map(|(day, ratio)| (day as i64 * 24, model.to_owned(), 100.0, 100.0 * ratio))
            .collect()
    }

    // Three days on at a new ratio is a price; one odd day isn't, nor is
    // a small drift.
    #[test]
    fn a_price_is_three_days_at_a_new_ratio() {
        let usual: Vec<f64> = (0..20).map(|day| 0.70 + 0.02 * (day % 3) as f64).collect();
        let moved = [usual.clone(), vec![0.55, 0.56, 0.54]].concat();
        let shifts = |rows: &[(i64, String, f64, f64)]| shifts(rows, |hour| hour.div_euclid(24));
        let found = shifts(&days("opus", &moved));
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].model.as_str(), found[0].since), ("opus", 20));
        assert!((found[0].now - 0.55).abs() < 1e-9);
        assert!((found[0].before - 0.72).abs() < 0.01);
        let once = [usual.clone(), vec![0.71, 0.40, 0.72]].concat();
        assert!(shifts(&days("opus", &once)).is_empty());
        let unsteady = [usual.clone(), vec![1.2, 1.9, 1.4]].concat();
        assert!(shifts(&days("opus", &unsteady)).is_empty());
        let mut gone = days("opus", &moved);
        gone.push((30 * 24, "sonnet".to_owned(), 10.0, 7.0));
        assert!(shifts(&gone).is_empty());
        let drift = [usual.clone(), vec![0.67, 0.67, 0.67]].concat();
        assert!(shifts(&days("opus", &drift)).is_empty());
        assert!(shifts(&days("opus", &[0.7, 0.5, 0.5, 0.5])).is_empty());
    }

    #[test]
    fn a_ratio_is_billed_over_list_with_its_spread() {
        let mut rows = Vec::new();
        for day in 0..10 {
            let billed = if day % 2 == 0 { 76.0 } else { 90.0 };
            rows.push((day * 24, "opus".to_owned(), 100.0, billed));
        }
        let ratios = Ratios::learn(&rows);
        let opus = &ratios.models["opus"];
        assert!((opus.ratio - 0.83).abs() < 1e-9);
        assert!(opus.low <= 0.77 && opus.high >= 0.89);
        assert_eq!(
            ratios.of("never-billed"),
            Some(ratios.overall.unwrap().ratio)
        );
    }

    #[test]
    fn too_little_spend_teaches_nothing() {
        let ratios = Ratios::learn(&[(0, "opus".to_owned(), 5.0, 4.0)]);
        assert!(ratios.models.is_empty() && ratios.overall.is_none());
    }
}
