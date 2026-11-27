//! Whether a look's choice poses as one of the launcher's own acts on a
//! look, as the kit's ask.ts `poses` reads one, and Now.qml's lookOffers:
//! its label read as it draws, with what draws nothing dropped, what draws
//! as a space read as one, and what turns the letters after it around
//! refused whole. A choice that poses is offered neither in For You nor on
//! a page, so the server takes no answer of it either. The table of looks
//! both sides are tested against (kit/tests/offered.json) holds the two to
//! one reading.

/// The launcher's acts on a look, as ask.ts's `LAUNCHER_ACTS`.
const LAUNCHER_ACTS: [&str; 6] = ["Open", "Answer", "Clear It", "Go To", "Explain It", "Talk"];

/// Whether `label` would be read as one of the launcher's acts.
pub fn poses(label: &str) -> bool {
    let read = read_as(label);
    label.chars().any(turns) || LAUNCHER_ACTS.iter().any(|act| read_as(act) == read)
}

/// `name` as it reads on a button, as ask.ts's `readAs`: what draws
/// nothing dropped, what draws as a space and every run of spaces read as
/// one, a sentence's ending stops dropped, trimmed, and in lower case.
fn read_as(name: &str) -> String {
    let mut read = String::new();
    for drawn in name.chars().filter(|each| !unseen(*each)) {
        if drawn.is_whitespace() || filler(drawn) {
            if !read.ends_with(' ') {
                read.push(' ');
            }
        } else {
            read.push(drawn);
        }
    }
    read.trim_end_matches(|end: char| end.is_whitespace() || ".!?,;:\u{2026}".contains(end))
        .trim()
        .to_lowercase()
}

/// What draws nothing, as ask.ts's `UNDRAWN`, `UNSEEN` and `UNMARKED`.
fn unseen(each: char) -> bool {
    matches!(
        each,
        '\u{17b4}'
            | '\u{17b5}'
            | '\u{180b}'..='\u{180d}'
            | '\u{180f}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{1bca0}'..='\u{1bca3}'
            | '\u{1d173}'..='\u{1d17a}'
            | '\u{e0000}'..='\u{e0fff}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff0}'..='\u{fff8}'
            | '\u{00ad}'
            | '\u{061c}'
            | '\u{180e}'
            | '\u{034f}'
    )
}

/// What draws as a space, as ask.ts's `FILLERS`.
const fn filler(each: char) -> bool {
    matches!(
        each,
        '\u{115f}' | '\u{1160}' | '\u{2800}' | '\u{3164}' | '\u{ffa0}'
    )
}

/// What turns the letters after it around, as ask.ts's `TURNS`.
const fn turns(each: char) -> bool {
    matches!(
        each,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_reading_as_a_launchers_act_poses_however_its_written() {
        for label in [
            "Clear It",
            "clear it.",
            "Clear\u{200b} It",
            "Clear\u{3164}It",
            " Go  To!",
            "\u{202e}tI raelC",
            "Talk\u{fe0f}",
        ] {
            assert!(poses(label), "{label:?} passed for its own");
        }
        for label in ["Merge It", "Clear Them", "Opened", ""] {
            assert!(!poses(label), "{label:?} was taken for an act");
        }
    }
}
