//! The page's links: where each sits on the page and where it goes, and the
//! labels g x puts over the ones in view, as flash puts them in Joel's
//! Neovim, typed to follow one.
use std::path::{Path, PathBuf};

use typst::layout::{Frame, FrameItem, Point};
use typst::model::Destination;

use crate::numbers::f32_of;
use crate::text::Line;

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

/// Whether something standing on `baseline_pt` and starting `left_pt`
/// across is on `line`, as the page's words are read into lines: on its
/// baseline and starting before its end, so in a table row it's the
/// cursor's own cell or one before it, as the paths written there are.
/// What ends before the line starts ends before its characters too, so
/// it's never under or after one.
pub fn on_line(baseline_pt: f64, left_pt: f64, line: &crate::text::Line) -> bool {
    let (_, to) = crate::text::across(line);
    (baseline_pt - line.baseline_pt).abs() < crate::text::SAME_LINE_PT && left_pt < to
}

/// A name gf can take on a line: a link's target, or a path written there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Name {
    Link(String),
    Written(String),
}

impl Name {
    /// The name as the page writes it.
    pub fn said(&self) -> &str {
        let (Name::Link(said) | Name::Written(said)) = self;
        said
    }
}

/// The folder a page's names are read from: its own, or the one the
/// reader started in for a bare file name.
pub fn folder_of(file: &Path) -> &Path {
    file.parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// The link under a character `x_pt` across `line`, or else the first after
/// it there, of the page's link `pieces`: what Ctrl+] follows.
pub fn link_here<'a>(pieces: &'a [Link], line: &crate::text::Line, x_pt: f64) -> Option<&'a Link> {
    let named = pieces.iter().map(|link| {
        let right = link.left_pt + link.width_pt;
        (link.baseline_pt, link.left_pt, right, link)
    });
    under_or_after(named, line, x_pt)
}

/// The name under a character `x_pt` across `line`, or else the first after
/// it there, of the page's link `pieces` and the paths written on the line:
/// what gf takes. Links are given first, so one wins over the path its
/// label writes. A word under the character is taken as written, but one
/// after it only when `is_there` finds it, since words such as e.g. and
/// and/or read as paths too, and gf mustn't stop on them short of a link.
pub fn name_here(
    pieces: &[Link],
    line: &crate::text::Line,
    x_pt: f64,
    is_there: impl Fn(&str) -> bool,
) -> Option<Name> {
    let linked = pieces.iter().map(|link| {
        let right = link.left_pt + link.width_pt;
        (
            link.baseline_pt,
            link.left_pt,
            right,
            Name::Link(link.href.clone()),
        )
    });
    let written = crate::moves::written_paths(line)
        .into_iter()
        .filter(|(path, left, _)| *left <= x_pt || is_there(path))
        .map(|(path, left, right)| (line.baseline_pt, left, right, Name::Written(path)));
    under_or_after(linked.chain(written), line, x_pt)
}

/// Where gf goes for the name it took: a heading on the page shown, which
/// the page's own words say is there or not, or a file `is_file` found, at
/// the heading its link names, if any, as Ctrl+] goes, a link to the page
/// itself with none going to its top; or the page shown itself, its name
/// written, where gf stays, as Vim's does on the file it's editing.
#[derive(Debug, PartialEq)]
pub enum Going {
    Heading(String),
    File(PathBuf, Option<String>),
    Here,
}

/// Where gf goes for `name` from `page`, when it names a heading here or
/// a file on this machine that `is_file` finds.
pub fn going(name: &Name, page: &Path, is_file: impl Fn(&Path) -> bool) -> Option<Going> {
    let (file, heading) = match name {
        Name::Link(href) => match target(href, page) {
            Target::Here(anchor) => return Some(Going::Heading(anchor)),
            Target::Page(file, heading) => (Some(file), heading),
            Target::Elsewhere(_) => (file_of(name, page), None),
        },
        Name::Written(_) => (file_of(name, page), None),
    };
    let file = file.filter(|file| is_file(file))?;
    // Only a written name gets here naming the page shown, since a link
    // to it is a heading here. It's where he is: reopening it would jump
    // to where it was last left.
    if same(&file, page) {
        return Some(Going::Here);
    }
    Some(Going::File(file, heading))
}

/// Where gf goes with a character `x_pt` across `line` on the page `page`:
/// for the name under or after it, when it names a heading here or a file
/// `is_file` finds. The name is settled first and only then asked of the
/// disk, so one naming no file here is said, as Vim's E447, rather than
/// passed for the next; none at all is `Err(None)`, as Vim's E446. The
/// page's own name written after it is no stop, as gf from a line skips
/// it too, since it leads nowhere short of a link further on.
pub fn file_here(
    pieces: &[Link],
    line: &crate::text::Line,
    x_pt: f64,
    page: &Path,
    is_file: impl Fn(&Path) -> bool,
) -> Result<Going, Option<String>> {
    let there = |path: &str| {
        file_of(&Name::Written(path.into()), page).is_some_and(|f| is_file(&f) && !same(&f, page))
    };
    let name = name_here(pieces, line, x_pt, there).ok_or(None)?;
    going(&name, page, &is_file).ok_or_else(|| Some(name.said().to_string()))
}

/// What gf says staying on the page shown, as Vim's says of a file it
/// edits again: its name, and how many lines and bytes `text` takes.
pub fn file_info(page: &Path, text: &str) -> String {
    let name = page
        .file_name()
        .unwrap_or(page.as_os_str())
        .to_string_lossy();
    let unended = usize::from(!text.is_empty() && !text.ends_with('\n'));
    let lines = text.matches('\n').count() + unended;
    format!("\"{name}\" {lines}L, {}B", text.len())
}

/// Where gf goes from a whole line, `from_pt` down the page, in a view
/// ending at `foot_pt`: by the first of `linked` from there that names
/// another file `is_file` finds, or the first path written on `lines`
/// from there that is one, a line's links before its paths.
pub fn going_down(
    linked: Vec<Link>,
    lines: &[Line],
    (from_pt, foot_pt): (f64, f64),
    page: &Path,
    is_file: impl Fn(&Path) -> bool,
) -> Option<Going> {
    let mut named: Vec<(f64, f64, Going)> = linked
        .into_iter()
        .filter(|link| link.baseline_pt > from_pt && link.top_pt < foot_pt)
        .filter(|link| names_another_file(&link.href, page))
        .filter_map(|link| {
            let going = going(&Name::Link(link.href), page, &is_file)?;
            Some((link.baseline_pt.round(), link.left_pt, going))
        })
        .collect();
    for line in lines {
        if line.baseline_pt <= from_pt || crate::moves::foot_of(line) > foot_pt {
            continue;
        }
        for (path, ..) in crate::moves::written_paths(line) {
            // The page's own name, written, is no other file.
            if let Some(going @ Going::File(..)) = going(&Name::Written(path), page, &is_file) {
                named.push((line.baseline_pt.round(), f64::INFINITY, going));
            }
        }
    }
    named.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    named.into_iter().next().map(|(.., going)| going)
}

/// Whether gf from a whole line takes the link to `href`: one to a heading
/// on `page` is Ctrl+]'s to go to, and gf from a line wants another file.
pub fn names_another_file(href: &str, page: &Path) -> bool {
    !matches!(target(href, page), Target::Here(_))
}

/// The file a name stands for, from the page `page` in its folder, when it
/// names one on this machine at all: an address, a site's or a mail's,
/// names none, and a
/// heading on the page names the page. A written path from ~/ is from the
/// home folder, as a shell reads it. Whether the file is there is for the
/// disk to say.
pub fn file_of(name: &Name, page: &Path) -> Option<PathBuf> {
    match name {
        Name::Link(href) => match target(href, page) {
            Target::Here(_) => Some(page.to_path_buf()),
            Target::Page(file, _) => Some(file),
            Target::Elsewhere(to) if !address(&to) => Some(to.into()),
            Target::Elsewhere(_) => None,
        },
        Name::Written(path) => match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
            (Some(rest), Some(home)) => Some(PathBuf::from(home).join(rest)),
            _ => Some(folder_of(page).join(path)),
        },
    }
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
        if !on_line(baseline, left, line) || right <= x_pt {
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
    if address(href) {
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

/// Whether `href` is an address, a site's or a mail's, rather than a path.
fn address(href: &str) -> bool {
    href.contains("://") || href.starts_with("mailto:")
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
#[path = "links_tests.rs"]
mod tests;
