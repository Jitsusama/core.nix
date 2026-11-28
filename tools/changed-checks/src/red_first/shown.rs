//! How red first shows what a change wrote, its commits' trailers and its
//! files' paths, in the run's log: as written, but for what a terminal
//! would act on.

/// `text`, a change's own words, as the run's log shows them: each control
/// character escaped, as Rust writes it in a string, so an escape sequence
/// a trailer or path holds is read rather than run by the reader's
/// terminal; and so is each mark turning the direction text runs, which
/// shows one thing while holding another. A backslash is escaped too, so
/// an escape shown is never one the change wrote out itself.
pub fn shown(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() || character == '\\' {
                character.escape_debug().to_string()
            } else if turns(character) {
                character.escape_unicode().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// Whether `character` marks or turns the direction text runs, or is a
/// mark nothing shows, as the zero-width space: each can reorder or hide
/// what a line shows.
fn turns(character: char) -> bool {
    matches!(
        character,
        '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}
