//! The reader: a markdown file set by Typst as one tall page, drawn in bands
//! on worker threads, and scrolled, jumped and zoomed by moving textured
//! quads on the GPU, with Neovim's keys. A frame never waits on drawing: a
//! band not yet drawn shows a blurred stand-in of the page, never bare
//! ground, and a zoom stretches the bands it has until the new ones come.
//! The page is set again as the file is saved, and opens where it was last
//! closed.
//!
//! reader                   shows a page of markdown files to read
//! reader FILE              shows the file
//! reader --measure FILE    runs the frame-time script and prints what
//!                          each phase took, then closes
//! reader --keys            prints every chord it takes, for the launcher
//!
//! The measure reads when each frame reached the screen from the
//! compositor's presentation feedback, and holds the worst 1% of intervals
//! against one refresh interval.

mod gl;
mod keys;
mod links;
mod motion;
mod page;
mod painter;
mod place;
mod prompt;
mod search;
mod showing;
mod start;
mod text;
mod theme;
mod tiles;
mod walk;
mod watch;
mod web;
mod worker;
mod world;

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::io::Write;
use std::time::{Duration, Instant};

/// For tests: say each key's act and where the view heads after it.
fn keys_logged() -> bool {
    static LOGGED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LOGGED.get_or_init(|| std::env::var_os("READER_KEYS_LOG").is_some())
}

/// For the measure: draw nothing but the ground, so a run shows what the
/// compositor itself allows under the same load.
fn baseline() -> bool {
    static BASELINE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *BASELINE.get_or_init(|| std::env::var_os("READER_BASELINE").is_some())
}

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, FrameCallbackData};
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::channel::{self, Event};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::{
    Window, WindowConfigure, WindowDecorations, WindowHandler,
};
use smithay_client_toolkit::{delegate_registry, registry_handlers};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_seat, wl_surface};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::{
    self, WpFractionalScaleManagerV1,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::{
    self, WpFractionalScaleV1,
};
use wayland_protocols::wp::presentation_time::client::wp_presentation::{self, WpPresentation};
use wayland_protocols::wp::presentation_time::client::wp_presentation_feedback::{
    self, WpPresentationFeedback,
};
use wayland_protocols::wp::viewporter::client::wp_viewport::{self, WpViewport};
use wayland_protocols::wp::viewporter::client::wp_viewporter::{self, WpViewporter};

use painter::{BAND_PX, Band, Key, Painter};

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
/// Bands asked for and not yet back, at most.
const ASKED_AT_ONCE: usize = 8;
/// Bands with the loader at once, the most wanted first. Few, so a band
/// wanted after a jump waits behind no more than these.
const LOADING_AT_ONCE: usize = 4;
/// The scale bands are keyed against: a band's key is its level, the zoom
/// times the screen's scale over this, so a band at one scale stands in
/// for another's as a zoom's do.
const REFERENCE_SCALE: f64 = 2.0;
/// Points of the page a logical pixel of the window holds at zoom 1, as
/// CSS has it.
const PT_A_PX: f64 = 0.75;
/// How long the first frame may wait for its screen's bands: design.md's
/// first frame is complete, within the 250 ms the first screen is allowed.
const FIRST_FRAME_AT_MOST: Duration = Duration::from_millis(250);

/// How long this thread has waited, ready to run, for a processor, in
/// nanoseconds: schedstat's second figure. Zero where it can't be read.
fn run_delay_ns() -> u64 {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0)
}

/// The file as last saved, set and drawing while the page before it still
/// shows. It takes the page's place at once, whole, when its bands cover
/// the view where it will show, so a save never shows half a page.
struct Incoming {
    version: u32,
    page: std::sync::Arc<typst_layout::Page>,
    /// Where the view will be on it, in device pixels: the same words at
    /// the top as on the page before.
    offset: f64,
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

/// A place on the jump list: a file, and how far down it, in points.
#[derive(Clone, Debug, PartialEq)]
struct Spot {
    file: std::path::PathBuf,
    pt: f64,
}

/// Where the view lands on a file opened in the window's place.
enum Landing {
    /// A heading on it, by its anchor, as a link names one.
    Heading(String),
    /// A place on the jump list, in points.
    At(f64),
    /// Where he was when he last left it, as the reader opens a file.
    Kept,
}

/// Another file being set for the window, and where it will land.
struct Following {
    file: std::path::PathBuf,
    landing: Landing,
}

/// What the reloader is asked: the file saved, so set it again, set
/// another file from now on, or set the page at another width from now on,
/// for a window resized.
enum Reload {
    Saved,
    Open(String),
    Fit(page::Width),
}

/// A page the reloader set, of the file it names, or why it couldn't.
type Reset = (
    String,
    Result<(typst_layout::Page, f64, Vec<text::Line>), String>,
);

/// Where the view's top line is on a new page: the same words, the same
/// how-many-th time they come, at the same height in the view; the end of
/// the page if the view was at the end, as while writing there. Points.
fn anchored(
    old: &[text::Line],
    new: &[text::Line],
    top_pt: f64,
    at_end: bool,
    new_most_pt: f64,
) -> f64 {
    if at_end {
        return new_most_pt;
    }
    let Some((index, line)) = old
        .iter()
        .enumerate()
        .find(|(_, line)| line.baseline_pt >= top_pt)
    else {
        return top_pt.min(new_most_pt);
    };
    let nth = old[..index]
        .iter()
        .filter(|other| other.text == line.text)
        .count();
    let same: Vec<&text::Line> = new.iter().filter(|other| other.text == line.text).collect();
    match same.get(nth).or(same.last()) {
        Some(found) => (found.baseline_pt - (line.baseline_pt - top_pt)).clamp(0.0, new_most_pt),
        None => top_pt.min(new_most_pt),
    }
}

/// What the loader is told of a band, to know it again when its texture
/// is made: its key, the page it was drawn from, and how long it took.
#[derive(Clone, Copy)]
struct Loading {
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
struct Drawn {
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

#[expect(
    clippy::struct_excessive_bools,
    reason = "each says one thing about the window that a frame reads: shown, a frame owed, closing, a search marked, the prompt or the key list up"
)]
struct App {
    registry: RegistryState,
    outputs: OutputState,
    window: Window,
    qh: QueueHandle<App>,
    display: *mut c_void,
    presentation: Option<WpPresentation>,
    /// Cuts the buffer to the window's size, so the buffer can be larger.
    viewport: Option<WpViewport>,
    gl: Option<gl::Gl>,
    painter: Painter,
    /// The window's size in logical pixels, and the buffer's scale.
    size: (u32, u32),
    scale: f64,
    /// The compositor's own scale for the window, where it gives one: the
    /// monitor's 1.5 drawn at 1.5, not at 2 and shrunk.
    fractional: Option<WpFractionalScaleV1>,
    /// How far down the page the view's top is, in device pixels at `zoom`.
    offset: f64,
    zoom: f64,
    textures: HashMap<Key, gl::Texture>,
    /// Makes textures off the frame's path, once the window has its GL.
    loader: Option<gl::Loader<Loading>>,
    /// Where the loader sends what it made.
    made_to: channel::Sender<gl::Made<Loading>>,
    /// Bands with the loader, and textures it made that the GPU may still
    /// be filling: neither asked nor held, so nothing asks for them again.
    loading: HashSet<Key>,
    made: Vec<gl::Made<Loading>>,
    /// Textures taken from the loader and not yet drawn, for the measure.
    fresh: HashSet<Key>,
    asked: HashSet<Key>,
    arrived: VecDeque<Band>,
    frame_pending: bool,
    /// Whether a frame has been shown yet, and when the window was made.
    shown: bool,
    opened: Instant,
    /// Where the first screen's page settled, in points: bands below it
    /// drawn from that page are stale once the whole page is set.
    settled_pt: f64,
    /// The first screen's page's height, in points, kept as the page's end
    /// from when the whole page is set until the whole page's bands cover
    /// the view: until then the view shows the first screen's page as it
    /// was, so the page grows only into what is drawn.
    held_pt: Option<f64>,
    stale: HashSet<Key>,
    jump: Option<motion::Jump>,
    /// Under reduced motion a move cuts straight there.
    reduced_motion: bool,
    measure: Option<Measure>,
    exit: bool,
    seats: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    loop_handle: LoopHandle<'static, App>,
    keys: keys::Machine,
    ctrl: bool,
    /// Marks by letter, where the view's top was, in points down the page,
    /// so a mark holds across a zoom.
    marks: HashMap<char, f64>,
    /// Where the view opens on the page, in points: where he was when he
    /// last closed the file. Taken at the first frame.
    start_pt: Option<f64>,
    /// Marks kept from the last time, whose blocks aren't set yet.
    kept_marks: Vec<(char, usize, f64)>,
    /// The places jumped from, oldest first, each with its file, and where
    /// Ctrl+O and Ctrl+I stand in them; at its length, past the newest.
    jumps: Vec<Spot>,
    jump_at: usize,
    /// Where the last jump left from, for ''.
    last_jump: Option<f64>,
    /// The page's lines, read off its frames for search; none until read.
    words: Option<Vec<text::Line>>,
    /// The file as last saved, drawing until it can take the page's place.
    incoming: Option<Incoming>,
    /// The version the next save's page is drawn as.
    next_version: u32,
    /// Whether a save has taken the page's place, after which the rest of
    /// the first set is older than what shows.
    reloaded: bool,
    /// How wide the page is set for the window: what shows, or what the
    /// reloader was last asked for when the window was resized.
    width: page::Width,
    words_to: channel::Sender<(u32, Vec<text::Line>)>,
    /// A search being typed after / or ?.
    typing: Option<search::Typing>,
    /// The last search made, and whether it went forward, for n and N.
    searched: Option<(String, bool)>,
    found: Vec<text::Found>,
    /// The match the view is on.
    current: Option<usize>,
    /// Whether the matches are marked: Escape at rest unmarks them, as
    /// Neovim's :nohlsearch, and n or N marks them again.
    highlighting: bool,
    /// The marks' swatches and blends: Search's, then `CurSearch`'s.
    lit: Option<[(gl::Texture, gl::Blend); 2]>,
    prompt: Option<prompt::Prompt>,
    prompt_to: channel::Sender<prompt::Set>,
    prompt_serial: u32,
    prompt_shown: bool,
    prompt_set: Option<prompt::Set>,
    prompt_canvas: Option<gl::Texture>,
    /// Whether Space ?'s key list shows, the scale it was set at, and it
    /// set, waiting to be painted and painted.
    keys_shown: bool,
    keys_px: f64,
    keys_set: Option<prompt::Set>,
    keys_canvas: Option<gl::Texture>,
    /// The file shown, which a link's path is taken from.
    file: std::path::PathBuf,
    /// Another file being set to take this one's place, once it's drawn.
    following: Option<Following>,
    /// Asks the reloader to set the file again, or another file.
    reload_to: Option<std::sync::mpsc::Sender<Reload>>,
    /// The watch on the file shown, which moves with it.
    watch: Option<watch::Watch>,
    /// Whether the reader runs the measure, which says nothing of what
    /// it shows.
    measuring: bool,
    /// g x's labels while they show: what's typed of one, and the links in
    /// view they stand for, in the labels' order.
    labelling: Option<(links::Labelling, Vec<links::Link>)>,
    labels_set: Option<Vec<prompt::Label>>,
    /// Each label painted, and how far below its top its letters stand.
    label_canvases: Vec<(gl::Texture, f64)>,
}

/// One line of the page's body text, what j and k move by.
const LINE_PT: f64 = page::PITCH_PT;

struct Measure {
    start: Option<Instant>,
    last: Option<Instant>,
    drawn: Vec<Drawn>,
    refresh_ns: u32,
    discarded: u32,
    configures: u32,
    jumps: u32,
    drawn_ms: Vec<f64>,
    set_ms: f64,
    /// The set's parts: name, time and time waiting for a processor.
    set_parts: Vec<(&'static str, f64, f64)>,
    /// From the program's start to its window's first configure.
    configured_ms: Option<f64>,
    /// Memory and bands held as each phase ends.
    held: Vec<(&'static str, [u64; 2], usize)>,
    /// How long the rest of the page took to set, once it had.
    rest_ms: Option<f64>,
    /// From the program's start to its first frame.
    first_frame_ms: Option<f64>,
    /// How long each jump held the view for its bands.
    held_ms: Vec<f64>,
    /// What a jump that waited out its hold still lacked.
    missing: Vec<String>,
}

impl App {
    /// Shows the buffer's top left at the window's size.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn cut(&self) {
        if let Some(viewport) = &self.viewport {
            let (w, h) = self.size;
            let (bw, bh) = self.buffer_size();
            // The source is in the surface's own pixels: the buffer's over
            // its buffer scale, which is 1 under a fractional scale.
            let over = f64::from(self.buffer_scale());
            viewport.set_source(0.0, 0.0, f64::from(bw) / over, f64::from(bh) / over);
            viewport.set_destination(w as i32, h as i32);
        }
    }

    /// The buffer for the window at its scale, in device pixels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn buffer_size(&self) -> (i32, i32) {
        let (w, h) = self.size;
        (
            (f64::from(w) * self.scale).round() as i32,
            (f64::from(h) * self.scale).round() as i32,
        )
    }

    /// The surface's buffer scale: 1 under a fractional scale, which the
    /// viewport carries instead, and the whole scale otherwise.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn buffer_scale(&self) -> i32 {
        if self.fractional.is_some() {
            1
        } else {
            self.scale.round() as i32
        }
    }

    /// Draws at a new scale from here on: the bands drawn at the old one
    /// are let go and the view keeps its place.
    fn rescale(&mut self, scale: f64) {
        if (scale - self.scale).abs() < 1e-6 {
            return;
        }
        self.offset *= scale / self.scale;
        self.scale = scale;
        self.window
            .wl_surface()
            .set_buffer_scale(self.buffer_scale());
        // The bands drawn at the old scale are of another level now, and
        // show stretched until this level's replace them, as a zoom's do.
        let (w, h) = self.buffer_size();
        if let Some(gl) = self.gl.as_mut() {
            gl.resize(w, h);
        }
        self.cut();
        if !self.frame_pending {
            self.draw();
        }
    }

    /// How large the page is drawn: the zoom over the screen's scale, against
    /// the reference scale bands are keyed by.
    fn level(&self) -> f64 {
        self.zoom * self.scale / REFERENCE_SCALE
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn zoom_key(&self) -> u32 {
        // Drawn in twentieths of a level, so a glide asks for a band set a
        // step rather than every frame, and a new scale is one more step.
        ((self.level() * 20.0).round() * 50.0) as u32
    }

    fn screen_px(&self) -> f64 {
        self.gl.as_ref().map_or(0.0, |gl| f64::from(gl.height))
    }

    /// Where the page's left edge sits in the window, in device pixels: the
    /// page centred, so a wide window leaves even room either side of its
    /// measure, on a whole pixel so its type is sharp, and at the window's
    /// edge once the page is as wide as the window.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window's pixels as f32, as GL takes them"
    )]
    fn page_x(&self) -> f32 {
        let window = self.gl.as_ref().map_or(0.0, |gl| f64::from(gl.width));
        ((window - self.painter.width_pt() * self.px_per_pt()) / 2.0)
            .max(0.0)
            .round() as f32
    }

    fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, self.most());
    }

    fn set_zoom(&mut self, zoom: f64) {
        self.offset *= zoom / self.zoom;
        self.zoom = zoom;
    }

    /// Moves the view as the measure's script says at `t` seconds, `dt` since
    /// the last frame.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn script(&mut self, t: f64, dt: f64) {
        let flick = FLICK * self.scale * dt;
        let screen = self.screen_px();
        match phase(t) {
            "scroll" if t < SCROLL_UNTIL / 2.0 => self.offset += flick,
            "scroll" => self.offset -= flick,
            "jump" => {
                // A screen down every 0.4 s, then to the end and the top, as
                // Ctrl+F, G and gg would.
                let step = ((t - SCROLL_UNTIL) / 0.4).floor() as u32;
                let measure = self.measure.as_mut().expect("measuring");
                if step > measure.jumps {
                    measure.jumps = step;
                    let to = match step {
                        1..=7 => self.offset + screen,
                        8 => f64::MAX,
                        _ => 0.0,
                    };
                    self.jump_to(to);
                }
            }
            "zoom" => {
                self.jump = None;
                let s = t - JUMP_UNTIL;
                let zoom = if s < 2.0 {
                    [1.25, 1.5, 1.25, 1.0][((s / 0.5) as usize).min(3)]
                } else {
                    1.0 + 0.5 * (std::f64::consts::PI * (s - 2.0) / 2.0).sin()
                };
                self.set_zoom(zoom);
            }
            _ => {
                // A floating window in a headless sway is resized by
                // nobody, so the reader resizes its own buffer every quarter
                // second, which costs it what a resize from niri would.
                self.set_zoom(1.0);
                self.offset += 300.0 * self.scale * dt;
                let wide = ((t - ZOOM_UNTIL) / 0.25) as u32 % 2 == 1;
                let width = if wide { 1200 } else { 960 };
                if width != self.size.0 {
                    self.size.0 = width;
                    let (w, h) = self.buffer_size();
                    if let Some(gl) = self.gl.as_mut() {
                        gl.resize(w, h);
                    }
                    self.cut();
                    if let Some(measure) = self.measure.as_mut() {
                        measure.configures += 1;
                    }
                }
            }
        }
    }

    /// Asks for the bands in view and a screen either side, nearest first.
    /// The furthest down the view's top may go, in device pixels.
    fn most(&self) -> f64 {
        (self.end_px() - self.screen_px()).max(0.0)
    }

    /// Whether the view is at the page's end, which it keeps to as the page
    /// or the window changes under it, as while writing there.
    fn at_end(&self) -> bool {
        self.most() > 0.0 && self.offset >= self.most() - 1.0
    }

    /// Where the page shown ends, in device pixels from its top at the
    /// zoom shown: the first screen's page's end while it is held.
    fn end_px(&self) -> f64 {
        let pt = self.held_pt.unwrap_or_else(|| self.painter.height_pt());
        pt * self.px_per_pt()
    }

    /// Whether the whole page's own bands, at any zoom or the stand-in's,
    /// cover the view down to the whole page's end, with nothing of the
    /// first screen's page needed.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn whole_page_covers(&self) -> bool {
        let screen = self.screen_px() as f32;
        let mut spans: Vec<(f32, f32)> = self
            .textures
            .iter()
            .filter(|(k, _)| !self.stale.contains(k))
            .map(|(k, texture)| {
                let (top, height) = self.placed(*k, texture.height);
                (top.max(0.0), (top + height).min(screen))
            })
            .filter(|(top, foot)| foot > top)
            .collect();
        let end = (self.painter.height_pt() * self.px_per_pt() - self.offset) as f32;
        covers(&mut spans, screen.min(end))
    }

    /// Starts a jump: it holds the view until the bands it lands on are
    /// drawn, then moves there (motion.rs).
    fn jump_to(&mut self, to: f64) {
        let to = to.clamp(0.0, self.most());
        let from = self
            .jump
            .as_ref()
            .map_or(self.offset, |jump| jump.at(Instant::now(), false).0);
        self.offset = from;
        self.jump = Some(motion::Jump::new(
            from,
            to,
            self.screen_px(),
            Instant::now(),
        ));
    }

    /// Device pixels a point of the page takes at the zoom shown.
    fn px_per_pt(&self) -> f64 {
        self.painter.scale(1000) * self.level()
    }

    /// Where the view is going: a jump's landing while one runs.
    fn heading(&self) -> f64 {
        self.jump.as_ref().map_or(self.offset, |jump| jump.to)
    }

    /// Moves at once, as a line does: a jump in progress is cut short.
    fn step_to(&mut self, to: f64) {
        self.jump = None;
        self.offset = to;
        self.clamp();
        self.ask_for_bands();
    }

    /// A jump vi remembers: where it left from goes on the jump list and
    /// becomes the place '' goes back to.
    fn leap(&mut self, to: f64) {
        self.remember(self.heading() / self.px_per_pt());
        self.jump_to(to);
    }

    /// Puts a place, in points, on the jump list as the last jump's start.
    fn remember(&mut self, here: f64) {
        self.jumps.truncate(self.jump_at);
        self.jumps.push(Spot {
            file: self.file.clone(),
            pt: here,
        });
        self.jump_at = self.jumps.len();
        self.last_jump = Some(here);
    }

    /// Device pixels a point of the prompt takes: the page's at zoom 1.
    fn prompt_px_per_pt(&self) -> f64 {
        self.painter.base * self.scale / REFERENCE_SCALE
    }

    /// Whether a match shows whole in the view it's heading for, above
    /// the prompt's row.
    fn in_view(&self, i: usize) -> bool {
        let px = self.px_per_pt();
        let found = &self.found[i];
        let top = search::top_pt(found) * px;
        let foot = search::foot_pt(found) * px;
        let room = self.screen_px() - prompt::ROW_PT * self.prompt_px_per_pt();
        let view = self.heading();
        top >= view && foot <= view + room
    }

    /// Brings a match into view, a third of the way down, unless it shows.
    fn show_match(&mut self, i: usize, leap: bool) {
        if self.in_view(i) {
            return;
        }
        let to = search::top_pt(&self.found[i]) * self.px_per_pt() - self.screen_px() / 3.0;
        if leap {
            self.leap(to);
        } else {
            self.step_to(to);
        }
    }

    /// Reads the page's lines for search on a thread of its own.
    fn read_words(&self) {
        let page = self.painter.page();
        let version = self.painter.version;
        let to = self.words_to.clone();
        let _ = std::thread::Builder::new()
            .name("words".into())
            .spawn(move || {
                let _ = to.send((version, text::lines(text::runs(&page.frame))));
            });
    }

    fn words_read(&mut self, version: u32, lines: Vec<text::Line>) {
        if version != self.painter.version {
            return;
        }
        if let Some(needle) = std::env::var_os("READER_WORDS_DUMP") {
            let needle = needle.to_string_lossy().to_lowercase();
            for line in lines
                .iter()
                .filter(|line| line.text.to_lowercase().contains(&needle))
            {
                eprintln!(
                    "words v{version} {:.1},{:.1} {:?}",
                    line.x_pt, line.baseline_pt, line.text
                );
            }
        }
        self.words = Some(lines);
        self.refind();
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Finds what is typed, or the last search, again. While typing, the
    /// view shows the match from where the search began, as incsearch.
    fn refind(&mut self) {
        let (query, forward, from_pt) = match (&self.typing, &self.searched) {
            (Some(typing), _) => (typing.typed.clone(), typing.forward, typing.from_pt),
            (None, Some((query, forward))) => {
                (query.clone(), *forward, self.heading() / self.px_per_pt())
            }
            (None, None) => return,
        };
        self.found = self
            .words
            .as_ref()
            .map_or_else(Vec::new, |lines| text::find(lines, &query));
        self.current = search::from(&self.found, from_pt, forward);
        if self.typing.is_some() {
            self.highlighting = true;
            self.step_to(from_pt * self.px_per_pt());
            if let Some(i) = self.current {
                self.show_match(i, false);
            }
        }
        if self.typing.is_some() || self.prompt_shown {
            self.ask_prompt();
        }
    }

    /// n and N: on to the next match the last search's way, or back.
    fn go_on(&mut self, same_way: bool, times: u32) {
        let Some((_, forward)) = self.searched.clone() else {
            return;
        };
        let forward = if same_way { forward } else { !forward };
        self.highlighting = true;
        let count = self.found.len();
        if count > 0 {
            let view_pt = self.heading() / self.px_per_pt();
            let (mut i, left) = match self.current.filter(|&i| self.in_view(i)) {
                Some(i) => (i, times),
                None => (
                    search::from(&self.found, view_pt, forward).unwrap_or(0),
                    times.saturating_sub(1),
                ),
            };
            for _ in 0..(left as usize % count) {
                i = search::step(count, i, forward);
            }
            self.current = Some(i);
            self.show_match(i, true);
        }
        self.ask_prompt();
    }

    /// Takes a key while a search is typed.
    fn typed(&mut self, said: search::Typed) {
        match said {
            search::Typed::Changed => self.refind(),
            search::Typed::Done => {
                let Some(typing) = self.typing.take() else {
                    return;
                };
                // An empty search is the last one again, as in Neovim.
                let query = if typing.typed.is_empty() {
                    self.searched.as_ref().map(|(query, _)| query.clone())
                } else {
                    Some(typing.typed.clone())
                };
                if let Some(query) = query {
                    self.searched = Some((query, typing.forward));
                    self.step_to(typing.from_pt * self.px_per_pt());
                    self.remember(typing.from_pt);
                    self.refind();
                    if let Some(i) = self.current {
                        self.show_match(i, false);
                    }
                } else {
                    self.prompt_shown = false;
                }
            }
            search::Typed::Cancelled => {
                if let Some(typing) = self.typing.take() {
                    self.step_to(typing.from_pt * self.px_per_pt());
                }
                // The last search's matches come back, as Neovim's do.
                self.refind();
                self.highlighting = self.searched.is_some();
                self.prompt_shown = false;
            }
            search::Typed::Nothing => {}
        }
        self.log_search(&format!("{said:?}"));
        if !self.frame_pending {
            self.draw();
        }
    }

    fn log_search(&self, what: &str) {
        if keys_logged() {
            let typed = self.typing.as_ref().map(|typing| typing.typed.as_str());
            eprintln!(
                "search {what} {typed:?} {:?}: {} found, on {:?}, heading {:.1} pt, marked {}, prompt {}",
                self.searched,
                self.found.len(),
                self.current,
                self.heading() / self.px_per_pt(),
                self.highlighting,
                self.prompt_shown
            );
        }
    }

    /// Asks for the prompt as it should read now.
    fn ask_prompt(&mut self) {
        let (forward, typed, cursor) = match (&self.typing, &self.searched) {
            (Some(typing), _) => (typing.forward, typing.typed.clone(), true),
            (None, Some((query, forward))) => (*forward, query.clone(), false),
            (None, None) => return,
        };
        let px_per_pt = self.prompt_px_per_pt();
        let (Some(prompt), Some(gl)) = (self.prompt.as_ref(), self.gl.as_ref()) else {
            return;
        };
        self.prompt_serial += 1;
        prompt.ask(prompt::Ask {
            serial: self.prompt_serial,
            keys: false,
            labels: Vec::new(),
            forward,
            typed,
            cursor,
            count: self
                .words
                .as_ref()
                .map(|_| (self.current.map_or(0, |i| i + 1), self.found.len())),
            width_pt: f64::from(gl.width) / px_per_pt,
            px_per_pt,
            page: self.width,
        });
        self.prompt_shown = true;
    }

    /// Shows or hides the key list, asking for it set at this scale if it
    /// isn't yet.
    fn show_keys(&mut self, shown: bool) {
        self.keys_shown = shown;
        let px_per_pt = self.prompt_px_per_pt();
        if shown
            && (self.keys_px - px_per_pt).abs() > f64::EPSILON
            && let Some(prompt) = self.prompt.as_ref()
        {
            self.keys_px = px_per_pt;
            prompt.ask(prompt::Ask {
                serial: 0,
                keys: true,
                labels: Vec::new(),
                forward: true,
                typed: String::new(),
                cursor: false,
                count: None,
                width_pt: 0.0,
                px_per_pt,
                page: self.width,
            });
        }
    }

    /// g x's labels, each over its link's first letters with its letters'
    /// baseline on the link's, on whole pixels so its letters are sharp; a
    /// newly set batch is painted first.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    fn draw_labels(&mut self) {
        if self.labelling.is_none() {
            return;
        }
        let px = self.px_per_pt();
        let offset = self.offset;
        let left = self.page_x();
        let Some(gl) = self.gl.as_ref() else { return };
        if let Some(set) = self.labels_set.take() {
            for (old, _) in self.label_canvases.drain(..) {
                gl.forget(old);
            }
            for label in set {
                if let Ok(canvas) = gl.canvas(label.width, label.height) {
                    gl.paint(&canvas, &label.pixels);
                    self.label_canvases.push((canvas, label.baseline_px));
                }
            }
        }
        if let Some((_, shown)) = self.labelling.as_ref() {
            for (link, (canvas, baseline_px)) in shown.iter().zip(&self.label_canvases) {
                let (width, height) = (canvas.width as f32, canvas.height as f32);
                let x = (link.left_pt * px).round() as f32 + left;
                let y = links::label_top(link, *baseline_px, px, offset);
                gl.draw(canvas, x, y, width, height);
            }
        }
    }

    /// Labels every link in view, as flash labels a jump: a label's letters
    /// typed follow it, and anything else puts them away.
    fn show_links(&mut self) {
        let px = self.px_per_pt();
        let top = self.heading() / px;
        let foot = top + self.screen_px() / px;
        let shown: Vec<links::Link> = links::links(&self.painter.page().frame)
            .into_iter()
            .filter(|link| link.top_pt + link.height_pt > top && link.top_pt < foot)
            .collect();
        if keys_logged() {
            eprintln!("links: {} in view", shown.len());
        }
        if shown.is_empty() {
            return;
        }
        let labelling = links::Labelling::new(shown.len());
        if let Some(prompt) = self.prompt.as_ref() {
            prompt.ask(prompt::Ask {
                serial: 0,
                keys: false,
                labels: labelling.labels.clone(),
                forward: true,
                typed: String::new(),
                cursor: false,
                count: None,
                width_pt: 0.0,
                px_per_pt: px,
                page: self.width,
            });
        }
        self.labelling = Some((labelling, shown));
    }

    /// A key while the labels show: a letter of one, or else they go.
    fn label_key(&mut self, key: keys::Key) {
        let Some((labelling, shown)) = self.labelling.as_mut() else {
            return;
        };
        let chosen = match key {
            keys::Key::Char(letter) => match labelling.key(letter) {
                links::Typed::Waiting => return,
                links::Typed::Chosen(at) => shown.get(at).cloned(),
                links::Typed::Gone => None,
            },
            _ => None,
        };
        self.labelling = None;
        for (old, _) in self.label_canvases.drain(..) {
            if let Some(gl) = self.gl.as_ref() {
                gl.forget(old);
            }
        }
        if let Some(link) = chosen {
            self.follow(&link);
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Goes where a link points: to a heading here, as a jump Ctrl+O comes
    /// back from, or anywhere else as the desktop opens it.
    fn follow(&mut self, link: &links::Link) {
        if keys_logged() {
            eprintln!("link: {}", link.href);
        }
        match links::target(&link.href, &self.file) {
            // A link to the page itself, with no heading, goes to its top.
            links::Target::Here(anchor) if anchor.is_empty() => self.leap(0.0),
            links::Target::Here(anchor) => {
                let heading = self
                    .words
                    .as_ref()
                    .and_then(|lines| links::heading_at(lines, &anchor));
                match heading {
                    Some(pt) => self.leap(pt.max(0.0) * self.px_per_pt()),
                    None => eprintln!("reader: no heading here is #{anchor}"),
                }
            }
            links::Target::Page(file, heading) => {
                if !file.is_file() {
                    eprintln!("reader: there's no {}", file.display());
                    return;
                }
                self.remember(self.heading() / self.px_per_pt());
                self.open(file, heading.map_or(Landing::Kept, Landing::Heading));
            }
            links::Target::Elsewhere(to) => {
                let opened = std::process::Command::new("xdg-open")
                    .arg(&to)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                match opened {
                    // Waited for on a thread of its own, so it never
                    // lingers once it ends.
                    Ok(mut child) => {
                        std::thread::spawn(move || child.wait());
                    }
                    Err(error) => eprintln!("reader: can't open {to}: {error}"),
                }
            }
        }
    }

    fn prompt_arrived(&mut self, set: prompt::Set) {
        if !set.labels.is_empty() {
            if self.labelling.is_some() {
                self.labels_set = Some(set.labels);
                if !self.frame_pending {
                    self.draw();
                }
            }
            return;
        }
        if set.keys {
            self.keys_set = Some(set);
            if self.keys_shown && !self.frame_pending {
                self.draw();
            }
            return;
        }
        if set.serial == self.prompt_serial {
            self.prompt_set = Some(set);
            if !self.frame_pending {
                self.draw();
            }
        }
    }

    /// Goes to a place on the jump list without adding to it: on this page
    /// as a jump, or on another file, set in this one's place.
    fn revisit(&mut self, spot: Spot) {
        if !links::same(&spot.file, &self.file) {
            self.open(spot.file, Landing::At(spot.pt));
            return;
        }
        // Back on the file shown before another took its place: the other
        // is let go, and the reloader set on this one again.
        if self.following.take().is_some() {
            self.let_incoming_go();
            self.set_on(&self.file);
        }
        self.jump_to(spot.pt * self.px_per_pt());
    }

    /// Sets another file to take this one's place in the window once its
    /// bands cover the view, where `landing` says.
    fn open(&mut self, file: std::path::PathBuf, landing: Landing) {
        self.let_incoming_go();
        self.set_on(&file);
        self.following = Some(Following { file, landing });
    }

    /// Asks the page set again for the window's width, when that width
    /// takes another page than the one asked for last: a window narrower
    /// than the whole page's 540 pt, or one widened back past it. The page
    /// shown stays until the new one is set, with the view on the same
    /// words.
    fn fit_window(&mut self) {
        let width = page::Width::for_window(f64::from(self.size.0) * PT_A_PX);
        if width == self.width {
            return;
        }
        // Taken as asked only once the reloader has it, so a page is never
        // held to a width nothing will set.
        if let Some(to) = self.reload_to.as_ref()
            && to.send(Reload::Fit(width)).is_ok()
        {
            self.width = width;
        }
    }

    /// Moves the reloader and the watch to `file`, which the reloader sets
    /// at once and then at each save.
    fn set_on(&self, file: &std::path::Path) {
        if let Some(to) = self.reload_to.as_ref() {
            let _ = to.send(Reload::Open(file.to_string_lossy().into_owned()));
        }
        if let Some(watch) = self.watch.as_ref()
            && let Err(error) = watch.follow(file)
        {
            eprintln!("reader: can't watch {} for saves: {error}", file.display());
        }
    }

    /// Lets go of a page set to take the page's place, and its bands.
    fn let_incoming_go(&mut self) {
        if let Some(old) = self.incoming.take()
            && let Some(gl) = self.gl.as_ref()
        {
            for (_, texture) in old.textures {
                gl.forget(texture);
            }
        }
    }

    /// Does what a key asks, `count` times where a count means anything.
    fn act(&mut self, act: keys::Act, count: Option<u32>) {
        use keys::Act;
        let n = f64::from(count.unwrap_or(1));
        let screen = self.screen_px();
        let line = LINE_PT * self.px_per_pt();
        let at = self.heading();
        // The prompt stays while n and N go on, as Neovim's search count.
        if !matches!(
            act,
            Act::Next | Act::Previous | Act::SearchForward | Act::SearchBackward
        ) {
            self.prompt_shown = false;
        }
        // Any key closes the key list, as which-key's, and does what it does;
        // Escape only closes it, and Space ? again too.
        let listed = self.keys_shown;
        if listed {
            self.show_keys(false);
        }
        match act {
            Act::Down | Act::ScrollDown => self.step_to(at + n * line),
            Act::Up | Act::ScrollUp => self.step_to(at - n * line),
            // The page is set to fit the window's width, so there's never
            // anything beside it to move to.
            Act::Left | Act::Right => {}
            Act::HalfDown => self.jump_to(at + n * screen / 2.0),
            Act::HalfUp => self.jump_to(at - n * screen / 2.0),
            Act::ScreenDown => self.jump_to(at + n * screen),
            Act::ScreenUp => self.jump_to(at - n * screen),
            Act::First => self.leap(count.map_or(0.0, |n| f64::from(n.saturating_sub(1)) * line)),
            Act::Last => {
                self.leap(count.map_or(f64::MAX, |n| f64::from(n.saturating_sub(1)) * line));
            }
            Act::SetMark(letter) => {
                let here = at / self.px_per_pt();
                self.marks.insert(letter, here);
            }
            Act::GoToMark(letter) => {
                if let Some(&pt) = self.marks.get(&letter) {
                    self.leap(pt * self.px_per_pt());
                }
            }
            Act::LastJump => {
                if let Some(pt) = self.last_jump {
                    self.leap(pt * self.px_per_pt());
                }
            }
            Act::JumpBack => {
                if self.jump_at > 0 {
                    if self.jump_at == self.jumps.len() {
                        self.jumps.push(Spot {
                            file: self.file.clone(),
                            pt: at / self.px_per_pt(),
                        });
                    }
                    self.jump_at -= 1;
                    self.revisit(self.jumps[self.jump_at].clone());
                }
            }
            Act::JumpForward => {
                if self.jump_at + 1 < self.jumps.len() {
                    self.jump_at += 1;
                    self.revisit(self.jumps[self.jump_at].clone());
                }
            }
            Act::Close => self.exit = true,
            Act::Links => self.show_links(),
            // A tenth larger or smaller a step, as a browser's, the view on
            // the same words, within half and three times its own size.
            Act::ZoomIn | Act::ZoomOut | Act::ZoomBack => {
                let step = match act {
                    Act::ZoomIn => 1.1_f64.powf(n),
                    Act::ZoomOut => 1.1_f64.powf(-n),
                    _ => 1.0 / self.zoom,
                };
                self.jump = None;
                self.set_zoom((self.zoom * step).clamp(ZOOM_LEAST, ZOOM_MOST));
                self.clamp();
            }
            Act::SearchForward | Act::SearchBackward => {
                self.typing = Some(search::Typing::new(
                    act == Act::SearchForward,
                    at / self.px_per_pt(),
                ));
                self.refind();
            }
            Act::Next | Act::Previous => self.go_on(act == Act::Next, count.unwrap_or(1)),
            Act::ShowKeys => self.show_keys(!listed),
            Act::Rest if listed => {}
            Act::Rest => self.highlighting = false,
        }
        if keys_logged() {
            eprintln!(
                "key {act:?} {count:?}: heading {:.1} pt",
                self.heading() / self.px_per_pt()
            );
            if matches!(
                act,
                Act::Next | Act::Previous | Act::SearchForward | Act::SearchBackward | Act::Rest
            ) {
                self.log_search(&format!("{act:?}"));
            }
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Whether the view shows no ground: its bands at any zoom, stand-in
    /// included, cover it down to the page's end.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn covered(&self) -> bool {
        let screen = self.screen_px() as f32;
        let mut spans: Vec<(f32, f32)> = self
            .textures
            .iter()
            .map(|(k, texture)| {
                let (top, height) = self.placed(*k, texture.height);
                let foot = if self.stale.contains(k) && self.held_pt.is_none() {
                    let ratio = self.level() * 1000.0 / f64::from(k.zoom);
                    let settled = self.settled_pt * self.painter.scale(k.zoom) * ratio;
                    (settled - self.offset) as f32
                } else {
                    top + height
                };
                (top.max(0.0), foot.min(top + height).min(screen))
            })
            .filter(|(top, foot)| foot > top)
            .collect();
        let page_end = (self.end_px() - self.offset) as f32;
        covers(&mut spans, screen.min(page_end))
    }

    /// Whether the bands for a view whose top is at `top` are all drawn.
    fn drawn_at(&self, top: f64) -> bool {
        self.missing_at(top).is_empty()
    }

    /// The bands a view whose top is at `top` lacks, each as its index and
    /// whether it is asked, waiting to upload, or not asked at all.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn missing_at(&self, top: f64) -> Vec<(u32, &'static str)> {
        let key = self.zoom_key();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let first = (top / band).floor().max(0.0) as u32;
        let last = ((top + self.screen_px()) / band).ceil() as u32;
        (first..last.min(self.painter.bands(key)))
            .filter_map(|index| {
                let k = Key { zoom: key, index };
                if self.textures.contains_key(&k) {
                    None
                } else if self.asked.contains(&k) {
                    Some((index, "asked"))
                } else if self.arrived.iter().any(|band| band.key == k) {
                    Some((index, "waiting"))
                } else {
                    Some((index, "unasked"))
                }
            })
            .collect()
    }

    /// Moves a jump on: held until its bands are drawn or it has waited
    /// long enough, then eased there.
    fn move_jump(&mut self, now: Instant) {
        let Some(jump) = self.jump.as_ref() else {
            return;
        };
        if jump.moving.is_none() {
            let ready = self.drawn_at(jump.from) && self.drawn_at(jump.to);
            let waited = now - jump.asked;
            if !ready && waited < motion::HOLD_AT_MOST {
                return;
            }
            let (from, to) = (jump.from, jump.to);
            let missing = format!(
                "from {} {:?} to {} {:?} of {}",
                from.round(),
                self.missing_at(from),
                to.round(),
                self.missing_at(to),
                self.painter.bands(self.zoom_key())
            );
            if let Some(measure) = self.measure.as_mut() {
                measure.held_ms.push(waited.as_secs_f64() * 1000.0);
                if !ready {
                    measure.missing.push(missing);
                }
            }
            if let Some(jump) = self.jump.as_mut() {
                jump.moving = Some(now);
            }
        }
        let Some(jump) = self.jump.as_ref() else {
            return;
        };
        let (at, landed) = jump.at(now, self.reduced_motion);
        self.offset = at;
        if landed {
            self.jump = None;
        }
    }

    fn ask_for_bands(&mut self) {
        // A jump's landing first, then where it leaves from, then the view.
        let tops: Vec<f64> = match &self.jump {
            Some(jump) if jump.moving.is_none() => vec![jump.to, jump.from, self.offset],
            _ => vec![self.offset],
        };
        for top in tops {
            self.ask_for_bands_at(top);
        }
        // From the start, so a view that moves at once, or a first screen
        // that isn't drawn in time, has its stand-in.
        self.ask_for_stand_ins();
    }

    /// Asks for the stand-in's bands not held yet, nearest the view first,
    /// a few at a time beside the view's own.
    fn ask_for_stand_ins(&mut self) {
        let in_flight = self.asked.iter().filter(|k| k.zoom == STAND_IN).count();
        if in_flight >= STAND_INS_AT_ONCE {
            return;
        }
        let ratio = self.level() * 1000.0 / f64::from(STAND_IN);
        let band = f64::from(BAND_PX) * ratio;
        let middle = (self.offset + self.screen_px() / 2.0) / band;
        let mut wanted: Vec<u32> = (0..self.painter.bands(STAND_IN))
            .filter(|&index| {
                let key = Key {
                    zoom: STAND_IN,
                    index,
                };
                let held = self.textures.contains_key(&key) && !self.stale.contains(&key);
                !held
                    && !self.asked.contains(&key)
                    && !self.loading.contains(&key)
                    && !self.arrived.iter().any(|band| band.key == key)
            })
            .collect();
        wanted.sort_by(|a, b| {
            (f64::from(*a) + 0.5 - middle)
                .abs()
                .total_cmp(&(f64::from(*b) + 0.5 - middle).abs())
        });
        for index in wanted.into_iter().take(STAND_INS_AT_ONCE - in_flight) {
            let key = Key {
                zoom: STAND_IN,
                index,
            };
            self.asked.insert(key);
            self.painter.ask(key);
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn ask_for_bands_at(&mut self, top: f64) {
        let key = self.zoom_key();
        let ratio = self.level() * 1000.0 / f64::from(key);
        let screen = self.screen_px();
        let band = f64::from(BAND_PX) * ratio;
        let first = ((top - screen) / band).floor().max(0.0) as u32;
        let last = (((top + 2.0 * screen) / band).ceil() as u32).min(self.painter.bands(key));
        let middle = (top + screen / 2.0) / band;
        let mut wanted: Vec<u32> = (first..last).collect();
        wanted.sort_by(|a, b| {
            (f64::from(*a) - middle)
                .abs()
                .total_cmp(&(f64::from(*b) - middle).abs())
        });
        for index in wanted {
            // The stand-in's asks have their own few.
            if self.asked.iter().filter(|k| k.zoom != STAND_IN).count() >= ASKED_AT_ONCE {
                break;
            }
            let key = Key { zoom: key, index };
            // A band waiting to be uploaded is neither asked nor held yet;
            // asking again drew it twice and queued both, megabytes each.
            let waiting =
                self.loading.contains(&key) || self.arrived.iter().any(|band| band.key == key);
            let held = self.textures.contains_key(&key) && !self.stale.contains(&key);
            if !waiting && !held && self.asked.insert(key) {
                self.painter.ask(key);
            }
        }
    }

    /// Where a band sits on screen now: its top and height in device pixels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn placed(&self, key: Key, height: u32) -> (f32, f32) {
        let ratio = self.level() * 1000.0 / f64::from(key.zoom);
        let top = f64::from(key.index) * f64::from(BAND_PX) * ratio - self.offset;
        (top as f32, (f64::from(height) * ratio) as f32)
    }

    /// Takes in the textures the loader has finished: a saved page's to
    /// its incoming page, this zoom's and the stand-in's to the view, and
    /// any other let go. One whose fence hasn't signalled waits.
    fn adopt_made(&mut self) {
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        for made in std::mem::take(&mut self.made) {
            match gl.adopt(made) {
                Ok((loading, texture)) if loading.version > self.painter.version => {
                    match self.incoming.as_mut() {
                        Some(incoming) if incoming.version == loading.version => {
                            incoming.loading.remove(&loading.key);
                            if loading.key.zoom == key {
                                if let Some(old) = incoming.textures.insert(loading.key, texture) {
                                    gl.forget(old);
                                }
                            } else {
                                gl.forget(texture);
                            }
                        }
                        _ => gl.forget(texture),
                    }
                }
                Ok((loading, texture)) => {
                    self.loading.remove(&loading.key);
                    // A zoom since passed, or a band of the first screen's
                    // page below where it settled, is let go, as `arrive`
                    // lets such a band go.
                    let wanted = loading.key.zoom == key || loading.key.zoom == STAND_IN;
                    let outdated = loading.version != self.painter.version
                        && self.painter.foot_pt(loading.key) > self.settled_pt;
                    if !wanted || outdated {
                        gl.forget(texture);
                        continue;
                    }
                    if let Some(measure) = self.measure.as_mut() {
                        measure.drawn_ms.push(loading.drawn_ms);
                    }
                    self.stale.remove(&loading.key);
                    self.fresh.insert(loading.key);
                    if let Some(old) = self.textures.insert(loading.key, texture) {
                        gl.forget(old);
                    }
                }
                Err(made) => self.made.push(made),
            }
        }
    }

    /// Takes the textures the loader has finished, then hands it the most
    /// wanted of the bands that have arrived. Nothing here makes a texture.
    fn upload_arrived(&mut self) {
        self.adopt_made();
        let key = self.zoom_key();
        let Some(loader) = self.loader.as_ref() else {
            return;
        };
        // A saved page's bands go before any other: until they cover the
        // view, the page shown is out of date.
        if let Some(incoming) = self.incoming.as_mut() {
            while let Some(band) = incoming.arrived.pop_front() {
                let loading = Loading {
                    key: band.key,
                    version: band.version,
                    drawn_ms: band.drawn_ms,
                };
                if loader.load(loading, band.width, band.height, band.pixels) {
                    incoming.loading.insert(band.key);
                }
            }
        }
        // This zoom's bands first, nearest the view being waited for: a held
        // jump's landing, or else the view. In arrival order, the landing's
        // last band waited behind bands for the view the jump left.
        // A held jump's whole stretch, from where it starts moving to where
        // it lands, comes before anything else.
        let (wanted, stretch) = match &self.jump {
            Some(jump) if jump.moving.is_none() => (
                jump.to,
                Some((jump.from.min(jump.to), jump.from.max(jump.to))),
            ),
            _ => (self.offset, None),
        };
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let screen = self.screen_px();
        let middle = (wanted + screen / 2.0) / band;
        let outside = |i: usize| {
            stretch.is_some_and(|(top, bottom)| {
                let index = f64::from(self.arrived[i].key.index);
                (index + 1.0) * band <= top || index * band >= bottom + screen
            })
        };
        let mut order: Vec<usize> = (0..self.arrived.len()).collect();
        order.sort_by(|&a, &b| {
            let near = |i: usize| (f64::from(self.arrived[i].key.index) + 0.5 - middle).abs();
            // The stand-in first once the window shows: a few small bands,
            // and what keeps any view from showing ground. Before it shows,
            // the first screen's own bands come first.
            let first = !self.shown && self.opened.elapsed() < FIRST_FRAME_AT_MOST;
            let rank = |i: usize| match self.arrived[i].key.zoom {
                STAND_IN => u8::from(first),
                zoom if zoom == key => u8::from(!first),
                _ => 2,
            };
            rank(a)
                .cmp(&rank(b))
                .then(outside(a).cmp(&outside(b)))
                .then(near(a).total_cmp(&near(b)))
        });
        // Before the window shows, every band that waits goes at once, since
        // only what the loader sends back wakes the first frame.
        let room = if self.shown {
            LOADING_AT_ONCE.saturating_sub(self.loading.len())
        } else {
            usize::MAX
        };
        let taken: Vec<Key> = order
            .into_iter()
            .take(room)
            .map(|i| self.arrived[i].key)
            .collect();
        for key in taken {
            let Some(i) = self.arrived.iter().position(|band| band.key == key) else {
                continue;
            };
            let band = self.arrived.remove(i).expect("an arrived band");
            let loading = Loading {
                key,
                version: band.version,
                drawn_ms: band.drawn_ms,
            };
            if loader.load(loading, band.width, band.height, band.pixels) {
                self.loading.insert(key);
            }
        }
    }

    /// Lets go of bands well out of view, and of other zooms' once this
    /// zoom's cover the view.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn evict(&mut self, covered: bool) {
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        let screen = self.screen_px() as f32;
        // A held jump's start and landing are in play as much as the view,
        // or the bands it waits for would be let go as they arrive.
        let views: Vec<f32> = match &self.jump {
            Some(jump) if jump.moving.is_none() => {
                vec![
                    0.0,
                    (jump.from - self.offset) as f32,
                    (jump.to - self.offset) as f32,
                ]
            }
            _ => vec![0.0],
        };
        let mut gone = Vec::new();
        for (k, texture) in &self.textures {
            // The stand-in stays for as long as the page does.
            if k.zoom == STAND_IN {
                continue;
            }
            let (top, height) = self.placed(*k, texture.height);
            let far = views
                .iter()
                .all(|view| top + height < view - 2.0 * screen || top > view + 3.0 * screen);
            if far || (k.zoom != key && covered) {
                gone.push(*k);
            }
        }
        let many = gone.len() >= 4;
        for k in gone {
            self.stale.remove(&k);
            if let Some(texture) = self.textures.remove(&k) {
                gl.forget(texture);
            }
        }
        // Letting go of a zoom's bands frees tens of megabytes that malloc
        // would otherwise keep: give them back as they go, not on a timer.
        if many {
            page::trim();
        }
    }

    /// Takes the measure's script a frame on, saying how long that took,
    /// or nothing once the script has ended and the reader closes.
    fn measure_step(&mut self, began: Instant) -> Option<f64> {
        let measure = self.measure.as_mut()?;
        {
            let start = *measure.start.get_or_insert(began);
            let dt = measure
                .last
                .map_or(0.0, |last| (began - last).as_secs_f64());
            measure.last = Some(began);
            let t = (began - start).as_secs_f64();
            let ended = measure
                .drawn
                .last()
                .map(|d| d.phase)
                .filter(|p| *p != phase(t));
            if let Some(ended) = ended.or((t >= RESIZE_UNTIL).then_some("resize")) {
                measure.held.push((ended, memory(), self.textures.len()));
            }
            if t >= RESIZE_UNTIL {
                self.exit = true;
                return None;
            }
            let scripting = Instant::now();
            self.script(t, dt);
            Some(scripting.elapsed().as_secs_f64() * 1000.0)
        }
    }

    /// Whether the window may show its first frame: once its screen is
    /// drawn, so it's never bare ground, each band that arrives asking
    /// again. Past `FIRST_FRAME_AT_MOST`, a view the stand-in covers will
    /// do; one nothing covers never shows, since bare ground is never drawn.
    fn first_frame_ready(&mut self) -> bool {
        {
            // Where he was, now the window's scale is known, so the first
            // bands asked are the ones he'll see.
            if let Some(pt) = self.start_pt.take() {
                self.offset = pt * self.px_per_pt();
                self.clamp();
            }
            self.upload_arrived();
            let late = self.opened.elapsed() >= FIRST_FRAME_AT_MOST;
            if !self.drawn_at(self.offset) && (!late || !self.covered()) {
                self.log_first();
                self.ask_for_bands();
                return false;
            }
            self.shown = true;
            // Search's pieces start once the first frame is up, so they
            // take nothing from it.
            self.prompt = Some(prompt::Prompt::new(self.prompt_to.clone()));
            self.read_words();
            if let Some(measure) = self.measure.as_mut() {
                measure.first_frame_ms = Some(self.opened.elapsed().as_secs_f64() * 1000.0);
            }
        }
        true
    }

    /// With `READER_FIRST_LOG`, says why the first frame waits.
    fn log_first(&self) {
        if std::env::var_os("READER_FIRST_LOG").is_some() {
            eprintln!(
                "first: {:.0} ms, missing {:?}, bands {}, end {:.0} px, screen {:.0} px, textures {:?}, loading {}, arrived {}",
                self.opened.elapsed().as_secs_f64() * 1000.0,
                self.missing_at(self.offset),
                self.painter.bands(self.zoom_key()),
                self.end_px(),
                self.screen_px(),
                self.textures
                    .keys()
                    .map(|k| (k.zoom, k.index))
                    .collect::<Vec<_>>(),
                self.loading.len(),
                self.arrived.len()
            );
        }
    }

    /// Asks for the next frame while something moves or waits to be drawn,
    /// saying whether it did, and, measuring, when this one reaches the
    /// screen.
    fn ask_next_frame(&self) -> bool {
        let surface = self.window.wl_surface();
        let asks = self.measure.is_some()
            || self.jump.is_some()
            || !self.arrived.is_empty()
            || !self.loading.is_empty()
            || !self.asked.is_empty();
        if asks {
            surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        }
        if let (Some(presentation), Some(measure)) = (&self.presentation, &self.measure) {
            presentation.feedback(surface, &self.qh, Feedback(measure.drawn.len() as u64));
        }
        asks
    }

    /// Takes in what's come for the frame: textures, and bands for the
    /// loader. The first screen's page is let go once the whole page covers
    /// the view, and a saved page takes its place once it covers it.
    fn take_in(&mut self) {
        self.upload_arrived();
        if self.held_pt.is_some() && self.whole_page_covers() {
            // At the first screen's end, the view goes on to the whole
            // page's, which is where he was.
            let at_end = self.at_end();
            self.held_pt = None;
            if at_end {
                self.offset = self.most();
                self.ask_for_bands();
            }
        }
        if self.incoming.is_some() {
            if self.incoming_covers() {
                self.take_incoming();
            } else {
                self.ask_incoming();
            }
        }
    }

    /// Each search match over the page, the one the view is on in the
    /// current match's colours.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    fn draw_marks(&self, gl: &gl::Gl, screen: f32) {
        let (true, false, Some([others, current])) =
            (self.highlighting, baseline(), self.lit.as_ref())
        else {
            return;
        };
        let px = (self.painter.scale(1000) * self.level()) as f32;
        let offset = self.offset as f32;
        let left = self.page_x();
        for (i, found) in self.found.iter().enumerate() {
            let (swatch, blend) = if Some(i) == self.current {
                current
            } else {
                others
            };
            for piece in &found.pieces {
                let top = (piece.baseline_pt - piece.size_pt * 0.92) as f32 * px - offset;
                let height = (piece.size_pt * 1.2) as f32 * px;
                if top + height < 0.0 || top > screen {
                    continue;
                }
                let width = (piece.to_pt - piece.from_pt) as f32 * px;
                gl.mark(
                    swatch,
                    blend,
                    left + piece.from_pt as f32 * px,
                    top,
                    width,
                    height,
                );
            }
        }
    }

    fn draw(&mut self) {
        let began = Instant::now();
        let waited_before = run_delay_ns();
        if !self.shown && !self.first_frame_ready() {
            return;
        }
        let mut script_ms = 0.0;
        if self.measure.is_some() {
            match self.measure_step(began) {
                Some(ms) => script_ms = ms,
                None => return,
            }
        }
        self.draw_after_measure(began, waited_before, script_ms);
    }

    /// The frame past the first frame's wait and the measure's script: the
    /// view moved, bands taken in and asked for, drawn, marked, and swapped.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn draw_after_measure(&mut self, began: Instant, waited_before: u64, script_ms: f64) {
        self.move_jump(began);
        self.clamp();
        let uploading = Instant::now();
        self.take_in();
        let upload_ms = uploading.elapsed().as_secs_f64() * 1000.0;
        let asking = Instant::now();
        self.ask_for_bands();
        let ask_ms = asking.elapsed().as_secs_f64() * 1000.0;
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        let screen = self.screen_px() as f32;
        let left = self.page_x();
        let drawing = Instant::now();
        // The first GL call after a swap, where Mesa takes the next back
        // buffer and waits if the compositor holds them all.
        gl.clear(ground());
        let clear_ms = drawing.elapsed().as_secs_f64() * 1000.0;
        // The stand-in under everything, other zooms' bands over it, and
        // this zoom's on top, so a view shows the sharpest it has.
        let mut keys: Vec<&Key> = if baseline() {
            Vec::new()
        } else {
            self.textures.keys().collect()
        };
        keys.sort_by_key(|k| (k.zoom != STAND_IN, k.zoom == key, k.index));
        let mut shown = Vec::new();
        let mut sharp = Vec::new();
        let mut drawn_now = Vec::new();
        for k in keys {
            let texture = &self.textures[k];
            let (top, height) = self.placed(*k, texture.height);
            if top + height < 0.0 || top > screen {
                continue;
            }
            let ratio = (self.level() * 1000.0 / f64::from(k.zoom)) as f32;
            gl.draw(texture, left, top, texture.width as f32 * ratio, height);
            drawn_now.push(*k);
            // A band of the first screen's page holds the page only down to
            // where it settled; below that it shows ground, so it counts so.
            // While that page is held it is the page shown, so it counts whole.
            let foot = if self.stale.contains(k) && self.held_pt.is_none() {
                let settled = self.settled_pt * self.painter.scale(k.zoom) * f64::from(ratio);
                (settled - self.offset) as f32
            } else {
                top + height
            };
            let span = (top.max(0.0), foot.min(top + height).min(screen));
            shown.push(span);
            if k.zoom == key {
                sharp.push(span);
            }
        }
        // Blank: some of the view shows only the ground. Stood in: some of
        // it shows a stand-in rather than this zoom's own bands.
        let page_end = (self.end_px() - self.offset) as f32;
        let draw_ms = drawing.elapsed().as_secs_f64() * 1000.0;
        let fresh = drawn_now.iter().filter(|k| self.fresh.remove(k)).count();
        let blank = !covers(&mut shown, screen.min(page_end));
        let stood_in = !blank && !covers(&mut sharp, screen.min(page_end));
        // Other zooms' bands go only once this zoom's alone cover the view;
        // the stand-in never does.
        // counting the others in let the first new band send them all away.
        let covered = self.drawn_at(self.offset);
        let evicting = Instant::now();
        self.evict(covered);
        let evict_ms = evicting.elapsed().as_secs_f64() * 1000.0;
        let Some(gl) = self.gl.as_ref() else { return };
        // The matches over the page, then the prompt over its foot.
        self.draw_marks(gl, screen);
        self.draw_labels();
        let Some(gl) = self.gl.as_ref() else { return };
        if self.prompt_shown
            && let Some(canvas) = painted(gl, &mut self.prompt_canvas, self.prompt_set.take())
        {
            let (width, height) = (canvas.width as f32, canvas.height as f32);
            gl.draw(canvas, 0.0, screen - height, width, height);
        }
        // Centred, as his which-key, on whole pixels so its type is sharp.
        if self.keys_shown
            && let Some(canvas) = painted(gl, &mut self.keys_canvas, self.keys_set.take())
        {
            let (width, height) = (canvas.width as f32, canvas.height as f32);
            let x = ((gl.width as f32 - width) / 2.0).round();
            let y = ((screen - height) / 2.0).round();
            gl.draw(canvas, x, y, width, height);
        }
        if self.ask_next_frame() {
            self.frame_pending = true;
        }
        let swapping = Instant::now();
        if let Err(error) = gl.swap() {
            eprintln!("can't show the frame: {error}");
        }
        let swap_ms = swapping.elapsed().as_secs_f64() * 1000.0;
        if let Some(measure) = self.measure.as_mut() {
            let t = (began - measure.start.unwrap_or(began)).as_secs_f64();
            measure.drawn.push(Drawn {
                phase: phase(t),
                t,
                work_ms: began.elapsed().as_secs_f64() * 1000.0,
                parts_ms: [
                    upload_ms, evict_ms, swap_ms, ask_ms, script_ms, draw_ms, clear_ms,
                ],
                waited_ms: run_delay_ns().saturating_sub(waited_before) as f64 / 1e6,
                blank,
                stood_in,
                fresh,
                presented_ns: None,
            });
        }
    }

    /// The whole page has been set: draw from it, keeping each band above
    /// where the first screen's page settled and redrawing the rest in place.
    /// A save's page, set: it draws while the page before shows, from where
    /// the view's top words are on it.
    /// A page the reloader set: the file shown, saved, or the file being
    /// opened in its place. One of a file no longer wanted is let go.
    fn saved(&mut self, (file, set): Reset) {
        let file = std::path::PathBuf::from(file);
        let opens = match &self.following {
            Some(following) if following.file == file => true,
            None if file == self.file => false,
            _ => return,
        };
        let (page, ms, lines) = match set {
            Ok(set) => set,
            Err(error) => {
                eprintln!("reader: can't set {}: {error}", file.display());
                if opens {
                    // Left where it was, and saves of it heard again.
                    self.following = None;
                    self.set_on(&self.file);
                }
                return;
            }
        };
        // One set for a width the window has since left is let go, since
        // the reloader sets it again at the new one.
        if !self.fits(&page) {
            return;
        }
        self.let_incoming_go();
        let px_per_pt = self.px_per_pt();
        let screen_pt = self.screen_px() / px_per_pt;
        let new_most_pt = (page.frame.size().y.to_pt() - screen_pt).max(0.0);
        let offset_pt = if opens {
            self.landing(&file, &page, &lines, new_most_pt)
        } else {
            let at_end = self.at_end();
            let top_pt = self.offset / px_per_pt;
            match &self.words {
                Some(old) => anchored(old, &lines, top_pt, at_end, new_most_pt),
                None => top_pt.min(new_most_pt),
            }
        };
        let version = self.next_version.max(self.painter.version + 1);
        self.next_version = version + 1;
        self.incoming = Some(Incoming {
            version,
            page: std::sync::Arc::new(page),
            offset: offset_pt * px_per_pt,
            lines,
            textures: HashMap::new(),
            asked: HashSet::new(),
            loading: HashSet::new(),
            arrived: VecDeque::new(),
            since: Instant::now(),
            set_ms: ms,
            opens: opens.then_some(file),
        });
        self.ask_incoming();
    }

    /// Where the view lands on a file opened in this one's place, in
    /// points: the heading a link names, the place a jump left, or where
    /// he last left the file, and otherwise its top.
    fn landing(
        &self,
        file: &std::path::Path,
        page: &typst_layout::Page,
        lines: &[text::Line],
        most_pt: f64,
    ) -> f64 {
        let Some(following) = self.following.as_ref() else {
            return 0.0;
        };
        let pt = match &following.landing {
            Landing::Heading(anchor) => links::heading_at(lines, anchor).unwrap_or_else(|| {
                eprintln!("reader: no heading in {} is #{anchor}", file.display());
                0.0
            }),
            Landing::At(pt) => *pt,
            Landing::Kept => match place::read(file) {
                Some(kept) if kept.at_end => most_pt,
                Some(kept) => place::pt_of(&place::starts(&page.frame), kept.block, kept.into_pt)
                    .unwrap_or(0.0),
                None => 0.0,
            },
        };
        pt.clamp(0.0, most_pt)
    }

    /// The bands the saved page needs to cover the view where it will show,
    /// at the zoom shown.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn ask_incoming(&mut self) {
        let key = self.zoom_key();
        let screen = self.screen_px();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let scale = self.painter.scale(key);
        let Some(incoming) = self.incoming.as_mut() else {
            return;
        };
        let height = incoming.page.frame.size().y.to_pt() * scale;
        let first = (incoming.offset / band).floor().max(0.0) as u32;
        let last =
            (((incoming.offset + screen) / band).ceil() as u32).min((height / band).ceil() as u32);
        for index in first..last {
            let k = Key { zoom: key, index };
            let have = incoming.textures.contains_key(&k)
                || incoming.loading.contains(&k)
                || incoming.arrived.iter().any(|b| b.key == k);
            if !have && incoming.asked.insert(k) {
                self.painter
                    .ask_from(std::sync::Arc::clone(&incoming.page), incoming.version, k);
            }
        }
    }

    /// Whether the saved page's bands cover the view where it will show.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn incoming_covers(&self) -> bool {
        let Some(incoming) = self.incoming.as_ref() else {
            return false;
        };
        let key = self.zoom_key();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let height = incoming.page.frame.size().y.to_pt() * self.painter.scale(key);
        let first = (incoming.offset / band).floor().max(0.0) as u32;
        let last = (((incoming.offset + self.screen_px()) / band).ceil() as u32)
            .min((height / band).ceil() as u32);
        (first..last).all(|index| incoming.textures.contains_key(&Key { zoom: key, index }))
    }

    /// The saved page takes the page's place, whole, with the view on the
    /// same words, and everything of the page before is let go.
    fn take_incoming(&mut self) {
        let Some(incoming) = self.incoming.take() else {
            return;
        };
        // Another file's page: where the view was on this one is kept for
        // the next time it's shown, from its own page, before it goes.
        if incoming.opens.is_some() && !self.measuring {
            self.keep_place();
        }
        if let Some(gl) = self.gl.as_ref() {
            for (_, texture) in self.textures.drain() {
                gl.forget(texture);
            }
        }
        self.stale.clear();
        self.fresh.clear();
        self.asked.clear();
        self.arrived.clear();
        self.held_pt = None;
        self.settled_pt = f64::NEG_INFINITY;
        self.reloaded = true;
        self.jump = None;
        self.fresh.extend(incoming.textures.keys().copied());
        self.textures = incoming.textures;
        // Set again at another width, the page's blocks have moved, so each
        // mark moves with the block it was in. A save keeps them where they
        // were, as the words below them may have changed.
        let reflowed = incoming.opens.is_none()
            && (incoming.page.frame.size().x.to_pt() - self.painter.width_pt()).abs() >= 0.01;
        let before = reflowed.then(|| place::starts(&self.painter.page().frame));
        self.painter.take(incoming.page, incoming.version);
        if let Some(before) = before {
            let after = place::starts(&self.painter.page().frame);
            for pt in self.marks.values_mut() {
                let (block, into_pt) = place::at(&before, *pt);
                *pt = place::pt_of(&after, block, into_pt).unwrap_or(*pt);
            }
        }
        self.offset = incoming.offset;
        self.clamp();
        if keys_logged() && incoming.opens.is_none() {
            eprintln!(
                "saved: set {:.0} ms, shown {:.0} ms after, heading {:.1} pt",
                incoming.set_ms,
                incoming.since.elapsed().as_secs_f64() * 1000.0,
                self.offset / self.px_per_pt()
            );
        }
        self.words = Some(incoming.lines);
        if let Some(file) = incoming.opens {
            self.shown_now(file);
        }
        self.refind();
    }

    /// Another file is the one shown: its marks come back, '' starts over,
    /// and the launcher's Recent is told.
    fn shown_now(&mut self, file: std::path::PathBuf) {
        self.following = None;
        self.marks.clear();
        self.kept_marks = place::read(&file)
            .map(|kept| kept.marks)
            .unwrap_or_default();
        self.take_kept_marks();
        self.last_jump = None;
        if !self.measuring
            && let Err(error) = showing::keep(&file)
        {
            eprintln!("reader: can't say {} is shown: {error}", file.display());
        }
        if !self.measuring && !start::is(&file) {
            showing::opened(&file);
        }
        if keys_logged() {
            eprintln!(
                "opened: {}, heading {:.1} pt",
                file.display(),
                self.offset / self.px_per_pt()
            );
        }
        self.file = file;
    }

    /// Takes back the marks kept from the last time whose blocks the page
    /// shown has.
    fn take_kept_marks(&mut self) {
        if self.kept_marks.is_empty() {
            return;
        }
        let starts = place::starts(&self.painter.page().frame);
        let marks = &mut self.marks;
        self.kept_marks.retain(|(letter, block, into_pt)| {
            match place::pt_of(&starts, *block, *into_pt) {
                Some(pt) => {
                    marks.entry(*letter).or_insert(pt);
                    false
                }
                None => true,
            }
        });
    }

    /// Keeps where the view is on the file shown, and its marks, for the
    /// next time it opens.
    fn keep_place(&self) {
        if let Err(error) = place::keep_window(f64::from(self.size.0) * PT_A_PX) {
            eprintln!("reader: can't keep the window's width: {error}");
        }
        let file = self.file.as_path();
        let starts = place::starts(&self.painter.page().frame);
        // Closed while a jump moves, it keeps where the keys sent it.
        let here = self.jump.as_ref().map_or(self.offset, |jump| jump.to);
        let (block, into_pt) = place::at(&starts, here / self.px_per_pt());
        let marks = self
            .marks
            .iter()
            .map(|(letter, pt)| {
                let (block, into_pt) = place::at(&starts, *pt);
                (*letter, block, into_pt)
            })
            .collect();
        let at_end = self.most() > 0.0 && here >= self.most() - 1.0;
        if let Err(error) = place::write(
            file,
            &place::Place {
                block,
                into_pt,
                at_end,
                marks,
            },
        ) {
            eprintln!("reader: can't keep where {} was: {error}", file.display());
        }
    }

    /// Whether a page was set at the width the window asks for now.
    fn fits(&self, page: &typst_layout::Page) -> bool {
        (page.frame.size().x.to_pt() - self.width.page_pt()).abs() < 0.01
    }

    fn replace(&mut self, page: typst_layout::Page, ms: f64) {
        // A save set since is newer than the rest of the first set, and
        // a window resized since wants the whole page at its own width.
        if self.reloaded || self.incoming.is_some() || !self.fits(&page) {
            return;
        }
        self.held_pt = Some(self.painter.height_pt());
        self.settled_pt = self.painter.height_pt() - page::BOTTOM_PT - 1.0;
        self.painter.replace(page);
        let settled = self.settled_pt;
        let below: Vec<Key> = self
            .textures
            .keys()
            .filter(|k| self.painter.foot_pt(**k) > settled)
            .copied()
            .collect();
        self.stale.extend(below);
        self.take_kept_marks();
        let painter = &self.painter;
        self.asked.retain(|k| painter.foot_pt(*k) <= settled);
        self.arrived
            .retain(|band| painter.foot_pt(band.key) <= settled);
        if let Some(measure) = self.measure.as_mut() {
            measure.rest_ms = Some(ms);
        }
        self.read_words();
        if !self.frame_pending {
            self.draw();
        }
    }

    fn arrive(&mut self, band: Band) {
        // A saved page's band waits with the saved page; one of a save set
        // over since, or of a page let go, is let go.
        if band.version > self.painter.version {
            let key = self.zoom_key();
            if let Some(incoming) = self.incoming.as_mut()
                && incoming.version == band.version
            {
                incoming.asked.remove(&band.key);
                if band.key.zoom == key {
                    incoming.arrived.push_back(band);
                }
                if !self.frame_pending {
                    self.draw();
                }
            }
            return;
        }
        // Drawn from the first screen's page below where it settled: wrong
        // now, and asked again already.
        if band.version != self.painter.version && self.painter.foot_pt(band.key) > self.settled_pt
        {
            return;
        }
        self.asked.remove(&band.key);
        // A band for a zoom since passed would only wait to be uploaded and
        // then let go: a glide's stale bands held 500 MB that way. The
        // stand-in's are wanted at any zoom.
        let key = self.zoom_key();
        let wanted = |zoom: u32| zoom == key || zoom == STAND_IN;
        self.arrived.retain(|queued| wanted(queued.key.zoom));
        let had = (self.textures.contains_key(&band.key) && !self.stale.contains(&band.key))
            || self.loading.contains(&band.key)
            || self.arrived.iter().any(|queued| queued.key == band.key);
        if wanted(band.key.zoom) && !had {
            self.arrived.push_back(band);
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn report(&self) {
        let Some(measure) = self.measure.as_ref() else {
            return;
        };
        // A compositor without a fixed refresh, as a headless one, gives
        // none, so the median interval stands in for it.
        let all: Vec<f64> = measure
            .drawn
            .windows(2)
            .filter_map(|pair| Some((pair[1].presented_ns? - pair[0].presented_ns?) as f64 / 1e6))
            .collect();
        let refresh_given = measure.refresh_ns > 0;
        let refresh_ms = if refresh_given {
            f64::from(measure.refresh_ns) / 1e6
        } else {
            percentile(&all, 0.5)
        };
        let mut phases = serde_json::Map::new();
        for name in ["scroll", "jump", "zoom", "resize"] {
            phases.insert(
                name.to_string(),
                Self::phase_report(measure, name, refresh_ms),
            );
        }
        let worst = phases
            .values()
            .map(|p| p["interval_ms_p99"].as_f64().unwrap_or(f64::MAX))
            .fold(0.0, f64::max);
        let report = serde_json::json!({
            "refresh_ms": refresh_ms,
            "refresh_given": refresh_given,
            "scale": self.scale,
            "size": [self.size.0, self.size.1],
            "set_ms": measure.set_ms,
            "set_parts": measure.set_parts.iter().map(|(name, ms, waited)| serde_json::json!({"part": name, "ms": (ms * 10.0).round() / 10.0, "waited_ms": (waited * 10.0).round() / 10.0})).collect::<Vec<_>>(),
            "configured_ms": measure.configured_ms,
            "rest_set_ms": measure.rest_ms,
            "first_frame_ms": measure.first_frame_ms,
            "band_drawn_ms_p50": percentile(&measure.drawn_ms, 0.50),
            "band_drawn_ms_p99": percentile(&measure.drawn_ms, 0.99),
            "discarded": measure.discarded,
            "jumps_missing": measure.missing,
            "jumps_held_ms": measure.held_ms.iter().map(|ms| (ms * 10.0).round() / 10.0).collect::<Vec<_>>(),
            "configures_while_resizing": measure.configures,
            "worst_p99_interval_ms": worst,
            "within_one_refresh": refresh_ms > 0.0 && worst <= refresh_ms * 1.05,
            "phases": phases,
            "memory_mb": memory(),
            "held_at_each_end": measure.held.iter().map(|(p, m, n)| serde_json::json!({"phase": p, "memory_mb": m, "bands": n})).collect::<Vec<_>>(),
        });
        println!("{report}");
    }

    /// One phase of the measure's script: its frames, when each reached
    /// the screen against the refresh, and what each frame's work took.
    #[expect(
        clippy::cast_precision_loss,
        reason = "nanoseconds between frames, which never pass a mantissa's 52 bits"
    )]
    fn phase_report(measure: &Measure, name: &str, refresh_ms: f64) -> serde_json::Value {
        let frames: Vec<(usize, &Drawn)> = measure
            .drawn
            .iter()
            .enumerate()
            .filter(|(_, d)| d.phase == name)
            .collect();
        let mut intervals = Vec::new();
        for (i, d) in &frames {
            if *i == 0 {
                continue;
            }
            if let (Some(now), Some(before)) = (d.presented_ns, measure.drawn[i - 1].presented_ns) {
                intervals.push((now.saturating_sub(before)) as f64 / 1e6);
            }
        }
        let work: Vec<f64> = frames.iter().map(|(_, d)| d.work_ms).collect();
        let drops = intervals
            .iter()
            .filter(|ms| **ms > 1.5 * refresh_ms)
            .count();
        let blank = frames.iter().filter(|(_, d)| d.blank).count();
        let stood_in = frames.iter().filter(|(_, d)| d.stood_in).count();
        let part = |n: usize| {
            let all: Vec<f64> = frames.iter().map(|(_, d)| d.parts_ms[n]).collect();
            percentile(&all, 1.0)
        };
        let worst_frame = frames
                .iter()
                .max_by(|a, b| a.1.work_ms.total_cmp(&b.1.work_ms))
                .map(|(_, d)| serde_json::json!({"t": d.t, "work_ms": d.work_ms, "upload_ms": d.parts_ms[0], "evict_ms": d.parts_ms[1], "swap_ms": d.parts_ms[2], "ask_ms": d.parts_ms[3], "script_ms": d.parts_ms[4], "draw_ms": d.parts_ms[5], "clear_ms": d.parts_ms[6], "waited_ms": d.waited_ms, "fresh": d.fresh}));
        let fresh_max = frames.iter().map(|(_, d)| d.fresh).max().unwrap_or(0);
        let slow_fresh = frames
            .iter()
            .filter(|(_, d)| d.parts_ms[5] > 4.0)
            .map(|(_, d)| d.fresh)
            .collect::<Vec<_>>();
        let waited: Vec<f64> = frames.iter().map(|(_, d)| d.waited_ms).collect();
        serde_json::json!({
            "frames": frames.len(),
            "presented": intervals.len(),
            "interval_ms_p50": percentile(&intervals, 0.50),
            "interval_ms_p99": percentile(&intervals, 0.99),
            "interval_ms_max": percentile(&intervals, 1.0),
            "dropped": drops,
            "work_ms_p50": percentile(&work, 0.50),
            "work_ms_p99": percentile(&work, 0.99),
            "work_ms_max": percentile(&work, 1.0),
            "upload_ms_max": part(0),
            "evict_ms_max": part(1),
            "swap_ms_max": part(2),
            "ask_ms_max": part(3),
            "script_ms_max": part(4),
            "draw_ms_max": part(5),
            "clear_ms_max": part(6),
            "fresh_max": fresh_max,
            "fresh_in_slow_draws": slow_fresh,
            "waited_ms_p99": percentile(&waited, 0.99),
            "waited_ms_max": percentile(&waited, 1.0),
            "worst_frame": worst_frame,
            "blank_frames": blank,
            "stand_in_frames": stood_in,
            "blank_at_s": frames.iter().filter(|(_, d)| d.blank).map(|(_, d)| (d.t * 1000.0).round() / 1000.0).collect::<Vec<_>>(),
        })
    }
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

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
)]
fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    (sorted[i] * 100.0).round() / 100.0
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

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        // Under a fractional scale the compositor says the scale exactly,
        // and this whole one is only rounded.
        if self.fractional.is_some() || surface != self.window.wl_surface() {
            return;
        }
        self.rescale(f64::from(factor));
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.frame_pending = false;
        self.draw();
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

/// A key as the reader's table knows it: by what it types, so Colemak's
/// letters are vi's, with Ctrl named apart since it changes what's typed.
/// How far the page zooms: half its own size, and three times it.
const ZOOM_LEAST: f64 = 0.5;
const ZOOM_MOST: f64 = 3.0;

fn key_of(event: &KeyEvent, ctrl: bool) -> Option<keys::Key> {
    Some(match event.keysym {
        Keysym::Escape => keys::Key::Escape,
        Keysym::Return | Keysym::KP_Enter => keys::Key::Return,
        Keysym::BackSpace => keys::Key::Backspace,
        Keysym::Up => keys::Key::Up,
        Keysym::Down => keys::Key::Down,
        Keysym::Left => keys::Key::Left,
        Keysym::Right => keys::Key::Right,
        Keysym::Page_Up => keys::Key::PageUp,
        Keysym::Page_Down => keys::Key::PageDown,
        sym => {
            let typed = sym.key_char()?;
            if ctrl {
                keys::Key::Ctrl(typed.to_ascii_lowercase())
            } else {
                keys::Key::Char(typed)
            }
        }
    })
}

impl App {
    fn key(&mut self, event: &KeyEvent) {
        if let Some(key) = key_of(event, self.ctrl) {
            if self.labelling.is_some() {
                self.label_key(key);
                return;
            }
            if let Some(typing) = self.typing.as_mut() {
                let said = typing.key(key);
                self.typed(said);
                return;
            }
            if let Some((act, count)) = self.keys.press(key) {
                self.act(act, count);
            }
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let repeat = Box::new(
                |app: &mut App, _: &wl_keyboard::WlKeyboard, event: KeyEvent| app.key(&event),
            );
            match self.seats.get_keyboard_with_repeat(
                qh,
                &seat,
                None,
                self.loop_handle.clone(),
                repeat,
            ) {
                Ok(keyboard) => {
                    if keys_logged() {
                        eprintln!("keys: a keyboard");
                    }
                    self.keyboard = Some(keyboard);
                }
                Err(error) => eprintln!("no keys: {error}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        if keys_logged() {
            eprintln!("keys: focused");
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        // A key waiting for its second is dropped when focus goes, as vi's
        // would be by anything else typed.
        self.keys = keys::Machine::default();
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(&event);
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(&event);
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.ctrl = modifiers.ctrl;
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        let (width, height) = configure.new_size;
        let at_end = self.shown && self.at_end();
        self.size = (
            width.map_or(960, std::num::NonZero::get),
            height.map_or(900, std::num::NonZero::get),
        );
        self.fit_window();
        let (w, h) = self.buffer_size();
        if let Some(measure) = self.measure.as_mut()
            && measure.configured_ms.is_none()
        {
            measure.configured_ms = Some(self.opened.elapsed().as_secs_f64() * 1000.0);
        }
        if let Some(measure) = self.measure.as_mut()
            && measure
                .start
                .is_some_and(|s| s.elapsed().as_secs_f64() >= ZOOM_UNTIL)
        {
            measure.configures += 1;
        }
        if let Some(gl) = self.gl.as_mut() {
            gl.resize(w, h);
        } else {
            window.wl_surface().set_buffer_scale(self.buffer_scale());
            // SAFETY: the display is the connection's, which outlives
            // the window and its GL.
            let cut = self.viewport.is_some();
            match unsafe { gl::Gl::new(self.display, window.wl_surface(), w, h, cut) } {
                Ok(gl) => {
                    match gl.loader(self.made_to.clone()) {
                        Ok(loader) => self.loader = Some(loader),
                        Err(error) => {
                            eprintln!("can't draw: {error}");
                            self.exit = true;
                            return;
                        }
                    }
                    let make = |(ground, ink): ([f32; 3], [f32; 3])| {
                        let blend = gl::Blend::between(self::ground(), page_ink(), ground, ink);
                        gl.swatch(blend.source).ok().map(|swatch| (swatch, blend))
                    };
                    self.lit = make(search_marks())
                        .zip(make(current_marks()))
                        .map(|(a, b)| [a, b]);
                    self.gl = Some(gl);
                }
                Err(error) => {
                    eprintln!("can't draw: {error}");
                    self.exit = true;
                    return;
                }
            }
        }
        // The view's end moves with the buffer's height, so a view at the
        // end keeps to it once the buffer is the window's new size.
        if at_end {
            self.offset = self.most();
        }
        self.cut();
        if !self.frame_pending {
            self.draw();
        }
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);

struct Presentation;

/// The viewporter and its viewports send nothing.
struct Viewporting;

impl Dispatch2<WpViewporter, App> for Viewporting {
    fn event(
        &self,
        _: &mut App,
        _: &WpViewporter,
        _: wp_viewporter::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpViewport, App> for Viewporting {
    fn event(
        &self,
        _: &mut App,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

/// The compositor's scale for the window, in 120ths.
struct Scaling;

impl Dispatch2<WpFractionalScaleManagerV1, App> for Scaling {
    fn event(
        &self,
        _: &mut App,
        _: &WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpFractionalScaleV1, App> for Scaling {
    fn event(
        &self,
        app: &mut App,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            app.rescale(f64::from(scale) / 120.0);
        }
    }
}

impl Dispatch2<WpPresentation, App> for Presentation {
    fn event(
        &self,
        _: &mut App,
        _: &WpPresentation,
        _: wp_presentation::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

/// Which frame a presentation feedback is for.
struct Feedback(u64);

impl Dispatch2<WpPresentationFeedback, App> for Feedback {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn event(
        &self,
        app: &mut App,
        _: &WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let Some(measure) = app.measure.as_mut() else {
            return;
        };
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                ..
            } => {
                let seconds = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                if let Some(drawn) = measure.drawn.get_mut(self.0 as usize) {
                    drawn.presented_ns = Some(seconds * 1_000_000_000 + u64::from(tv_nsec));
                }
                if refresh > 0 {
                    measure.refresh_ns = refresh;
                }
            }
            wp_presentation_feedback::Event::Discarded => measure.discarded += 1,
            _ => (),
        }
    }
}

/// The width the first screen is set at: the window's when the reader last
/// closed, which niri most likely opens it at again, so the first screen
/// seldom sets twice. Measuring sets the whole page each time.
/// Where on the first page the view opens for a place kept, in points.
/// Kept at the end, it opens at the end, however the window or the page
/// has changed since: the first frame clamps it there.
fn start_at(place: &place::Place, starts: &[(usize, f64)]) -> Option<f64> {
    if place.at_end {
        Some(f64::INFINITY)
    } else {
        place::pt_of(starts, place.block, place.into_pt)
    }
}

fn first_width(measuring: bool) -> page::Width {
    if measuring {
        page::Width::WHOLE
    } else {
        place::window_pt().map_or(page::Width::WHOLE, page::Width::for_window)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (measuring, file) = match args.as_slice() {
        [flag] if flag == "--keys" => {
            print_keys();
            return Ok(());
        }
        [flag, file] if flag == "--measure" => (true, file.clone()),
        [file] => (false, file.clone()),
        // Started with nothing to show, as from the launcher's Programs, it
        // shows what there is to read.
        [] => (false, start::page()?.to_string_lossy().into_owned()),
        _ => return Err("usage: reader [--measure] FILE, or reader --keys".into()),
    };
    let born = Instant::now();
    let scale: f64 = std::env::var("READER_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2.0);
    // Where he was when he last closed it, unless measuring, which starts
    // at the top each time.
    let kept = if measuring {
        None
    } else {
        place::read(std::path::Path::new(&file))
    };
    let width = first_width(measuring);
    let first = page::set_first(&file, kept.as_ref().map(|place| place.block), width)?;
    let starts = place::starts(&first.page.frame);
    let start_pt = kept.as_ref().and_then(|place| start_at(place, &starts));
    let kept_marks = kept.map(|place| place.marks).unwrap_or_default();
    let page::First {
        page,
        ms: set_ms,
        rest,
        parts: set_parts,
    } = first;

    let conn = Connection::connect_to_env()?;
    let (globals, queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    WaylandSource::new(conn.clone(), queue)
        .insert(event_loop.handle())
        .map_err(|e| format!("the loop: {e}"))?;
    let handle = event_loop.handle();
    let senders = Senders {
        done: heard(&handle, App::arrive)?,
        made_to: heard(&handle, |app: &mut App, made| {
            app.made.push(made);
            if !app.frame_pending {
                app.draw();
            }
        })?,
        words_to: heard(&handle, |app: &mut App, (version, lines)| {
            app.words_read(version, lines);
        })?,
        prompt_to: heard(&handle, App::prompt_arrived)?,
    };
    let surface = surface(&globals, &qh)?;
    let opening = Opening {
        file: std::path::PathBuf::from(&file),
        page,
        born,
        scale,
        measuring,
        start_pt,
        kept_marks,
        set_ms,
        set_parts,
        width,
    };
    let mut app = App::new(
        &globals,
        qh,
        &conn,
        handle.clone(),
        surface,
        senders,
        opening,
    );
    if let Some(rest) = rest {
        set_rest(&handle, rest)?;
    }
    let (reload_to, watch) = reload_on_save(&handle, &file, width)?;
    app.reload_to = Some(reload_to);
    app.watch = watch;
    app.take_kept_marks();
    if !measuring && let Err(error) = showing::keep(std::path::Path::new(&file)) {
        eprintln!("reader: can't say {file} is shown: {error}");
    }
    let mut ended = Ok(());
    while !app.exit && ended.is_ok() {
        ended = event_loop.dispatch(None, &mut app);
    }
    // Kept before anything is let go, so however the reader ends from
    // here, where it was is never lost.
    if !measuring {
        app.keep_place();
        showing::forget();
    }
    ended?;
    // Let the last frames' feedback in before reading it.
    let _ = event_loop.dispatch(std::time::Duration::from_millis(100), &mut app);
    app.report();
    Ok(())
}

impl Drop for App {
    /// Lets go of the loader, waiting for it to leave the driver, before
    /// the GL it shares; the window and the Wayland connection go after.
    /// Left to exit, the driver's own teardown freed what a loader still
    /// making a texture used, and it died in Mesa.
    fn drop(&mut self) {
        self.loader = None;
        self.gl = None;
    }
}

/// Prints every chord the reader takes, with its group and what it does,
/// for the launcher's list to be checked against.
fn print_keys() {
    let keys: Vec<serde_json::Value> = keys::chords()
        .into_iter()
        .map(|(group, chord, does)| serde_json::json!({"group": group, "chord": chord, "does": does}))
        .collect();
    // A reader of the list that stops early, as head does, ends it quietly.
    let _ = writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::Value::Array(keys)
    );
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

/// A channel whose every message is handed to the app on the loop.
fn heard<T: 'static>(
    handle: &LoopHandle<'static, App>,
    mut then: impl FnMut(&mut App, T) + 'static,
) -> Result<channel::Sender<T>, String> {
    let (to, from) = channel::channel::<T>();
    handle
        .insert_source(from, move |event, (), app: &mut App| {
            if let Event::Msg(it) = event {
                then(app, it);
            }
        })
        .map_err(|e| format!("the loop: {e}"))?;
    Ok(to)
}

/// The window, and what the compositor lends it: when each frame showed, a
/// viewport to cut the buffer to it, and its fractional scale.
struct Surface {
    window: Window,
    presentation: Option<WpPresentation>,
    viewport: Option<WpViewport>,
    fractional: Option<WpFractionalScaleV1>,
}

fn surface(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<App>,
) -> Result<Surface, Box<dyn std::error::Error>> {
    let compositor = CompositorState::bind(globals, qh)?;
    let shell = XdgShell::bind(globals, qh)?;
    let presentation = globals
        .bind::<WpPresentation, App, Presentation>(qh, 1..=2, Presentation)
        .ok();
    let surface = compositor.create_surface(qh);
    let viewport = globals
        .bind::<WpViewporter, App, Viewporting>(qh, 1..=1, Viewporting)
        .ok()
        .map(|viewporter| viewporter.get_viewport(&surface, qh, Viewporting));
    // A fractional scale is drawn through the viewport, so it needs one.
    let fractional = viewport.as_ref().and_then(|_| {
        globals
            .bind::<WpFractionalScaleManagerV1, App, Scaling>(qh, 1..=1, Scaling)
            .ok()
            .map(|manager| manager.get_fractional_scale(&surface, qh, Scaling))
    });
    let window = shell.create_window(surface, WindowDecorations::ServerDefault, qh);
    window.set_title("Reader");
    window.set_app_id("reader");
    window.commit();
    Ok(Surface {
        window,
        presentation,
        viewport,
        fractional,
    })
}

/// Where the app sends what its threads make: bands drawn, textures made,
/// the words of a page, and the prompt set.
struct Senders {
    done: channel::Sender<Band>,
    made_to: channel::Sender<gl::Made<Loading>>,
    words_to: channel::Sender<(u32, Vec<text::Line>)>,
    prompt_to: channel::Sender<prompt::Set>,
}

/// How the reader opened: the first screen's page, when, at what scale,
/// whether it measures, and where it was last closed.
struct Opening {
    file: std::path::PathBuf,
    page: typst_layout::Page,
    born: Instant,
    scale: f64,
    measuring: bool,
    start_pt: Option<f64>,
    kept_marks: Vec<(char, usize, f64)>,
    set_ms: f64,
    set_parts: Vec<(&'static str, f64, f64)>,
    width: page::Width,
}

impl Measure {
    fn new(set_ms: f64, set_parts: Vec<(&'static str, f64, f64)>) -> Self {
        Measure {
            start: None,
            last: None,
            drawn: Vec::new(),
            refresh_ns: 0,
            discarded: 0,
            configures: 0,
            jumps: 0,
            drawn_ms: Vec::new(),
            set_ms,
            set_parts,
            configured_ms: None,
            rest_ms: None,
            first_frame_ms: None,
            held: Vec::new(),
            held_ms: Vec::new(),
            missing: Vec::new(),
        }
    }
}

impl App {
    fn new(
        globals: &wayland_client::globals::GlobalList,
        qh: QueueHandle<App>,
        conn: &Connection,
        loop_handle: LoopHandle<'static, App>,
        surface: Surface,
        senders: Senders,
        opening: Opening,
    ) -> App {
        let Opening {
            file,
            page,
            born,
            scale,
            measuring,
            start_pt,
            kept_marks,
            set_ms,
            set_parts,
            width,
        } = opening;
        App {
            seats: SeatState::new(globals, &qh),
            registry: RegistryState::new(globals),
            outputs: OutputState::new(globals, &qh),
            window: surface.window,
            qh,
            display: conn.backend().display_ptr().cast(),
            presentation: surface.presentation,
            viewport: surface.viewport,
            gl: None,
            painter: Painter::new(page, REFERENCE_SCALE / 0.75, senders.done),
            size: (960, 900),
            scale,
            fractional: surface.fractional,
            offset: 0.0,
            zoom: 1.0,
            textures: HashMap::new(),
            loader: None,
            made_to: senders.made_to,
            loading: HashSet::new(),
            made: Vec::new(),
            fresh: HashSet::new(),
            asked: HashSet::new(),
            arrived: VecDeque::new(),
            frame_pending: false,
            shown: false,
            opened: born,
            settled_pt: f64::INFINITY,
            held_pt: None,
            stale: HashSet::new(),
            jump: None,
            reduced_motion: std::env::var_os("READER_REDUCED_MOTION").is_some_and(|v| v == "1"),
            measure: measuring.then(|| Measure::new(set_ms, set_parts)),
            exit: false,
            keyboard: None,
            loop_handle,
            keys: keys::Machine::default(),
            ctrl: false,
            marks: HashMap::new(),
            start_pt,
            kept_marks,
            jumps: Vec::new(),
            jump_at: 0,
            last_jump: None,
            words: None,
            incoming: None,
            next_version: 1,
            reloaded: false,
            width,
            words_to: senders.words_to,
            typing: None,
            searched: None,
            found: Vec::new(),
            current: None,
            highlighting: false,
            lit: None,
            prompt: None,
            prompt_to: senders.prompt_to,
            prompt_serial: 0,
            prompt_shown: false,
            prompt_set: None,
            prompt_canvas: None,
            keys_shown: false,
            keys_px: 0.0,
            keys_set: None,
            keys_canvas: None,
            file,
            following: None,
            reload_to: None,
            watch: None,
            measuring,
            labelling: None,
            labels_set: None,
            label_canvases: Vec::new(),
        }
    }
}

/// Sets the rest of the file on a thread of its own while the first screen
/// shows; it comes back through the loop.
fn set_rest(
    handle: &LoopHandle<'static, App>,
    rest: page::Rest,
) -> Result<(), Box<dyn std::error::Error>> {
    let whole = heard(handle, |app: &mut App, (page, ms)| app.replace(page, ms))?;
    std::thread::Builder::new()
        .name("setter".into())
        .spawn(move || match rest.set() {
            Ok(done) => {
                let _ = whole.send(done);
            }
            Err(error) => eprintln!("can't set the rest: {error}"),
        })?;
    Ok(())
}

/// Sets the file again on a thread of its own at each save, or once a
/// batch of web pictures has come, and sets another file when a link or a
/// jump opens one in the window. The thread's setter is made at the first
/// ask, so it costs the first screen nothing, and asks that come while one
/// sets are taken as one: the newest file asked for, as it is now, at the
/// newest width. Gives back what asks it, and the watch on the file's
/// saves, which moves with the file shown.
fn reload_on_save(
    handle: &LoopHandle<'static, App>,
    file: &str,
    width: page::Width,
) -> Result<(std::sync::mpsc::Sender<Reload>, Option<watch::Watch>), Box<dyn std::error::Error>> {
    let reset_to = heard(handle, App::saved)?;
    let (reload_to, asks) = std::sync::mpsc::channel::<Reload>();
    let first = file.to_string();
    std::thread::Builder::new()
        .name("reloader".into())
        .spawn(move || {
            let mut setter: Option<page::Setter> = None;
            while let Ok(ask) = asks.recv() {
                let mut open = None;
                let mut fit = None;
                for ask in std::iter::once(ask).chain(std::iter::from_fn(|| asks.try_recv().ok())) {
                    match ask {
                        Reload::Open(file) => open = Some(file),
                        Reload::Fit(width) => fit = Some(width),
                        Reload::Saved => (),
                    }
                }
                let setter = setter.get_or_insert_with(|| {
                    let mut setter = page::Setter::new(&first);
                    setter.fit(width);
                    setter
                });
                if let Some(file) = open {
                    setter.open(&file);
                }
                if let Some(width) = fit {
                    setter.fit(width);
                }
                let set = setter.set().map(|(page, ms)| {
                    let lines = text::lines(text::runs(&page.frame));
                    (page, ms, lines)
                });
                if reset_to.send((setter.file().to_string(), set)).is_err() {
                    return;
                }
            }
        })?;
    // Pictures fetched from the web set the page again, as a save does.
    let fetched_to = reload_to.clone();
    web::when_fetched(move || {
        let _ = fetched_to.send(Reload::Saved);
    });
    let save_to = reload_to.clone();
    let watch = watch::saves(std::path::Path::new(file), move || {
        save_to.send(Reload::Saved).is_ok()
    })
    .map_err(|error| eprintln!("reader: can't watch {file} for saves: {error}"))
    .ok();
    Ok((reload_to, watch))
}
