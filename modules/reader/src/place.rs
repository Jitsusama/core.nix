//! Where he was in a file, kept when the reader closes and taken back when
//! it opens the file again, as Neovim takes back where its cursor was. A
//! place is kept as a block of the file and a distance into it, so the
//! first set reaches it without setting the whole file, and it holds when
//! the file has changed above it.
//!
//! Beside it is kept what a person can read of it, since a block's number
//! means nothing to one and the headings exist only while the page is set:
//! the heading it sits under, how far down the page it is, and when it was
//! left. The launcher lists them (`listed`, which `reader --places` prints)
//! for the reader's Recent, so nothing there sets a page to say where.
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use typst::foundations::Value;
use typst::introspection::{MetadataElem, Tag};
use typst::layout::{Frame, FrameItem, Point};

/// A place on a page: the block the view's top is in, how far into it in
/// points, and whether the view was at the page's end.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub block: usize,
    pub into_pt: f64,
    pub at_end: bool,
    /// Each letter's mark, kept the same way.
    pub marks: Vec<Mark>,
    /// Where it is, as a person reads it.
    pub said: Said,
    /// When it was left, in seconds since 1970. A place kept before this
    /// was written has none, and its file's own time stands for it.
    pub left: Option<u64>,
}

/// A letter's mark, kept as a place is, and said as one is.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub letter: char,
    pub block: usize,
    pub into_pt: f64,
    pub said: Said,
}

/// Where a place is, as a person reads it: the heading it sits under, in
/// the words gO's outline gives it, and how far down the page it is, from
/// 0 at the top to 1 at the end. Either may be missing: a place above the
/// first heading sits under none, and a place kept before the whole page
/// was set can't say how far down a page of unknown height it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Said {
    pub heading: Option<String>,
    pub through: Option<f64>,
}

/// A block of the page, as its metadata says it (walk/): its number,
/// where it starts, the source lines it was written on, and a heading's
/// level and words.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub number: usize,
    pub top_pt: f64,
    pub lines: Option<(usize, usize)>,
    pub heading: Option<(u8, String)>,
}

/// Where each block starts on a page, by its number, top to bottom.
pub fn starts(frame: &Frame) -> Vec<(usize, f64)> {
    blocks(frame)
        .into_iter()
        .map(|block| (block.number, block.top_pt))
        .collect()
}

/// Every block of a page, top to bottom.
pub fn blocks(frame: &Frame) -> Vec<Block> {
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found.sort_by(|a, b| a.top_pt.total_cmp(&b.top_pt));
    found
}

fn walk(frame: &Frame, at: Point, found: &mut Vec<Block>) {
    for (place, item) in frame.items() {
        let here = at + *place;
        match item {
            FrameItem::Group(group) => {
                let moved = Point::new(group.transform.tx, group.transform.ty);
                walk(&group.frame, here + moved, found);
            }
            FrameItem::Tag(Tag::Start(content, _)) => {
                if let Some(metadata) = content.to_packed::<MetadataElem>()
                    && let Some(block) = said(&metadata.value, here.y.to_pt())
                {
                    found.push(block);
                }
            }
            _ => {}
        }
    }
}

/// A block's metadata read back: a dictionary of its number, lines and
/// heading, or its number alone.
fn said(value: &Value, top_pt: f64) -> Option<Block> {
    let number = |value: &Value| match value {
        Value::Int(n) => usize::try_from(*n).ok(),
        _ => None,
    };
    match value {
        Value::Int(_) => Some(Block {
            number: number(value)?,
            top_pt,
            lines: None,
            heading: None,
        }),
        Value::Dict(dict) => {
            let lines = match dict.get("lines").ok() {
                Some(Value::Array(pair)) => match pair.as_slice() {
                    [from, to] => number(from).zip(number(to)),
                    _ => None,
                },
                _ => None,
            };
            let heading = match dict.get("heading").ok() {
                Some(Value::Array(pair)) => match pair.as_slice() {
                    [level, Value::Str(words)] => number(level)
                        .and_then(|level| u8::try_from(level).ok())
                        .map(|level| (level, words.as_str().to_string())),
                    _ => None,
                },
                _ => None,
            };
            Some(Block {
                number: number(dict.get("block").ok()?)?,
                top_pt,
                lines,
                heading,
            })
        }
        _ => None,
    }
}

/// The block a point is in, and how far into it: the last block starting
/// at or above it.
pub fn at(starts: &[(usize, f64)], pt: f64) -> (usize, f64) {
    starts
        .iter()
        .rev()
        .find(|(_, top)| *top <= pt)
        .map_or((0, pt), |(block, top)| (*block, pt - top))
}

/// A letter's mark as it's kept: the block its line is in and how far the
/// line's top sits from the block's, with the heading it's under and how
/// far down the page it is. `line` is the line's top and baseline, in
/// points, and the page's height is known only of the whole file's page.
///
/// A line is in the block its baseline is in, as ]] finds a heading's
/// line (moves.rs): a heading's block starts at its capitals, below its
/// line's top, so a mark found by its top alone was kept under the
/// heading before. Its distance into its block is then less than nothing,
/// which reaches the same top again. And a mark is a point, not a view,
/// so how far down it is is its line's share of the page's whole height:
/// a view's share of how far it can go reads every mark on the last
/// screen as all of it.
pub fn mark(blocks: &[Block], letter: char, line: (f64, f64), height_pt: Option<f64>) -> Mark {
    let starts: Vec<(usize, f64)> = blocks
        .iter()
        .map(|block| (block.number, block.top_pt))
        .collect();
    let (top, baseline) = line;
    let (block, below_its_top) = at(&starts, baseline);
    Mark {
        letter,
        block,
        into_pt: below_its_top - (baseline - top),
        said: Said {
            heading: heading_of(blocks, block),
            through: height_pt.map(|height| share(top, height)),
        },
    }
}

/// How far down a page a point is, as a share of the page's whole height.
/// A page of no height has nothing to be through.
fn share(pt: f64, height_pt: f64) -> f64 {
    if height_pt <= 0.0 {
        0.0
    } else {
        (pt / height_pt).clamp(0.0, 1.0)
    }
}

/// The heading a block sits under: the nearest heading at or before it in
/// the page's order, in the words gO's outline gives it, or none above the
/// first. Blocks are as `blocks` gives them, top to bottom.
pub fn heading_of(blocks: &[Block], block: usize) -> Option<String> {
    let at = blocks.iter().position(|each| each.number == block)?;
    blocks[..=at]
        .iter()
        .rev()
        .find_map(|each| each.heading.as_ref().map(|(_, words)| words.clone()))
}

/// How far down the page a view's top at `pt` is, as a share of the furthest
/// it can go, `most_pt`: the share a scrollbar's thumb would stand at, so
/// the page's end is the whole of it. A page no taller than its window is
/// read whole wherever its view is.
pub fn through(pt: f64, most_pt: f64) -> f64 {
    if most_pt <= 0.0 {
        1.0
    } else {
        (pt / most_pt).clamp(0.0, 1.0)
    }
}

/// The letter `--at` names a mark by: one letter alone, as m keeps one.
pub fn letter(said: &str) -> Option<char> {
    let mut letters = said.chars();
    match (letters.next(), letters.next()) {
        (Some(letter), None) if letter.is_alphabetic() => Some(letter),
        _ => None,
    }
}

/// Where a file opens: the block the view's top is in, how far into it,
/// and whether at the page's end, from the mark kept under `letter` when
/// one is asked for, or else where it was left; and the letter asked for,
/// when no mark is kept under it, to be said.
pub fn opening(
    kept: Option<&Place>,
    letter: Option<char>,
) -> (Option<(usize, f64, bool)>, Option<char>) {
    let left = kept.map(|place| (place.block, place.into_pt, place.at_end));
    let Some(letter) = letter else {
        return (left, None);
    };
    match kept.and_then(|place| place.marks.iter().find(|mark| mark.letter == letter)) {
        Some(mark) => (Some((mark.block, mark.into_pt, false)), None),
        None => (left, Some(letter)),
    }
}

/// Seconds since 1970 now, as a place says when it was left.
pub fn now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// Where a block and a distance into it are on a page, if the page has the
/// block.
pub fn pt_of(starts: &[(usize, f64)], block: usize, into_pt: f64) -> Option<f64> {
    starts
        .iter()
        .find(|(b, _)| *b == block)
        .map(|(_, top)| top + into_pt)
}

/// The state home, where places are kept.
fn state() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
}

/// The file a file's place is kept in: one per file, named for its whole
/// path, in the reader's state.
fn kept_at(state: &Path, file: &Path) -> Option<PathBuf> {
    let file = std::fs::canonicalize(file).ok()?;
    let mut name = DefaultHasher::new();
    file.hash(&mut name);
    Some(
        state
            .join("reader")
            .join("places")
            .join(format!("{:016x}.json", name.finish())),
    )
}

/// The place kept for a file, if there is one.
pub fn read(file: &Path) -> Option<Place> {
    read_in(&state()?, file)
}

fn read_in(state: &Path, file: &Path) -> Option<Place> {
    let text = std::fs::read(kept_at(state, file)?).ok()?;
    from_json(&serde_json::from_slice(&text).ok()?)
}

/// A place as it's kept, read back. Only its block and the way into it are
/// needed, so a place kept before the rest was written still reads.
fn from_json(kept: &serde_json::Value) -> Option<Place> {
    let block = usize::try_from(kept.get("block")?.as_u64()?).ok()?;
    let marks = kept
        .get("marks")
        .and_then(serde_json::Value::as_array)
        .map(|marks| {
            marks
                .iter()
                .filter_map(|mark| {
                    Some(Mark {
                        letter: mark.get("letter")?.as_str()?.chars().next()?,
                        block: usize::try_from(mark.get("block")?.as_u64()?).ok()?,
                        into_pt: mark.get("into_pt")?.as_f64()?,
                        said: said_of(mark),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Place {
        block,
        into_pt: kept.get("into_pt")?.as_f64()?,
        at_end: kept
            .get("at_end")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        marks,
        said: said_of(kept),
        left: kept.get("left").and_then(serde_json::Value::as_u64),
    })
}

/// What a place or a mark says of where it is, whatever of it was kept.
fn said_of(kept: &serde_json::Value) -> Said {
    Said {
        heading: kept
            .get("heading")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        through: kept
            .get("through")
            .and_then(serde_json::Value::as_f64)
            .filter(|share| share.is_finite()),
    }
}

/// A place's or a mark's words, beside the fields it's found by: only
/// what's known is written, so the launcher never reads a guess.
fn with_said(mut fields: serde_json::Value, said: &Said) -> serde_json::Value {
    if let Some(heading) = &said.heading {
        fields["heading"] = serde_json::json!(heading);
    }
    if let Some(through) = said.through {
        fields["through"] = serde_json::json!(through);
    }
    fields
}

/// Keeps a file's place, whole or not at all; never the page of what to
/// read's, which is written fresh each time.
pub fn write(file: &Path, place: &Place) -> std::io::Result<()> {
    if crate::start::is(file) {
        return Ok(());
    }
    match state() {
        Some(state) => write_in(&state, file, place),
        None => Ok(()),
    }
}

fn write_in(state: &Path, file: &Path, place: &Place) -> std::io::Result<()> {
    let Some(kept) = kept_at(state, file) else {
        return Ok(());
    };
    let marks: Vec<serde_json::Value> = place
        .marks
        .iter()
        .map(|mark| {
            let fields = serde_json::json!({
                "letter": mark.letter.to_string(),
                "block": mark.block,
                "into_pt": mark.into_pt,
            });
            with_said(fields, &mark.said)
        })
        .collect();
    let mut fields = serde_json::json!({
        "file": std::fs::canonicalize(file)?.to_string_lossy(),
        "block": place.block,
        "into_pt": place.into_pt,
        "at_end": place.at_end,
        "marks": marks,
    });
    if let Some(left) = place.left {
        fields["left"] = serde_json::json!(left);
    }
    let text = with_said(fields, &place.said);
    if let Some(folder) = kept.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let fresh = kept.with_extension(format!("{}.part", std::process::id()));
    std::fs::write(&fresh, text.to_string())?;
    std::fs::rename(&fresh, &kept)
}

/// The most places the launcher is given: twice its Recent's nine, since
/// Recent leaves out the files open now, which are few, and a place past
/// its ninth is never seen.
const LISTED: usize = 18;

/// Every place kept of a file that's still there, newest left first, the
/// most the launcher shows, each as a line of JSON: the file, when it was
/// left in seconds since 1970, whether at its end, the heading it sits
/// under and how far down it is where those are known, and each mark, by
/// its letter, said the same way. The launcher runs it as the reader is
/// stepped into, once an opening, so no key reads a folder.
pub fn listed() -> Vec<String> {
    state().map_or_else(Vec::new, |state| listed_in(&state, LISTED))
}

fn listed_in(state: &Path, most: usize) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(state.join("reader").join("places")) else {
        return Vec::new();
    };
    let mut found: Vec<(u64, String, Place)> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ending| ending == "json")
        })
        .filter_map(|entry| {
            let kept: serde_json::Value =
                serde_json::from_slice(&std::fs::read(entry.path()).ok()?).ok()?;
            let file = kept.get("file")?.as_str()?.to_string();
            if !Path::new(&file).is_file() {
                return None;
            }
            let place = from_json(&kept)?;
            // A place kept before it said when was written as it was left.
            let left = place.left.or_else(|| {
                let modified = entry.metadata().ok()?.modified().ok()?;
                Some(modified.duration_since(UNIX_EPOCH).ok()?.as_secs())
            })?;
            Some((left, file, place))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found
        .into_iter()
        .take(most)
        .map(|(left, file, place)| {
            let marks: Vec<serde_json::Value> = place
                .marks
                .iter()
                .map(|mark| {
                    with_said(
                        serde_json::json!({"letter": mark.letter.to_string()}),
                        &mark.said,
                    )
                })
                .collect();
            let fields = serde_json::json!({
                "file": file,
                "left": left,
                "at_end": place.at_end,
                "marks": marks,
            });
            with_said(fields, &place.said).to_string()
        })
        .collect()
}

/// How wide the window was when the reader last closed, in points of the
/// page at zoom 1, so the first screen is set for the window it will most
/// likely open in again. One for the reader, since niri gives its window
/// the same column whatever file it shows.
pub fn window_pt() -> Option<f64> {
    window_in(&state()?)
}

fn window_in(state: &Path) -> Option<f64> {
    std::fs::read_to_string(state.join("reader").join("window"))
        .ok()?
        .trim()
        .parse()
        .ok()
        .filter(|pt: &f64| pt.is_finite() && *pt > 0.0)
}

/// Keeps how wide the window is, for the next time the reader opens.
pub fn keep_window(pt: f64) -> std::io::Result<()> {
    match state() {
        Some(state) => keep_window_in(&state, pt),
        None => Ok(()),
    }
}

fn keep_window_in(state: &Path, pt: f64) -> std::io::Result<()> {
    let folder = state.join("reader");
    std::fs::create_dir_all(&folder)?;
    let fresh = folder.join(format!("window.{}.part", std::process::id()));
    std::fs::write(&fresh, format!("{pt}\n"))?;
    std::fs::rename(&fresh, folder.join("window"))
}

#[cfg(test)]
mod tests {
    use super::{
        Block, Mark, Place, Said, at, heading_of, keep_window_in, kept_at, letter, listed_in, mark,
        opening, pt_of, read_in, through, window_in, write_in,
    };

    /// A block of a page, a heading's words given.
    fn block(number: usize, heading: Option<&str>) -> Block {
        Block {
            number,
            top_pt: f64::from(u32::try_from(number).unwrap_or(0)) * 100.0,
            lines: None,
            heading: heading.map(|words| (2, words.to_string())),
        }
    }

    #[test]
    fn a_place_sits_under_the_nearest_heading_at_or_before_it() {
        let blocks = [
            block(0, None),
            block(1, Some("Installing")),
            block(2, None),
            block(3, Some("Using It")),
        ];
        assert_eq!(heading_of(&blocks, 2).as_deref(), Some("Installing"));
        assert_eq!(
            heading_of(&blocks, 3).as_deref(),
            Some("Using It"),
            "a heading's own block is under it"
        );
        assert_eq!(
            heading_of(&blocks, 0),
            None,
            "above the first heading, none"
        );
        assert_eq!(heading_of(&blocks, 9), None, "a block the page lacks, none");
    }

    #[test]
    fn a_mark_on_a_headings_line_is_kept_under_that_heading() {
        // A heading's block starts at its capitals, a few points below its
        // line's top, which is where the cursor's line and so the mark are.
        let blocks = [
            block(0, Some("A Page")),
            block(1, None),
            block(2, Some("Further On")),
        ];
        let kept = mark(&blocks, 'a', (196.0, 210.0), Some(1000.0));
        assert_eq!(kept.said.heading.as_deref(), Some("Further On"), "{kept:?}");
        assert_eq!(kept.block, 2, "{kept:?}");
        assert_eq!(
            pt_of(
                &[(0, 0.0), (1, 100.0), (2, 200.0)],
                kept.block,
                kept.into_pt
            ),
            Some(196.0),
            "it's kept where its line's top is"
        );
        let words = mark(&blocks, 'b', (120.0, 134.0), Some(1000.0));
        assert_eq!(
            (words.block, words.said.heading.as_deref()),
            (1, Some("A Page")),
            "a line of words under a heading is in its own block"
        );
    }

    #[test]
    fn a_mark_says_its_lines_share_of_the_whole_page() {
        // The view goes no further than 700 down a 1000-point page, so a
        // view's share of 950 would be all of it; a mark there is a point.
        let blocks = [block(0, None), block(9, None)];
        let kept = mark(&blocks, 'a', (950.0, 964.0), Some(1000.0));
        let share = kept
            .said
            .through
            .expect("a whole page's mark says how far down");
        assert!((share - 0.95).abs() < 1e-9, "{share} isn't 950 of 1000");
        assert_eq!(
            mark(&blocks, 'a', (950.0, 964.0), None).said.through,
            None,
            "a page not yet set whole can't say"
        );
    }

    #[test]
    fn how_far_down_is_a_share_of_the_furthest_the_view_goes() {
        assert!(
            (through(400.0, 1000.0) - 0.4).abs() < 1e-9,
            "400 down of 1000 is two fifths"
        );
        assert!(
            (through(1000.0, 1000.0) - 1.0).abs() < 1e-9,
            "the end is the whole"
        );
        assert!(
            (through(1200.0, 1000.0) - 1.0).abs() < 1e-9,
            "never past the whole"
        );
        assert!(
            (through(0.0, 0.0) - 1.0).abs() < 1e-9,
            "a page within its window is read"
        );
    }

    #[test]
    fn a_file_opens_at_the_mark_asked_for_or_where_it_was_left() {
        let place = Place {
            block: 9,
            into_pt: 4.0,
            at_end: true,
            marks: vec![Mark {
                letter: 'a',
                block: 3,
                into_pt: 7.0,
                said: Said::default(),
            }],
            said: Said::default(),
            left: None,
        };
        assert_eq!(
            opening(Some(&place), Some('a')),
            (Some((3, 7.0, false)), None)
        );
        assert_eq!(
            opening(Some(&place), Some('b')),
            (Some((9, 4.0, true)), Some('b')),
            "no mark b, so where it was left, saying so"
        );
        assert_eq!(opening(Some(&place), None), (Some((9, 4.0, true)), None));
        assert_eq!(opening(None, Some('a')), (None, Some('a')), "nothing kept");
        assert_eq!(letter("a"), Some('a'));
        assert_eq!(letter("Q"), Some('Q'));
        assert_eq!(letter("ab"), None, "one letter alone");
        assert_eq!(letter("1"), None);
        assert_eq!(letter(""), None);
    }

    #[test]
    fn a_place_kept_before_it_said_where_still_reads() {
        let dir = std::env::temp_dir().join(format!("reader-old-place-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, "# A page").expect("the file");
        let state = dir.join("state");
        let kept = kept_at(&state, &file).expect("where it's kept");
        std::fs::create_dir_all(kept.parent().expect("its folder")).expect("made");
        let old = serde_json::json!({
            "file": file.to_string_lossy(),
            "block": 4,
            "into_pt": 2.5,
            "at_end": false,
            "marks": [{"letter": "a", "block": 1, "into_pt": 0.0}],
        });
        std::fs::write(&kept, old.to_string()).expect("kept");
        let place = read_in(&state, &file).expect("read");
        assert_eq!(
            (place.block, place.left, place.said),
            (4, None, Said::default())
        );
        assert_eq!(place.marks.len(), 1);
        assert_eq!(place.marks[0].said, Said::default());
        let listed = listed_in(&state, 9);
        assert_eq!(listed.len(), 1, "listed by its file's time");
        let line: serde_json::Value = serde_json::from_str(&listed[0]).expect("JSON");
        assert!(line["left"].as_u64().is_some_and(|left| left > 0), "{line}");
        assert!(
            line.get("heading").is_none() && line.get("through").is_none(),
            "{line}"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn the_places_are_listed_newest_left_first_without_a_file_gone() {
        let dir = std::env::temp_dir().join(format!("reader-listed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let state = dir.join("state");
        let keep = |name: &str, left: u64, heading: Option<&str>| {
            let file = dir.join(name);
            std::fs::write(&file, "# A page").expect("the file");
            let said = Said {
                heading: heading.map(str::to_string),
                through: Some(0.4),
            };
            let place = Place {
                block: 2,
                into_pt: 0.0,
                at_end: false,
                marks: vec![Mark {
                    letter: 'b',
                    block: 1,
                    into_pt: 0.0,
                    said: said.clone(),
                }],
                said,
                left: Some(left),
            };
            write_in(&state, &file, &place).expect("kept");
            file
        };
        keep("older.md", 100, None);
        keep("newer.md", 300, Some("Installing"));
        let gone = keep("gone.md", 200, None);
        std::fs::remove_file(gone).expect("gone");
        let listed: Vec<serde_json::Value> = listed_in(&state, 9)
            .iter()
            .map(|line| serde_json::from_str(line).expect("JSON"))
            .collect();
        let files: Vec<&str> = listed
            .iter()
            .filter_map(|line| line["file"].as_str()?.rsplit('/').next())
            .collect();
        assert_eq!(files, ["newer.md", "older.md"]);
        assert_eq!(listed[0]["heading"], "Installing");
        assert_eq!(listed[0]["through"], 0.4);
        assert_eq!(listed[0]["left"], 300);
        assert_eq!(listed[0]["marks"][0]["letter"], "b");
        assert_eq!(listed[0]["marks"][0]["heading"], "Installing");
        assert!(listed[1].get("heading").is_none(), "no heading, none said");
        assert_eq!(listed_in(&state, 1).len(), 1, "no more than asked");
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn the_windows_width_is_kept_and_taken_back() {
        let state = std::env::temp_dir().join(format!("reader-window-{}", std::process::id()));
        assert_eq!(window_in(&state), None, "nothing kept yet");
        keep_window_in(&state, 360.0).expect("kept");
        assert_eq!(window_in(&state), Some(360.0));
        std::fs::remove_dir_all(&state).expect("cleaned");
    }

    #[test]
    fn a_place_is_the_last_block_starting_above_it_and_the_way_into_it() {
        let starts = [(0, 36.0), (1, 120.0), (2, 400.0)];
        assert_eq!(at(&starts, 130.0), (1, 10.0));
        assert_eq!(at(&starts, 400.0), (2, 0.0));
        assert_eq!(
            at(&starts, 10.0),
            (0, 10.0),
            "above the first block is the page's top"
        );
        assert_eq!(pt_of(&starts, 1, 10.0), Some(130.0));
        assert_eq!(
            pt_of(&starts, 7, 0.0),
            None,
            "a block the page lacks has no place"
        );
    }

    #[test]
    fn a_place_kept_is_the_place_read_back() {
        let dir = std::env::temp_dir().join(format!("reader-place-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, "# A page").expect("the file");
        let state = dir.join("state");
        let place = Place {
            block: 12,
            into_pt: 40.5,
            at_end: false,
            marks: vec![Mark {
                letter: 'a',
                block: 3,
                into_pt: 7.0,
                said: Said {
                    heading: Some("Before".to_string()),
                    through: Some(0.1),
                },
            }],
            said: Said {
                heading: Some("Installing".to_string()),
                through: Some(0.4),
            },
            left: Some(1_700_000_000),
        };
        write_in(&state, &file, &place).expect("kept");
        assert_eq!(read_in(&state, &file), Some(place));
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }
}
