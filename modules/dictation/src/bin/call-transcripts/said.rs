//! What a call's words amount to. A take is what was said in it, without
//! what a speech model writes of the sound itself, and a lone "No." or
//! "Okay." is an answer kept like any other, though a lone "Mm." is a
//! hesitation and no turn; a call of greetings and little else is too
//! little to sum up, so no model is asked to make something of it.

use std::collections::HashSet;

use dictation::{transcript, wer};

/// What a call with too little said gets in place of a summary.
pub const TOO_LITTLE: &str = "Too little was said to sum up.";

/// What a call gets in place of a summary when too little of it was
/// heard, the speech model having failed on the rest, so it isn't taken
/// for a call where little was said.
pub const TOO_LITTLE_HEARD: &str =
    "Too little was heard to sum up: the speech model failed on part of the call.";

/// Fewer words than this, each counted once and greetings and thanks
/// left out, is too little to sum up. A question and its answer, the
/// shortest exchange worth a summary, say more.
const LEAST_WORDS: usize = 6;

/// Words that carry nothing a summary could use: greetings, thanks,
/// short answers, and what Whisper wrote for a click or a breath ("You",
/// "Thank you."). They're kept in a transcript; only a call's summary
/// leaves them out.
const FILLER: [&str; 22] = [
    "bye", "cool", "go", "great", "hello", "hey", "hi", "hmm", "huh", "mm", "no", "oh", "ok",
    "okay", "so", "thank", "thanks", "uh", "um", "wow", "yeah", "you",
];

/// What a take's words say, without what a speech model writes of the
/// sound itself in brackets ("[static]", "(wind blowing)"); or nothing
/// when that leaves no word but hesitations ("Mm.", "Uh, um."), which the
/// meetings' score leaves out too. A take of filler alone is kept: a
/// take is only cut where a voice is heard (`takes`), so a breath that
/// Parakeet would hear as "Yeah." never reaches it, and a lone "Yeah."
/// that does was said; dropping it cost meetings' transcripts their
/// short answers.
pub fn spoken(text: &str) -> Option<String> {
    let mut said = String::new();
    let mut depth = 0_usize;
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => said.push(c),
            _ => {}
        }
    }
    let said = said.split_whitespace().collect::<Vec<_>>().join(" ");
    let any = !wer::words(&said).is_empty();
    any.then_some(said)
}

/// Whether too little was said in `transcript` to sum it up.
pub fn too_little(transcript: &str) -> bool {
    // Only what was said counts here, not whose it was, so no name is
    // Joel's.
    let said: HashSet<String> = transcript::read(transcript, "")
        .iter()
        .flat_map(|line| words(&line.text).collect::<Vec<_>>())
        .filter(|word| !FILLER.contains(&word.as_str()))
        .collect();
    said.len() < LEAST_WORDS
}

/// What `transcript` gets in place of a summary, or none when enough of
/// it was said to sum it up.
pub fn in_place_of_a_summary(transcript: &str) -> Option<&'static str> {
    too_little(transcript).then_some(TOO_LITTLE)
}

/// `text`'s words, lowercase, without what's around them or a
/// possessive's ending, so "Joel's" is Joel.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .map(|word| {
            word.trim_matches('\'')
                .trim_end_matches("'s")
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exchange the call checks and VM test hold: short, and worth a
    /// summary.
    const SHORT: &str = "# Transcript\n\n**Me**, 0:00: Can we move the review to Thursday?\n\n\
         **Others**, 0:04: Thursday morning works for everyone.\n\n\
         **Me**, 0:09: Perfect. Thank you so much.\n";

    /// A call of greetings and a word Whisper made up from a sound, as
    /// a real one on 31 October came out once the words Whisper had been
    /// told were taken from it.
    const HELLOS: &str = "# Transcript\n\n**Joel**, 0:01: Okay. Hello? Hello? Hello.\n\n\
         **Others**, 0:43: Binks.\n\n**Joel**, 0:46: Hello, hello, hello. Hello? Thank you.\n\n\
         **Others**, 8:15: Hello. Hello. Hello. Hello.\n\n**Others**, 9:15: Thank you. Go. Hi.\n\n\
         **Others**, 10:54: You\n";

    #[test]
    fn a_take_of_sounds_alone_is_no_turn() {
        assert_eq!(spoken(" "), None);
        assert_eq!(spoken("[static]"), None);
        assert_eq!(spoken("(wind blowing) [click] ."), None);
    }

    #[test]
    fn a_take_of_hesitations_alone_is_no_turn() {
        assert_eq!(spoken("Mm."), None);
        assert_eq!(spoken("Uh, um."), None);
        assert_eq!(spoken("Mm-hmm."), None);
        assert_eq!(spoken("Um, no.").as_deref(), Some("Um, no."));
    }

    #[test]
    fn a_short_answer_of_filler_alone_is_a_turn() {
        assert_eq!(spoken("No.").as_deref(), Some("No."));
        assert_eq!(spoken("Okay, yeah.").as_deref(), Some("Okay, yeah."));
        assert_eq!(spoken("(laughs) Thank you.").as_deref(), Some("Thank you."));
    }

    #[test]
    fn speech_keeps_its_words_but_not_the_sounds_around_them() {
        assert_eq!(
            spoken("Perfect. Thank you so much.").as_deref(),
            Some("Perfect. Thank you so much.")
        );
        assert_eq!(
            spoken("[static] Can we move the review to Thursday?").as_deref(),
            Some("Can we move the review to Thursday?")
        );
    }

    #[test]
    fn a_question_and_its_answer_are_enough_to_sum_up() {
        assert!(!too_little(SHORT));
    }

    #[test]
    fn greetings_and_a_stray_word_are_too_little_to_sum_up() {
        assert!(too_little(HELLOS));
    }

    #[test]
    fn nothing_written_down_is_too_little_to_sum_up() {
        assert!(too_little("# Transcript\n"));
    }

    #[test]
    fn a_call_mostly_not_heard_says_so_in_place_of_a_summary() {
        assert_eq!(in_place_of_a_summary(SHORT), None);
        assert_eq!(in_place_of_a_summary(HELLOS), Some(TOO_LITTLE));
        let note = crate::unheard::and(None, 0).note();
        let unheard = format!("{note}\n\n{HELLOS}");
        assert_eq!(
            in_place_of_a_summary(&unheard),
            Some(TOO_LITTLE_HEARD),
            "a call the speech model failed on read as one where little was said"
        );
    }
}
