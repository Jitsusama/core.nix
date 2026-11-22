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

/// How recent a line must be to have a say in when the modem last
/// started: the day keeps one a minute, so the two newest, with room for
/// a few minutes missed. An older line, kept before net or its machine
/// was down, may say a start the modem has since moved past.
const RECENT: u64 = 5 * 60;

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
/// after the modem last started. The walk back stops at a line read
/// before the modem last started, or where every channel that had
/// counted any reads lower after it, which is a start too though one the
/// modem may not count. A line is judged by when it was read, not by
/// when it said the modem started, so a line whose uptime was misread
/// costs nothing unless it is one of the few that say when the modem
/// last started. Net can't tell a step in its own clock from time
/// passing. One forward by S passes by the lines read before it in the
/// modem's first S of uptime, all of them if it came sooner, and
/// stretches the span it names by S when the stretch's first line was
/// read before it. One back by S hides each line read before it until S
/// after it was read, since it is stamped after now, and counts lines
/// read up to S before a start as after it, leaving only the counts to
/// show that start. In the first two minutes back from a gap, or from a
/// step back of five minutes or more, now alone says when the modem
/// started: a misread short uptime then cuts the stretch to the start it
/// says, or to nothing lately, and none leaves only the counts to show a
/// start. The page misreads a count now and then, often several channels
/// in one minute, so counts must read lower in both newer lines. The
/// newest line has only now after it, so its counts are judged against
/// now alone: in the first minute after a restart the older lines still
/// outvote now's new start, and only the counts show it, while a misread
/// now costs at most a minute with no count lately, in which a channel
/// far above the rest is a concern. A minute the modem didn't answer
/// says nothing of it, so the walk passes it by.
fn lately(modem: &mut Modem, started: Option<u64>, day: &[Moment], now: u64) {
    let lines: Vec<&Moment> = day
        .iter()
        .rev()
        .filter(|moment| moment.at < now && !moment.lost.is_empty())
        .collect();
    let latest = latest_start(started, &lines, now);
    let mut newer: Vec<Vec<Lost>> = vec![Moment::lost_of(modem)];
    let mut since: Vec<&Moment> = Vec::new();
    for &moment in &lines {
        let started_since = latest.is_some_and(|latest| latest > moment.at);
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
/// newest lines say, so with three to say one misread uptime doesn't
/// move it; with fewer, what now says, or else the newest line, so a
/// misread then does. Only lines from the last five minutes have a say,
/// so in the first minutes after net was down across a restart, now's
/// new start isn't outvoted by two from before it.
fn latest_start(started: Option<u64>, lines: &[&Moment], now: u64) -> Option<u64> {
    let mut starts: Vec<u64> = started
        .into_iter()
        .chain(
            lines
                .iter()
                .take_while(|moment| now.saturating_sub(moment.at) <= RECENT)
                .filter_map(|moment| moment.modem_started),
        )
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

    /// The golden record with the modem up `up` seconds, so it says it
    /// started that long before now.
    fn up_for(up: u64) -> Network {
        let mut now = network();
        device_mut(&mut now, "modem").up_s = Some(up);
        now
    }

    fn device_mut<'a>(network: &'a mut Network, id: &str) -> &'a mut crate::model::Device {
        network
            .devices
            .iter_mut()
            .find(|device| device.id == id)
            .unwrap_or_else(|| panic!("the record has the {id}"))
    }

    #[test]
    fn lately_counts_only_since_the_modem_last_started() {
        // The modem started three hours ago. The two lines before it say
        // the start before that, and channel 16 read lower in them than
        // since, so only when each line was read shows the start: it
        // counts from 860 two hours ago, not the older lines.
        let now = up_for(3 * HOUR);
        let day = [
            earlier(&network(), 6 * HOUR, 5, 41),
            earlier(&network(), 5 * HOUR, 10, 41),
            earlier(&now, 2 * HOUR, 860, 41),
        ];
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
    fn a_line_read_before_the_modem_started_is_before_the_start() {
        // The modem started five hours ago: a line read then is after it,
        // and one read a second sooner is before it.
        let now = up_for(5 * HOUR);
        for (ago, lately_s) in [(5 * HOUR, Some(5 * HOUR)), (5 * HOUR + 1, None)] {
            let day = [earlier(&network(), ago, 800, 41)];
            let network = recalled(now.clone(), &day);
            assert_eq!(
                modem(&network).lately_s,
                lately_s,
                "a line read {ago} seconds ago was judged wrongly"
            );
        }
    }

    #[test]
    fn a_step_in_nets_clock_is_no_start() {
        // Net's clock stepped an hour forward three minutes ago. The lines
        // before it say the modem started an hour before the lines since
        // do, but the modem was up all along, so lately reaches back to
        // the day's first line. Read five hours ago, it's stamped five
        // hours and the step ago, and that is the span lately names: net
        // can't tell the step from time passing.
        const STEP: u64 = HOUR;
        let now = network();
        let before = |ago, sixteen| {
            let mut line = Moment::of(&read_earlier(&now, ago + STEP, STEP));
            line.lost
                .iter_mut()
                .filter(|lost| lost.channel == Some(16))
                .for_each(|lost| lost.uncorrectable = sixteen);
            line
        };
        let day = [
            before(5 * HOUR, 800),
            before(3 * HOUR, 840),
            before(4 * 60, 870),
            before(3 * 60, 870),
            earlier(&now, 2 * 60, 875, 41),
            earlier(&now, 60, 875, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR + STEP),
            "a step in net's clock was taken as the modem starting"
        );
        assert_eq!(
            sixteen(&network).lost_lately,
            Some(78),
            "channel 16 counted from the wrong reading"
        );
    }

    #[test]
    fn a_start_while_net_was_down_is_seen_in_its_first_minute_back() {
        // Net was down three hours, and the modem started two hours ago.
        // The lines from before are too old to outvote now's new start,
        // though channel 16 has since climbed past them; nothing is
        // counted lately rather than less than was lost.
        let now = up_for(2 * HOUR);
        let day = [
            earlier(&network(), 5 * HOUR, 800, 41),
            earlier(&network(), 3 * HOUR + 60, 820, 41),
            earlier(&network(), 3 * HOUR, 820, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            (modem(&network).lately_s, sixteen(&network).lost_lately),
            (None, None),
            "the walk back crossed a start made while net was down"
        );
    }

    #[test]
    fn a_restart_in_the_last_minute_is_seen_by_its_counts() {
        // The modem says it started thirty seconds ago. The day's lines
        // still outvote now's new start, so only every count falling to 0
        // against now alone shows it: with the same uptime and counts that
        // kept climbing, the day reads on.
        for (restarted, lately_s) in [(true, None), (false, Some(5 * HOUR))] {
            let mut now = network();
            let day = [
                earlier(&now, 5 * HOUR, 800, 41),
                earlier(&now, 3 * HOUR, 840, 41),
                earlier(&now, 2 * 60, 870, 41),
                earlier(&now, 60, 875, 41),
            ];
            let modem_now = now
                .devices
                .iter_mut()
                .find(|device| device.id == "modem")
                .expect("the record has the modem");
            modem_now.up_s = Some(30);
            if restarted {
                for channel in &mut modem_now
                    .modem
                    .as_mut()
                    .expect("the modem has its channels")
                    .downstream
                {
                    channel.uncorrectable = Some(0);
                }
            }
            let network = recalled(now, &day);
            assert_eq!(
                modem(&network).lately_s,
                lately_s,
                "lately read wrongly {}",
                if restarted {
                    "after every count fell to 0"
                } else {
                    "with counts still climbing"
                }
            );
        }
    }

    #[test]
    fn a_misread_now_costs_one_minute_with_no_count_lately() {
        // Channel 16 lost nothing all day. Now reads each channel that had
        // counted any one lower, so the newest line falls against now alone,
        // nothing is counted lately, and channel 16's old count is a
        // concern. A minute on, the misread is a line that now outvotes,
        // and the day reads on, calm again.
        let now = network();
        let day = |now: &Network| {
            [
                earlier(now, 5 * HOUR, 878, 41),
                earlier(now, 3 * HOUR, 878, 41),
            ]
        };
        let mut misread_now = now.clone();
        for channel in &mut misread_now
            .devices
            .iter_mut()
            .find(|device| device.id == "modem")
            .and_then(|device| device.modem.as_mut())
            .expect("the record has the modem's channels")
            .downstream
        {
            channel.uncorrectable = channel.uncorrectable.map(|count| count.saturating_sub(1));
        }
        let mut misread = Moment::of(&misread_now);
        misread.at = now.collected_at - 60;
        let [five, three] = day(&now);
        let misread_minute = recalled(misread_now, &day(&now));
        let next_minute = recalled(now, &[five, three, misread]);
        assert_eq!(
            (
                modem(&misread_minute).lately_s,
                modem(&next_minute).lately_s
            ),
            (None, Some(5 * HOUR)),
            "a misread now cost other than its own minute's count lately"
        );
        assert!(
            says(&misread_minute, ConcernKind::Lost).contains("if that keeps climbing"),
            "the misread minute raised no concern over channel 16's old count"
        );
        assert!(
            next_minute
                .concerns
                .iter()
                .all(|concern| concern.kind != ConcernKind::Lost),
            "the concern outlived the misread minute: {:?}",
            next_minute.concerns
        );
    }

    #[test]
    fn a_newest_line_whose_uptime_read_short_is_outvoted() {
        // The newest line's uptime read short, so it says the modem
        // started after the line three hours ago was read; now and the
        // line before it say it didn't, and outvote it.
        let now = network();
        let mut misread = earlier(&now, 60, 875, 41);
        misread.modem_started = Some(now.collected_at - 3 * HOUR + 60);
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            earlier(&now, 3 * HOUR, 840, 41),
            earlier(&now, 2 * 60, 870, 41),
            misread,
        ];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "one newest line's short uptime was taken as the modem starting"
        );
    }

    #[test]
    fn a_newest_line_whose_uptime_read_long_is_outvoted_across_a_restart() {
        // The modem started three hours ago, and the lines before it read
        // channel 16 lower than since, so only the start ends the walk.
        // The newest line's uptime read long, saying a start before all
        // the day's lines; now and the line before it outvote it, so the
        // walk still stops at the start rather than crossing it.
        let now = up_for(3 * HOUR);
        let mut misread = earlier(&now, 60, 875, 41);
        misread.modem_started = Some(now.collected_at - 10 * HOUR);
        let day = [
            earlier(&network(), 6 * HOUR, 5, 41),
            earlier(&network(), 5 * HOUR, 10, 41),
            earlier(&now, 2 * HOUR, 860, 41),
            earlier(&now, 2 * 60, 870, 41),
            misread,
        ];
        let network = recalled(now, &day);
        assert_eq!(
            (modem(&network).lately_s, sixteen(&network).lost_lately),
            (Some(2 * HOUR), Some(18)),
            "one newest line's long uptime carried the walk across the start"
        );
    }

    #[test]
    fn with_one_line_recent_now_says_when_the_modem_started() {
        // Net was down a while, so only the line a minute ago is recent,
        // and its uptime read short. The line three hours ago misread the
        // same way, so were it given a say the two would outvote now; with
        // two to say, now's start stands.
        let now = network();
        let misread = |ago, sixteen| {
            let mut line = earlier(&now, ago, sixteen, 41);
            line.modem_started = Some(now.collected_at - 3 * HOUR + 60);
            line
        };
        let day = [
            earlier(&now, 5 * HOUR, 800, 41),
            misread(3 * HOUR, 840),
            misread(60, 875),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            modem(&network).lately_s,
            Some(5 * HOUR),
            "the one recent line's start was taken over now's"
        );
    }

    #[test]
    fn a_line_has_a_say_in_the_start_for_five_minutes() {
        // Now's uptime reads four hours. A line read five minutes ago
        // still votes with the newest against it; one read a second
        // longer ago doesn't, so now's misread start stands with only the
        // newest line beside it, and the walk stops at that misread start.
        for (ago, lately_s) in [(5 * 60, Some(5 * HOUR)), (5 * 60 + 1, Some(3 * HOUR))] {
            let now = network();
            let day = [
                earlier(&now, 5 * HOUR, 800, 41),
                earlier(&now, 3 * HOUR, 840, 41),
                earlier(&now, ago, 870, 41),
                earlier(&now, 60, 875, 41),
            ];
            let mut now = now;
            device_mut(&mut now, "modem").up_s = Some(4 * HOUR);
            let network = recalled(now, &day);
            assert_eq!(
                modem(&network).lately_s,
                lately_s,
                "a line read {ago} seconds ago had the wrong say in the start"
            );
        }
    }

    #[test]
    fn a_restart_is_seen_by_its_start_in_the_second_minute_after_it() {
        // The modem started ninety seconds ago. Now and the line a minute
        // ago say so, and the line before says the start before; channel
        // 16 kept climbing, so only the start, outvoting the old one,
        // stops the walk before the restart. Had the newest line said the
        // old start too, now's would be outvoted and the day read on.
        for (newest_saw_it, lately) in [(true, (None, None)), (false, (Some(5 * HOUR), Some(78)))] {
            let now = up_for(90);
            let before_it = network();
            let newest = if newest_saw_it { &now } else { &before_it };
            let day = [
                earlier(&network(), 5 * HOUR, 800, 41),
                earlier(&network(), 3 * HOUR, 840, 41),
                earlier(&network(), 2 * 60, 870, 41),
                earlier(newest, 60, 875, 41),
            ];
            let network = recalled(now, &day);
            assert_eq!(
                (modem(&network).lately_s, sixteen(&network).lost_lately),
                lately,
                "the start was judged wrongly when the newest line {} the restart",
                if newest_saw_it {
                    "had seen"
                } else {
                    "hadn't seen"
                }
            );
        }
    }

    #[test]
    fn in_the_first_two_minutes_back_from_a_gap_now_alone_says_when_the_modem_started() {
        // Net was down a while. With no line recent, or one, now's start
        // stands alone: an uptime read four hours cuts the stretch to the
        // line three hours ago, and none leaves the counts, which kept
        // climbing, to read on. With now's unsaid, the one recent line's
        // misread start stands instead and cuts the stretch to that line.
        let gap = || {
            vec![
                earlier(&network(), 5 * HOUR, 800, 41),
                earlier(&network(), 3 * HOUR, 840, 41),
            ]
        };
        let mut misread = earlier(&network(), 60, 875, 41);
        misread.modem_started = Some(network().collected_at - 3 * HOUR + 60);
        let cases = [
            (
                "a short uptime, first minute",
                Some(4 * HOUR),
                None,
                Some(3 * HOUR),
            ),
            (
                "a short uptime, second minute",
                Some(4 * HOUR),
                Some(earlier(&network(), 60, 875, 41)),
                Some(3 * HOUR),
            ),
            ("no uptime, first minute", None, None, Some(5 * HOUR)),
            ("no uptime, second minute", None, Some(misread), None),
        ];
        for (what, up_s, recent, lately_s) in cases {
            let mut now = network();
            device_mut(&mut now, "modem").up_s = up_s;
            let mut day = gap();
            day.extend(recent);
            let network = recalled(now, &day);
            assert_eq!(
                modem(&network).lately_s,
                lately_s,
                "back from a gap with {what}, the start was judged wrongly"
            );
        }
    }

    #[test]
    fn a_step_back_in_nets_clock_hides_the_lines_before_it_and_shortens_the_stretch() {
        // Net's clock stepped an hour back three minutes ago. A line read
        // before it is stamped an hour later than it was read, and says
        // the modem started an hour later too: the one four minutes ago
        // is stamped after now and hidden, and the stretch from the line
        // five hours ago reads an hour short. The two lines since vote
        // with now, so the start stands.
        const STEP: u64 = HOUR;
        let now = network();
        let before = |ago, sixteen| {
            let mut line = earlier(&now, ago, sixteen, 41);
            line.at += STEP;
            line.modem_started = line.modem_started.map(|started| started + STEP);
            line
        };
        let day = [
            before(5 * HOUR, 800),
            before(3 * HOUR, 840),
            before(4 * 60, 870),
            earlier(&now, 2 * 60, 875, 41),
            earlier(&now, 60, 875, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            (modem(&network).lately_s, sixteen(&network).lost_lately),
            (Some(5 * HOUR - STEP), Some(78)),
            "a step back in net's clock cost other than the step"
        );
    }

    #[test]
    fn a_step_in_nets_clock_cuts_the_stretch_after_a_start_by_the_step() {
        // The modem started three real hours ago, and net's clock stepped
        // an hour forward an hour ago. A line read a minute before the
        // modem's first hour was up is stamped before now's start, by the
        // new clock, and passed by; lately counts from the line read a
        // minute after it, stamped the step earlier. That is the step's cost, since net
        // can't tell it from time passing; reading time from the modem's
        // own uptime would change both figures on purpose.
        const STEP: u64 = HOUR;
        let now = up_for(3 * HOUR);
        // A line from before the step is stamped by the old clock, and
        // says the modem started three real hours ago by it.
        let before = |ago, sixteen| {
            let mut line = earlier(&now, ago + STEP, sixteen, 41);
            line.modem_started = Some(now.collected_at - 3 * HOUR - STEP);
            line
        };
        let day = [
            before(2 * HOUR + 60, 840),
            before(2 * HOUR - 60, 850),
            earlier(&now, 2 * 60, 870, 41),
            earlier(&now, 60, 875, 41),
        ];
        let network = recalled(now, &day);
        assert_eq!(
            (modem(&network).lately_s, sixteen(&network).lost_lately),
            (Some(2 * HOUR - 60 + STEP), Some(28)),
            "a forward step cut the stretch other than by the step"
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
        for (after, ofdm_lately) in [(180, Some(0)), (181, None)] {
            let mut old = earlier(&now, 5 * HOUR, 800, 41);
            old.modem_started = None;
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
