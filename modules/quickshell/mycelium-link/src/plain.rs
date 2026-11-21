//! A sprout link's words as one plain line, the letters any page may write
//! that Joel can't see or that move the launcher's line left out, as
//! launcher/Sprouting.js's `offered` reads them again.

/// `words` as one plain line: each run of spaces, tabs and line ends one
/// space, any other control, format or blank character gone, so nothing in
/// them can move the launcher's line, turn its text around, hide in it or
/// speak to a terminal, and trimmed.
pub fn plain(words: &str) -> String {
    words
        .split(char::is_whitespace)
        .map(|word| {
            word.chars()
                .filter(|&letter| !letter.is_control() && !formatting(letter) && !blank(letter))
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Unicode's format characters (general category Cf), which draw nothing
/// yet can turn text around (U+202E), hide in it (U+200B) or join it.
const FORMATTING: [(char, char); 21] = [
    ('\u{00AD}', '\u{00AD}'),
    ('\u{0600}', '\u{0605}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{06DD}', '\u{06DD}'),
    ('\u{070F}', '\u{070F}'),
    ('\u{0890}', '\u{0891}'),
    ('\u{08E2}', '\u{08E2}'),
    ('\u{180E}', '\u{180E}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFF9}', '\u{FFFB}'),
    ('\u{110BD}', '\u{110BD}'),
    ('\u{110CD}', '\u{110CD}'),
    ('\u{13430}', '\u{1343F}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0001}', '\u{E0001}'),
    ('\u{E0020}', '\u{E007F}'),
];

/// Whether `letter` is one of Unicode's format characters.
fn formatting(letter: char) -> bool {
    FORMATTING
        .iter()
        .any(|&(first, last)| (first..=last).contains(&letter))
}

/// What draws nothing or a blank yet isn't a format character: Unicode's
/// other default ignorables (Hangul fillers, variation selectors and the
/// rest) and the braille blank, U+2800, which the shell draws full width.
/// Kept, a run of them pushes a page's words out of sight between the
/// start Joel reads and the end his Return waits for.
const BLANK: [(char, char); 14] = [
    ('\u{034F}', '\u{034F}'),
    ('\u{115F}', '\u{1160}'),
    ('\u{17B4}', '\u{17B5}'),
    ('\u{180B}', '\u{180D}'),
    ('\u{180F}', '\u{180F}'),
    ('\u{2065}', '\u{2065}'),
    ('\u{2800}', '\u{2800}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{FE00}', '\u{FE0F}'),
    ('\u{FFA0}', '\u{FFA0}'),
    ('\u{FFF0}', '\u{FFF8}'),
    ('\u{E0000}', '\u{E0000}'),
    ('\u{E0002}', '\u{E001F}'),
    ('\u{E0080}', '\u{E0FFF}'),
];

/// Whether `letter` draws nothing or a blank without being a format
/// character.
fn blank(letter: char) -> bool {
    BLANK
        .iter()
        .any(|&(first, last)| (first..=last).contains(&letter))
}
