//! What a call's words amount to. A take that's only what Whisper writes
//! for a click or a breath is no turn; a call of greetings and little else
//! is too little to sum up, so no model is asked to make something of it;
//! and a summary naming what the call never named isn't kept, since a
//! wrong summary is worse than none.

use std::collections::HashSet;

use dictation::transcript;

/// What a call with too little said gets in place of a summary.
pub const TOO_LITTLE: &str = "Too little was said to sum up.";

/// Fewer words than this, each counted once and greetings and thanks
/// left out, is too little to sum up. A question and its answer, the
/// shortest exchange worth a summary, say more.
const LEAST_WORDS: usize = 6;

/// Words that carry nothing a summary could use: greetings, thanks,
/// short answers, and what Whisper writes for a click or a breath
/// ("You", "Thank you.").
const FILLER: [&str; 22] = [
    "bye", "cool", "go", "great", "hello", "hey", "hi", "hmm", "huh", "mm", "no", "oh", "ok",
    "okay", "so", "thank", "thanks", "uh", "um", "wow", "yeah", "you",
];

/// What a take's words say, without what Whisper writes of the sound
/// itself in brackets ("[static]", "(wind blowing)"); or nothing when
/// that leaves only filler, which is what Whisper writes for sound that
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

/// The names `summary` gives that `transcript` never says, each once, in
/// the order the summary gives them. A name is a word written with a
/// capital where a sentence or a list's line doesn't start.
pub fn strays(summary: &str, transcript: &str) -> Vec<String> {
    let known: HashSet<String> = words(transcript).collect();
    let mut strays: Vec<String> = Vec::new();
    for line in summary.lines() {
        let mut starts = true;
        for word in line.split_whitespace() {
            let bare = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .trim_end_matches("'s")
                .trim_end_matches("\u{2019}s");
            // A word opening a bracket, as "(All of them)", may be
            // capitalised as a sentence's first word is.
            let named = !starts
                && !word.starts_with(['(', '[', '"'])
                && bare != "I"
                && !bare.starts_with("I'")
                && bare.chars().any(char::is_uppercase);
            if named && !known.contains(&bare.to_lowercase()) && !strays.iter().any(|s| s == bare) {
                strays.push(bare.to_owned());
            }
            // A sentence's end inside markdown's marks, as "**Decided:**"
            // or "(at ten).", still ends it, and a dash or a bullet on
            // its own starts what follows.
            starts = bare.is_empty()
                || word
                    .trim_end_matches(['*', '_', ')', '"', '\''])
                    .ends_with(['.', '!', '?', ':', ';']);
        }
    }
    strays
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

    /// That call's summary, which the model made up from little more.
    const MADE_UP: &str = "The call was primarily about Quorvex, a company or project \
         associated with Binks and Tallimander. The group discussed:\n\n\
         - The positive and negative aspects of Quorvex\n\
         - Joel's gratitude for the discussion and his inquiries\n\
         - Joel's greeting and introduction, followed by the group's acknowledgment\n";

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

    #[test]
    fn a_summary_naming_what_the_call_never_did_strays() {
        assert_eq!(strays(MADE_UP, HELLOS), ["Quorvex", "Tallimander"]);
    }

    #[test]
    fn a_summary_naming_only_what_the_call_did_doesnt_stray() {
        let summary = "The call moved the review to Thursday.\n\n\
             - Others said Thursday morning works for everyone.\n\
             - Me thanked them.\n";
        assert!(strays(summary, SHORT).is_empty());
    }

    #[test]
    fn a_name_is_known_however_its_written() {
        let summary = "They agreed on THURSDAY, and Joel's review moves.";
        assert!(strays(summary, "**Joel**, 0:00: The review is thursday.").is_empty());
    }

    #[test]
    fn a_sentence_starts_after_markdowns_marks() {
        let summary = "**Decided:** All agreed on Thursday (morning). Everyone's in - \
             Others too (Each said so).";
        assert!(strays(summary, SHORT).is_empty());
    }
}
