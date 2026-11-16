//! Why the upload holds where it does, in one sentence from a fixed
//! vocabulary, for `status.json`'s `heard.why`, the launcher's network row
//! and `shaper status`. Each says the rate, what set it and how long it
//! has held so, so a night like 15 November, when the upload sat at a
//! third of the line for hours behind cuts for the VPN's far path, reads
//! as that from a glance rather than from a trace.

/// What last set the upload's rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Why {
    /// No queue has stood on this network yet, so nothing holds it.
    Unmeasured,
    /// No reflector has answered for a while, so nothing measures a queue.
    Unanswered,
    /// The guard sent it back to its base three times in a row, and left
    /// it unlimited a while.
    GaveUp,
    /// Nothing stands, and it holds or climbs where it may.
    Clear,
    /// The line's own delay stood this long, in ms, and it was cut; eased
    /// once the cut drained it.
    Line { delay_ms: f64, eased: bool },
    /// A queue only the VPN's flow waits in stood this long past the line,
    /// in ms, and it was cut.
    Vpn { delay_ms: f64 },
    /// The VPN's round trip stands this far past the line, in ms, where
    /// cuts didn't drain it, so it goes unheeded.
    Far { delay_ms: f64 },
    /// Held under what got through a router that queues flow by flow, in
    /// kbit/s, where only data waiting shows its queue.
    Router { plateau: f64 },
    /// Sent back to the anchor's hold, after a minute under the guard's
    /// share of it with no cut that drained a queue.
    Guarded,
}

impl Why {
    /// Whether this is the same kind of reason as `other`, whatever its
    /// numbers, so how long a reason has held isn't reset by its delay
    /// moving.
    pub fn same_kind(self, other: Why) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// Says `why` in one sentence: `limit` is what the upload is held to, in
/// kbit/s, `anchor` the network's declared rate, if it has one, and
/// `for_s` how long, in seconds, the reason has held.
pub fn say(why: Why, limit: Option<u64>, anchor: Option<f64>, for_s: f64) -> String {
    let rate = limit.map_or_else(|| "unlimited".to_owned(), |kbit| mbit(lossless(kbit)));
    let held = format!("{} {}", plural(for_s), "held");
    match why {
        Why::Unmeasured => format!("Upload {rate}: no queue has stood on this network yet."),
        Why::Unanswered => {
            format!("Upload {rate}: no reflector answers, so nothing measures a queue.")
        }
        Why::GaveUp => format!(
            "Upload {rate}: three returns in a row found no queue a cut drains, so it's left \
             alone a while."
        ),
        Why::Clear => match anchor {
            Some(anchor) => {
                format!(
                    "Upload {rate} of {}, the line's rate: clear for {}.",
                    mbit(anchor),
                    plural(for_s)
                )
            }
            None => format!("Upload {rate}: clear for {}.", plural(for_s)),
        },
        Why::Line { delay_ms, eased } => format!(
            "Upload {rate}: the line's delay stood {delay_ms:.0} ms, so it was cut{}; {held}.",
            if eased { ", and that eased it" } else { "" }
        ),
        Why::Vpn { delay_ms } => format!(
            "Upload {rate}: the VPN's flow waited {delay_ms:.0} ms past the line, so it was cut; \
             {held}."
        ),
        Why::Far { delay_ms } => format!(
            "Upload {rate}: the VPN reads {delay_ms:.0} ms past the line, which no cut here \
             reaches, so none is made."
        ),
        Why::Router { plateau } => format!(
            "Upload {rate}: held under the router's own queue, which let {} through; {held}.",
            mbit(plateau)
        ),
        Why::Guarded => format!(
            "Upload {rate}: back to the line's hold after a minute under {} with no cut that \
             eased a queue.",
            anchor.map_or_else(
                || "its share".to_owned(),
                |a| mbit(a * crate::oneway::ANCHOR_LOW)
            )
        ),
    }
}

fn lossless(kbit: u64) -> f64 {
    // A rate is a few hundred thousand kbit/s at most.
    #[expect(clippy::cast_precision_loss, reason = "a rate far under 2^52")]
    let kbit = kbit as f64;
    kbit
}

fn mbit(kbit: f64) -> String {
    format!("{:.1} Mbit/s", kbit / 1_000.0)
}

/// A span as a person says it: seconds under a minute, else minutes.
fn plural(s: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a span of a stay, positive and far under u64's range"
    )]
    let (secs, mins) = (s.max(0.0) as u64, (s.max(0.0) / 60.0) as u64);
    match (secs, mins) {
        (1, _) => "1 s".to_owned(),
        (secs, 0) => format!("{secs} s"),
        (_, mins) => format!("{mins} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clear_upload_says_its_rate_against_the_line_and_for_how_long() {
        assert_eq!(
            say(Why::Clear, Some(10_800), Some(12_000.0), 754.0),
            "Upload 10.8 Mbit/s of 12.0 Mbit/s, the line's rate: clear for 12 min."
        );
        assert_eq!(
            say(Why::Clear, Some(10_800), None, 42.0),
            "Upload 10.8 Mbit/s: clear for 42 s."
        );
    }

    #[test]
    fn a_far_path_says_no_cut_is_made() {
        assert_eq!(
            say(
                Why::Far { delay_ms: 120.4 },
                Some(10_800),
                Some(12_000.0),
                5.0
            ),
            "Upload 10.8 Mbit/s: the VPN reads 120 ms past the line, which no cut here \
             reaches, so none is made."
        );
    }

    #[test]
    fn a_cut_says_the_delay_that_set_it_and_whether_it_eased() {
        let line = |eased| {
            say(
                Why::Line {
                    delay_ms: 22.0,
                    eased,
                },
                Some(9_100),
                Some(12_000.0),
                1.0,
            )
        };
        assert_eq!(
            line(true),
            "Upload 9.1 Mbit/s: the line's delay stood 22 ms, so it was cut, and that eased it; \
             1 s held."
        );
        assert!(!line(false).contains("eased"));
    }

    #[test]
    fn the_guard_says_the_share_it_found_the_upload_under() {
        assert_eq!(
            say(Why::Guarded, Some(10_800), Some(12_000.0), 0.0),
            "Upload 10.8 Mbit/s: back to the line's hold after a minute under 9.6 Mbit/s with \
             no cut that eased a queue."
        );
    }

    #[test]
    fn an_unlimited_upload_says_so() {
        assert_eq!(
            say(Why::Unmeasured, None, None, 0.0),
            "Upload unlimited: no queue has stood on this network yet."
        );
    }

    #[test]
    fn a_reason_s_numbers_moving_keeps_its_kind() {
        assert!(Why::Far { delay_ms: 1.0 }.same_kind(Why::Far { delay_ms: 90.0 }));
        assert!(!Why::Far { delay_ms: 1.0 }.same_kind(Why::Vpn { delay_ms: 1.0 }));
    }
}
