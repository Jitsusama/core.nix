//! Where a Mac's power comes from, as `pmset -g batt` says, and whether
//! its person asked to save it, as `pmset -g` does. Read on any platform,
//! so Linux's tests hold it.

use crate::moments::Power;

/// Whether Low Power Mode is on among the settings in use.
pub fn low_power(text: &str) -> bool {
    text.lines().any(|line| {
        let mut words = line.split_whitespace();
        words.next() == Some("lowpowermode") && words.next() == Some("1")
    })
}

/// The source it's drawing from, which is the charger only as AC Power, and the
/// average charge of the machine's own batteries. A UPS's charge isn't the
/// machine's. Nothing when the output doesn't say where power comes from.
pub fn power(text: &str) -> Option<Power> {
    let source = text
        .lines()
        .find_map(|line| line.strip_prefix("Now drawing from '"))?
        .split('\'')
        .next()?;
    let charges: Vec<u32> = text
        .lines()
        .filter(|line| line.trim_start().starts_with("-InternalBattery"))
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_suffix("%;")?.parse().ok())
        })
        .collect();
    let total: u32 = charges.iter().sum();
    let battery = u32::try_from(charges.len())
        .ok()
        .and_then(|count| total.checked_div(count))
        .and_then(|mean| u8::try_from(mean).ok());
    Some(Power {
        on_charger: source == "AC Power",
        battery,
    })
}
