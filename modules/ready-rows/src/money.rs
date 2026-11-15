//! Dollars as a person reads them, so neither the shell nor a second
//! daemon formats money: cents below ten dollars, whole dollars grouped
//! by thousands above, and "about" before a figure that's estimated.
//! Spend-watch says every amount it serves with it, and the agents daemon
//! a sum of spend-watch's figures, such as what a network's sessions
//! spent, so the two read alike.

/// An amount as a person reads it: "$4.20", "$1,404".
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
#[must_use]
pub fn maybe_about(text: String, estimated: bool) -> String {
    if estimated {
        format!("about {text}")
    } else {
        text
    }
}

/// A whole number grouped by thousands with commas: "15,000".
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_amounts_keep_their_cents() {
        assert_eq!(dollars(4.2), "$4.20", "under ten dollars, cents");
        assert_eq!(dollars(0.0), "$0.00", "nothing, in cents");
    }

    #[test]
    fn large_amounts_are_whole_and_grouped() {
        assert_eq!(dollars(1403.6), "$1,404", "rounded and grouped");
        assert_eq!(dollars(15000.0), "$15,000", "grouped by thousands");
        assert_eq!(whole(214.4), "$214", "whole dollars");
    }

    #[test]
    fn an_estimate_says_so() {
        assert_eq!(
            maybe_about(dollars(812.0), true),
            "about $812",
            "an estimate"
        );
        assert_eq!(
            maybe_about(dollars(812.0), false),
            "$812",
            "a billed figure"
        );
    }
}
