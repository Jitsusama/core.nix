//! gO's outline: the page's headings as a list at the window's foot, as
//! Neovim's gO opens them in a location list, moved through in normal mode
//! with j and k, gg and G and counts, searched with / and ? then n and N,
//! Return going to the heading chosen and Escape or ZZ putting it away.
//! :g/pat/, :marks and :jumps list their lines in it the same way. Kept
//! apart from the window so it can be tested; prompt.rs sets it.
use crate::place::Block;

/// The rows the list shows at once, as Neovim's location list opens ten
/// lines tall; a longer outline scrolls within them.
pub const SHOWN: usize = 10;

/// A row of the list: its level, its words, and where Return goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub level: u8,
    pub words: String,
    pub to: To,
}

/// Where a row goes, in points down its page.
#[derive(Clone, Debug, PartialEq)]
pub enum To {
    /// A heading, set at the view's top, as gO's go.
    Heading(f64),
    /// A line, the cursor put on it and the view moved as Vim's is.
    Line(f64),
    /// A place on the jump list, on this file or another: the view's top
    /// and the cursor's line, as Ctrl+O goes back to it.
    Jump(std::path::PathBuf, f64, f64),
}

pub struct Outline {
    pub rows: Vec<Row>,
    /// The row chosen, and the first row shown.
    pub chosen: usize,
    pub first: usize,
    /// The last search through the rows, and whether it went down.
    pub searched: Option<(String, bool)>,
}

impl Outline {
    /// The page's headings, the one the view is in chosen, as Neovim's
    /// list opens on the entry the cursor is in; none on a page without a
    /// heading, where gO opens nothing.
    pub fn new(blocks: &[Block], view_pt: f64) -> Option<Outline> {
        let rows: Vec<Row> = blocks
            .iter()
            .filter_map(|block| {
                let (level, words) = block.heading.clone()?;
                Some(Row {
                    level,
                    words,
                    to: To::Heading(block.top_pt),
                })
            })
            .collect();
        let chosen = rows
            .iter()
            .rposition(|row| matches!(row.to, To::Heading(top) if top <= view_pt + 1.0))
            .unwrap_or(0);
        Outline::of(rows, chosen)
    }

    /// A list of rows, one chosen; none for no rows.
    pub fn of(rows: Vec<Row>, chosen: usize) -> Option<Outline> {
        if rows.is_empty() {
            return None;
        }
        let mut outline = Outline {
            rows,
            chosen: 0,
            first: 0,
            searched: None,
        };
        outline.choose(chosen);
        Some(outline)
    }

    /// Chooses a row, scrolling the list just enough to show it.
    pub fn choose(&mut self, row: usize) {
        self.chosen = row.min(self.rows.len() - 1);
        if self.chosen < self.first {
            self.first = self.chosen;
        } else if self.chosen >= self.first + SHOWN {
            self.first = self.chosen + 1 - SHOWN;
        }
    }

    /// Down or up `by` rows, stopping at the ends, as j and k do.
    pub fn by(&mut self, by: isize) {
        self.choose(self.chosen.saturating_add_signed(by));
    }

    /// The rows shown, each with whether it's chosen.
    pub fn shown(&self) -> impl Iterator<Item = (&Row, bool)> {
        self.rows
            .iter()
            .enumerate()
            .skip(self.first)
            .take(SHOWN)
            .map(|(at, row)| (row, at == self.chosen))
    }

    /// Searches the rows' words for `query` from the row chosen, the way
    /// asked, `times` over, round the list's ends as wrapscan has it, case
    /// ignored until a capital is typed. Whether it found any.
    pub fn find(&mut self, query: &str, forward: bool, times: u32) -> bool {
        let ignore_case = !query.chars().any(char::is_uppercase);
        let matches = |words: &str| {
            if ignore_case {
                words.to_lowercase().contains(&query.to_lowercase())
            } else {
                words.contains(query)
            }
        };
        let count = self.rows.len();
        let mut at = self.chosen;
        let mut found = false;
        for _ in 0..times.max(1) {
            let next = (1..=count)
                .map(|step| {
                    if forward {
                        (at + step) % count
                    } else {
                        (at + count - step % count) % count
                    }
                })
                .find(|&row| matches(&self.rows[row].words));
            match next {
                Some(row) => {
                    at = row;
                    found = true;
                }
                None => break,
            }
        }
        self.choose(at);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(count: usize) -> Vec<Block> {
        (0..count)
            .map(|at| Block {
                number: at,
                top_pt: f64::from(u32::try_from(at).unwrap_or(0)) * 100.0,
                lines: None,
                // Every other block a heading: the first a title, the rest
                // sections.
                heading: (at % 2 == 0).then(|| (if at == 0 { 1 } else { 2 }, format!("Part {at}"))),
            })
            .collect()
    }

    #[test]
    fn it_lists_only_the_headings_and_opens_on_the_one_the_view_is_in() {
        let outline = Outline::new(&blocks(6), 250.0).expect("headings");
        let words: Vec<&str> = outline.rows.iter().map(|row| row.words.as_str()).collect();
        assert_eq!(words, ["Part 0", "Part 2", "Part 4"]);
        assert_eq!(outline.chosen, 1);
        assert!(Outline::new(&[], 0.0).is_none(), "no headings, no list");
    }

    #[test]
    fn j_and_k_stop_at_its_ends_and_a_long_list_scrolls_to_the_row_chosen() {
        let mut outline = Outline::new(&blocks(40), 0.0).expect("headings");
        outline.by(-1);
        assert_eq!(outline.chosen, 0);
        outline.by(12);
        assert_eq!((outline.chosen, outline.first), (12, 3));
        outline.by(100);
        assert_eq!((outline.chosen, outline.first), (19, 10));
        assert_eq!(outline.shown().count(), SHOWN);
        assert!(outline.shown().last().is_some_and(|(_, chosen)| chosen));
    }

    #[test]
    fn a_search_goes_round_the_list_either_way_with_smartcase() {
        let mut outline = Outline::new(&blocks(10), 0.0).expect("headings");
        assert!(outline.find("part 4", true, 1));
        assert_eq!(outline.chosen, 2);
        assert!(outline.find("part", true, 3));
        assert_eq!(outline.chosen, 0, "round past the end");
        assert!(outline.find("Part", false, 1));
        assert_eq!(outline.chosen, 4);
        assert!(!outline.find("PART", true, 1));
        assert_eq!(outline.chosen, 4, "nothing found stays put");
    }
}
