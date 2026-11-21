//! A day of the network's figures, one line a minute, kept beside the
//! record in `$XDG_STATE_HOME/net/day.jsonl` so net can say what changed
//! over the day and not only what is. Only figures that climb or swing
//! are kept: what each of the modem's channels lost beyond repair, and
//! each radio's busy share. A line that doesn't parse is skipped, so a
//! half-written last line after a crash costs one minute, not the day.
//! What the day says is filled in beside the record here (`recall`); the
//! concerns that say it are worked out after, in src/room.rs.
//!
//! Long because the modem's page misreads counts and uptimes now and
//! then, and each way it does has a case below proving the walk back
//! through the day isn't fooled by it.

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

/// How much earlier a line may say the modem started than it last did
/// and still mean the same start: each is a reading's time less the
/// modem's uptime, and a reading taken under load lands some seconds
/// late.
const ONE_START: u64 = 5 * 60;

/// The network's swinging figures at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moment {
    pub at: u64,
    /// When the modem last started, by net's clock: the moment less the
    /// modem's uptime. A line kept before net kept it, or in a minute the
    /// modem didn't answer, has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modem_started: Option<u64>,
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

    /// Channels are told apart by kind, number and frequency, since the
    /// provider can move a numbered channel to another frequency. A line
    /// kept before net kept frequencies has none, so there a numbered
    /// channel is told apart by its number alone. An OFDM channel has no
    /// number and a modem locks to more than one, so without a frequency
    /// on both sides it matches nothing.
    fn is(&self, kind: &str, channel: Option<u32>, frequency_hz: Option<u64>) -> bool {
        self.kind == kind
            && self.channel == channel
            && match (self.frequency_hz, frequency_hz) {
                (Some(this), Some(that)) => this == that,
                _ => channel.is_some(),
            }
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
        let answered = network
            .devices
            .iter()
            .find(|device| device.id == modem::ID && device.modem.is_some());
        let lost = answered
            .and_then(|device| device.modem.as_ref())
            .map(Self::lost_of)
            .unwrap_or_default();
        let modem_started = answered
            .and_then(|device| device.up_s)
            .map(|up| network.collected_at.saturating_sub(up));
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
            modem_started,
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
        let started = device.up_s.map(|up| now.saturating_sub(up));
        if let Some(modem) = device.modem.as_mut() {
            lately(modem, started, day, now);
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
/// after the modem last started. The walk back stops at a line saying
/// the modem started over five minutes before it last did, or where
/// every channel that had counted any reads lower after it, which is a
/// start too though one the modem may not count. The page misreads an
/// uptime or a count now and then, often several channels in one
/// minute, so either needs two lines to show it: the line before one
/// that says it started earlier must say so too, and counts must read
/// lower in both newer lines. The newest line has only now after it, so
/// its counts are judged against now alone: in the first minute after a
/// restart the older lines still outvote now's new start, and only the
/// counts show it, while a misread now costs at most a minute with no
/// count lately. A minute the modem didn't answer says nothing of it,
/// so the walk passes it by.
fn lately(modem: &mut Modem, started: Option<u64>, day: &[Moment], now: u64) {
    let lines: Vec<&Moment> = day
        .iter()
        .rev()
        .filter(|moment| moment.at < now && !moment.lost.is_empty())
        .collect();
    let latest = latest_start(started, &lines);
    let before_start = |moment: &Moment| {
        moment
            .modem_started
            .zip(latest)
            .is_some_and(|(then, latest)| latest > then + ONE_START)
    };
    let mut newer: Vec<Vec<Lost>> = vec![Moment::lost_of(modem)];
    let mut since: Vec<&Moment> = Vec::new();
    for (at, &moment) in lines.iter().enumerate() {
        let started_since =
            before_start(moment) && lines.get(at + 1).copied().is_none_or(before_start);
        let fell = newer
            .iter()
            .rev()
            .take(2)
            .all(|later| all_fell(&moment.lost, later));
        if started_since || fell {
            break;
        }
        since.push(moment);
        newer.push(moment.lost.clone());
    }
    let base = since.last().filter(|base| now - base.at >= LATELY);
    modem.lately_s = base.map(|base| now - base.at);
    for channel in &mut modem.downstream {
        channel.lost_lately = base.and_then(|_| {
            let then = first_count(&since, channel)?;
            Some(channel.uncorrectable?.saturating_sub(then))
        });
    }
}

/// When the modem last started: the middle of what now and the two
/// newest lines say, so one misread uptime now doesn't move it; with
/// fewer, what now says, or else the newest line.
fn latest_start(started: Option<u64>, lines: &[&Moment]) -> Option<u64> {
    let mut starts: Vec<u64> = started
        .into_iter()
        .chain(lines.iter().filter_map(|moment| moment.modem_started))
        .take(3)
        .collect();
    if starts.len() == 3 {
        starts.sort_unstable();
        return Some(starts[1]);
    }
    starts.first().copied()
}

/// How long after the stretch's first line its first readings may come:
/// the day keeps a line a minute, so three, with room for a late one.
const FIRST_READINGS: u64 = 3 * 60;

/// What a channel had lost when the stretch began, from its readings in
/// the stretch's first three lines: the middle of three, or the higher
/// of two, since counts only climb and a misread reads low, so one is
/// outvoted at the cost of at most two minutes' losses. Only lines
/// within its first minutes count, and a channel they don't name has no
/// count lately, since one taken later would cover less than the
/// stretch a warning names.
fn first_count(since: &[&Moment], channel: &Channel) -> Option<u64> {
    let first = since.last()?.at;
    let mut firsts: Vec<u64> = since
        .iter()
        .rev()
        .take_while(|moment| moment.at <= first + FIRST_READINGS)
        .take(3)
        .filter_map(|moment| moment.lost.iter().find(|each| each.of(channel)))
        .map(|each| each.uncorrectable)
        .collect();
    firsts.sort_unstable();
    match firsts[..] {
        [_, middle, _] => Some(middle),
        [.., highest] => Some(highest),
        [] => None,
    }
}

/// Whether every channel that had counted any reads lower in the later
/// reading, as after a start, which sets every count back to zero. One
/// channel reading lower among others is the page misreading it, which
/// it does now and then, often as a zero.
fn all_fell(earlier: &[Lost], later: &[Lost]) -> bool {
    let counted: Vec<bool> = earlier
        .iter()
        .filter(|earlier| earlier.uncorrectable > 0)
        .filter_map(|earlier| {
            let each = later.iter().find(|each| each.same(earlier))?;
            Some(earlier.uncorrectable > each.uncorrectable)
        })
        .collect();
    !counted.is_empty() && counted.iter().all(|fell| *fell)
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
            modem_started: None,
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
        // Five hours ago the modem said it had started a day before it
        // says now, so it started again between, though channel 16 then
        // read lower than since; it counts from 860 two hours ago.
        let now = network();
        let mut before = earlier(&now, 5 * HOUR, 10, 41);
        before.modem_started = before.modem_started.map(|started| started - DAY);
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

    /// The golden record as read `ago` seconds earlier, its uptime short
    /// by the same less `drift`, so it says the modem started `drift`
    /// seconds before the record does.
    fn read_earlier(now: &Network, ago: u64, drift: u64) -> Network {
        let mut then = now.clone();
        then.collected_at -= ago;
        let modem = then
            .devices
            .iter_mut()
            .find(|device| device.id == "modem")
            .expect("the record has the modem");
        modem.up_s = modem.up_s.map(|up| up - ago + drift);
        then
    }

    #[test]
    fn a_start_is_when_the_modem_started_moving_later_by_over_five_minutes() {
        let now = network();
        assert_eq!(
            Moment::of(&read_earlier(&now, 2 * HOUR, 0)).modem_started,
            Moment::of(&now).modem_started,
            "a start is a reading's time less the modem's uptime"
        );
        for (drift, lately_s) in [(ONE_START, Some(5 * HOUR)), (ONE_START + 1, None)] {
            let day = [Moment::of(&read_earlier(&now, 5 * HOUR, drift))];
            let network = recalled(now.clone(), &day);
            assert_eq!(
                modem(&network).lately_s,
                lately_s,
                "a start {drift} seconds later was judged wrongly"
            );
        }
    }

    #[test]
    fn a_start_in_the_middle_of_the_day_is_found_by_uptime() {
        // Two lines say the modem started a day before the latest line
        // does, and the walk stops at the newer of them, not the oldest.
        let now = network();
        let mut day: Vec<Moment> = [(6 * HOUR, 5), (5 * HOUR, 10)]
            .into_iter()
            .map(|(ago, sixteen)| {
                let mut before = earlier(&now, ago, sixteen, 41);
                before.modem_started = before.modem_started.map(|started| started - DAY);
                before
            })
            .collect();
        day.push(earlier(&now, 2 * HOUR, 860, 41));
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(2 * HOUR),
            "the walk stopped somewhere other than the start"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(18),
            "channel 16 counted from a line before the start"
        );
    }

    #[test]
    fn a_restart_in_the_last_minute_is_seen_by_its_counts() {
        // The modem started thirty seconds ago, so every count reads 0. The
        // day's lines still outvote now's new start, so the counts falling
        // against now alone are what show it.
        let mut now = network();
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            earlier(&now, 3 * HOUR, 840, 41),
        ];
        let modem_now = now
            .devices
            .iter_mut()
            .find(|device| device.id == "modem")
            .expect("the record has the modem");
        modem_now.up_s = Some(30);
        for channel in &mut modem_now
            .modem
            .as_mut()
            .expect("the modem has its channels")
            .downstream
        {
            channel.uncorrectable = Some(0);
        }
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            None,
            "a restart in the last minute was walked past"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            None,
            "channel 16 counted across a restart"
        );
    }

    #[test]
    fn a_misread_uptime_is_passed_by() {
        // One line's uptime read ten minutes short, so it says the modem
        // started ten minutes after it did; that is no start.
        let now = network();
        let mut misread = earlier(&now, 3 * HOUR, 840, 41);
        misread.modem_started = misread.modem_started.map(|started| started + 10 * 60);
        let day = [earlier(&now, 5 * HOUR, 800, 41), misread];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "a misread uptime was taken as the modem starting"
        );
    }

    #[test]
    fn both_ofdm_channels_misread_together_is_no_start() {
        // Soon after a start only the OFDM channels have counted any. The
        // page misreads both as 0 in one minute more often than either
        // alone, and that is no start, since the next minute reads 18.
        let mut now = network();
        let channels = now
            .devices
            .iter_mut()
            .find(|device| device.id == "modem")
            .and_then(|device| device.modem.as_mut())
            .expect("the record has the modem's channels");
        for channel in &mut channels.downstream {
            if channel.kind != "ofdm" {
                channel.uncorrectable = Some(0);
            }
        }
        let mut misread = earlier(&now, 3 * HOUR, 0, 41);
        ofdm(&mut misread, 455_600_000, 0);
        ofdm(&mut misread, 635_600_000, 0);
        let day = [earlier(&now, 5 * HOUR, 0, 41), misread];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "one minute misreading both counting channels was taken as a start"
        );
    }

    #[test]
    fn an_old_line_whose_uptime_read_long_is_passed_by() {
        // One line's uptime read ten minutes long, so it says the modem
        // started ten minutes before it did; the line before it says the
        // start it did, so that is no start.
        let now = network();
        let mut misread = earlier(&now, 4 * HOUR, 820, 41);
        misread.modem_started = misread.modem_started.map(|started| started - 10 * 60);
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            misread,
            earlier(&now, 3 * HOUR, 840, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "an uptime misread long was taken as the modem starting"
        );
    }

    #[test]
    fn an_uptime_misread_now_is_outvoted_by_the_newest_lines() {
        // Now's uptime reads ten minutes short, so now says the modem
        // started ten minutes after the day's lines do; they outvote it.
        let mut now = network();
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            earlier(&now, 3 * HOUR, 840, 41),
            earlier(&now, 2 * HOUR, 860, 41),
        ];
        let modem_now = now
            .devices
            .iter_mut()
            .find(|device| device.id == "modem")
            .expect("the record has the modem");
        modem_now.up_s = modem_now.up_s.map(|up| up - 10 * 60);
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "an uptime misread now was taken as the modem starting"
        );
    }

    #[test]
    fn a_channel_the_first_lines_lack_has_no_count_lately() {
        // The first three lines were kept before net kept frequencies, so
        // they can't name either OFDM channel: counting one from a later
        // line would cover less than the five hours a warning would name.
        let now = network();
        let mut day: Vec<Moment> = (0..3)
            .map(|minute| {
                let mut old = earlier(&now, 5 * HOUR - minute * 60, 800, 41);
                old.modem_started = None;
                for lost in &mut old.lost {
                    lost.frequency_hz = None;
                }
                old
            })
            .collect();
        day.push(earlier(&now, 2 * HOUR, 850, 41));
        let network = recalled(now, &day);
        let ofdm_lately: Vec<Option<u64>> = modem(&network)
            .downstream
            .iter()
            .filter(|channel| channel.channel.is_none())
            .map(|channel| channel.lost_lately)
            .collect();
        assert_eq!(
            ofdm_lately,
            [None, None],
            "an OFDM channel was counted over less than the stretch named"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(78),
            "channel 16 lost its count"
        );
    }

    #[test]
    fn first_readings_come_only_from_the_stretchs_first_three_minutes() {
        // The first line was kept before net kept frequencies. A line that
        // names the OFDM channels counts for them only within three
        // minutes of it, since one later would cover less than the
        // stretch a warning names.
        let now = network();
        for (after, ofdm_lately) in [(FIRST_READINGS, Some(0)), (FIRST_READINGS + 1, None)] {
            let mut old = earlier(&now, 5 * HOUR, 800, 41);
            for lost in &mut old.lost {
                lost.frequency_hz = None;
            }
            let day = [
                old,
                earlier(&now, 5 * HOUR - after, 820, 41),
                earlier(&now, 2 * HOUR, 850, 41),
            ];
            let network = recalled(now.clone(), &day);
            let lately: Vec<Option<u64>> = modem(&network)
                .downstream
                .iter()
                .filter(|channel| channel.kind == "ofdm")
                .map(|channel| channel.lost_lately)
                .collect();
            assert_eq!(
                lately,
                [ofdm_lately, ofdm_lately],
                "a line {after} seconds after the first was counted wrongly"
            );
        }
    }

    #[test]
    fn a_misread_first_reading_is_outvoted() {
        // The day's first reading misread 455.6 MHz's 18 as 0; the next
        // two outvote it, so the channel lost none lately, not 18.
        let now = network();
        let mut misread = earlier(&now, 5 * HOUR, 878, 41);
        ofdm(&mut misread, 455_600_000, 0);
        let day = [
            misread,
            earlier(&now, 5 * HOUR - 60, 878, 41),
            earlier(&now, 5 * HOUR - 120, 878, 41),
        ];
        let network = recalled(now, &day);
        let lately = modem(&network)
            .downstream
            .iter()
            .find(|channel| channel.frequency_hz == Some(455_600_000))
            .and_then(|channel| channel.lost_lately);
        assert_eq!(lately, Some(0), "a misread first reading was taken as real");
    }

    #[test]
    fn a_day_kept_before_start_times_sees_a_start_through_a_silent_minute() {
        // With no start time kept, a start is every counting channel
        // reading lower after it. While the modem restarted it didn't
        // answer, and that minute's empty line stands for no reading.
        let now = network();
        let mut before = earlier(&now, 5 * HOUR, 900, 41);
        ofdm(&mut before, 455_600_000, 30);
        ofdm(&mut before, 635_600_000, 30);
        let mut silent = earlier(&now, 3 * HOUR, 0, 41);
        silent.lost.clear();
        let day = [before, silent, earlier(&now, 2 * HOUR, 860, 41)].map(|mut moment| {
            moment.modem_started = None;
            moment
        });
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(2 * HOUR),
            "the walk back went past the start"
        );
    }

    #[test]
    fn a_channel_is_told_apart_by_frequency_whenever_both_lines_name_one() {
        let lost = |kind: &str, channel, frequency_hz| Lost {
            kind: kind.into(),
            channel,
            frequency_hz,
            uncorrectable: 0,
        };
        let cases = [
            (
                lost("sc-qam", Some(16), Some(483_000_000)),
                Some(489_000_000),
                false,
            ),
            (lost("sc-qam", Some(16), None), Some(483_000_000), true),
            (lost("ofdm", None, None), None, false),
            (
                lost("ofdm", None, Some(455_600_000)),
                Some(455_600_000),
                true,
            ),
            (
                lost("ofdm", None, Some(455_600_000)),
                Some(635_600_000),
                false,
            ),
        ];
        for (one, frequency_hz, same) in cases {
            let other = Lost {
                frequency_hz,
                ..one.clone()
            };
            assert_eq!(
                one.same(&other),
                same,
                "{one:?} against {frequency_hz:?} was matched wrongly"
            );
        }
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
            earlier(&now, 5 * HOUR - 60, 800, 41),
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
        old.modem_started = None;
        for lost in &mut old.lost {
            lost.frequency_hz = None;
        }
        let line = serde_json::to_string(&old).expect("a moment writes");
        assert!(
            !line.contains("frequency_hz") && !line.contains("modem_started"),
            "the old line still names a frequency or a start: {line}"
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
