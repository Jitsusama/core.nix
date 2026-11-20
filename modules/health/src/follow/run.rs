//! Health's run over its followers' lines: the half-minute tick that
//! looks at the lid and writes the day, and when the run ends.

use std::io;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::spell::{self, Spell};
use super::{EVERY, Follower, SESSIONS_A_TICK, SETTLED};
use crate::lid::Looked;

/// How long a wait for a line goes before `run` looks for a stop again:
/// the day's replay holds senders a stop doesn't end, as its histories
/// aren't followers, and its merge may send nothing for a while, so a
/// wait for its next line could outlast the stop's `WRITES_WITHIN`.
const STOP_HEARD_WITHIN: Duration = Duration::from_millis(500);

/// Take the followers' lines until one has ended, which its `None` says,
/// and health has run `SETTLED` by `now`, the awake clock and the wall's,
/// in microseconds, and every half minute, however busy the journals are,
/// look at the lid and write the day. Each follows for as long as health
/// runs, so an end is a fault, and health starting again follows every one
/// again.
///
/// Health ends no sooner than `SETTLED` from its start, however its
/// followers end, so the day isn't read again every few seconds where
/// they end at every start: until then whatever is still heard is taken,
/// and once nothing can be, health `wait`s out the rest, still looking
/// every half minute. What went unheard that day is heard from the day's
/// replay once health starts again. Once `stopped`, as by a signal, it
/// ends at its next look at the clock, however long it has run, writing
/// the day and telling what a spell kept unsaid, as any end does; but
/// before its first tick it writes only the days it holds, since the day's
/// replay may be under way, and its day so far would be written over the
/// whole one the last run kept, which the next start replays anyway. It
/// waits for a line no longer than `STOP_HEARD_WITHIN` at a time, so a
/// stop is heard however quiet the journals are.
///
/// A day that can't be written, as on a full disk, is said through `say`,
/// and once written again through `notice`, as `Spell` decides. Health
/// tries every half minute for as long as `GIVES_UP`, as a start frees no
/// room and, past midnight, would lose the old day; a new day starts as
/// ever meanwhile, holding the old, so what's heard and seen goes into it.
/// Then it tries once more, and carries on if that writes everything.
///
/// # Errors
///
/// When the day can't be written as health ends, or on that last try.
#[expect(clippy::too_many_arguments, reason = "each is a seam its tests drive")]
pub(super) fn run(
    follower: &mut Follower,
    read: &Receiver<Option<String>>,
    mut now: impl FnMut() -> (Instant, i64),
    mut wait: impl FnMut(Duration),
    mut look: impl FnMut() -> Looked,
    stopped: impl Fn() -> bool,
    say: &dyn Fn(&str),
    notice: &dyn Fn(&str),
) -> io::Result<()> {
    let (started, _) = now();
    let mut ticked = started;
    let mut ended = false;
    let mut hearing = true;
    let mut spell = Spell::default();
    loop {
        let (at, wall) = now();
        let ran = at.saturating_duration_since(started);
        if spell.gave_up(at) && ran >= SETTLED {
            spell.last_try(follower, at, wall, say, notice)?;
        }
        if stopped() || (ended && !spell.is_on() && ran >= SETTLED) {
            break;
        }
        let left = EVERY.saturating_sub(at.saturating_duration_since(ticked));
        let mut refused = None;
        if hearing {
            match read.recv_timeout(left.min(STOP_HEARD_WITHIN)) {
                // Only a new day's line writes, and only a write that
                // failed says so, so a line taken without one leaves the
                // spell as it was.
                Ok(Some(line)) => refused = follower.take(&line).err(),
                Ok(None) => ended = true,
                Err(RecvTimeoutError::Timeout) => {}
                // Every sender is gone, so nothing more can be heard.
                Err(RecvTimeoutError::Disconnected) => {
                    hearing = false;
                    ended = true;
                }
            }
        } else {
            // Past `SETTLED`, only a day still unwritten keeps health
            // here, and the next try is the next tick's, so it never
            // waits nothing and spins.
            let settles = SETTLED.saturating_sub(ran);
            wait(if settles.is_zero() {
                left
            } else {
                left.min(settles)
            });
        }
        // Dated once the line is taken, as the wait for it may be long.
        let (at, wall) = now();
        if let Some(error) = refused {
            spell.tried(Some(&error), at, wall, &follower.into, say, notice);
        }
        if at.saturating_duration_since(ticked) >= EVERY {
            ticked = at;
            follower.look_at_lid(look(), wall);
            follower.mind(wall);
            follower.count_the_dead(wall);
            follower.weigh(wall);
            follower.guard(wall);
            follower.hear_peers(wall);
            follower.read_reports();
            follower.feel();
            follower.time_typing();
            follower.pool_keys();
            follower.hear(SESSIONS_A_TICK);
            let failed = follower.write().err();
            spell.tried(failed.as_ref(), at, wall, &follower.into, say, notice);
        }
    }
    if stopped() && ticked == started {
        // Only the days held are written, as the day so far may be only
        // the part of it the replay has sent yet.
        follower.changed = false;
    }
    let written = spell.ending(follower.write(), &follower.into, notice);
    let lost = spell::losing(&follower.held);
    written.map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("the day couldn't be written as health ended{lost}: {error}"),
        )
    })
}
