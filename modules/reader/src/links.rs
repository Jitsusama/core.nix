//! The page's links: where each sits on the page and where it goes, and the
//! labels g x puts over the ones in view, as flash puts them in Joel's
//! Neovim, typed to follow one.
use std::path::{Path, PathBuf};

use typst::layout::{Frame, FrameItem, Point};
use typst::model::Destination;

use crate::numbers::f32_of;

/// A link's first piece on the page, or any one piece of it, in points from
/// the page's top left, and what it points at as written.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub left_pt: f64,
    /// How far across the piece runs, to its line's end when it breaks.
    pub width_pt: f64,
    pub top_pt: f64,
    pub height_pt: f64,
    /// Where its words stand, which its label's letters stand on too.
    pub baseline_pt: f64,
    pub href: String,
}

/// Every link on the page, in reading order, each once: a link broken
/// across lines is labelled where it starts.
pub fn links(frame: &Frame) -> Vec<Link> {
    let mut kept: Vec<Link> = Vec::new();
    for link in pieces(frame) {
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

/// Every piece of every link on the page, in reading order: a link broken
/// across lines is a piece on each, so what's under a character on its
/// second line is the link too.
pub fn pieces(frame: &Frame) -> Vec<Link> {
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found.sort_by(|a, b| {
        a.top_pt
            .total_cmp(&b.top_pt)
            .then(a.left_pt.total_cmp(&b.left_pt))
    });
    found
}

/// Whether something standing on `baseline_pt` is on `line`, as the page's
/// words are read into lines.
pub fn on_line(baseline_pt: f64, line: &crate::text::Line) -> bool {
    (baseline_pt - line.baseline_pt).abs() < crate::text::SAME_LINE_PT
}

fn walk(frame: &Frame, at: Point, found: &mut Vec<Link>) {
    // Typst sets a link's box in the frame of the text it covers, and a
    // text item stands at its baseline.
    let baseline = frame.items().find_map(|(place, item)| {
        matches!(item, FrameItem::Text(_)).then_some((at.y + place.y).to_pt())
    });
    for (place, item) in frame.items() {
        let here = at + *place;
        match item {
            FrameItem::Group(group) => {
                let moved = Point::new(group.transform.tx, group.transform.ty);
                walk(&group.frame, here + moved, found);
            }
            FrameItem::Link(Destination::Url(url), size) => found.push(Link {
                left_pt: here.x.to_pt(),
                width_pt: size.x.to_pt(),
                top_pt: here.y.to_pt(),
                height_pt: size.y.to_pt(),
                baseline_pt: baseline.unwrap_or((here.y + size.y).to_pt()),
                href: url.as_str().to_string(),
            }),
            _ => {}
        }
    }
}

/// Of what the page names, each with its baseline and where it starts and
/// ends across, the one under a character `x_pt` across `line`, or else the
/// first after it there, as Vim's gf and Ctrl+] take the name under or
/// after the cursor. Of names starting within half a point of each other,
/// as a link and the path its label writes do, the first given is taken.
pub fn under_or_after<T>(
    named: impl IntoIterator<Item = (f64, f64, f64, T)>,
    line: &crate::text::Line,
    x_pt: f64,
) -> Option<T> {
    let mut taken: Option<(f64, T)> = None;
    for (baseline, left, right, item) in named {
        if !on_line(baseline, line) || right <= x_pt {
            continue;
        }
        if taken.as_ref().is_none_or(|(least, _)| left < least - 0.5) {
            taken = Some((left, item));
        }
    }
    taken.map(|(_, item)| item)
}

/// Where a label is drawn, in whole device pixels down the view, so its
/// letters' baseline, `baseline_px` below its top, stands on its link's,
/// on a page `px_per_pt` device pixels a point with the view's top
/// `offset_px` down it.
pub fn label_top(link: &Link, baseline_px: f64, px_per_pt: f64, offset_px: f64) -> f32 {
    f32_of((link.baseline_pt * px_per_pt - offset_px - baseline_px).round())
}

/// Where a link goes.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// A heading on this page, by the name GitHub gives its anchor, or its
    /// top for none.
    Here(String),
    /// Another markdown file, set in this window in this page's place, and
    /// a heading on it, if one is named.
    Page(PathBuf, Option<String>),
    /// Anything else: a site, an address, or a file, opened as the desktop
    /// opens it.
    Elsewhere(String),
}

/// Whether a file's name says it's markdown, as the desktop's type for
/// markdown names its files.
pub fn markdown(file: &Path) -> bool {
    file.extension()
        .and_then(|it| it.to_str())
        .is_some_and(|it| {
            ["md", "markdown", "mdown", "mkd", "mkdn"]
                .iter()
                .any(|ext| it.eq_ignore_ascii_case(ext))
        })
}

/// Where `href` goes, from the page at `page`: a bare fragment, or this
/// page's own name, is here; a path is taken from the page's folder, and
/// one to markdown is a page of its own.
pub fn target(href: &str, page: &Path) -> Target {
    if let Some(fragment) = href.strip_prefix('#') {
        return Target::Here(fragment.to_string());
    }
    if href.contains("://") || href.starts_with("mailto:") {
        return Target::Elsewhere(href.to_string());
    }
    let (path, fragment) = href.split_once('#').unwrap_or((href, ""));
    let path = unescaped(path);
    let path = path.as_str();
    let folder = page.parent().unwrap_or(Path::new("/"));
    let file: PathBuf = if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        folder.join(path)
    };
    if same(&file, page) {
        return Target::Here(fragment.to_string());
    }
    if markdown(&file) {
        let heading = (!fragment.is_empty()).then(|| fragment.to_string());
        return Target::Page(file, heading);
    }
    Target::Elsewhere(file.to_string_lossy().into_owned())
}

/// A path as its link wrote it, its percent escapes read back, as the
/// page's HTML gives a space, `%20`.
fn unescaped(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[at] == b'%'
            && let (Some(Some(high)), Some(Some(low))) = (
                bytes.get(at + 1).map(|b| hex(*b)),
                bytes.get(at + 2).map(|b| hex(*b)),
            )
        {
            out.push(u8::try_from(high * 16 + low).unwrap_or(b'%'));
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether two paths are one file, however each is written.
pub fn same(a: &Path, b: &Path) -> bool {
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
    fn the_name_under_the_character_or_the_next_after_it_on_its_line_is_taken() {
        // Two links on the line set on 40 pt, given out of their order
        // across, and one on the line under it.
        let named = || {
            [
                (41.5, 100.0, 160.0, "second"),
                (40.0, 10.0, 60.0, "first"),
                (61.0, 0.0, 300.0, "under"),
            ]
        };
        let line = crate::text::test_line(40.0, "words");
        assert_eq!(under_or_after(named(), &line, 30.0), Some("first"), "on it");
        assert_eq!(
            under_or_after(named(), &line, 60.0),
            Some("second"),
            "at the first's very end, the next, a superscript's nudge off"
        );
        assert_eq!(
            under_or_after(named(), &line, 5.0),
            Some("first"),
            "before them all, the least across, not the first given"
        );
        assert_eq!(
            under_or_after(named(), &line, 170.0),
            None,
            "past the line's last, none, though a line below has one"
        );
        let together = [(40.0, 10.2, 60.0, "link"), (40.0, 10.0, 60.0, "path")];
        assert_eq!(
            under_or_after(together, &line, 30.0),
            Some("link"),
            "of two starting together, the first given"
        );
    }

    /// A page set from `markdown`, its lines and the pieces of its links.
    fn set(name: &str, markdown: &str) -> (Vec<crate::text::Line>, Vec<Link>) {
        let dir = std::env::temp_dir().join(format!("reader-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let (page, _) = crate::page::Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let lines = crate::text::lines(crate::text::runs(&page.frame));
        (lines, pieces(&page.frame))
    }

    /// What a character takes, links before written paths as gf gives
    /// them: the line that has `words`, at the middle of the character
    /// `at` characters into them.
    fn taken(lines: &[crate::text::Line], pieces: &[Link], words: &str, at: usize) -> String {
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        let line = lines
            .iter()
            .find(|line| line.text.contains(words))
            .unwrap_or_else(|| panic!("no line has {words:?}: {texts:?}"));
        let ch = line.text[..line.text.find(words).expect("found")]
            .chars()
            .count()
            + at;
        let (from, to) = line.spans[ch];
        let links = pieces.iter().map(|link| {
            let right = link.left_pt + link.width_pt;
            (link.baseline_pt, link.left_pt, right, link.href.clone())
        });
        let written = crate::moves::written_paths(line)
            .into_iter()
            .map(|(path, left, right)| (line.baseline_pt, left, right, path));
        under_or_after(links.chain(written), line, f64::midpoint(from, to))
            .unwrap_or_else(|| format!("nothing at {words:?}"))
    }

    #[test]
    fn a_character_on_a_lines_second_link_takes_that_one() {
        let (lines, pieces) = set(
            "two-links",
            "Read [the one](one.md) and then [the other](other.md) here, or notes.md.\n",
        );
        assert_eq!(taken(&lines, &pieces, "the other", 4), "other.md", "on it");
        assert_eq!(
            taken(&lines, &pieces, "the one", 4),
            "one.md",
            "on the first"
        );
        assert_eq!(
            taken(&lines, &pieces, "and then", 0),
            "other.md",
            "between them, the one after"
        );
        assert_eq!(
            taken(&lines, &pieces, "here", 0),
            "notes.md",
            "past the last link, the path written after, never a link behind"
        );
        assert_eq!(
            taken(&lines, &pieces, "notes.md", 8),
            "nothing at \"notes.md\"",
            "on the full stop past it all, nothing"
        );
    }

    #[test]
    fn a_character_on_a_broken_links_second_line_takes_that_link() {
        let (lines, pieces) = set(
            "broken-link",
            &format!(
                "{}[a link long enough to run on past the end of its first line and onto the next one, so it breaks](https://example.com) and [a second](#here).\n",
                "Words before it. ".repeat(5)
            ),
        );
        let ends = lines
            .iter()
            .find(|line| line.text.contains("so it"))
            .expect("the link's end is set");
        assert!(
            !ends.text.contains("a link long"),
            "the link ends on a line below its start: {:?}",
            ends.text
        );
        assert_eq!(
            taken(&lines, &pieces, "so it", 0),
            "https://example.com",
            "the link the character is on, not the next"
        );
    }

    #[test]
    fn a_link_whose_label_is_a_path_is_taken_over_the_path() {
        let (lines, pieces) = set("labelled", "See [docs/design.md](other.md) first.\n");
        assert_eq!(taken(&lines, &pieces, "docs/design.md", 0), "other.md");
        assert_eq!(taken(&lines, &pieces, "docs/design.md", 6), "other.md");
    }

    #[test]
    fn a_paths_escapes_are_read_back() {
        let page = Path::new("/notes/page.md");
        assert_eq!(
            target("/notes/a%20b/%5Bold%5D.md", page),
            Target::Page(PathBuf::from("/notes/a b/[old].md"), None)
        );
        assert_eq!(unescaped("100%"), "100%");
        assert_eq!(unescaped("caf%C3%A9"), "caf\u{e9}");
    }

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
            Target::Page("/notes/other.md".into(), None)
        );
        assert_eq!(
            target("../up.MARKDOWN#top", page),
            Target::Page("/notes/../up.MARKDOWN".into(), Some("top".into()))
        );
        assert_eq!(
            target("page.md#the-keys", page),
            Target::Here("the-keys".into())
        );
        assert_eq!(target("page.md", page), Target::Here(String::new()));
        assert_eq!(
            target("sheet.pdf", page),
            Target::Elsewhere("/notes/sheet.pdf".into())
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
        assert!(
            many.iter().all(|label| label.len() == 2),
            "thirty labels weren't all two letters: {many:?}"
        );
        for (i, a) in many.iter().enumerate() {
            for b in &many[i + 1..] {
                assert!(
                    !b.starts_with(a.as_str()) && !a.starts_with(b.as_str()),
                    "one of {a} and {b} begins the other"
                );
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
