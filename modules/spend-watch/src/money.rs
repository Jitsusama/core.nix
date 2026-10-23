//! Dollars as a person reads them, written here so the shell never formats
//! money: cents below ten dollars, whole dollars grouped by thousands above.

pub fn dollars(amount: f64) -> String {
    let sign = if amount < 0.0 { "-" } else { "" };
    let amount = amount.abs();
    if amount < 10.0 {
        format!("{sign}${amount:.2}")
    } else {
        format!("{sign}${}", grouped(rounded(amount)))
    }
}

/// Whole dollars, for a badge or a headline where cents are noise.
pub fn whole(amount: f64) -> String {
    let sign = if amount < 0.0 { "-" } else { "" };
    format!("{sign}${}", grouped(rounded(amount.abs())))
}

/// An amount of at least zero, rounded to whole dollars.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounding to whole dollars is the intent; callers pass amounts of at least \
              zero, and no bill nears u64's limit"
)]
fn rounded(amount: f64) -> u64 {
    amount.round() as u64
}

/// A figure that's estimated reads "about" first.
pub fn maybe_about(text: String, estimated: bool) -> String {
    if estimated {
        format!("about {text}")
    } else {
        text
    }
}

pub fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "rounding to a whole percent is the intent, and a share is far inside i64"
)]
pub fn percent(fraction: f64) -> String {
    format!("{}%", (fraction * 100.0).round() as i64)
}

/// Tokens, shortened: 1.2M, 340k, 900.
pub fn tokens(count: f64) -> String {
    if count >= 1e9 {
        format!("{:.1}B", count / 1e9)
    } else if count >= 1e6 {
        format!("{:.1}M", count / 1e6)
    } else if count >= 1e3 {
        format!("{:.0}k", count / 1e3)
    } else {
        format!("{count:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_amounts_keep_their_cents() {
        assert_eq!(dollars(4.2), "$4.20");
        assert_eq!(dollars(0.0), "$0.00");
    }

    #[test]
    fn large_amounts_are_whole_and_grouped() {
        assert_eq!(dollars(1403.6), "$1,404");
        assert_eq!(dollars(15000.0), "$15,000");
        assert_eq!(whole(214.4), "$214");
    }

    #[test]
    fn an_estimate_says_so() {
        assert_eq!(maybe_about(dollars(812.0), true), "about $812");
        assert_eq!(maybe_about(dollars(812.0), false), "$812");
    }
}
