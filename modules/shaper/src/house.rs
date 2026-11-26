//! What another machine on the same line sends up, as it reports it, so
//! the upload the two share is held under the house's ceiling. The other
//! machine's packets go out its own Wi-Fi, which this one never sees, and
//! the router's queue they fill is the one the VPN waits in: on 21
//! November the VPN ran late in 6 to 14% of seconds while the two sent
//! under 5 Mb/s between them, and in 87% at 11 to 12, whichever sent it.
//!
//! The other machine reports from the address of whichever interface it
//! sends by, so a house names one for each, or, keyed, none (below), and of the fresh reports the
//! one saying the most says what it sends (src/main.rs's `house_holds`),
//! so a report from an address it isn't sending by can't loosen the hold.
//!
//! A house with no key takes a report by its source address alone, so any
//! device on the line can forge one by sending from that address: it
//! holds this machine's upload down to the least a trial may set, or lets
//! it go, for as long as it keeps sending. Unlike a trial it needs no
//! administrator. A private IPv4 or unique local IPv6 address is the one
//! to name; a link-local one is shared by every link the machine has, a
//! virtual machine's bridge among them, and the firewall's rule names no
//! interface.
//!
//! A house naming a key takes only reports signed with it, so no forger
//! can say what the house sends: `"<bytes> <ms> <unix_ms> <mac>"`, `unix_ms` the reporter's wall
//! clock and `mac` the first 16 bytes of HMAC-SHA256 over the text before
//! it, in hex. One stamped more than `FRESH` from this machine's clock,
//! either way, or no later than the last taken from its address, is
//! refused, so one caught off the line replays for at most `FRESH`, and
//! from its own address only in place of a newer one. The mac doesn't
//! cover the address, so one sent again from another address the house
//! names is taken there once, but the house reads the most its fresh
//! addresses say, and copies of true reports say about what the true ones
//! do: a replay there keeps an old word alive and can only tighten the
//! hold. An unsigned report is refused there too,
//! so a forger can't fall back to the old shape. A house with no key
//! takes a signed report's first two words, so a reporter given the key
//! before this machine is heard as before. Signing guards what a report
//! says, not that one comes: anything that keeps the true reports from
//! arriving, on a line where addresses aren't proven, lets the hold go
//! once they're `FRESH` gone, as it was before a house was named (house
//! signing round 2's F20).
//!
//! A keyed house naming no address takes a signed report from any
//! address, and the key that signed it says whose it is: its rate and its
//! last stamp are the house's wherever a lease or a cable moves the
//! machine, so a report replayed from another address meets the same
//! stamp. Every house holding the key that signed it hears it, the key
//! saying no more than that, so one machine reporting under one key is
//! the house on each network naming it; one any such house's stamp
//! refuses is taken by none. A report from an address another house
//! names is tried under these keys too: when that house refuses or
//! ignores it, so a lease of that address doesn't hide this house, and
//! when that house's key signed it, so both stamp it (house any round
//! 2's F7, F12 and F15). The firewall opens the port to every private IPv4 address for
//! it, house-report sending over IPv4 alone (house any round 1's F2, F3
//! and F10).
//!
//! A report is one datagram a second to `PORT`: the bytes the machine's
//! uplink has sent since it started, and its own clock in milliseconds,
//! as text, `"<bytes> <ms>"`. Bytes over milliseconds, times eight, is
//! kbit/s, measured on the machine's clock rather than by when the
//! datagrams arrive. Signed, `"<bytes> <ms> <unix_ms> <mac>"`, as above.
//!
//! Long because its tests, over half of it, walk each way a report is
//! taken or refused: forged, short, unsigned, stale, repeated and keyless,
//! and the ear's sockets, which all read the one format, side by side.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Where reports are heard, clear of the ports the probes use.
pub const PORT: u16 = 47_110;

/// How long a report's rate stands: past this the machine may have
/// stopped sending, or gone, and holding this one under it for nothing
/// would cost its agents' uploads.
pub const FRESH: Duration = Duration::from_secs(3);

/// The most two reports may lie apart on the reporter's clock and still
/// give a rate: past it, one was lost and the bytes between are spread
/// too thin to say a second's sending.
const APART_MS: u64 = 3_000;

/// How far ahead of the time since the last report a report may be on the
/// reporter's clock and still be taken, in ms. Reports come a second
/// apart, so two the line held back and let through together lie a
/// second apart on the clock and a moment apart here; a report lost
/// between widens the time since and the clock alike.
const AHEAD_MS: u64 = 1_500;

/// The most a rate is read as, in kbit/s: the most a house's ceiling may
/// name (nixos.nix). Reports are numbers off the wire, so one claiming
/// more is read as this, which holds this machine at its least all the
/// same, and the journal and the trace never say a count past it.
const SENT_MOST_KBIT: f64 = 10_000_000.0;

/// The most bytes two reports may lie apart by and be read as such: what
/// `SENT_MOST_KBIT` sends in `APART_MS`. The rate is bounded after all the
/// same; this keeps what `rate::float` is handed far below 2^53, as its
/// reason says every count is.
const BYTES_MOST: u64 = 3_750_000_000;

/// How long the controller is given the highest of the house's ceilings.
/// What the other machine sends moves each second with its own TCP's
/// sawtooth, and a ceiling that moved with it would clamp the controller
/// to each second's low and let it climb back only by its 5% a second, so
/// it would hold this machine under the lowest of the last several, not
/// what the house leaves. The link is held to each second's all the same.
pub const STEADY: Duration = Duration::from_secs(5);

/// The step the controller's ceiling moves by, in kbit/s, so it is tuned
/// again only when the house's moves by this much.
const STEP_KBIT: u64 = 250;

/// The fewest bytes a house's key may hold: a shorter one can be guessed.
/// The key is made as 32 random bytes; house-report refuses the same.
const KEY_LEAST: usize = 16;

/// How long a key read stands before the ear reads its file again, so a
/// flood of well-shaped forgeries costs an HMAC each, not a read of the
/// file on the thread the true reports wait behind (house signing round
/// 2's F19), while a key put in place or swapped is taken within it.
const KEY_READ_APART: Duration = Duration::from_secs(1);

/// The bytes of the mac a signed report carries, half of SHA-256's. One
/// carrying fewer is refused, since a mac of a byte or two is guessed.
const MAC_BYTES: usize = 16;

/// One report: the bytes sent, by the reporter's clock in ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    pub bytes: u64,
    pub ms: u64,
}

/// Reads a report's text, or nothing where it isn't one.
pub fn report(text: &str) -> Option<Report> {
    let mut words = text.split_whitespace();
    let bytes = words.next()?.parse().ok()?;
    let ms = words.next()?.parse().ok()?;
    words.next().is_none().then_some(Report { bytes, ms })
}

/// Reads a report where its house names no key: the old shape, or a
/// signed one's first two words, its mac unread.
fn unkeyed(text: &str) -> Option<Report> {
    report(text).or_else(
        || match text.split_whitespace().collect::<Vec<_>>().as_slice() {
            [bytes, ms, _, _] => report(&format!("{bytes} {ms}")),
            _ => None,
        },
    )
}

/// Why a report under a house's key is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Refused {
    /// It isn't signed, as a forger falling back to the old shape sends.
    Unsigned,
    /// Its mac isn't the key's: forged, or the machines' keys differ.
    Forged,
    /// Stamped more than `FRESH` from this machine's clock.
    Stale,
    /// Stamped no later than the last taken from its address, or from its
    /// house where that names none.
    Repeated,
}

/// The bytes `text` spells in hex, two digits a byte, or nothing where
/// it spells none.
fn hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

/// Reads `text` as a report signed under `key`, at `now_ms` on this
/// machine's wall clock, when the last report taken from its address was
/// stamped `last`: the report, and its stamp to take as the next `last`.
/// The mac is checked in constant time, and before the stamp, so nothing
/// is said of a stamp a forger chose. The ear reads the parts before the
/// key (`Ear::heard`), and the tests read the two together.
#[cfg(test)]
fn signed(
    text: &str,
    key: &[u8],
    now_ms: u64,
    last: Option<u64>,
) -> Result<(Report, u64), Refused> {
    verified(&Signed::of(text)?, key, now_ms, last)
}

/// A signed report's parts, read before its key is: the text its mac is
/// over, the report, its stamp and its mac.
struct Signed<'a> {
    stamped: &'a str,
    report: Report,
    stamp: u64,
    mac: Vec<u8>,
}

impl Signed<'_> {
    /// The parts of `text`, or why it's refused: a text of another shape
    /// costs no read of the key nor an HMAC, so a flood of them takes
    /// the ear's one thread no longer than unsigned reports did.
    fn of(text: &str) -> Result<Signed<'_>, Refused> {
        let (stamped, mac) = text.rsplit_once(' ').ok_or(Refused::Unsigned)?;
        let (plain, stamp) = stamped.rsplit_once(' ').ok_or(Refused::Unsigned)?;
        let report = report(plain).ok_or(Refused::Unsigned)?;
        let stamp: u64 = stamp.parse().map_err(|_| Refused::Unsigned)?;
        let mac = hex(mac)
            .filter(|mac| mac.len() == MAC_BYTES)
            .ok_or(Refused::Forged)?;
        Ok(Signed {
            stamped,
            report,
            stamp,
            mac,
        })
    }
}

/// `signed`'s report and stamp once its parts are read.
fn verified(
    signed: &Signed<'_>,
    key: &[u8],
    now_ms: u64,
    last: Option<u64>,
) -> Result<(Report, u64), Refused> {
    let &Signed {
        stamped,
        report,
        stamp,
        ref mac,
    } = signed;
    let mut hash = <Hmac<Sha256>>::new_from_slice(key).expect("any key fits an HMAC");
    hash.update(stamped.as_bytes());
    hash.verify_truncated_left(mac)
        .map_err(|_| Refused::Forged)?;
    let fresh = u64::try_from(FRESH.as_millis()).unwrap_or(u64::MAX);
    if now_ms.abs_diff(stamp) > fresh {
        return Err(Refused::Stale);
    }
    if last.is_some_and(|last| stamp <= last) {
        return Err(Refused::Repeated);
    }
    Ok((report, stamp))
}

/// The key in `path`, or why it can't be had.
fn key_in(path: &Path) -> Result<Vec<u8>, String> {
    let key = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if key.len() < KEY_LEAST {
        return Err(format!(
            "{} holds {} bytes, under the {KEY_LEAST} a key needs",
            path.display(),
            key.len()
        ));
    }
    Ok(key)
}

/// Who reports are heard from.
#[derive(Debug, Default, PartialEq)]
pub struct Reporters {
    /// The addresses houses name, each with the file holding the key its
    /// reports must be signed with, where its house names one.
    pub named: BTreeMap<IpAddr, Option<PathBuf>>,
    /// Each keyed house that names no address, by its network, with its
    /// key file: a report from an address no house names, or one its
    /// house refuses or ignores, is taken for every one of these whose
    /// key signed it and whose stamp takes it, so its key alone says who
    /// sent it, wherever its address moves.
    pub unnamed: Vec<(String, PathBuf)>,
}

impl<const N: usize> From<[(IpAddr, Option<PathBuf>); N]> for Reporters {
    fn from(named: [(IpAddr, Option<PathBuf>); N]) -> Self {
        Reporters {
            named: BTreeMap::from(named),
            unnamed: Vec::new(),
        }
    }
}

/// Every house's addresses and keys. An address two houses name is held
/// to a key either names, so naming a key is never undone by another
/// house leaving it out. A house naming no address is heard by its key
/// alone (networks.rs keeps one only where it names a key).
pub fn reporters(houses: &crate::networks::Houses) -> Reporters {
    let mut reporters = Reporters::default();
    for (network, house) in houses {
        if house.from.is_empty()
            && let Some(key) = &house.key
        {
            reporters.unnamed.push((network.clone(), key.clone()));
        }
        for &address in &house.from {
            let key = reporters.named.entry(address).or_insert(None);
            if key.is_none() {
                key.clone_from(&house.key);
            }
        }
    }
    reporters
}

/// What a report is heard as: the address a house names that it came
/// from, or, for a house naming none, that house, by its network, so its
/// rate and its last stamp stand wherever its address moves, and a
/// report replayed from another address meets the same stamp.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Address(IpAddr),
    House(String),
}

impl From<IpAddr> for Source {
    fn from(address: IpAddr) -> Self {
        Source::Address(address)
    }
}

/// Why a datagram held nothing.
#[derive(Debug, PartialEq)]
enum Unheard {
    /// No report any house takes from its address: nothing to say, as a
    /// stray datagram is said nothing of.
    Ignored,
    /// Its house's key can't be read.
    Unkeyed(String),
    /// Refused under its house's key.
    Refused(Refused),
}

/// Why a datagram held nothing, and whose refusal that is, for the
/// journal: an address a house names, a house naming none whose key
/// signed it, or `None`, no house's.
type Refusal = (Option<Source>, Unheard);

/// What the ear made of a datagram, as the tests read it: the first
/// house its report was taken for, or the refusal that says most of why
/// (`Verdicts::first`).
#[cfg(test)]
type Verdict = Result<Source, Refusal>;

/// What the ear made of a datagram for every house it could be: each its
/// report was taken for, the first being whose it is, with the report,
/// and each house's refusal of it, so one house's refusal isn't left
/// unsaid while another takes the report (house any round 4's F2, F7
/// and F9). A stamp refused under a key that signed the report refuses
/// it for every house, leaving none taken, so a replay meets every stamp.
#[derive(Debug, Default)]
struct Verdicts {
    taken: Vec<Source>,
    report: Option<Report>,
    refused: Vec<Refusal>,
}

impl Verdicts {
    /// `report`, taken for `source`.
    fn heard(source: Source, report: Report) -> Self {
        Verdicts {
            taken: vec![source],
            report: Some(report),
            refused: Vec::new(),
        }
    }

    /// A datagram that held nothing, for `refusal`.
    fn refused(refusal: Refusal) -> Self {
        Verdicts {
            refused: vec![refusal],
            ..Verdicts::default()
        }
    }

    /// A report taken for `source`, whatever it said.
    #[cfg(test)]
    fn taken(source: Source) -> Self {
        Verdicts {
            taken: vec![source],
            ..Verdicts::default()
        }
    }

    /// These and `other`'s together, a stamp either refuses refusing the
    /// report for both.
    #[must_use]
    fn and(mut self, other: Self) -> Self {
        self.taken.extend(other.taken);
        self.report = self.report.or(other.report);
        self.refused.extend(other.refused);
        self.under_stamps()
    }

    /// Whether a key among these signed the report: one took it, or its
    /// stamp refused it.
    fn signed(&self) -> bool {
        !self.taken.is_empty() || self.refused.iter().any(|(_, why)| refuses_all(why))
    }

    /// These, taken for none where a stamp was refused under a key that
    /// signed the report.
    #[must_use]
    fn under_stamps(mut self) -> Self {
        if self.refused.iter().any(|(_, why)| refuses_all(why)) {
            self.taken.clear();
        }
        self
    }

    /// The first house the report was taken for, or else the refusal that
    /// says most of why, the first of those that say as much.
    #[cfg(test)]
    fn first(self) -> Verdict {
        if let Some(source) = self.taken.into_iter().next() {
            return Ok(source);
        }
        let mut most: Option<Refusal> = None;
        for refusal in self.refused {
            if most
                .as_ref()
                .is_none_or(|most| telling(&refusal.1) > telling(&most.1))
            {
                most = Some(refusal);
            }
        }
        Err(most.unwrap_or((None, Unheard::Ignored)))
    }
}

/// Whether `why` is a stamp refused under a key that signed the report,
/// which refuses it for every house.
fn refuses_all(why: &Unheard) -> bool {
    matches!(why, Unheard::Refused(Refused::Stale | Refused::Repeated))
}

/// How much `why` says of a report no house took: a key that can't be
/// read says more than a mac no key made. A stamp refused under a key
/// that signed it says most, and refuses the report for every house
/// (`refuses_all`).
#[cfg(test)]
fn telling(why: &Unheard) -> u8 {
    match why {
        Unheard::Ignored => 0,
        Unheard::Refused(Refused::Unsigned | Refused::Forged) => 1,
        Unheard::Unkeyed(_) => 2,
        Unheard::Refused(Refused::Stale | Refused::Repeated) => 3,
    }
}

/// The house `whose` is, for the journal: by its network where it names
/// no address, as another house may take a report from the same one.
fn named(whose: Option<&Source>) -> String {
    match whose {
        Some(Source::House(network)) => format!("the house on {network}"),
        _ => "the house".to_owned(),
    }
}

/// What the journal says the first time reports from `address` go
/// unheard for `why`, naming the house, `whose` (`named`).
fn unheard_said(address: IpAddr, whose: Option<&Source>, why: &Unheard) -> Option<String> {
    let why = match why {
        Unheard::Ignored => return None,
        Unheard::Unkeyed(why) => format!("the house's key can't be read ({why})"),
        Unheard::Refused(Refused::Unsigned) => "one came unsigned under the house's key".into(),
        Unheard::Refused(Refused::Forged) => {
            "one's mac isn't the house's key's, so it's forged or the two machines' keys differ"
                .into()
        }
        Unheard::Refused(Refused::Stale) => {
            "one was stamped more than 3 s from this machine's clock, so the two clocks \
             disagree or it was sent again"
                .into()
        }
        Unheard::Refused(Refused::Repeated) => {
            "one was stamped no later than the last taken, so it was sent again".into()
        }
    };
    Some(format!(
        "a report from {address} held nothing for {}, and those like it go unsaid \
         for a minute: {why}",
        named(whose)
    ))
}

/// How long after an address's reports are said to hold nothing they go
/// unsaid, so a forger sending between the true reports writes the
/// journal a line or two a minute rather than one a second.
const SAID_APART: Duration = Duration::from_secs(60);

/// What the journal has said of the house's addresses: each whose first
/// report it said was heard, for each house that took it, and when it
/// last said reports held nothing, by whose they were (`Refusal`).
#[derive(Debug, Default)]
struct Said {
    first: BTreeSet<(IpAddr, Source)>,
    troubled: BTreeMap<Option<Source>, Instant>,
}

impl Said {
    /// The journal's lines for a datagram from `address`, heard `now`, as
    /// the ear made of it for every house (`Ear::verdicts`): its first
    /// report heard for each house that took it, named as its refusals
    /// are, so a reporter given another house's key is seen heard as that
    /// house's, and a second house's first report from the address is
    /// still said (every verdict round 2's F7). A report under a key two
    /// houses share says its first for each house that takes it, wherever
    /// the machine is (names its taker round 2's F14; house ends round 1's
    /// F3, F7 and F15). Each house's refusal is said at most once each
    /// `SAID_APART` for whose it is, whatever kinds of refusal come, and,
    /// once that has passed, that address's or house's reports are said to
    /// be taken again, for each house that takes them. Refusals no
    /// house's key signed, from addresses no house names, are said
    /// together, so a forger sending from many addresses writes no more,
    /// and never hide a house's own; their end is never said, since no
    /// report is theirs (house any round 1's F4, F12, F17 and F22). They
    /// go unsaid where a house took the report or refused it as its own,
    /// which says more, so a keyless house's reporter isn't said to send
    /// unsigned to the keyed (house any round 4's F2 and F9).
    fn of(&mut self, address: IpAddr, heard: &Verdicts, now: Instant) -> Vec<String> {
        let lately = |troubled: &BTreeMap<Option<Source>, Instant>, kin: &Option<Source>| {
            troubled
                .get(kin)
                .is_some_and(|&said| now.saturating_duration_since(said) < SAID_APART)
        };
        let mut lines = Vec::new();
        for source in &heard.taken {
            let house = named(Some(source));
            if self.first.insert((address, source.clone())) {
                lines.push(format!(
                    "the first report from {address} for {house} was heard"
                ));
            }
            // Its end is said at most once, since `remove` finds the
            // refusal only the first time, whatever `taken` holds (names
            // its taker round 1's F1, F8 and F12; round 2's F3, F5, F7
            // and F12).
            let kin = Some(source.clone());
            if !lately(&self.troubled, &kin) && self.troubled.remove(&kin).is_some() {
                lines.push(format!(
                    "the reports from {address} for {house} are taken again"
                ));
            }
        }
        let owned = !heard.taken.is_empty()
            || heard
                .refused
                .iter()
                .any(|(kin, why)| kin.is_some() && *why != Unheard::Ignored);
        for (kin, why) in &heard.refused {
            if kin.is_none() && owned {
                continue;
            }
            if let Some(said) = unheard_said(address, kin.as_ref(), why)
                && !lately(&self.troubled, kin)
            {
                lines.push(said);
                self.troubled.insert(kin.clone(), now);
            }
        }
        lines
    }
}

/// This machine's wall clock in milliseconds since 1970, or 0 on a clock
/// set before it, which refuses every signed report as stale.
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The rate between two reports in kbit/s, never past `SENT_MOST_KBIT`,
/// or nothing where they can't say one: the counter went back, as when
/// the machine restarts, the clock didn't move or went back, or they lie
/// too far apart.
pub fn rate(was: Report, now: Report) -> Option<f64> {
    let bytes = now.bytes.checked_sub(was.bytes)?.min(BYTES_MOST);
    let ms = now
        .ms
        .checked_sub(was.ms)
        .filter(|&ms| ms > 0 && ms <= APART_MS)?;
    Some((crate::rate::float(bytes) * 8.0 / crate::rate::float(ms)).min(SENT_MOST_KBIT))
}

/// The most this machine's upload may be held to while the house's
/// ceiling is `most` and the other machine last sent `sent` kbit/s,
/// `age` ago: what the ceiling leaves, never under `floor`. A rate that
/// isn't fresh, or none, holds nothing.
pub fn held(most: u64, sent: Option<(f64, Duration)>, floor: u64) -> Option<u64> {
    let (kbit, _) = sent.filter(|&(_, age)| age <= FRESH)?;
    Some(crate::rate::whole(crate::rate::float(most) - kbit).max(floor))
}

/// The house's ceilings heard over the last `STEADY`, by network, so the
/// controller is given a ceiling that holds still. The controller may sit
/// above the link's hold for as long, by as much as the house swings. A
/// plateau it lowers is read from the most the link carried over a span
/// as long (`PROVEN_S` in src/oneway.rs), which while the link is busy is
/// near the high this ceiling holds, and never under a declared line's
/// hold, so a second the house held isn't learned as the line.
#[derive(Default)]
pub struct Steady {
    seen: BTreeMap<String, VecDeque<(Instant, u64)>>,
}

impl Steady {
    /// The ceiling the controller is given on each network at `now`, in
    /// kbit/s, from this moment's `held`: the highest of the last
    /// `STEADY`, rounded down to `STEP_KBIT`. `held` is never under the
    /// least a trial may set, a multiple of `STEP_KBIT` (a test holds it
    /// to that), so neither is this. A network whose house holds nothing now is forgotten at once,
    /// so a house that lets go lets the controller go too.
    pub fn of(&mut self, held: &BTreeMap<String, u64>, now: Instant) -> BTreeMap<String, u64> {
        self.seen.retain(|network, _| held.contains_key(network));
        held.iter()
            .map(|(network, &most)| {
                let seen = self.seen.entry(network.clone()).or_default();
                seen.push_back((now, most));
                while seen
                    .front()
                    .is_some_and(|&(at, _)| now.saturating_duration_since(at) > STEADY)
                {
                    seen.pop_front();
                }
                let high = seen.iter().map(|&(_, most)| most).max().unwrap_or(most);
                (network.clone(), high / STEP_KBIT * STEP_KBIT)
            })
            .collect()
    }
}

/// What each reporter last said, by its `Source`: its report, and the
/// rate it gives with the one before and when that came.
#[derive(Clone, Default)]
pub struct Ear {
    heard: Arc<Mutex<BTreeMap<Source, Heard>>>,
    /// The stamp of the last signed report taken from each source.
    stamps: Arc<Mutex<BTreeMap<Source, u64>>>,
    /// Each key file as last read, and when, or why it couldn't be.
    keys: Arc<Mutex<BTreeMap<PathBuf, KeyRead>>>,
}

/// When a key file was read, and the key it held or why it couldn't be.
type KeyRead = (Instant, Result<Vec<u8>, String>);

#[derive(Clone, Copy)]
struct Heard {
    report: Report,
    /// When `report` was heard.
    at: Instant,
    rate: Option<(f64, Instant)>,
}

impl Ear {
    /// Hears reports on `PORT` from `reporters` alone, each under its
    /// house's key where it names one, on a thread of its own, which
    /// tries for the port each second until it has it.
    pub fn start(reporters: Reporters) -> Self {
        let ear = Ear::default();
        let heard = ear.clone();
        thread::spawn(move || {
            let from: Vec<IpAddr> = reporters.named.keys().copied().collect();
            let socket = bound(&from, !reporters.unnamed.is_empty());
            heard.listen(&socket, &reporters);
        });
        ear
    }

    /// Hears reports for as long as it runs. Each address's first report
    /// taken for each house is said in the journal, and so is a report its
    /// key refuses, at most once each `SAID_APART`; one taken once that has
    /// passed says its reports are taken again.
    fn listen(&self, socket: &UdpSocket, reporters: &Reporters) {
        let mut buf = [0_u8; 128];
        let mut failed = false;
        let mut said = Said::default();
        loop {
            let (n, peer) = match socket.recv_from(&mut buf) {
                Ok(heard) => heard,
                Err(e) => {
                    if !failed {
                        eprintln!("{e}: the house's reports go unheard until a read works");
                    }
                    failed = true;
                    // A read that keeps failing would otherwise spin a
                    // core; a report a second needs no quicker retry.
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            if failed {
                eprintln!("the house's reports are read again");
            }
            failed = false;
            let address = canonical(peer.ip());
            let now = Instant::now();
            let heard = self.verdicts(reporters, address, &buf[..n], now, unix_ms());
            for line in said.of(address, &heard, now) {
                eprintln!("{line}");
            }
        }
    }

    /// `verdicts`, as the first house a report was taken for, or the
    /// refusal that says most of why.
    #[cfg(test)]
    fn heard(
        &self,
        reporters: &Reporters,
        peer: IpAddr,
        datagram: &[u8],
        at: Instant,
        now_ms: u64,
    ) -> Verdict {
        self.verdicts(reporters, peer, datagram, at, now_ms).first()
    }

    /// Takes a datagram from `peer`, heard `at` and at `now_ms` on this
    /// machine's wall clock, when it comes from an address `reporters`
    /// names and reads as a report under its house's key, if any, or is
    /// signed under the key of a house naming none, giving each house it
    /// was taken for and each house's refusal. The key is read again once
    /// `KEY_READ_APART` has passed, so one put in place or swapped is taken
    /// without a restart, and a report not shaped as a signed one reads no
    /// key.
    fn verdicts(
        &self,
        reporters: &Reporters,
        peer: IpAddr,
        datagram: &[u8],
        at: Instant,
        now_ms: u64,
    ) -> Verdicts {
        let address = canonical(peer);
        let source = Source::Address(address);
        let Ok(text) = std::str::from_utf8(datagram) else {
            return Verdicts::refused((Some(source), Unheard::Ignored));
        };
        // A house naming no address may have been leased one another house
        // names, so a report its key signed is its own there too, unless
        // that house's key signed it (house any round 1's F2 and F20). A
        // keyless house takes it too, as it takes any report from its
        // address, so one machine reporting under a key to both is heard
        // by both (house any round 2's F15).
        let verdicts = match reporters.named.get(&address) {
            None => self.unnamed(&reporters.unnamed, text, at, now_ms),
            Some(None) => {
                let others = self.unnamed(&reporters.unnamed, text, at, now_ms);
                let signed = others.report.filter(|_| !others.taken.is_empty());
                let own = match signed.or_else(|| unkeyed(text)) {
                    Some(report) => Verdicts::heard(source, report),
                    None => Verdicts::refused((Some(source), Unheard::Ignored)),
                };
                others.and(own)
            }
            Some(Some(path)) => match self.signed(text, &source, path, at, now_ms) {
                // Refused by the address's own stamp before a house naming
                // none reads it, so theirs stand.
                Err(why) if refuses_all(&why) => Verdicts::refused((Some(source), why)),
                // A report an unnamed house on this key took elsewhere is
                // refused here too, so a replay meets both stamps (house any
                // round 3's F1, F11 and F13), and a refusal on either side
                // is said while the other takes the report (round 4's F2
                // and F7). A mac the address's key didn't make but an
                // unnamed house's did is that house's leased here, so it's
                // no house's forgery, said nothing of beside the house's
                // own verdict (every verdict round 1's F2, F5, F11 and F14).
                named => {
                    let others = self.unnamed(&reporters.unnamed, text, at, now_ms);
                    let own = match named {
                        Ok(report) => Verdicts::heard(source, report),
                        Err(why @ Unheard::Refused(Refused::Forged)) if others.signed() => {
                            Verdicts::refused((None, why))
                        }
                        Err(why) => Verdicts::refused((Some(source), why)),
                    };
                    own.and(others)
                }
            },
        };
        if let Some(report) = verdicts.report {
            for source in &verdicts.taken {
                self.hear(source.clone(), report, at);
            }
        }
        verdicts
    }

    /// `heard`, as the address a report was taken from, or why it held
    /// nothing.
    #[cfg(test)]
    fn take(
        &self,
        reporters: &Reporters,
        peer: IpAddr,
        datagram: &[u8],
        at: Instant,
        now_ms: u64,
    ) -> Result<IpAddr, Unheard> {
        self.heard(reporters, peer, datagram, at, now_ms)
            .map(|_| canonical(peer))
            .map_err(|(_, why)| why)
    }

    /// The report in `text`, signed under the key in `path` and stamped
    /// later than the last taken from `source`, which it then is.
    fn signed(
        &self,
        text: &str,
        source: &Source,
        path: &Path,
        at: Instant,
        now_ms: u64,
    ) -> Result<Report, Unheard> {
        let parts = Signed::of(text).map_err(Unheard::Refused)?;
        let key = self.key(path, at).map_err(Unheard::Unkeyed)?;
        let mut stamps = self.stamps.lock().unwrap_or_else(PoisonError::into_inner);
        let last = stamps.get(source).copied();
        let (report, stamp) = verified(&parts, &key, now_ms, last).map_err(Unheard::Refused)?;
        stamps.insert(source.clone(), stamp);
        Ok(report)
    }

    /// Each house of `unnamed` whose key signed the report in `text` and
    /// whose stamp took it, each against its own, with the report, so one
    /// machine reporting under one key is the house on every network
    /// naming it, and a replay meets every such house's stamp, taken by
    /// none when one refuses it (house any round 1's F1, F7 and F15; round
    /// 2's F12 and F15); and every house's refusal of it. A key that can't
    /// be read is its house's, so its report taken again is said (round
    /// 2's F5). A mac malformed or no key's is no house's (round 2's F1),
    /// even one another house's key made, since a copy of a key made
    /// differently can't be told from another house's own key (every
    /// verdict round 1's F1, F10 and F13). An unsigned report is
    /// refused as no house's too, for a reporter not yet given the key to
    /// be said once a minute (round 1's F21); any other text is a stray,
    /// said nothing of.
    fn unnamed(
        &self,
        unnamed: &[(String, PathBuf)],
        text: &str,
        at: Instant,
        now_ms: u64,
    ) -> Verdicts {
        if unnamed.is_empty() {
            return Verdicts::refused((None, Unheard::Ignored));
        }
        match Signed::of(text) {
            Ok(_) => {}
            Err(refused @ Refused::Forged) => {
                return Verdicts::refused((None, Unheard::Refused(refused)));
            }
            Err(_) if report(text).is_some() => {
                return Verdicts::refused((None, Unheard::Refused(Refused::Unsigned)));
            }
            Err(_) => return Verdicts::refused((None, Unheard::Ignored)),
        }
        let mut verdicts = Verdicts::default();
        for (network, path) in unnamed {
            let source = Source::House(network.clone());
            match self.signed(text, &source, path, at, now_ms) {
                Ok(report) => {
                    verdicts.report = Some(report);
                    verdicts.taken.push(source);
                }
                Err(why @ Unheard::Refused(Refused::Forged)) => verdicts.refused.push((None, why)),
                Err(why) => verdicts.refused.push((Some(source), why)),
            }
        }
        verdicts.under_stamps()
    }

    /// The key in `path` at `at`, as read within `KEY_READ_APART`, or read
    /// now.
    fn key(&self, path: &Path, at: Instant) -> Result<Vec<u8>, String> {
        let mut keys = self.keys.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((read, key)) = keys.get(path)
            && at.saturating_duration_since(*read) < KEY_READ_APART
        {
            return key.clone();
        }
        let key = key_in(path);
        keys.insert(path.to_path_buf(), (at, key.clone()));
        key
    }

    /// Takes a report from `address`, heard `at`. Within `FRESH` of the
    /// last one taken, a report is left out when it's no later on the
    /// reporter's clock, as a copy or one come out of order is, or further
    /// ahead than the time since plus `AHEAD_MS`, as a clock that jumped
    /// is: either would become what the next is read against, and the
    /// second would leave out every true one for `FRESH`. A forged report
    /// in step is still taken, and is what the next true one is read
    /// against, so it sets that second's rate: one with a low count puts
    /// this machine at its floor for a second, and one within `AHEAD_MS`
    /// ahead leaves out the true ones for as long. A forger sending its own
    /// reports could do either anyway.
    /// Past `FRESH` any report is read against from then, as one is after
    /// the reporter restarts. One that can't give a rate with the last,
    /// after a report was lost, is read against from then too, and the
    /// last rate stands until it is past `FRESH`, as it would through
    /// silence.
    pub fn hear(&self, source: impl Into<Source>, report: Report, at: Instant) {
        let source = source.into();
        let mut heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let was = heard.get(&source).copied();
        let since = |w: &Heard| at.saturating_duration_since(w.at);
        let in_step = |w: &Heard| {
            let since_ms = u64::try_from(since(w).as_millis()).unwrap_or(u64::MAX);
            report.ms > w.report.ms && report.ms - w.report.ms <= since_ms.saturating_add(AHEAD_MS)
        };
        if was.is_some_and(|w| since(&w) <= FRESH && !in_step(&w)) {
            return;
        }
        let rate = was
            .and_then(|w| rate(w.report, report))
            .map(|kbit| (kbit, at))
            .or(was.and_then(|w| w.rate));
        heard.insert(source, Heard { report, at, rate });
    }

    /// What `source` last sent in kbit/s, and how long ago it said so.
    pub fn sent(&self, source: impl Into<Source>, now: Instant) -> Option<(f64, Duration)> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let (kbit, at) = heard.get(&source.into())?.rate?;
        Some((kbit, now.saturating_duration_since(at)))
    }
}

/// The addresses to listen on for houses at `from`, in the order tried:
/// IPv4's alone where every one is IPv4, so a machine with IPv6 turned off
/// hears them, else IPv6's, which hears both, then IPv4's, so IPv4 houses
/// are still heard where IPv6's port can't be had.
fn wildcards(from: &[IpAddr]) -> Vec<IpAddr> {
    let v4 = Ipv4Addr::UNSPECIFIED.into();
    if from.iter().all(IpAddr::is_ipv4) {
        vec![v4]
    } else {
        vec![Ipv6Addr::UNSPECIFIED.into(), v4]
    }
}

/// A socket on `PORT` for houses at `from`, and at any address where
/// `anywhere`, as a house naming none is, tried for each second until
/// one is had, saying once that it isn't, and saying where reports are
/// heard once it is, so a house configured but never heard shows why. A
/// house naming no address is heard over IPv4, as house-report sends.
fn bound(from: &[IpAddr], anywhere: bool) -> UdpSocket {
    let mut named: Vec<String> = from.iter().map(ToString::to_string).collect();
    if anywhere {
        named.push("any private IPv4 address, under a house's key".to_owned());
    }
    let mut said = false;
    loop {
        let mut refused = Vec::new();
        for any in wildcards(from) {
            match UdpSocket::bind((any, PORT)) {
                Ok(socket) => {
                    // Only IPv6's can be refused before another is had.
                    for e in &refused {
                        eprintln!("{e}: the house's reports over IPv6 go unheard");
                    }
                    eprintln!(
                        "the house's reports are heard on {any} port {PORT}, from {}",
                        named.join(", ")
                    );
                    return socket;
                }
                Err(e) => refused.push(e),
            }
        }
        if let Some(e) = refused.last().filter(|_| !said) {
            eprintln!("{e}: the house's reports on port {PORT} go unheard until it can be had");
            said = true;
        }
        // Another program may hold the port a while, as across a switch;
        // a report a second needs no quicker try.
        thread::sleep(Duration::from_secs(1));
    }
}

/// An IPv4 address heard on the IPv6 socket, as IPv4, so it matches the
/// address named.
fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_its_bytes_and_its_clock() {
        assert_eq!(
            report("1250000 9000\n"),
            Some(Report {
                bytes: 1_250_000,
                ms: 9_000
            })
        );
        for wrong in ["", "12", "12 x", "-1 5", "1 2 3"] {
            assert_eq!(report(wrong), None, "{wrong:?}");
        }
    }

    #[test]
    fn two_reports_give_the_rate_between_them_by_the_reporter_s_clock() {
        let was = Report {
            bytes: 0,
            ms: 1_000,
        };
        let now = Report {
            bytes: 500_000,
            ms: 2_000,
        };
        let kbit = rate(was, now).expect("a rate");
        assert!(
            (kbit - 4_000.0).abs() < 1e-9,
            "500 kB in a second is 4 Mb/s: {kbit}"
        );
    }

    #[test]
    fn reports_that_can_t_say_a_rate_give_none() {
        let was = Report {
            bytes: 900,
            ms: 5_000,
        };
        for (now, case) in [
            (
                Report {
                    bytes: 100,
                    ms: 6_000,
                },
                "the counter went back",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 5_000,
                },
                "the clock didn't move",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 4_000,
                },
                "the clock went back",
            ),
            (
                Report {
                    bytes: 999,
                    ms: 8_001,
                },
                "too far apart",
            ),
        ] {
            assert_eq!(rate(was, now), None, "{case}");
        }
        let apart = Report {
            bytes: 999,
            ms: 5_000 + APART_MS,
        };
        assert!(rate(was, apart).is_some(), "reports just near enough");
    }

    #[test]
    fn a_rate_is_never_read_past_the_most_a_house_may_name() {
        let was = Report { bytes: 0, ms: 0 };
        let now = Report {
            bytes: u64::MAX,
            ms: 1,
        };
        assert_eq!(rate(was, now), Some(SENT_MOST_KBIT), "a count off the wire");
    }

    // The next report reads 4 Mb/s against the last in order and 3 Mb/s
    // against the late one, so a late one taken as its baseline shows.
    #[test]
    fn a_copy_or_a_report_out_of_order_leaves_the_rate_as_it_was() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let ms = |ms: u64| at + Duration::from_millis(ms);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 0), at);
        ear.hear(mac, report(250_000, 1_000), ms(1_000));
        let half = Some((2_000.0, Duration::from_millis(500)));
        ear.hear(mac, report(250_000, 1_000), ms(1_500));
        assert_eq!(
            ear.sent(mac, ms(1_500)),
            half,
            "a copy, which renews nothing"
        );
        ear.hear(mac, report(0, 0), ms(1_500));
        assert_eq!(ear.sent(mac, ms(1_500)), half, "one come late");
        ear.hear(mac, report(750_000, 2_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((4_000.0, Duration::ZERO)),
            "the next read against the last in order"
        );
    }

    // A report far ahead on the reporter's clock, forged or from a clock
    // that jumped, would otherwise be what the next is read against, and
    // leave out every true one for `FRESH`.
    #[test]
    fn a_report_far_ahead_of_its_clock_is_left_out() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let ms = |ms: u64| at + Duration::from_millis(ms);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 0), at);
        ear.hear(mac, report(250_000, 1_000), ms(1_000));
        ear.hear(mac, report(0, u64::MAX), ms(1_500));
        ear.hear(mac, report(0, 1_000 + 500 + AHEAD_MS + 1), ms(1_500));
        ear.hear(mac, report(750_000, 2_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((4_000.0, Duration::ZERO)),
            "the true report after them heard at once"
        );
        ear.hear(mac, report(875_000, 3_000), ms(2_000));
        assert_eq!(
            ear.sent(mac, ms(2_000)),
            Some((1_000.0, Duration::ZERO)),
            "one a second ahead, as two the line held back come, taken"
        );
        ear.hear(mac, report(0, 3_000 + 500 + AHEAD_MS), ms(2_500));
        ear.hear(mac, report(1_000_000, 4_000), ms(3_000));
        ear.hear(mac, report(1_125_000, 5_000), ms(4_000));
        assert_eq!(
            ear.sent(mac, ms(4_000)),
            Some((1_000.0, Duration::from_secs(2))),
            "a forged one within the slack leaves out the true ones until they pass it"
        );
        ear.hear(mac, report(1_250_000, 6_000), ms(5_000));
        assert_eq!(
            ear.sent(mac, ms(5_000)),
            Some((10_000.0, Duration::ZERO)),
            "and the first past it is read against it, as against a forger's own reports"
        );
    }

    // The controller's ceiling holds the highest of the last few seconds,
    // stepped, while the other machine's sending swings each second.
    #[test]
    fn the_controller_s_ceiling_holds_still_while_the_house_s_swings() {
        let mut steady = Steady::default();
        let at = Instant::now();
        let home = |most| BTreeMap::from([("home".to_string(), most)]);
        let at_s = |s: u64| at + Duration::from_secs(s);
        let mut said = Vec::new();
        for (s, most) in (0..).zip([5_400, 4_600, 5_300, 4_500, 5_200, 4_700]) {
            said.push(steady.of(&home(most), at_s(s))["home"]);
        }
        assert_eq!(said, [5_250; 6], "the highest, stepped down");
        assert_eq!(
            steady.of(&home(4_600), at_s(7))["home"],
            5_250,
            "one 5 s old kept"
        );
        assert_eq!(
            steady.of(&home(4_600), at_s(8))["home"],
            5_000,
            "one 6 s old gone, the next highest stepped down"
        );
        assert_eq!(
            steady.of(&home(4_600), at_s(14))["home"],
            4_500,
            "a lower that lasts"
        );
        assert!(
            steady.of(&BTreeMap::new(), at_s(14)).is_empty(),
            "a house let go"
        );
        assert_eq!(
            steady.of(&home(4_100), at_s(14))["home"],
            4_000,
            "from nothing after it"
        );
        assert_eq!(
            crate::rate::whole(crate::oneway::MOST_KBIT_LEAST) % STEP_KBIT,
            0,
            "the least a trial may set is a step, so rounding never takes a ceiling under it"
        );
    }

    #[test]
    fn ipv4_houses_alone_listen_on_ipv4_and_any_ipv6_one_tries_ipv6_first() {
        let v4: IpAddr = "192.0.2.7".parse().expect("an address");
        let v6: IpAddr = "2001:db8::7".parse().expect("an address");
        let any4: IpAddr = Ipv4Addr::UNSPECIFIED.into();
        let any6: IpAddr = Ipv6Addr::UNSPECIFIED.into();
        assert_eq!(wildcards(&[v4]), [any4], "IPv4 alone");
        assert_eq!(
            wildcards(&[v4, v6]),
            [any6, any4],
            "IPv6's, then IPv4's where it can't be had"
        );
    }

    #[test]
    fn a_restarted_reporter_is_read_again_once_its_last_rate_is_past() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        let second = |s: u64| at + Duration::from_secs(s);
        let report = |bytes, ms| Report { bytes, ms };
        ear.hear(mac, report(0, 100_000), at);
        ear.hear(mac, report(250_000, 101_000), second(1));
        ear.hear(mac, report(0, 0), second(2));
        assert_eq!(
            ear.sent(mac, second(2)),
            Some((2_000.0, Duration::from_secs(1))),
            "its last rate stands through the restart"
        );
        ear.hear(mac, report(0, 0), second(5));
        ear.hear(mac, report(125_000, 1_000), second(6));
        assert_eq!(
            ear.sent(mac, second(6)),
            Some((1_000.0, Duration::ZERO)),
            "read from its restart on"
        );
    }

    #[test]
    fn only_a_report_from_an_address_named_is_taken() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().expect("an address");
        let other: IpAddr = "192.0.2.8".parse().expect("an address");
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let named = Reporters::from([(mac, None)]);
        let _ = ear.take(&named, other, b"0 0", at, 0);
        let _ = ear.take(&named, other, b"250000 1000", later, 0);
        assert_eq!(ear.sent(other, later), None, "an address not named");
        let _ = ear.take(&named, mapped, b"0 0", at, 0);
        let _ = ear.take(&named, mapped, b"not a report", later, 0);
        assert_eq!(ear.sent(mac, later), None, "no report in it");
        let _ = ear.take(&named, mapped, b"250000 1000", later, 0);
        assert_eq!(
            ear.sent(mac, later),
            Some((2_000.0, Duration::ZERO)),
            "the named address, heard on the IPv6 socket"
        );
    }

    /// The key the vector is made under: the bytes 0 to 31.
    fn vector_key() -> Vec<u8> {
        (0..32).collect()
    }

    /// The vector's stamp, and its report signed under `vector_key`, made
    /// with Python's hmac and openssl's dgst alike and written by
    /// house-report's own test of the same.
    const STAMP: u64 = 1_763_920_000_000;
    const VECTOR: &str = "1500 2000 1763920000000 ad712c900326603c04326ac069041fe5";

    #[test]
    fn a_report_signed_under_the_key_is_taken_with_its_stamp() {
        assert_eq!(
            signed(VECTOR, &vector_key(), STAMP + 500, None),
            Ok((
                Report {
                    bytes: 1_500,
                    ms: 2_000
                },
                STAMP
            )),
            "the vector house-report signs wasn't taken"
        );
    }

    #[test]
    fn a_forged_short_or_unsigned_report_is_refused_under_a_key() {
        let key = vector_key();
        let at = STAMP;
        let other_key: Vec<u8> = (1..33).collect();
        for (text, key, why) in [
            (
                "1500 2000 1763920000000 bd712c900326603c04326ac069041fe5",
                &key,
                "a mac a digit off",
            ),
            (
                "9500 2000 1763920000000 ad712c900326603c04326ac069041fe5",
                &key,
                "more bytes under the true mac",
            ),
            (VECTOR, &other_key, "another key"),
            ("1500 2000 1763920000000 ad", &key, "a mac of one byte"),
            (
                "1500 2000 1763920000000 ad712c900326603c04326ac069041f",
                &key,
                "a mac a byte short",
            ),
            (
                "1500 2000 1763920000000 +d712c900326603c04326ac069041fe5",
                &key,
                "a mac with a sign in it",
            ),
        ] {
            assert_eq!(
                signed(text, key, at, None),
                Err(Refused::Forged),
                "{why} was taken"
            );
        }
        for text in ["1500 2000", "1500 2000 1763920000000", "", "a b c d"] {
            assert_eq!(
                signed(text, &key, at, None),
                Err(Refused::Unsigned),
                "{text:?} wasn't refused as unsigned"
            );
        }
    }

    #[test]
    fn a_report_stamped_past_fresh_either_way_or_again_is_refused() {
        let key = vector_key();
        let fresh = u64::try_from(FRESH.as_millis()).expect("a few seconds");
        for (now, last, wanted, why) in [
            (STAMP + fresh, None, Ok(()), "just in time"),
            (STAMP - fresh, None, Ok(()), "just ahead in time"),
            (STAMP + fresh + 1, None, Err(Refused::Stale), "late"),
            (
                STAMP - fresh - 1,
                None,
                Err(Refused::Stale),
                "too far ahead",
            ),
            (STAMP, Some(STAMP), Err(Refused::Repeated), "sent again"),
            (
                STAMP,
                Some(STAMP + 1),
                Err(Refused::Repeated),
                "older than the last",
            ),
            (STAMP, Some(STAMP - 1), Ok(()), "newer than the last"),
        ] {
            assert_eq!(
                signed(VECTOR, &key, now, last).map(|_| ()),
                wanted,
                "a report stamped {why}"
            );
        }
    }

    #[test]
    fn a_house_with_no_key_takes_either_shape() {
        let plain = Report {
            bytes: 1_500,
            ms: 2_000,
        };
        assert_eq!(unkeyed("1500 2000"), Some(plain), "the old shape");
        assert_eq!(unkeyed(VECTOR), Some(plain), "a signed one, its mac unread");
        assert_eq!(unkeyed("1500 2000 1763920000000"), None, "three words");
    }

    #[test]
    fn a_keyed_house_hears_only_what_its_key_signed_and_nothing_while_it_can_t_be_read() {
        let folder = std::env::temp_dir().join(format!("shaper-house-key-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let path = folder.join("house.key");
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let keyed = Reporters::from([(mac, Some(path.clone()))]);
        let ear = Ear::default();
        let at = Instant::now();
        // Each read of the key a KEY_READ_APART after the last.
        let read = |n: u32| at + KEY_READ_APART * n;
        // Misshapen, a report is refused before any key is read, so with
        // none there it's said unsigned, not unkeyed (house signing round
        // 2's F4 and F7).
        let junk = ear.take(&keyed, mac, b"1500 2000", read(0), STAMP);
        let short_mac = ear.take(&keyed, mac, b"1500 2000 1763920000000 zz", read(0), STAMP);
        let unread = ear.take(&keyed, mac, VECTOR.as_bytes(), read(0), STAMP);
        fs::write(&path, [7_u8; KEY_LEAST - 1]).expect("a short key");
        let short = ear.take(&keyed, mac, VECTOR.as_bytes(), read(1), STAMP);
        fs::write(&path, vector_key()).expect("a key");
        let unsigned = ear.take(&keyed, mac, b"1500 2000", read(2), STAMP);
        // A forgery stamped ahead of the true report, and one stamped long
        // ago, are refused as forged, and their stamps never become the
        // last the true one is read against (house signing round 1's F7).
        let ahead_text = format!(
            "1500 2000 {} ad712c900326603c04326ac069041fe5",
            STAMP + 2_000
        );
        let ahead = ear.take(&keyed, mac, ahead_text.as_bytes(), read(2), STAMP);
        let long_ago = ear.take(
            &keyed,
            mac,
            b"1500 2000 1 ad712c900326603c04326ac069041fe5",
            read(2),
            STAMP,
        );
        let taken = ear.take(&keyed, mac, VECTOR.as_bytes(), read(2), STAMP);
        let again = ear.take(&keyed, mac, VECTOR.as_bytes(), read(2), STAMP);
        // A key read stands for KEY_READ_APART, then is read again.
        fs::remove_file(&path).expect("the key removed");
        let held = ear.take(&keyed, mac, ahead_text.as_bytes(), read(2), STAMP);
        let gone = ear.take(&keyed, mac, ahead_text.as_bytes(), read(3), STAMP);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            junk,
            Err(Unheard::Refused(Refused::Unsigned)),
            "a misshapen report read the key first"
        );
        assert_eq!(
            short_mac,
            Err(Unheard::Refused(Refused::Forged)),
            "a report with a misshapen mac read the key first"
        );
        assert_eq!(
            held,
            Err(Unheard::Refused(Refused::Forged)),
            "a forgery within a second of the key's read read it again"
        );
        assert!(
            matches!(&gone, Err(Unheard::Unkeyed(_))),
            "a key removed wasn't read again past KEY_READ_APART: {gone:?}"
        );
        assert!(
            matches!(&unread, Err(Unheard::Unkeyed(why)) if why.contains("house.key")),
            "a missing key wasn't said to be unread: {unread:?}"
        );
        assert!(
            matches!(&short, Err(Unheard::Unkeyed(why)) if why.contains("15 bytes")),
            "a 15-byte key was taken: {short:?}"
        );
        assert_eq!(unsigned, Err(Unheard::Refused(Refused::Unsigned)));
        for (forged, why) in [(ahead, "ahead"), (long_ago, "long ago")] {
            assert_eq!(
                forged,
                Err(Unheard::Refused(Refused::Forged)),
                "a forgery stamped {why} wasn't refused as forged"
            );
        }
        assert_eq!(
            taken,
            Ok(mac),
            "the signed report wasn't taken after forgeries"
        );
        assert_eq!(
            again,
            Err(Unheard::Refused(Refused::Repeated)),
            "the same report taken twice"
        );
        assert!(
            unheard_said(mac, None, &Unheard::Refused(Refused::Repeated))
                .is_some_and(|said| said.contains("192.0.2.7")),
            "a refusal goes unsaid, or without its address"
        );
        assert_eq!(
            unheard_said(mac, None, &Unheard::Ignored),
            None,
            "a stray datagram is said"
        );
    }

    #[test]
    fn refusals_are_said_once_a_minute_an_address_and_their_end_after() {
        let mut said = Said::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let other: IpAddr = "192.0.2.8".parse().expect("an address");
        let at = Instant::now();
        let secs = |s: u64| at + Duration::from_secs(s);
        let taken = Verdicts::taken(mac.into());
        let forged = refused(mac.into(), Refused::Forged);
        let unsigned = refused(mac.into(), Refused::Unsigned);
        let other_unsigned = refused(other.into(), Refused::Unsigned);
        let stray = Verdicts::refused((Some(mac.into()), Unheard::Ignored));
        assert_eq!(
            said.of(mac, &taken, at).len(),
            1,
            "the first report went unsaid"
        );
        assert!(
            said.of(mac, &taken, secs(1)).is_empty(),
            "a second report was said"
        );
        assert_eq!(
            said.of(mac, &forged, secs(2)).len(),
            1,
            "a forgery went unsaid"
        );
        assert!(
            said.of(mac, &unsigned, secs(3)).is_empty(),
            "another kind of refusal within the minute was said"
        );
        assert_eq!(
            said.of(other, &other_unsigned, secs(3)).len(),
            1,
            "another address's refusal went unsaid"
        );
        assert!(
            said.of(mac, &taken, secs(30)).is_empty(),
            "a report taken within the minute was said taken again"
        );
        assert!(
            said.of(mac, &taken, secs(62))
                .iter()
                .any(|line| line.contains("taken again")),
            "a report taken past the minute wasn't said taken again"
        );
        assert!(
            said.of(mac, &taken, secs(63)).is_empty(),
            "taken again said twice"
        );
        assert_eq!(
            said.of(other, &other_unsigned, secs(64)).len(),
            1,
            "a refusal past the minute went unsaid"
        );
        assert!(
            said.of(mac, &stray, secs(65)).is_empty(),
            "a stray was said"
        );
    }

    /// A datagram refused for `why`, as `whose` refusal.
    fn refused(whose: Source, why: Refused) -> Verdicts {
        Verdicts::refused((Some(whose), Unheard::Refused(why)))
    }

    /// `plain` signed under `key` at `stamp`, as house-report signs it.
    fn sign(plain: &str, stamp: u64, key: &[u8]) -> String {
        use std::fmt::Write;
        let stamped = format!("{plain} {stamp}");
        let mut hash = <Hmac<Sha256>>::new_from_slice(key).expect("any key fits an HMAC");
        hash.update(stamped.as_bytes());
        let mac = hash.finalize().into_bytes();
        let hex = mac[..MAC_BYTES]
            .iter()
            .fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            });
        format!("{stamped} {hex}")
    }

    #[test]
    fn the_test_s_signer_signs_as_house_report_does() {
        assert_eq!(
            sign("1500 2000", STAMP, &vector_key()),
            VECTOR,
            "the tests sign otherwise than the vector"
        );
    }

    #[test]
    fn a_keyed_house_naming_no_address_is_heard_from_any_by_its_key_alone() {
        let folder = std::env::temp_dir().join(format!("shaper-any-house-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let other = folder.join("other.key");
        fs::write(&other, (1..33).collect::<Vec<u8>>()).expect("another key");
        let named: IpAddr = "192.0.2.7".parse().expect("an address");
        let moved: IpAddr = "192.0.2.12".parse().expect("an address");
        let elsewhere: IpAddr = "192.0.2.13".parse().expect("an address");
        let house = "00:00:5e:00:53:01".to_owned();
        let reporters = Reporters {
            named: BTreeMap::from([(named, None)]),
            unnamed: vec![
                ("00:00:5e:00:53:09".to_owned(), other.clone()),
                (house.clone(), key.clone()),
            ],
        };
        let ear = Ear::default();
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let first = sign("0 0", STAMP, &vector_key());
        let next = sign("250000 1000", STAMP + 1_000, &vector_key());
        let taken = ear.take(&reporters, moved, first.as_bytes(), at, STAMP);
        // Kept by house, the stamp a replay meets is the house's, from
        // whichever address it comes.
        let replayed = ear.take(&reporters, elsewhere, first.as_bytes(), at, STAMP);
        let moved_on = ear.take(&reporters, elsewhere, next.as_bytes(), later, STAMP + 1_000);
        let forgery = sign("0 0", STAMP + 2_000, &[9; 32]);
        let forged = ear.take(&reporters, moved, forgery.as_bytes(), later, STAMP + 2_000);
        // A mac of another length is said forged too, as no house's (house
        // any round 2's F1; round 3's F10).
        let long = format!("0 0 {} {}", STAMP + 2_000, "ab".repeat(32));
        let misshapen = ear.heard(&reporters, moved, long.as_bytes(), later, STAMP + 2_000);
        let stray = ear.take(&reporters, moved, b"not a report", later, STAMP);
        let unsigned = ear.take(&reporters, moved, b"0 0", later, STAMP);
        let keyless = ear.take(&reporters, named, b"0 0", at, STAMP);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            taken,
            Ok(moved),
            "a report under the house's key went unheard"
        );
        assert_eq!(
            replayed,
            Err(Unheard::Refused(Refused::Repeated)),
            "a report replayed from another address was taken again"
        );
        assert_eq!(
            moved_on,
            Ok(elsewhere),
            "the house's next report, from another address, went unheard"
        );
        assert_eq!(
            ear.sent(Source::House(house), later),
            Some((2_000.0, Duration::ZERO)),
            "the house's rate wasn't read across its addresses as its own"
        );
        assert_eq!(
            (forged, misshapen),
            (
                Err(Unheard::Refused(Refused::Forged)),
                Err((None, Unheard::Refused(Refused::Forged)))
            ),
            "a report under no house's key, or with a mac of another length, wasn't forged"
        );
        assert_eq!(
            stray,
            Err(Unheard::Ignored),
            "a datagram from an unnamed address, not shaped as a report, was said"
        );
        assert_eq!(
            unsigned,
            Err(Unheard::Refused(Refused::Unsigned)),
            "an unsigned report from an unnamed address went unsaid"
        );
        assert_eq!(
            keyless,
            Ok(named),
            "an address a keyless house names went unheard beside a house naming none"
        );
    }

    #[test]
    fn refusals_no_house_s_key_signed_are_said_together_and_a_house_s_own_apart() {
        let mut said = Said::default();
        let one: IpAddr = "192.0.2.12".parse().expect("an address");
        let two: IpAddr = "192.0.2.13".parse().expect("an address");
        let named: IpAddr = "192.0.2.7".parse().expect("an address");
        let house = Source::House("00:00:5e:00:53:01".to_owned());
        let at = Instant::now();
        let secs = |s: u64| at + Duration::from_secs(s);
        let forged = Verdicts::refused((None, Unheard::Refused(Refused::Forged)));
        let again = |lines: Vec<String>| lines.iter().any(|line| line.contains("taken again"));
        assert_eq!(
            said.of(one, &forged, at).len(),
            1,
            "a forgery no house's key signed went unsaid"
        );
        assert!(
            said.of(two, &forged, secs(1)).is_empty(),
            "another address's forgery within the minute was said"
        );
        assert_eq!(
            said.of(named, &refused(named.into(), Refused::Forged), secs(1))
                .len(),
            1,
            "a named address's refusal was held with those no house's key signed"
        );
        assert_eq!(
            said.of(two, &refused(house.clone(), Refused::Stale), secs(2))
                .len(),
            1,
            "a forger's refusal hid the house's stale report"
        );
        assert!(
            !again(said.of(one, &Verdicts::taken(house.clone()), secs(30))),
            "the house's report within the minute was said taken again"
        );
        assert!(
            again(said.of(one, &Verdicts::taken(house.clone()), secs(63))),
            "the house's report past the minute wasn't said taken again"
        );
        // With no house's own refusal left to end, nothing is said taken
        // again (house any round 2's F4 and F10).
        let mut alone = Said::default();
        assert_eq!(alone.of(one, &forged, at).len(), 1, "a forgery went unsaid");
        assert!(
            !again(alone.of(two, &Verdicts::taken(house), secs(61))),
            "refusals no house's key signed were said to end with a house's report"
        );
    }

    // A house's refusal is said once a minute however often its reports
    // fail, and its end once, naming the house, when the first report past
    // the minute is taken for it, not one taken within it, while the
    // keyless address beside it says nothing more (every verdict round 1's
    // F8; names its taker round 1's F2, F3, F7 and F16).
    #[test]
    fn a_house_s_refusal_is_said_once_a_minute_and_its_end_once() {
        let folder =
            std::env::temp_dir().join(format!("shaper-taken-again-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        let one = "00:00:5e:00:53:01".to_owned();
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let reporters = Reporters {
            named: BTreeMap::from([(from, None)]),
            unnamed: vec![(one.clone(), key.clone())],
        };
        let ear = Ear::default();
        let mut said = Said::default();
        let at = Instant::now();
        let mut hear = |s: u64| {
            let stamp = STAMP + s * 1_000;
            let report = sign(
                &format!("{} {}", s * 250_000, s * 1_000),
                stamp,
                &vector_key(),
            );
            let now = at + Duration::from_secs(s);
            let verdicts = ear.verdicts(&reporters, from, report.as_bytes(), now, stamp);
            said.of(from, &verdicts, now)
        };
        let unread = hear(0);
        let still = hear(30);
        fs::write(&key, vector_key()).expect("a key");
        let mended = hear(40);
        let past = hear(63);
        let after = hear(64);
        let _ = fs::remove_dir_all(&folder);
        let house_one = format!("the house on {one}");
        assert_eq!(
            [&unread, &still, &mended, &past, &after].map(Vec::len),
            [2, 0, 1, 1, 0],
            "a step said more or less than its lines: \
             {unread:?} {still:?} {mended:?} {past:?} {after:?}"
        );
        // The keyless address's first report, then the house's refusal.
        assert_eq!(
            unread[0],
            format!("the first report from {from} for the house was heard"),
            "the keyless address's first report went unsaid: {unread:?}"
        );
        assert!(
            unread[1].contains(&house_one) && unread[1].contains("can't be read"),
            "the house's unread key went unsaid: {unread:?}"
        );
        assert_eq!(
            mended,
            [format!(
                "the first report from {from} for {house_one} was heard"
            )],
            "the house's first report within the minute wasn't said alone"
        );
        assert_eq!(
            past,
            [format!(
                "the reports from {from} for {house_one} are taken again"
            )],
            "the house's mended key wasn't said to end its refusal, once and by name"
        );
    }

    // Each house that takes one report says its first, not only the first
    // house to take it (names its taker round 1's F6).
    #[test]
    fn each_house_taking_one_report_says_its_first() {
        let folder = std::env::temp_dir().join(format!("shaper-both-take-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let one = "00:00:5e:00:53:01".to_owned();
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let reporters = Reporters {
            named: BTreeMap::from([(from, None)]),
            unnamed: vec![(one.clone(), key)],
        };
        let at = Instant::now();
        let report = sign("0 0", STAMP, &vector_key());
        let verdicts = Ear::default().verdicts(&reporters, from, report.as_bytes(), at, STAMP);
        let said = Said::default().of(from, &verdicts, at);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            said,
            [
                format!("the first report from {from} for the house on {one} was heard"),
                format!("the first report from {from} for the house was heard"),
            ],
            "a house taking the report beside another wasn't said to: {said:?}"
        );
    }

    // A house's end is said when it isn't the first to take the report:
    // beside an address with its own key, which takes first, a house whose
    // key comes to be read past the minute says its end (names its taker
    // round 2's F4).
    #[test]
    fn a_house_taking_after_an_address_s_own_key_says_its_end() {
        let folder =
            std::env::temp_dir().join(format!("shaper-taken-second-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let own = folder.join("own.key");
        fs::write(&own, vector_key()).expect("a key");
        let key = folder.join("house.key");
        let one = "00:00:5e:00:53:01".to_owned();
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let reporters = Reporters {
            named: BTreeMap::from([(from, Some(own))]),
            unnamed: vec![(one.clone(), key.clone())],
        };
        let ear = Ear::default();
        let mut said = Said::default();
        let at = Instant::now();
        let mut hear = |s: u64| {
            let stamp = STAMP + s * 1_000;
            let report = sign(
                &format!("{} {}", s * 250_000, s * 1_000),
                stamp,
                &vector_key(),
            );
            let now = at + Duration::from_secs(s);
            let verdicts = ear.verdicts(&reporters, from, report.as_bytes(), now, stamp);
            (verdicts.taken.clone(), said.of(from, &verdicts, now))
        };
        let (_, unread) = hear(0);
        fs::write(&key, vector_key()).expect("a key");
        let (taken, within) = hear(30);
        let (_, past) = hear(63);
        let _ = fs::remove_dir_all(&folder);
        let house_one = format!("the house on {one}");
        // Each step is counted whole, so a line past what it should say
        // fails it, and the house is pinned second in `taken`, the case
        // the test is for, so a change of the order fails here rather than
        // leaving the test to pass on the first taker (house ends round
        // 1's F5, F6 and F12).
        assert_eq!(
            unread.len(),
            2,
            "the address's first report and the house's unread key, alone: {unread:?}"
        );
        assert_eq!(
            unread[0],
            format!("the first report from {from} for the house was heard"),
            "the address's own key took first: {unread:?}"
        );
        assert!(
            unread[1].contains(&house_one) && unread[1].contains("can't be read"),
            "the house's unread key went unsaid: {unread:?}"
        );
        assert_eq!(
            taken,
            [Source::Address(from), Source::House(one.clone())],
            "the house no longer takes second, so this test no longer tests that"
        );
        assert_eq!(
            within,
            [format!(
                "the first report from {from} for {house_one} was heard"
            )],
            "the house taking second ended its refusal within the minute: {within:?}"
        );
        assert_eq!(
            past,
            [format!(
                "the reports from {from} for {house_one} are taken again"
            )],
            "the house taking second wasn't said to end its refusal: {past:?}"
        );
    }

    #[test]
    fn a_report_under_a_shared_key_is_every_house_s_that_holds_it() {
        let folder = std::env::temp_dir().join(format!("shaper-one-key-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let first = Source::House("00:00:5e:00:53:01".to_owned());
        let second = "00:00:5e:00:53:02".to_owned();
        let shared = Reporters {
            named: BTreeMap::new(),
            unnamed: vec![
                ("00:00:5e:00:53:01".to_owned(), key.clone()),
                (second.clone(), key),
            ],
        };
        let first_alone = Reporters {
            named: BTreeMap::new(),
            unnamed: shared.unnamed[..1].to_vec(),
        };
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let report = sign("0 0", STAMP, &vector_key());
        let next = sign("250000 1000", STAMP + 1_000, &vector_key());
        let ear = Ear::default();
        let taken = ear.heard(&shared, from, report.as_bytes(), at, STAMP);
        let replayed = ear.heard(&shared, from, report.as_bytes(), at, STAMP);
        let moved_on = ear.heard(&shared, from, next.as_bytes(), later, STAMP + 1_000);
        // The first house alone heard it; a replay once the second is
        // named meets the first's stamp, and the second takes it neither.
        let one_first = Ear::default();
        let _ = one_first.heard(&first_alone, from, report.as_bytes(), at, STAMP);
        let met = one_first.heard(&shared, from, report.as_bytes(), at, STAMP);
        let _ = one_first.heard(&shared, from, next.as_bytes(), later, STAMP + 1_000);
        let _ = fs::remove_dir_all(&folder);
        let both = (Ok(first.clone()), Ok(first.clone()));
        assert_eq!(
            (taken, moved_on),
            both,
            "reports under a shared key went unheard"
        );
        let rate = Some((2_000.0, Duration::ZERO));
        assert_eq!(
            (
                ear.sent(first.clone(), later),
                ear.sent(Source::House(second.clone()), later)
            ),
            (rate, rate),
            "reports under a shared key weren't every house's that holds it"
        );
        assert_eq!(
            (replayed, met),
            (
                Err((Some(first.clone()), Unheard::Refused(Refused::Repeated))),
                Err((Some(first.clone()), Unheard::Refused(Refused::Repeated)))
            ),
            "a replay a house's stamp refused wasn't refused as the first house's"
        );
        assert_eq!(
            one_first.sent(Source::House(second), later),
            None,
            "a replay one house's stamp refused was taken by another holding its key"
        );
    }

    // Two houses naming no address and sharing a key each say their first
    // report from an address, as `Said::of` says they do, not only the
    // first to take it (house ends round 1's F2 and F7).
    #[test]
    fn a_report_under_a_shared_key_says_its_first_for_each_house() {
        let folder =
            std::env::temp_dir().join(format!("shaper-shared-first-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let (one, two) = ("00:00:5e:00:53:01", "00:00:5e:00:53:02");
        let reporters = Reporters {
            named: BTreeMap::new(),
            unnamed: vec![(one.to_owned(), key.clone()), (two.to_owned(), key)],
        };
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let at = Instant::now();
        let report = sign("0 0", STAMP, &vector_key());
        let verdicts = Ear::default().verdicts(&reporters, from, report.as_bytes(), at, STAMP);
        let said = Said::default().of(from, &verdicts, at);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            said,
            [
                format!("the first report from {from} for the house on {one} was heard"),
                format!("the first report from {from} for the house on {two} was heard"),
            ],
            "a report under a shared key wasn't said first for each house: {said:?}"
        );
    }

    // A key that can't be read says more than a mac no key made, in either
    // order, and is its house's; neither outweighs a stamp refused, so a
    // replay one house refuses isn't taken by another whose stamp lags
    // (house any round 1's F5; round 2's F5 and F6; round 3's F4, F7 and
    // F9).
    #[test]
    fn the_most_telling_refusal_is_kept_in_either_order() {
        fn houses(unnamed: &[(&String, &PathBuf)]) -> Reporters {
            Reporters {
                named: BTreeMap::new(),
                unnamed: unnamed
                    .iter()
                    .map(|(network, path)| ((*network).clone(), (*path).clone()))
                    .collect(),
            }
        }
        let folder = std::env::temp_dir().join(format!("shaper-telling-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let other = folder.join("other.key");
        fs::write(&other, (1..33).collect::<Vec<u8>>()).expect("another key");
        let missing = folder.join("missing.key");
        let [one, two, three] = ["01", "02", "03"].map(|n| format!("00:00:5e:00:53:{n}"));
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let at = Instant::now();
        let forgery = sign("0 0", STAMP, &[9; 32]);
        let report = sign("0 0", STAMP, &vector_key());
        let ear = Ear::default();
        let unread_first = ear.heard(
            &houses(&[(&one, &missing), (&two, &other)]),
            from,
            forgery.as_bytes(),
            at,
            STAMP,
        );
        let unread_last = ear.heard(
            &houses(&[(&one, &other), (&two, &missing)]),
            from,
            forgery.as_bytes(),
            at,
            STAMP,
        );
        let _ = ear.heard(&houses(&[(&one, &key)]), from, report.as_bytes(), at, STAMP);
        let lagging = houses(&[(&one, &key), (&two, &key), (&three, &missing)]);
        let replayed = ear.heard(&lagging, from, report.as_bytes(), at, STAMP);
        let _ = fs::remove_dir_all(&folder);
        let unkeyed = |verdict: &Verdict, whose: &String| match verdict {
            Err((Some(Source::House(network)), Unheard::Unkeyed(_))) => network == whose,
            _ => false,
        };
        assert!(
            unkeyed(&unread_first, &one) && unkeyed(&unread_last, &two),
            "a key that can't be read wasn't said over a mac no key made, as its house's: \
             {unread_first:?}, {unread_last:?}"
        );
        assert_eq!(
            replayed,
            Err((
                Some(Source::House(one)),
                Unheard::Refused(Refused::Repeated)
            )),
            "a key that can't be read outweighed a stamp refused, letting a replay through"
        );
    }

    // Every house's refusal of a datagram is said, by whose it is, while
    // another house takes its report: an unnamed house's unread key beside
    // a keyless or keyed address, an address's own unread key beside an
    // unnamed house. A mac no house's key made, a second house's on a key
    // of its own included, goes unsaid beside a house that took the
    // report, and is no house's where none did (house any round 4's F2,
    // F5, F7 and F9; every verdict round 1's F1, F10 and F13).
    #[test]
    fn every_house_s_refusal_is_said_while_another_takes_the_report() {
        fn says(lines: &[String], words: &[&str]) -> bool {
            lines
                .iter()
                .any(|line| words.iter().all(|word| line.contains(word)))
        }
        let folder =
            std::env::temp_dir().join(format!("shaper-every-verdict-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let other = folder.join("other.key");
        fs::write(&other, (1..33).collect::<Vec<u8>>()).expect("another key");
        let missing = folder.join("missing.key");
        let [one, two] = ["01", "02"].map(|n| format!("00:00:5e:00:53:{n}"));
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let at = Instant::now();
        let report = sign("0 0", STAMP, &vector_key());
        let reporters =
            |named: Option<Option<&PathBuf>>, unnamed: &[(&String, &PathBuf)]| Reporters {
                named: named.map(|key| (from, key.cloned())).into_iter().collect(),
                unnamed: unnamed
                    .iter()
                    .map(|(network, path)| ((*network).clone(), (*path).clone()))
                    .collect(),
            };
        let said = |reporters: &Reporters, datagram: &str| {
            let verdicts = Ear::default().verdicts(reporters, from, datagram.as_bytes(), at, STAMP);
            Said::default().of(from, &verdicts, at)
        };
        let report = report.as_str();
        let keyless = said(&reporters(Some(None), &[(&one, &missing)]), report);
        let keyed = said(&reporters(Some(Some(&key)), &[(&one, &missing)]), report);
        let unread_own = said(&reporters(Some(Some(&missing)), &[(&one, &key)]), report);
        let two_keys = said(&reporters(None, &[(&one, &key), (&two, &other)]), report);
        let plain = said(&reporters(Some(None), &[(&one, &key)]), "0 0");
        let unmade = Ear::default().heard(
            &reporters(None, &[(&one, &key)]),
            from,
            sign("0 0", STAMP, &[9; 32]).as_bytes(),
            at,
            STAMP,
        );
        let _ = fs::remove_dir_all(&folder);
        // Counted whole, so a line of any kind or wording past what each
        // case should say fails it, not only the words looked for (every
        // verdict round 2's F3).
        let first = "the first report from 192.0.2.12 for the house was heard";
        let unread_one = format!("the house on {one}");
        let unread_one = unread_one.as_str();
        assert_eq!(
            [&keyless, &keyed, &unread_own, &two_keys, &plain].map(Vec::len),
            [2, 2, 2, 1, 1],
            "a case said more or less than its lines: {keyless:?} {keyed:?} {unread_own:?} \
             {two_keys:?} {plain:?}"
        );
        assert!(
            says(&keyless, &[first]) && says(&keyless, &[unread_one, "can't be read"]),
            "an unnamed house's unread key went unsaid beside a keyless address: {keyless:?}"
        );
        assert!(
            says(&keyed, &[first]) && says(&keyed, &[unread_one, "can't be read"]),
            "an unnamed house's unread key went unsaid beside a keyed address: {keyed:?}"
        );
        assert!(
            says(&unread_own, &["first report from", unread_one])
                && says(&unread_own, &["nothing for the house,", "can't be read"]),
            "an address's own unread key went unsaid beside an unnamed house: {unread_own:?}"
        );
        assert!(
            says(&two_keys, &["first report from", unread_one]) && !says(&two_keys, &["mac isn't"]),
            "a second house on a key of its own was said forged: {two_keys:?}"
        );
        assert!(
            says(&plain, &[first]) && !says(&plain, &["unsigned"]),
            "a keyless address's plain report was said unsigned to a keyed house: {plain:?}"
        );
        assert_eq!(
            unmade,
            Err((None, Unheard::Refused(Refused::Forged))),
            "a well-shaped mac no house's key made was taken as a house's refusal"
        );
    }

    // A mac a house's own key didn't make is said forged only when no
    // house's key made it: not from an address a keyed house names that
    // an unnamed house was leased, taken or replayed, and not beside a
    // house's own refusal, while a stray's verdict says nothing in its
    // place (every verdict round 1's F2, F5, F6, F11 and F14).
    #[test]
    fn no_house_s_key_unmade_mac_is_said_beside_one_its_key_made() {
        fn says(lines: &[String], words: &[&str]) -> bool {
            lines
                .iter()
                .any(|line| words.iter().all(|word| line.contains(word)))
        }
        let folder = std::env::temp_dir().join(format!("shaper-unmade-mac-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let other_key: Vec<u8> = (1..33).collect();
        let other = folder.join("other.key");
        fs::write(&other, &other_key).expect("another key");
        let missing = folder.join("missing.key");
        let one = "00:00:5e:00:53:01".to_owned();
        let from: IpAddr = "192.0.2.12".parse().expect("an address");
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let report = sign("0 0", STAMP, &vector_key());
        let leased = Reporters {
            named: BTreeMap::from([(from, Some(other))]),
            unnamed: vec![(one.clone(), key.clone())],
        };
        let ear = Ear::default();
        let mut said = Said::default();
        let taken = ear.verdicts(&leased, from, report.as_bytes(), at, STAMP);
        let taken = said.of(from, &taken, at);
        let replayed = ear.verdicts(&leased, from, report.as_bytes(), later, STAMP);
        let replayed = said.of(from, &replayed, later);
        // The address's own house's first report, after another's from the
        // same address, is said too (every verdict round 2's F7).
        let own = sign("0 1", STAMP + 2_000, &other_key);
        let own = ear.verdicts(&leased, from, own.as_bytes(), later, STAMP + 2_000);
        let own = said.of(from, &own, later);
        let keyless = Reporters {
            named: BTreeMap::from([(from, None)]),
            unnamed: vec![(one.clone(), key.clone())],
        };
        let unread = Reporters {
            named: BTreeMap::new(),
            unnamed: vec![
                (one.clone(), missing),
                ("00:00:5e:00:53:02".to_owned(), key),
            ],
        };
        let forged = sign("0 0", STAMP, &[9; 32]);
        let unread = Ear::default().verdicts(&unread, from, forged.as_bytes(), at, STAMP);
        let unread = Said::default().of(from, &unread, at);
        // A mac that isn't 16 hex bytes is forged before any key is read,
        // and five words are no keyless report, so a keyless address
        // ignores it while no house took it (every verdict round 2's F4).
        // It rests on the tab: the signed text is split at its last space,
        // reading "ab\tcd" as one mac, and a keyless report at any
        // whitespace, reading five words (names its taker round 1's F9 and
        // F19).
        let misshapen = format!("0 0 {STAMP} ab\tcd");
        let stray = Ear::default().verdicts(&keyless, from, misshapen.as_bytes(), at, STAMP);
        let _ = fs::remove_dir_all(&folder);
        let stray = Said::default().of(from, &stray, at);
        let house_one = format!("the house on {one}");
        let house_one = house_one.as_str();
        assert_eq!(
            [&taken, &replayed, &own, &unread, &stray].map(Vec::len),
            [1, 1, 1, 1, 1],
            "a case said more or less than its line: {taken:?} {replayed:?} {own:?} {unread:?} \
             {stray:?}"
        );
        assert!(
            says(&taken, &["first report from", house_one]) && !says(&taken, &["mac isn't"]),
            "a house's report from an address it was leased wasn't said as its own: {taken:?}"
        );
        assert!(
            says(
                &own,
                &["first report from 192.0.2.12 for the house was heard"]
            ),
            "the address's own house's first report went unsaid after another's: {own:?}"
        );
        assert!(
            says(&replayed, &[house_one, "sent again"]) && !says(&replayed, &["mac isn't"]),
            "a replay from a leased address wasn't said as the house's alone: {replayed:?}"
        );
        assert!(
            says(&unread, &[house_one, "can't be read"]) && !says(&unread, &["mac isn't"]),
            "a forgery beside a house's unread key wasn't left to the key's line: {unread:?}"
        );
        assert!(
            says(&stray, &["mac isn't"]),
            "a stray's verdict kept a forgery from being said: {stray:?}"
        );
    }

    #[test]
    fn a_house_naming_no_address_is_heard_from_one_another_house_names() {
        let folder = std::env::temp_dir().join(format!("shaper-leased-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let named_key: Vec<u8> = (1..33).collect();
        let other = folder.join("other.key");
        fs::write(&other, &named_key).expect("another key");
        let keyless: IpAddr = "192.0.2.7".parse().expect("an address");
        let keyed: IpAddr = "192.0.2.8".parse().expect("an address");
        let house = Source::House("00:00:5e:00:53:01".to_owned());
        let reporters = Reporters {
            named: BTreeMap::from([(keyless, None), (keyed, Some(other))]),
            unnamed: vec![("00:00:5e:00:53:01".to_owned(), key)],
        };
        let ear = Ear::default();
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let latest = at + Duration::from_secs(2);
        let first = sign("0 0", STAMP, &vector_key());
        let next = sign("250000 1000", STAMP + 1_000, &vector_key());
        let third = sign("500000 2000", STAMP + 2_000, &vector_key());
        let own = sign("0 0", STAMP, &named_key);
        let from_keyless = ear.heard(&reporters, keyless, first.as_bytes(), at, STAMP);
        let keyless_again = ear.heard(&reporters, keyless, first.as_bytes(), at, STAMP);
        let from_keyed = ear.heard(&reporters, keyed, next.as_bytes(), later, STAMP + 1_000);
        // The keyless house hears the signed reports beside the house
        // naming none (house any round 3's F3 and F8).
        let _ = ear.heard(&reporters, keyless, third.as_bytes(), latest, STAMP + 2_000);
        let own_taken = ear.heard(&reporters, keyed, own.as_bytes(), at, STAMP);
        let own_again = ear.heard(&reporters, keyed, own.as_bytes(), at, STAMP);
        let plain = ear.heard(&reporters, keyless, b"0 0", at, STAMP);
        let _ = fs::remove_dir_all(&folder);
        let rate = Some((2_000.0, Duration::ZERO));
        assert_eq!(
            (ear.sent(keyless, latest), ear.sent(house.clone(), latest)),
            (rate, rate),
            "signed reports from a keyless house's address weren't both houses'"
        );
        assert_eq!(
            keyless_again,
            Err((Some(house.clone()), Unheard::Refused(Refused::Repeated))),
            "a replay from a keyless house's address wasn't refused on the house's stamp"
        );
        assert_eq!(
            from_keyless,
            Ok(house.clone()),
            "the house's report from an address a keyless house names was taken as that one's"
        );
        assert_eq!(
            from_keyed,
            Ok(house.clone()),
            "the house's report from an address a keyed house names was refused"
        );
        assert_eq!(
            (own_taken, own_again),
            (
                Ok(Source::Address(keyed)),
                Err((
                    Some(Source::Address(keyed)),
                    Unheard::Refused(Refused::Repeated)
                ))
            ),
            "a report the named house's key signed wasn't that address's own"
        );
        assert_eq!(
            plain,
            Ok(Source::Address(keyless)),
            "a keyless house's unsigned report went unheard beside a house naming no address"
        );
    }

    // The named house holds the unnamed one's key: each hears what the
    // other's address sends, a replay either stamp refuses is refused at
    // both addresses, and a forgery is the named address's own (house any
    // round 2's F3, F9 and F12; round 3's F1, F8, F11 and F13).
    #[test]
    fn a_named_house_on_a_house_s_key_meets_its_stamp_both_ways() {
        let folder = std::env::temp_dir().join(format!("shaper-both-ways-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let key = folder.join("house.key");
        fs::write(&key, vector_key()).expect("a key");
        let keyed: IpAddr = "192.0.2.8".parse().expect("an address");
        let elsewhere: IpAddr = "192.0.2.13".parse().expect("an address");
        let house = Source::House("00:00:5e:00:53:01".to_owned());
        let one_key = Reporters {
            named: BTreeMap::from([(keyed, Some(key.clone()))]),
            unnamed: vec![("00:00:5e:00:53:01".to_owned(), key)],
        };
        let at = Instant::now();
        let later = at + Duration::from_secs(1);
        let latest = at + Duration::from_secs(2);
        let first = sign("0 0", STAMP, &vector_key());
        let next = sign("250000 1000", STAMP + 1_000, &vector_key());
        let third = sign("500000 2000", STAMP + 2_000, &vector_key());
        let forgery = sign("0 0", STAMP + 1_000, &[9; 32]);
        let shared = Ear::default();
        let shared_taken = shared.heard(&one_key, keyed, first.as_bytes(), at, STAMP);
        let shared_again = shared.heard(&one_key, keyed, first.as_bytes(), at, STAMP);
        let shared_elsewhere = shared.heard(&one_key, elsewhere, first.as_bytes(), at, STAMP);
        let shared_next = shared.heard(&one_key, keyed, next.as_bytes(), later, STAMP + 1_000);
        let forged = shared.heard(&one_key, keyed, forgery.as_bytes(), later, STAMP + 1_000);
        let third_elsewhere =
            shared.heard(&one_key, elsewhere, third.as_bytes(), latest, STAMP + 2_000);
        let third_again = shared.heard(&one_key, keyed, third.as_bytes(), latest, STAMP + 2_000);
        let _ = fs::remove_dir_all(&folder);
        let rate = Some((2_000.0, Duration::ZERO));
        assert_eq!(
            (shared_next, third_elsewhere),
            (Ok(Source::Address(keyed)), Ok(house.clone())),
            "reports under the shared key went unheard"
        );
        assert_eq!(
            (
                shared.sent(keyed, later),
                shared.sent(house.clone(), latest)
            ),
            (rate, rate),
            "reports at the named address weren't the unnamed house's too"
        );
        assert_eq!(
            third_again,
            Err((Some(house.clone()), Unheard::Refused(Refused::Repeated))),
            "a report the unnamed house took was taken again at the named address"
        );
        let address = Some(Source::Address(keyed));
        assert_eq!(
            (shared_taken, shared_again),
            (
                Ok(Source::Address(keyed)),
                Err((address.clone(), Unheard::Refused(Refused::Repeated)))
            ),
            "a replay the named house's stamp refused wasn't its own, under a shared key"
        );
        assert_eq!(
            (shared_elsewhere, forged),
            (
                Err((Some(house), Unheard::Refused(Refused::Repeated))),
                Err((address, Unheard::Refused(Refused::Forged)))
            ),
            "a report the named house took was taken again elsewhere, or its forgery not its own"
        );
    }

    #[test]
    fn a_keyed_house_naming_no_address_is_heard_by_its_key_and_a_keyless_one_not_at_all() {
        use crate::networks::House;
        let key = PathBuf::from("/etc/house.key");
        let house = |key: Option<PathBuf>| House {
            most: 7_000,
            from: Vec::new(),
            key,
        };
        let houses = crate::networks::Houses::from([
            ("00:00:5e:00:53:01".to_string(), house(Some(key.clone()))),
            ("00:00:5e:00:53:02".to_string(), house(None)),
        ]);
        assert_eq!(
            reporters(&houses),
            Reporters {
                named: BTreeMap::new(),
                unnamed: vec![("00:00:5e:00:53:01".to_string(), key)],
            },
            "a house naming no address wasn't heard by its key alone"
        );
    }

    #[test]
    fn an_address_two_houses_name_is_held_to_either_s_key() {
        use crate::networks::House;
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let key = PathBuf::from("/etc/house.key");
        let house = |key: Option<PathBuf>| House {
            most: 7_000,
            from: vec![mac],
            key,
        };
        for (first, second) in [(None, Some(key.clone())), (Some(key.clone()), None)] {
            let houses = crate::networks::Houses::from([
                ("00:00:5e:00:53:01".to_string(), house(first)),
                ("00:00:5e:00:53:02".to_string(), house(second)),
            ]);
            assert_eq!(
                reporters(&houses),
                Reporters::from([(mac, Some(key.clone()))]),
                "one house leaving the key out undid the other's"
            );
        }
    }

    #[test]
    fn the_house_s_ceiling_holds_what_the_other_machine_leaves() {
        let now = Duration::from_millis(500);
        assert_eq!(held(7_000, Some((2_500.0, now)), 1_000), Some(4_500));
        assert_eq!(
            held(7_000, Some((6_800.0, now)), 1_000),
            Some(1_000),
            "never under the floor"
        );
        assert_eq!(
            held(7_000, Some((0.0, now)), 1_000),
            Some(7_000),
            "a quiet machine leaves the whole ceiling"
        );
    }

    #[test]
    fn a_stale_report_or_none_holds_nothing() {
        assert_eq!(held(7_000, None, 1_000), None, "no report");
        assert_eq!(
            held(
                7_000,
                Some((2_500.0, FRESH + Duration::from_millis(1))),
                1_000
            ),
            None,
            "a report past its time"
        );
        assert_eq!(
            held(7_000, Some((2_500.0, FRESH)), 1_000),
            Some(4_500),
            "one just in time"
        );
    }

    #[test]
    fn the_ear_gives_the_rate_from_a_reporter_s_last_two_reports() {
        let ear = Ear::default();
        let mac: IpAddr = "192.0.2.7".parse().expect("an address");
        let at = Instant::now();
        ear.hear(mac, Report { bytes: 0, ms: 0 }, at);
        assert_eq!(ear.sent(mac, at), None, "one report gives no rate");
        ear.hear(
            mac,
            Report {
                bytes: 250_000,
                ms: 1_000,
            },
            at,
        );
        let later = at + Duration::from_secs(2);
        assert_eq!(
            ear.sent(mac, later),
            Some((2_000.0, Duration::from_secs(2)))
        );
        let other: IpAddr = "192.0.2.8".parse().expect("an address");
        assert_eq!(ear.sent(other, later), None, "another address said nothing");
    }

    #[test]
    fn an_ipv4_address_heard_on_the_ipv6_socket_reads_as_ipv4() {
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().expect("an address");
        assert_eq!(
            canonical(mapped),
            "192.0.2.7".parse::<IpAddr>().expect("an address")
        );
        let v6: IpAddr = "2001:db8::7".parse().expect("an address");
        assert_eq!(canonical(v6), v6);
    }
}
