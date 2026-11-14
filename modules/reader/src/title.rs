//! The window's title: what the document is called, so two readers open
//! side by side, and the launcher's lists of windows, tell them apart.
use comrak::nodes::NodeValue;

/// The title a window showing `file` gives: the document's first title,
/// its level-one heading's words, or else the file's own name; then the
/// program's, after a middot.
pub fn of(file: &std::path::Path, markdown: &str) -> String {
    let name = heading(markdown).or_else(|| {
        file.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    match name {
        Some(name) if !name.is_empty() => format!("{name} \u{b7} Reader"),
        _ => "Reader".to_string(),
    }
}

/// The first title's words. Only the file up to the first line that looks
/// like one is read, since it is set again on every save and the title is
/// nearly always near the top; when that line was none, the whole is.
fn heading(markdown: &str) -> Option<String> {
    let end = first_title_line(markdown)?;
    titled(&markdown[..end]).or_else(|| {
        if end < markdown.len() {
            titled(markdown)
        } else {
            None
        }
    })
}

/// Where the first line that could be a level-one heading ends, past the
/// front matter and outside fenced code: a `#` and a space, or a line of
/// `=` under a line of words. Comrak has the last word on whether it is.
fn first_title_line(markdown: &str) -> Option<usize> {
    let mut from = 0;
    if let Some(after) = markdown.strip_prefix("---\n")
        && let Some(close) = after.find("\n---")
    {
        from = 4 + close + 4;
    }
    let mut fence: Option<(char, usize)> = None;
    let mut after_words = false;
    let mut at = from;
    for line in markdown[from..].split_inclusive('\n') {
        at += line.len();
        let text = line.trim_end();
        let set = text.trim_start_matches(' ');
        if text.len() - set.len() > 3 {
            after_words = !set.is_empty();
            continue;
        }
        let run = |mark: char| set.chars().take_while(|&c| c == mark).count();
        if let Some((mark, count)) = fence {
            if run(mark) >= count && set.trim_start_matches(mark).is_empty() {
                fence = None;
            }
            after_words = false;
            continue;
        }
        if let Some(mark) = ['`', '~'].into_iter().find(|&mark| run(mark) >= 3) {
            fence = Some((mark, run(mark)));
            after_words = false;
            continue;
        }
        let atx = set == "#" || set.starts_with("# ") || set.starts_with("#\t");
        let setext = after_words && !set.is_empty() && set.chars().all(|c| c == '=');
        if atx || setext {
            return Some(at);
        }
        after_words = !set.is_empty();
    }
    None
}

/// The words of the first level-one heading in `source` that has any, as
/// plain text: its marks gone, code and math as written, and its spaces
/// run together.
fn titled(source: &str) -> Option<String> {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, source, &crate::walk::options());
    root.children().find_map(|node| {
        if !matches!(&node.data().value, NodeValue::Heading(heading) if heading.level == 1) {
            return None;
        }
        let mut words = String::new();
        for part in node.descendants() {
            match &part.data().value {
                NodeValue::Text(text) => words.push_str(text),
                NodeValue::Code(code) => words.push_str(&code.literal),
                NodeValue::Math(math) => words.push_str(&math.literal),
                NodeValue::SoftBreak | NodeValue::LineBreak => words.push(' '),
                _ => (),
            }
        }
        let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
        (!words.is_empty()).then_some(words)
    })
}

#[cfg(test)]
mod tests {
    use super::of;

    fn titled(markdown: &str) -> String {
        of(std::path::Path::new("/notes/now.md"), markdown)
    }

    #[test]
    fn it_says_the_title_in_plain_words() {
        assert_eq!(
            titled("# The **Bold** `plan`, [linked](x.md)\n\nWords.\n"),
            "The Bold plan, linked \u{b7} Reader"
        );
    }

    #[test]
    fn a_title_set_off_by_a_rule_is_a_title_too() {
        assert_eq!(
            titled("Notes for Today\n===\n\nWords.\n"),
            "Notes for Today \u{b7} Reader"
        );
    }

    #[test]
    fn front_matter_and_code_never_title_it() {
        let markdown = "---\n# a comment\nkind: plan\n---\n\n```sh\n# not a title\n```\n\n\
                        Words first.\n\n# Sanderling Notes\n";
        assert_eq!(titled(markdown), "Sanderling Notes \u{b7} Reader");
    }

    #[test]
    fn without_a_title_it_says_the_files_name() {
        assert_eq!(
            titled("## Only a Section\n\nWords.\n"),
            "now.md \u{b7} Reader"
        );
        assert_eq!(titled("#\n\nWords.\n"), "now.md \u{b7} Reader");
        assert_eq!(titled(""), "now.md \u{b7} Reader");
    }

    #[test]
    fn its_spaces_run_together() {
        assert_eq!(
            titled("#   Two  \t Spaced   \n"),
            "Two Spaced \u{b7} Reader"
        );
    }
}
