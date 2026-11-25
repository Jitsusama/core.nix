//! Which access point of its network a card should be on, from what each
//! carried from here (measure.rs), changed rarely and never during a call.
//!
//! The best access point changes with the hour and the room (Joel, 14 Nov
//! 2026: "fixating on one could hurt me"), so none is fixed: the one the
//! card is on is kept while it carries well, and another is tried only
//! when it carries badly two minutes running, or fails, or when one
//! measured lately carried far better. A card on a wider band slowed by
//! its own download keeps off a narrower band, which has less room to
//! bring the download down, unless one is known quicker under load and
//! the card isn't using the room. Its trial waits for the download to
//! end, unless it was made under load too. Figures are weighed like with
//! like: under load against what others carried under load, slowed by its
//! own download against what its own point usually carries too, and a
//! wider band from a narrower one carrying well against what the card's
//! own access point usually carries as well as its last two minutes, since
//! one slow minute isn't how a point carries.
//! A wider band, 5 or 6 GHz, is never left for 2.4 GHz while it loses
//! little and is heard well, however slow its tail or quick 2.4 GHz is
//! known to be, unless the card moves little there on slow air, a slow
//! middle or a busy channel, and is tried from 2.4 GHz while the card moves enough to
//! use its room, or when one heard well enough is known to answer no
//! slower: 2.4 GHz can be quicker to the gateway but carries far less.
//! wpa_supplicant still roams on its own when the signal fades; this
//! moves the card only between access points it can already hear, and
//! has it look for what it doesn't, as often as it moves: another of its
//! network when it carries badly, or one on a wider band from 2.4 GHz
//! while the card moves enough to use its room.
//!
//! The figures to beat are a desk's on 14 Nov 2026, one access point's
//! two radios at 4 dB apart: the gateway's p95 was 15 ms on one and
//! 119 ms on the other, with 2% and 31% of ten-second spells over 150 ms.
//!
//! Long because its cases, near two thirds of it, stay beside the private
//! rules they prove one by one, as every module in wifid keeps them.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

use crate::measure::{Measured, SPAN};

/// How long after a move the card stays put, so it never hunts.
pub const SETTLE: Duration = Duration::from_secs(600);

/// How long a move is on trial: worse by then, and the card goes back.
/// One carrying badly at two choices running, the later's span past the
/// minute it arrived in, is judged then, and goes back where it would at
/// its end, as one failing is on its first: on 23 Nov 2026 a trial whose
/// video echoes waited 0.75 to 2.8 s at p95, its best effort quick, held
/// the card for five minutes, as no more than 10% of its echoes went
/// unanswered or past a second until its last minute. The minute it
/// arrived in counts for none, since a span holds two: that night another
/// trial's video waited 0.8 s at p95 in its first minute alone.
pub const TRIAL: Duration = Duration::from_secs(300);

/// How long after a move a choice's span is past the minute the card
/// arrived in: a span and that minute, less half a minute, as choices fall
/// on the minute a moment early or late, and one at `SPAN` itself still
/// holds the arrival.
const ARRIVED: Duration = Duration::from_secs(150);

/// How long an access point found worse from here is left untried the
/// first time, and the start of each longer keep-off after (`SPURNED_MOST`).
pub const SPURNED: Duration = Duration::from_secs(3600);

/// The longest an access point found worse again and again is left
/// untried. Each keep-off of a point found worse within this of its last
/// keep-off ending is twice that one, from `SPURNED`, until a trial of it
/// stands: on the night of 23 Nov 2026 a card on its desk's 2.4 GHz radio
/// tried the same access point's ch52 radio at 20:25, 22:06, 23:34, 01:32
/// and 02:48, each read as untried at that hour as learning is by the
/// hour, and each carried 0.8 to 2.8 s at p95. Those 13 minutes took the
/// night's p95 from 32.7 ms to 54.2. Kept off for an hour, then two, then
/// four, at that night's tries it would have been tried three times
/// before 04:00, and tried again as each keep-off ended, four. Twelve
/// hours still tries such a point at another time of day, where it may
/// carry well.
pub const SPURNED_MOST: Duration = Duration::from_hours(12);

/// Past either, an access point carries badly enough to try another: a
/// p95 near three times the better radio's on 14 Nov, or a loss a call
/// hears.
const SLOW: Duration = Duration::from_millis(40);
const LOSSY: f64 = 2.0;

/// How quick the middle echo may stay, while the card moves at least
/// `LEAST` and loses no more than `LOSSY`, for a slow end to be the queue
/// of what the card brings down: echoes come back behind it in the access
/// point's queue for the card. On 17 Nov 2026 the card brought downloads
/// down at 26 to 55 Mbit/s in seven minutes past `SLOW`, on both bands of
/// two access points, with at most 0.8% lost: its p50 held at 4.6 to 12 ms
/// while its p95 rose to 43 to 154 ms. Twice it left 5 GHz for 2.4
/// mid-download, and what it moved fell from about 50 to 27 to 38 Mbit/s,
/// while once, from 2.4 GHz to 5, its p95 fell from 65 to 8 ms.
const STEADY: Duration = Duration::from_millis(12);

/// How quick the middle echo may stay for the slow end to be the card's
/// own queue while it moves `USED` or more, which fills the queue past the
/// middle too: at 22:12 on 18 Nov 2026 a download of 122 Mbit/s on ch52
/// read 59 ms at p95 with its p50 at 12.1 ms, the card tried its 2.4 GHz
/// radio for the air, and the download fell to 68 Mbit/s while the p95
/// rose to 89 ms. The middle tells little of the air: the card's 43
/// lightly loaded minutes past `SLOW` on 5 GHz from 14 to 19 Nov, losing
/// at most `LOSSY`, all read their p50 under 24 ms.
const HEAVY_STEADY: Duration = Duration::from_millis(24);

/// How long after a minute slowed by a download the next is still read as
/// its tail: what the card moved is the span's mean, so a download ending
/// within it reads under `LEAST` while its queued echoes still hold the
/// slow end past `SLOW`.
const TAIL: Duration = Duration::from_secs(90);

/// Past this loss the link is failing, and it's left even during a call,
/// but in the span just after a back, whose losses may be the way back's
/// (`holds_still`).
const FAILING: f64 = 10.0;

/// How much fainter than the one it's on another access point may be
/// heard and still be tried: about a quarter of the power. It holds for
/// one known to carry quickly too, since an echo's figure says nothing of
/// room: on 15 Nov 2026 the card moved twice from -47 dBm to another eero
/// heard at -65, and its link rate fell from about 900 Mbit/s to between
/// 245 and 432 as each carried worse than what it left.
const FAINTER: i16 = 6;

/// How busy an untried access point's channel may say it is and still be
/// tried. On 15 Nov 2026 every eero's ch149 radio averaged 42 to 46% busy,
/// and the one the card tried four times that afternoon carried badly in
/// about 20 of its 35 minutes; their ch52 radios averaged 15 to 21%, and
/// were past 40% in 1% of the eero's samples.
pub const BUSIEST: u8 = 40;

/// How much less busy a channel must say it is than the card's own for the
/// card to leave its own for room. The busy channels of 15 and 16 Nov 2026
/// ran 41 to 68% against 15 to 37% beside them, so 15 points keeps every
/// move seen then while two channels near `BUSIEST` stay put.
const QUIETER: u8 = 15;

/// From this frequency in MHz a radio is on 5 or 6 GHz, whose channels are
/// wider than 2.4 GHz's: on 15 Nov 2026 the card's link rate fell from
/// about 1200 to 258 Mbit/s moving to the same access point's 2.4 GHz
/// radio, below what the internet brings down.
const WIDE: u32 = 5000;

/// How faint a radio on a wider band may be heard and still be tried for
/// it: the same access point is heard up to about 9 dB fainter on 5 GHz
/// than on 2.4, so the margin of `FAINTER` alone would rarely let one be
/// tried.
const FAINTEST: i16 = -67;

/// How many echoes a span must hold for its share lost to say a wider band
/// holds well: nine tenths of `SPAN`'s one a second. An echo that couldn't
/// be sent isn't counted, so a span with few, as when the gateway can't be
/// reached to echo, reads few or none lost.
const ECHOED: usize = 108;

/// How much fainter than `FAINTER` a radio on a wider band may be heard
/// and still be tried for it: the same access point's 5 GHz radio was 2 to
/// 9 dB fainter than its 2.4 GHz one, hour by hour, on 15 and 16 Nov 2026.
/// At 04:54 on the 16th a card heard at -36 dBm on 2.4 GHz moved to
/// another access point's 5 GHz radio at -66, linked at 432 Mbit/s and
/// carried worse, while its own 5 GHz radio was heard at -45.
const ACROSS: i16 = 10;

/// How loud a radio on a wider band is heard that's within reach however
/// loud the card's own point: on 16 Nov 2026 a card heard its access
/// point's 2.4 GHz radio at -29 dBm, and kept off the same one's 5 GHz
/// radio, 17 to 19 dB fainter, through four looks over 43 minutes. Heard
/// at -45 at last, it moved and linked at 1080 Mbit/s. From 15 to 16 Nov
/// the card's 5 GHz links heard at -50 to -54 dBm ran at a mean 730
/// Mbit/s, none under 400, against about 287 on 2.4 GHz.
const STRONG: i16 = -50;

/// How much a card must move, both ways together in Mbit/s over the span,
/// for a wider band's room to count for more than how quickly it answers,
/// or half what its link sends at where that's less, since a faint link
/// can't carry this much however much is wanted of it. Over
/// the night of 16 to 17 Nov 2026 at a desk, the access point's 2.4 GHz
/// radio answered quicker than its 5 GHz one in four of five hours, and the
/// card went back to 5 GHz four times for room it never used: no ten
/// seconds passed 85 Mbit/s on 2.4 GHz, linked at 287, nor 181 on 5 GHz.
const USED: u32 = 100;

/// The least a card must move for a wider band's room to count, however
/// slowly its link says it sends, so a rate read low isn't taken for a
/// faint link; below it, a card on a wider band is resting there. Over the
/// same night, the card's two minutes passed 25 Mbit/s in 4% of spans, and
/// 10 Mbit/s in 85%.
const LEAST: u32 = 25;

/// How much quicker one measured lately must have been to be moved to:
/// both, so neither a small figure's ratio nor a large one's few
/// milliseconds moves the card.
const QUICKER_BY: f64 = 0.7;
const QUICKER: Duration = Duration::from_millis(10);

/// How much worse a trial may carry before the card goes back.
const WORSE: Duration = Duration::from_millis(10);
const WORSE_LOSS: f64 = 2.0;

/// How much of what a download moved before a trial made under load must
/// still move for the trial to stand. The 17 Nov 2026 moves to 2.4 GHz
/// left downloads moving 67 to 69% of what they had.
const MOVED_BY: f64 = 0.7;

/// What another access point of the network is known by.
#[derive(Clone, Debug, PartialEq)]
pub struct Other {
    pub point: String,
    /// How loud it's heard, in dBm.
    pub signal: i16,
    /// How busy it says its channel is, in percent, where it says.
    pub busy: Option<u8>,
    /// Its channel, by frequency in MHz; 0 where wpa_supplicant doesn't say.
    pub frequency: u32,
    /// What it carried while the card was last on it, within the span, or
    /// was learned to carry, as loaded as the card is now: under load
    /// where the card moves `LEAST` or more, else not. For the card's own
    /// access point it's what that usually carries, never the span.
    pub measured: Option<Measured>,
}

/// What the card should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Stay,
    /// Move to this access point, on trial.
    Try(String),
    /// Go back to the access point the trial left, as the trial carried
    /// worse than it did, or than it usually does.
    Back(String),
    /// Look for another access point of the network, none being heard.
    Look,
    /// Look for an access point of the network on a wider band, none being
    /// heard there within reach.
    LookWider,
    /// Look for an access point of the network on a quieter channel, the
    /// card's own saying it's past `BUSIEST` and none quieter being heard
    /// within reach.
    LookQuieter,
}

/// What's been chosen lately, to keep the card from hunting.
#[derive(Default)]
pub struct Chooser {
    moved: Option<Instant>,
    /// When it last had the card look for another access point, which it
    /// does no more often than it moves.
    looked: Option<Instant>,
    /// The access point the card carried badly on at the last choice. One
    /// minute's figure alone moves nothing: on 15 Nov 2026 one point read
    /// 45 ms at p95 for a minute between 11 and 18, and the one tried for
    /// it carried 32 to 64 for the next 14.
    badly_on: Option<String>,
    /// Where it carried badly the minute before the last choice, which a
    /// choice again within that minute goes by, so one bad minute isn't
    /// counted twice.
    badly_before: Option<String>,
    /// When the card's look ended, where it's chosen for again then.
    again: Option<Instant>,
    trial: Option<Trial>,
    /// Each access point kept off and until when.
    spurned: HashMap<String, Kept>,
    /// Each access point found worse lately, how many times running and
    /// until when it was last kept off for it, so it's kept off longer
    /// each time (`SPURNED_MOST`). Every back that keeps a point off counts:
    /// carrying worse than the point left, slower under load, badly for a
    /// wider band's room, or worse than the usual a second time, as each
    /// is a trial that didn't pay. Like `spurned`, it lives only as long as
    /// the card's chooser: a restart of wifid, or a minute's tick finding
    /// the card unjoined, as on a resume, starts every point afresh. Its
    /// times are awake time, as `Instant` doesn't count a suspend.
    struck: HashMap<String, Struck>,
    /// Each move, from one access point to another, that went back for
    /// carrying worse only than the usual, and when, so a second within
    /// `SPURNED` keeps the point tried off.
    usually: HashMap<(String, String), Instant>,
    /// Why the last choice sent the card back, for the journal to say once.
    went_back: Option<Against>,
    /// The access point the card was last sent back to, and when, so a
    /// failing link there waits out the span after.
    back_to: Option<(String, Instant)>,
    /// The access point a failing link was held on at the last choice, as
    /// the card was sent back there under a span ago, for the journal.
    held: Option<String>,
    /// The access point kept off for carrying worse only than the usual
    /// that the last choice tried anyway, the card's link failing a whole
    /// span, for the journal to say the keep-off gave way.
    yielded: Option<String>,
    /// The access point a trial standing as the usual freed, and the point
    /// it was tried from, for the journal to say the keep-off ended early.
    freed: Option<(String, String)>,
    /// The access point a move on trial never reached, kept off from the
    /// last choice, for the journal to say so.
    unreached: Option<String>,
    /// When a move judged last never stood. Another within `SPURNED` of it
    /// holds a whole settle, not a span: a stranger in radio range sending
    /// the network's name from point after point, none of which answers,
    /// could otherwise have the card leave its own every span, a second or
    /// more off the network each time. One an hour or more on is taken as
    /// a first again.
    missed: Option<Instant>,
    /// When a card on a wider band was last slowed by its own download.
    queued: Option<Instant>,
    /// The card's access point and the narrower one a wider band holding
    /// well kept it off at this choice, where it would once have tried it.
    kept_wide: Option<(String, String)>,
    /// The same from the choice before, which any choice that returns
    /// early leaves empty, so a hold after a call or a move is said anew.
    kept_lately: Option<(String, String)>,
    /// The same, from the choice that began keeping it off, for the
    /// journal to say once.
    began_keeping: Option<String>,
    /// The card's access point and when a choice first found it there, so
    /// a span younger than `SPAN` since isn't taken for one whose echoes
    /// went unsent: after a resume, a join or a roam wpa_supplicant made
    /// itself, the first choice or two read 0 to about 108 echoes.
    joined: Option<(String, Instant)>,
}

/// An access point kept off, until when, and until when a failing link
/// may not leave for it either.
struct Kept {
    until: Instant,
    /// One kept off for carrying worse only than the usual carried no
    /// worse than the point left did, so a failing link may leave for it
    /// at once. One that carried worse than the point left holds against a
    /// failing link for `SPURNED` at most, as it always did: past that,
    /// what it carried is old, and kept off longer (`SPURNED_MOST`) it
    /// could otherwise hold a card on a failing link, the one other point
    /// in reach, for half a day.
    firm: Instant,
    /// The access point the move kept off left, where a trial judged
    /// against what that point carried earned it, so only another trial
    /// from there frees it.
    from: Option<String>,
}

/// How the card's link carried at a choice, as a trial is judged by it.
#[derive(Clone, Copy, PartialEq)]
enum Carrying {
    /// It lost more than `FAILING`.
    Failing,
    /// It carried badly at this choice and the one before, this one's span
    /// past the minute the card arrived in.
    Badly,
    /// Neither.
    Enough,
}

/// How many times running an access point's trials went back, and until
/// when the last kept it off.
struct Struck {
    times: u32,
    until: Instant,
}

/// Why a trial went back.
#[derive(Clone, Debug, PartialEq)]
pub enum Against {
    /// It carried worse than the point left did, this.
    Worse(Measured),
    /// It carried worse only than what the point left usually carries,
    /// this, with the minute's loss.
    Usually(Measured),
    /// It brought a download down slower than the point left did, at this
    /// many Mbit/s.
    Slowed(u32),
    /// It was for a wider band's room and carried badly there.
    Badly,
}

struct Trial {
    from: String,
    to: String,
    before: Measured,
    /// What the point left usually carries, as loaded as the card was,
    /// where it's learned: a trial carrying worse than the quicker of this
    /// and `before`'s slow end goes back, where the move was off a slow
    /// minute rather than a lossy or quick one and the trial and this are
    /// as loaded as it. On 19 Nov 2026 at a desk the hall's radio
    /// was tried on ch52's one minute of 75.5 ms at p95, where it usually
    /// carried about 14, and kept 21 minutes at 45 to 57 ms, as it was
    /// only less bad than that minute.
    usual: Option<Measured>,
    /// When the card last moved before this one, which counts again where
    /// this one never stood.
    moved_before: Option<Instant>,
    /// Whether a choice has judged the card on the point it tried. One that
    /// stood there and is later found back where it began went back some
    /// other way, as when that point went or an account moved it, so its
    /// settle holds. A move gone before a choice judged it is taken as one
    /// that never stood: one that stood less than a minute, or only while
    /// choices passed it over, as through a call or with its own point
    /// unheard. That costs its settle cut to a span, its point still kept
    /// off.
    stood: bool,
    /// Whether the move was for a wider band's room, the card using it,
    /// kept unless it carries badly rather than unless it carries worse
    /// than the narrower one left. One made idle, the wider band known no
    /// slower, is judged as any other.
    banded: bool,
    /// Whether the move was from a narrower band to a wider one, which
    /// doesn't go back while the wider band holds well.
    widened: bool,
}

impl Chooser {
    /// Chooses, from what the access point the card is on carried over the
    /// span, the others it hears, and whether a call is on. The card's own
    /// point may be among the others, as it's heard, to say how busy its
    /// channel is.
    pub fn choose(
        &mut self,
        now: Instant,
        on: (&str, &Measured, i16, u32),
        others: &[Other],
        calling: bool,
    ) -> Choice {
        let (point, measured, signal, frequency) = on;
        self.kept_lately = self.kept_wide.take();
        let young = self.young(now, point);
        let failing = measured.lost > FAILING;
        let carried_badly = badly(measured);
        // A card on a wider band bringing a download down is slowed by the
        // access point's queue for it, which another radio on a wider band
        // may drain quicker, and a narrower band only by bringing it down
        // slower.
        let queued = wide(frequency) && slowed_by_its_own(measured);
        let queued_lately = queued || self.queued.is_some_and(|at| now.duration_since(at) <= TAIL);
        if queued {
            self.queued = Some(now);
        }
        // A failing link is left on its first minute, a merely bad one on
        // its second.
        let lately = if self.again.take() == Some(now) {
            self.badly_before.clone()
        } else {
            let lately =
                std::mem::replace(&mut self.badly_on, carried_badly.then(|| point.to_owned()));
            self.badly_before.clone_from(&lately);
            lately
        };
        let bad = carried_badly && (failing || lately.as_deref() == Some(point));
        if calling && !failing {
            return Choice::Stay;
        }
        // A wider band losing at most `LOSSY` and heard at `FAINTEST` or
        // louder is never left for a narrower one, by a trial's going back
        // or any rule below: on 19 Nov 2026 the card left ch52 for 2.4 GHz
        // twice, idle at 10 to 24 Mbit/s, on a tail of 34.3 and 51.6 ms
        // with none lost, for 2.4 GHz's 11.8 ms at that hour. Once a
        // download came, 2.4 GHz carried 183 and 101 ms, and a call begun
        // there would have kept it there. Its middle echo held at 4.3 to
        // 5.8 ms then. Moving less than `LEAST`, a wider band whose middle
        // is past `STEADY`, or whose own channel is past `BUSIEST`, is slow
        // for its air and may yield, as 2.4 GHz has room enough for it: on
        // 19 Nov 2026 from 21:00 the card on 5745 MHz moving 11 to 22
        // Mbit/s read a middle of 11 to 24 ms and a p95 up to 274.7 ms, its
        // channel 54% busy, and downloads stalled there ten times in 12
        // minutes. On its access point's 2.4 GHz radio from 21:20 it read 5
        // ms and 8 to 16 ms. Moving more, or moving it doesn't know, it
        // holds, since 2.4 GHz brought the 17 Nov downloads down to 54 to
        // 76% of what they had moved. How busy the channel is comes from
        // the last scan, which may be some minutes old (`crowding`). A span
        // younger than `SPAN` holds on what it has echoed so far.
        let slow_air = measured.p50.is_some_and(|p50| p50 > STEADY)
            || crowding((point, frequency), others).is_some();
        let holds_wide = wide(frequency)
            && (measured.echoes >= ECHOED || young)
            && measured.lost <= LOSSY
            && (!resting(measured) || !slow_air)
            && heard(measured, signal) >= i32::from(FAINTEST);
        let carrying = self.carrying(now, failing, bad);
        let judging = (carrying, queued_lately, holds_wide);
        if let Some(judged) = self.judge(now, (point, measured), judging) {
            return judged;
        }
        if self.holds_still(now, point, failing) {
            self.held = failing.then(|| point.to_owned());
            return Choice::Stay;
        }
        self.forget_kept_off(now);
        // A card on a wider band carrying well keeps off a narrower one,
        // however quick, since quickness isn't room to bring things down,
        // unless it's known to move less than `LEAST`: then it may take one
        // known far quicker. Overnight on 16 to 17 Nov 2026 at a desk, the
        // access point's 2.4 GHz radio answered quicker than its 5 GHz one
        // in four of five hours. Between `LEAST` and `used`'s bar the card
        // keeps to whichever band it's on, so use near the bar can't swap it.
        // Slowed by its own download, it may take a narrower radio known
        // quicker under load while it isn't using the wider band's room:
        // on 18 Nov 2026 at a desk the access point's 5 GHz radio carried
        // downloads of 25 to 65 Mbit/s at 76 to 136 ms, and twice the card
        // kept off its 2.4 GHz radio, which had carried 25 to 81 at a mean
        // 37.5 ms. A narrower band may bring the download down slower, so
        // its trial goes back where it does (`MOVED_BY`).
        // None of that leaves a wider band holding well (`holds_wide`).
        // A point kept off only for carrying worse than the usual twice
        // carried no worse than the point left did, so a failing link may
        // still leave for it, once a whole span says it's failing, as after
        // a back: one may come from a join or a resume as well. So may it
        // for one kept off past its first hour (`Kept::firm`).
        let resting = resting(measured);
        let may_narrow = queued && !used(measured);
        let untried = others.iter().filter(|other| {
            other.point != point
                && self.may_try(&other.point, failing && !young, now)
                && (wide(other.frequency) || !holds_wide)
                && ((bad && !queued)
                    || resting
                    || wide(other.frequency)
                    || !wide(frequency)
                    || (may_narrow && other.measured.is_some()))
        });
        // Carrying well on a narrower band, the card weighs a wider one
        // against what its own point usually carries too: on 18 Nov 2026 it
        // left a 2.4 GHz radio three times for the 5 GHz one's learned 14.5
        // to 27.7 ms on a minute of 21.1 to 35.7, where its own usually
        // carried about 10 to 12, and 5 GHz was slower each time. The
        // quicker rule goes by the span alone, so a point slow now is left
        // however well it usually carries, unless its own download's queue
        // slowed it, below.
        let usual = others
            .iter()
            .find(|other| other.point == point)
            .and_then(|other| other.measured.as_ref());
        let narrow = frequency != 0 && !wide(frequency);
        let banded = if narrow && !bad {
            let ours = quickest(measured, usual);
            for_the_band(untried.clone(), (signal, frequency), (measured, ours))
        } else {
            None
        };
        let crowded = crowding((point, frequency), others).filter(|_| !carried_badly);
        let quieter =
            crowded.and_then(|busy| for_quiet(untried.clone(), (signal, frequency, busy)));
        let reached = untried.filter(|other| within_reach(other, signal, frequency));
        let ours = weighed_by(measured, usual, queued_lately && measured.lost <= LOSSY);
        let faster = reached.clone().filter_map(|other| {
            let theirs = other.measured.as_ref()?;
            quicker(theirs, &ours).then_some((slowness(theirs), other))
        });
        let near = reached.filter(|other| {
            bad && other.measured.is_none() && other.busy.is_none_or(|busy| busy <= BUSIEST)
        });
        let chosen = banded.or_else(|| {
            faster
                .min_by_key(|(slowness, _)| *slowness)
                .map(|(_, other)| other)
                .or(quieter)
                .or_else(|| near.max_by_key(|other| (wide(other.frequency), other.signal)))
        });
        let once = (bad && !queued, resting || may_narrow);
        let kept = holds_wide && chosen.is_none();
        self.keep_wide(kept, (point, signal, frequency), others, once, &ours);
        let Some(chosen) = chosen else {
            return self.unchosen(now, on, others, (bad, holds_wide && !queued), crowded);
        };
        let banded = banded.is_some() && used(measured);
        let widened = !wide(frequency) && wide(chosen.frequency);
        let usual = trial_bar(usual, measured);
        self.try_point(
            now,
            (point, measured),
            &chosen.point,
            usual,
            (banded, widened),
        )
    }

    /// Moves the card on trial to `to` from what it carried on `from`,
    /// judged against `usual` where that's the bar: a point kept off is
    /// tried as yielding, and the hold of any back before it ends.
    fn try_point(
        &mut self,
        now: Instant,
        (from, before): (&str, &Measured),
        to: &str,
        usual: Option<Measured>,
        (banded, widened): (bool, bool),
    ) -> Choice {
        self.yielded = self.spurned.contains_key(to).then(|| to.to_owned());
        let moved_before = self.moved.replace(now);
        self.back_to = None;
        self.trial = Some(Trial {
            from: from.to_owned(),
            to: to.to_owned(),
            before: before.clone(),
            usual,
            moved_before,
            stood: false,
            banded,
            widened,
        });
        Choice::Try(to.to_owned())
    }

    /// Whether the card stays where it is whatever else it hears: a link
    /// not failing holds until the last move settles, and a card just sent
    /// back leaves a failing link only once a whole span since says it's
    /// failing, as echoes lost on the way back may fall in the first, and
    /// the point it left was just judged no better. Only that span is: a
    /// card come back to the point later, as by a roam, is judged as any.
    /// The span is counted from the back, as `young` is where the card is
    /// back on the point by the next choice, so the hold then ends as a
    /// keep-off that yields gives way. A roam back landing later starts
    /// `young` on landing, so the keep-off holds up to that much longer.
    fn holds_still(&self, now: Instant, point: &str, failing: bool) -> bool {
        if failing {
            self.back_to.as_ref().is_some_and(|(to, back)| {
                to == point && now.saturating_duration_since(*back) < SPAN
            })
        } else {
            !self.settled(now)
        }
    }

    /// Whether the card's span on its access point is younger than `SPAN`,
    /// counted from the first choice that found it there.
    fn young(&mut self, now: Instant, point: &str) -> bool {
        let since = match &self.joined {
            Some((on, since)) if on == point => *since,
            _ => self.joined.insert((point.to_owned(), now)).1,
        };
        now.duration_since(since) < SPAN
    }

    /// What a card none was chosen for does: looks for another access point
    /// where it hears none of its network it may take, a wider band where
    /// it uses its narrower one's room, or a quieter channel where its own
    /// is crowded. A wider band holding well may take no narrower one, so
    /// carrying badly, not for its own download (`held`), and hearing only
    /// those, it looks for another.
    fn unchosen(
        &mut self,
        now: Instant,
        (point, measured, signal, frequency): (&str, &Measured, i16, u32),
        others: &[Other],
        (bad, held): (bool, bool),
        crowded: Option<u8>,
    ) -> Choice {
        let narrow = frequency != 0 && !wide(frequency);
        let alone = bad
            && others
                .iter()
                .all(|other| other.point == point || (held && !wide(other.frequency)));
        let hemmed = hemmed_in(crowded, (signal, frequency), others);
        let unbanded = narrow
            && !bad
            && used(measured)
            && !others
                .iter()
                .any(|other| in_band_reach(other, signal, frequency));
        let looks = [
            (alone, Choice::Look),
            (unbanded, Choice::LookWider),
            (hemmed, Choice::LookQuieter),
        ];
        let look = looks
            .into_iter()
            .find_map(|(wanted, look)| wanted.then_some(look));
        self.look(now, look)
    }

    /// Takes in the narrower access point a wider band holding well, with
    /// nothing chosen (`kept`), keeps the card off, where the card would
    /// once have tried it: one known quicker on a bad minute not its own
    /// download's (`bad`) or while it rests or isn't using its room
    /// (`quick`), or one not yet measured on a channel not past `BUSIEST`
    /// on such a bad minute. It's said once a spell, so the journal shows
    /// the hold.
    fn keep_wide(
        &mut self,
        kept: bool,
        (point, signal, frequency): (&str, i16, u32),
        others: &[Other],
        (bad, quick): (bool, bool),
        ours: &Measured,
    ) {
        let would = |other: &&Other| {
            let known_quicker =
                (other.measured.as_ref()).is_some_and(|theirs| quicker(theirs, ours));
            let unknown = other.measured.is_none() && other.busy.is_none_or(|busy| busy <= BUSIEST);
            (known_quicker && (bad || quick)) || (unknown && bad)
        };
        // The one it would have tried: the quickest known, else the
        // loudest not yet measured.
        let narrower = others
            .iter()
            .filter(|other| {
                kept && other.point != point
                    && !wide(other.frequency)
                    && !self.spurned.contains_key(&other.point)
                    && within_reach(other, signal, frequency)
            })
            .filter(would)
            .min_by_key(|other| {
                let known = other.measured.as_ref();
                (
                    known.is_none(),
                    known.map(slowness),
                    std::cmp::Reverse(other.signal),
                )
            });
        let kept = narrower.map(|other| (point.to_owned(), other.point.clone()));
        if kept.is_some() && kept != self.kept_lately {
            self.began_keeping = kept.as_ref().map(|(_, narrower)| narrower.clone());
        }
        self.kept_wide = kept;
    }

    /// Takes in that the card's look ended now, so the choice made now is
    /// one again within the minute rather than the next minute's.
    pub fn looked_over(&mut self, now: Instant) {
        self.again = Some(now);
    }

    /// Whether the card's last move is a settle behind it, or it never
    /// moved.
    fn settled(&self, now: Instant) -> bool {
        self.moved
            .is_none_or(|moved| now.duration_since(moved) >= SETTLE)
    }

    /// The access point the last choice kept off as a move never reached,
    /// if it did, said once.
    pub fn unreached(&mut self) -> Option<String> {
        self.unreached.take()
    }

    /// Why the last choice sent the card back, if it did, said once.
    pub fn went_back(&mut self) -> Option<Against> {
        self.went_back.take()
    }

    /// The access point the last choice held a failing link on, as the
    /// card was just sent back there, if it did, said once.
    pub fn held(&mut self) -> Option<String> {
        self.held.take()
    }

    /// The access point kept off that the last choice tried anyway, as
    /// the card's link failed a whole span, if it did, said once.
    pub fn yielded(&mut self) -> Option<String> {
        self.yielded.take()
    }

    /// The access point a trial standing as the usual freed of its
    /// keep-off, and the point it was tried from, if one did, said once.
    pub fn freed(&mut self) -> Option<(String, String)> {
        self.freed.take()
    }

    /// Forgets each access point kept off whose keep-off is over, and
    /// each found worse whose last keep-off ended `SPURNED_MOST` ago.
    fn forget_kept_off(&mut self, now: Instant) {
        self.spurned.retain(|_, kept| now < kept.until);
        self.struck
            .retain(|_, struck| now < struck.until + SPURNED_MOST);
    }

    /// How long an access point a trial went back from is kept off from
    /// now: `SPURNED` the first time, and twice the last keep-off each time
    /// it's found worse again within `SPURNED_MOST` of that one ending, up
    /// to `SPURNED_MOST`. A trial of it standing carrying well forgets it
    /// (`stood`).
    fn spurn_for(&mut self, point: &str, now: Instant) -> Duration {
        let times = self
            .struck
            .get(point)
            .filter(|struck| now < struck.until + SPURNED_MOST)
            .map_or(1, |struck| struck.times.saturating_add(1));
        let doubled = SPURNED.saturating_mul(2_u32.saturating_pow(times - 1));
        let lasts = doubled.min(SPURNED_MOST);
        let until = now + lasts;
        self.struck
            .insert(point.to_owned(), Struck { times, until });
        lasts
    }

    /// Keeps off an access point a trial from `from` went back from, for as
    /// long as `spurn_for` says, yielding to a failing link or not.
    /// A keep-off yielding holds against a failing link not at all, and
    /// one not for its first `SPURNED` alone (`Kept::firm`).
    fn spurn(&mut self, point: &str, from: &str, yields: bool, now: Instant) {
        let lasts = self.spurn_for(point, now);
        let kept = Kept {
            until: now + lasts,
            firm: if yields {
                now
            } else {
                now + lasts.min(SPURNED)
            },
            from: Some(from.to_owned()),
        };
        self.spurned.insert(point.to_owned(), kept);
    }

    /// How long from now the card keeps off this access point, if it does.
    pub fn kept_for(&self, point: &str, now: Instant) -> Option<Duration> {
        self.spurned
            .get(point)
            .filter(|kept| now < kept.until)
            .map(|kept| kept.until.duration_since(now))
    }

    /// Whether the card may try this access point, kept off or not, its
    /// own link failing or not.
    fn may_try(&self, point: &str, failing: bool, now: Instant) -> bool {
        self.spurned
            .get(point)
            .is_none_or(|kept| failing && now >= kept.firm)
    }

    /// Forgets how often an access point was found worse once a trial of
    /// it stands carrying well: as the point left usually does, or not
    /// badly at all. One that stands only for carrying better than a
    /// failing, lossy or slow link says nothing of it, as 390 ms beating
    /// 400 ms isn't a point come good.
    fn stood(&mut self, point: &str, measured: &Measured, as_usual: bool) {
        if as_usual || !badly(measured) {
            self.struck.remove(point);
        }
    }

    /// Keeps off an access point a move never reached for `SPURNED`, firm
    /// against a failing link, or as long as it was already kept off if
    /// longer, so a miss through a keep-off that gave way doesn't shorten
    /// it.
    fn keep_off_unreached(&mut self, point: &str, now: Instant) {
        let until = now + SPURNED;
        let kept = self.spurned.entry(point.to_owned()).or_insert(Kept {
            until,
            firm: until,
            from: None,
        });
        kept.until = kept.until.max(until);
        kept.firm = kept.firm.max(until);
    }

    /// Whether the card keeps off this access point now, which the tests
    /// ask where the journal asks for how long.
    #[cfg(test)]
    pub fn keeps_off(&self, point: &str, now: Instant) -> bool {
        self.kept_for(point, now).is_some()
    }

    /// The narrower access point the last choice began keeping a wider band
    /// holding well off, if it did, said once.
    pub fn began_keeping_wide(&mut self) -> Option<String> {
        self.began_keeping.take()
    }

    /// How a link failing, or carrying badly two choices running (`bad`),
    /// carried for a trial's judging: badly only once the span is past the
    /// minute the card arrived in, so one bad arrival isn't read twice.
    fn carrying(&self, now: Instant, failing: bool, bad: bool) -> Carrying {
        let arrived = self
            .moved
            .is_none_or(|moved| now.duration_since(moved) >= ARRIVED);
        match (failing, bad && arrived) {
            (true, _) => Carrying::Failing,
            (false, true) => Carrying::Badly,
            (false, false) => Carrying::Enough,
        }
    }

    /// Judges the move on trial, if any, from what the card's point carried:
    /// `Stay` while it's on trial, `Back` where it carried worse than the
    /// point it left did or usually does, whichever is quicker, or nothing
    /// once it's over and choosing goes on. A trial failing, or carrying
    /// badly at two choices running past its first minute, is judged before
    /// it's over: one failing
    /// that doesn't go back ends its trial, and one carrying badly that
    /// doesn't stays on trial. A card not on the point it tried
    /// never reached it, and that point is kept off as one carrying worse is.
    /// A trial over while the card brings a download down waits for it to
    /// end, as its figures aren't the ones the move was made on, until a
    /// settle past the trial, when it ends unjudged and choosing goes on.
    /// One made under load and over under load is judged at once, like
    /// with like, and goes back where it brings the download down slower
    /// too; one made for a wider band's room is left to wait, since its
    /// download's queue there says nothing of the room. One that widened
    /// the card's band stands while the wider band holds well, unless it
    /// brings a download down slower under load.
    fn judge(
        &mut self,
        now: Instant,
        on: (&str, &Measured),
        (carrying, queued, holds_wide): (Carrying, bool, bool),
    ) -> Option<Choice> {
        let (point, measured) = on;
        let mut trial = self.trial.take()?;
        // What the card's on now isn't what was tried, as when the move
        // found it gone (NotNearby on 15 and 16 Nov 2026), so its figures
        // judge nothing: the tried point is kept off rather than tried again
        // each settle, and the card isn't sent back to where it may be.
        // Back on the point it left, never having stood on the one tried,
        // the move never got the card off it: it was sent back as the move
        // didn't get there in time (arrive.rs), refused, found the point
        // gone or kept the card in place. So the move doesn't hold the card
        // from choosing again for a settle, only for a `SPAN`: the next
        // choices then judge a span without the failed move's own outage,
        // which would read as loss wifid caused, and a card hearing stale
        // points on a link that isn't failing tries one a span on, each
        // kept off once missed, and the next a settle on if that misses
        // within an hour (`missed`). A failing link holds only after a
        // back (`holds_still`), so it may try another at each choice.
        if point != trial.to {
            if point == trial.from && !trial.stood {
                let lately = self
                    .missed
                    .is_some_and(|missed| now.saturating_duration_since(missed) < SPURNED);
                if !lately {
                    let spanned = now.checked_sub(SETTLE.saturating_sub(SPAN));
                    self.moved = unmade(now, trial.moved_before, spanned);
                }
                self.missed = Some(now);
            }
            self.keep_off_unreached(&trial.to, now);
            self.unreached = Some(trial.to);
            return None;
        }
        trial.stood = true;
        let over = self
            .moved
            .is_none_or(|moved| now.duration_since(moved) >= TRIAL);
        if !over && carrying == Carrying::Enough {
            self.trial = Some(trial);
            return Some(Choice::Stay);
        }
        let under_load = !trial.banded && loaded(&trial.before) && loaded(measured);
        if queued && carrying != Carrying::Failing && !under_load {
            let waited = self
                .moved
                .is_some_and(|moved| now.duration_since(moved) >= TRIAL + SETTLE);
            if waited {
                return None;
            }
            self.trial = Some(trial);
            return Some(Choice::Stay);
        }
        let slowed = under_load && slower(measured, &trial.before);
        // What the point left usually carries judges only a trial made off
        // a slow minute that echoed, rather than a lossy, a quick or a
        // silent one, as loaded as the trial: a point that loses is going
        // wrong, one usually quick says nothing of it now, a move off a
        // quick minute, to a quieter channel or at rest, was never made for
        // the point's slowness, and a minute with no echo couldn't reach
        // its gateway at all, and one echoing less than `ECHOED` gives a
        // tail from too few to weigh. The usual was asked for as loaded as the card
        // was at the move, which a learned figure doesn't say itself: under
        // load what the point carried loaded, and nothing where that isn't
        // learned. Its loss stays the minute's, as the quicker rule's does.
        let like = loaded(&trial.before) == loaded(measured)
            && trial.before.echoes >= ECHOED
            && slowness(&trial.before) > SLOW
            && trial.before.lost <= LOSSY;
        let against = weighed_by(&trial.before, trial.usual.as_ref(), like);
        let cut = Against::Slowed(trial.before.moving.unwrap_or(0));
        let why = if trial.widened && holds_wide {
            slowed.then_some(cut)
        } else if trial.banded {
            badly(measured).then_some(Against::Badly)
        } else if worse(measured, &trial.before) {
            Some(Against::Worse(trial.before.clone()))
        } else if slowed {
            Some(cut)
        } else {
            worse(measured, &against).then_some(Against::Usually(against))
        };
        // Judged early for carrying badly, a trial with nothing to go back
        // for waits out its trial as before, lest a point that carried no
        // better stand on two minutes. A banded one goes back for carrying
        // badly alone, as it would at its end.
        if !over && carrying != Carrying::Failing && why.is_none() {
            self.trial = Some(trial);
            return Some(Choice::Stay);
        }
        let moved = (trial.from.clone(), point.to_owned());
        // A trial that stands carrying as well as the point left usually
        // does, as loaded as that was learned, losing no more than `LOSSY`
        // over echoes enough to say so (`ECHOED`), carried well, so a back
        // on the usual before it is no longer the first of a repeat, and a
        // keep-off the same move earned, which a failing link took it
        // through, ends. One that stands only for carrying better than a
        // failing link, or with no usual to weigh, says nothing of the
        // usual, so both hold. Another point's keep-off was earned against
        // another bar, so holds too.
        let Some(why) = why else {
            let as_usual = trial.usual.as_ref().is_some_and(|usual| {
                let usual = weighed_by(&trial.before, Some(usual), true);
                loaded(&trial.before) == loaded(measured)
                    && measured.echoes >= ECHOED
                    && measured.lost <= LOSSY
                    && !worse(measured, &usual)
            });
            // One run out is no longer kept, so isn't said freed.
            let earned_here =
                |kept: &Kept| kept.from.as_deref() == Some(trial.from.as_str()) && now < kept.until;
            self.stood(point, measured, as_usual);
            if as_usual {
                self.usually.remove(&moved);
                if self.spurned.get(point).is_some_and(earned_here) {
                    self.spurned.remove(point);
                    self.freed = Some((point.to_owned(), trial.from.clone()));
                }
            }
            return None;
        };
        // A trial worse only than what the point left usually carries isn't
        // kept off the first time: the settle every move takes already keeps
        // the card from trying it again at once, and a keep-off past that
        // would hold the card off its way out should the point gone back to
        // fail. A second of the same move within `SPURNED` is: learning
        // doesn't end the repeats, since a usual is blended over neighbouring
        // steps, and under load over every hour, so the slow minutes behind
        // a back move it only in part. Another point's usual is another
        // bar, so counts apart.
        // That keep-off yields to a failing link, as the point tried carried
        // no worse than the one left, so the card isn't held on a point
        // gone bad.
        self.usually
            .retain(|_, at| now.saturating_duration_since(*at) < SPURNED);
        let usual = matches!(why, Against::Usually(_));
        let again = usual && self.usually.insert(moved, now).is_some();
        if again || !usual {
            self.spurn(point, &trial.from, again, now);
        }
        self.moved = Some(now);
        self.went_back = Some(why);
        self.back_to = Some((trial.from.clone(), now));
        // The card's span on the point gone back to starts now, as it
        // roams there at once, so a keep-off that yields gives way as the
        // hold after the back ends.
        self.joined = Some((trial.from.clone(), now));
        Some(Choice::Back(trial.from))
    }

    /// Has a card look for what it doesn't hear, once a settle at most, so
    /// the next choice has it to choose from: another access point of its
    /// network, carrying badly, one on a wider band, on 2.4 GHz using its
    /// room, or one on a quieter channel, on a busy one. A
    /// heard access point is forgotten 180 s after the last look, so on 15
    /// Nov 2026 a card that tried 2.4 GHz and carried well soon heard no
    /// 5 GHz point to go back to, and kept to a third of the link rate.
    fn look(&mut self, now: Instant, look: Option<Choice>) -> Choice {
        let rested = self
            .looked
            .is_none_or(|looked| now.duration_since(looked) >= SETTLE);
        match look {
            Some(look) if rested => {
                self.looked = Some(now);
                look
            }
            _ => Choice::Stay,
        }
    }
}

/// Where a card on a wider band, with no bad minute, goes off a channel
/// busier than `BUSIEST`: the loudest untried on a wider band within reach
/// that says it's no busier than that, and `QUIETER` less busy than its own,
/// so two channels near the bar don't swap. A point already measured from
/// here is left to the quicker rule, which takes it only if it carried
/// quicker. From 10:23 to 12:14 on 16 Nov 2026 a card sat on a channel 50
/// to 58% busy at 33 to 65 ms p95, mostly under `SLOW`, while the same
/// access point's channel 20% busy, heard as loud, carried 10 to 14 ms.
fn for_quiet<'a>(
    untried: impl Iterator<Item = &'a Other>,
    (signal, frequency, busy): (i16, u32, u8),
) -> Option<&'a Other> {
    untried
        .filter(|other| other.measured.is_none() && roomier(other, (signal, frequency, busy)))
        .max_by_key(|other| other.signal)
}

/// Whether a card on a channel this busy hears nowhere quieter to go, so a
/// look could find one. One heard but known no quicker, or spurned, is
/// left as it is, since looking again won't make it roomier.
fn hemmed_in(crowded: Option<u8>, (signal, frequency): (i16, u32), others: &[Other]) -> bool {
    crowded.is_some_and(|busy| {
        !others
            .iter()
            .any(|other| roomier(other, (signal, frequency, busy)))
    })
}

/// Whether another access point is heard within reach on a wider band,
/// saying its channel is quieter than the card's own, this busy.
fn roomier(other: &Other, (signal, frequency, busy): (i16, u32, u8)) -> bool {
    wide(other.frequency)
        && other.busy.is_some_and(|theirs| quieter(theirs, busy))
        && within_reach(other, signal, frequency)
}

/// Whether a channel this busy is quiet enough to leave one that busy for:
/// no busier than `BUSIEST`, and `QUIETER` less busy.
pub fn quieter(theirs: u8, busy: u8) -> bool {
    theirs <= BUSIEST && theirs + QUIETER <= busy
}

/// How busy the card's own channel says it is, where it's on a wider band
/// and past `BUSIEST`. Its own access point is among the others as it was
/// last heard, which may be some minutes old: on 16 Nov 2026 the card's
/// own was last heard 997 s before, so a look refreshes it too.
fn crowding((point, frequency): (&str, u32), others: &[Other]) -> Option<u8> {
    let itself = others.iter().find(|other| other.point == point)?;
    let busy = itself.busy?;
    (wide(frequency) && busy > BUSIEST).then_some(busy)
}

/// Where a card on a narrower band carrying well, as `ours` says, goes for
/// a wider one: the quickest within reach known to carry well from here,
/// else the loudest untried within reach and not too busy. While the card
/// moves less than `used` asks, the wider band's room goes unused, so only
/// one known to answer no slower is gone to, as heard by wpa_supplicant's
/// own look each hour or the shell's while Wi-Fi shows; it's then judged
/// as any trial, so it goes back if it carries worse. One known wins over a
/// louder one untried, since what it carried was learnt at this hour heard
/// about this loud, and a banded trial goes back if it carries badly.
/// Using the room, the card is handed what each carried under load, which
/// is past `SLOW` as often as not, so one known no slower than its own
/// under load is gone to too, as one untried would be.
fn for_the_band<'a>(
    untried: impl Iterator<Item = &'a Other> + Clone,
    (signal, frequency): (i16, u32),
    (measured, ours): (&Measured, &Measured),
) -> Option<&'a Other> {
    let using = used(measured);
    let on_it = untried.filter(|other| in_band_reach(other, signal, frequency));
    let known = on_it.clone().filter_map(|other| {
        let theirs = other.measured.as_ref()?;
        let no_slower = slowness(theirs) <= slowness(ours);
        let worth = if using {
            no_slower || !badly(theirs)
        } else {
            no_slower && !badly(theirs)
        };
        worth.then_some((slowness(theirs), other))
    });
    let heard = on_it.filter(|other| {
        using && other.measured.is_none() && other.busy.is_none_or(|busy| busy <= BUSIEST)
    });
    known
        .min_by_key(|(slowness, _)| *slowness)
        .map(|(_, other)| other)
        .or_else(|| heard.max_by_key(|other| other.signal))
}

/// Whether another access point is on a wider band and heard loud enough,
/// from one heard this loud on this channel, to be tried for the band:
/// within reach and no fainter than `FAINTEST`, however faint the card's
/// own point, since a band heard that faintly carries no more.
fn in_band_reach(other: &Other, signal: i16, frequency: u32) -> bool {
    wide(other.frequency) && other.signal >= FAINTEST && within_reach(other, signal, frequency)
}

/// Whether another access point is heard loud enough, from one heard this
/// loud on this channel, to be tried: within `FAINTER`, or on a wider band,
/// where the same access point is heard fainter, within `ACROSS` more and
/// no fainter than `FAINTEST`, or at `STRONG` or louder.
fn within_reach(other: &Other, signal: i16, frequency: u32) -> bool {
    other.signal >= signal - FAINTER
        || (wider(other.frequency, frequency)
            && other.signal >= (signal - FAINTER - ACROSS).clamp(FAINTEST, STRONG))
}

/// Whether a channel is on 5 or 6 GHz.
fn wide(frequency: u32) -> bool {
    frequency >= WIDE
}

/// Whether a channel is on a wider band than another's, where both are
/// known.
pub fn wider(to: u32, from: u32) -> bool {
    from != 0 && wide(to) && !wide(from)
}

/// What the card's own point usually carries, as the bar a trial off this
/// minute must beat. Under load, a point slow idle is known by that idle
/// floor where it never learned more, which weighs the move but is no bar
/// for a loaded trial to beat: carrying worse than no download could would
/// send it back. A figure learned slow under load can't be told from one,
/// so goes too, leaving its trial judged by the minute it left alone. The
/// bar reads only a usual's slow end, so its loss, blended over every
/// minute, never takes one away.
fn trial_bar(usual: Option<&Measured>, measured: &Measured) -> Option<Measured> {
    usual
        .filter(|usual| !loaded(measured) || !slow(usual))
        .cloned()
}

/// Whether an access point carries badly enough to try another.
pub fn badly(measured: &Measured) -> bool {
    slow(measured) || measured.lost > LOSSY
}

/// Whether an access point's slow end is past `SLOW`.
pub fn slow(measured: &Measured) -> bool {
    slowness(measured) > SLOW
}

/// Whether what a card brings down made its slow end slow, rather than
/// the air alone: its echoes waited behind it while the middle one held
/// and nothing much was lost.
pub fn slowed_by_its_own(measured: &Measured) -> bool {
    let steady = if measured.moving.is_some_and(|moving| moving >= USED) {
        HEAVY_STEADY
    } else {
        STEADY
    };
    slowness(measured) > SLOW
        && measured.lost <= LOSSY
        && measured.moving.is_some_and(|moving| moving >= LEAST)
        && measured.p50.is_some_and(|p50| p50 <= steady)
}

/// When a move never made is taken to have been made, so it holds the
/// card from choosing again only a `SPAN` (`spanned`, a settle less a span
/// before now), or till the settle of the move before it, if later. On
/// Linux an instant before boot is still one, so a boot's first minutes
/// hold a span too; a clock that can't reach back that far, giving none,
/// has the move hold a whole settle from now rather than none.
fn unmade(now: Instant, before: Option<Instant>, spanned: Option<Instant>) -> Option<Instant> {
    before.max(Some(spanned.unwrap_or(now)))
}

/// Whether a card moves enough for a wider band's room to count, which a
/// card whose kernel doesn't say what it moved isn't known to.
fn used(measured: &Measured) -> bool {
    let half = measured.room.map(|room| room / 2);
    let bar = half.map_or(USED, |half| half.clamp(LEAST, USED));
    measured.moving.is_some_and(|moving| moving >= bar)
}

/// Whether a card is known to move too little to be using a wider band's
/// room however slowly its link sends, as one that may leave it.
pub fn resting(measured: &Measured) -> bool {
    measured.moving.is_some_and(|moving| moving < LEAST)
}

/// Whether a card is known to bring a download down, `LEAST` or more, so
/// what its access point carries is what it carries under load.
pub fn loaded(measured: &Measured) -> bool {
    measured.moving.is_some_and(|moving| moving >= LEAST)
}

/// The quicker of what the card's point carried lately and what it
/// usually carries, where that's known: the one the card's gone by.
fn quickest<'a>(lately: &'a Measured, usual: Option<&'a Measured>) -> &'a Measured {
    match usual {
        Some(usual) if slowness(usual) < slowness(lately) => usual,
        _ => lately,
    }
}

/// What the card's own point is weighed by against another's quicker:
/// the span, or, slowed by its own download lately and losing no more than
/// `LOSSY`, the quicker slow end of the span and what the point usually
/// carries, since the download's
/// queue slows any point a while. At 04:48 on 19 Nov 2026 ch52, whose
/// minutes moving `LEAST` or more had carried a mean 21.5 ms at p95 that
/// night, read 163 ms, and the card tried the hall's radio on its 49.2
/// at that hour, where it carried 124 to 137. Its loss is the span's.
fn weighed_by(measured: &Measured, usual: Option<&Measured>, queued: bool) -> Measured {
    let p95 = if queued {
        quickest(measured, usual).p95
    } else {
        measured.p95
    };
    Measured {
        p95,
        ..measured.clone()
    }
}

/// An access point's p95, where nothing answered as slow as can be.
fn slowness(measured: &Measured) -> Duration {
    measured.p95.unwrap_or(Duration::MAX)
}

/// How loud a card hears its own access point: the fainter of the scan's
/// signal and the station's own, as the scan's may be minutes old.
pub fn heard(measured: &Measured, signal: i16) -> i32 {
    measured
        .signal
        .map_or(i32::from(signal), |live| live.min(i32::from(signal)))
}

/// Whether one access point carried far quicker than another, and lost no
/// more.
fn quicker(theirs: &Measured, ours: &Measured) -> bool {
    let (their_p95, our_p95) = (slowness(theirs), slowness(ours));
    theirs.lost <= ours.lost.max(LOSSY)
        && their_p95 <= our_p95.mul_f64(QUICKER_BY)
        && our_p95.saturating_sub(their_p95) >= QUICKER
}

/// Whether a trial brought a download down slower than what it left, by
/// more than `MOVED_BY` allows: on 17 Nov 2026 moves from 5 to 2.4 GHz cut
/// downloads of 39 to 55 Mbit/s to 26 to 38.
fn slower(trial: &Measured, before: &Measured) -> bool {
    match (trial.moving, before.moving) {
        (Some(trial), Some(before)) => f64::from(trial) < f64::from(before) * MOVED_BY,
        _ => false,
    }
}

/// Whether a trial carried worse than what it left.
fn worse(trial: &Measured, before: &Measured) -> bool {
    slowness(trial) > slowness(before).saturating_add(WORSE)
        || trial.lost > before.lost + WORSE_LOSS
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A chooser that has seen this access point carry badly a minute
    /// before, so a second bad minute is acted on.
    pub(crate) fn after_a_bad_minute(point: &str) -> Chooser {
        Chooser {
            badly_on: Some(point.to_owned()),
            ..Chooser::default()
        }
    }

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";
    /// The desk's access point's own 2.4 GHz radio.
    const NEAR: &str = "6c:ae:f6:c1:84:27";
    const CH52: u32 = 5260;
    const CH149: u32 = 5745;
    const CH6: u32 = 2437;

    /// What a card in use carried, moving enough that a wider band's room
    /// counts.
    fn carried(p95: u64, lost: f64) -> Measured {
        Measured {
            echoes: 120,
            p50: Some(Duration::from_millis(p95 / 3)),
            p95: Some(Duration::from_millis(p95)),
            lost,
            retried: 1.0,
            failed: 0.0,
            signal: Some(-50),
            moving: Some(200),
            room: None,
        }
    }

    /// What a card carried moving just too little for a wider band's room
    /// to count.
    fn idle(mut measured: Measured) -> Measured {
        measured.moving = Some(99);
        measured
    }

    /// What a card carried moving just too little to be using a wider
    /// band's room however slowly its link sends.
    fn resting(mut measured: Measured) -> Measured {
        measured.moving = Some(24);
        measured
    }

    fn hall(signal: i16, busy: Option<u8>, measured: Option<Measured>) -> Other {
        Other {
            point: HALL.into(),
            signal,
            busy,
            frequency: CH52,
            measured,
        }
    }

    #[test]
    fn an_access_point_carrying_well_is_kept() {
        let mut chooser = Chooser::default();
        let well = carried(15, 0.0);
        let louder = hall(-40, Some(10), None);

        let choice = chooser.choose(Instant::now(), (DESK, &well, -50, CH52), &[louder], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "a louder one untried is no reason to move"
        );
    }

    #[test]
    fn one_minute_carrying_badly_moves_nothing_but_two_running_do() {
        let mut chooser = Chooser::default();
        let (badly, well) = (carried(45, 0.0), carried(15, 0.0));
        let near = [hall(-41, Some(20), None)];
        let (start, minute) = (Instant::now(), Duration::from_secs(60));

        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        let eased = chooser.choose(start + minute, (DESK, &well, -50, CH52), &near, false);
        let again = chooser.choose(start + minute * 2, (DESK, &badly, -50, CH52), &near, false);
        let still = chooser.choose(start + minute * 3, (DESK, &badly, -50, CH52), &near, false);

        assert_eq!(first, Choice::Stay, "one minute at 45 ms moves nothing");
        assert_eq!(eased, Choice::Stay, "15 ms carries well");
        assert_eq!(again, Choice::Stay, "a bad minute after a good one is new");
        assert_eq!(still, Choice::Try(HALL.into()), "two running are");
    }

    #[test]
    fn one_minute_carrying_badly_asks_no_look_but_two_running_do() {
        let mut chooser = Chooser::default();
        let badly = carried(52, 0.0);
        let start = Instant::now();

        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &[], false);
        let minute = start + Duration::from_secs(60);
        let second = chooser.choose(minute, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(first, Choice::Stay, "one minute at 52 ms asks no look");
        assert_eq!(second, Choice::Look, "two running do");
    }

    #[test]
    fn a_failing_link_is_left_on_its_first_minute() {
        let mut chooser = Chooser::default();
        let failing = carried(400, 15.0);
        let near = hall(-53, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &failing, -50, CH52), &[near], false);

        assert_eq!(choice, Choice::Try(HALL.into()), "15% lost is left at once");
    }

    #[test]
    fn one_carrying_badly_tries_another_heard_about_as_well() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(119, 0.5);
        let near = hall(-53, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50, CH52), &[near], false);

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "119 ms at p95 is worth a try"
        );
    }

    /// What a card carried on 17 Nov 2026 at 15:53, bringing a download
    /// down at 52 Mbit/s on a 5 GHz link: its middle echo held at 5.8 ms
    /// with none lost while its slow end waited behind the download.
    fn downloading(p95: u64) -> Measured {
        Measured {
            echoes: 121,
            p50: Some(Duration::from_micros(5800)),
            p95: Some(Duration::from_millis(p95)),
            lost: 0.0,
            retried: 0.0,
            failed: 0.0,
            signal: Some(-36),
            moving: Some(52),
            room: Some(1200),
        }
    }

    #[test]
    fn a_download_on_a_wider_band_keeps_off_a_narrower_one() {
        let mut chooser = after_a_bad_minute(DESK);
        let narrower = [near_at(-36, None)];

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &downloading(143), -40, CH52),
            &narrower,
            false,
        );

        assert_eq!(
            choice,
            Choice::Stay,
            "2.4 GHz would only bring the download down slower"
        );
    }

    /// What the card carried on ch52 before each of its moves to 2.4 GHz on
    /// 19 Nov 2026, nearly idle with none lost.
    fn idle_on_ch52(p95: u64, moving: u32) -> Measured {
        Measured {
            echoes: 120,
            p50: Some(Duration::from_millis(4)),
            p95: Some(Duration::from_millis(p95)),
            lost: 0.0,
            retried: 1.0,
            failed: 0.0,
            signal: Some(-38),
            moving: Some(moving),
            room: Some(1080),
        }
    }

    /// The far access point's 2.4 GHz radio as the card heard it then, with
    /// what it had carried at that hour before.
    fn far_on_ch6() -> Other {
        Other {
            point: "6c:ae:f6:ba:85:87".into(),
            signal: -38,
            busy: Some(20),
            frequency: CH6,
            measured: Some(idle_on_ch52(11, 10)),
        }
    }

    #[test]
    fn a_wider_band_resting_with_a_slow_tail_keeps_off_a_narrower_one_known_quicker() {
        // 17:29 on 19 Nov 2026: 34.3 ms at p95, moving 10 Mbit/s.
        let mut chooser = Chooser::default();
        let on = ("6c:ae:f6:ba:85:86", &idle_on_ch52(34, 10), -38, CH52);

        let start = Instant::now();

        let choice = chooser.choose(start, on, &[far_on_ch6()], false);
        let said = chooser.began_keeping_wide();
        let next = chooser.choose(start + Duration::from_secs(60), on, &[far_on_ch6()], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "2.4 GHz's quick tail at rest isn't worth its room"
        );
        assert_eq!(next, Choice::Stay, "nor the next minute");
        assert_eq!(
            (said, chooser.began_keeping_wide()),
            (Some("6c:ae:f6:ba:85:87".to_owned()), None),
            "the hold is said once a spell"
        );
        let called = chooser.choose(start + Duration::from_secs(120), on, &[far_on_ch6()], true);
        assert_eq!(called, Choice::Stay, "a call keeps the card where it is");
        chooser.choose(start + Duration::from_secs(180), on, &[far_on_ch6()], false);
        assert_eq!(
            chooser.began_keeping_wide(),
            Some("6c:ae:f6:ba:85:87".to_owned()),
            "a hold after the call is a new spell, said again"
        );
    }

    #[test]
    fn a_wider_band_kept_off_two_narrower_ones_names_the_quickest() {
        let mut chooser = Chooser::default();
        let on = ("6c:ae:f6:ba:85:86", &idle_on_ch52(34, 10), -38, CH52);
        let quicker = Other {
            point: NEAR.into(),
            measured: Some(idle_on_ch52(8, 10)),
            ..far_on_ch6()
        };

        chooser.choose(Instant::now(), on, &[far_on_ch6(), quicker], false);

        assert_eq!(
            chooser.began_keeping_wide(),
            Some(NEAR.to_owned()),
            "8 ms is the one it would have tried, not 11 heard first"
        );
    }

    #[test]
    fn a_wider_band_kept_to_where_nothing_narrower_would_have_been_tried_says_nothing() {
        let mut chooser = Chooser::default();
        let on = ("6c:ae:f6:ba:85:86", &idle_on_ch52(12, 10), -38, CH52);

        let choice = chooser.choose(Instant::now(), on, &[far_on_ch6()], false);

        assert_eq!(choice, Choice::Stay, "12 ms carries well");
        assert_eq!(
            chooser.began_keeping_wide(),
            None,
            "11 ms isn't far quicker"
        );
    }

    #[test]
    fn a_wider_band_slow_two_minutes_but_losing_nothing_keeps_off_a_narrower_one() {
        // 18:03 on 19 Nov 2026: 51.6 ms at p95 two minutes running, none lost.
        let mut chooser = after_a_bad_minute("6c:ae:f6:ba:85:86");
        let on = ("6c:ae:f6:ba:85:86", &idle_on_ch52(52, 24), -36, CH52);

        let choice = chooser.choose(Instant::now(), on, &[far_on_ch6()], false);

        assert_eq!(
            choice,
            Choice::Look,
            "a slow tail with nothing lost looks for another 5 GHz radio, not 2.4 GHz"
        );
        assert_eq!(
            chooser.began_keeping_wide(),
            Some("6c:ae:f6:ba:85:87".to_owned()),
            "the hold is said"
        );
    }

    /// What the card carried on 5745 MHz on the evening of 19 Nov 2026,
    /// moving 11 to 22 Mbit/s with none lost.
    fn on_5745(p50: Duration, p95: u64, moving: u32) -> Measured {
        Measured {
            echoes: 121,
            p50: Some(p50),
            p95: Some(Duration::from_millis(p95)),
            lost: 0.0,
            retried: 0.0,
            failed: 0.0,
            signal: Some(-46),
            moving: Some(moving),
            room: Some(1200),
        }
    }

    /// The 5745 MHz radio the card was on then, saying how busy its channel
    /// was, and the same access point's 2.4 GHz radio, with what that had
    /// carried at that hour before. Where `busy` is None the channel goes
    /// unheard, so a case isolates the middle's half of slow air.
    fn the_hall_then(busy: Option<u8>) -> [Other; 2] {
        let itself = Other {
            point: HALL.into(),
            signal: -46,
            busy,
            frequency: CH149,
            measured: None,
        };
        let narrower = Other {
            point: NEAR.into(),
            signal: -45,
            busy: Some(13),
            frequency: CH6,
            measured: Some(idle_on_ch52(17, 10)),
        };
        [itself, narrower]
    }

    #[test]
    fn a_wider_band_slow_in_its_middle_yields_to_a_narrower_one_known_quicker() {
        // 21:10 on 19 Nov 2026: a middle of 24.4 ms and 274.7 ms at p95.
        let mut chooser = after_a_bad_minute(HALL);
        let slow = on_5745(Duration::from_micros(24_400), 275, 22);

        let choice = chooser.choose(
            Instant::now(),
            (HALL, &slow, -44, CH149),
            &the_hall_then(None),
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(NEAR.into()),
            "a middle past 12 ms is the air's, which 2.4 GHz may beat"
        );
        assert_eq!(chooser.began_keeping_wide(), None, "nothing is held");
    }

    #[test]
    fn a_wider_band_on_a_busy_channel_yields_to_a_narrower_one_known_quicker() {
        // 21:20 on 19 Nov 2026: a middle of 11.9 ms, 46.4 ms at p95 over the
        // span, the channel 54% busy.
        let mut chooser = after_a_bad_minute(HALL);
        let slow = on_5745(Duration::from_micros(11_900), 46, 11);

        let choice = chooser.choose(
            Instant::now(),
            (HALL, &slow, -47, CH149),
            &the_hall_then(Some(54)),
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(NEAR.into()),
            "a channel 54% busy is the air's, which 2.4 GHz may beat"
        );
        let downloading = on_5745(Duration::from_micros(11_900), 46, 30);
        let held = after_a_bad_minute(HALL).choose(
            Instant::now(),
            (HALL, &downloading, -47, CH149),
            &the_hall_then(Some(54)),
            false,
        );
        assert_eq!(
            held,
            Choice::Stay,
            "moving 30 Mbit/s, 2.4 GHz would bring the download down slower"
        );
    }

    /// A chooser that has seen this access point carry badly a minute
    /// before, having found the card there a span ago, so its span is
    /// whole.
    fn a_span_on(point: &str) -> Chooser {
        Chooser {
            joined: Some((point.to_owned(), Instant::now() - SPAN)),
            ..after_a_bad_minute(point)
        }
    }

    #[test]
    fn a_wider_band_holds_at_the_edges_of_steady_busiest_least_and_echoed() {
        let (steady, busy, moving) = (Duration::from_micros(11_900), Some(40), 11);
        let cases = [
            ("a 12 ms middle", Duration::from_millis(12), 11, None, 121),
            (
                "a 12.1 ms middle",
                Duration::from_micros(12_100),
                11,
                None,
                121,
            ),
            ("a channel 40% busy", steady, moving, busy, 121),
            ("a channel 41% busy", steady, moving, Some(41), 121),
            ("moving 25", Duration::from_micros(24_400), 25, None, 121),
            ("moving 24", Duration::from_micros(24_400), 24, None, 121),
            ("108 echoes", Duration::from_millis(5), 11, None, 108),
            ("107 echoes", Duration::from_millis(5), 11, None, 107),
        ];

        let held: Vec<_> = cases
            .into_iter()
            .map(|(case, p50, moving, busy, echoes)| {
                let measured = Measured {
                    echoes,
                    ..on_5745(p50, 46, moving)
                };
                let choice = a_span_on(HALL).choose(
                    Instant::now(),
                    (HALL, &measured, -47, CH149),
                    &the_hall_then(busy),
                    false,
                );
                (case, choice == Choice::Look)
            })
            .collect();

        let expected = [
            ("a 12 ms middle", true),
            ("a 12.1 ms middle", false),
            ("a channel 40% busy", true),
            ("a channel 41% busy", false),
            ("moving 25", true),
            ("moving 24", false),
            ("108 echoes", true),
            ("107 echoes", false),
        ];
        assert_eq!(
            held, expected,
            "each holds, looking for 5 GHz, just to its edge"
        );
        let unknown = Measured {
            moving: None,
            ..on_5745(Duration::from_micros(24_400), 46, 11)
        };
        let choice = a_span_on(HALL).choose(
            Instant::now(),
            (HALL, &unknown, -47, CH149),
            &the_hall_then(None),
            false,
        );
        assert_eq!(choice, Choice::Look, "moving it doesn't know, it holds");
    }

    #[test]
    fn a_wider_band_holds_on_a_young_span_and_not_on_a_whole_one_half_unsent() {
        // A resume or a roam wpa_supplicant made itself: the 17:29 figures
        // on a span not two minutes old.
        let start = Instant::now();
        let young = Measured {
            echoes: 55,
            ..idle_on_ch52(34, 10)
        };
        let on = ("6c:ae:f6:ba:85:86", &young, -38, CH52);
        let mut chooser = Chooser::default();

        let first = chooser.choose(start, on, &[far_on_ch6()], false);
        let whole = chooser.choose(start + SPAN, on, &[far_on_ch6()], false);

        assert_eq!(
            first,
            Choice::Stay,
            "55 echoes a minute in is all a young span holds"
        );
        assert_eq!(
            whole,
            Choice::Try("6c:ae:f6:ba:85:87".into()),
            "55 a span on is half of them unsent"
        );
    }

    #[test]
    fn a_wider_band_holds_at_the_edges_of_lossy_and_faintest() {
        let point = "6c:ae:f6:ba:85:86";
        let judged = |lost, signal| {
            let measured = Measured {
                lost,
                signal: Some(signal),
                ..idle_on_ch52(52, 24)
            };
            let on = (point, &measured, i16::try_from(signal).unwrap_or(-38), CH52);
            let far = Other {
                signal: -66,
                ..far_on_ch6()
            };
            after_a_bad_minute(point).choose(Instant::now(), on, &[far], false)
        };

        assert_eq!(
            judged(2.0, -67),
            Choice::Look,
            "2% lost at -67 dBm still holds"
        );
        let far = Choice::Try("6c:ae:f6:ba:85:87".into());
        assert_eq!(judged(2.1, -67), far, "past 2% it doesn't");
        assert_eq!(judged(2.0, -68), far, "nor past -67 dBm");
    }

    #[test]
    fn a_wider_band_unechoed_or_fading_on_its_own_station_doesnt_hold() {
        let point = "6c:ae:f6:ba:85:86";
        let unechoed = Measured {
            echoes: 60,
            ..idle_on_ch52(52, 24)
        };
        let fading = Measured {
            signal: Some(-72),
            ..idle_on_ch52(52, 24)
        };
        let judged = |measured: &Measured| {
            let on = (point, measured, -38, CH52);
            a_span_on(point).choose(Instant::now(), on, &[far_on_ch6()], false)
        };

        let far = Choice::Try("6c:ae:f6:ba:85:87".into());
        assert_eq!(
            judged(&unechoed),
            far,
            "half a span's echoes, the rest unsent, says little of loss"
        );
        assert_eq!(
            judged(&fading),
            far,
            "the station hears -72 though the scan said -38"
        );
    }

    #[test]
    fn a_wider_band_losing_or_faint_may_still_take_a_narrower_one() {
        let point = "6c:ae:f6:ba:85:86";
        let mut lossy = idle_on_ch52(52, 24);
        lossy.lost = 3.0;

        let losing = after_a_bad_minute(point).choose(
            Instant::now(),
            (point, &lossy, -38, CH52),
            &[far_on_ch6()],
            false,
        );
        let faint = after_a_bad_minute(point).choose(
            Instant::now(),
            (point, &idle_on_ch52(52, 24), -70, CH52),
            &[Other {
                signal: -66,
                ..far_on_ch6()
            }],
            false,
        );

        let far = Choice::Try("6c:ae:f6:ba:85:87".into());
        assert_eq!(losing, far, "3% lost is the air, which 2.4 GHz may answer");
        assert_eq!(faint, far, "heard past FAINTEST, 5 GHz is fading");
    }

    #[test]
    fn a_download_on_a_wider_band_may_try_another_as_wide() {
        let mut chooser = after_a_bad_minute(DESK);
        let near = [hall(-41, Some(20), None), near_at(-36, None)];

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &downloading(143), -40, CH52),
            &near,
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "another radio as wide may drain the queue quicker"
        );
    }

    #[test]
    fn a_download_on_a_narrower_band_may_leave_for_a_wider_one() {
        let mut chooser = after_a_bad_minute(NEAR);
        let wider = [hall(-41, Some(20), None)];

        let choice = chooser.choose(
            Instant::now(),
            (NEAR, &downloading(65), -36, CH6),
            &wider,
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "5 GHz has the room 2.4 GHz's queue wants, as at 19:10 on 17 Nov"
        );
    }

    #[test]
    fn what_is_slowed_by_a_download_is_told_at_its_edges() {
        let with = |p50: Option<u64>, lost: f64, moving: Option<u32>, p95: u64| Measured {
            p50: p50.map(Duration::from_micros),
            lost,
            moving,
            ..downloading(p95)
        };
        let edges = [
            (
                "a 12 ms middle",
                with(Some(12_000), 0.0, Some(52), 143),
                true,
            ),
            (
                "a 12.1 ms middle",
                with(Some(12_100), 0.0, Some(52), 143),
                false,
            ),
            ("2% lost", with(Some(5800), 2.0, Some(52), 143), true),
            ("2.1% lost", with(Some(5800), 2.1, Some(52), 143), false),
            ("moving 25", with(Some(5800), 0.0, Some(25), 143), true),
            ("moving 24", with(Some(5800), 0.0, Some(24), 143), false),
            ("moving unread", with(Some(5800), 0.0, None, 143), false),
            ("a middle unread", with(None, 0.0, Some(52), 143), false),
            (
                "a slow end at 40 ms",
                with(Some(5800), 0.0, Some(52), 40),
                false,
            ),
            (
                "a 24 ms middle moving 100",
                with(Some(24_000), 0.0, Some(100), 143),
                true,
            ),
            (
                "a 24.1 ms middle moving 100",
                with(Some(24_100), 0.0, Some(100), 143),
                false,
            ),
            (
                "a 24 ms middle moving 100 losing 2.1%",
                with(Some(24_000), 2.1, Some(100), 143),
                false,
            ),
            (
                "a 12.1 ms middle moving 99",
                with(Some(12_100), 0.0, Some(99), 143),
                false,
            ),
        ];

        for (case, measured, slowed) in edges {
            assert_eq!(slowed_by_its_own(&measured), slowed, "{case}");
        }
    }

    #[test]
    fn a_download_slow_in_the_middle_leaves_for_a_narrower_band_only_lossy_or_faint() {
        let narrower = [near_at(-36, None)];
        let mut slow_middle = downloading(143);
        slow_middle.p50 = Some(Duration::from_millis(20));
        let mut lossy = slow_middle.clone();
        lossy.lost = 2.1;
        let cases = [
            (
                "a 20 ms middle heard well looks for 5 GHz",
                &slow_middle,
                -40,
                Choice::Look,
            ),
            ("2.1% lost", &lossy, -40, Choice::Try(NEAR.into())),
            ("heard at -68", &slow_middle, -68, Choice::Try(NEAR.into())),
        ];

        for (case, measured, signal, expected) in cases {
            let mut chooser = after_a_bad_minute(DESK);
            let choice = chooser.choose(
                Instant::now(),
                (DESK, measured, signal, CH52),
                &narrower,
                false,
            );
            assert_eq!(choice, expected, "{case}");
        }
    }

    #[test]
    fn a_download_on_a_wider_band_takes_a_narrower_one_known_quicker_under_load_only_faint() {
        let quick = [near_at(-35, Some(downloading(37)))];
        let chosen = |signal| {
            Chooser::default().choose(
                Instant::now(),
                (DESK, &downloading(136), signal, CH52),
                &quick,
                false,
            )
        };

        assert_eq!(
            chosen(-43),
            Choice::Stay,
            "37 ms against 136 is the download's own queue, which 2.4 GHz only slows"
        );
        assert_eq!(
            chosen(-68),
            Choice::Try(NEAR.into()),
            "heard past FAINTEST, 5 GHz is fading, and 2.4 GHz brought downloads down at 37 ms"
        );
    }

    #[test]
    fn a_download_using_a_wider_band_s_room_keeps_off_a_narrower_one_known_quicker() {
        let mut chooser = Chooser::default();
        let quick = [near_at(-35, Some(downloading(37)))];
        let using = Measured {
            moving: Some(100),
            ..downloading(136)
        };

        let choice = chooser.choose(Instant::now(), (DESK, &using, -68, CH52), &quick, false);

        assert_eq!(
            choice,
            Choice::Stay,
            "100 Mbit/s wants the room 2.4 GHz lacks, however quick or faint"
        );
    }

    /// What a card carried on 18 Nov 2026 at 22:12, bringing a download
    /// down at 122 Mbit/s on ch52: its middle echo rose just past `STEADY`
    /// with none lost.
    fn using_its_room(lost: f64) -> Measured {
        Measured {
            p50: Some(Duration::from_micros(12_100)),
            lost,
            moving: Some(122),
            ..downloading(72)
        }
    }

    #[test]
    fn a_download_using_a_wider_band_s_room_keeps_off_a_narrower_one_unless_lossy() {
        let narrower = [near_at(-37, None)];
        let tried = |measured: &Measured| {
            after_a_bad_minute(DESK).choose(
                Instant::now(),
                (DESK, measured, -44, CH52),
                &narrower,
                false,
            )
        };

        assert_eq!(
            tried(&using_its_room(2.0)),
            Choice::Stay,
            "122 Mbit/s wants the room 2.4 GHz lacks, slow in the middle or not"
        );
        assert_eq!(
            tried(&using_its_room(2.1)),
            Choice::Try(NEAR.into()),
            "2.1% lost is the air"
        );
    }

    #[test]
    fn a_download_weighs_another_against_what_its_point_usually_carries_under_load() {
        let slowed = Measured {
            moving: Some(77),
            ..downloading(163)
        };
        let hall_learned = hall(-43, None, Some(carried(49, 0.0)));
        let usual = desk_at(-41, Some(carried(21, 0.0)));
        let on = (DESK, &slowed, -41, CH52);

        let weighed = after_a_bad_minute(DESK).choose(
            Instant::now(),
            on,
            &[usual, hall_learned.clone()],
            false,
        );
        let unweighed = after_a_bad_minute(DESK).choose(
            Instant::now(),
            on,
            std::slice::from_ref(&hall_learned),
            false,
        );
        let quicker_still = hall(-43, None, Some(carried(9, 0.0)));
        let beaten = desk_at(-41, Some(carried(21, 0.0)));
        let bettered =
            after_a_bad_minute(DESK).choose(Instant::now(), on, &[beaten, quicker_still], false);
        // The desk known under load only by its idle 60 ms, a floor.
        let floor = || desk_at(-41, Some(carried(60, 0.0)));
        let floored = after_a_bad_minute(DESK).choose(
            Instant::now(),
            on,
            &[floor(), hall_learned.clone()],
            false,
        );

        assert_eq!(
            weighed,
            Choice::Stay,
            "49 ms is slower than the 21 ch52 carries downloads at, as at 04:48 on 19 Nov"
        );
        assert_eq!(
            unweighed,
            Choice::Try(HALL.into()),
            "with nothing usual known, the span is gone by"
        );
        assert_eq!(
            bettered,
            Choice::Try(HALL.into()),
            "9 ms is far quicker than the 21 ch52 usually carries downloads at"
        );
        assert_eq!(
            floored,
            Choice::Stay,
            "the hall's idle 49 ms isn't quicker than the desk's own idle floor of 60"
        );
    }

    #[test]
    fn a_trial_is_barred_by_its_points_usual_save_a_floor_under_load() {
        let start = Instant::now();
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let loaded = |p95| Measured {
            moving: Some(77),
            ..downloading(p95)
        };
        // Tried off the desk, usually carrying this, for a hall known at
        // 9 ms, and judged a trial on: what the card did, and why.
        let judged = |minute: &Measured, usual: Measured, there: &Measured| {
            let mut chooser = after_a_bad_minute(DESK);
            let others = [
                desk_at(-41, Some(usual)),
                hall(-43, None, Some(carried(9, 0.0))),
            ];
            let tried = chooser.choose(start, (DESK, minute, -41, CH52), &others, false);
            assert_eq!(tried, Choice::Try(HALL.into()), "9 ms is far quicker");
            let now = start + TRIAL;
            let judged = chooser.choose(now, (HALL, there, -43, CH52), &others, false);
            assert!(chooser.trial.is_none(), "the trial ended, judged");
            (judged, chooser.went_back())
        };
        let usually = |(choice, why): &(Choice, Option<Against>)| {
            *choice == Choice::Back(DESK.into()) && matches!(why, Some(Against::Usually(_)))
        };

        let learned = judged(&loaded(163), carried(21, 0.0), &loaded(100));
        let lossy = judged(&loaded(163), carried(21, 3.0), &loaded(100));
        let floor = judged(&loaded(163), carried(60, 0.0), &loaded(100));
        let at_slow = judged(&loaded(163), carried(40, 0.0), &loaded(100));
        let past_slow = judged(&loaded(163), carried(41, 0.0), &loaded(100));
        let idle = judged(&unloaded(120), carried(50, 0.0), &unloaded(70));

        assert!(
            usually(&learned),
            "100 ms under load is worse than the desk's learned 21: {learned:?}"
        );
        assert!(
            usually(&lossy),
            "and the usual's blended loss doesn't take the bar away: {lossy:?}"
        );
        assert_eq!(
            floor,
            (Choice::Stay, None),
            "but 60 ms is a floor, no bar for a download to beat"
        );
        assert!(
            usually(&at_slow),
            "a usual of SLOW is no floor, so still bars: {at_slow:?}"
        );
        assert_eq!(
            past_slow,
            (Choice::Stay, None),
            "one a millisecond past it is"
        );
        assert!(
            usually(&idle),
            "idle, a slow usual still bars: 70 ms is worse than 50: {idle:?}"
        );
    }

    #[test]
    fn a_lossy_minute_just_after_a_download_goes_by_its_span() {
        let start = Instant::now();
        let slowed = Measured {
            moving: Some(77),
            ..downloading(163)
        };
        let lossy = Measured {
            lost: 2.1,
            ..slowed.clone()
        };
        let others = [
            desk_at(-41, Some(carried(21, 0.0))),
            hall(-43, None, Some(carried(49, 0.0))),
        ];
        let mut chooser = after_a_bad_minute(DESK);

        let first = chooser.choose(start, (DESK, &slowed, -41, CH52), &others, false);
        let next = chooser.choose(start + TAIL, (DESK, &lossy, -41, CH52), &others, false);

        assert_eq!(first, Choice::Stay, "its own queue is weighed by its usual");
        assert_eq!(
            next,
            Choice::Try(HALL.into()),
            "2.1% lost in the tail is the air"
        );
    }

    #[test]
    fn a_lossy_download_leaves_however_well_its_point_usually_carries_under_load() {
        let lossy = Measured {
            lost: 2.1,
            moving: Some(77),
            ..downloading(163)
        };
        let hall_learned = hall(-43, None, Some(carried(49, 0.0)));
        let usual = desk_at(-41, Some(carried(21, 0.0)));

        let choice = after_a_bad_minute(DESK).choose(
            Instant::now(),
            (DESK, &lossy, -41, CH52),
            &[usual, hall_learned],
            false,
        );

        assert_eq!(choice, Choice::Try(HALL.into()), "2.1% lost is the air");
    }

    #[test]
    fn a_download_on_a_narrower_band_goes_by_its_span_however_well_it_usually_carries() {
        let slowed = Measured {
            moving: Some(77),
            ..downloading(163)
        };
        let usual = near_at(-36, Some(carried(21, 0.0)));
        let wider = hall(-43, None, Some(carried(49, 0.0)));

        let choice = after_a_bad_minute(NEAR).choose(
            Instant::now(),
            (NEAR, &slowed, -36, CH6),
            &[usual, wider],
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "2.4 GHz's queue is drained quicker on 5 GHz, as at 19:10 on 17 Nov"
        );
    }

    #[test]
    fn a_trial_made_under_load_is_judged_under_load() {
        let start = Instant::now();
        let judged = |lost| {
            let mut chooser = after_a_bad_minute(NEAR);
            let wider = [desk_at(-43, None)];
            let tried = chooser.choose(start, (NEAR, &downloading(89), -35, CH6), &wider, false);
            assert_eq!(tried, Choice::Try(DESK.into()), "the first move is asked");
            let worse = Measured {
                lost,
                ..downloading(136)
            };
            chooser.choose(start + TRIAL, (DESK, &worse, -43, CH52), &[], false)
        };

        assert_eq!(
            judged(3.0),
            Choice::Back(NEAR.into()),
            "136 ms under load losing 3% is worse than the 89 it left under load"
        );
        assert_eq!(
            judged(0.0),
            Choice::Stay,
            "losing nothing, the wider band holds"
        );
    }

    #[test]
    fn a_trial_made_under_load_and_carrying_no_worse_under_load_stands() {
        let mut chooser = after_a_bad_minute(NEAR);
        let start = Instant::now();
        let wider = [desk_at(-43, None)];
        let tried = chooser.choose(start, (NEAR, &downloading(89), -35, CH6), &wider, false);
        assert_eq!(tried, Choice::Try(DESK.into()), "the first move is asked");

        let judged = chooser.choose(
            start + TRIAL,
            (DESK, &downloading(99), -43, CH52),
            &[],
            false,
        );

        assert_eq!(judged, Choice::Stay, "99 ms is within 10 of the 89 it left");
        assert_eq!(
            chooser.choose(
                start + TRIAL + TAIL + Duration::from_secs(30),
                (DESK, &carried(200, 1.0), -43, CH52),
                &[],
                false,
            ),
            Choice::Stay,
            "its trial was judged, not held for the download's tail, and a settle keeps it"
        );
    }

    #[test]
    fn a_trial_made_under_load_bringing_the_download_down_slower_goes_back() {
        let start = Instant::now();
        let quick = [near_at(-35, Some(downloading(37)))];
        let back_to = [desk_at(-43, None)];
        let slower = Measured {
            moving: Some(30),
            ..downloading(40)
        };
        let as_fast = Measured {
            moving: Some(45),
            ..downloading(40)
        };
        let judged = |moving: &Measured| {
            let mut chooser = Chooser::default();
            let on = (DESK, &downloading(136), -68, CH52);
            assert_eq!(
                chooser.choose(start, on, &quick, false),
                Choice::Try(NEAR.into())
            );
            chooser.choose(start + TRIAL, (NEAR, moving, -35, CH6), &back_to, false)
        };

        assert_eq!(
            judged(&slower),
            Choice::Back(DESK.into()),
            "30 Mbit/s of the 52 it left is a download cut, as on 17 Nov"
        );
        assert_eq!(judged(&as_fast), Choice::Stay, "45 of 52 is as fast");
    }

    #[test]
    fn a_move_to_a_wider_band_under_load_goes_back_where_it_brings_the_download_down_slower() {
        let start = Instant::now();
        let judged = |moving, usual: Option<Measured>| {
            let mut chooser = after_a_bad_minute(NEAR);
            let wider = [desk_at(-43, None), near_at(-35, usual)];
            let tried = chooser.choose(start, (NEAR, &downloading(89), -35, CH6), &wider, false);
            assert_eq!(tried, Choice::Try(DESK.into()), "the first move is asked");
            let there = Measured {
                moving: Some(moving),
                ..downloading(99)
            };
            chooser.choose(start + TRIAL, (DESK, &there, -43, CH52), &[], false)
        };

        assert_eq!(
            judged(30, None),
            Choice::Back(NEAR.into()),
            "30 Mbit/s of the 52 it left is a download cut, wider band or not"
        );
        assert_eq!(judged(45, None), Choice::Stay, "45 of 52 stands");
        assert_eq!(
            judged(
                45,
                Some(Measured {
                    moving: None,
                    ..downloading(20)
                })
            ),
            Choice::Stay,
            "a wider band holding well stands however quick the narrower usually is"
        );
    }

    #[test]
    fn a_trial_made_for_a_wider_band_s_room_waits_out_its_download_s_queue() {
        let mut chooser = Chooser::default();
        let start = Instant::now();
        let wider = [desk_at(-43, None)];
        let using = on_a_link(150, 286);
        let tried = chooser.choose(start, (NEAR, &using, -35, CH6), &wider, false);
        assert_eq!(tried, Choice::Try(DESK.into()), "the move is for the room");

        let judged = chooser.choose(
            start + TRIAL,
            (DESK, &downloading(68), -43, CH52),
            &[],
            false,
        );

        assert_eq!(
            judged,
            Choice::Stay,
            "68 ms is the download's own queue, which says nothing of the room"
        );
    }

    #[test]
    fn a_card_using_2_4_ghz_s_room_takes_5_ghz_no_slower_under_load() {
        let known = || [desk_at(-43, Some(downloading(68)))];
        let slow = Measured {
            p95: Some(Duration::from_millis(77)),
            ..on_a_link(150, 286)
        };
        let quick = Measured {
            p95: Some(Duration::from_millis(30)),
            ..on_a_link(150, 286)
        };

        let took =
            Chooser::default().choose(Instant::now(), (NEAR, &slow, -35, CH6), &known(), false);
        let kept =
            Chooser::default().choose(Instant::now(), (NEAR, &quick, -35, CH6), &known(), false);

        assert_eq!(
            took,
            Choice::Try(DESK.into()),
            "68 ms under load is no slower than 77"
        );
        assert_eq!(kept, Choice::Stay, "68 is slower than 30, and past `SLOW`");
    }

    #[test]
    fn a_trial_slowed_by_what_the_card_moves_waits_to_be_judged() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let off_load = resting(carried(119, 0.5));
        let tried = chooser.choose(start, (DESK, &off_load, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let minute = Duration::from_secs(60);
        let queued = chooser.choose(
            start + TRIAL,
            (HALL, &downloading(143), -53, CH52),
            &[],
            false,
        );
        let ending = Measured {
            moving: Some(20),
            ..downloading(80)
        };
        let tail = chooser.choose(
            start + TRIAL + minute,
            (HALL, &ending, -53, CH52),
            &[],
            false,
        );
        let judged = chooser.choose(
            start + TRIAL + minute * 2,
            (HALL, &carried(200, 1.0), -53, CH52),
            &[],
            false,
        );

        assert_eq!(queued, Choice::Stay, "a download's minute judges nothing");
        assert_eq!(tail, Choice::Stay, "nor the minute its download ended in");
        assert_eq!(
            judged,
            Choice::Back(DESK.into()),
            "the next is judged worse"
        );
    }

    #[test]
    fn a_trial_held_by_a_long_download_ends_unjudged_a_settle_on() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let off_load = resting(carried(119, 0.5));
        let tried = chooser.choose(start, (DESK, &off_load, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let minute = Duration::from_secs(60);
        let narrower = [near_at(-36, None)];
        let mut at = start + TRIAL;
        while at <= start + TRIAL + SETTLE {
            let held = chooser.choose(at, (HALL, &downloading(143), -53, CH52), &narrower, false);
            assert_eq!(held, Choice::Stay, "held at {:?}", at - start);
            at += minute;
        }
        let worse = carried(200, 1.0);
        let after = chooser.choose(at + minute * 2, (HALL, &worse, -53, CH52), &narrower, false);

        assert_ne!(after, Choice::Back(DESK.into()), "its trial ended unjudged");
    }

    #[test]
    fn one_carrying_badly_and_hearing_no_other_looks_for_one() {
        let mut chooser = Chooser::default();
        let (badly, well) = (carried(52, 0.0), carried(15, 0.0));
        let now = Instant::now();

        let carrying_well = chooser.choose(now, (DESK, &well, -50, CH52), &[], false);
        let calling = chooser.choose(now, (DESK, &badly, -50, CH52), &[], true);
        let looking = chooser.choose(now, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(
            carrying_well,
            Choice::Stay,
            "one carrying well needs no other"
        );
        assert_eq!(calling, Choice::Stay, "no look in a call, as no move");
        assert_eq!(looking, Choice::Look, "52 ms at p95 is worth a look");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_and_hearing_no_5_ghz_looks_for_it() {
        let mut chooser = Chooser::default();
        let well = carried(17, 0.0);
        let now = Instant::now();

        let calling = chooser.choose(now, (NEAR, &well, -43, CH6), &[], true);
        let looking = chooser.choose(now, (NEAR, &well, -43, CH6), &[], false);
        let soon = now + Duration::from_secs(60);
        let soon = chooser.choose(soon, (NEAR, &well, -43, CH6), &[], false);
        let later = chooser.choose(now + SETTLE, (NEAR, &well, -43, CH6), &[], false);

        assert_eq!(calling, Choice::Stay, "no look in a call");
        assert_eq!(looking, Choice::LookWider, "5 GHz is worth a look");
        assert_eq!(soon, Choice::Stay, "a minute on, it isn't asked again");
        assert_eq!(later, Choice::LookWider, "once settled, it is");
    }

    #[test]
    fn a_card_on_2_4_ghz_hearing_only_2_4_ghz_looks_for_5_ghz() {
        let mut chooser = Chooser::default();
        let well = carried(17, 0.0);
        let other = Other {
            point: HALL.into(),
            ..near_at(-45, None)
        };

        let choice = chooser.choose(Instant::now(), (NEAR, &well, -43, CH6), &[other], false);

        assert_eq!(choice, Choice::LookWider, "2.4 GHz is no wider");
    }

    #[test]
    fn a_look_for_another_is_asked_again_only_once_settled() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(52, 0.0);
        let start = Instant::now();
        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &[], false);

        let soon = start + Duration::from_secs(60);
        let soon = chooser.choose(soon, (DESK, &badly, -50, CH52), &[], false);
        let later = chooser.choose(start + SETTLE, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(first, Choice::Look, "the first look is asked");
        assert_eq!(soon, Choice::Stay, "a minute on, it isn't asked again");
        assert_eq!(later, Choice::Look, "once settled, it is");
    }

    #[test]
    fn one_carrying_badly_keeps_off_another_heard_much_more_faintly() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let faint = hall(-70, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50, CH52), &[faint], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "20 dB fainter would carry worse still"
        );
    }

    #[test]
    fn one_carrying_badly_keeps_off_an_untried_channel_45_percent_busy() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(72, 0.0);
        let busy = hall(-45, Some(45), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -47, CH52), &[busy], false);

        assert_eq!(choice, Choice::Stay, "45% busy carried badly all afternoon");
    }

    #[test]
    fn one_measured_lately_and_carrying_far_better_is_moved_to() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let better = hall(-55, None, Some(carried(12, 0.0)));

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &middling, -50, CH52),
            &[better],
            false,
        );

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "12 ms against 30 beats both margins"
        );
    }

    #[test]
    fn one_carrying_only_a_little_better_is_not() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let little = hall(-55, None, Some(carried(24, 0.0)));

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &middling, -50, CH52),
            &[little],
            false,
        );

        assert_eq!(choice, Choice::Stay, "6 ms better is within the margin");
    }

    #[test]
    fn nothing_moves_during_a_call_unless_the_link_is_failing() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let failing = carried(400, 15.0);
        let near = hall(-53, Some(20), None);
        let now = Instant::now();

        let on = (DESK, &badly, -50, CH52);
        let held = chooser.choose(now, on, std::slice::from_ref(&near), true);
        let saved = chooser.choose(now, (DESK, &failing, -50, CH52), &[near], true);

        assert_eq!(held, Choice::Stay, "a slow link is borne through a call");
        assert_eq!(saved, Choice::Try(HALL.into()), "a failing one isn't");
    }

    #[test]
    fn a_card_just_moved_stays_put_a_while() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(119, 0.5);
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50, CH52),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            ..near
        };

        let soon = start + TRIAL + Duration::from_secs(1);
        let choice = chooser.choose(soon, (HALL, &badly, -53, CH52), &[desk], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "no second move before the card settles"
        );
    }

    #[test]
    fn a_move_that_carries_worse_goes_back_and_is_not_tried_again_soon() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let badly = carried(119, 0.5);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50, CH52),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            signal: -50,
            busy: None,
            frequency: CH52,
            measured: Some(badly.clone()),
        };

        let worse = carried(200, 1.0);
        let back = chooser.choose(start + TRIAL, (HALL, &worse, -53, CH52), &[desk], false);
        let later = start + SETTLE + TRIAL + Duration::from_secs(1);
        let again = chooser.choose(later, (DESK, &badly, -50, CH52), &[near], false);

        assert_eq!(
            back,
            Choice::Back(DESK.into()),
            "200 ms is worse than the 119 it left"
        );
        assert_eq!(
            again,
            Choice::Stay,
            "the hall carried worse from here an hour ago"
        );
    }

    #[test]
    fn a_move_never_made_keeps_off_the_point_it_was_for_an_hour() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let minute = start + Duration::from_secs(60);
        chooser.choose(minute, (DESK, &badly, -50, CH52), &near, false);
        let (said, again) = (chooser.unreached(), chooser.unreached());
        let later = start + SETTLE + Duration::from_secs(1);
        let settled = chooser.choose(later, (DESK, &badly, -50, CH52), &near, false);
        let spent = minute + SPURNED;
        let nearly = spent - Duration::from_secs(1);
        let short = chooser.choose(nearly, (DESK, &badly, -50, CH52), &near, false);
        let hour = chooser.choose(spent, (DESK, &badly, -50, CH52), &near, false);

        assert_eq!(said, Some(HALL.into()), "the hall's kept off, and said so");
        assert_eq!(again, None, "it's said once");
        assert_eq!(settled, Choice::Stay, "the hall wasn't reached from here");
        assert_eq!(short, Choice::Stay, "an hour from its judgement");
        assert_eq!(hour, Choice::Try(HALL.into()), "tried again after an hour");
    }

    /// The far access point's 5 GHz radio on channel 52, heard more faintly
    /// than the hall's but within reach, so it's chosen only where the
    /// hall is kept off.
    fn elsewhere() -> Other {
        Other {
            point: "6c:ae:f6:ba:85:86".into(),
            ..hall(-55, Some(20), None)
        }
    }

    #[test]
    fn a_move_never_made_holds_till_the_later_of_its_span_and_the_last_settle() {
        let now = Instant::now() + SETTLE;
        let spanned = now.checked_sub(SETTLE.saturating_sub(SPAN));
        let earlier = spanned.and_then(|spanned| spanned.checked_sub(SPAN));

        let first = unmade(now, None, spanned);
        let after_a_late_one = unmade(now, Some(now), spanned);
        let after_an_old_one = unmade(now, earlier, spanned);
        let unreachable = unmade(now, None, None);

        assert_eq!(first, spanned, "a first move holds a span");
        assert_eq!(
            after_a_late_one,
            Some(now),
            "a later settle before it holds"
        );
        assert_eq!(after_an_old_one, spanned, "an older one doesn't");
        assert_eq!(
            unreachable,
            Some(now),
            "no instant a span back, so a settle from now"
        );
    }

    #[test]
    fn a_move_never_made_on_a_failing_link_tries_another_at_once() {
        let start = Instant::now();
        let near = [hall(-53, Some(20), None), elsewhere()];
        let failing = carried(400, 15.0);
        let after = |at| {
            let mut chooser = after_a_bad_minute(DESK);
            let badly = carried(119, 0.5);
            let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near[..1], false);
            assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
            chooser.choose(at, (DESK, &failing, -50, CH52), &near, false)
        };

        let minute = after(start + Duration::from_secs(60));
        // A whole span on the desk, so a keep-off that yielded would let
        // the hall be tried.
        let whole = after(start + SPAN);

        assert_eq!(
            minute,
            Choice::Try(elsewhere().point),
            "a failing desk leaves at once for another, the hall kept off"
        );
        assert_eq!(
            whole,
            Choice::Try(elsewhere().point),
            "and the hall stays kept off from a link failing a whole span"
        );
    }

    /// A fourth access point on channel 52, known to carry far quicker than
    /// a bad minute, so a card settled takes it.
    fn quicker_still() -> Other {
        Other {
            point: "6c:ae:f6:ba:85:88".into(),
            ..hall(-54, Some(20), Some(carried(15, 0.0)))
        }
    }

    #[test]
    fn a_move_sent_back_where_it_began_settles_from_the_move_before_it() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let well = carried(15, 0.0);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &hall_alone, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
        let minute = start + Duration::from_secs(60);
        chooser.choose(minute, (HALL, &well, -50, CH52), &hall_alone, false);
        let kept = chooser.choose(start + TRIAL, (HALL, &well, -50, CH52), &hall_alone, false);
        assert_eq!(kept, Choice::Stay, "the hall stood its trial");

        // Failing on the hall, a second move is tried before the first's
        // settle, and sent back to the hall.
        let failing = carried(400, 15.0);
        let second = start + TRIAL + Duration::from_secs(60);
        let on = (HALL, &failing, -50, CH52);
        let tried_again = chooser.choose(second, on, &[elsewhere()], false);
        let desk = Other {
            point: DESK.into(),
            ..hall(-53, Some(20), None)
        };
        let near = [desk, elsewhere()];
        let back = second + Duration::from_secs(60);
        let waits = chooser.choose(back, (HALL, &badly, -50, CH52), &near, false);
        // Past a span-only hold from the way back, short of the first's
        // settle, and a second past where a settle from the span alone
        // would end, so that one shows on any comparison.
        let spanned = back + SPAN + Duration::from_secs(1);
        let held = chooser.choose(spanned, (HALL, &badly, -50, CH52), &near, false);
        let settled = start + SETTLE + Duration::from_secs(1);
        let next = chooser.choose(settled, (HALL, &badly, -50, CH52), &near, false);

        assert_eq!(held, Choice::Stay, "held to the first move's settle");
        assert_eq!(
            tried_again,
            Choice::Try(elsewhere().point),
            "tried as it failed"
        );
        assert_eq!(
            waits,
            Choice::Stay,
            "the move before it still settles: the second never counted, nor cleared it"
        );
        assert_eq!(
            next,
            Choice::Try(DESK.into()),
            "tried once the first move has settled, not the second"
        );
    }

    #[test]
    fn a_first_move_sent_back_where_it_began_waits_a_span_not_a_settle() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &hall_alone, false);
        assert_eq!(
            tried,
            Choice::Try(HALL.into()),
            "the card's first move is asked"
        );

        // The move never got there, and the card was sent back to the desk.
        let near = [hall(-53, Some(20), None), quicker_still()];
        let back = start + Duration::from_secs(60);
        let waits = chooser.choose(back, (DESK, &badly, -50, CH52), &near, false);
        let short = back + SPAN - Duration::from_secs(1);
        let still = chooser.choose(short, (DESK, &badly, -50, CH52), &near, false);
        // A second past the span, so it holds on any comparison.
        let spanned = back + SPAN + Duration::from_secs(1);
        let hall_only = chooser.choose(spanned, (DESK, &badly, -50, CH52), &hall_alone, false);
        let next = chooser.choose(spanned, (DESK, &badly, -50, CH52), &near, false);
        // Exactly a stay, which moves nothing `next` is chosen from.
        assert_eq!(hall_only, Choice::Stay, "the hall it missed is kept off");

        assert_eq!(
            waits,
            Choice::Stay,
            "its span still holds the failed move's outage"
        );
        assert_eq!(still, Choice::Stay, "short of a span since it came back");
        assert_eq!(
            next,
            Choice::Try(quicker_still().point),
            "tried a span on, long before a settle"
        );
    }

    #[test]
    fn a_second_never_stood_miss_within_an_hour_holds_a_whole_settle() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let on = (DESK, &badly, -50, CH52);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, on, &hall_alone, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
        let back = start + Duration::from_secs(60);
        chooser.choose(back, on, &hall_alone, false);
        let spanned = back + SPAN;
        let second = chooser.choose(spanned, on, &[elsewhere()], false);
        assert_eq!(
            second,
            Choice::Try(elsewhere().point),
            "a span after the first miss, another is tried"
        );

        // That one never got there either.
        let near = [quicker_still()];
        let back_again = spanned + Duration::from_secs(60);
        chooser.choose(back_again, on, &near, false);
        // A second past the span, so it holds on any comparison.
        let past_span = back_again + SPAN + Duration::from_secs(1);
        let span_on = chooser.choose(past_span, on, &near, false);
        let short = chooser.choose(spanned + SETTLE - Duration::from_secs(1), on, &near, false);
        let settled = spanned + SETTLE + Duration::from_secs(1);
        let next = chooser.choose(settled, on, &near, false);

        assert_eq!(
            span_on,
            Choice::Stay,
            "a second miss within an hour holds past a span"
        );
        assert_eq!(short, Choice::Stay, "to a settle from its move");
        assert_eq!(
            next,
            Choice::Try(quicker_still().point),
            "and tries once that has passed"
        );
    }

    #[test]
    fn a_move_that_stands_between_two_misses_still_has_the_second_hold_a_settle() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let well = carried(15, 0.0);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &hall_alone, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
        let back = start + Duration::from_secs(60);
        chooser.choose(back, (DESK, &badly, -50, CH52), &hall_alone, false);

        // The next move stands, and holds well past a settle.
        let spanned = back + SPAN;
        let second = chooser.choose(spanned, (DESK, &badly, -50, CH52), &[elsewhere()], false);
        assert_eq!(
            second,
            Choice::Try(elsewhere().point),
            "a span after the miss"
        );
        let there = elsewhere().point;
        let stood = (there.as_str(), &well, -50, CH52);
        chooser.choose(spanned + Duration::from_secs(60), stood, &[], false);
        let kept = chooser.choose(spanned + TRIAL, stood, &[], false);
        assert_eq!(kept, Choice::Stay, "the second stood its trial");

        // Failing there a settle on, a third is tried and never made.
        let failing = carried(400, 15.0);
        let third = spanned + SETTLE + Duration::from_secs(60);
        let on = (there.as_str(), &failing, -50, CH52);
        let tried_again = chooser.choose(third, on, &[quicker_still()], false);
        assert_eq!(
            tried_again,
            Choice::Try(quicker_still().point),
            "tried as it failed"
        );
        let desk = Other {
            point: DESK.into(),
            ..hall(-53, Some(20), None)
        };
        let near = [desk];
        let missed = third + Duration::from_secs(60);
        let on = (there.as_str(), &badly, -50, CH52);
        chooser.choose(missed, on, &near, false);
        // A second past the span, so it holds on any comparison.
        let past_span = missed + SPAN + Duration::from_secs(1);
        let span_on = chooser.choose(past_span, on, &near, false);
        let next = chooser.choose(third + SETTLE + Duration::from_secs(1), on, &near, false);

        assert_eq!(
            span_on,
            Choice::Stay,
            "a stand between misses doesn't make the second a first"
        );
        assert_eq!(
            next,
            Choice::Try(DESK.into()),
            "tried a settle from the move"
        );
    }

    #[test]
    fn the_hour_runs_from_the_last_miss_not_the_first() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let on = (DESK, &badly, -50, CH52);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, on, &hall_alone, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
        let back = start + Duration::from_secs(60);
        chooser.choose(back, on, &hall_alone, false);

        // Fifty minutes on, a second misses, and holds a settle.
        let second_try = back + Duration::from_mins(50);
        let second = chooser.choose(second_try, on, &[elsewhere()], false);
        assert_eq!(
            second,
            Choice::Try(elsewhere().point),
            "fifty minutes after the first miss, another is tried"
        );
        let near = [quicker_still()];
        chooser.choose(second_try + Duration::from_secs(60), on, &near, false);

        // A third, past an hour from the first miss but minutes from the
        // second, misses too.
        let third_try = second_try + SETTLE + Duration::from_secs(60);
        let third = chooser.choose(third_try, on, &near, false);
        assert_eq!(
            third,
            Choice::Try(quicker_still().point),
            "a settle after the second, a third is tried"
        );
        let missed = third_try + Duration::from_secs(60);
        chooser.choose(missed, on, &hall_alone, false);
        // A second past the span, so it holds on any comparison.
        let past_span = missed + SPAN + Duration::from_secs(1);
        let span_on = chooser.choose(past_span, on, &hall_alone, false);
        let settled = third_try + SETTLE + Duration::from_secs(1);
        let next = chooser.choose(settled, on, &hall_alone, false);

        assert_eq!(
            span_on,
            Choice::Stay,
            "within an hour of the second miss, the third holds past a span"
        );
        assert_eq!(
            next,
            Choice::Try(HALL.into()),
            "tried a settle from its move"
        );
    }

    #[test]
    fn a_miss_an_hour_after_the_last_waits_a_span_again() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so taking one off is an instant on any clock.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let on = (DESK, &badly, -50, CH52);
        let hall_alone = [hall(-53, Some(20), None)];
        let tried = chooser.choose(start, on, &hall_alone, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");
        let back = start + Duration::from_secs(60);
        chooser.choose(back, on, &hall_alone, false);
        let later = back + SPURNED + Duration::from_secs(60);
        let second = chooser.choose(later, on, &[elsewhere()], false);
        assert_eq!(
            second,
            Choice::Try(elsewhere().point),
            "an hour after the first miss, another is tried"
        );

        // That one never got there either, an hour after the first.
        let near = [quicker_still()];
        let back_again = later + Duration::from_secs(60);
        chooser.choose(back_again, on, &near, false);
        let short = back_again + SPAN - Duration::from_secs(1);
        let still = chooser.choose(short, on, &near, false);
        // A second past the span, so it holds on any comparison.
        let spanned = back_again + SPAN + Duration::from_secs(1);
        let next = chooser.choose(spanned, on, &near, false);

        assert_eq!(still, Choice::Stay, "short of a span since it came back");
        assert_eq!(
            next,
            Choice::Try(quicker_still().point),
            "a miss an hour on is a first again, held a span, not a settle"
        );
    }

    #[test]
    fn a_move_ending_on_a_third_point_still_settles() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so a span's hold is an instant on any clock and
        // can't pass as a full settle.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let hall_alone = [hall(-53, Some(20), None)];
        chooser.choose(start, (DESK, &badly, -50, CH52), &hall_alone, false);

        let third = elsewhere().point;
        let near = [hall(-53, Some(20), None), quicker_still()];
        let on = (third.as_str(), &badly, -55, CH52);
        let minute = start + Duration::from_secs(60);
        let waits = chooser.choose(minute, on, &near, false);
        // A second past the span, so it holds on any comparison.
        let past_span = minute + SPAN + Duration::from_secs(1);
        let spanned = chooser.choose(past_span, on, &near, false);
        let settled = start + SETTLE + Duration::from_secs(1);
        let next = chooser.choose(settled, on, &near, false);

        assert_eq!(
            waits,
            Choice::Stay,
            "not back where it began, so it settles"
        );
        assert_eq!(spanned, Choice::Stay, "held past a span, as a settle");
        assert_eq!(
            next,
            Choice::Try(quicker_still().point),
            "and tries once settled"
        );
    }

    #[test]
    fn a_move_that_stood_then_ended_where_it_began_still_settles() {
        let mut chooser = after_a_bad_minute(DESK);
        // A settle on, so a span's hold is an instant on any clock and
        // can't pass as a full settle.
        let start = Instant::now() + SETTLE;
        let badly = carried(119, 0.5);
        let well = carried(15, 0.0);
        let hall_alone = [hall(-53, Some(20), None)];
        chooser.choose(start, (DESK, &badly, -50, CH52), &hall_alone, false);
        let minute = start + Duration::from_secs(60);
        let stood = chooser.choose(minute, (HALL, &well, -50, CH52), &hall_alone, false);

        // The hall gone, the card rejoined the desk on its own.
        let near = [quicker_still()];
        let rejoined = minute + Duration::from_secs(60);
        let waits = chooser.choose(rejoined, (DESK, &badly, -50, CH52), &near, false);
        // A second past the span, so it holds on any comparison.
        let spanned = rejoined + SPAN + Duration::from_secs(1);
        let held = chooser.choose(spanned, (DESK, &badly, -50, CH52), &near, false);
        let settled = start + SETTLE + Duration::from_secs(1);
        let next = chooser.choose(settled, (DESK, &badly, -50, CH52), &near, false);

        assert_eq!(stood, Choice::Stay, "on trial on the hall");
        assert_eq!(held, Choice::Stay, "held past a span, as a settle");
        assert_eq!(
            next,
            Choice::Try(quicker_still().point),
            "and tries once settled"
        );
        assert_eq!(
            waits,
            Choice::Stay,
            "the move stood, so its settle holds however the card got back"
        );
    }

    #[test]
    fn a_card_on_neither_point_of_its_move_is_judged_on_neither() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let worse = carried(200, 1.0);
        let point = elsewhere().point;
        let on = (point.as_str(), &worse, -55, CH52);
        let judged = chooser.choose(start + TRIAL, on, &near, false);

        assert_eq!(judged, Choice::Stay, "a third's figures judge nothing");
        assert_eq!(chooser.unreached(), Some(HALL.into()), "unreached");
    }

    #[test]
    fn a_move_on_trial_is_not_judged_on_one_bad_minute() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let worse = carried(200, 1.0);
        let minute = start + Duration::from_secs(60);
        let early = chooser.choose(minute, (HALL, &worse, -53, CH52), &[], false);
        let judged = chooser.choose(start + TRIAL, (HALL, &worse, -53, CH52), &[], false);

        assert_eq!(early, Choice::Stay, "a minute is no trial");
        assert_eq!(judged, Choice::Back(DESK.into()), "worse once tried");
    }

    #[test]
    fn a_second_back_on_the_usual_within_the_hour_keeps_the_point_off() {
        let (mut chooser, again) = hall_kept_off_on_the_usual(Instant::now());
        let slow = Measured {
            moving: None,
            ..carried(76, 0.0)
        };
        let around = around_the_desk();
        // A merely bad link is left on its second bad minute, so the third
        // try would follow one on the desk.
        let primed = SETTLE.saturating_sub(Duration::from_secs(60));
        chooser.choose(again + primed, (DESK, &slow, -50, CH52), &around, false);
        let third = chooser.choose(again + SETTLE, (DESK, &slow, -50, CH52), &around, false);
        let kept = chooser.keeps_off(HALL, again + HOUR_LESS_A_SECOND);
        let ended = chooser.keeps_off(HALL, again + SPURNED);

        assert!(kept, "so kept off to the hour's last second");
        assert!(!ended, "and no longer");
        assert_eq!(
            third,
            Choice::Stay,
            "the hall kept off, it stays where it is"
        );
    }

    /// What a card idle at the desk hears: the desk, which usually carries
    /// 14 ms, and the hall.
    fn around_the_desk() -> [Other; 2] {
        around(DESK)
    }

    /// What a card idle at this access point hears: it, which usually
    /// carries 14 ms, and the hall.
    fn around(point: &str) -> [Other; 2] {
        let usual = Measured {
            moving: None,
            ..carried(14, 0.0)
        };
        let here = Other {
            point: point.into(),
            ..desk_at(-50, Some(usual))
        };
        [here, hall(-45, Some(20), None)]
    }

    /// An access point on the stairs, heard as the desk is.
    const STAIRS: &str = "6c:ae:f6:c1:84:2a";

    /// A chooser gone back from the hall to the desk twice within the hour
    /// for carrying worse only than the desk usually does, so keeping the
    /// hall off, and when the second back was. The desk's span has grown a
    /// minute since, as a choice every minute grows it.
    fn hall_kept_off_on_the_usual(start: Instant) -> (Chooser, Instant) {
        let (mut chooser, again) = back_twice_on_the_usual(start);
        let slow = Measured {
            moving: None,
            ..carried(76, 0.0)
        };
        let minute = Duration::from_secs(60);
        let on = (DESK, &slow, -50, CH52);
        chooser.choose(again + minute, on, &around_the_desk(), false);
        assert!(chooser.keeps_off(HALL, again), "the hall is kept off");
        (chooser, again)
    }

    /// A chooser that has just sent the card back from the hall to the
    /// desk a second time within the hour for carrying worse only than the
    /// desk usually does, why not yet said, and when.
    pub(crate) fn back_twice_on_the_usual(start: Instant) -> (Chooser, Instant) {
        back_twice_from(start, DESK)
    }

    /// A chooser whose card, on a desk failing past the hall's keep-off,
    /// tried the hall, which stood carrying as the desk usually does, so
    /// freed it, that not yet said.
    pub(crate) fn hall_freed(start: Instant) -> Chooser {
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };
        let quick = Measured {
            moving: None,
            ..carried(20, 0.0)
        };
        let around = around_the_desk();
        let (mut chooser, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let tried = chooser.choose(settled, (DESK, &failing, -50, CH52), &around, false);
        assert_eq!(
            tried,
            Choice::Try(HALL.into()),
            "the desk failing tries the hall"
        );
        chooser.yielded();
        chooser.choose(settled + TRIAL, (HALL, &quick, -45, CH52), &around, false);
        chooser
    }

    /// The same, from this access point rather than the desk.
    fn back_twice_from(start: Instant, from: &str) -> (Chooser, Instant) {
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let (slow, there) = (unloaded(76), unloaded(50));
        let around = around(from);
        let mut chooser = after_a_bad_minute(from);
        let mut on = |at, point, measured: &Measured| {
            chooser.choose(at, (point, measured, -50, CH52), &around, false)
        };
        let back = start + TRIAL;
        let again = back + SETTLE + TRIAL;
        let minute = Duration::from_secs(60);
        // A merely bad link is left on its second bad minute, so each
        // retry follows one on the desk.
        let primed = SETTLE.saturating_sub(minute);
        let tried = on(start, from, &slow);
        let first = on(back, HALL, &there);
        on(back + primed, from, &slow);
        let retried = on(back + SETTLE, from, &slow);
        let second = on(again, HALL, &there);
        assert_eq!(tried, Choice::Try(HALL.into()), "the hall is tried");
        assert_eq!(first, Choice::Back(from.into()), "and gone back from");
        assert_eq!(retried, Choice::Try(HALL.into()), "tried again, settled");
        assert_eq!(
            second,
            Choice::Back(from.into()),
            "and gone back from again"
        );
        (chooser, again)
    }

    #[test]
    fn a_failing_link_may_leave_for_a_point_kept_off_only_on_the_usual() {
        let start = Instant::now();
        let around = around_the_desk();
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };
        let minute = Duration::from_secs(60);
        let desk = |chooser: &mut Chooser, at| {
            chooser.choose(at, (DESK, &failing, -50, CH52), &around, false)
        };
        let hall = |chooser: &mut Chooser, at, measured: &Measured| {
            chooser.choose(at, (HALL, measured, -45, CH52), &around, false)
        };

        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };

        let (mut early, again) = hall_kept_off_on_the_usual(start);
        let young = desk(
            &mut early,
            again + SPAN.saturating_sub(Duration::from_secs(1)),
        );
        let yielded_young = early.yielded();
        let whole = desk(&mut early, again + SPAN);
        let yielded_whole = early.yielded();

        let (mut stood, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        desk(&mut stood, settled);
        let judged = hall(&mut stood, settled + TRIAL, &unloaded(20));
        let still_kept = stood.keeps_off(HALL, settled + TRIAL);

        let (mut less_bad, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        desk(&mut less_bad, settled);
        let less = hall(&mut less_bad, settled + TRIAL, &unloaded(50));
        let kept_still = less_bad.keeps_off(HALL, settled + TRIAL);

        let (mut worse, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        desk(&mut worse, settled);
        let gone_bad = Measured {
            moving: None,
            ..carried(600, 40.0)
        };
        let back = hall(&mut worse, settled + minute, &gone_bad);
        // Its span on the desk whole again, so the hall would be tried if
        // the keep-off yielded.
        desk(&mut worse, settled + minute * 2);
        let retried = desk(&mut worse, settled + minute * 2 + SPAN);

        assert_eq!(
            young,
            Choice::Stay,
            "a desk failing to the last second of the span after the back keeps off the hall"
        );
        assert_eq!(yielded_young, None, "saying no keep-off gave way");
        assert_eq!(
            whole,
            Choice::Try(HALL.into()),
            "a desk failing a whole span since the back leaves for the hall, which carried \
             better than it"
        );
        assert_eq!(
            yielded_whole,
            Some(HALL.into()),
            "saying the keep-off gave way"
        );
        assert_eq!(judged, Choice::Stay, "the hall at 20 ms stands");
        assert!(
            !still_kept,
            "and, carrying as the desk usually does, isn't kept off"
        );
        assert_eq!(less, Choice::Stay, "the hall at 50 ms stands too");
        assert!(
            kept_still,
            "but, carrying only better than a failing desk, stays kept off"
        );
        assert_eq!(
            back,
            Choice::Back(DESK.into()),
            "the hall failing worse goes back"
        );
        assert_eq!(
            retried,
            Choice::Stay,
            "kept off now for carrying worse than the failing desk, the hall isn't tried again"
        );
    }

    #[test]
    fn a_trial_standing_frees_its_point_only_carrying_as_a_learned_usual() {
        let start = Instant::now();
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };

        let (mut unlearned, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let unknown = [desk_at(-50, None), hall(-45, Some(20), None)];
        let tried_unlearned =
            unlearned.choose(settled, (DESK, &failing, -50, CH52), &unknown, false);
        let said_yielded = unlearned.yielded();
        let quick = Measured {
            moving: None,
            ..carried(20, 0.0)
        };
        let there = (HALL, &quick, -45, CH52);
        let unlearned_stands = unlearned.choose(settled + TRIAL, there, &unknown, false);
        let kept_unlearned = unlearned.keeps_off(HALL, settled + TRIAL);

        let around = around_the_desk();
        let (mut lossy, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let tried_lossy = lossy.choose(settled, (DESK, &failing, -50, CH52), &around, false);
        // Losing past `LOSSY` but short of `FAILING`, so only the freeing
        // rule's own bar keeps it.
        let losing = Measured {
            moving: None,
            ..carried(20, 5.0)
        };
        lossy.choose(settled + TRIAL, (HALL, &losing, -45, CH52), &around, false);
        let kept_lossy = lossy.keeps_off(HALL, settled + TRIAL);
        let freed_lossy = lossy.freed();

        // Under load, the hall carries as the desk usually does idle, but
        // that says nothing of the usual under load.
        let (mut downloading, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let on = (DESK, &failing, -50, CH52);
        let tried_loaded = downloading.choose(settled, on, &around, false);
        let loaded = carried(20, 0.0);
        downloading.choose(settled + TRIAL, (HALL, &loaded, -45, CH52), &around, false);
        let kept_loaded = downloading.keeps_off(HALL, settled + TRIAL);

        // Few echoes sent read few lost, which says nothing either.
        let (mut unechoed, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let on = (DESK, &failing, -50, CH52);
        let tried_unechoed = unechoed.choose(settled, on, &around, false);
        let few = Measured {
            echoes: ECHOED - 1,
            ..quick.clone()
        };
        unechoed.choose(settled + TRIAL, (HALL, &few, -45, CH52), &around, false);
        let kept_unechoed = unechoed.keeps_off(HALL, settled + TRIAL);

        // Echoes enough, just, say so, and the hall is freed.
        let (mut echoed, again) = hall_kept_off_on_the_usual(start);
        let settled = again + SETTLE;
        let tried_echoed = echoed.choose(settled, on, &around, false);
        let enough = Measured {
            echoes: ECHOED,
            ..quick.clone()
        };
        echoed.choose(settled + TRIAL, (HALL, &enough, -45, CH52), &around, false);
        let freed = !echoed.keeps_off(HALL, settled + TRIAL);
        let said_freed = echoed.freed();
        let said_again = echoed.freed();

        assert_eq!(
            unlearned_stands,
            Choice::Stay,
            "the hall stands with no usual"
        );
        assert!(
            kept_unlearned,
            "but with no usual to weigh, says nothing of it, so the keep-off holds"
        );
        assert!(
            kept_lossy,
            "a hall quick but losing 5% didn't carry as the desk usually does"
        );
        assert_eq!(freed_lossy, None, "so says nothing freed");
        assert!(
            kept_loaded,
            "nor did one quick under load, the usual learned idle"
        );
        assert!(kept_unechoed, "nor one echoing too little to say");
        assert!(freed, "one echoing just enough frees the hall");
        assert_eq!(said_freed, Some((HALL.into(), DESK.into())), "and says so");
        assert_eq!(said_again, None, "once");
        let tries = [
            tried_unlearned,
            tried_lossy,
            tried_loaded,
            tried_unechoed,
            tried_echoed,
        ];
        assert!(
            tries.iter().all(|tried| *tried == Choice::Try(HALL.into())),
            "each failing desk tries the hall through its keep-off: {tries:?}"
        );
        assert_eq!(
            said_yielded,
            Some(HALL.into()),
            "saying the keep-off gave way"
        );
    }

    #[test]
    fn a_keep_off_earned_from_another_point_holds_however_well_the_point_stands() {
        let start = Instant::now();
        let (mut chooser, again) = back_twice_from(start, STAIRS);
        assert!(chooser.keeps_off(HALL, again), "the hall is kept off");
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };
        let quick = Measured {
            moving: None,
            ..carried(20, 0.0)
        };
        let around = around_the_desk();
        // wpa_supplicant roams the card to the desk, whose link then fails
        // a whole span.
        let roamed = again + SETTLE;
        let judged = roamed + SPAN + TRIAL;
        chooser.choose(roamed, (DESK, &failing, -50, CH52), &around, false);
        let tried = chooser.choose(roamed + SPAN, (DESK, &failing, -50, CH52), &around, false);
        let stands = chooser.choose(judged, (HALL, &quick, -45, CH52), &around, false);
        let kept = chooser.keeps_off(HALL, judged);
        let said_freed = chooser.freed();

        assert_eq!(
            tried,
            Choice::Try(HALL.into()),
            "a desk failing a whole span tries the hall through its keep-off"
        );
        assert_eq!(
            stands,
            Choice::Stay,
            "the hall, as quick as the desk usually is, stands"
        );
        assert!(
            kept,
            "but the keep-off the stairs' usual earned holds, as the desk's says nothing of it"
        );
        assert_eq!(said_freed, None, "and nothing is said freed");
    }

    #[test]
    fn a_trial_standing_once_its_keep_off_ran_out_says_nothing_freed() {
        let start = Instant::now();
        let (mut chooser, again) = hall_kept_off_on_the_usual(start);
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };
        let quick = Measured {
            moving: None,
            ..carried(20, 0.0)
        };
        let around = around_the_desk();
        let late = again + SPURNED.saturating_sub(Duration::from_secs(60));
        let tried = chooser.choose(late, (DESK, &failing, -50, CH52), &around, false);
        let stands = chooser.choose(late + TRIAL, (HALL, &quick, -45, CH52), &around, false);

        assert_eq!(
            tried,
            Choice::Try(HALL.into()),
            "a desk failing in the keep-off's last minute tries the hall through it"
        );
        assert_eq!(
            stands,
            Choice::Stay,
            "the hall, as quick as the desk usually is, stands"
        );
        assert_eq!(
            chooser.freed(),
            None,
            "but its keep-off had run out, so nothing is said freed"
        );
    }

    #[test]
    fn a_failing_link_new_on_a_point_waits_a_whole_span_for_a_keep_off_to_give_way() {
        let start = Instant::now();
        let (mut chooser, again) = hall_kept_off_on_the_usual(start);
        let failing = Measured {
            moving: None,
            ..carried(400, 15.0)
        };
        let stairs = Other {
            point: STAIRS.into(),
            ..desk_at(-50, None)
        };
        let around = [stairs, hall(-45, Some(20), None)];
        // wpa_supplicant roams the card to the stairs within the span the
        // back holds the desk, so only the point it went back to would be
        // held, and only the stairs' span there being young keeps it.
        let roamed = again + Duration::from_secs(61);
        let on = (STAIRS, &failing, -50, CH52);
        let landed = chooser.choose(roamed, on, &around, false);
        let held = chooser.held();
        let young = chooser.choose(
            roamed + SPAN.saturating_sub(Duration::from_secs(1)),
            on,
            &around,
            false,
        );
        let whole = chooser.choose(roamed + SPAN, on, &around, false);

        assert!(
            !matches!(landed, Choice::Try(_)),
            "the stairs failing as they're landed on keep off the hall: {landed:?}"
        );
        assert!(
            !matches!(young, Choice::Try(_)),
            "the stairs failing less than a whole span keep off the hall: {young:?}"
        );
        assert_eq!(held, None, "held by no back, as the stairs aren't the desk");
        assert_eq!(
            whole,
            Choice::Try(HALL.into()),
            "failing a whole span, they leave for it"
        );
    }

    #[test]
    fn a_failing_link_waits_a_span_only_on_the_point_just_gone_back_to() {
        let start = Instant::now();
        let now = start + TRIAL;
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let failing = carried(400, 15.0);
        let around = [desk_at(-50, None), hall(-45, Some(20), None), elsewhere()];
        // Gone back from the hall for carrying worse than the desk did, so
        // the hall is kept off and only the third may be tried.
        let back = || {
            let mut chooser = after_a_bad_minute(DESK);
            let left = unloaded(45);
            let tried = chooser.choose(start, (DESK, &left, -50, CH52), &around[..2], false);
            assert_eq!(tried, Choice::Try(HALL.into()), "the hall is tried");
            let there = unloaded(60);
            let judged = chooser.choose(now, (HALL, &there, -45, CH52), &around[..2], false);
            assert_eq!(judged, Choice::Back(DESK.into()), "and gone back from");
            chooser
        };
        let minute = now + Duration::from_secs(60);
        let mut desk = back();
        let young = desk.choose(minute, (DESK, &failing, -50, CH52), &around, false);
        let said = desk.held();
        let said_again = desk.held();
        let whole = desk.choose(now + SPAN, (DESK, &failing, -50, CH52), &around, false);
        let said_whole = desk.held();
        // The roam back refused, so the card is still on the hall.
        let mut stuck = back();
        let still = stuck.choose(minute, (HALL, &failing, -45, CH52), &around, false);
        // Moved on to the desk since, within the span after the back, it's
        // judged as any card there, no longer held.
        let landed_at = minute + Duration::from_secs(30);
        let landed = stuck.choose(landed_at, (DESK, &failing, -50, CH52), &around, false);
        let landed_held = stuck.held();

        assert_eq!(
            young,
            Choice::Stay,
            "a desk failing in the span after the back waits"
        );
        assert_eq!(said, Some(DESK.into()), "and says it's held");
        assert_eq!(said_again, None, "once");
        assert_eq!(said_whole, None, "and not once it leaves");
        assert_eq!(
            whole,
            Choice::Try(elsewhere().point),
            "a whole span since the back, it leaves for the third"
        );
        assert_eq!(
            still,
            Choice::Try(DESK.into()),
            "a card not back on the desk isn't held on the hall, and tries the loudest it hears"
        );
        assert_eq!(
            landed,
            Choice::Try(elsewhere().point),
            "a card moved on since the back isn't held where it went back to"
        );
        assert_eq!(landed_held, None, "nor said held");
    }

    #[test]
    fn a_back_on_the_usual_an_hour_after_the_last_keeps_nothing_off() {
        let start = Instant::now();
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let (slow, there) = (unloaded(76), unloaded(50));
        let mut chooser = after_a_bad_minute(DESK);
        let around = [desk_at(-50, Some(unloaded(14))), hall(-45, Some(20), None)];
        let mut on = |at, point, measured: &Measured| {
            chooser.choose(at, (point, measured, -50, CH52), &around, false)
        };
        let back = start + TRIAL;
        // Retried so its trial ends just as the first back's hour does.
        let retry = back + SPURNED.saturating_sub(TRIAL);
        let primed = retry
            .checked_sub(Duration::from_secs(60))
            .expect("after the start");

        let tried = on(start, DESK, &slow);
        let first = on(back, HALL, &there);
        on(primed, DESK, &slow);
        let retried = on(retry, DESK, &slow);
        let second = on(back + SPURNED, HALL, &there);
        let kept = chooser.keeps_off(HALL, back + SPURNED);
        let why = chooser.went_back();

        assert_eq!(tried, Choice::Try(HALL.into()), "the hall is tried");
        assert_eq!(first, Choice::Back(DESK.into()), "and gone back from");
        assert_eq!(
            retried,
            Choice::Try(HALL.into()),
            "tried again near the hour"
        );
        assert_eq!(
            second,
            Choice::Back(DESK.into()),
            "and gone back from again"
        );
        assert!(
            matches!(why, Some(Against::Usually(_))),
            "on the usual again: {why:?}"
        );
        assert!(
            !kept,
            "an hour on, it counts as a first, so nothing's kept off"
        );
    }

    #[test]
    fn a_trial_only_less_bad_than_one_slow_minute_goes_back() {
        let start = Instant::now();
        let unloaded = |p95, lost| Measured {
            moving: None,
            ..carried(p95, lost)
        };
        let now = start + TRIAL;
        // Tried and judged through `choose`, as the night's trials were:
        // the choice a trial past the start, why, how long the hall is kept
        // off, and the chooser after.
        let judged = |left: Measured, usual: Option<Measured>, there: Measured| {
            let mut chooser = after_a_bad_minute(DESK);
            let around = [desk_at(-50, usual), hall(-45, Some(20), None)];
            let tried = chooser.choose(start, (DESK, &left, -50, CH52), &around, false);
            assert_eq!(tried, Choice::Try(HALL.into()), "the hall is tried");
            let judged = chooser.choose(now, (HALL, &there, -45, CH52), &around, false);
            assert!(chooser.trial.is_none(), "the trial ended, judged");
            let kept = chooser.keeps_off(HALL, now + HOUR_LESS_A_SECOND);
            (judged, chooser.went_back(), kept, chooser)
        };

        let slow = unloaded(76, 0.0);
        let (back, why, kept, mut chooser) =
            judged(slow.clone(), Some(unloaded(14, 0.0)), unloaded(50, 0.0));
        assert_eq!(back, Choice::Back(DESK.into()), "as on 19 Nov");
        let usual = Measured {
            p95: Some(Duration::from_millis(14)),
            ..slow.clone()
        };
        assert_eq!(why, Some(Against::Usually(usual)), "the usual judged");
        assert!(!kept, "not kept off, as the desk may be slow");
        let failing = carried(400, 15.0);
        let around = [desk_at(-50, None), hall(-45, Some(20), None)];
        let minute = now + Duration::from_secs(60);
        assert_eq!(
            chooser.choose(minute, (DESK, &failing, -50, CH52), &around, false),
            Choice::Stay,
            "a desk failing in its first span after the back waits it out"
        );
        assert_eq!(
            chooser.choose(now + SPAN, (DESK, &failing, -50, CH52), &around, false),
            Choice::Try(HALL.into()),
            "a desk failing a whole span since the back, within the settle, is left for the hall"
        );

        // Gone back on the usual alone, the settle from the back holds the
        // card on a desk still slow, past the settle from the first move.
        let (_, _, _, mut held) = judged(slow.clone(), Some(unloaded(14, 0.0)), unloaded(50, 0.0));
        let mut on_the_desk = |at| held.choose(at, (DESK, &slow, -50, CH52), &around, false);
        // A slow minute first, so the next is a second, which only the
        // settle holds.
        on_the_desk(now + SETTLE.saturating_sub(Duration::from_secs(61)));
        let held = on_the_desk(now + SETTLE.saturating_sub(Duration::from_secs(1)));
        assert_eq!(held, Choice::Stay, "held to the settle's last second");
        assert_eq!(
            on_the_desk(now + SETTLE),
            Choice::Try(HALL.into()),
            "the hall is tried again once the back has settled"
        );

        let stood = judged(unloaded(76, 0.0), None, unloaded(50, 0.0));
        assert_eq!(
            (stood.0, stood.1, stood.2),
            (Choice::Stay, None, false),
            "nothing learned: 50 beats 76"
        );

        let (back, why, kept, mut kept_worse) = judged(
            unloaded(45, 0.0),
            Some(unloaded(70, 0.0)),
            unloaded(60, 0.0),
        );
        assert_eq!(back, Choice::Back(DESK.into()), "60 is worse than 45");
        let left = unloaded(45, 0.0);
        assert_eq!(why, Some(Against::Worse(left)), "the quicker judged");
        assert!(kept, "worse than it left, kept off an hour");
        // The span since the back is whole by the settle, so the desk's
        // failing link would leave through a keep-off that yields.
        let at = now + SETTLE;
        let failed = kept_worse.choose(at, (DESK, &failing, -50, CH52), &around, false);
        assert_eq!(
            failed,
            Choice::Stay,
            "a point that carried worse than the desk stays kept off, the desk failing or not"
        );

        let lossy = judged(
            unloaded(76, 1.5),
            Some(unloaded(14, 0.0)),
            unloaded(20, 3.5),
        );
        assert_eq!(lossy.0, Choice::Stay, "loss is judged by the minute's 1.5%");

        let edge = judged(
            unloaded(76, LOSSY),
            Some(unloaded(14, 0.0)),
            unloaded(30, 0.0),
        );
        assert_eq!(
            edge.0,
            Choice::Back(DESK.into()),
            "a minute losing LOSSY is slow"
        );
        let past = judged(
            unloaded(76, LOSSY + 0.1),
            Some(unloaded(14, 0.0)),
            unloaded(30, 0.0),
        );
        assert_eq!(
            past.0,
            Choice::Stay,
            "a point losing past LOSSY isn't gone back to"
        );
    }

    #[test]
    fn a_trial_is_judged_by_what_its_point_usually_carries_only_like_with_like() {
        let start = Instant::now();
        // A usual says nothing of its load, as learned figures don't.
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let moving = |p95, moving| Measured {
            moving: Some(moving),
            ..carried(p95, 0.0)
        };
        let judged = |before, usual, there: &Measured| {
            judged_off_the_desk(start, (before, usual), there, false)
        };

        let quick = judged(unloaded(30), Some(unloaded(14)), &unloaded(25));
        assert_eq!(
            quick,
            (None, false),
            "a move off a quick minute isn't for slowness"
        );
        let at_slow = judged(unloaded(40), Some(unloaded(14)), &unloaded(30));
        assert_eq!(at_slow, (None, false), "a minute at SLOW isn't slow");
        let past_slow = judged(unloaded(41), Some(unloaded(14)), &unloaded(30));
        assert!(
            matches!(past_slow.0, Some(Against::Usually(_))),
            "a minute past SLOW is judged by its usual: {past_slow:?}"
        );
        let unlike = judged(moving(76, 40), Some(unloaded(14)), &unloaded(30));
        assert_eq!(
            unlike,
            (None, false),
            "a loaded minute doesn't judge an idle trial"
        );
        let unlike = judged(unloaded(76), Some(unloaded(14)), &moving(30, 40));
        assert_eq!(
            unlike,
            (None, false),
            "an idle minute doesn't judge a loaded one"
        );
        let silent = Measured {
            echoes: 0,
            p50: None,
            p95: None,
            ..unloaded(76)
        };
        let silent = judged(silent, Some(unloaded(14)), &unloaded(30));
        assert_eq!(
            silent,
            (None, false),
            "a minute with no echo isn't slow, it's unreached"
        );

        let learned = judged(moving(163, 40), Some(unloaded(21)), &moving(100, 40));
        let usual = Measured {
            p95: Some(Duration::from_millis(21)),
            ..moving(163, 40)
        };
        let usually = (Some(Against::Usually(usual)), false);
        assert_eq!(
            learned, usually,
            "a trial made and over under load is judged by its usual"
        );

        let cut = judged(moving(136, 50), Some(unloaded(20)), &moving(40, 30));
        let slowed = (Some(Against::Slowed(50)), true);
        assert_eq!(
            cut, slowed,
            "a download cut is kept off an hour, quick usual or not"
        );
    }

    /// A trial off the desk to the hall, made off this minute with this
    /// usual, judged a trial past its start on what the hall carried: why
    /// it went back, and whether the hall is kept off near the hour's end.
    fn judged_off_the_desk(
        start: Instant,
        (before, usual): (Measured, Option<Measured>),
        there: &Measured,
        banded: bool,
    ) -> (Option<Against>, bool) {
        let mut chooser = after_a_bad_minute(DESK);
        chooser.trial = Some(Trial {
            from: DESK.into(),
            to: HALL.into(),
            before,
            usual,
            moved_before: None,
            stood: false,
            banded,
            widened: false,
        });
        chooser.moved = Some(start);
        let now = start + TRIAL;
        chooser.judge(now, (HALL, there), (Carrying::Enough, false, false));
        let kept = chooser.keeps_off(HALL, now + HOUR_LESS_A_SECOND);
        (chooser.went_back(), kept)
    }

    #[test]
    fn a_usual_judges_only_a_minute_echoing_enough_and_never_a_banded_trial() {
        let start = Instant::now();
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let sparse = |echoes| Measured {
            echoes,
            ..unloaded(41)
        };
        let usual = || Some(unloaded(14));
        let judged =
            |before, banded| judged_off_the_desk(start, (before, usual()), &unloaded(30), banded);

        let few = judged(sparse(ECHOED - 1), false);
        let enough = judged(sparse(ECHOED), false);
        // A trial made for a wider band's room, off a minute using the
        // narrower one's, goes back only carrying badly, never for what
        // the narrower one usually carries.
        let banded =
            judged_off_the_desk(start, (carried(76, 0.0), usual()), &carried(30, 0.0), true);

        assert_eq!(
            few,
            (None, false),
            "a minute echoing short of ECHOED gives too few to weigh"
        );
        assert!(
            matches!(enough.0, Some(Against::Usually(_))),
            "one echoing ECHOED is judged by its usual: {enough:?}"
        );
        assert_eq!(
            banded,
            (None, false),
            "a banded trial isn't judged by the usual"
        );
    }

    /// Just short of `SPURNED`, where a point kept off an hour still is.
    const HOUR_LESS_A_SECOND: Duration = SPURNED.saturating_sub(Duration::from_secs(1));

    #[test]
    fn a_back_on_the_usual_counts_toward_a_second_only_for_the_same_move_since_one_stood() {
        let start = Instant::now();
        let unloaded = |p95| Measured {
            moving: None,
            ..carried(p95, 0.0)
        };
        let stairs = elsewhere().point;
        let mut chooser = after_a_bad_minute(DESK);
        // A trial to the hall off a slow minute where the point left
        // usually carries 14 ms, made this many trials past the start and
        // judged a trial later: whether it went back, and whether the hall
        // is kept off.
        let mut judged = |trials: u32, from: &str, there| {
            let moved = start + TRIAL * trials;
            chooser.trial = Some(Trial {
                from: from.into(),
                to: HALL.into(),
                before: unloaded(76),
                usual: Some(unloaded(14)),
                moved_before: None,
                stood: false,
                banded: false,
                widened: false,
            });
            chooser.moved = Some(moved);
            let now = moved + TRIAL;
            chooser.judge(
                now,
                (HALL, &unloaded(there)),
                (Carrying::Enough, false, false),
            );
            let back = chooser.went_back().is_some();
            (back, chooser.keeps_off(HALL, now))
        };

        let first = judged(0, DESK, 50);
        let other_move = judged(2, &stairs, 50);
        let stood = judged(4, DESK, 10);
        let after_standing = judged(6, DESK, 50);
        let second = judged(8, DESK, 50);

        assert_eq!(first, (true, false), "a first back keeps nothing off");
        assert_eq!(
            other_move,
            (true, false),
            "a back to the hall off another point counts apart"
        );
        assert_eq!(stood, (false, false), "a trial at 10 ms stands");
        assert_eq!(
            after_standing,
            (true, false),
            "a back after one stood is a first again"
        );
        assert_eq!(
            second,
            (true, true),
            "a second of the same move keeps it off"
        );
    }

    #[test]
    fn a_move_on_trial_failing_is_judged_at_once() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let failing = carried(400, 15.0);
        let minute = start + Duration::from_secs(60);
        let judged = chooser.choose(minute, (HALL, &failing, -53, CH52), &[], false);

        assert_eq!(judged, Choice::Back(DESK.into()), "15% lost is judged");
    }

    #[test]
    fn a_move_on_trial_carrying_badly_and_worse_past_its_arrival_goes_back() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        // Its video echoes held as on 23 Nov 2026, the card moving 15 Mbit/s.
        let held = Measured {
            moving: Some(15),
            ..carried(754, 1.9)
        };
        // The second falls a moment late, as the daemon's choices may.
        let minutes = [60_000, 120_010].map(|millis| {
            let now = start + Duration::from_millis(millis);
            chooser.choose(now, (HALL, &held, -53, CH52), &[], false)
        });
        let third = start + Duration::from_secs(180);
        let judged = chooser.choose(third, (HALL, &held, -53, CH52), &[], false);

        let stays = vec![Choice::Stay; 2];
        assert_eq!(minutes.to_vec(), stays, "its arrival is in both spans");
        assert_eq!(
            judged,
            Choice::Back(DESK.into()),
            "a span past it is worse too"
        );
        assert!(chooser.keeps_off(HALL, third), "and kept off");
    }

    #[test]
    fn a_move_on_trial_carrying_badly_only_as_it_arrived_stays_on_trial() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        // Its first minute's video echoes held, as one trial's did on 23 Nov
        // 2026, so the second's span, holding the first, reads as badly,
        // and the second falls a moment late, as the daemon's choices may.
        let arriving = carried(823, 0.5);
        let minutes = [
            (60_000, arriving.clone()),
            (120_010, arriving),
            (180_000, carried(30, 0.0)),
        ]
        .map(|(millis, measured)| {
            let now = start + Duration::from_millis(millis);
            chooser.choose(now, (HALL, &measured, -53, CH52), &[], false)
        });
        let worse = carried(200, 1.0);
        let judged = chooser.choose(start + TRIAL, (HALL, &worse, -53, CH52), &[], false);

        let stays = vec![Choice::Stay; 3];
        assert_eq!(minutes.to_vec(), stays, "one bad arrival is no trial");
        assert_eq!(judged, Choice::Back(DESK.into()), "and it's still on trial");
    }

    #[test]
    fn a_move_on_trial_carrying_badly_one_minute_past_its_arrival_stays_on_trial() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let quick = carried(30, 0.0);
        let minutes = [(60, quick.clone()), (120, quick), (180, carried(754, 1.9))].map(
            |(seconds, measured)| {
                let now = start + Duration::from_secs(seconds);
                chooser.choose(now, (HALL, &measured, -53, CH52), &[], false)
            },
        );

        let stays = vec![Choice::Stay; 3];
        assert_eq!(
            minutes.to_vec(),
            stays,
            "one bad minute running is no trial"
        );
    }

    #[test]
    fn a_move_on_trial_carrying_badly_but_no_worse_waits_out_its_trial() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let as_badly = carried(125, 0.5);
        let minutes = [60, 120, 180].map(|seconds| {
            let now = start + Duration::from_secs(seconds);
            chooser.choose(now, (HALL, &as_badly, -53, CH52), &[], false)
        });
        let worse = carried(200, 1.0);
        let judged = chooser.choose(start + TRIAL, (HALL, &worse, -53, CH52), &[], false);

        let stays = vec![Choice::Stay; 3];
        assert_eq!(minutes.to_vec(), stays, "no worse, it stays on trial");
        assert_eq!(judged, Choice::Back(DESK.into()), "judged at its end");
    }

    #[test]
    fn a_move_never_made_sends_the_card_back_nowhere() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = [hall(-53, Some(20), None)];
        let badly = carried(119, 0.5);
        let tried = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is asked");

        let worse = carried(200, 1.0);
        let judged = chooser.choose(start + TRIAL, (DESK, &worse, -50, CH52), &near, false);

        assert_eq!(judged, Choice::Stay, "the card never left to go back");
    }

    /// The desk's 5 GHz radio, heard this loud.
    fn desk_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: DESK.into(),
            signal,
            busy: Some(20),
            frequency: CH52,
            measured,
        }
    }

    /// The desk's 2.4 GHz radio, heard this loud.
    fn near_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: NEAR.into(),
            signal,
            busy: Some(20),
            frequency: CH6,
            measured,
        }
    }

    #[test]
    fn a_card_that_looked_for_another_tries_what_it_heard_at_once() {
        let mut chooser = after_a_bad_minute(DESK);
        let now = Instant::now();
        let later = now + Duration::from_secs(12);
        let slow = carried(106, 0.0);
        let on = (DESK, &slow, -45, CH52);

        let looks = chooser.choose(now, on, &[], false);
        chooser.looked_over(later);
        let again = chooser.choose(later, on, &[hall(-46, Some(20), None)], false);

        assert_eq!(looks, Choice::Look, "it hears no other");
        assert_eq!(again, Choice::Try(HALL.into()), "tries what it heard");
    }

    #[test]
    fn a_card_chosen_for_again_after_a_look_counts_its_bad_minute_once() {
        let mut chooser = Chooser::default();
        let now = Instant::now();
        let later = now + Duration::from_secs(12);
        let slow = carried(60, 0.0);
        let on = (DESK, &slow, -45, CH6);
        let heard = [near_at(-40, None)];

        let looks = chooser.choose(now, on, &[], false);
        chooser.looked_over(later);
        let again = chooser.choose(later, on, &heard, false);
        let next = chooser.choose(now + Duration::from_secs(60), on, &heard, false);

        assert_eq!(looks, Choice::LookWider, "it hears no 5 GHz");
        assert_eq!(again, Choice::Stay, "one bad minute");
        assert_eq!(next, Choice::Try(NEAR.into()), "two bad minutes");
    }

    #[test]
    fn a_look_ended_with_no_choice_then_leaves_the_next_minute_as_it_was() {
        let mut chooser = Chooser::default();
        let now = Instant::now();
        let slow = carried(60, 0.0);
        let on = (DESK, &slow, -45, CH6);

        let heard = [near_at(-40, None)];

        chooser.choose(now, on, &[], false);
        chooser.looked_over(now + Duration::from_secs(12));
        let next = chooser.choose(now + Duration::from_secs(60), on, &heard, false);

        assert_eq!(next, Choice::Try(NEAR.into()), "two bad minutes");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_tries_5_ghz_heard_well_enough() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-45, None)], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "2.4 GHz carries a quarter of what 5 GHz does, however quick"
        );
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_heard_too_faintly_and_looks_for_nearer() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-75, None)], false);

        assert_eq!(choice, Choice::LookWider, "at -75 dBm it would carry worse");
    }

    #[test]
    fn a_faint_card_on_2_4_ghz_keeps_off_5_ghz_fainter_than_the_band_carries() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);

        let on = (NEAR, &well, -72, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-77, None)], false);

        assert_eq!(choice, Choice::LookWider, "-77 dBm is past FAINTEST");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_tries_5_ghz_16_db_fainter_not_17() {
        let well = carried(11, 0.0);
        let on = (NEAR, &well, -36, CH6);

        let within = Chooser::default().choose(Instant::now(), on, &[desk_at(-52, None)], false);
        let past = Chooser::default().choose(Instant::now(), on, &[desk_at(-53, None)], false);

        assert_eq!(within, Choice::Try(DESK.into()), "16 dB is within reach");
        assert_eq!(past, Choice::LookWider, "17 dB is past them");
    }

    #[test]
    fn a_card_beside_its_access_point_on_2_4_ghz_tries_5_ghz_heard_at_50_dbm() {
        let well = carried(11, 0.0);
        let on = (NEAR, &well, -29, CH6);

        let strong = Chooser::default().choose(Instant::now(), on, &[desk_at(-50, None)], false);
        let weaker = Chooser::default().choose(Instant::now(), on, &[desk_at(-51, None)], false);

        assert_eq!(strong, Choice::Try(DESK.into()), "-50 dBm is strong");
        assert_eq!(weaker, Choice::LookWider, "-51 is past -29's reach");
    }

    #[test]
    fn a_card_beside_its_access_point_failing_on_2_4_ghz_tries_5_ghz_at_50_dbm() {
        let failing = carried(20, 18.0);
        let on = (NEAR, &failing, -29, CH6);

        let strong = Chooser::default().choose(Instant::now(), on, &[desk_at(-50, None)], false);
        let weaker = Chooser::default().choose(Instant::now(), on, &[desk_at(-51, None)], false);

        assert_eq!(strong, Choice::Try(DESK.into()), "-50 dBm is strong");
        assert_eq!(weaker, Choice::Stay, "-51 is past -29's reach");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_keeps_off_5_ghz_known_quicker_30_db_fainter() {
        let mut chooser = Chooser::default();
        let well = carried(11, 0.0);
        let known = far_at(-66, Some(carried(9, 0.0)));

        let on = (NEAR, &well, -36, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(choice, Choice::LookWider, "that's another room");
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_45_percent_busy() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let busy = Other {
            busy: Some(45),
            ..desk_at(-45, None)
        };

        let choice = chooser.choose(Instant::now(), (NEAR, &well, -34, CH6), &[busy], false);

        assert_eq!(choice, Choice::Stay, "a busy channel is no more room");
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_known_to_carry_badly_from_here() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let known = desk_at(-45, Some(carried(119, 0.0)));

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(choice, Choice::Stay, "119 ms at p95 is no better band");
    }

    #[test]
    fn a_card_on_2_4_ghz_moves_to_5_ghz_known_to_carry_well_though_slower() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let known = desk_at(-45, Some(carried(20, 0.0)));

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "20 ms at p95 carries well, and on the wider band"
        );
    }

    #[test]
    fn a_card_using_5_ghz_carrying_well_keeps_off_2_4_ghz_however_quick() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let quick = near_at(-34, Some(carried(9, 0.0)));

        let on = (DESK, &middling, -68, CH52);
        let choice = chooser.choose(Instant::now(), on, &[quick], false);

        assert_eq!(choice, Choice::Stay, "its room is used, faint or not");
    }

    #[test]
    fn a_resting_card_on_5_ghz_takes_a_2_4_ghz_radio_known_far_quicker_only_faint() {
        let middling = resting(carried(30, 0.0));
        let quick = [near_at(-34, Some(carried(9, 0.0)))];
        let chosen = |signal| {
            let on = (DESK, &middling, signal, CH52);
            Chooser::default().choose(Instant::now(), on, &quick, false)
        };

        assert_eq!(
            chosen(-45),
            Choice::Stay,
            "heard well, 5 GHz is kept for when it's used"
        );
        assert_eq!(
            chosen(-68),
            Choice::Try(NEAR.into()),
            "faint, its room goes unused"
        );
    }

    #[test]
    fn a_card_slow_now_leaves_however_well_its_point_usually_carries() {
        let mut chooser = Chooser::default();
        let slow = resting(carried(40, 0.0));
        let usual = near_at(-34, Some(carried(12, 0.0)));
        let quicker = desk_at(-45, Some(carried(15, 0.0)));

        let on = (NEAR, &slow, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[usual, quicker], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "15 ms is slower than the usual 12, but far quicker than 40 now"
        );
    }

    #[test]
    fn a_card_on_2_4_ghz_weighs_5_ghz_against_what_its_point_usually_carries() {
        let well = idle(carried(21, 0.0));
        let wider = desk_at(-45, Some(carried(14, 0.0)));
        let usual = near_at(-34, Some(carried(11, 0.0)));
        let on = (NEAR, &well, -34, CH6);

        let weighed = Chooser::default().choose(Instant::now(), on, &[usual, wider.clone()], false);
        let unweighed = Chooser::default().choose(Instant::now(), on, &[wider], false);

        assert_eq!(
            weighed,
            Choice::Stay,
            "5 GHz's 14 ms is slower than the 11 its own usually carries, as at 08:49 on 18 Nov"
        );
        assert_eq!(
            unweighed,
            Choice::Try(DESK.into()),
            "with nothing usual known, its last minute is gone by"
        );
    }

    #[test]
    fn a_card_on_2_4_ghz_slower_than_usual_still_takes_5_ghz_known_quicker_still() {
        let well = idle(carried(21, 0.0));
        let wider = desk_at(-45, Some(carried(10, 0.0)));
        let usual = near_at(-34, Some(carried(11, 0.0)));

        let on = (NEAR, &well, -34, CH6);
        let choice = Chooser::default().choose(Instant::now(), on, &[usual, wider], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "10 ms is no slower than 11"
        );
    }

    #[test]
    fn a_resting_card_on_5_ghz_keeps_off_a_2_4_ghz_radio_a_little_quicker() {
        let mut chooser = Chooser::default();
        let middling = resting(carried(20, 0.0));
        let quick = near_at(-34, Some(carried(11, 0.0)));

        let on = (DESK, &middling, -68, CH52);
        let choice = chooser.choose(Instant::now(), on, &[quick], false);

        assert_eq!(choice, Choice::Stay, "9 ms quicker isn't far");
    }

    #[test]
    fn a_card_moving_25_mbit_s_on_5_ghz_keeps_off_2_4_ghz_however_quick() {
        let (mut more, mut less) = (Chooser::default(), Chooser::default());
        let some = Measured {
            moving: Some(25),
            ..carried(30, 0.0)
        };
        let little = resting(carried(30, 0.0));
        let quick = [near_at(-34, Some(carried(9, 0.0)))];
        let now = Instant::now();

        let kept = more.choose(now, (DESK, &some, -68, CH52), &quick, false);
        let took = less.choose(now, (DESK, &little, -68, CH52), &quick, false);

        assert_eq!(kept, Choice::Stay, "25 Mbit/s may be using its room");
        assert_eq!(took, Choice::Try(NEAR.into()), "24 isn't");
    }

    #[test]
    fn a_card_on_5_ghz_whose_kernel_says_nothing_moved_keeps_off_2_4_ghz() {
        let mut chooser = Chooser::default();
        let unsaid = Measured {
            moving: None,
            ..carried(30, 0.0)
        };
        let quick = near_at(-34, Some(carried(9, 0.0)));

        let on = (DESK, &unsaid, -68, CH52);
        let choice = chooser.choose(Instant::now(), on, &[quick], false);

        assert_eq!(choice, Choice::Stay, "it may be using its room");
    }

    #[test]
    fn a_resting_card_s_move_to_2_4_ghz_holds_until_it_moves_enough_for_5_ghz() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let middling = resting(carried(30, 0.0));
        let quick = [near_at(-34, Some(carried(9, 0.0)))];
        let tried = chooser.choose(start, (DESK, &middling, -68, CH52), &quick, false);
        assert_eq!(
            tried,
            Choice::Try(NEAR.into()),
            "the move is made, 5 GHz faint"
        );
        let slower = [desk_at(-45, Some(middling.clone()))];
        let (quiet, busy) = (resting(carried(9, 0.0)), carried(9, 0.0));
        let some = Measured {
            room: Some(286),
            ..idle(carried(9, 0.0))
        };

        let kept = chooser.choose(start + TRIAL, (NEAR, &quiet, -34, CH6), &slower, false);
        let held = chooser.choose(start + SETTLE, (NEAR, &some, -34, CH6), &slower, false);
        let used = chooser.choose(start + SETTLE, (NEAR, &busy, -34, CH6), &slower, false);

        assert_eq!(kept, Choice::Stay, "9 ms is no worse than 30");
        assert_eq!(held, Choice::Stay, "99 Mbit/s doesn't use 5 GHz's room");
        assert_eq!(used, Choice::Try(DESK.into()), "its room is used now");
    }

    /// The hall's 5 GHz radio as the card on it hears it, saying how busy
    /// its channel is.
    fn the_hall_itself(busy: u8) -> Other {
        Other {
            point: HALL.into(),
            signal: -41,
            busy: Some(busy),
            frequency: CH149,
            measured: None,
        }
    }

    /// What a fresh chooser does on the hall's ch149 radio, carrying this
    /// well with its channel this busy, hearing the desk.
    fn off_the_hall(p95: u64, busy: u8, desk: Other) -> Choice {
        let measured = carried(p95, 0.0);
        let on = (HALL, &measured, -41, CH149);
        let others = [the_hall_itself(busy), desk];
        Chooser::default().choose(Instant::now(), on, &others, false)
    }

    /// The desk's ch52 radio, heard as loud as the hall's, saying this busy.
    fn desk_saying(busy: Option<u8>) -> Other {
        Other {
            busy,
            ..desk_at(-43, None)
        }
    }

    #[test]
    fn a_card_on_a_channel_56_percent_busy_tries_a_quieter_one_within_reach() {
        let choice = off_the_hall(38, 56, desk_saying(Some(20)));

        assert_eq!(choice, Choice::Try(DESK.into()), "ch52 ran 20% busy");
    }

    #[test]
    fn a_card_on_a_channel_just_past_40_percent_busy_tries_a_quieter_one() {
        let choice = off_the_hall(38, 41, desk_saying(Some(20)));

        assert_eq!(choice, Choice::Try(DESK.into()), "41% is past the bar");
    }

    #[test]
    fn a_card_on_a_channel_40_percent_busy_stays() {
        let choice = off_the_hall(38, 40, desk_saying(Some(20)));

        assert_eq!(choice, Choice::Stay, "40% busy is the bar, not past it");
    }

    #[test]
    fn a_card_on_a_channel_saying_nothing_of_its_load_stays() {
        let measured = carried(38, 0.0);
        let itself = Other {
            busy: None,
            ..the_hall_itself(56)
        };
        let others = [itself, desk_saying(Some(20))];

        let on = (HALL, &measured, -41, CH149);
        let choice = Chooser::default().choose(Instant::now(), on, &others, false);

        assert_eq!(choice, Choice::Stay, "a channel's load unsaid isn't busy");
    }

    #[test]
    fn a_card_on_a_busy_channel_tries_one_just_40_percent_busy() {
        let choice = off_the_hall(38, 56, desk_saying(Some(40)));

        assert_eq!(choice, Choice::Try(DESK.into()), "40% is within the bar");
    }

    #[test]
    fn a_card_on_a_busy_channel_keeps_off_one_past_40_percent() {
        let choice = off_the_hall(38, 70, desk_saying(Some(41)));

        assert_eq!(choice, Choice::LookQuieter, "41% is past the bar");
    }

    #[test]
    fn a_card_on_a_busy_channel_keeps_off_one_saying_nothing_of_its_load() {
        let choice = off_the_hall(38, 56, desk_saying(None));

        assert_eq!(choice, Choice::LookQuieter, "an unsaid load may be as busy");
    }

    #[test]
    fn a_card_on_a_channel_near_the_bar_keeps_off_one_little_quieter() {
        let choice = off_the_hall(38, 48, desk_saying(Some(34)));

        assert_eq!(choice, Choice::LookQuieter, "14 points is too little");
    }

    #[test]
    fn a_card_on_a_channel_near_the_bar_tries_one_15_points_quieter() {
        let choice = off_the_hall(38, 48, desk_saying(Some(33)));

        assert_eq!(choice, Choice::Try(DESK.into()), "15 points is room enough");
    }

    #[test]
    fn a_card_on_a_busy_channel_keeps_off_a_quieter_one_out_of_reach() {
        let choice = off_the_hall(38, 56, desk_at(-50, None));

        assert_eq!(choice, Choice::LookQuieter, "9 dB fainter, same band");
    }

    #[test]
    fn a_card_on_a_busy_channel_keeps_off_a_quieter_one_known_no_quicker() {
        let choice = off_the_hall(38, 56, desk_at(-43, Some(carried(36, 0.0))));

        assert_eq!(choice, Choice::Stay, "36 ms is no quicker than 38");
    }

    #[test]
    fn a_card_on_a_busy_channel_takes_one_known_quicker_over_a_quieter_untried() {
        let measured = carried(38, 0.0);
        let quick = Other {
            point: "6c:ae:f6:ba:85:86".into(),
            signal: -40,
            busy: Some(35),
            frequency: CH52,
            measured: Some(carried(10, 0.0)),
        };
        let others = [the_hall_itself(56), desk_saying(Some(20)), quick];

        let on = (HALL, &measured, -41, CH149);
        let choice = Chooser::default().choose(Instant::now(), on, &others, false);

        let to = Choice::Try("6c:ae:f6:ba:85:86".into());
        assert_eq!(choice, to, "10 ms known beats 20% busy untried");
    }

    #[test]
    fn a_card_on_a_busy_channel_stays_through_its_first_bad_minute() {
        let choice = off_the_hall(45, 56, desk_saying(Some(20)));

        assert_eq!(choice, Choice::Stay, "a bad minute alone moves nothing");
    }

    #[test]
    fn a_move_off_a_busy_channel_goes_back_if_it_carries_worse_not_badly() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let quick = carried(25, 0.0);
        let others = [the_hall_itself(56), desk_saying(Some(20))];
        let left = chooser.choose(start, (HALL, &quick, -41, CH149), &others, false);

        let worse = carried(36, 0.0);
        let on = (DESK, &worse, -43, CH52);
        let back = chooser.choose(start + TRIAL, on, &[the_hall_itself(56)], false);

        assert_eq!(left, Choice::Try(DESK.into()), "ch149 was 56% busy");
        assert_eq!(back, Choice::Back(HALL.into()), "36 ms is worse than 25");
    }

    #[test]
    fn a_card_back_on_a_busy_channel_keeps_off_the_quieter_one_it_left() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let quick = carried(25, 0.0);
        let others = [the_hall_itself(56), desk_saying(Some(20))];
        chooser.choose(start, (HALL, &quick, -41, CH149), &others, false);
        let worse = carried(36, 0.0);
        chooser.choose(start + TRIAL, (DESK, &worse, -43, CH52), &others, false);

        let later = start + TRIAL + SETTLE;
        let again = chooser.choose(later, (HALL, &quick, -41, CH149), &others, false);

        assert_eq!(again, Choice::Stay, "the desk carried worse lately");
    }

    #[test]
    fn a_card_on_a_busy_channel_tries_what_its_look_found() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let measured = carried(38, 0.0);
        let on = (HALL, &measured, -41, CH149);
        let looked = chooser.choose(start, on, &[the_hall_itself(56)], false);

        let found = [the_hall_itself(56), desk_saying(Some(20))];
        let after = chooser.choose(start + Duration::from_secs(60), on, &found, false);

        assert_eq!(looked, Choice::LookQuieter, "it heard nothing quieter");
        assert_eq!(after, Choice::Try(DESK.into()), "the look found the desk");
    }

    #[test]
    fn a_card_on_a_busy_channel_tries_the_loudest_of_two_quieter() {
        let measured = carried(38, 0.0);
        let louder = Other {
            point: "6c:ae:f6:ba:85:86".into(),
            signal: -38,
            ..desk_saying(Some(30))
        };
        let others = [the_hall_itself(56), desk_saying(Some(20)), louder];

        let on = (HALL, &measured, -41, CH149);
        let choice = Chooser::default().choose(Instant::now(), on, &others, false);

        let to = Choice::Try("6c:ae:f6:ba:85:86".into());
        assert_eq!(choice, to, "-38 dBm is louder than -43");
    }

    #[test]
    fn a_busy_channel_on_2_4_ghz_is_left_to_the_band_rule() {
        let near = Other {
            point: NEAR.into(),
            signal: -40,
            busy: Some(56),
            frequency: CH6,
            measured: None,
        };

        let crowded = crowding((NEAR, CH6), &[near]);

        assert_eq!(crowded, None, "2.4 GHz is no wider band to be crowded on");
    }

    #[test]
    fn a_card_on_a_busy_channel_hearing_nothing_else_looks_for_a_quieter_one() {
        let measured = carried(38, 0.0);
        let on = (HALL, &measured, -41, CH149);

        let choice = Chooser::default().choose(Instant::now(), on, &[the_hall_itself(56)], false);

        assert_eq!(choice, Choice::LookQuieter, "ch149 said 56% busy");
    }

    #[test]
    fn a_card_on_a_busy_channel_looks_once_a_settle() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let measured = carried(38, 0.0);
        let (on, others) = ((HALL, &measured, -41, CH149), [the_hall_itself(56)]);

        let first = chooser.choose(start, on, &others, false);
        let soon = chooser.choose(start + TRIAL, on, &others, false);
        let settled = chooser.choose(start + SETTLE, on, &others, false);

        assert_eq!(first, Choice::LookQuieter, "it hears nothing quieter");
        assert_eq!(soon, Choice::Stay, "a look rests a settle");
        assert_eq!(settled, Choice::LookQuieter, "then it looks again");
    }

    #[test]
    fn a_card_on_a_busy_channel_looks_for_no_quieter_one_in_a_call() {
        let measured = carried(38, 0.0);
        let on = (HALL, &measured, -41, CH149);

        let choice = Chooser::default().choose(Instant::now(), on, &[the_hall_itself(56)], true);

        assert_eq!(choice, Choice::Stay, "a call outranks room");
    }

    #[test]
    fn one_carrying_badly_tries_5_ghz_before_a_louder_2_4_ghz() {
        let mut chooser = after_a_bad_minute(HALL);
        let badly = carried(119, 0.0);
        let others = [near_at(-34, None), desk_at(-44, None)];

        let on = (HALL, &badly, -46, 5745);
        let choice = chooser.choose(Instant::now(), on, &others, false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "the 5 GHz radio, heard well enough, comes before the louder 2.4"
        );
    }

    #[test]
    fn one_on_2_4_ghz_carrying_badly_tries_5_ghz_heard_well_enough_though_fainter() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.0);

        let on = (NEAR, &badly, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-45, None)], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "5 GHz is heard fainter than 2.4 from the same radio, and -45 is loud"
        );
    }

    /// Another eero's 5 GHz radio, heard from the desk far more faintly.
    fn far_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: "6c:ae:f6:ba:85:88".into(),
            signal,
            busy: Some(20),
            frequency: 5745,
            measured,
        }
    }

    #[test]
    fn one_on_5_ghz_carrying_badly_keeps_off_5_ghz_heard_much_more_faintly() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(56, 0.0);

        let on = (DESK, &badly, -47, CH52);
        let choice = chooser.choose(Instant::now(), on, &[far_at(-65, None)], false);

        assert_eq!(choice, Choice::Stay, "18 dB fainter carries less");
    }

    #[test]
    fn one_carrying_badly_keeps_off_one_known_quicker_but_heard_much_more_faintly() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(61, 0.0);
        let known = far_at(-65, Some(carried(26, 0.0)));

        let on = (DESK, &badly, -48, CH52);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(choice, Choice::Stay, "17 dB fainter isn't room");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_keeps_off_5_ghz_heard_30_db_fainter() {
        let mut chooser = Chooser::default();
        let well = carried(11, 0.0);

        let on = (NEAR, &well, -36, CH6);
        let choice = chooser.choose(Instant::now(), on, &[far_at(-66, None)], false);

        assert_eq!(choice, Choice::LookWider, "there it carried worse");
    }

    #[test]
    fn one_on_2_4_ghz_carrying_badly_keeps_off_5_ghz_heard_30_db_fainter() {
        let mut chooser = after_a_bad_minute(NEAR);
        let badly = carried(119, 0.0);

        let on = (NEAR, &badly, -36, CH6);
        let choice = chooser.choose(Instant::now(), on, &[far_at(-66, None)], false);

        assert_eq!(choice, Choice::Stay, "30 dB fainter is another room");
    }

    #[test]
    fn one_on_2_4_ghz_carrying_badly_moves_to_5_ghz_known_quicker_though_fainter() {
        let mut chooser = after_a_bad_minute(NEAR);
        let badly = carried(119, 0.0);
        let known = desk_at(-45, Some(carried(20, 0.0)));

        let on = (NEAR, &badly, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        let to_the_desk = Choice::Try(DESK.into());
        assert_eq!(choice, to_the_desk, "5 GHz is heard fainter, and wider");
    }

    #[test]
    fn a_move_to_a_wider_band_isn_t_judged_on_a_heavy_download_s_own_queue() {
        let start = Instant::now();
        let well = carried(9, 0.0);
        let mut chooser = Chooser::default();
        let tried = chooser.choose(start, (NEAR, &well, -34, CH6), &[desk_at(-45, None)], false);
        assert_eq!(tried, Choice::Try(DESK.into()), "the first move is made");
        let near = near_at(-34, Some(well.clone()));
        let queued = carried(60, 0.0);

        let judged = chooser.choose(start + TRIAL, (DESK, &queued, -45, CH52), &[near], false);

        assert_eq!(
            judged,
            Choice::Stay,
            "60 ms moving 200 with a 20 ms middle is its own queue, which 2.4 GHz drains slower"
        );
    }

    #[test]
    fn a_move_to_a_wider_band_goes_back_only_if_it_carries_badly_and_lossy_or_faint() {
        let start = Instant::now();
        let well = carried(9, 0.0);
        let on = (NEAR, &well, -34, CH6);
        let near = [near_at(-34, Some(well.clone()))];
        let judged = |measured: &Measured, signal| {
            let mut chooser = Chooser::default();
            let tried = chooser.choose(start, on, &[desk_at(-45, None)], false);
            assert_eq!(tried, Choice::Try(DESK.into()), "the first move is made");
            chooser.choose(start + TRIAL, (DESK, measured, signal, CH52), &near, false)
        };
        let (slower, slow, lossy) = (carried(25, 0.0), carried(90, 0.0), carried(90, 3.0));

        assert_eq!(
            judged(&slower, -45),
            Choice::Stay,
            "25 ms on 5 GHz still carries well"
        );
        assert_eq!(
            judged(&slow, -45),
            Choice::Stay,
            "90 ms with none lost is no reason for 2.4 GHz"
        );
        let back = Choice::Back(NEAR.into());
        let resting = |p50| Measured {
            p50: Some(Duration::from_millis(p50)),
            moving: Some(10),
            ..carried(90, 0.0)
        };
        assert_eq!(
            judged(&resting(5), -45),
            Choice::Stay,
            "90 ms at rest on a 5 ms middle still holds"
        );
        assert_eq!(
            judged(&resting(15), -45),
            back,
            "90 ms at rest on a 15 ms middle is slow air"
        );
        assert_eq!(judged(&lossy, -45), back, "90 ms losing 3% is the air");
        assert_eq!(
            judged(&slow, -68),
            back,
            "90 ms heard past FAINTEST is fading"
        );
    }

    #[test]
    fn an_idle_move_to_a_wider_band_carrying_worse_stands_while_it_holds() {
        let start = Instant::now();
        let well = idle(carried(9, 0.0));
        let on = (NEAR, &well, -34, CH6);
        let wider = [desk_at(-45, Some(idle(carried(9, 0.0))))];
        let near = [near_at(-34, Some(well.clone()))];
        let judged = |lost| {
            let mut chooser = Chooser::default();
            let tried = chooser.choose(start, on, &wider, false);
            assert_eq!(tried, Choice::Try(DESK.into()), "the first move is made");
            let worse = idle(carried(30, lost));
            chooser.choose(start + TRIAL, (DESK, &worse, -45, CH52), &near, false)
        };

        assert_eq!(
            judged(0.0),
            Choice::Stay,
            "30 ms against 9, losing nothing, holds"
        );
        assert_eq!(judged(3.0), Choice::Back(NEAR.into()), "losing 3% doesn't");
    }

    #[test]
    fn an_idle_card_on_2_4_ghz_keeps_off_a_slower_5_ghz_radio() {
        let mut chooser = Chooser::default();
        let quick = idle(carried(15, 0.0));
        let slower = [desk_at(-45, Some(carried(25, 0.0)))];

        let choice = chooser.choose(Instant::now(), (NEAR, &quick, -36, CH6), &slower, false);

        assert_eq!(choice, Choice::Stay, "its room goes unused");
    }

    #[test]
    fn a_card_moving_100_mbit_s_on_2_4_ghz_takes_a_slower_5_ghz_radio() {
        let mut chooser = Chooser::default();
        let busy = Measured {
            moving: Some(100),
            ..carried(15, 0.0)
        };
        let slower = [desk_at(-45, Some(carried(25, 0.0)))];

        let choice = chooser.choose(Instant::now(), (NEAR, &busy, -36, CH6), &slower, false);

        assert_eq!(choice, Choice::Try(DESK.into()), "its room is used");
    }

    #[test]
    fn an_idle_card_on_2_4_ghz_takes_a_5_ghz_radio_no_slower() {
        let mut chooser = Chooser::default();
        let quick = idle(carried(15, 0.0));
        let as_quick = [desk_at(-45, Some(carried(15, 0.0)))];

        let choice = chooser.choose(Instant::now(), (NEAR, &quick, -36, CH6), &as_quick, false);

        assert_eq!(choice, Choice::Try(DESK.into()), "it costs nothing");
    }

    #[test]
    fn an_idle_card_on_2_4_ghz_leaves_an_untried_5_ghz_radio() {
        let mut chooser = Chooser::default();
        let quick = idle(carried(15, 0.0));
        let untried = [desk_at(-45, None)];

        let choice = chooser.choose(Instant::now(), (NEAR, &quick, -36, CH6), &untried, false);

        assert_eq!(choice, Choice::Stay, "it may be slower");
    }

    #[test]
    fn an_idle_card_on_2_4_ghz_hearing_no_5_ghz_doesnt_look_for_it() {
        let mut chooser = Chooser::default();
        let quick = idle(carried(15, 0.0));

        let choice = chooser.choose(Instant::now(), (NEAR, &quick, -36, CH6), &[], false);

        assert_eq!(choice, Choice::Stay, "a look takes it off air");
    }

    #[test]
    fn a_card_whose_kernel_says_nothing_moved_counts_as_idle() {
        let mut chooser = Chooser::default();
        let unsaid = Measured {
            moving: None,
            ..carried(15, 0.0)
        };
        let untried = [desk_at(-45, None)];

        let choice = chooser.choose(Instant::now(), (NEAR, &unsaid, -36, CH6), &untried, false);

        assert_eq!(choice, Choice::Stay, "nothing says its room is used");
    }

    #[test]
    fn a_card_moving_half_a_faint_link_on_2_4_ghz_takes_5_ghz() {
        let (mut half, mut less) = (Chooser::default(), Chooser::default());
        let (busy, short) = (on_a_link(45, 90), on_a_link(44, 90));
        let untried = [desk_at(-62, None)];
        let now = Instant::now();

        let took = half.choose(now, (NEAR, &busy, -60, CH6), &untried, false);
        let kept = less.choose(now, (NEAR, &short, -60, CH6), &untried, false);

        assert_eq!(took, Choice::Try(DESK.into()), "45 of 90 Mbit/s is used");
        assert_eq!(kept, Choice::Stay, "44 isn't");
    }

    #[test]
    fn a_card_on_a_fast_2_4_ghz_link_takes_5_ghz_once_it_moves_100_mbit_s() {
        let (mut more, mut less) = (Chooser::default(), Chooser::default());
        let (busy, short) = (on_a_link(100, 286), on_a_link(99, 286));
        let untried = [desk_at(-45, None)];
        let now = Instant::now();

        let took = more.choose(now, (NEAR, &busy, -36, CH6), &untried, false);
        let kept = less.choose(now, (NEAR, &short, -36, CH6), &untried, false);

        assert_eq!(took, Choice::Try(DESK.into()), "100 is enough");
        assert_eq!(kept, Choice::Stay, "99 isn't");
    }

    #[test]
    fn a_card_on_a_link_said_to_send_slowly_still_moves_25_mbit_s_to_use_it() {
        let (mut more, mut less) = (Chooser::default(), Chooser::default());
        let (busy, short) = (on_a_link(25, 2), on_a_link(24, 2));
        let untried = [desk_at(-62, None)];
        let now = Instant::now();

        let took = more.choose(now, (NEAR, &busy, -60, CH6), &untried, false);
        let kept = less.choose(now, (NEAR, &short, -60, CH6), &untried, false);

        assert_eq!(took, Choice::Try(DESK.into()), "25 Mbit/s is used");
        assert_eq!(kept, Choice::Stay, "24 isn't, whatever the link says");
    }

    /// What a card on a link sending at `room` Mbit/s carried, moving this
    /// much.
    fn on_a_link(moving: u32, room: u32) -> Measured {
        Measured {
            moving: Some(moving),
            room: Some(room),
            ..carried(15, 0.0)
        }
    }

    #[test]
    fn an_idle_card_s_move_to_5_ghz_no_slower_goes_back_if_it_carries_worse_and_faint() {
        let start = Instant::now();
        let mut chooser = Chooser::default();
        let quick = idle(carried(15, 0.0));
        let as_quick = [desk_at(-45, Some(carried(15, 0.0)))];
        let tried = chooser.choose(start, (NEAR, &quick, -36, CH6), &as_quick, false);
        assert_eq!(tried, Choice::Try(DESK.into()), "the move is made");
        let near = [near_at(-36, Some(quick.clone()))];
        let worse = idle(carried(30, 0.0));

        let back = chooser.choose(start + TRIAL, (DESK, &worse, -68, CH52), &near, false);

        assert_eq!(
            back,
            Choice::Back(NEAR.into()),
            "30 ms heard at -68 is worse than 15"
        );
    }

    /// Has a card on the desk's 2.4 GHz radio, carrying badly a second
    /// minute as it did on the night of 23 Nov 2026, try the ch52 radio
    /// untried at `at`, and judges the trial at its end on what ch52
    /// carried at 20:30 that night: when it went back.
    fn tried_ch52_and_back(chooser: &mut Chooser, at: Instant) -> Instant {
        chooser.badly_on = Some(NEAR.into());
        let near = carried(201, 0.0);
        let ch52 = [desk_at(-48, None)];
        let tried = chooser.choose(at, (NEAR, &near, -50, CH6), &ch52, false);
        assert_eq!(
            tried,
            Choice::Try(DESK.into()),
            "ch52, untried, wasn't tried off 201 ms"
        );
        let back = at + TRIAL;
        let worse = carried(775, 3.3);
        let judged = chooser.choose(back, (DESK, &worse, -48, CH52), &[], false);
        assert_eq!(
            judged,
            Choice::Back(NEAR.into()),
            "775 ms with 3.3% lost didn't go back to the 201 left"
        );
        back
    }

    const HOUR: Duration = Duration::from_hours(1);

    /// A chooser whose card tried ch52 off the desk's 2.4 GHz radio and
    /// went back this many times, each try as the last keep-off ended, and
    /// when it went back the last time.
    fn found_worse(times: usize, start: Instant) -> (Chooser, Instant) {
        let mut chooser = Chooser::default();
        let mut back = tried_ch52_and_back(&mut chooser, start);
        for _ in 1..times {
            let ended = chooser.kept_for(DESK, back).unwrap_or_default();
            back = tried_ch52_and_back(&mut chooser, back + ended);
        }
        (chooser, back)
    }

    /// The same, gone back twice, for a keep-off of two hours.
    pub(crate) fn found_worse_twice(start: Instant) -> (Chooser, Instant) {
        found_worse(2, start)
    }

    #[test]
    fn a_point_found_worse_again_and_again_is_kept_off_twice_as_long_each_time_up_to_12_hours() {
        let mut chooser = Chooser::default();
        let mut at = Instant::now();

        let kept: Vec<_> = (0..6)
            .map(|_| {
                let back = tried_ch52_and_back(&mut chooser, at);
                let lasts = chooser.kept_for(DESK, back);
                // Tried again the moment its keep-off ends.
                at = back + lasts.unwrap_or_default();
                lasts
            })
            .collect();

        let hours = [1, 2, 4, 8, 12, 12].map(|hours| Some(HOUR * hours));
        assert_eq!(kept, hours, "each keep-off wasn't twice the last, to 12 h");
    }

    #[test]
    fn a_point_found_worse_again_is_left_untried_until_its_longer_keep_off_ends() {
        let near = carried(201, 0.0);
        let ch52 = [desk_at(-48, None)];
        let chosen_after = |after: Duration| {
            let (mut chooser, second) = found_worse_twice(Instant::now());
            chooser.badly_on = Some(NEAR.into());
            chooser.choose(second + after, (NEAR, &near, -50, CH6), &ch52, false)
        };

        let short = chosen_after((HOUR * 2).saturating_sub(Duration::from_secs(1)));
        let over = chosen_after(HOUR * 2);

        assert_ne!(
            short,
            Choice::Try(DESK.into()),
            "ch52 found worse twice was tried inside its two hours"
        );
        assert_eq!(
            over,
            Choice::Try(DESK.into()),
            "ch52 wasn't tried once its two hours were over"
        );
    }

    #[test]
    fn a_failing_link_may_leave_for_a_point_kept_off_longer_after_its_first_hour() {
        let failing = carried(400, 15.0);
        let ch52 = [desk_at(-48, None)];
        // Kept off eight hours, where only a first hour holds against a
        // failing link, not all but the last.
        let chosen_after = |after: Duration| {
            let (mut chooser, fourth) = found_worse(4, Instant::now());
            assert_eq!(
                chooser.kept_for(DESK, fourth),
                Some(HOUR * 8),
                "ch52 found worse four times wasn't kept off eight hours"
            );
            chooser.badly_on = Some(NEAR.into());
            chooser.choose(fourth + after, (NEAR, &failing, -50, CH6), &ch52, false)
        };

        let firm = chosen_after(HOUR.saturating_sub(Duration::from_secs(1)));
        let yields = chosen_after(HOUR);

        assert_ne!(
            firm,
            Choice::Try(DESK.into()),
            "a failing link left for ch52 inside the hour its keep-off holds against one"
        );
        assert_eq!(
            yields,
            Choice::Try(DESK.into()),
            "a failing link was held off ch52, the one other point, past its first hour"
        );
    }

    #[test]
    fn a_trial_standing_only_for_beating_a_failing_link_forgets_nothing() {
        let (mut chooser, second) = found_worse_twice(Instant::now());
        let third = second + HOUR * 2;
        chooser.badly_on = Some(NEAR.into());
        let failing = carried(400, 15.0);
        let ch52 = [desk_at(-48, None)];
        let tried = chooser.choose(third, (NEAR, &failing, -50, CH6), &ch52, false);
        assert_eq!(
            tried,
            Choice::Try(DESK.into()),
            "ch52 wasn't tried off a failing link"
        );
        let slow = carried(300, 2.0);
        let stood = chooser.choose(third + TRIAL, (DESK, &slow, -48, CH52), &[], false);
        assert!(
            !matches!(stood, Choice::Back(_)),
            "ch52 carrying better than the failing link went back: {stood:?}"
        );

        let later = tried_ch52_and_back(&mut chooser, third + TRIAL + SETTLE);

        assert_eq!(
            chooser.kept_for(DESK, later),
            Some(HOUR * 4),
            "ch52, standing only for beating a failing link, was forgiven its two backs"
        );
    }

    /// How long ch52 is kept off when found worse once more, after a
    /// third trial of it off the 2.4 GHz radio's 201 ms stood carrying
    /// this.
    fn kept_off_after_standing(stood_carrying: &Measured) -> Option<Duration> {
        let (mut chooser, second) = found_worse_twice(Instant::now());
        let third = second + HOUR * 2;
        chooser.badly_on = Some(NEAR.into());
        let ch52 = [desk_at(-48, None)];
        let tried = chooser.choose(third, (NEAR, &carried(201, 0.0), -50, CH6), &ch52, false);
        assert_eq!(
            tried,
            Choice::Try(DESK.into()),
            "ch52 wasn't tried a third time"
        );
        let there = (DESK, stood_carrying, -48, CH52);
        let stood = chooser.choose(third + TRIAL, there, &[], false);
        assert!(
            !matches!(stood, Choice::Back(_)),
            "ch52 carrying {stood_carrying:?} off 201 ms went back: {stood:?}"
        );
        let later = tried_ch52_and_back(&mut chooser, third + TRIAL + SETTLE);
        chooser.kept_for(DESK, later)
    }

    #[test]
    fn a_trial_standing_carrying_well_forgets_how_often_its_point_was_found_worse() {
        assert_eq!(
            kept_off_after_standing(&carried(15, 0.0)),
            Some(HOUR),
            "ch52 found worse after a trial stood wasn't kept off an hour, as at first"
        );
    }

    #[test]
    fn a_move_missing_a_point_kept_off_longer_keeps_the_longer_keep_off() {
        let (mut chooser, fourth) = found_worse(4, Instant::now());
        let missed = fourth + HOUR * 2;

        chooser.keep_off_unreached(DESK, missed);
        let mut fresh = Chooser::default();
        fresh.keep_off_unreached(DESK, missed);

        assert_eq!(
            chooser.kept_for(DESK, missed),
            Some(HOUR * 6),
            "a move missing ch52 two hours into eight shortened its keep-off"
        );
        assert_eq!(
            fresh.kept_for(DESK, missed),
            Some(SPURNED),
            "a move missing a point kept off by nothing didn't keep it off an hour"
        );
    }

    #[test]
    fn a_trial_standing_only_for_beating_a_slow_link_forgets_nothing() {
        // No worse than the 201 ms left, so it stands, but slow all the
        // same: a point no better than a bad link hasn't come good.
        assert_eq!(
            kept_off_after_standing(&carried(205, 0.0)),
            Some(HOUR * 4),
            "ch52, standing only as slow as the link it left, was forgiven its two backs"
        );
    }

    #[test]
    fn a_point_untried_for_12_hours_after_its_keep_off_is_found_worse_afresh() {
        // The gap runs from the first keep-off ending to the second going
        // back, which comes a trial after the second try.
        let kept_after = |untried: Duration| {
            let mut chooser = Chooser::default();
            let first = tried_ch52_and_back(&mut chooser, Instant::now());
            let tried = first + HOUR + untried.saturating_sub(TRIAL);
            let again = tried_ch52_and_back(&mut chooser, tried);
            chooser.kept_for(DESK, again)
        };

        let within = kept_after(SPURNED_MOST.saturating_sub(Duration::from_secs(1)));
        let past = kept_after(SPURNED_MOST);

        assert_eq!(
            within,
            Some(HOUR * 2),
            "found worse again within 12 h, not doubled"
        );
        assert_eq!(past, Some(HOUR), "found worse 12 h after, not afresh");
    }
}
