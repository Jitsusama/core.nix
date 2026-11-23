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
//!
//! Long because a mark kept across widths, saves and closes is decided
//! here in pure functions its tests reach, apart from the window.
use std::collections::HashMap;
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
    /// How tall its block was where it was kept, so a page set at another
    /// width finds it as far through the block; a place kept before this
    /// was written, or in the page's last block, has none.
    pub block_pt: Option<f64>,
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
    pub block_pt: Option<f64>,
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
    let mut found: Vec<Block> = marks(frame)
        .into_iter()
        .filter_map(|(top_pt, value)| said(value, top_pt))
        .collect();
    found.sort_by(|a, b| a.top_pt.total_cmp(&b.top_pt));
    found
}

/// Every metadata mark the walk left on a page, with its top in points, in
/// the frame's order: the blocks' marks and visual mode's pieces' alike.
pub fn marks(frame: &Frame) -> Vec<(f64, &Value)> {
    fn walk<'a>(frame: &'a Frame, at: Point, found: &mut Vec<(f64, &'a Value)>) {
        for (place, item) in frame.items() {
            let here = at + *place;
            match item {
                FrameItem::Group(group) => {
                    let moved = Point::new(group.transform.tx, group.transform.ty);
                    walk(&group.frame, here + moved, found);
                }
                FrameItem::Tag(Tag::Start(content, _)) => {
                    if let Some(metadata) = content.to_packed::<MetadataElem>() {
                        found.push((here.y.to_pt(), &metadata.value));
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found
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
/// at or above it. A point over every block, as the eyebrow's line is,
/// stands that far over the first, a distance less than nothing that
/// keeps it as far above the first block wherever that starts, as what
/// stands over the page's words moves with them.
pub fn at(starts: &[(usize, f64)], pt: f64) -> (usize, f64) {
    starts
        .iter()
        .rev()
        .find(|(_, top)| *top <= pt)
        .or(starts.first())
        .map_or((0, pt), |(block, top)| (*block, pt - top))
}

/// The block a line is in, by its baseline, and how far its top stands
/// from the block's: `line` is its top and baseline in points. A
/// heading's block starts at its capitals, below its line's top, so a
/// line found by its top alone is the block's before, and its distance
/// into its own is less than nothing, which reaches the same top again.
pub fn line_at(starts: &[(usize, f64)], line: (f64, f64)) -> (usize, f64) {
    let (top, baseline) = line;
    let (block, below_its_top) = at(starts, baseline);
    (block, below_its_top - (baseline - top))
}

/// A letter's mark as it's kept: the block its line is in and how far the
/// line's top sits from the block's, with the heading it's under and how
/// far down the page it is. `line` is the line's top and baseline, in
/// points, and the page's height is known only of the whole file's page.
///
/// A line is in the block its baseline is in, as ]] finds a heading's
/// line (moves.rs), by `line_at`. And a mark is a point, not a view,
/// so how far down it is is its line's share of the page's whole height:
/// a view's share of how far it can go reads every mark on the last
/// screen as all of it.
pub fn mark(blocks: &[Block], letter: char, line: (f64, f64), height_pt: Option<f64>) -> Mark {
    let starts: Vec<(usize, f64)> = blocks
        .iter()
        .map(|block| (block.number, block.top_pt))
        .collect();
    let (block, into_pt) = line_at(&starts, line);
    Mark {
        letter,
        block,
        into_pt,
        block_pt: height_of(&starts, block),
        said: Said {
            heading: heading_of(blocks, block),
            through: height_pt.map(|height| share(line.0, height)),
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
/// how tall the block stood when kept, and whether at the page's end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opens {
    pub block: usize,
    pub into_pt: f64,
    pub block_pt: Option<f64>,
    pub at_end: bool,
}

impl Opens {
    /// Where on a page whose blocks start at `starts` the view opens, in
    /// points. Kept at the end, it opens at the end, however the window or
    /// the page has changed since: the first frame clamps it there.
    pub fn at(self, starts: &[(usize, f64)]) -> Option<f64> {
        if self.at_end {
            Some(f64::INFINITY)
        } else {
            pt_of(starts, self.block, self.into_pt, self.block_pt)
        }
    }
}

/// Where a file opens, from the mark kept under `letter` when one is
/// asked for, or else where it was left; and the letter asked for, when
/// no mark is kept under it, to be said.
pub fn opening(kept: Option<&Place>, letter: Option<char>) -> (Option<Opens>, Option<char>) {
    let left = kept.map(|place| Opens {
        block: place.block,
        into_pt: place.into_pt,
        block_pt: place.block_pt,
        at_end: place.at_end,
    });
    let Some(letter) = letter else {
        return (left, None);
    };
    match kept.and_then(|place| place.marks.iter().find(|mark| mark.letter == letter)) {
        Some(mark) => (
            Some(Opens {
                block: mark.block,
                into_pt: mark.into_pt,
                block_pt: mark.block_pt,
                at_end: false,
            }),
            None,
        ),
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
/// block. Kept where the block stood `block_pt` tall, and set now at
/// another height, as a page set at another width sets it, the distance
/// is taken as far through it, so a place on its fourth line of eight
/// lands on the middle of it however its lines wrap now. A distance less
/// than nothing is a heading's line standing over its capitals, which its
/// type sets and no wrapping changes, so it's kept as it is.
pub fn pt_of(
    starts: &[(usize, f64)],
    block: usize,
    into_pt: f64,
    block_pt: Option<f64>,
) -> Option<f64> {
    let top = starts.iter().find(|(b, _)| *b == block)?.1;
    let into_pt = match (block_pt, height_of(starts, block)) {
        (Some(was), Some(now))
            if was >= LEAST_BLOCK_PT && into_pt > 0.0 && (now - was).abs() >= 0.01 =>
        {
            // No place stands past its block's foot, so a distance kept
            // further is a file gone wrong, held to the block.
            into_pt.min(was) * now / was
        }
        _ => into_pt,
    };
    Some(top + into_pt)
}

/// Where a mark kept in `block`, `into_pt` into it when it stood
/// `block_pt` tall, lands on a page whose blocks start at `starts`, or
/// none while it waits: for its block, and on the first screen's page,
/// not `whole`, for the block after it too when it was kept with its
/// height, since only that says how tall its block stands now.
pub fn mark_at(
    starts: &[(usize, f64)],
    whole: bool,
    block: usize,
    into_pt: f64,
    block_pt: Option<f64>,
) -> Option<f64> {
    if !whole && block_pt.is_some() && height_of(starts, block).is_none() {
        return None;
    }
    pt_of(starts, block, into_pt, block_pt)
}

/// The marks kept from the last time to write back as they were read:
/// those still waiting on the first screen's page, whose letter no mark
/// set since has taken, so closing before the whole page arrives loses
/// none. On the whole page a mark left waiting has lost its block, as a
/// file cut shorter loses it, so it goes.
pub fn still_waiting(kept: &[Mark], whole: bool, set: impl Fn(char) -> bool) -> Vec<Mark> {
    if whole {
        return Vec::new();
    }
    kept.iter()
        .filter(|mark| !set(mark.letter))
        .cloned()
        .collect()
}

/// The least height a kept block is believed to have stood: a line of the
/// smallest type is taller, so one kept less is a file gone wrong, whose
/// share would throw the place far down the page.
const LEAST_BLOCK_PT: f64 = 1.0;

/// How tall a block stands, from its top to the next block's; the page's
/// last block, or one the page lacks, says none, as does one standing at
/// its next's top, with nothing to be through.
pub fn height_of(starts: &[(usize, f64)], block: usize) -> Option<f64> {
    let at = starts.iter().position(|(b, _)| *b == block)?;
    let next = starts.get(at + 1)?;
    Some(next.1 - starts[at].1).filter(|pt| *pt >= LEAST_BLOCK_PT)
}

/// Where a line, its top and baseline, on a page set `before` lands on
/// the same page set again `after`, at another width: as far through the
/// same block, and then on the line of that block nearest it, of the
/// tops and baselines `lines_after`, or between them while it has none.
pub fn moved(
    before: &[(usize, f64)],
    after: &[(usize, f64)],
    line: (f64, f64),
    lines_after: &[(f64, f64)],
) -> Option<f64> {
    let (block, into_pt) = line_at(before, line);
    let pt = pt_of(after, block, into_pt, height_of(before, block))?;
    Some(line_in(after, block, pt, lines_after).unwrap_or(pt))
}

/// The top of the line in `block` nearest `pt`, of `lines` given by their
/// tops and baselines. A place landed as far through its block falls
/// between its lines, and the line nearest it is the one it was on, but
/// only among its own block's: past its last line the nearest may be the
/// next block's heading, and over its first the line above it the block
/// before's.
pub fn line_in(
    starts: &[(usize, f64)],
    block: usize,
    pt: f64,
    lines: &[(f64, f64)],
) -> Option<f64> {
    // A line is in the block its baseline is in, as `line_at` finds it:
    // from the block's top to the next's, and for the first block from
    // over the page too. Found once, so a resize on a long page asks each
    // line once rather than searching every block for it.
    let at = starts.iter().position(|(b, _)| *b == block)?;
    let from = if at == 0 {
        f64::NEG_INFINITY
    } else {
        starts[at].1
    };
    let to = starts.get(at + 1).map_or(f64::INFINITY, |next| next.1);
    lines
        .iter()
        .filter(|(_, baseline)| (from..to).contains(baseline))
        .map(|(top, _)| *top)
        .min_by(|a, b| (a - pt).abs().total_cmp(&(b - pt).abs()))
}

/// A kept mark taken back onto the page: as it was read, where it stands
/// now, and whether that's on a line of its block yet. While the mark
/// stays where it landed it is the kept one, which a resize lands again
/// from what was kept and closing writes back as it was read, so landing
/// it costs nothing it was kept with.
#[derive(Clone, Debug, PartialEq)]
pub struct Landed {
    pub kept: Mark,
    pub pt: f64,
    pub on_line: bool,
}

impl Landed {
    /// Whether the mark still stands where it landed, in `marks`: one set
    /// or moved since is his own, no longer the kept one.
    fn holds(&self, marks: &HashMap<char, f64>) -> bool {
        marks
            .get(&self.kept.letter)
            .is_some_and(|pt| (pt - self.pt).abs() < 0.01)
    }
}

/// Puts each landed mark not on a line yet on the line of its block
/// nearest it, of `lines` by their tops and baselines, giving the letters
/// and the lines' tops: between lines, going to it would take the line
/// below, and past the block's last line that's the next block's heading.
/// One whose block has no line read yet waits; one set or moved since goes.
pub fn on_lines(
    landed: &mut Vec<Landed>,
    marks: &HashMap<char, f64>,
    starts: &[(usize, f64)],
    lines: &[(f64, f64)],
) -> Vec<(char, f64)> {
    landed.retain(|one| one.holds(marks));
    let mut put = Vec::new();
    for one in landed.iter_mut().filter(|one| !one.on_line) {
        if let Some(top) = line_in(starts, one.kept.block, one.pt, lines) {
            (one.pt, one.on_line) = (top, true);
            put.push((one.kept.letter, top));
        }
    }
    put
}

/// Lands again each landed mark still where it landed on the page set
/// again at another width, whose blocks start at `after`, as far through
/// its block as it was kept, giving the letters and where they land, to
/// go on a line once its lines are read. Found again from its own line,
/// one not on a line yet would be found in the block before.
pub fn landed_again(
    landed: &mut Vec<Landed>,
    marks: &HashMap<char, f64>,
    after: &[(usize, f64)],
) -> Vec<(char, f64)> {
    landed.retain(|one| one.holds(marks));
    landed
        .iter_mut()
        .filter_map(|one| {
            let kept = &one.kept;
            let pt = pt_of(after, kept.block, kept.into_pt, kept.block_pt)?;
            (one.pt, one.on_line) = (pt, false);
            Some((kept.letter, pt))
        })
        .collect()
}

/// The kept mark a letter's mark at `pt` still is, to write back as it
/// was read, with its block's height, which the first screen's page may
/// not know, and before it's on a line, where its own line isn't.
pub fn as_read(landed: &[Landed], letter: char, pt: f64) -> Option<&Mark> {
    landed
        .iter()
        .find(|one| one.kept.letter == letter && (one.pt - pt).abs() < 0.01)
        .map(|one| &one.kept)
}

/// Takes the kept marks whose blocks the page starting at `starts` has,
/// `whole` when no block past them comes, into `marks`, each kept as read
/// in `landed`; the others wait in `kept`. A mark set while it waited is
/// his, and beats it. One kept before its block's height was would land
/// unscaled at every width, so it takes the height its block has here,
/// where its distance is read as it was, once that's known.
pub fn take_kept(
    kept: &mut Vec<Mark>,
    marks: &mut HashMap<char, f64>,
    landed: &mut Vec<Landed>,
    starts: &[(usize, f64)],
    whole: bool,
) {
    kept.retain(|one| {
        let Some(pt) = mark_at(starts, whole, one.block, one.into_pt, one.block_pt) else {
            return true;
        };
        if let std::collections::hash_map::Entry::Vacant(mark) = marks.entry(one.letter) {
            mark.insert(pt);
            let block_pt = one.block_pt.or_else(|| height_of(starts, one.block));
            landed.push(Landed {
                kept: Mark {
                    block_pt,
                    ..one.clone()
                },
                pt,
                on_line: false,
            });
        }
        false
    });
}

/// Gives each landed mark kept without its block's height the height its
/// block has on the page starting at `starts`, once that's known. One in
/// the first screen's last block lands before the block after it is set,
/// so it learns its height only from the whole page; without it, a resize
/// would land it unscaled for good.
pub fn heights_known(landed: &mut [Landed], starts: &[(usize, f64)]) {
    for one in landed.iter_mut().filter(|one| one.kept.block_pt.is_none()) {
        one.kept.block_pt = height_of(starts, one.kept.block);
    }
}

/// Moves `marks` for the page set again, its blocks starting at `after`:
/// a kept mark where it landed lands again from what was kept, and the
/// others go as `moved` takes them. An `edited` file may have renumbered
/// the blocks under a kept mark, so where it stands then is his, no
/// longer the kept one's.
pub fn set_again(
    landed: &mut Vec<Landed>,
    marks: &mut HashMap<char, f64>,
    after: &[(usize, f64)],
    edited: bool,
    moved: impl Fn(f64) -> f64,
) {
    if edited {
        landed.clear();
    }
    let again: HashMap<char, f64> = landed_again(landed, marks, after).into_iter().collect();
    for (letter, pt) in marks.iter_mut() {
        *pt = again.get(letter).copied().unwrap_or_else(|| moved(*pt));
    }
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
                        block_pt: block_pt_of(mark),
                        said: said_of(mark),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Place {
        block,
        into_pt: kept.get("into_pt")?.as_f64()?,
        block_pt: block_pt_of(kept),
        at_end: kept
            .get("at_end")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        marks,
        said: said_of(kept),
        left: kept.get("left").and_then(serde_json::Value::as_u64),
    })
}

/// How tall a place's or a mark's block was, where it was kept.
fn block_pt_of(kept: &serde_json::Value) -> Option<f64> {
    kept.get("block_pt")
        .and_then(serde_json::Value::as_f64)
        .filter(|pt| pt.is_finite() && *pt >= LEAST_BLOCK_PT)
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
            let mut fields = serde_json::json!({
                "letter": mark.letter.to_string(),
                "block": mark.block,
                "into_pt": mark.into_pt,
            });
            if let Some(block_pt) = mark.block_pt {
                fields["block_pt"] = serde_json::json!(block_pt);
            }
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
    if let Some(block_pt) = place.block_pt {
        fields["block_pt"] = serde_json::json!(block_pt);
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
mod tests;
