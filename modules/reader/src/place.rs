//! Where he was in a file, kept when the reader closes and taken back when
//! it opens the file again, as Neovim takes back where its cursor was. A
//! place is kept as a block of the file and a distance into it, so the
//! first set reaches it without setting the whole file, and it holds when
//! the file has changed above it.
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

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
    pub marks: Vec<(char, usize, f64)>,
}

/// Where each block starts on a page, by its number, top to bottom.
pub fn starts(frame: &Frame) -> Vec<(usize, f64)> {
    let mut found = Vec::new();
    walk(frame, Point::zero(), &mut found);
    found.sort_by(|a, b| a.1.total_cmp(&b.1));
    found
}

fn walk(frame: &Frame, at: Point, found: &mut Vec<(usize, f64)>) {
    for (place, item) in frame.items() {
        let here = at + *place;
        match item {
            FrameItem::Group(group) => {
                let moved = Point::new(group.transform.tx, group.transform.ty);
                walk(&group.frame, here + moved, found);
            }
            FrameItem::Tag(Tag::Start(content, _)) => {
                if let Some(metadata) = content.to_packed::<MetadataElem>()
                    && let Value::Int(block) = metadata.value
                    && let Ok(block) = usize::try_from(block)
                {
                    found.push((block, here.y.to_pt()));
                }
            }
            _ => {}
        }
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
    let kept: serde_json::Value = serde_json::from_slice(&text).ok()?;
    let block = usize::try_from(kept.get("block")?.as_u64()?).ok()?;
    let marks = kept
        .get("marks")
        .and_then(serde_json::Value::as_array)
        .map(|marks| {
            marks
                .iter()
                .filter_map(|mark| {
                    let letter = mark.get("letter")?.as_str()?.chars().next()?;
                    let block = usize::try_from(mark.get("block")?.as_u64()?).ok()?;
                    Some((letter, block, mark.get("into_pt")?.as_f64()?))
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
    })
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
        .map(|(letter, block, into_pt)| serde_json::json!({"letter": letter.to_string(), "block": block, "into_pt": into_pt}))
        .collect();
    let text = serde_json::json!({
        "file": std::fs::canonicalize(file)?.to_string_lossy(),
        "block": place.block,
        "into_pt": place.into_pt,
        "at_end": place.at_end,
        "marks": marks,
    });
    if let Some(folder) = kept.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let fresh = kept.with_extension(format!("{}.part", std::process::id()));
    std::fs::write(&fresh, text.to_string())?;
    std::fs::rename(&fresh, &kept)
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
    use super::{Place, at, keep_window_in, pt_of, read_in, window_in, write_in};

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
            marks: vec![('a', 3, 7.0)],
        };
        write_in(&state, &file, &place).expect("kept");
        assert_eq!(read_in(&state, &file), Some(place));
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }
}
