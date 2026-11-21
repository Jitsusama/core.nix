//! A day of the network's figures, one line a minute, kept beside the
//! record in `$XDG_STATE_HOME/net/day.jsonl` so net can say what changed
//! over the day and not only what is. Only figures that climb or swing
//! are kept: what each of the modem's channels lost beyond repair, and
//! each radio's busy share. A line that doesn't parse is skipped, so a
//! half-written last line after a crash costs one minute, not the day.
//! What the day says is filled in beside the record here (`recall`); the
//! concerns that say it are worked out after, in src/room.rs.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Channel, Modem, Network};
use crate::sources::modem;

/// How far back the day reaches.
pub const DAY: u64 = 24 * 60 * 60;

/// How far back the day's readings must reach before net says what a
/// channel lost lately: less is a few minutes' noise, not a trend.
const LATELY: u64 = 60 * 60;

/// How far past a day the file may run before it's rewritten without its
/// oldest lines: rewriting each minute would write the whole day each
/// time, so it's trimmed about once an hour instead.
const SLACK: u64 = 60 * 60;

/// The network's swinging figures at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moment {
    pub at: u64,
    pub lost: Vec<Lost>,
    pub busy: Vec<Busy>,
}

/// What one of the modem's downstream channels had lost beyond repair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lost {
    pub kind: String,
    pub channel: Option<u32>,
    /// A day file written before net kept it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_hz: Option<u64>,
    pub uncorrectable: u64,
}

impl Lost {
    /// Whether this is the count of that channel.
    pub fn of(&self, channel: &Channel) -> bool {
        self.is(&channel.kind, channel.channel, channel.frequency_hz)
    }

    /// Whether this and `other` count the same channel.
    fn same(&self, other: &Self) -> bool {
        self.is(&other.kind, other.channel, other.frequency_hz)
    }

    /// A numbered channel is told apart by its kind and number, so a line
    /// kept before net kept frequencies still counts it. An OFDM channel
    /// has no number and a modem locks to more than one, so only its
    /// frequency tells it apart, and an old line counts none of them.
    fn is(&self, kind: &str, channel: Option<u32>, frequency_hz: Option<u64>) -> bool {
        self.kind == kind
            && self.channel == channel
            && (channel.is_some() || self.frequency_hz == frequency_hz)
    }
}

/// How busy one radio was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Busy {
    pub device: String,
    pub radio: String,
    pub pct: u32,
}

impl Moment {
    /// What a modem's downstream channels say they've lost.
    pub fn lost_of(modem: &Modem) -> Vec<Lost> {
        modem
            .downstream
            .iter()
            .filter_map(|channel| {
                Some(Lost {
                    kind: channel.kind.clone(),
                    channel: channel.channel,
                    frequency_hz: channel.frequency_hz,
                    uncorrectable: channel.uncorrectable?,
                })
            })
            .collect()
    }

    /// The figures of a record, as it was collected.
    pub fn of(network: &Network) -> Self {
        let lost = network
            .devices
            .iter()
            .filter(|device| device.id == modem::ID)
            .filter_map(|device| device.modem.as_ref())
            .flat_map(Self::lost_of)
            .collect();
        let busy = network
            .devices
            .iter()
            .flat_map(|device| {
                device.radios.iter().filter_map(|radio| {
                    Some(Busy {
                        device: device.id.clone(),
                        radio: radio.name.clone(),
                        pct: radio.busy_pct?,
                    })
                })
            })
            .collect();
        Self {
            at: network.collected_at,
            lost,
            busy,
        }
    }
}

/// The day's moments up to now, oldest first.
pub fn read(file: &Path, now: u64) -> Vec<Moment> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Moment>(line).ok())
        .filter(|moment| moment.at + DAY >= now && moment.at <= now)
        .collect()
}

/// Adds a moment to the day's file, trimming the file to the day once its
/// oldest line is more than an hour past it.
pub fn keep(file: &Path, moment: &Moment) -> io::Result<()> {
    let line = serde_json::to_string(moment).map_err(io::Error::other)? + "\n";
    let oldest = fs::read_to_string(file)
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| serde_json::from_str::<Moment>(line).ok())
        })
        .map(|first| first.at);
    if oldest.is_some_and(|at| at + DAY + SLACK < moment.at) {
        let mut kept: String = read(file, moment.at)
            .iter()
            .filter_map(|each| serde_json::to_string(each).ok())
            .map(|each| each + "\n")
            .collect();
        kept.push_str(&line);
        let fresh = file.with_extension("jsonl.new");
        fs::write(&fresh, kept)?;
        return fs::rename(fresh, file);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?
        .write_all(line.as_bytes())
}

/// Fills in what the day's readings say beside the record: what the
/// modem's channels lost lately, and each radio's busiest reading and
/// when. The concerns that say them are worked out after it.
pub fn recall(network: &mut Network, day: &[Moment]) {
    let now = network.collected_at;
    for device in &mut network.devices {
        if let Some(modem) = device.modem.as_mut() {
            lately(modem, day, now);
        }
        for radio in &mut device.radios {
            // The earliest of equal readings, so "its busiest" names when
            // it first got there.
            let busiest = day
                .iter()
                .filter_map(|moment| {
                    moment
                        .busy
                        .iter()
                        .find(|busy| busy.device == device.id && busy.radio == radio.name)
                        .map(|busy| (busy.pct, moment.at))
                })
                .chain(radio.busy_pct.map(|pct| (pct, now)))
                .max_by_key(|(pct, at)| (*pct, std::cmp::Reverse(*at)));
            radio.busiest_pct = busiest.map(|(pct, _)| pct);
            radio.busiest_at = busiest.map(|(_, at)| at);
        }
    }
}

/// What each downstream channel lost since the day's earliest reading
/// after the modem last started: a start sets every count back to zero,
/// so the walk back stops at the first reading where every channel that
/// had counted any reads higher than in the one after it. One channel
/// reading lower alone is the page misreading it, which it does now and
/// then, often as a zero.
fn lately(modem: &mut Modem, day: &[Moment], now: u64) {
    let mut later: Vec<Lost> = Moment::lost_of(modem);
    let mut base: Option<&Moment> = None;
    for moment in day.iter().rev().filter(|moment| moment.at < now) {
        let mut counted = moment
            .lost
            .iter()
            .filter(|earlier| earlier.uncorrectable > 0)
            .filter_map(|earlier| {
                let each = later.iter().find(|each| each.same(earlier))?;
                Some(earlier.uncorrectable > each.uncorrectable)
            })
            .peekable();
        let started_since = counted.peek().is_some() && counted.all(|fell| fell);
        if started_since {
            break;
        }
        base = Some(moment);
        later.clone_from(&moment.lost);
    }
    let base = base.filter(|base| now - base.at >= LATELY);
    modem.lately_s = base.map(|base| now - base.at);
    for channel in &mut modem.downstream {
        channel.lost_lately = base.and_then(|base| {
            let then = base.lost.iter().find(|each| each.of(channel))?;
            Some(channel.uncorrectable?.saturating_sub(then.uncorrectable))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::model::ConcernKind;
    use crate::room::tests::{busy, device, earlier, network};
    use crate::room::work_out;

    /// A day file of the test's own, in a folder no other test uses.
    fn day_file(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("net-history-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("the test's folder can be made");
        folder.join("day.jsonl")
    }

    fn moment(at: u64, lost: u64) -> Moment {
        Moment {
            at,
            lost: vec![Lost {
                kind: "sc-qam".into(),
                channel: Some(16),
                frequency_hz: Some(483_000_000),
                uncorrectable: lost,
            }],
            busy: vec![Busy {
                device: "eero:2003".into(),
                radio: "5 GHz high".into(),
                pct: 40,
            }],
        }
    }

    #[test]
    fn what_is_kept_is_read_back_in_order() {
        let file = day_file("kept");
        for at in [100, 160, 220] {
            keep(&file, &moment(at, at)).expect("a moment is kept");
        }
        let day = read(&file, 220);
        let ats: Vec<u64> = day.iter().map(|moment| moment.at).collect();
        assert_eq!(ats, [100, 160, 220], "read back: {day:?}");
        assert_eq!(day[0], moment(100, 100), "a moment reads back whole");
    }

    #[test]
    fn only_the_last_day_is_read() {
        let file = day_file("day");
        keep(&file, &moment(1_000, 1)).expect("a moment is kept");
        keep(&file, &moment(1_000 + DAY, 2)).expect("a moment is kept");
        let day = read(&file, 1_001 + DAY);
        assert_eq!(day.len(), 1, "the first is past a day: {day:?}");
        assert_eq!(day[0].at, 1_000 + DAY);
    }

    #[test]
    fn the_file_is_trimmed_once_its_oldest_line_is_an_hour_past_the_day() {
        let file = day_file("trim");
        keep(&file, &moment(1_000, 1)).expect("a moment is kept");
        keep(&file, &moment(1_000 + DAY + SLACK, 2)).expect("a moment is kept");
        let lines = fs::read_to_string(&file)
            .expect("the file reads")
            .lines()
            .count();
        assert_eq!(lines, 2, "an hour past the day isn't trimmed yet");
        keep(&file, &moment(1_001 + DAY + SLACK, 3)).expect("a moment is kept");
        let text = fs::read_to_string(&file).expect("the file reads");
        assert_eq!(text.lines().count(), 2, "the oldest is dropped: {text}");
        assert!(
            !file.with_extension("jsonl.new").exists(),
            "the rewrite leaves nothing beside the file"
        );
    }

    #[test]
    fn a_line_that_does_not_parse_costs_only_itself() {
        let file = day_file("torn");
        keep(&file, &moment(100, 1)).expect("a moment is kept");
        fs::write(
            &file,
            fs::read_to_string(&file).expect("the file reads") + "{\"at\": 16\n",
        )
        .expect("a torn line is written");
        keep(&file, &moment(220, 3)).expect("a moment is kept after it");
        let ats: Vec<u64> = read(&file, 220).iter().map(|moment| moment.at).collect();
        assert_eq!(ats, [100, 220], "the torn line alone is skipped");
    }

    #[test]
    fn no_file_is_an_empty_day() {
        let file = day_file("none");
        assert!(read(&file, 100).is_empty(), "nothing kept yet");
    }

    const HOUR: u64 = 60 * 60;

    /// The golden record with the day recalled and its concerns worked
    /// out over it, as the daemon does each minute.
    fn recalled(mut network: Network, day: &[Moment]) -> Network {
        recall(&mut network, day);
        work_out(&mut network);
        network
    }

    fn modem(network: &Network) -> &Modem {
        device(network, "modem")
            .modem
            .as_ref()
            .expect("the record has the modem's channels")
    }

    fn sixteen(network: &Network) -> &Channel {
        modem(network)
            .downstream
            .iter()
            .find(|channel| channel.channel == Some(16))
            .expect("the modem has channel 16")
    }

    fn says(network: &Network, kind: ConcernKind) -> &str {
        &network
            .concerns
            .iter()
            .find(|concern| concern.kind == kind)
            .unwrap_or_else(|| panic!("a {kind:?} concern: {:?}", network.concerns))
            .says
    }

    #[test]
    fn what_a_channel_lost_lately_counts_from_the_days_first_reading() {
        // 800 five hours ago and 878 now: 78 lost in the last 5 hours.
        let now = network();
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            earlier(&now, HOUR, 850, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(modem(&network).lately_s, Some(5 * HOUR));
        assert_eq!(sixteen(&network).lost_lately, Some(78));
        assert_eq!(
            says(&network, ConcernKind::Lost),
            "The modem's downstream channel 16 has lost 878 pieces of data it couldn't repair \
             since the modem started, against at most 18 on any other, 78 of them in the last 5 \
             hours; Example ISP can check the line."
        );
    }

    #[test]
    fn each_ofdm_channel_counts_what_it_lost_on_its_own() {
        // Neither has a number, so only the frequency tells them apart.
        let now = network();
        let mut then = earlier(&now, 5 * HOUR, 878, 41);
        for lost in &mut then.lost {
            if lost.frequency_hz == Some(635_600_000) {
                lost.uncorrectable = 10;
            }
        }
        let network = recalled(now, &[then]);
        let lately: Vec<(Option<u64>, Option<u64>)> = modem(&network)
            .downstream
            .iter()
            .filter(|channel| channel.kind == "ofdm")
            .map(|channel| (channel.frequency_hz, channel.lost_lately))
            .collect();
        assert_eq!(
            lately,
            [(Some(455_600_000), Some(0)), (Some(635_600_000), Some(8))],
            "one OFDM channel's loss was counted against the other's"
        );
    }

    #[test]
    fn a_channel_that_lost_nothing_lately_is_no_concern() {
        // Its counts stay in the record for whoever reads the channels; a
        // concern is what's worth a look now, and a calm line isn't.
        let now = network();
        let day = [earlier(&now, 5 * HOUR, 878, 41)];
        let network = recalled(now, &day);
        assert_eq!(sixteen(&network).lost_lately, Some(0));
        assert_eq!(sixteen(&network).uncorrectable, Some(878));
        assert!(
            network
                .concerns
                .iter()
                .all(|concern| concern.kind != ConcernKind::Lost),
            "a channel losing nothing lately is still a concern: {:?}",
            network.concerns
        );
    }

    /// Sets what the channel at `frequency_hz` had lost in a moment.
    fn ofdm(moment: &mut Moment, frequency_hz: u64, uncorrectable: u64) {
        moment
            .lost
            .iter_mut()
            .find(|lost| lost.frequency_hz == Some(frequency_hz))
            .expect("the moment has the OFDM channel")
            .uncorrectable = uncorrectable;
    }

    #[test]
    fn lately_counts_only_since_the_modem_last_started() {
        // Five hours ago every channel that had counted any read higher
        // than two hours ago, so the modem started again between; it
        // counts from 860 two hours ago.
        let now = network();
        let mut before = earlier(&now, 5 * HOUR, 900, 41);
        ofdm(&mut before, 455_600_000, 30);
        ofdm(&mut before, 635_600_000, 30);
        let day = [before, earlier(&now, 2 * HOUR, 860, 41)];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(2 * HOUR),
            "the walk back went past the start"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(18),
            "channel 16 counted from the wrong reading"
        );
    }

    #[test]
    fn one_channel_reading_low_is_no_start() {
        // The page misreads an OFDM channel's 18 as 0 now and then. Only
        // that channel fell, so the modem didn't start: lately still
        // reaches five hours back.
        let now = network();
        let mut misread = earlier(&now, 2 * HOUR, 850, 41);
        ofdm(&mut misread, 455_600_000, 0);
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            earlier(&now, 3 * HOUR, 840, 41),
            misread,
        ];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "a misread channel was taken as the modem starting"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(78),
            "channel 16 counted from the wrong reading"
        );
    }

    #[test]
    fn a_day_kept_before_frequencies_still_counts_a_numbered_channel() {
        // A line written before net kept frequencies has none, so it can't
        // tell the two OFDM channels apart, but channel 16's number still
        // names it: its loss lately is said, and the OFDM channels' isn't.
        let now = network();
        let mut old = earlier(&now, 5 * HOUR, 800, 41);
        for lost in &mut old.lost {
            lost.frequency_hz = None;
        }
        let line = serde_json::to_string(&old).expect("a moment writes");
        assert!(
            !line.contains("frequency_hz"),
            "the old line still names a frequency: {line}"
        );
        let old: Moment = serde_json::from_str(&line).expect("an old line reads");
        let network = recalled(now, &[old]);
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(78),
            "an old line no longer counts channel 16"
        );
        assert!(
            modem(&network)
                .downstream
                .iter()
                .filter(|channel| channel.kind == "ofdm")
                .all(|channel| channel.lost_lately.is_none()),
            "an old line was taken for one OFDM channel or the other"
        );
        assert!(
            says(&network, ConcernKind::Lost).contains("78 of them in the last 5 hours"),
            "the concern lost what channel 16 lost lately"
        );
    }

    #[test]
    fn under_an_hour_of_readings_says_nothing_of_lately() {
        let now = network();
        let before = recalled(now.clone(), &[]).concerns;
        let day = [earlier(&now, HOUR / 2, 800, 41)];
        let network = recalled(now, &day);
        assert_eq!(modem(&network).lately_s, None);
        assert_eq!(sixteen(&network).lost_lately, None);
        assert_eq!(network.concerns, before, "half an hour is too short to say");
    }

    #[test]
    fn a_radios_busiest_reading_of_the_day_is_kept_with_when() {
        let mut now = network();
        busy(&mut now, "eero:2003", 55);
        let day = [earlier(&now, 3 * HOUR, 70, 70), earlier(&now, HOUR, 45, 45)];
        let network = recalled(now, &day);
        let hall = device(&network, "eero:2003")
            .radios
            .iter()
            .find(|radio| radio.name == "5 GHz high")
            .expect("Hall has a 5 GHz high radio");
        assert_eq!(hall.busiest_pct, Some(70));
        assert_eq!(hall.busiest_at, Some(network.collected_at - 3 * HOUR));
        assert_eq!(
            says(&network, ConcernKind::Busy),
            "Hall's 5 GHz high radio, on channel 149, is busy 55% of the time; 70% at its \
             busiest, 3 hours ago."
        );
    }

    #[test]
    fn a_radio_busiest_now_says_so_once() {
        let mut now = network();
        busy(&mut now, "eero:2003", 55);
        let day = [earlier(&now, HOUR, 878, 45)];
        let network = recalled(now, &day);
        assert_eq!(
            says(&network, ConcernKind::Busy),
            "Hall's 5 GHz high radio, on channel 149, is busy 55% of the time."
        );
    }
}
