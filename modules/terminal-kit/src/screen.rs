//! A page drawn over the last one, for a command that watches something
//! and says it afresh as it changes. Only the kit spells the terminal's
//! escapes (docs/commands.md, Inks, and commands-agree's inks point), so
//! a watching command asks it here rather than writing them itself.

/// Where the terminal's cursor goes home, opening each page.
const HOME: &str = "\u{1b}[H";

/// What clears a line from the cursor to its end.
const LINE_END: &str = "\u{1b}[K";

/// What clears everything below the cursor.
const BELOW: &str = "\u{1b}[J";

/// A page as a terminal draws it over the last: from the top, each line
/// cleared past its end and everything below the page cleared. It doesn't
/// blank the screen first, so a reader never sees it empty between pages.
/// Only for a terminal: anywhere else a page follows the last as it is.
/// A page should fit the terminal's rows: each line ends in a newline, so
/// one as tall as the screen scrolls its first line away on each draw.
pub fn over(page: &str) -> String {
    let mut drawn = String::from(HOME);
    for line in page.lines() {
        drawn.push_str(line);
        drawn.push_str(LINE_END);
        drawn.push('\n');
    }
    drawn.push_str(BELOW);
    drawn
}

/// The pages a run of [`over`] drew, as they read, for a test of a
/// watching command to read what it drew. Each line reads back ending in
/// a newline, as `over` draws it, whether or not the page's last did,
/// and anything written before the first page isn't a page. It reads
/// only pages holding no escapes of their own that `over` writes.
/// Public so a watching command's own tests can read it.
pub fn pages(drawn: &str) -> Vec<String> {
    drawn
        .split(HOME)
        .skip(1)
        .map(|page| page.replace(LINE_END, "").replace(BELOW, ""))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_drawn_from_the_top_each_line_and_below_it_cleared() {
        assert_eq!(
            over("one\ntwo\n"),
            "\u{1b}[Hone\u{1b}[K\ntwo\u{1b}[K\n\u{1b}[J",
            "a page isn't drawn over the last"
        );
    }

    #[test]
    fn a_page_with_no_lines_still_clears_what_was_there() {
        assert_eq!(over(""), "\u{1b}[H\u{1b}[J", "an empty page left the last");
    }

    #[test]
    fn the_pages_drawn_read_as_they_were_given() {
        let drawn = over("one\ntwo\n") + &over("three\n");
        assert_eq!(
            pages(&drawn),
            ["one\ntwo\n", "three\n"],
            "the pages don't read as given"
        );
    }

    #[test]
    fn a_page_reads_back_with_its_last_line_ended() {
        assert_eq!(
            pages(&(String::from("waiting\n") + &over("one"))),
            ["one\n"],
            "a page's last line didn't read back ended, or words before it read as a page"
        );
    }
}
