//! Text an owner or a node wrote, made safe to show: what the system says
//! goes to a terminal, the launcher's rows and the realm's other machines,
//! so nothing in it may act on a terminal, or move or hide the words around
//! it. One filter for every such text, so a rule learnt for one reader
//! holds for all of them, and beside it the terminal kit's own list of
//! what draws nothing (terminal-kit's `one_line`), so nothing the kit keeps
//! from a terminal reaches one through here.

use std::iter::Peekable;
use std::str::Chars;

use terminal_kit::layout::one_line;

/// `line` as plain words: a terminal's escape sequences gone whole, as a
/// coloured or linked complaint writes them; every other control character
/// and each line or paragraph separator one space, however many stand
/// together, so the words either side of a tab or a return stay apart; and
/// the format characters that turn, hide or break text gone, so a row reads
/// as it's written.
pub fn words(line: &str) -> String {
    let mut kept = String::new();
    let mut chars = line.chars().peekable();
    while let Some(each) = chars.next() {
        match each {
            '\u{1b}' => escape(&mut chars),
            // The one-character forms of a control sequence and a string,
            // as characters; written as raw bytes, they were never UTF-8
            // and reach here as a replacement character, which is harmless.
            '\u{9b}' => control_sequence(&mut chars),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => string(&mut chars),
            each if each.is_control() || matches!(each, '\u{2028}' | '\u{2029}') => {
                if !kept.is_empty() && !kept.ends_with(' ') {
                    kept.push(' ');
                }
            }
            each if hides(each) => {}
            each => kept.push(each),
        }
    }
    one_line(&kept).trim().to_owned()
}

/// Passes over an escape sequence's rest, its escape already taken: a
/// control sequence such as a colour, a string such as a title or a link,
/// or one that runs through intermediates to its final character, such as
/// a character set chosen; any other takes the one character after it.
fn escape(chars: &mut Peekable<Chars>) {
    match chars.next() {
        Some('[') => control_sequence(chars),
        Some(']' | 'P' | 'X' | '^' | '_') => string(chars),
        Some(' '..='/') => {
            chars.find(|end| ('0'..='~').contains(end));
        }
        _ => {}
    }
}

/// Passes over a control sequence's rest, to its final character.
fn control_sequence(chars: &mut Peekable<Chars>) {
    chars.find(|end| ('@'..='~').contains(end));
}

/// Passes over a string's rest, to the bell or string terminator ending it.
fn string(chars: &mut Peekable<Chars>) {
    while let Some(each) = chars.next() {
        match each {
            '\u{7}' | '\u{9c}' => return,
            // An escape ends the string; one that isn't its terminator
            // starts a sequence of its own, which goes as any other does.
            '\u{1b}' => {
                if chars.next_if_eq(&'\\').is_none() {
                    escape(chars);
                }
                return;
            }
            _ => {}
        }
    }
}

/// Whether `each` is a format character that turns, hides or breaks text:
/// a soft hyphen, direction marks and overrides, joiners and invisible
/// operators, annotation marks, and tags. Some of these the terminal kit
/// keeps, since a word or an emoji needs them drawn, but a row of plain
/// words needs none of them, and each could hide a difference.
fn hides(each: char) -> bool {
    matches!(
        each,
        '\u{ad}'
            | '\u{61c}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0000}'..='\u{e007f}'
    )
}

#[cfg(test)]
mod tests {
    use super::words;

    #[test]
    fn what_would_act_on_a_terminal_goes_whole_and_words_stay_apart() {
        for (said, shown, what) in [
            ("a\tb\rc", "a b c", "a tab or a return parts words"),
            ("\u{1b}[31mred\u{1b}[m", "red", "a colour goes whole"),
            (
                "\u{1b}]8;;https://x\u{1b}\\link\u{1b}]8;;\u{1b}\\ broke",
                "link broke",
                "a link keeps its words, not its address",
            ),
            (
                "\u{1b}]0;a title\u{7}said",
                "said",
                "a title set goes whole",
            ),
            (
                "\u{9b}31mred\u{9d}0;title\u{9c}",
                "red",
                "one-character forms go whole",
            ),
            (
                "\u{202e}gone\u{200b}\u{2066}\u{e0041}\u{ad}",
                "gone",
                "what turns or hides text goes",
            ),
            (
                "\u{7}",
                "",
                "a line of only control characters leaves nothing",
            ),
            (
                "a\u{2028}\tb",
                "a b",
                "a line separator parts words as a tab does",
            ),
            ("a;\t\tb", "a; b", "controls together leave one space"),
            (
                "\u{1b}]0;title\u{1b}[31mred",
                "red",
                "a sequence that ends a string goes whole too",
            ),
            (
                "said\u{1b}]0;never ended",
                "said",
                "a string never ended takes the rest with it",
            ),
            (
                "a\u{3164}\u{1d173}b",
                "ab",
                "what the terminal kit keeps from a terminal goes",
            ),
        ] {
            assert_eq!(words(said), shown, "{what}");
        }
    }
}
