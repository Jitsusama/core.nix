//! Spend-watch's figures as a person reads them, written here so the
//! shell never formats one. Dollars are ready-rows', which the agents
//! daemon says its sums of spend-watch's figures with too, so the two
//! read alike; shares and tokens are spend-watch's alone.

pub use ready_rows::money::{dollars, grouped, maybe_about, whole};

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
