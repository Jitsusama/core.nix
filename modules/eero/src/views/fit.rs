//! How a line of runs gives way to a terminal's width (docs/commands.md,
//! Width and What Gives Way): wrapped at its items first, the separator
//! going with the break, then at its words, never parting a figure from
//! its unit or breaking a command; a name shortened in its middle, and a
//! last column cut at a whole word, each by the terminal kit's own rule
//! with every run keeping its role. Into a pipe nothing here is asked.

use terminal_kit::layout::{cut, shortened};

use super::marks::marks;
use super::text::{Line, Role, Run, run, width};

/// What the terminal kit leaves where it cuts a word or a name.
const CUT: char = '\u{2026}';

/// The units eero's pages write after a figure, each held to it. A list
/// rather than a length, since a short word such as `on` or `of` after a
/// figure begins the next phrase and may part from it.
const UNITS: [&str; 29] = [
    "B", "kB", "MB", "GB", "TB", "b/s", "kb/s", "Mb/s", "Gb/s", "bit/s", "kbit/s", "Mbit/s",
    "Gbit/s", "Hz", "kHz", "MHz", "GHz", "dB", "dBm", "%", "ns", "\u{b5}s", "ms", "s", "min", "h",
    "d", "y", "bars",
];

/// A line as its letters, each with its role and the run it came from.
type Letters = Vec<(char, Role, usize)>;

fn letters(line: &[Run]) -> Letters {
    line.iter()
        .enumerate()
        .flat_map(|(at, run)| run.text.chars().map(move |letter| (letter, run.role, at)))
        .collect()
}

fn plain_of(letters: &[(char, Role, usize)]) -> String {
    letters.iter().map(|(letter, _, _)| *letter).collect()
}

/// Letters as runs again, a run for each stretch of one role.
fn runs(letters: &[(char, Role, usize)]) -> Line {
    let mut line: Line = Vec::new();
    for (letter, role, _) in letters {
        match line.last_mut() {
            Some(last) if last.role == *role => last.text.push(*letter),
            _ => line.push(run(letter.to_string(), *role)),
        }
    }
    line
}

/// The letters a word set shorter keeps, in their roles, with the cut
/// between its ends muted; the whole set in the first letter's role where
/// it isn't the word's ends.
fn restyled(word: &[(char, Role, usize)], set: &str) -> Line {
    let whole = plain_of(word);
    if set == whole {
        return runs(word);
    }
    let fallback = || {
        let role = word.first().map_or(Role::Text, |(_, role, _)| *role);
        vec![run(set, role)]
    };
    let Some((head, tail)) = set.split_once(CUT) else {
        return fallback();
    };
    let (head_n, tail_n) = (head.chars().count(), tail.chars().count());
    if !whole.starts_with(head) || !whole.ends_with(tail) || head_n + tail_n > word.len() {
        return fallback();
    }
    let mut kept: Letters = word[..head_n].to_vec();
    kept.push((CUT, Role::Muted, usize::MAX));
    kept.extend_from_slice(&word[word.len() - tail_n..]);
    runs(&kept)
}

/// A cell shortened in its middle to `cells`, as a name is.
pub fn shorter(line: &[Run], cells: usize) -> Line {
    let word = letters(line);
    restyled(&word, &shortened(&plain_of(&word), cells))
}

/// A cell cut to `cells` at a whole word, as a table's last column is.
pub fn cut_to(line: &[Run], cells: usize) -> Line {
    let word = letters(line);
    restyled(&word, &cut(&plain_of(&word), cells))
}

fn line_width(line: &[Run]) -> usize {
    line.iter().map(|run| width(&run.text)).sum()
}

/// Whether a run parts a line's items: the separator between facts, or
/// a run of spaces alone, as sets a graph apart from the next figure.
fn parts_items(run: &Run) -> bool {
    (run.role == Role::Muted && run.text == format!(" {} ", marks().separator))
        || (run.text.len() >= 2 && run.text.trim().is_empty())
}

/// A line wrapped to `measure` cells: at its items first, what parts
/// them going with the break, and an item too long for a line of its own
/// at its words.
pub fn wrapped(line: &[Run], measure: usize) -> Vec<Line> {
    let mut items: Vec<(Option<Run>, Line)> = vec![(None, Vec::new())];
    for each in line {
        if parts_items(each) {
            items.push((Some(each.clone()), Vec::new()));
        } else if let Some((_, item)) = items.last_mut() {
            item.push(each.clone());
        }
    }
    if items.len() == 1 {
        return words(line, measure);
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut current: Line = Vec::new();
    for (separator, item) in items {
        let between = separator.as_ref().map_or(0, |run| width(&run.text));
        let joined = line_width(&current) + between + line_width(&item);
        if current.is_empty() && line_width(&item) <= measure {
            current = item;
        } else if !current.is_empty() && joined <= measure {
            current.extend(separator);
            current.extend(item);
        } else if line_width(&item) <= measure {
            lines.push(std::mem::replace(&mut current, item));
        } else {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut pieces = words(&item, measure);
            current = pieces.pop().unwrap_or_default();
            lines.extend(pieces);
        }
    }
    lines.push(current);
    lines
}

/// Whether a word is a figure, which a unit after it stays with.
fn figure(word: &str) -> bool {
    word.trim_start_matches(['(', '[', '~', '\u{2212}', '-', '+'])
        .starts_with(|first: char| first.is_ascii_digit())
}

/// Whether a word is the unit of a figure before it, with any stop after.
fn unit(word: &str) -> bool {
    UNITS.contains(&word.trim_end_matches(['.', ',', ';', ':', ')', ']']))
}

/// A line wrapped to `measure` cells at its words. A break never parts a
/// figure from its unit, nor the words of one strong run, which is a name
/// or a command to type whole; a piece too long for a line of its own is
/// shortened in its middle.
fn words(line: &[Run], measure: usize) -> Vec<Line> {
    let all = letters(line);
    let mut pieces: Vec<Letters> = Vec::new();
    let mut word: Letters = Vec::new();
    let mut held = false;
    for (at, letter) in all.iter().enumerate() {
        if letter.0 != ' ' {
            word.push(*letter);
            continue;
        }
        // A space inside one strong run, between a figure and its unit, or
        // between a word and the figure that numbers it, as a model's or a
        // channel's, holds the words either side together, so no line opens
        // on a figure parted from what it counts.
        let next: String = all[at + 1..]
            .iter()
            .take_while(|(letter, _, _)| *letter != ' ')
            .map(|(letter, _, _)| *letter)
            .collect();
        let strong_within = letter.1 == Role::Strong
            && all
                .get(at + 1)
                .is_some_and(|after| after.2 == letter.2 && after.0 != ' ');
        let said = plain_of(&word);
        let numbered = !figure(&said)
            && !said.ends_with(|last: char| last.is_ascii_punctuation())
            && figure(&next);
        let holds = strong_within || numbered || (figure(&said) && unit(&next));
        if !word.is_empty() {
            if held && let Some(piece) = pieces.last_mut() {
                piece.push((' ', Role::Text, usize::MAX));
                piece.append(&mut word);
            } else {
                pieces.push(std::mem::take(&mut word));
            }
        }
        held = holds;
    }
    if !word.is_empty() {
        if held && let Some(piece) = pieces.last_mut() {
            piece.push((' ', Role::Text, usize::MAX));
            piece.append(&mut word);
        } else {
            pieces.push(word);
        }
    }
    let across = |line: &[Line]| {
        line.iter().map(|piece| line_width(piece)).sum::<usize>() + line.len().saturating_sub(1)
    };
    let mut lines: Vec<Vec<Line>> = vec![Vec::new()];
    for piece in pieces {
        let whole = runs(&piece);
        let set = if line_width(&whole) > measure {
            shorter(&whole, measure)
        } else {
            whole
        };
        let Some(current) = lines.last_mut() else {
            continue;
        };
        if current.is_empty() || across(current) + 1 + line_width(&set) <= measure {
            current.push(set);
        } else {
            lines.push(vec![set]);
        }
    }
    // No last word is left alone on its line: the word before it comes
    // down too, where both fit and the line it leaves keeps two. A last
    // piece of several words held together, as a figure and its unit, is
    // no orphan.
    if let [.., before, last] = lines.as_mut_slice()
        && let [alone] = last.as_slice()
        && !alone.iter().any(|each| each.text.contains(' '))
        && before.len() > 2
        && before
            .last()
            .is_some_and(|word| across(last) + 1 + line_width(word) <= measure)
        && let Some(word) = before.pop()
    {
        last.insert(0, word);
    }
    lines
        .into_iter()
        .map(|pieces| {
            let mut line = Vec::new();
            for (at, piece) in pieces.into_iter().enumerate() {
                if at > 0 {
                    line.push(run(" ", Role::Text));
                }
                line.extend(piece);
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::text::{muted, plain, strong};
    use super::*;

    fn said(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|run| run.text.as_str()).collect())
            .collect()
    }

    fn dot() -> Run {
        muted(" · ")
    }

    #[test]
    fn a_line_wraps_at_its_items_and_the_dot_goes_with_the_break() {
        let line = vec![
            plain("370"),
            muted(" Mb/s"),
            plain(" down"),
            dot(),
            plain("11.9"),
            muted(" Mb/s"),
            plain(" up"),
            dot(),
            plain("2026-11-05 05:52, 2 d 12 h ago"),
        ];
        assert_eq!(
            said(&wrapped(&line, 30)),
            [
                "370 Mb/s down · 11.9 Mb/s up",
                "2026-11-05 05:52, 2 d 12 h ago"
            ]
        );
    }

    #[test]
    fn words_wrap_holding_a_figure_to_its_unit_and_a_command_whole() {
        let line = vec![
            plain("nothing new for 2 h 5 min; eero-daemon may have stopped: "),
            strong("systemctl --user start eero-daemon"),
        ];
        assert_eq!(
            said(&wrapped(&line, 40)),
            [
                "nothing new for 2 h 5 min; eero-daemon",
                "may have stopped:",
                "systemctl --user start eero-daemon"
            ]
        );
        let out = vec![plain("It was out for 1 min 30 s.")];
        assert_eq!(
            said(&wrapped(&out, 21)),
            ["It was out", "for 1 min 30 s."],
            "a last word is never left alone on its line"
        );
        let joined = vec![plain("tablet 1 connected to Den eero Pro 6 on channel 52.")];
        assert_eq!(
            said(&wrapped(&joined, 34)),
            ["tablet 1 connected to Den eero", "Pro 6 on channel 52."],
            "a figure opens no line apart from the word it numbers"
        );
        assert_eq!(
            said(&wrapped(&joined, 36)),
            ["tablet 1 connected to Den eero Pro 6", "on channel 52."],
            "a figure holds to its units alone, not to a short word after it"
        );
    }

    #[test]
    fn each_run_keeps_its_role_through_a_wrap() {
        let line = vec![plain("10.0.0.1"), dot(), plain("fe80::7/64")];
        let lines = wrapped(&line, 12);
        assert_eq!(said(&lines), ["10.0.0.1", "fe80::7/64"]);
        assert_eq!(lines[1][0].role, Role::Text);
    }

    #[test]
    fn a_name_is_shortened_in_its_middle_and_a_cell_cut_at_a_word() {
        assert_eq!(
            said(&[shorter(&[plain("unknown computer 1")], 12)]),
            ["unkno…uter 1"]
        );
        let cut = cut_to(&[plain("Wi-Fi 5.8 GHz from Hall")], 16);
        assert_eq!(said(std::slice::from_ref(&cut)), ["Wi-Fi 5.8 GHz…"]);
        assert_eq!(cut.last().map(|run| run.role), Some(Role::Muted));
    }
}
