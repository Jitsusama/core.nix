//! What a proof's pairs of blocks say: each block's measures, and the
//! verdict on the shaper over all of them.

/// How far over the idle median a call's 99th percentile may go.
const CALL_MARGIN_MS: f64 = 30.0;
/// The least share of what the link sends that Joel's upload must get.
const SHARE: f64 = 0.5;
/// Of every `FAVOUR_OF` pairs, how many must favour the shaper: ten of
/// twelve is a sign test's one in fifty by chance.
const FAVOUR: f64 = 10.0;
const FAVOUR_OF: f64 = 12.0;

/// What one block measured.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Block {
    pub shaped: bool,
    /// Joel's upload and download, and all the link sent, in kbit/s.
    pub up_kbit: f64,
    pub down_kbit: f64,
    pub sent_kbit: f64,
    /// The call's round trips, in ms, and how many went unanswered.
    pub call_ms: Vec<f64>,
    pub lost: usize,
    /// A fresh connection's time to its first byte, in ms.
    pub first_byte_ms: Vec<f64>,
    /// Whether CAKE held a queue at any read: the shaper owned it.
    pub owned: bool,
    /// How many of Joel's requests the server refused, and how many it
    /// never answered at all.
    pub refused: usize,
    pub unanswered: usize,
}

impl Block {
    pub fn share(&self) -> f64 {
        if self.sent_kbit > 0.0 {
            self.up_kbit / self.sent_kbit
        } else {
            0.0
        }
    }

    /// Why the block says nothing of the shaper, if it doesn't.
    pub fn void(&self) -> Option<&'static str> {
        if self.shaped && !self.owned {
            Some("CAKE held no queue")
        } else if self.refused > 0 {
            Some("the server refused Joel's requests")
        } else {
            None
        }
    }
}

/// What curl said of a probe's requests, a line each of its status and a
/// measure: bytes sent or brought in, or seconds to the first byte.
#[derive(Debug, Default, PartialEq)]
pub struct Answers {
    /// The measures added up, and each answered one, in order.
    pub total: f64,
    pub each: Vec<f64>,
    pub refused: usize,
    pub unanswered: usize,
}

/// Reads curl's lines of `%{http_code} <measure>`. A status of 000 is a
/// request nothing answered, which measured nothing; one of 400 or more
/// the server refused, though what it carried still counts; and 100 is
/// an upload cut off at its time limit, still waiting to be answered.
pub fn answers(text: &str) -> Answers {
    let mut all = Answers::default();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(Ok(status)), Some(Ok(measure))) = (
            words.next().map(str::parse::<u16>),
            words.next().map(str::parse::<f64>),
        ) else {
            continue;
        };
        if status == 0 {
            all.unanswered += 1;
            continue;
        }
        if status >= 400 {
            all.refused += 1;
        }
        all.total += measure;
        all.each.push(measure);
    }
    all
}

#[derive(Debug, serde::Serialize)]
pub struct Verdict {
    pub idle_call_ms: f64,
    pub shaped_call_p99_ms: f64,
    pub shaped_share: f64,
    pub unshaped_share: f64,
    pub pairs: usize,
    pub favour_shaper: usize,
    pub void: usize,
    pub calm: bool,
    pub fair: bool,
    pub consistent: bool,
}

impl Verdict {
    pub fn passed(&self) -> bool {
        self.calm && self.fair && self.consistent
    }

    pub fn say(&self) {
        eprintln!(
            "the call: idle median {:.1} ms, shaped p99 {:.1} ms ({}); \
             Joel's upload: {:.0}% shaped, {:.0}% unshaped ({}); \
             {} of {} pairs favour the shaper ({}), {} void",
            self.idle_call_ms,
            self.shaped_call_p99_ms,
            if self.calm { "calm" } else { "not calm" },
            self.shaped_share * 100.0,
            self.unshaped_share * 100.0,
            if self.fair { "fair" } else { "not fair" },
            self.favour_shaper,
            self.pairs,
            if self.consistent {
                "consistent"
            } else {
                "not consistent"
            },
            self.void
        );
    }
}

/// The value at `p` (0 to 1) through some values, or `NaN` with none.
pub fn percentile(values: &[f64], p: f64) -> f64 {
    let mut all = values.to_vec();
    all.sort_by(f64::total_cmp);
    if all.is_empty() {
        return f64::NAN;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "an index into a few thousand values"
    )]
    let at = ((all.len() - 1) as f64 * p).round() as usize;
    all[at]
}

/// Judges the pairs: whether the shaper kept the call calm and Joel's
/// upload its share, and whether the pairs agree. A pair with a void
/// block counts for neither arm.
pub fn judge(idle_call: &[f64], pairs: &[[Block; 2]]) -> Verdict {
    let counted: Vec<&[Block; 2]> = pairs
        .iter()
        .filter(|pair| pair.iter().all(|b| b.void().is_none()))
        .collect();
    let arm = |shaped: bool| {
        counted
            .iter()
            .flat_map(|p| p.iter())
            .filter(move |b| b.shaped == shaped)
    };
    let calls: Vec<f64> = arm(true).flat_map(|b| b.call_ms.iter().copied()).collect();
    let shares = |shaped: bool| arm(shaped).map(Block::share).collect::<Vec<f64>>();
    let favour = counted
        .iter()
        .filter(|pair| {
            let (on, off) = if pair[0].shaped {
                (&pair[0], &pair[1])
            } else {
                (&pair[1], &pair[0])
            };
            on.share() > off.share()
        })
        .count();
    let idle = percentile(idle_call, 0.5);
    let p99 = percentile(&calls, 0.99);
    let shaped_share = percentile(&shares(true), 0.5);
    #[expect(clippy::cast_precision_loss, reason = "a few dozen pairs")]
    let consistent =
        !counted.is_empty() && favour as f64 >= counted.len() as f64 * FAVOUR / FAVOUR_OF;
    Verdict {
        idle_call_ms: idle,
        shaped_call_p99_ms: p99,
        shaped_share,
        unshaped_share: percentile(&shares(false), 0.5),
        pairs: counted.len(),
        favour_shaper: favour,
        void: pairs.len() - counted.len(),
        calm: p99 <= idle + CALL_MARGIN_MS,
        fair: shaped_share >= SHARE,
        consistent,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn block(shaped: bool, share: f64, call: f64) -> Block {
        Block {
            shaped,
            up_kbit: share * 10_000.0,
            sent_kbit: 10_000.0,
            call_ms: vec![call; 100],
            owned: true,
            ..Block::default()
        }
    }

    pub fn pairs(n: usize, on: f64, off: f64) -> Vec<[Block; 2]> {
        (0..n)
            .map(|_| [block(true, on, 25.0), block(false, off, 300.0)])
            .collect()
    }

    #[test]
    fn a_shaper_that_keeps_the_call_and_the_upload_passes() {
        let verdict = judge(&[20.0; 50], &pairs(12, 0.8, 0.05));
        assert!(verdict.passed(), "{verdict:?}");
    }

    #[test]
    fn a_call_far_over_its_idle_fails() {
        let mut all = pairs(12, 0.8, 0.05);
        for pair in &mut all {
            pair[0].call_ms = vec![80.0; 100];
        }
        let verdict = judge(&[20.0; 50], &all);
        assert!(!verdict.calm, "{verdict:?}");
    }

    #[test]
    fn an_upload_under_half_the_link_fails() {
        let verdict = judge(&[20.0; 50], &pairs(12, 0.4, 0.05));
        assert!(!verdict.fair, "{verdict:?}");
    }

    #[test]
    fn pairs_that_disagree_fail() {
        let mut all = pairs(12, 0.8, 0.05);
        for pair in all.iter_mut().take(3) {
            pair[1] = block(false, 0.9, 300.0);
        }
        let verdict = judge(&[20.0; 50], &all);
        assert_eq!(verdict.favour_shaper, 9, "{verdict:?}");
        assert!(!verdict.consistent, "nine of twelve counted as consistent");
    }

    #[test]
    fn a_block_whose_cake_held_no_queue_is_void() {
        let mut all = pairs(12, 0.8, 0.05);
        all[0][0].owned = false;
        let verdict = judge(&[20.0; 50], &all);
        assert_eq!((verdict.pairs, verdict.void), (11, 1), "{verdict:?}");
    }

    #[test]
    fn a_pair_whose_server_refused_joel_is_void_in_either_arm() {
        let mut all = pairs(12, 0.8, 0.05);
        all[0][0].refused = 1;
        all[5][1].refused = 3;
        let verdict = judge(&[20.0; 50], &all);
        assert_eq!((verdict.pairs, verdict.void), (10, 2), "{verdict:?}");
        assert_eq!(all[5][1].void(), Some("the server refused Joel's requests"));
    }

    #[test]
    fn requests_never_answered_still_count() {
        let mut all = pairs(12, 0.8, 0.05);
        all[0][0].unanswered = 4;
        let verdict = judge(&[20.0; 50], &all);
        assert_eq!(
            (verdict.pairs, verdict.void),
            (12, 0),
            "the shaper starving Joel's download would be hidden"
        );
    }

    #[test]
    fn curls_answers_are_added_up_and_refusals_and_silences_counted() {
        let read = answers("200 25000000\n200 25000000\n429 12\n000 0\n");
        assert!(
            (read.total - 50_000_012.0).abs() < 0.5,
            "added up to {}",
            read.total
        );
        assert_eq!((read.refused, read.unanswered), (1, 1));
        assert_eq!(read.each.len(), 3, "an unanswered request measured nothing");
        // An upload cut off at its time limit was answered "continue".
        let cut = answers("100 980000\n");
        assert_eq!((cut.total, cut.refused, cut.unanswered), (980_000.0, 0, 0));
        assert_eq!(answers(""), Answers::default());
    }
}
