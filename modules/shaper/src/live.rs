//! The one-way controller (src/oneway.rs), shaping as the recorder writes.
//! The recorder (src/record.rs) writes each probe's one-way delays to a day
//! file; the shaper reads each line the file gains, makes seconds of them
//! as a replay does (src/replay.rs), and steps the controller on each, so
//! the rates that shape a link are the ones a replay of its lines scores.
//! Each stay starts as a network never met does, or at its anchor's hold
//! where its line is declared: what a stay learned ends with it.
//!
//! Long because its tests replay recorded days through every path a stay
//! takes, which only reads whole beside the follower they step.

use crate::networks::Anchors;
use crate::oneway::{self, Ours, Tuning};
use crate::record::day_file;
use crate::replay::{Controller as _, Rates, Reading, Sample, parse};
use crate::vpndelay::Probe;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How long the recorder may write nothing, in ms, before its link is let
/// go unlimited: with no signal, the controller has nothing to hold a rate
/// to, and a rate held from a minute ago may be far under the
/// connection's now.
const SILENT_MS: u64 = 10_000;

/// What the controller last heard of its link's second, and what it made
/// of it: what a trace needs to say why a rate held where it did.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Heard {
    /// Seconds since the stay began.
    pub t_s: f64,
    /// What the link carried each way, in kbit/s.
    pub up_kbit: f64,
    pub down_kbit: f64,
    /// Each direction's own delay, and the gateway's round trip, in ms.
    pub up_delay_ms: Option<f64>,
    pub down_delay_ms: Option<f64>,
    pub gateway_ms: Option<f64>,
    /// Whose the upload's queue was, and what was sent again and dropped.
    pub ours: Option<Ours>,
    /// A call's round trip through the VPN, and the least it was taken to
    /// be, in ms: what one took past the other is the queue only the VPN's
    /// flow waits in.
    pub vpn_ms: Option<f64>,
    pub vpn_base_ms: Option<f64>,
    /// What the upload is held to, in kbit/s, and where it holds under.
    pub up_limit: Option<u64>,
    pub plateau: Option<f64>,
    /// The trial the upload is tuned by, while one is on: what a
    /// second's limit and delay are to be judged as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial: Option<Tuning>,
    /// What each of the VPN's tins sent up over the last second, in
    /// kbit/s: each class's share of the upload, read where the tunnel's
    /// classes still show. The shaper fills it in, beside what the
    /// controller heard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_tins_kbit: Option<crate::qdisc::Tins>,
    /// Why the upload holds where it does, in one sentence (src/why.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// The trial written at `path`, held within its bounds: none while there
/// is no file, and today's tuning for whatever it doesn't say. A file
/// that can't be read as one is no trial, so a slip in writing it leaves
/// the shaper as it was. A ceiling ends `CEILING_KEPT` after the file was
/// written, so one a script left behind can't hold the upload down on
/// every network after.
pub fn trial(path: &Path) -> Option<Tuning> {
    let text = fs::read_to_string(path).ok()?;
    let written = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let tuning = serde_json::from_str::<Tuning>(&text).ok()?.bounded();
    Some(ended(tuning, SystemTime::now().duration_since(written)))
}

/// How long a trial's ceiling holds after its file was written.
const CEILING_KEPT: Duration = Duration::from_secs(600);

/// `tuning`, its ceiling gone once `age` has passed `CEILING_KEPT`, or
/// when the file's time is past the clock's, which no age can be told by.
fn ended(tuning: Tuning, age: Result<Duration, std::time::SystemTimeError>) -> Tuning {
    if age.is_ok_and(|age| age <= CEILING_KEPT) {
        tuning
    } else {
        Tuning {
            most_kbit: None,
            ..tuning
        }
    }
}

/// The name a recording gives a network it couldn't name, which is no one
/// network to remember.
const UNKNOWN: &str = "unknown";

/// What the recorder's lines have said: the link it records, and the
/// controller stepped on each second they finished.
pub struct Follow {
    link: Option<String>,
    reading: Reading,
    controller: oneway::Controller,
    rates: Rates,
    /// When the latest line was written, in ms since 1970.
    last_ms: u64,
    /// Whose each link's upload queue was, as the shaper last saw it.
    ours: BTreeMap<String, Ours>,
    /// A call's round trip through the VPN riding the link, as last heard.
    vpn_ms: Option<f64>,
    /// The last second the controller stepped on, and when it began, in
    /// seconds since 1970, for the trace (src/trace.rs).
    last: Option<crate::replay::Second>,
    last_at: Option<u64>,
    /// The network of the stay.
    network: String,
    /// When the shaper started, in ms since 1970: what was recorded before
    /// it is no stay of its own.
    from_ms: u64,
    /// The trial the upload is tuned by, which outlasts a stay.
    trial: Option<Tuning>,
    /// Each network's anchor, which a stay on it is guarded by.
    anchors: Anchors,
}

impl Follow {
    pub fn new() -> Follow {
        Follow {
            link: None,
            reading: Reading::new(UNKNOWN.into()),
            controller: oneway::Controller::new(None, None),
            rates: Rates::default(),
            last_ms: 0,
            ours: BTreeMap::new(),
            vpn_ms: None,
            last: None,
            last_at: None,
            network: UNKNOWN.into(),
            from_ms: 0,
            trial: None,
            anchors: Anchors::new(),
        }
    }

    /// Guards each network's upload by its anchor, the stay's from now.
    pub fn anchor(&mut self, anchors: Anchors) {
        self.controller.anchor(anchors.get(&self.network).copied());
        self.controller.start_at_the_anchor();
        self.anchors = anchors;
    }

    /// Tunes the upload by a trial from now on, or by none, and in every
    /// stay to come.
    pub fn tune(&mut self, trial: Option<Tuning>) {
        // A trial whose ceiling has ended and that set nothing else is
        // none, so the record stops calling every second a trial.
        self.trial = trial
            .map(Tuning::bounded)
            .filter(|t| *t != Tuning::default());
        self.controller.up.tune(self.trial.unwrap_or_default());
    }

    /// Takes whose a link's upload queue is now, for the seconds to come.
    pub fn see(&mut self, link: &str, ours: Ours) {
        self.ours.insert(link.to_string(), ours);
    }

    /// Takes a call's round trip through the VPN, for the seconds to come.
    pub fn hear_vpn(&mut self, ms: Option<f64>) {
        self.vpn_ms = ms;
    }

    /// Takes one line's sample. A recording begun anew is a new stay,
    /// started as a network never met is, or at its anchor.
    pub fn take(&mut self, sample: &Sample) {
        self.last_ms = self.last_ms.max(sample.at());
        if let Sample::Pool { network, link, .. } = sample {
            if link.is_some() {
                self.link.clone_from(link);
            }
            if let Some(network) = network {
                self.reading = Reading::new(network.clone());
                self.controller = oneway::Controller::new(None, None);
                self.controller.up.tune(self.trial.unwrap_or_default());
                self.controller.anchor(self.anchors.get(network).copied());
                self.controller.start_at_the_anchor();
                self.network.clone_from(network);
                self.rates = Rates::default();
                return;
            }
        }
        // The stay's network is named from before the start, but nothing
        // recorded then is stepped on: the shaper held nothing, so its
        // guard would give up on rates nothing held, and the controller
        // would learn from what no CAKE counted.
        if sample.at() < self.from_ms {
            return;
        }
        self.reading.take(sample);
        let ours = self
            .link
            .as_ref()
            .and_then(|link| self.ours.get(link))
            .copied();
        for mut second in self.reading.drain() {
            second.up_ours = ours;
            second.vpn_ms = self.vpn_ms;
            let guarded = self.controller.guarded;
            self.rates = self.controller.second(&second);
            if self.controller.guarded > guarded {
                eprintln!(
                    "the upload was held under {:.0}% of {}'s anchor for a minute with no cut \
                     that drained a queue, so it's back to the anchor's hold",
                    oneway::ANCHOR_LOW * 100.0,
                    self.network
                );
            }
            self.last_at = self
                .reading
                .began_s()
                .map(|began| began + crate::rate::whole(second.t_s));
            self.last = Some(second);
        }
    }

    /// The link the controller holds, and what it last heard of it and
    /// made of it, once it has stepped on a second.
    pub fn heard(&self) -> Option<(&str, Heard)> {
        let (link, second) = (self.link.as_deref()?, self.last.as_ref()?);
        let heard = Heard {
            t_s: second.t_s,
            up_kbit: second.up_kbit,
            down_kbit: second.down_kbit,
            up_delay_ms: second.up_delay_ms,
            down_delay_ms: second.down_delay_ms,
            gateway_ms: second.gateway_ms,
            ours: second.up_ours,
            vpn_ms: second.vpn_ms,
            vpn_base_ms: self.controller.vpn_base(),
            up_limit: self.controller.up.limit(),
            plateau: self.controller.up.plateau(),
            trial: self.trial,
            up_tins_kbit: None,
            why: Some(self.controller.why()),
        };
        Some((link, heard))
    }

    /// Whether the recorder has gone silent by a moment in ms since 1970,
    /// judged once for the rates and the reason alike.
    fn silent(&self, at_ms: u64) -> bool {
        at_ms.saturating_sub(self.last_ms) > SILENT_MS
    }

    /// What the controller's judgement says of the rates `now` hands over
    /// at a moment in ms since 1970: why it holds the upload where it does,
    /// and the upload that reason speaks of, the one answered when it's
    /// the controller's own rate. Nothing it judged holds once the recorder
    /// has gone silent or while it lets the upload go, and its rate from
    /// before waits until it steps on a second heard after a silence, or
    /// after lines that answer no second, as lost probes alone do.
    pub fn why(&self, at_ms: u64) -> crate::why::Judged {
        use crate::why::Judged;
        if self.silent(at_ms) || self.controller.lets_go() {
            return Judged::Nothing;
        }
        // The second last stepped on is within the silence of the last line
        // read, or the rate is from before a silence just ended.
        let fresh = self
            .last_at
            .is_some_and(|s| s.saturating_mul(1_000).saturating_add(SILENT_MS) >= self.last_ms);
        if !fresh {
            return Judged::Waiting;
        }
        let own = self.controller.up.limit();
        Judged::Holds(crate::why::Reason {
            up: self.rates.up.filter(|&up| Some(up) == own),
            said: self.controller.why(),
        })
    }

    /// The link the recorder follows and the rates to hold it to, at a
    /// moment in ms since 1970: no limit once the recorder has gone silent.
    pub fn now(&self, at_ms: u64) -> Option<(&str, Rates)> {
        let link = self.link.as_deref()?;
        if self.silent(at_ms) {
            // A trial's ceiling still holds what nothing else does.
            let up = self.trial.and_then(|t| t.most_kbit).map(crate::rate::whole);
            return Some((link, Rates { up, down: None }));
        }
        Some((link, self.rates))
    }
}

/// The recorder's day files, read as they grow.
pub struct Live {
    folder: PathBuf,
    /// The day file being read, and how far into it.
    file: (PathBuf, u64),
    /// A line the recorder is still writing.
    partial: Vec<u8>,
    follow: Follow,
    /// A call's round trip through the VPN riding the link.
    vpn: Probe,
}

impl Live {
    /// Starts at yesterday's file, so a stay begun before midnight is known
    /// from its first line. Only what's recorded from `now` on is stepped
    /// on.
    pub fn new(folder: &Path, now: SystemTime) -> Live {
        let yesterday = now.checked_sub(Duration::from_hours(24)).unwrap_or(now);
        Live {
            folder: folder.to_path_buf(),
            file: (day_file(folder, yesterday), 0),
            partial: Vec::new(),
            follow: Follow {
                from_ms: ms(now),
                ..Follow::new()
            },
            vpn: Probe::default(),
        }
    }

    /// Takes whose a link's upload queue is now.
    pub fn see(&mut self, link: &str, ours: Ours) {
        self.follow.see(link, ours);
    }

    /// Tunes the upload by a trial, or by none.
    pub fn tune(&mut self, trial: Option<Tuning>) {
        self.follow.tune(trial);
    }

    /// Guards each network's upload by its anchor.
    pub fn anchor(&mut self, anchors: Anchors) {
        self.follow.anchor(anchors);
    }

    /// The link the controller holds, and what it last heard of it.
    pub fn heard(&self) -> Option<(&str, Heard)> {
        self.follow.heard()
    }

    /// What the controller's judgement says of the rates now (`Follow::why`).
    pub fn why(&self, now: SystemTime) -> crate::why::Judged {
        self.follow.why(ms(now))
    }

    /// The line for the trace of the recorded second the controller last
    /// stepped on, once it has stepped on one.
    pub fn traced(&self) -> Option<crate::trace::Line> {
        let (link, heard) = self.follow.heard()?;
        Some(crate::trace::Line::of(self.follow.last_at?, link, &heard))
    }

    /// Probes through the VPN riding the link, or none, and takes what it
    /// heard over the last second.
    pub fn hear_vpn(&mut self, device: Option<&str>) {
        self.vpn.follow(device);
        self.follow.hear_vpn(self.vpn.heard(Instant::now()));
    }

    /// Reads what the recorder wrote since, and answers the link it
    /// follows and the rates to hold it to. A day's file is read to its end
    /// before the next day's is begun.
    pub fn poll(&mut self, now: SystemTime) -> Option<(&str, Rates)> {
        let today = day_file(&self.folder, now);
        self.read_on();
        if self.file.0 != today {
            self.file = (today, 0);
            self.partial.clear();
            self.read_on();
        }
        self.follow.now(ms(now))
    }

    /// The share of its lane the VPN riding the link is held to (src/tunnel.rs).
    /// A recorder gone silent judges no wait, so the VPN has its whole
    /// share then, under a trial's ceiling as under none. Judged at `now`,
    /// the moment the rates and the reason were.
    pub fn vpn_share(&self, now: SystemTime) -> f64 {
        if self.follow.silent(ms(now)) {
            return 1.0;
        }
        self.follow.controller.tunnel_share()
    }

    /// Takes each whole line the file gained, keeping one half written.
    fn read_on(&mut self) {
        let Ok(mut file) = File::open(&self.file.0) else {
            return;
        };
        if file.seek(SeekFrom::Start(self.file.1)).is_err() {
            return;
        }
        let mut reader = BufReader::new(file);
        while let Ok(read) = reader.read_until(b'\n', &mut self.partial) {
            if read == 0 || self.partial.last() != Some(&b'\n') {
                self.file.1 += u64::try_from(read).unwrap_or(0);
                break;
            }
            self.file.1 += u64::try_from(read).unwrap_or(0);
            if let Some(sample) = parse(&String::from_utf8_lossy(&self.partial)) {
                self.follow.take(&sample);
            }
            self.partial.clear();
        }
    }
}

fn ms(at: SystemTime) -> u64 {
    u64::try_from(
        at.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay;
    use crate::why::Judged;
    use serde_json::json;
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    /// 3 November 2026, 12:00 UTC, in ms since 1970.
    const NOV3: u64 = 1_793_707_200_000;

    fn pool(at: u64, network: Option<&str>) -> String {
        let mut line = json!({"at": at, "pool": [["Quad9", "9.9.9.9"]], "operators": 1,
                              "enough": false, "link": "wlan0"});
        if let Some(network) = network {
            line["network"] = json!(network);
        }
        line.to_string()
    }

    fn stamp(at: u64, back: f64, rx: u64) -> String {
        json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "out": 10.0,
               "back": back, "rtt": 20.0, "rx": rx, "tx": at - NOV3})
        .to_string()
    }

    /// Six seconds of a 200 Mbit/s download, probed every 100 ms, whose
    /// own delay rises 40 ms after two.
    fn queued() -> Vec<String> {
        let mut lines = vec![pool(NOV3, Some("home"))];
        for i in 0..60_u64 {
            let back = if i < 20 { 10.0 } else { 50.0 };
            lines.push(stamp(NOV3 + 100 * (i + 1), back, 2_500_000 * (i + 1)));
        }
        lines
    }

    fn followed(lines: &[String]) -> Follow {
        let mut follow = Follow::new();
        for line in lines {
            follow.take(&parse(line).expect("a sample"));
        }
        follow
    }

    #[test]
    fn a_queue_standing_on_the_recorders_link_is_cut_live() {
        let follow = followed(&queued());
        let (link, rates) = follow.now(NOV3 + 6_000).expect("a link");
        assert_eq!(link, "wlan0");
        assert!(rates.down.is_some_and(|d| d < 200_000), "{rates:?}");
        assert_eq!(rates.up, None);
    }

    #[test]
    fn the_live_controller_answers_what_a_replay_of_the_same_lines_does() {
        let lines = queued();
        let stays = replay::stays(lines.iter().filter_map(|l| parse(l)));
        let mut controller = oneway::Controller::new(None, None);
        let mut replayed = Rates::default();
        for second in &stays[0].seconds {
            replayed = controller.second(second);
        }
        assert!(
            replayed.down.is_some(),
            "the replay cuts nothing to compare"
        );
        let follow = followed(&lines);
        assert_eq!(follow.now(NOV3 + 6_000).map(|(_, r)| r), Some(replayed));
    }

    /// Ten seconds of a 12 Mbit/s upload whose delay never rises, as behind
    /// a router that queues flow by flow, from `from` ms.
    fn flow_queued(from: u64) -> Vec<String> {
        let mut lines = vec![pool(from, Some("home"))];
        for i in 0..100_u64 {
            let at = from + 100 * (i + 1);
            lines.push(
                json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "out": 10.0,
                       "back": 10.0, "rtt": 20.0, "rx": 1_000 * i, "tx": 150_000 * (i + 1)})
                .to_string(),
            );
        }
        lines
    }

    /// A follower that hears data waiting and our CAKE holding nothing.
    fn waiting() -> Follow {
        let mut follow = Follow::new();
        follow.see(
            "wlan0",
            Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            },
        );
        follow
    }

    // What a stay learned of a network that isn't declared starts nothing
    // after it: on 16 November home's 4,586 was the cuts' own doing and
    // held every start there, and a network's line moves between stays.
    // A network met again is found again, as one never met is.
    #[test]
    fn a_network_met_again_starts_as_one_never_met() {
        let mut follow = waiting();
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        assert!(
            follow.controller.up.plateau().is_some(),
            "home's first stay learned no plateau"
        );
        for line in [
            pool(NOV3 + 11_000, Some("hotel")),
            pool(NOV3 + 12_000, Some("home")),
        ] {
            follow.take(&parse(&line).expect("a pool"));
        }
        assert_eq!(follow.controller.up.limit(), None, "home started held");
        assert_eq!(
            follow.controller.up.plateau(),
            None,
            "home started under the plateau it learned"
        );
    }

    #[test]
    fn a_queue_past_cake_that_the_probes_cant_see_is_taken_back_live() {
        let lines = flow_queued(NOV3);
        let mut follow = Follow::new();
        follow.see(
            "wlan0",
            Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            },
        );
        for line in &lines {
            follow.take(&parse(line).expect("a sample"));
        }
        let (_, rates) = follow.now(NOV3 + 10_000).expect("a link");
        assert!(
            rates.up.is_some_and(|up| (11_000..12_000).contains(&up)),
            "held under the router: {rates:?}"
        );
    }

    // A trace of a live proof could show what the link carried, but not
    // what the controller made of it: whose the queue was, the VPN's round
    // trip and its least, the plateau. Without them, a rate held over the
    // line can't be told from a signal that never came.
    #[test]
    fn what_the_controller_heard_last_is_told() {
        let mut follow = waiting();
        follow.hear_vpn(Some(80.0));
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        let (link, heard) = follow.heard().expect("a second heard");
        assert_eq!(link, "wlan0", "told of the wrong link");
        assert_eq!(heard.vpn_ms, Some(80.0), "the VPN's round trip: {heard:?}");
        assert!(
            heard.vpn_base_ms.is_some_and(|base| base <= 80.0),
            "the VPN's least: {heard:?}"
        );
        assert!(
            heard.ours.is_some_and(|ours| ours.waiting && !ours.queued),
            "whose the queue was: {heard:?}"
        );
        assert!(heard.up_kbit > 10_000.0, "what went up: {heard:?}");
        assert_eq!(
            heard.up_limit,
            follow.controller.up.limit(),
            "the limit held: {heard:?}"
        );
        assert_eq!(
            heard.plateau,
            follow.controller.up.plateau(),
            "the plateau: {heard:?}"
        );
    }

    // Under a silent recorder, a trial's ceiling is all that holds the
    // upload, and the controller's last reason was about a second it
    // heard before.
    #[test]
    fn a_silent_recorder_gives_no_reason() {
        let mut follow = waiting();
        follow.tune(Some(Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        }));
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        let last = follow.last_ms;
        assert!(
            follow.why(last).reason().is_some(),
            "a heard second gave no reason"
        );
        let silent = last + SILENT_MS + 1;
        let (_, rates) = follow.now(silent).expect("a link");
        assert_eq!(rates.up, Some(8_500), "the ceiling went with the recorder");
        assert_eq!(
            follow.why(silent),
            Judged::Nothing,
            "a silent recorder gave the last second's reason"
        );
    }

    // The journal holds a reason against the upload the shaper was
    // answered, so a heard second's reason speaks of that upload and
    // reaches the line.
    #[test]
    fn a_reason_speaks_of_the_upload_the_shaper_is_answered() {
        let mut follow = waiting();
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        let last = follow.last_ms;
        let (_, rates) = follow.now(last).expect("a link");
        assert!(rates.up.is_some(), "the flow held no upload: {rates:?}");
        let judged = follow.why(last);
        let reason = judged.reason().expect("a reason");
        assert_eq!(reason.up, rates.up);
        let line = crate::why::holding("wlp0s20f3", rates.up, None, Some(reason));
        assert!(line.ends_with(&reason.said), "{line}");
    }

    /// `seconds` of the upload carried on, each 0.1 s answered by the
    /// gateway alone, from `from` ms, after `flow_queued`'s ten.
    fn gateway_only(from: u64, seconds: u64) -> Vec<String> {
        (0..seconds * 10)
            .map(|i| {
                json!({"at": from + 100 * (i + 1), "gateway": 5.0, "rx": 100_000 + i,
                       "tx": 15_000_000 + 150_000 * (i + 1)})
                .to_string()
            })
            .collect()
    }

    // A second no reflector answers, as one the guard has given up in,
    // lets the upload go to a trial's ceiling: the controller holds
    // nothing then, whatever its own rate reads, so it gives no reason.
    #[test]
    fn a_controller_letting_go_gives_no_reason() {
        let mut follow = waiting();
        follow.tune(Some(Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        }));
        let mut lines = flow_queued(NOV3);
        lines.extend(gateway_only(NOV3 + 10_000, 15));
        for line in &lines {
            follow.take(&parse(line).expect("a sample"));
        }
        let last = follow.last_ms;
        let (_, rates) = follow.now(last).expect("a link");
        assert_eq!(rates.up, Some(8_500), "the ceiling wasn't let go to");
        assert_eq!(
            follow.why(last),
            Judged::Nothing,
            "a controller letting go gave a reason"
        );
    }

    // A recorder heard again after a minute, past the gap a reading spreads
    // seconds across, has its first lines read before any second after
    // its silence is stepped on: the controller's rate from before holds
    // then, its reason waiting, and no ceiling is what holds it.
    #[test]
    fn a_recorder_heard_again_waits_until_a_second_after_is_stepped() {
        let mut follow = waiting();
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        let back = NOV3 + 10_000 + 70_000;
        let lines: Vec<String> = (0..50_u64)
            .map(|i| {
                json!({"at": back + 100 * i, "reflector": "9.9.9.9", "operator": "Quad9",
                       "out": 10.0, "back": 10.0, "rtt": 20.0, "rx": 100_000 + 1_000 * i,
                       "tx": 15_000_000 + 150_000 * (i + 1)})
                .to_string()
            })
            .collect();
        let first = parse(&lines[0]).expect("a sample");
        follow.take(&first);
        let (_, rates) = follow.now(follow.last_ms).expect("a link");
        assert!(rates.up.is_some(), "the rate from before let go: {rates:?}");
        assert_eq!(
            follow.why(follow.last_ms),
            Judged::Waiting,
            "the first line heard again didn't wait for a second after"
        );
        for line in &lines[1..] {
            follow.take(&parse(line).expect("a sample"));
        }
        assert!(
            follow.why(follow.last_ms).reason().is_some(),
            "seconds heard again gave no reason"
        );
    }

    // A trial set before a stay begins tunes it too: the A/B it serves
    // runs over roams and rejoins, and each begins a stay.
    #[test]
    fn a_trial_tunes_every_stay_until_it_ends() {
        let trial = Tuning {
            late_ms: 22.0,
            ..Tuning::default()
        };
        let mut follow = waiting();
        follow.tune(Some(trial));
        for line in &flow_queued(NOV3) {
            follow.take(&parse(line).expect("a sample"));
        }
        assert_eq!(follow.controller.up.tuning(), trial, "the stay's upload");
        assert_eq!(
            follow.controller.down.tuning(),
            Tuning::default(),
            "the download is no part of the trial"
        );
        let (_, heard) = follow.heard().expect("a second heard");
        assert_eq!(heard.trial, Some(trial), "the trial isn't told: {heard:?}");
        follow.tune(None);
        assert_eq!(follow.controller.up.tuning(), Tuning::default(), "ended");
        assert_eq!(follow.heard().expect("heard").1.trial, None);
    }

    #[test]
    fn a_trial_is_read_from_what_its_file_says() {
        let folder = std::env::temp_dir().join(format!("shaper-trial-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let path = folder.join("upload.json");
        assert_eq!(trial(&path), None, "no file is no trial");
        fs::write(&path, r#"{"late_ms": 22, "hold": 5}"#).expect("written");
        assert_eq!(
            trial(&path),
            Some(Tuning {
                late_ms: 22.0,
                hold: 1.0,
                ..Tuning::default()
            }),
            "what it says, within bounds, and today's for the rest"
        );
        fs::write(&path, "late_ms = 22").expect("written");
        assert_eq!(trial(&path), None, "a file that isn't one is no trial");
        fs::write(&path, r#"{"most_kbit": 8500}"#).expect("written");
        let ceiling = trial(&path).expect("a trial").most_kbit;
        assert_eq!(ceiling, Some(8_500.0), "a ceiling just written holds");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_trial_s_ceiling_ends_ten_minutes_after_its_file_was_written() {
        let ceiling = Tuning {
            most_kbit: Some(8_500.0),
            ..Tuning::default()
        };
        let at = |s| Ok(Duration::from_secs(s));
        assert_eq!(ended(ceiling, at(599)), ceiling, "before ten minutes");
        let after = ended(ceiling, at(601));
        assert_eq!(after, Tuning::default(), "after ten minutes");
        let ahead = SystemTime::UNIX_EPOCH.duration_since(SystemTime::now());
        assert_eq!(ended(ceiling, ahead), Tuning::default(), "written ahead");
    }

    // On 11 November a shaper started at 13:17 stepped through two hours
    // recorded while it was off: its guard gave up three times on a rate
    // nothing held, so it held nothing for ten minutes, and a transfer
    // across the home network taught the upload a base of 282 Mbit/s.
    #[test]
    fn a_start_steps_on_no_second_recorded_before_it() {
        let mut follow = Follow::new();
        follow.from_ms = NOV3 + 20_000;
        let mut lines = vec![pool(NOV3, Some("home"))];
        // The upload queues after its first two seconds, so a second
        // stepped would cut it.
        for i in 0..100_u64 {
            let at = NOV3 + 100 * (i + 1);
            let out = if i < 20 { 10.0 } else { 60.0 };
            lines.push(
                json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "out": out,
                       "back": 10.0, "rtt": 20.0, "rx": 1_000 * i, "tx": 3_750_000 * (i + 1)})
                .to_string(),
            );
        }
        for line in &lines {
            follow.take(&parse(line).expect("a sample"));
        }
        assert!(
            follow.last.is_none(),
            "stepped on a second before the start"
        );
        assert_eq!(
            follow.controller.up.limit(),
            None,
            "held from a stay the shaper wasn't in"
        );
        assert_eq!(follow.network, "home", "the stay's network wasn't named");
    }

    #[test]
    fn a_silent_recorder_lets_its_link_go() {
        let follow = followed(&queued());
        assert_eq!(
            follow.now(NOV3 + 6_000 + 11_000),
            Some(("wlan0", Rates::default()))
        );
    }

    #[test]
    fn a_recording_begun_anew_starts_unlimited() {
        let mut lines = queued();
        lines.push(pool(NOV3 + 6_100, Some("hotel")));
        let follow = followed(&lines);
        assert_eq!(follow.now(NOV3 + 6_200), Some(("wlan0", Rates::default())));
    }

    #[test]
    fn an_hourly_pool_names_the_link_to_a_shaper_started_mid_stay() {
        let mut lines = vec![pool(NOV3, None)];
        lines.extend(queued().into_iter().skip(1));
        let follow = followed(&lines);
        assert!(
            follow
                .now(NOV3 + 6_000)
                .is_some_and(|(l, r)| l == "wlan0" && r.down.is_some())
        );
    }

    #[test]
    fn the_day_files_are_read_as_they_grow_and_across_midnight() {
        let folder = std::env::temp_dir().join(format!("shaper-live-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let lines = queued();
        let today = SystemTime::UNIX_EPOCH + Duration::from_millis(NOV3 + 6_000);
        let yesterday = today - Duration::from_hours(24);
        // The stay began yesterday; today's file holds its seconds, the
        // last line still half written.
        fs::write(
            crate::record::day_file(&folder, yesterday),
            format!("{}\n", lines[0]),
        )
        .expect("yesterday's file");
        let (last, rest) = lines[1..].split_last().expect("lines");
        let (half, other_half) = last.split_at(last.len() / 2);
        let path = crate::record::day_file(&folder, today);
        fs::write(&path, format!("{}\n{half}", rest.join("\n"))).expect("today's file");
        // The shaper started as the stay did.
        let started = SystemTime::UNIX_EPOCH + Duration::from_millis(NOV3);
        let mut live = Live::new(&folder, started);
        let first = live.poll(today).map(|(l, r)| (l.to_string(), r));
        assert!(
            first
                .as_ref()
                .is_some_and(|(l, r)| l == "wlan0" && r.down.is_some()),
            "{first:?}"
        );
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("today's file");
        writeln!(file, "{other_half}").expect("the rest");
        let _ = live.poll(today);
        assert_eq!(
            live.follow.now(NOV3 + 6_000).map(|(_, r)| r),
            followed(&lines).now(NOV3 + 6_000).map(|(_, r)| r)
        );
        assert_eq!(live.follow.last_ms, NOV3 + 6_000);
        let _ = fs::remove_dir_all(&folder);
    }
}
