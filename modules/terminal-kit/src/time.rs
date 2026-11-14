//! Times as a JSON answer says them: RFC 3339 with their offset
//! (docs/commands.md, The Answer), in UTC, whose offset is `Z`, so the
//! kit needs no time zone database and every command says a time alike.
//! The date comes from Howard Hinnant's days-to-civil arithmetic, which
//! holds for every day a 64-bit count of seconds reaches.

/// The time `seconds` after 1970 began, in UTC, as RFC 3339 says it:
/// `2026-11-13T07:52:00Z`.
pub fn rfc3339(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let into = seconds.rem_euclid(86_400);
    let (year, month, day) = civil(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        into / 3600,
        into % 3600 / 60,
        into % 60
    )
}

/// How long `seconds` is, for a person, in its two largest units, the
/// second left off where it's nought: `4 s`, `2 m 14 s`, `3 h`, `1 d 2 h`.
/// as durations read (docs/commands.md, Figures and Words). The kit holds
/// no time zone, so a moment is said to a person as how long ago it was,
/// which reads at a glance where a time in UTC wouldn't.
pub fn span(seconds: u64) -> String {
    const UNITS: [(u64, &str); 4] = [(86_400, "d"), (3_600, "h"), (60, "m"), (1, "s")];
    let first = UNITS
        .iter()
        .position(|(size, _)| seconds >= *size)
        .unwrap_or(UNITS.len() - 1);
    let (size, unit) = UNITS[first];
    let whole = format!("{} {unit}", seconds / size);
    match UNITS.get(first + 1) {
        Some((next, unit)) if seconds % size / next > 0 => {
            format!("{whole} {} {unit}", seconds % size / next)
        }
        _ => whole,
    }
}

/// The year, month and day `days` after 1970 began.
fn civil(days: i64) -> (i64, i64, i64) {
    // Counted from 1 March of year 0, so a leap day ends its year.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let from_march = (5 * of_year + 2) / 153;
    let day = of_year - (153 * from_march + 2) / 5 + 1;
    let month = if from_march < 10 {
        from_march + 3
    } else {
        from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_its_first_second() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_day_is_said_in_its_month() {
        assert_eq!(rfc3339(1_794_556_320), "2026-11-13T07:52:00Z");
    }

    #[test]
    fn a_leap_day_is_kept() {
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(951_868_800), "2000-03-01T00:00:00Z");
    }

    #[test]
    fn a_span_reads_in_its_two_largest_units() {
        assert_eq!(span(0), "0 s");
        assert_eq!(span(4), "4 s");
        assert_eq!(span(134), "2 m 14 s");
        assert_eq!(span(3 * 3600 + 12 * 60 + 9), "3 h 12 m");
        assert_eq!(span(26 * 3600), "1 d 2 h");
    }

    #[test]
    fn a_span_leaves_off_a_unit_of_nought() {
        assert_eq!(span(3 * 3600), "3 h");
        assert_eq!(span(120), "2 m");
    }

    #[test]
    fn a_time_before_1970_counts_back() {
        assert_eq!(rfc3339(-1), "1969-12-31T23:59:59Z");
    }
}
