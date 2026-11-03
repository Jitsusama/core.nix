//! The page's links: where each sits on the page and where it goes, and the
//! labels g x puts over the ones in view, as flash puts them in Joel's
//! Neovim, typed to follow one.
use std::path::{Path, PathBuf};

use typst::layout::{Frame, FrameItem, Point};
use typst::model::Destination;

/// A link's first piece on the page, in points from the page's top left,
/// and what it points at as written.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub left_pt: f64,
    pub top_pt: f64,
    pub height_pt: f64,
    pub href: String,
}

/// Every link on the page, in reading order, each once: a link broken
/// across lines is labelled where it starts.
pub fn links(frame: &Frame) -> Vec<Link> {
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found.sort_by(|a, b| {
        a.top_pt
            .total_cmp(&b.top_pt)
            .then(a.left_pt.total_cmp(&b.left_pt))
    });
    let mut kept: Vec<Link> = Vec::new();
    for link in found {
        // A link's later pieces follow its first, on the lines below it.
        let continues = kept.last().is_some_and(|last| {
            last.href == link.href
                && link.top_pt > last.top_pt
                && link.top_pt - last.top_pt < 3.0 * last.height_pt
        });
        if !continues {
            kept.push(link);
        }
    }
    kept
}

fn walk(frame: &Frame, at: Point, found: &mut Vec<Link>) {
    for (place, item) in frame.items() {
        let here = at + *place;
        match item {
            FrameItem::Group(group) => {
                let moved = Point::new(group.transform.tx, group.transform.ty);
                walk(&group.frame, here + moved, found);
            }
            FrameItem::Link(Destination::Url(url), size) => found.push(Link {
                left_pt: here.x.to_pt(),
                top_pt: here.y.to_pt(),
                height_pt: size.y.to_pt(),
                href: url.as_str().to_string(),
            }),
            _ => {}
        }
    }
}

/// Where a link goes.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// A heading on this page, by the name GitHub gives its anchor.
    Here(String),
    /// Anything else: a site, an address, or a file, opened as the desktop
    /// opens it.
    Elsewhere(String),
}

/// Where `href` goes, from the page at `page`: a bare fragment, or this
/// page's own name with one, is here; a path is taken from the page's
/// folder.
pub fn target(href: &str, page: &Path) -> Target {
    if let Some(fragment) = href.strip_prefix('#') {
        return Target::Here(fragment.to_string());
    }
    if href.contains("://") || href.starts_with("mailto:") {
        return Target::Elsewhere(href.to_string());
    }
    let (path, fragment) = href.split_once('#').unwrap_or((href, ""));
    let folder = page.parent().unwrap_or(Path::new("/"));
    let file: PathBuf = if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        folder.join(path)
    };
    if !fragment.is_empty() && same(&file, page) {
        return Target::Here(fragment.to_string());
    }
    Target::Elsewhere(file.to_string_lossy().into_owned())
}

fn same(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// A heading's anchor as GitHub makes it: lowercased, spaces as hyphens,
/// and every other mark but hyphens and underscores left out.
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// Where the heading whose anchor is `anchor` starts, in points: its first
/// line's top. A heading set over several lines, as a long title is, is
/// read across up to six of them.
pub fn heading_at(lines: &[crate::text::Line], anchor: &str) -> Option<f64> {
    (0..lines.len()).find_map(|first| {
        let mut joined = String::new();
        for line in lines.iter().skip(first).take(6) {
            joined.push_str(&line.text);
            if slug(&joined) == anchor {
                let top = &lines[first];
                return Some(top.baseline_pt - top.size_pt);
            }
        }
        None
    })
}

/// The letters labels are made of, as flash's in his Neovim: the home row
/// first.
const LETTERS: &str = "asdfghjklqwertyuiopzxcvbnm";

/// A label for each of `count` links: one letter while they last, then two,
/// so no label begins another.
pub fn labels(count: usize) -> Vec<String> {
    let letters: Vec<char> = LETTERS.chars().collect();
    if count <= letters.len() {
        return letters.iter().take(count).map(char::to_string).collect();
    }
    letters
        .iter()
        .flat_map(|a| letters.iter().map(move |b| format!("{a}{b}")))
        .take(count)
        .collect()
}

/// What a letter typed while labels show did.
#[derive(Debug, PartialEq, Eq)]
pub enum Typed {
    /// It began a label of two letters: wait for the second.
    Waiting,
    /// It finished the label at this index.
    Chosen(usize),
    /// No label goes this way: they're put away.
    Gone,
}

/// The labels showing, and what's been typed of one.
pub struct Labelling {
    pub labels: Vec<String>,
    pub typed: String,
}

impl Labelling {
    pub fn new(count: usize) -> Self {
        Labelling {
            labels: labels(count),
            typed: String::new(),
        }
    }

    pub fn key(&mut self, letter: char) -> Typed {
        self.typed.push(letter);
        if let Some(at) = self.labels.iter().position(|label| *label == self.typed) {
            return Typed::Chosen(at);
        }
        if self
            .labels
            .iter()
            .any(|label| label.starts_with(&self.typed))
        {
            Typed::Waiting
        } else {
            Typed::Gone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_goes_here_by_its_fragment_and_elsewhere_otherwise() {
        let page = Path::new("/notes/page.md");
        assert_eq!(target("#the-keys", page), Target::Here("the-keys".into()));
        assert_eq!(
            target("https://example.com/a#b", page),
            Target::Elsewhere("https://example.com/a#b".into())
        );
        assert_eq!(
            target("mailto:joel@example.com", page),
            Target::Elsewhere("mailto:joel@example.com".into())
        );
        assert_eq!(
            target("other.md", page),
            Target::Elsewhere("/notes/other.md".into())
        );
        assert_eq!(
            target("../up.md#top", page),
            Target::Elsewhere("/notes/../up.md".into())
        );
        assert_eq!(
            target("page.md#the-keys", page),
            Target::Here("the-keys".into())
        );
        assert_eq!(
            target("/etc/hosts", page),
            Target::Elsewhere("/etc/hosts".into())
        );
    }

    #[test]
    fn a_headings_anchor_is_githubs() {
        assert_eq!(slug("Vi's Keys"), "vis-keys");
        assert_eq!(
            slug("  Every Pixel Accounted For "),
            "every-pixel-accounted-for"
        );
        assert_eq!(slug("A, B and C: done"), "a-b-and-c-done");
        assert_eq!(
            slug("snake_case and kebab-case"),
            "snake_case-and-kebab-case"
        );
    }

    #[test]
    fn labels_are_one_letter_while_they_last_and_none_begins_another() {
        assert_eq!(labels(3), ["a", "s", "d"]);
        let many = labels(30);
        assert_eq!(many.len(), 30);
        assert!(many.iter().all(|label| label.len() == 2));
        for (i, a) in many.iter().enumerate() {
            for b in &many[i + 1..] {
                assert!(!b.starts_with(a.as_str()) && !a.starts_with(b.as_str()));
            }
        }
    }

    #[test]
    fn a_typed_label_is_chosen_waited_on_or_gone() {
        let mut one = Labelling::new(3);
        assert_eq!(one.key('s'), Typed::Chosen(1));
        let mut none = Labelling::new(3);
        assert_eq!(none.key('z'), Typed::Gone);
        let mut two = Labelling::new(30);
        assert_eq!(two.key('a'), Typed::Waiting);
        assert_eq!(two.key('d'), Typed::Chosen(2));
    }

    #[test]
    fn a_headings_line_reads_as_its_anchor() {
        let dir = std::env::temp_dir().join(format!("reader-anchor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(
            &file,
            "# A Title Long Enough to Run Over Onto a Second Line, However Wide Its Face Is\n\nWords.\n\n## Vi's Keys\n",
        )
        .expect("the file");
        let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let lines = crate::text::lines(crate::text::runs(&page.frame));
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        let title = heading_at(
            &lines,
            "a-title-long-enough-to-run-over-onto-a-second-line-however-wide-its-face-is",
        );
        let keys = heading_at(&lines, "vis-keys");
        assert!(title.is_some() && keys.is_some(), "{texts:?}");
        assert!(title < keys, "{title:?} {keys:?}");
        assert_eq!(heading_at(&lines, "words-vis"), None);
        // A title is never hyphenated, however it wraps.
        assert!(
            lines.iter().all(|line| !line.text.contains('\u{ad}')),
            "{texts:?}"
        );
    }

    #[test]
    fn a_link_broken_across_lines_is_labelled_once_where_it_starts() {
        let markdown = format!(
            "{}[a link long enough to run on past the end of its first line and onto the next one, so it breaks](https://example.com) and [a second](#here).\n",
            "Words before it. ".repeat(5)
        );
        let dir = std::env::temp_dir().join(format!("reader-links-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let found = links(&page.frame);
        let hrefs: Vec<&str> = found.iter().map(|link| link.href.as_str()).collect();
        assert_eq!(hrefs, ["https://example.com", "#here"]);
    }
}
