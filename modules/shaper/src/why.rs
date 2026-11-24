//! Why the upload holds where it does, in a sentence or two from a fixed
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
    /// It was held on this network, and let go since: an idle spell left
    /// no line to hold to, the queue was another machine's, or it started
    /// over. Nothing holds it until a queue stands again.
    LetGo,
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
    /// The VPN waited this long, in ms, inside its own tunnel, under a
    /// line with room, so its own queue is held to `share` of its lane;
    /// `agents` is the share agents straight are still held to, climbing
    /// back from an earlier hold, while they wait, since it caps them too.
    Tunnel {
        delay_ms: f64,
        share: f64,
        agents: Option<f64>,
    },
    /// The VPN waited this long, in ms, past our queues while agents sent
    /// straight on the connection, so their lane is held to `share` of
    /// what it's split.
    Agents { delay_ms: f64, share: f64 },
    /// Held under what got through a router that queues flow by flow, in
    /// kbit/s, where only data waiting shows its queue.
    Router { plateau: f64 },
    /// Sent back to the anchor's hold, after a minute under the guard's
    /// share of it with no cut that drained a queue.
    Guarded,
    /// Held at a ceiling, a trial's, its network's or its house's, under
    /// what it would otherwise be given.
    Ceiling,
}

impl Why {
    /// Whether this is the same kind of reason as `other`, whatever its
    /// numbers, so how long a reason has held isn't reset by its delay
    /// moving.
    pub fn same_kind(self, other: Why) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// This kind of reason in a word, as a trace keeps it each second: the
    /// sentence `say` makes is too long for every line of a day.
    pub fn kind(self) -> &'static str {
        match self {
            Why::Unmeasured => "unmeasured",
            Why::LetGo => "let-go",
            Why::Unanswered => "unanswered",
            Why::GaveUp => "gave-up",
            Why::Clear => "clear",
            Why::Line { .. } => "line",
            Why::Vpn { .. } => "vpn",
            Why::Far { .. } => "far",
            Why::Tunnel { .. } => "tunnel",
            Why::Agents { .. } => "agents",
            Why::Router { .. } => "router",
            Why::Guarded => "guarded",
            Why::Ceiling => "ceiling",
        }
    }
}

/// Says `why` in a sentence, or two where agents' hold stands beside the
/// VPN's own; a share is rounded down, so a hold under one never reads as
/// 100%. `limit` is what the upload is held to, in
/// kbit/s, `anchor` the network's declared rate, if it has one, and
/// `for_s` how long, in seconds, the reason has held.
pub fn say(why: Why, limit: Option<u64>, anchor: Option<f64>, for_s: f64) -> String {
    let rate = limit.map_or_else(|| "unlimited".to_owned(), |kbit| mbit(lossless(kbit)));
    let held = format!("held so for {}", plural(for_s));
    match why {
        Why::Unmeasured => format!("Upload {rate}: no queue has stood on this network yet."),
        Why::LetGo => format!(
            "Upload {rate}: no queue has stood since its last hold let go, {} ago.",
            plural(for_s)
        ),
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
        Why::Tunnel {
            delay_ms,
            share,
            agents,
        } => format!(
            "Upload {rate}: the VPN waited {delay_ms:.0} ms inside its tunnel, so its own queue \
             is held to {:.0}% of its lane; {held}.{}",
            percent(share),
            // Agents' hold is said apart, with no time, since how long the
            // VPN's has held says nothing of theirs, nor whether theirs is
            // rising yet, which it does only once a judgement finds no
            // wait.
            agents.map_or_else(String::new, |agents| format!(
                " Agents' lane is still held from an earlier wait, to {:.0}% of its share, or \
                 its floor if more.",
                percent(agents)
            ))
        ),
        // The lanes never hold agents under their floor, which the
        // controller doesn't know, so the share is said as at most.
        Why::Agents { delay_ms, share } => format!(
            "Upload {rate}: the VPN waited {delay_ms:.0} ms past our queues beside agents' own \
             uploads, so their lane is held to {:.0}% of its share, or its floor if more; \
             {held}.",
            percent(share)
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
        Why::Ceiling => format!("Upload {rate}: held at its ceiling; {held}."),
    }
}

/// The controller's reason for the upload, as `say` gave it, and the
/// upload it speaks of, in kbit/s, as the shaper was answered: none when
/// the answer isn't the controller's own rate, as when it lets the upload
/// go, has given up, or hears no reflector and a trial's ceiling holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Reason {
    pub up: Option<u64>,
    pub said: String,
}

/// What the controller's judgement says of the rates the shaper is handed
/// at a moment.
#[derive(Clone, Debug, PartialEq)]
pub enum Judged {
    /// Its judgement holds them, for this reason.
    Holds(Reason),
    /// Nothing it judged holds them: the recorder has gone silent, or the
    /// controller lets the upload go, so a trial's ceiling holds what is
    /// held.
    Nothing,
    /// Its rate from before a silence holds them while it waits to step on
    /// a second heard since, so there is nothing new to say of them.
    Waiting,
}

impl Judged {
    /// The reason its judgement holds the rates, while it does.
    pub fn reason(&self) -> Option<&Reason> {
        match self {
            Judged::Holds(reason) => Some(reason),
            Judged::Nothing | Judged::Waiting => None,
        }
    }
}

/// The journal's line when the one-way controller begins to hold `link`:
/// what each way held is held to, in kbit/s, none for a way let go, and
/// the controller's reason where it speaks of the upload held. Another
/// upload's reason would give the line two rates: one the shaper clamped,
/// or a trial's ceiling holding what the controller let go, or none while
/// only the download is held.
pub fn holding(link: &str, up: Option<u64>, down: Option<u64>, reason: Option<&Reason>) -> String {
    let ways: Vec<String> = [(up, "up"), (down, "down")]
        .into_iter()
        .filter_map(|(rate, way)| rate.map(|r| format!("{} {way}", mbit(lossless(r)))))
        .collect();
    let held = format!("{link}: held to {}", ways.join(" and "));
    match reason.filter(|reason| reason.up.is_some() && reason.up == up) {
        Some(reason) => format!("{held}. {}", reason.said),
        None => format!("{held}."),
    }
}

/// The journal's line when a ceiling alone, a trial's, its network's or
/// its house's, holds `link`'s upload, at `up` kbit/s: the recorder has gone silent,
/// the controller lets the upload go or hasn't stepped yet, so nothing it
/// judged holds it.
pub fn ceiling(link: &str, up: u64) -> String {
    format!(
        "{link}: held to {} up by its ceiling alone, a trial's, its network's or its house's, \
         while the controller holds nothing.",
        mbit(lossless(up))
    )
}

/// A share in whole percent, rounded down, as the status and a trace say
/// it: a hold under one never reads as 100%. Rounded down from a billionth
/// over, since a double keeps a share such as 0.29 as 28.999... percent,
/// which would otherwise read a percent tighter than it was; a share that
/// billionth carries to 100 is still under one, so it reads 99.
pub fn percent(share: f64) -> f64 {
    let whole = (share * 100.0 + 1e-9).floor();
    if share < 1.0 { whole.min(99.0) } else { whole }
}

fn lossless(kbit: u64) -> f64 {
    // A rate is a few hundred thousand kbit/s at most.
    #[expect(clippy::cast_precision_loss, reason = "a rate far under 2^52")]
    let kbit = kbit as f64;
    kbit
}

fn mbit(kbit: f64) -> String {
    format!("{:.1} Mb/s", kbit / 1_000.0)
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
            "Upload 10.8 Mb/s of 12.0 Mb/s, the line's rate: clear for 12 min."
        );
        assert_eq!(
            say(Why::Clear, Some(10_800), None, 42.0),
            "Upload 10.8 Mb/s: clear for 42 s."
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
            "Upload 10.8 Mb/s: the VPN reads 120 ms past the line, which no cut here \
             reaches, so none is made."
        );
    }

    #[test]
    fn a_tunnel_s_own_queue_says_the_share_its_queue_is_held_to() {
        assert_eq!(
            say(
                Why::Tunnel {
                    delay_ms: 118.6,
                    share: 0.723,
                    agents: None,
                },
                Some(10_800),
                Some(12_000.0),
                8.0
            ),
            "Upload 10.8 Mb/s: the VPN waited 119 ms inside its tunnel, so its own queue is \
             held to 72% of its lane; held so for 8 s."
        );
        assert_eq!(
            say(
                Why::Tunnel {
                    delay_ms: 118.6,
                    share: 0.723,
                    agents: Some(0.814),
                },
                Some(10_800),
                Some(12_000.0),
                8.0
            ),
            "Upload 10.8 Mb/s: the VPN waited 119 ms inside its tunnel, so its own queue is \
             held to 72% of its lane; held so for 8 s. Agents' lane is still held from an \
             earlier wait, to 81% of its share, or its floor if more."
        );
        let nearly = say(
            Why::Tunnel {
                delay_ms: 118.6,
                share: 0.997,
                agents: Some(0.996),
            },
            Some(10_800),
            Some(12_000.0),
            8.0,
        );
        assert!(
            nearly.contains("to 99% of its lane") && nearly.contains("to 99% of its share"),
            "a share under one read as whole: {nearly}"
        );
    }

    #[test]
    fn agents_held_for_the_vpn_say_the_share_their_lane_is_held_to() {
        let nearly = say(
            Why::Agents {
                delay_ms: 80.0,
                share: 0.996,
            },
            Some(10_800),
            Some(12_000.0),
            8.0,
        );
        assert!(
            nearly.contains("to 99% of its share"),
            "agents' share under one read as whole: {nearly}"
        );
        assert_eq!(
            say(
                Why::Agents {
                    delay_ms: 96.2,
                    share: 0.614
                },
                Some(10_800),
                Some(12_000.0),
                14.0
            ),
            "Upload 10.8 Mb/s: the VPN waited 96 ms past our queues beside agents' own uploads, \
             so their lane is held to 61% of its share, or its floor if more; held so for 14 s."
        );
    }

    // A double keeps 0.29 as a hair under, and the tunnel's hold climbs from
    // 0.25 by 0.02 a second, so it lands on such shares; rounded down from
    // there they'd read a percent tighter than they were.
    #[test]
    fn a_share_reads_in_whole_percent_rounded_down_and_never_a_percent_low() {
        for (share, whole) in [
            (0.29, 29.0),
            (0.57, 57.0),
            (0.58, 58.0),
            (0.814, 81.0),
            (0.996, 99.0),
            (1.0 - 1e-12, 99.0),
            (1.0, 100.0),
        ] {
            let said = percent(share);
            assert!(
                (said - whole).abs() < 1e-12,
                "{share} read as {said}%, not {whole}%"
            );
        }
    }

    // A trace keeps each reason's word for every second of a day, which a
    // replay or a look back reads it by, so each word is fixed and is one
    // reason's alone.
    #[test]
    fn each_kind_of_reason_keeps_a_word_of_its_own() {
        let kinds = [
            (Why::Unmeasured, "unmeasured"),
            (Why::LetGo, "let-go"),
            (Why::Unanswered, "unanswered"),
            (Why::GaveUp, "gave-up"),
            (Why::Clear, "clear"),
            (
                Why::Line {
                    delay_ms: 1.0,
                    eased: false,
                },
                "line",
            ),
            (Why::Vpn { delay_ms: 1.0 }, "vpn"),
            (Why::Far { delay_ms: 1.0 }, "far"),
            (
                Why::Tunnel {
                    delay_ms: 1.0,
                    share: 0.5,
                    agents: None,
                },
                "tunnel",
            ),
            (
                Why::Agents {
                    delay_ms: 1.0,
                    share: 0.5,
                },
                "agents",
            ),
            (Why::Router { plateau: 1.0 }, "router"),
            (Why::Guarded, "guarded"),
            (Why::Ceiling, "ceiling"),
        ];
        for (why, word) in kinds {
            assert_eq!(why.kind(), word, "{why:?} took another word");
        }
        let words: std::collections::BTreeSet<_> =
            kinds.iter().map(|(why, _)| why.kind()).collect();
        assert_eq!(
            words.len(),
            kinds.len(),
            "two reasons share a word: {words:?}"
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
            "Upload 9.1 Mb/s: the line's delay stood 22 ms, so it was cut, and that eased it; \
             held so for 1 s."
        );
        assert!(
            !line(false).contains("eased"),
            "a cut that eased nothing said it did: {}",
            line(false)
        );
    }

    #[test]
    fn a_router_s_queue_says_what_it_let_through_in_the_rate_s_unit() {
        // The Work Network drawer says the rate in Mb/s and then this
        // reason, so a second unit inside it read as two vocabularies.
        assert_eq!(
            say(
                Why::Router { plateau: 9_600.0 },
                Some(7_420),
                Some(12_000.0),
                30.0
            ),
            "Upload 7.4 Mb/s: held under the router's own queue, which let 9.6 Mb/s through; \
             held so for 30 s."
        );
    }

    #[test]
    fn the_guard_says_the_share_it_found_the_upload_under() {
        assert_eq!(
            say(Why::Guarded, Some(10_800), Some(12_000.0), 0.0),
            "Upload 10.8 Mb/s: back to the line's hold after a minute under 9.6 Mb/s with \
             no cut that eased a queue."
        );
    }

    #[test]
    fn a_hold_beginning_says_the_controller_s_reason() {
        // Since the learned anchor, the controller holds at every start
        // with a clear line, and the journal said a hidden queue was
        // taken back each time (16 times on 16 November).
        let clear = Reason {
            up: Some(10_800),
            said: say(Why::Clear, Some(10_800), Some(12_000.0), 5.0),
        };
        assert_eq!(
            holding("wlp0s20f3", Some(10_800), None, Some(&clear)),
            "wlp0s20f3: held to 10.8 Mb/s up. Upload 10.8 Mb/s of 12.0 Mb/s, the line's \
             rate: clear for 5 s."
        );
    }

    // With no reason, what holds may be a trial's ceiling or the shaper's
    // own bounds, so the line names no cause at all.
    #[test]
    fn a_hold_with_no_reason_names_no_cause() {
        assert_eq!(
            holding("wlp0s20f3", Some(10_800), None, None),
            "wlp0s20f3: held to 10.8 Mb/s up."
        );
    }

    // A way let go goes at whatever the link carries: a rate beside it
    // would say it's held, and on Wi-Fi the rate is a 2 Gb/s placeholder.
    #[test]
    fn a_hold_names_only_the_ways_held() {
        assert_eq!(
            holding("wlp0s20f3", Some(10_800), Some(80_000), None),
            "wlp0s20f3: held to 10.8 Mb/s up and 80.0 Mb/s down."
        );
        assert_eq!(
            holding("wlp0s20f3", None, Some(80_000), None),
            "wlp0s20f3: held to 80.0 Mb/s down."
        );
    }

    // Under a silent recorder, or a controller letting go, nothing the
    // controller judged holds the upload, so what holds is a trial's
    // ceiling, and the line says so rather than naming no cause.
    #[test]
    fn a_ceiling_holding_alone_says_the_controller_holds_nothing() {
        assert_eq!(
            ceiling("wlp0s20f3", 8_500),
            "wlp0s20f3: held to 8.5 Mb/s up by its ceiling alone, a trial's, its network's or its \
             house's, while the controller holds nothing."
        );
    }

    // The shaper clamps what the controller asks to its own bounds, and the
    // reason says the controller's rate: said beside the clamped one, the
    // line gave the upload two rates.
    #[test]
    fn a_clamped_upload_is_not_given_the_controller_s_reason() {
        let cut = Reason {
            up: Some(11_500),
            said: say(
                Why::Line {
                    delay_ms: 22.0,
                    eased: false,
                },
                Some(11_500),
                Some(12_000.0),
                1.0,
            ),
        };
        assert_eq!(
            holding("wlp0s20f3", Some(10_800), None, Some(&cut)),
            "wlp0s20f3: held to 10.8 Mb/s up."
        );
    }

    // A trial's ceiling holds the upload the controller lets go, and the
    // controller's reason then says the upload is unlimited.
    #[test]
    fn a_ceiling_holding_a_let_go_upload_is_not_given_the_controller_s_reason() {
        let let_go = Reason {
            up: None,
            said: say(Why::Ceiling, None, Some(12_000.0), 5.0),
        };
        assert_eq!(
            holding("wlp0s20f3", Some(8_500), None, Some(&let_go)),
            "wlp0s20f3: held to 8.5 Mb/s up."
        );
    }

    // The reasons speak of the upload alone, so a spell the download opens
    // isn't explained by one.
    #[test]
    fn a_hold_of_the_download_alone_is_not_given_the_upload_s_reason() {
        let clear = Reason {
            up: None,
            said: say(Why::Clear, None, Some(12_000.0), 5.0),
        };
        assert_eq!(
            holding("wlp0s20f3", None, Some(80_000), Some(&clear)),
            "wlp0s20f3: held to 80.0 Mb/s down."
        );
    }

    #[test]
    fn an_unlimited_upload_says_so() {
        assert_eq!(
            say(Why::Unmeasured, None, None, 0.0),
            "Upload unlimited: no queue has stood on this network yet."
        );
        assert_eq!(
            say(Why::LetGo, None, None, 240.0),
            "Upload unlimited: no queue has stood since its last hold let go, 4 min ago.",
            "an upload let go after a hold read as one never held"
        );
    }

    #[test]
    fn a_reason_s_numbers_moving_keeps_its_kind() {
        assert!(
            Why::Far { delay_ms: 1.0 }.same_kind(Why::Far { delay_ms: 90.0 }),
            "a far delay that moved started its span again"
        );
        assert!(
            !Why::Far { delay_ms: 1.0 }.same_kind(Why::Vpn { delay_ms: 1.0 }),
            "a far delay and the VPN's queue read as one reason"
        );
    }
}
