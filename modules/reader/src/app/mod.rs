//! What the reader does, a file for each part: each holds the methods of App
//! (main.rs) for one part, and this one what they share, the page's colours,
//! the paces its work keeps and the small parts of what App holds. App stays in
//! main.rs, where every part reads its fields.

mod acting;
mod bands;
mod commands;
mod drawing;
mod labels;
mod menus;
mod opening;
mod outline;
mod places;
mod report;
mod searching;
mod view;
mod wayland;

use crate::painter::{Band, Key};
use crate::{gl, page, prompt, reading, text, theme};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

/// The page's ground, the theme's background.
fn ground() -> [f32; 3] {
    theme::get().background.gl()
}

/// The page's text, as the template sets it.
fn page_ink() -> [f32; 3] {
    theme::get().text.gl()
}

/// Neovim's Search in his theme: strong text on the raised ground.
fn search_marks() -> ([f32; 3], [f32; 3]) {
    let ink = theme::get();
    (ink.raised.gl(), ink.strong.gl())
}

/// Neovim's `CurSearch`, the match the view is on: the accent's text on it.
fn current_marks() -> ([f32; 3], [f32; 3]) {
    let ink = theme::get();
    (ink.accent.gl(), ink.on_accent.gl())
}

/// A selection, and a copy's flash: the selection's ground with its words
/// in strong, since the text on that ground falls short of the contrast
/// text needs, as the launcher's selected row does.
fn selection_marks() -> ([f32; 3], [f32; 3]) {
    let ink = theme::get();
    (ink.selection.gl(), ink.strong.gl())
}

/// Visual mode's cursor, a block on its character in the terminal's cursor
/// colours, as kitty and Neovim draw it.
fn cursor_marks() -> ([f32; 3], [f32; 3]) {
    let ink = theme::get();
    (ink.cursor.gl(), ink.cursor_text.gl())
}

/// Bands asked for and not yet back, at most.
const ASKED_AT_ONCE: usize = 8;

/// Bands with the loader at once, the most wanted first. Few, so a band
/// wanted after a jump waits behind no more than these.
const LOADING_AT_ONCE: usize = 4;

/// The scale bands are keyed against: a band's key is its level, the zoom
/// times the screen's scale over this, so a band at one scale stands in
/// for another's as a zoom's do.
const REFERENCE_SCALE: f64 = 2.0;

/// How long the first frame may wait for its screen's bands: design.md's
/// first frame is complete, within the 250 ms the first screen is allowed.
const FIRST_FRAME_AT_MOST: Duration = Duration::from_millis(250);

/// The file as last saved, set and drawing while the page before it still
/// shows. It takes the page's place at once, whole, when its bands cover
/// the view where it will show, so a save never shows half a page.
pub(crate) struct Incoming {
    version: u32,
    page: std::sync::Arc<typst_layout::Page>,
    /// Where the view will be on it, in device pixels: the same words at
    /// the top as on the page before.
    offset: f64,
    /// Where the view was going on the page before when this was set, in
    /// points, so a key that moves it before this shows is known.
    set_at_pt: f64,
    lines: Vec<text::Line>,
    textures: HashMap<Key, gl::Texture>,
    asked: HashSet<Key>,
    loading: HashSet<Key>,
    arrived: VecDeque<Band>,
    /// When the save's page arrived, to say how long it took to show.
    since: Instant,
    set_ms: f64,
    /// The file it's of, when it's another file a link or a jump opened
    /// in this window rather than this one saved.
    opens: Option<std::path::PathBuf>,
}

/// A place on the jump list: a file, how far down it the view's top was,
/// and the top of the cursor's line, in points.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spot {
    file: std::path::PathBuf,
    pt: f64,
    aim: f64,
}

/// Where the view lands on a file opened in the window's place.
pub(crate) enum Landing {
    /// A heading on it, by its anchor, as a link names one.
    Heading(String),
    /// A place on the jump list: the view's top and the cursor's line's.
    At(f64, f64),
    /// Where he was when he last left it, as the reader opens a file.
    Kept,
}

/// Another file being set for the window, and where it will land.
pub(crate) struct Following {
    file: std::path::PathBuf,
    landing: Landing,
}

/// What the reloader is asked: the file saved, so set it again, set
/// another file from now on, or set the page at another width from now on,
/// for a window resized.
pub(crate) enum Reload {
    Saved,
    Open(String),
    Fit(page::Width),
}

/// A page the reloader set, of the file it names, or why it couldn't.
type Reset = (
    String,
    Result<(typst_layout::Page, f64, Vec<text::Line>), String>,
);

/// What the loader is told of a band, to know it again when its texture
/// is made: its key, the page it was drawn from, and how long it took.
#[derive(Clone, Copy)]
pub(crate) struct Loading {
    key: Key,
    version: u32,
    drawn_ms: f64,
}

/// The stand-in's zoom, in thousandths: the whole page drawn once at an
/// eighth of reading size, a dozen bands or so, kept for as long as the page
/// is. Wherever a band isn't drawn yet, its stand-in shows, blurred, rather
/// than ground. No reading zoom is keyed on it, since those go in fiftieths.
const STAND_IN: u32 = 125;

/// Stand-in bands asked for at once, beside the view's own.
const STAND_INS_AT_ONCE: usize = 4;

/// The measure's script, by seconds from its start.
const SCROLL_UNTIL: f64 = 6.0;

const JUMP_UNTIL: f64 = 10.0;

const ZOOM_UNTIL: f64 = 14.0;

const RESIZE_UNTIL: f64 = 18.0;

/// A fast scroll, in logical pixels a second.
const FLICK: f64 = 1500.0;

fn phase(t: f64) -> &'static str {
    if t < SCROLL_UNTIL {
        "scroll"
    } else if t < JUMP_UNTIL {
        "jump"
    } else if t < ZOOM_UNTIL {
        "zoom"
    } else {
        "resize"
    }
}

/// One frame drawn, as the measure keeps it.
pub(crate) struct Drawn {
    phase: &'static str,
    /// Seconds into the script.
    t: f64,
    work_ms: f64,
    /// Of the work: uploading bands, letting bands go (with malloc's trim),
    /// handing the frame to the compositor, asking for bands, and the
    /// measure's own script, in milliseconds.
    parts_ms: [f64; 7],
    /// Of the frame's time, how long the thread was ready to run and waiting
    /// for a processor, so starvation tells apart from the reader's work.
    waited_ms: f64,
    blank: bool,
    /// Some of the view shows a stand-in, not its own sharp bands.
    stood_in: bool,
    /// Textures drawn for the first time since the loader made them.
    fresh: usize,
    presented_ns: Option<u64>,
}

/// Whether spans, each a top and a bottom on screen, leave no gap above
/// `bottom`.
fn covers(spans: &mut [(f32, f32)], bottom: f32) -> bool {
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reach = 0.0_f32;
    for &(top, foot) in spans.iter() {
        if top > reach + 1.0 {
            return false;
        }
        reach = reach.max(foot);
    }
    reach + 1.0 >= bottom
}

/// Peak resident and proportional memory, in megabytes.
fn memory() -> [u64; 2] {
    let field = |text: &str, name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
            .unwrap_or(0)
            / 1024
    };
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let rollup = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap_or_default();
    [field(&status, "VmHWM:"), field(&rollup, "Pss:")]
}

/// gO's chosen row's words reading on: how they slide and how far past
/// their room they run, their strip set whole and painted, when a timer
/// last asked to wake the frame after a rest, and whether they've moved
/// and come to rest at their end, each said once.
pub(crate) struct ReadsOn {
    reading: reading::Reading,
    over: f64,
    strip: prompt::Strip,
    canvas: Option<gl::Texture>,
    painted: bool,
    wake: Option<Instant>,
    moved: bool,
    ended: bool,
}

/// A set's pixels in its canvas, made again only when its size changes,
/// and the canvas to draw; the last one painted when nothing new came.
fn painted<'a>(
    gl: &gl::Gl,
    canvas: &'a mut Option<gl::Texture>,
    set: Option<prompt::Set>,
) -> Option<&'a gl::Texture> {
    if let Some(set) = set {
        let fits = canvas
            .as_ref()
            .is_some_and(|it| it.width == set.width && it.height == set.height);
        if !fits {
            if let Some(old) = canvas.take() {
                gl.forget(old);
            }
            *canvas = gl.canvas(set.width, set.height).ok();
        }
        if let Some(it) = canvas.as_ref() {
            gl.paint(it, &set.pixels);
        }
    }
    canvas.as_ref()
}

/// Opens what a link names as the desktop opens it: a site, an address, or
/// a file the reader doesn't show.
fn desktop_open(to: &str) {
    let opened = std::process::Command::new("xdg-open")
        .arg(to)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    match opened {
        // Waited for on a thread of its own, so it never lingers once it
        // ends.
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
        }
        Err(error) => eprintln!("reader: can't open {to}: {error}"),
    }
}

/// The folder a file is in, a bare name's being the one the reader runs
/// in.
fn folder_of(file: &std::path::Path) -> &std::path::Path {
    file.parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."))
}
