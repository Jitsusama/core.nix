//! What a call's words amount to. A take that's only what a speech model
//! writes for a click or a breath is no turn; a call of greetings and
//! little else is too little to sum up, so no model is asked to make
//! something of it.

use std::collections::HashSet;

use dictation::transcript;

/// What a call with too little said gets in place of a summary.
pub const TOO_LITTLE: &str = "Too little was said to sum up.";

/// Fewer words than this, each counted once and greetings and thanks
/// left out, is too little to sum up. A question and its answer, the
/// shortest exchange worth a summary, say more.
const LEAST_WORDS: usize = 6;

/// Words that carry nothing a summary could use: greetings, thanks,
/// short answers, and what a speech model writes for a click or a breath
/// (Whisper's "You", "Thank you.").
const FILLER: [&str; 22] = [
    "bye", "cool", "go", "great", "hello", "hey", "hi", "hmm", "huh", "mm", "no", "oh", "ok",
    "okay", "so", "thank", "thanks", "uh", "um", "wow", "yeah", "you",
];

/// What a take's words say, without what a speech model writes of the
/// sound itself in brackets ("[static]", "(wind blowing)"); or nothing
/// when that leaves only filler, which is what one writes for sound that
/// holds no speech.
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
    let speech = words(&said).any(|word| !FILLER.contains(&word.as_str()));
    speech.then_some(said)
}

/// Whether too little was said in `transcript` to sum it up.
pub fn too_little(transcript: &str) -> bool {
    let said: HashSet<String> = transcript::read(transcript)
        .iter()
        .flat_map(|line| words(&line.text).collect::<Vec<_>>())
        .filter(|word| !FILLER.contains(&word.as_str()))
        .collect();
    said.len() < LEAST_WORDS
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
    fn what_whisper_writes_for_a_sound_is_no_turn() {
        assert_eq!(spoken("You"), None);
        assert_eq!(spoken("Thank you."), None);
        assert_eq!(spoken(" "), None);
        assert_eq!(spoken("[static]"), None);
        assert_eq!(spoken("(wind blowing) You"), None);
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
}
