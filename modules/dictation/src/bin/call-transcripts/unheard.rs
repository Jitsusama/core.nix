//! What of a call the speech model couldn't hear, said in a note under
//! its transcript's heading, so a file left without those stretches never
//! reads as whole: not once the call's notice is gone, nor to whatever
//! sums it up, nor after a restart, which reads the note back.

use dictation::mel::RATE;
use dictation::transcript;

/// How the note opens, by which it's read back.
const OPENS: &str = "Part of the call wasn't heard: the speech model failed on ";

/// How the note opens while no line of the call has been heard.
const OPENS_NONE: &str = "None of the call was heard: the speech model failed on ";

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
            stretches: unheard.stretches.saturating_add(1),
            first: unheard.first.min(start),
        },
    }
}

impl Unheard {
    /// The note the transcript of a call none of which was heard says it
    /// with.
    pub fn note_with_none_heard(self) -> String {
        self.said(OPENS_NONE)
    }

    /// The note the transcript says it with.
    pub fn note(self) -> String {
        self.said(OPENS)
    }

    fn said(self, opens: &str) -> String {
        let at = transcript::clock(self.first);
        match self.stretches {
            1 => format!("{opens}a stretch of it, at {at}."),
            stretches => format!("{opens}{stretches} stretches of it, the first at {at}."),
        }
    }

    /// What a transcript's note says wasn't heard, read back; none when it
    /// has no such note.
    pub fn read(said: &str) -> Option<Self> {
        said.split("\n\n").find_map(|paragraph| {
            let paragraph = paragraph.trim();
            let rest = [OPENS, OPENS_NONE]
                .iter()
                .find_map(|opens| paragraph.strip_prefix(opens))?;
            let (stretches, at) = if let Some(at) = rest.strip_prefix("a stretch of it, at ") {
                (1, at)
            } else {
                let (stretches, at) = rest.split_once(" stretches of it, the first at ")?;
                // A file is anyone's to edit; no note is written with
                // fewer than two.
                let stretches = stretches.parse().ok().filter(|&stretches| stretches > 1)?;
                (stretches, at)
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
        // The note is read back across a restart, by whichever build
        // then runs, so its words are pinned here as written.
        let written = "# Transcript\n\nPart of the call wasn't heard: the speech model failed \
            on 3 stretches of it, the first at 12:30.\n\n**Joel**, 0:00: Hi.";
        let three = Unheard {
            stretches: 3,
            first: 12 * MINUTE + 30 * RATE,
        };
        assert_eq!(Unheard::read(written), Some(three), "{written}");
        assert_eq!(
            Unheard::read("# Transcript\n\n**Joel**, 0:00: Part of the call wasn't heard."),
            None,
            "a line saying so was read as the note"
        );
    }

    #[test]
    fn a_call_none_of_which_was_heard_says_so() {
        let unheard = and(Some(and(None, 2 * RATE)), 14 * MINUTE);
        let note = unheard.note_with_none_heard();
        assert_eq!(
            note,
            "None of the call was heard: the speech model failed on 2 stretches of it, the \
             first at 0:02.",
            "a call none of which was heard was said to be heard in part"
        );
        let said = format!("# Transcript\n\n{note}");
        assert_eq!(Unheard::read(&said), Some(unheard), "{said}");
    }

    #[test]
    fn a_note_too_far_in_to_be_a_call_is_left_unread() {
        // A file is anyone's to edit, so a clock past what a sample
        // count holds reads as no note rather than overflowing.
        for at in ["307445734561825861:00", "5124095576030431:00:00"] {
            let said = format!("{OPENS}a stretch of it, at {at}.");
            assert_eq!(Unheard::read(&said), None, "{said}");
        }
        // Nor does a count no note is written with read as one.
        for stretches in ["0", "1"] {
            let said = format!("{OPENS}{stretches} stretches of it, the first at 0:02.");
            assert_eq!(Unheard::read(&said), None, "{said}");
        }
        // And a count as high as one can be stays there with one more.
        let most = format!("{OPENS}{} stretches of it, the first at 0:02.", usize::MAX);
        let read = Unheard::read(&most);
        assert_eq!(and(read, 0).stretches, usize::MAX, "{most}");
    }
}
