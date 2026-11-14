//! Markdown, as a summary or a transcript is written, as a drawer's styled
//! text: a heading in bold, a list's items each a line led by a bullet, a
//! quote in italics, and a blank line between paragraphs, so it reads as
//! the reader would set it, on the drawer's one pitch. A plain line right
//! after another goes on the same paragraph, as markdown reads a line
//! wrapped by hand. Nothing a call said is read as markup: it's escaped
//! before markdown's own marks are styled.

/// The markdown as styled text.
pub fn styled(markdown: &str) -> String {
    let markdown = markdown.replace('\r', "");
    let mut paragraphs: Vec<Vec<&str>> = vec![];
    let mut lines: Vec<&str> = vec![];
    for line in markdown.split('\n').map(str::trim) {
        if line.is_empty() {
            if !lines.is_empty() {
                paragraphs.push(std::mem::take(&mut lines));
            }
        } else {
            lines.push(line);
        }
    }
    if !lines.is_empty() {
        paragraphs.push(lines);
    }
    paragraphs
        .iter()
        .map(|lines| paragraph(lines))
        .collect::<Vec<_>>()
        .join("<br><br>")
}

/// A paragraph's lines, a line apiece but for plain lines in a row, which
/// run on as one.
fn paragraph(lines: &[&str]) -> String {
    let mut out: Vec<String> = vec![];
    let mut plain = false;
    for line in lines {
        let mark = marked(line);
        match (&mark, out.last_mut()) {
            (None, Some(last)) if plain => {
                last.push(' ');
                last.push_str(&inline(line));
            }
            _ => out.push(mark.clone().unwrap_or_else(|| inline(line))),
        }
        plain = mark.is_none();
    }
    out.join("<br>")
}

/// A line markdown marks as a heading, a list's item or a quote, styled,
/// or None for a plain one.
fn marked(line: &str) -> Option<String> {
    let hashes = line.chars().take_while(|letter| *letter == '#').count();
    if (1..=6).contains(&hashes)
        && let Some(rest) = spaced(&line[hashes..])
    {
        return Some(format!("<b>{}</b>", inline(rest)));
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let item = if line.starts_with(['-', '*', '+']) {
        spaced(&line[1..])
    } else if digits > 0 && line[digits..].starts_with(['.', ')']) {
        spaced(&line[digits + 1..])
    } else {
        None
    };
    if let Some(rest) = item {
        return Some(format!("\u{2022} {}", inline(rest)));
    }
    line.strip_prefix('>').map(|rest| {
        let rest = rest.strip_prefix(char::is_whitespace).unwrap_or(rest);
        format!("<i>{}</i>", inline(rest))
    })
}

/// What follows at least one space, or None where none does.
fn spaced(text: &str) -> Option<&str> {
    text.starts_with(char::is_whitespace)
        .then(|| text.trim_start())
}

/// A line's own marks, bold and italics, as styled text, and code as its
/// plain words.
fn inline(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let bold = paired(&escaped, "**", "b");
    let starred = paired(&bold, "*", "i");
    let underscored = paired(&starred, "_", "i");
    paired(&underscored, "`", "")
}

/// Whether a letter is a word's, as a pattern's `\w` reads one.
fn wordy(letter: char) -> bool {
    letter.is_ascii_alphanumeric() || letter == '_'
}

/// Each run between two `mark`s wrapped in `tag`, or left bare with no
/// tag, as code is. Bold and code pair with the next mark. An italic's
/// opens only at the start or after a letter that isn't a word's (nor a
/// star, for a star), before one that isn't a space, and closes at the
/// first mark after it that no word's letter follows, so a name's
/// underscores stay as they are.
fn paired(text: &str, mark: &str, tag: &str) -> String {
    let letters: Vec<char> = text.chars().collect();
    let marks: Vec<char> = mark.chars().collect();
    let bold = tag != "i";
    let at = |index: usize| letters.get(index).copied();
    let is_mark = |index: usize| {
        marks
            .iter()
            .enumerate()
            .all(|(step, letter)| at(index + step) == Some(*letter))
    };
    let mut out = String::new();
    let mut index = 0;
    // Where the last pair closed, so a letter it took can't open another.
    let mut taken = 0;
    while index < letters.len() {
        // A letter the last pair took can't open another, as a pattern
        // matching on from where it stopped can't look back.
        let after_letter = index > taken
            && at(index - 1)
                .is_some_and(|letter| !(wordy(letter) || (mark == "*" && letter == '*')));
        let opens = is_mark(index)
            && (bold
                || ((index == 0 || after_letter)
                    && at(index + 1).is_some_and(|letter| !letter.is_whitespace())));
        let close = opens
            .then(|| {
                (index + marks.len() + 1..letters.len()).find(|&end| {
                    is_mark(end) && (bold || at(end + 1).is_none_or(|letter| !wordy(letter)))
                })
            })
            .flatten();
        if let Some(end) = close {
            let inside: String = letters[index + marks.len()..end].iter().collect();
            if tag.is_empty() {
                out.push_str(&inside);
            } else {
                out.push_str(&["<", tag, ">", &inside, "</", tag, ">"].concat());
            }
            index = end + marks.len();
            taken = index;
        } else {
            out.push(letters[index]);
            index += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::styled;

    #[test]
    fn markdown_is_set_as_the_drawer_sets_styled_text() {
        let cases = [
            (
                "**Joel**, 0:00: Can we move it?\n\n**Others**, 0:04: Thursday works.",
                "<b>Joel</b>, 0:00: Can we move it?<br><br><b>Others</b>, 0:04: Thursday works.",
            ),
            (
                "# Decided\n\n- Move it to *Thursday*\n- Ada writes the notes",
                "<b>Decided</b><br><br>\u{2022} Move it to <i>Thursday</i><br>\u{2022} Ada \
                 writes the notes",
            ),
            (
                "> \"We ship Friday.\"\n> Joel",
                "<i>\"We ship Friday.\"</i><br><i>Joel</i>",
            ),
            ("A line\nwrapped by hand", "A line wrapped by hand"),
            ("1 < 2 & `x` > 0", "1 &lt; 2 &amp; x &gt; 0"),
            ("snake_case_name stays", "snake_case_name stays"),
            ("an _aside_ here", "an <i>aside</i> here"),
            ("1. first\n2) second", "\u{2022} first<br>\u{2022} second"),
            ("#hashtag stays", "#hashtag stays"),
            ("a\r\n\r\nb", "a<br><br>b"),
            ("", ""),
        ];
        for (markdown, expected) in cases {
            assert_eq!(styled(markdown), expected, "styling {markdown:?}");
        }
    }
}
