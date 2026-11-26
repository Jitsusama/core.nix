//! Speaking to a light: the reports it reads and the answers it gives,
//! its range, and what it says of itself, its own buttons' changes among it.

use std::ffi::c_int;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{POLLIN, PollFd, poll};

const REPORT: usize = 20;

pub(crate) const POWER: u8 = 0x1c;
pub(crate) const POWER_IS: u8 = 0x01;
pub(crate) const BRIGHTNESS: u8 = 0x4c;
pub(crate) const BRIGHTNESS_IS: u8 = 0x31;
pub(crate) const TEMPERATURE: u8 = 0x9c;
pub(crate) const TEMPERATURE_IS: u8 = 0x81;
/// What the light says unasked when its own buttons change it.
const POWER_CHANGED: u8 = 0x00;
const BRIGHTNESS_CHANGED: u8 = 0x10;
const TEMPERATURE_CHANGED: u8 = 0x20;
/// The Glow's range: its own units for brightness, kelvin for warmth.
pub(crate) const DIMMEST: u16 = 20;
pub(crate) const BRIGHTEST: u16 = 250;
pub(crate) const WARMEST: u16 = 2700;
const COOLEST: u16 = 6500;

const O_NONBLOCK: c_int = 0o4000;
const LOCK_EX: c_int = 2;
const LOCK_NB: c_int = 4;
const LOCK_UN: c_int = 8;
/// How long a light's turn is waited for. A litra holds it while it sends
/// and waits up to a second for the reply, and leaves it free for `GAP`
/// before it sends again, so one held longer is held by a program that
/// won't let go, which every litra would otherwise wait on for good,
/// following the cameras included.
const TURN: Duration = Duration::from_secs(3);
/// How far apart the turn is tried for while another holds it: flock can't
/// wait with a bound, so it's asked without waiting, again and again.
const TRYING: Duration = Duration::from_millis(10);
/// How long a litra leaves the turn free between two of its own sends:
/// more than twice `TRYING`, so another trying for it meets it free, and
/// a run of sends from one can't keep the others out for the whole bound.
const GAP: Duration = Duration::from_millis(25);
/// How long after a light couldn't be told or found out, as when another
/// held its turn past the bound or it didn't answer, it's tried again: at
/// first, and at most, each time twice as long as the last, so one that
/// never answers costs the others little.
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LAST_RETRY: Duration = Duration::from_secs(60);

/// The wait before a light is tried again after waiting `last`, or the
/// first.
pub(crate) fn next_retry(last: Option<Duration>) -> Duration {
    last.map_or(FIRST_RETRY, |wait| (wait * 2).min(LAST_RETRY))
}

/// How long poll may wait, in milliseconds, for a try due `at`: for ever
/// with none due.
pub(crate) fn until(at: Option<Instant>) -> c_int {
    at.map_or(-1, |at| {
        let left = at.saturating_duration_since(Instant::now()).as_millis();
        c_int::try_from(left).unwrap_or(c_int::MAX)
    })
}

unsafe extern "C" {
    fn flock(fd: c_int, operation: c_int) -> c_int;
}

/// A report the light reads: its header, the command, then the values.
fn report(command: u8, values: &[u8]) -> [u8; REPORT] {
    let mut report = [0; REPORT];
    report[..4].copy_from_slice(&[0x11, 0xff, 0x04, command]);
    report[4..4 + values.len()].copy_from_slice(values);
    report
}

/// The values in the light's answer to a question, once one for that
/// question comes.
fn answer(reply: &[u8], question: u8) -> Option<&[u8]> {
    match reply {
        [0x11, 0xff, 0x04, code, values @ ..] if *code == question => Some(values),
        _ => None,
    }
}

/// A share of the brightest, as the light's own units.
pub(crate) fn units(percent: u16) -> u16 {
    let percent = percent.min(100);
    DIMMEST + ((BRIGHTEST - DIMMEST) * percent + 50) / 100
}

/// The light's own units as a share of the brightest.
pub(crate) fn percent(units: u16) -> u16 {
    let units = units.clamp(DIMMEST, BRIGHTEST) - DIMMEST;
    (units * 100 + (BRIGHTEST - DIMMEST) / 2) / (BRIGHTEST - DIMMEST)
}

/// Warmth the light can make, to its nearest hundred kelvin.
pub(crate) fn kelvin(asked: u16) -> u16 {
    (asked.clamp(WARMEST, COOLEST) + 50) / 100 * 100
}

/// A light's node, open. What's sent, asked or told, goes through
/// `exchange`, which takes the light's turn first: an flock on the node,
/// which every litra holds while it sends, whichever process it runs in,
/// waited for no longer than `turn`. The file is private to this module,
/// so nothing outside it can send around the turn through this descriptor;
/// code in this file is trusted to send through `exchange` alone. The node
/// opened again, by its path or through /proc, is held to the turn only
/// by what takes the lock on it, so a program writing it without the lock
/// isn't.
pub(crate) struct Light {
    file: File,
    turn: Duration,
    /// When this one last let go of the turn.
    let_go: Option<Instant>,
}

/// A raw descriptor, for poll, rather than `AsFd`: a borrowed descriptor
/// clones into an owned file without `unsafe`, which could then write
/// around the turn, while a raw one is written through only by `unsafe`
/// code. Neither stops the node being opened again.
impl AsRawFd for Light {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl Light {
    /// Reads what the light says unasked, such as its buttons. Reading needs
    /// no turn: only a report sent can be lost to one sent beside it.
    pub(crate) fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.read(buffer)
    }

    pub(crate) fn open(node: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(node)?;
        Ok(Self {
            file,
            turn: TURN,
            let_go: None,
        })
    }

    /// Takes the light's turn once another lets go of it, or fails once
    /// it has waited `turn`, rather than wait on a holder that won't. Just
    /// after this one let go, it leaves the turn free for `GAP` first.
    fn take_turn(&self) -> io::Result<()> {
        if let Some(let_go) = self.let_go {
            std::thread::sleep((let_go + GAP).saturating_duration_since(Instant::now()));
        }
        let fd = self.file.as_raw_fd();
        let deadline = Instant::now() + self.turn;
        loop {
            // SAFETY: a plain call on a descriptor of our own.
            if unsafe { flock(fd, LOCK_EX | LOCK_NB) } == 0 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            match error.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::WouldBlock if Instant::now() < deadline => {
                    std::thread::sleep(TRYING);
                }
                ErrorKind::WouldBlock => {
                    let held = format!("the light's turn was held for {:?}", self.turn);
                    return Err(io::Error::new(ErrorKind::TimedOut, held));
                }
                _ => return Err(error),
            }
        }
    }

    /// Sends a report and waits up to a second for the light's reply to it,
    /// an answer to a question or the bare acknowledgement of a command,
    /// handing whatever else it says meanwhile, such as its buttons, to
    /// `other`. The light keeps only the latest of reports sent closer than
    /// about 40 ms apart, so every program speaking to it takes its turn
    /// under a lock on its node, and lets go once the light has replied.
    fn exchange(
        &mut self,
        command: u8,
        values: &[u8],
        other: &mut dyn FnMut(&[u8]),
    ) -> io::Result<Vec<u8>> {
        self.take_turn()?;
        let replied = self
            .file
            .write_all(&report(command, values))
            .and_then(|()| self.reply(command, other));
        self.let_go_of_turn();
        replied
    }

    /// Lets go of the light's turn, noting when.
    fn let_go_of_turn(&mut self) {
        // SAFETY: a plain call on a descriptor of our own; closing the
        // file would let go of it too.
        unsafe { flock(self.file.as_raw_fd(), LOCK_UN) };
        self.let_go = Some(Instant::now());
    }

    /// Tells the light something and waits for it to say it heard.
    pub(crate) fn tell(&mut self, command: u8, values: &[u8]) -> io::Result<()> {
        self.exchange(command, values, &mut |_| {}).map(drop)
    }

    fn reply(&mut self, question: u8, other: &mut dyn FnMut(&[u8])) -> io::Result<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut reply = [0u8; 64];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(io::Error::new(ErrorKind::TimedOut, "no answer"));
            }
            let mut waiting = [PollFd {
                fd: self.file.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            }];
            let millis = c_int::try_from(left.as_millis()).unwrap_or(1000).max(1);
            // SAFETY: one valid pollfd for the length given.
            if unsafe { poll(waiting.as_mut_ptr(), 1, millis) } <= 0 {
                continue;
            }
            match self.file.read(&mut reply) {
                Ok(length) => match answer(&reply[..length], question) {
                    Some(values) => return Ok(values.to_vec()),
                    None => other(&reply[..length]),
                },
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

/// What a light is doing, as far as it has said.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct State {
    pub(crate) on: Option<bool>,
    pub(crate) units: Option<u16>,
    pub(crate) kelvin: Option<u16>,
}

impl State {
    fn complete(&self) -> bool {
        self.on.is_some() && self.units.is_some() && self.kelvin.is_some()
    }
}

/// What a report from the light calls for.
#[derive(Debug, PartialEq)]
pub(crate) enum Heard {
    /// It said something new about itself.
    Changed,
    /// A command changed it without saying to what, so ask.
    Ask(u8),
    Nothing,
}

/// Takes in a report the light sent: a button's new value, an answer to
/// anybody's question, or the bare acknowledgement of anybody's command.
pub(crate) fn hear(state: &mut State, report: &[u8]) -> Heard {
    let [0x11, 0xff, 0x04, code, values @ ..] = report else {
        return Heard::Nothing;
    };
    let word = values
        .get(..2)
        .map(|word| u16::from_be_bytes([word[0], word[1]]));
    let before = *state;
    match *code {
        POWER_CHANGED | POWER_IS => {
            if let Some(on) = values.first() {
                state.on = Some(*on == 1);
            }
        }
        BRIGHTNESS_CHANGED | BRIGHTNESS_IS => state.units = word.or(state.units),
        TEMPERATURE_CHANGED | TEMPERATURE_IS => state.kelvin = word.or(state.kelvin),
        POWER => return Heard::Ask(POWER_IS),
        BRIGHTNESS => return Heard::Ask(BRIGHTNESS_IS),
        TEMPERATURE => return Heard::Ask(TEMPERATURE_IS),
        _ => return Heard::Nothing,
    }
    if *state == before {
        Heard::Nothing
    } else {
        Heard::Changed
    }
}

/// Asks a light everything about itself.
pub(crate) fn ask_state(node: &Path) -> io::Result<State> {
    let mut state = State::default();
    find_out(
        &mut Light::open(node)?,
        &mut state,
        vec![TEMPERATURE_IS, BRIGHTNESS_IS, POWER_IS],
    )?;
    if state.complete() {
        Ok(state)
    } else {
        Err(io::Error::new(ErrorKind::InvalidData, "a short answer"))
    }
}

/// Asks a light each question in turn, last first, taking in whatever else
/// it says meanwhile, and asking about any command that's heard.
pub(crate) fn find_out(
    light: &mut Light,
    state: &mut State,
    mut asking: Vec<u8>,
) -> io::Result<()> {
    while let Some(question) = asking.pop() {
        let mut heard = Vec::new();
        let values = light.exchange(question, &[], &mut |report| {
            if let Heard::Ask(next) = hear(state, report) {
                heard.push(next);
            }
        })?;
        let mut answer = vec![0x11, 0xff, 0x04, question];
        answer.extend(values);
        hear(state, &answer);
        for next in heard {
            if !asking.contains(&next) {
                asking.insert(0, next);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use std::fs;

    /// Whether `file` was given the flock `operation` asks.
    fn locked(file: &File, operation: c_int) -> bool {
        // SAFETY: a plain call on a descriptor the file holds open.
        unsafe { flock(file.as_raw_fd(), operation) == 0 }
    }

    #[test]
    fn a_turn_held_elsewhere_keeps_back_what_is_sent_no_longer_than_its_bound() {
        let root = root("turn");
        let node = root.join("node");
        fs::write(&node, "").expect("a stand-in node is made");
        let holder = File::open(&node).expect("the stand-in node opens");
        assert!(
            locked(&holder, LOCK_EX | LOCK_NB),
            "another litra's turn wasn't taken"
        );
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&node)
            .expect("the stand-in node opens to be written");
        let turn = Duration::from_millis(50);
        let mut light = Light {
            file,
            turn,
            let_go: None,
        };
        let (light, held, waited) = within_ten_seconds(move || {
            let started = Instant::now();
            let held = light.tell(POWER, &[1]).map_err(|error| error.kind());
            (light, held, started.elapsed())
        });
        let written = fs::read(&node).expect("the stand-in node reads").len();
        assert!(
            locked(&holder, LOCK_UN),
            "another litra's turn wasn't let go of"
        );
        let taken = light.take_turn().map_err(|error| error.kind());
        let kept_out = locked(&holder, LOCK_EX | LOCK_NB);
        assert_eq!(
            (
                held,
                written,
                waited >= turn,
                waited < TURN,
                taken,
                kept_out
            ),
            (Err(ErrorKind::TimedOut), 0, true, true, Ok(()), false),
            "a report was sent around a turn held elsewhere, or the turn was waited for \
             other than its bound ({waited:?}), or not taken once let go of"
        );
    }

    /// A light on the stand-in `node`, waiting `turn` at most for it.
    fn stand_in(node: &Path, turn: Duration) -> Light {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(node)
            .expect("the stand-in node opens to be written");
        Light {
            file,
            turn,
            let_go: None,
        }
    }

    /// What `taking` gives, run apart, or a failure by its message should
    /// it still be waiting after ten seconds, as one that ignored its bound
    /// would wait for ever.
    fn within_ten_seconds<T: Send + 'static>(taking: impl FnOnce() -> T + Send + 'static) -> T {
        let (send, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = send.send(taking());
        });
        heard
            .recv_timeout(Duration::from_secs(10))
            .expect("the turn was still waited for after ten seconds, past any bound")
    }

    #[test]
    fn a_turn_let_go_of_mid_wait_is_taken_then_rather_than_at_the_bound() {
        let root = root("let-go");
        let node = root.join("node");
        fs::write(&node, "").expect("a stand-in node is made");
        let holder = File::open(&node).expect("the stand-in node opens");
        assert!(
            locked(&holder, LOCK_EX | LOCK_NB),
            "another litra's turn wasn't taken"
        );
        let light = stand_in(&node, Duration::from_secs(5));
        let held = Duration::from_millis(200);
        let letting_go = std::thread::spawn(move || {
            // Another litra's exchange, held this long before it lets go.
            std::thread::sleep(held);
            locked(&holder, LOCK_UN)
        });
        let (taken, waited) = within_ten_seconds(move || {
            let started = Instant::now();
            let taken = light.take_turn().map_err(|error| error.kind());
            (taken, started.elapsed())
        });
        let let_go = letting_go.join().expect("the holder lets go");
        assert_eq!(
            (
                let_go,
                taken,
                waited >= held,
                waited < Duration::from_secs(5)
            ),
            (true, Ok(()), true, true),
            "a turn let go of mid-wait wasn't taken then ({waited:?})"
        );
    }

    #[test]
    fn a_run_of_one_litra_s_sends_leaves_the_turn_free_for_another() {
        let root = root("run");
        let node = root.join("node");
        fs::write(&node, "").expect("a stand-in node is made");
        let mut sender = stand_in(&node, Duration::from_secs(5));
        let waiter = stand_in(&node, Duration::from_millis(500));
        let (done, stop) = std::sync::mpsc::channel::<()>();
        let sending = std::thread::spawn(move || {
            let mut sends = 0;
            let running = || matches!(stop.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));
            while running() && sends < 200 {
                sender.take_turn().expect("the sender takes its turn");
                // An exchange's send and reply, as long as a quick one.
                std::thread::sleep(Duration::from_millis(20));
                sender.let_go_of_turn();
                sends += 1;
            }
        });
        let taken = within_ten_seconds(move || {
            let mut waiter = waiter;
            let taken = waiter.take_turn().map_err(|error| error.kind());
            waiter.let_go_of_turn();
            taken
        });
        drop(done);
        sending.join().expect("the sender stops");
        assert_eq!(
            taken,
            Ok(()),
            "a run of one litra's sends kept another out for the whole bound"
        );
    }

    #[test]
    fn a_light_is_tried_again_ever_later_up_to_a_minute() {
        let waits: Vec<_> =
            std::iter::successors(Some(next_retry(None)), |wait| Some(next_retry(Some(*wait))))
                .take(8)
                .map(|wait| wait.as_secs())
                .collect();
        assert_eq!(
            (waits, until(None)),
            (vec![1, 2, 4, 8, 16, 32, 60, 60], -1),
            "a light was tried again other than at twice the last wait up to a minute, \
             or poll was given a bound with no try due"
        );
    }

    #[test]
    fn a_report_is_twenty_bytes_behind_its_header() {
        let made = report(POWER, &[1]);
        assert_eq!(made[..5], [0x11, 0xff, 0x04, 0x1c, 1]);
        assert!(
            made[5..].iter().all(|byte| *byte == 0),
            "a report's padding wasn't zero: {made:?}"
        );
        assert_eq!(
            report(TEMPERATURE, &4000u16.to_be_bytes())[4..6],
            [0x0f, 0xa0]
        );
    }

    #[test]
    fn an_answer_is_taken_only_for_its_question() {
        let reply = [0x11, 0xff, 0x04, 0x31, 0x00, 0x3b, 0, 0];
        assert_eq!(answer(&reply, BRIGHTNESS_IS), Some(&reply[4..]));
        assert_eq!(answer(&reply, POWER_IS), None);
        assert_eq!(answer(&[0x11, 0xff], POWER_IS), None);
    }

    #[test]
    fn brightness_maps_both_ways_over_the_lights_range() {
        assert_eq!(units(0), DIMMEST);
        assert_eq!(units(100), BRIGHTEST);
        assert_eq!(units(250), BRIGHTEST);
        assert_eq!(units(50), 135);
        for share in 0..=100 {
            assert_eq!(percent(units(share)), share);
        }
    }

    /// A report as the light sends it: its header, a code, then values.
    fn from_light(code: u8, values: &[u8]) -> Vec<u8> {
        let mut sent = vec![0x11, 0xff, 0x04, code];
        sent.extend_from_slice(values);
        sent.resize(REPORT, 0);
        sent
    }

    #[test]
    fn the_lights_buttons_say_their_new_values() {
        let mut state = State::default();
        // As a Glow sent them: power on, brightness 84, 4500 K.
        assert_eq!(hear(&mut state, &from_light(0x00, &[1])), Heard::Changed);
        assert_eq!(
            hear(&mut state, &from_light(0x10, &[0x00, 0x54])),
            Heard::Changed
        );
        assert_eq!(
            hear(&mut state, &from_light(0x20, &[0x11, 0x94])),
            Heard::Changed
        );
        assert_eq!(
            state,
            State {
                on: Some(true),
                units: Some(84),
                kelvin: Some(4500),
            }
        );
        assert_eq!(
            hear(&mut state, &from_light(0x20, &[0x11, 0x94])),
            Heard::Nothing
        );
        assert_eq!(hear(&mut state, &from_light(0x00, &[0])), Heard::Changed);
        assert_eq!(state.on, Some(false));
    }

    #[test]
    fn a_command_heard_from_anybody_is_asked_about() {
        let mut state = State::default();
        assert_eq!(
            hear(&mut state, &from_light(POWER, &[])),
            Heard::Ask(POWER_IS)
        );
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS, &[])),
            Heard::Ask(BRIGHTNESS_IS)
        );
        assert_eq!(
            hear(&mut state, &from_light(TEMPERATURE, &[])),
            Heard::Ask(TEMPERATURE_IS)
        );
        assert_eq!(state, State::default());
    }

    #[test]
    fn an_answer_to_anybodys_question_is_taken_and_never_asked_about() {
        let mut state = State::default();
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS_IS, &[0x00, 0x3d])),
            Heard::Changed
        );
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS_IS, &[0x00, 0x3d])),
            Heard::Nothing
        );
        assert_eq!(
            hear(&mut state, &from_light(TEMPERATURE_IS, &[0x13, 0xec])),
            Heard::Changed
        );
        assert_eq!(state.units, Some(61));
        assert_eq!(state.kelvin, Some(5100));
        assert_eq!(hear(&mut state, &[0x11, 0xff]), Heard::Nothing);
        assert_eq!(hear(&mut state, &from_light(0x7f, &[1])), Heard::Nothing);
    }

    #[test]
    fn warmth_is_held_to_what_the_light_makes() {
        assert_eq!(kelvin(1000), WARMEST);
        assert_eq!(kelvin(9000), COOLEST);
        assert_eq!(kelvin(4049), 4000);
        assert_eq!(kelvin(4050), 4100);
    }
}
