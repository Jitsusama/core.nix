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

#[cfg(test)]
mod tests {
    use super::*;

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
