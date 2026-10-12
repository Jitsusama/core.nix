//! The two kinds of quantity a config names: how long, and how much.

use std::time::Duration;

/// How long, written as a run of whole numbers each with a unit: `20s`, `5m`,
/// `6h`, `30d`, or several together, as `1d1h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span(pub Duration);

impl Span {
    pub fn parse(text: &str) -> Result<Span, String> {
        let refused = || format!("{text:?} is not a span such as 20s, 5m, 6h, 30d or 1d1h");
        if text.is_empty() {
            return Err(refused());
        }
        let mut seconds: u64 = 0;
        let mut digits = String::new();
        for c in text.chars() {
            if c.is_ascii_digit() {
                digits.push(c);
                continue;
            }
            let unit = match c {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                _ => return Err(refused()),
            };
            let count: u64 = digits.parse().map_err(|_| refused())?;
            seconds = count
                .checked_mul(unit)
                .and_then(|part| seconds.checked_add(part))
                .ok_or_else(refused)?;
            digits.clear();
        }
        if !digits.is_empty() {
            return Err(refused());
        }
        Ok(Span(Duration::from_secs(seconds)))
    }
}

/// How much space: a number of bytes, or a share of the filesystem it is
/// measured on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Bytes(u64),
    Percent(f64),
}

impl Size {
    /// Whole bytes, a share up to `100%`, or whole binary units from `KiB` to
    /// `TiB`. Decimal units are refused, since `24G` could mean either.
    pub fn parse(text: &str) -> Result<Size, String> {
        let refused = || format!("{text:?} is not a size such as 4096, 15% or 24GiB");
        if let Some(share) = text.strip_suffix('%') {
            let share: f64 = share.parse().map_err(|_| refused())?;
            return if (0.0..=100.0).contains(&share) {
                Ok(Size::Percent(share))
            } else {
                Err(refused())
            };
        }
        let (count, shift) = [("KiB", 10), ("MiB", 20), ("GiB", 30), ("TiB", 40)]
            .iter()
            .find_map(|(unit, shift)| text.strip_suffix(unit).map(|count| (count, *shift)))
            .unwrap_or((text, 0));
        if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
            return Err(refused());
        }
        let count: u64 = count.parse().map_err(|_| refused())?;
        count
            .checked_mul(1 << shift)
            .map(Size::Bytes)
            .ok_or_else(refused)
    }

    /// The bytes this size comes to on a filesystem of `total` bytes.
    pub fn of(self, total: u64) -> u64 {
        match self {
            Size::Bytes(bytes) => bytes,
            Size::Percent(share) => (total as f64 * share / 100.0) as u64,
        }
    }
}
