//! Where a Mac's power comes from, as `pmset -g batt` says. Read on any
//! platform, so Linux's tests hold it.

use crate::moments::Power;

/// The source it's drawing from, which is mains only as AC power, and the
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
    let battery = (!charges.is_empty())
        .then(|| u8::try_from(charges.iter().sum::<u32>() / charges.len() as u32).ok())
        .flatten();
    Some(Power {
        mains: source == "AC Power",
        battery,
    })
}
