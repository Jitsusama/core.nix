//! What of a call the speech model couldn't hear, said in a note under
//! its transcript's heading, so a file left without those stretches never
//! reads as whole: not once the call's notice is gone, nor to whatever
//! sums it up, nor after a restart, which reads the note back.

use dictation::mel::RATE;
use dictation::transcript;

/// How the note opens, by which it's read back.
const OPENS: &str = "Part of the call wasn't heard: the speech model failed on ";

/// The stretches of a call not heard: how many, and the sample of the
/// call the first starts at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unheard {
    pub stretches: usize,
    pub first: usize,
}

/// What's not heard of a call with one more stretch, `start` samples in.
pub fn and(unheard: Option<Unheard>, start: usize) -> Unheard {
    match unheard {
        None => Unheard {
            stretches: 1,
            first: start,
        },
        Some(unheard) => Unheard {
            stretches: unheard.stretches + 1,
            first: unheard.first.min(start),
        },
    }
}

impl Unheard {
    /// The note the transcript says it with.
    pub fn note(self) -> String {
        let at = transcript::clock(self.first);
        match self.stretches {
            1 => format!("{OPENS}a stretch of it, at {at}."),
            stretches => format!("{OPENS}{stretches} stretches of it, the first at {at}."),
        }
    }

    /// What a transcript's note says wasn't heard, read back; none when it
    /// has no such note.
    pub fn read(said: &str) -> Option<Self> {
        said.split("\n\n").find_map(|paragraph| {
            let rest = paragraph.trim().strip_prefix(OPENS)?;
            let (stretches, at) = if let Some(at) = rest.strip_prefix("a stretch of it, at ") {
                (1, at)
            } else {
                let (stretches, at) = rest.split_once(" stretches of it, the first at ")?;
                (stretches.parse().ok()?, at)
            };
            let seconds = at
                .strip_suffix('.')?
                .split(':')
                .try_fold(0, |sum: usize, part| {
                    sum.checked_mul(60)?.checked_add(part.parse().ok()?)
                })?;
            Some(Self {
                stretches,
                first: seconds.checked_mul(RATE)?,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: usize = 60 * RATE;

    #[test]
    fn each_stretch_not_heard_is_counted_from_the_first() {
        let one = and(None, 12 * MINUTE);
        assert_eq!(one.note(), format!("{OPENS}a stretch of it, at 12:00."));
        let three = and(Some(and(Some(one), 14 * MINUTE)), 3 * MINUTE);
        assert_eq!(
            three.note(),
            format!("{OPENS}3 stretches of it, the first at 3:00."),
            "the stretches weren't counted from the first"
        );
    }

    #[test]
    fn a_note_of_what_wasnt_heard_is_read_back() {
        for unheard in [
            and(None, 75 * MINUTE + 4 * RATE),
            Unheard {
                stretches: 37,
                first: 12 * MINUTE + 30 * RATE,
            },
        ] {
            let said = format!("# Transcript\n\n{}\n\n**Joel**, 0:00: Hi.", unheard.note());
            assert_eq!(Unheard::read(&said), Some(unheard), "{said}");
        }
        assert_eq!(
            Unheard::read("# Transcript\n\n**Joel**, 0:00: Part of the call wasn't heard."),
            None,
            "a line saying so was read as the note"
        );
    }

    #[test]
    fn a_note_too_far_in_to_be_a_call_is_left_unread() {
        // A file is anyone's to edit, so a clock past what a sample
        // count holds reads as no note rather than overflowing.
        for at in ["307445734561825861:00", "5124095576030431:00:00"] {
            let said = format!("{OPENS}a stretch of it, at {at}.");
            assert_eq!(Unheard::read(&said), None, "{said}");
        }
    }
}
