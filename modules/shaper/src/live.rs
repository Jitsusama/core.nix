//! The one-way controller (src/oneway.rs), shaping as the recorder writes.
//! The recorder (src/record.rs) writes each probe's one-way delays to a day
//! file; the shaper reads each line the file gains, makes seconds of them
//! as a replay does (src/replay.rs), and steps the controller on each, so
//! the rates that shape a link are the ones a replay of its lines scores.
//! What each network's directions learned is kept, on disk too, so a
//! network met again, after a restart or a rejoin, starts from it.

use crate::oneway::{self, Learned, Ours};
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

/// What each network's directions learned, up and down, by its name.
pub type Known = BTreeMap<String, (Learned, Learned)>;

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
    /// The last second the controller stepped on.
    last: Option<crate::replay::Second>,
    /// The network of the stay, and what each network met learned.
    network: String,
    known: Known,
}

impl Follow {
    pub fn new(known: Known) -> Follow {
        Follow {
            link: None,
            reading: Reading::new(UNKNOWN.into()),
            controller: oneway::Controller::new(None, None),
            rates: Rates::default(),
            last_ms: 0,
            ours: BTreeMap::new(),
            vpn_ms: None,
            last: None,
            network: UNKNOWN.into(),
            known,
        }
    }

    /// What each network met learned, the stay's included.
    pub fn known(&mut self) -> &Known {
        if self.network != UNKNOWN {
            let (up, down) = self.known.get(&self.network).copied().unwrap_or_default();
            let now = (
                self.controller.up.learned().over(up),
                self.controller.down.learned().over(down),
            );
            self.known.insert(self.network.clone(), now);
        }
        &self.known
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
    /// started from what its network learned before, if it was met.
    pub fn take(&mut self, sample: &Sample) {
        self.last_ms = self.last_ms.max(sample.at());
        if let Sample::Pool { network, link, .. } = sample {
            if link.is_some() {
                self.link.clone_from(link);
            }
            if let Some(network) = network {
                self.known();
                self.reading = Reading::new(network.clone());
                let (up, down) = self.known.get(network).copied().unwrap_or_default();
                self.controller = oneway::Controller::from_learned(up, down);
                self.network.clone_from(network);
                self.rates = Rates::default();
                return;
            }
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
            self.rates = self.controller.second(&second);
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
            plateau: self.controller.up.learned().plateau,
        };
        Some((link, heard))
    }

    /// The link the recorder follows and the rates to hold it to, at a
    /// moment in ms since 1970: no limit once the recorder has gone silent.
    pub fn now(&self, at_ms: u64) -> Option<(&str, Rates)> {
        let link = self.link.as_deref()?;
        if at_ms.saturating_sub(self.last_ms) > SILENT_MS {
            return Some((link, Rates::default()));
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
    /// Where what each network learned is kept, and what was last written.
    kept: PathBuf,
    written: Known,
    /// A call's round trip through the VPN riding the link.
    vpn: Probe,
}

impl Live {
    /// Starts at yesterday's file, so a stay begun before midnight is known
    /// from its first line, and from what each network learned, kept in
    /// `kept`.
    pub fn new(folder: &Path, now: SystemTime, kept: &Path) -> Live {
        let yesterday = now.checked_sub(Duration::from_hours(24)).unwrap_or(now);
        let written: Known = fs::read_to_string(kept)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Live {
            folder: folder.to_path_buf(),
            file: (day_file(folder, yesterday), 0),
            partial: Vec::new(),
            follow: Follow::new(written.clone()),
            kept: kept.to_path_buf(),
            written,
            vpn: Probe::default(),
        }
    }

    /// Writes what each network learned when a rate in it moved by more
    /// than a twentieth, so a rate that wanders doesn't wear the disk.
    fn keep(&mut self) {
        let known = self.follow.known();
        let moved = known.iter().any(|(network, &(up, down))| {
            let (was_up, was_down) = self.written.get(network).copied().unwrap_or_default();
            up.moved(was_up) || down.moved(was_down)
        });
        if !moved {
            return;
        }
        self.written = known.clone();
        let text = serde_json::to_string_pretty(&self.written).unwrap_or_default();
        let fresh = self.kept.with_extension("json.new");
        if fs::write(&fresh, text)
            .and_then(|()| fs::rename(&fresh, &self.kept))
            .is_err()
        {
            eprintln!("couldn't write {}", self.kept.display());
        }
    }

    /// Takes whose a link's upload queue is now.
    pub fn see(&mut self, link: &str, ours: Ours) {
        self.follow.see(link, ours);
    }

    /// The link the controller holds, and what it last heard of it.
    pub fn heard(&self) -> Option<(&str, Heard)> {
        self.follow.heard()
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
        self.keep();
        self.follow.now(ms(now))
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
        let mut follow = Follow::new(Known::new());
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
    fn waiting(known: Known) -> Follow {
        let mut follow = Follow::new(known);
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

    #[test]
    fn a_network_met_again_starts_held_under_its_router() {
        let mut follow = waiting(Known::new());
        let mut lines = flow_queued(NOV3);
        lines.push(pool(NOV3 + 11_000, Some("hotel")));
        lines.push(pool(NOV3 + 12_000, Some("home")));
        for line in &lines {
            follow.take(&parse(line).expect("a sample"));
        }
        let up = follow.controller.up.limit();
        assert!(
            up.is_some_and(|up| (11_000..12_000).contains(&up)),
            "home started at {up:?}"
        );
        let hotel = follow.known().get("hotel").copied().unwrap_or_default();
        assert_eq!(hotel.0.plateau, None, "the hotel took home's plateau");
    }

    #[test]
    fn a_queue_past_cake_that_the_probes_cant_see_is_taken_back_live() {
        let lines = flow_queued(NOV3);
        let mut follow = Follow::new(Known::new());
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
        let mut follow = waiting(Known::new());
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
            follow.controller.up.learned().plateau,
            "the plateau: {heard:?}"
        );
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
        let mut live = Live::new(&folder, today, &folder.join("learned.json"));
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

    #[test]
    fn a_restart_starts_a_network_from_what_it_learned() {
        let folder = std::env::temp_dir().join(format!("shaper-kept-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let kept = folder.join("learned.json");
        let today = SystemTime::UNIX_EPOCH + Duration::from_millis(NOV3 + 10_000);
        let lines = flow_queued(NOV3);
        fs::write(
            crate::record::day_file(&folder, today),
            format!("{}\n", lines.join("\n")),
        )
        .expect("today's file");
        let mut live = Live::new(&folder, today, &kept);
        live.see(
            "wlan0",
            Ours {
                queued: false,
                waiting: true,
                ..Ours::default()
            },
        );
        let _ = live.poll(today);
        let learned = Live::new(&folder, today, &kept).written;
        let up = learned.get("home").and_then(|(up, _)| up.plateau);
        assert!(
            up.is_some_and(|up| (11_000.0..13_000.0).contains(&up)),
            "kept {learned:?}"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
