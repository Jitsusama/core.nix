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
//! reader --at LETTER FILE  shows the file at the mark kept under LETTER,
//!                          or where it was left without one
//! reader --measure FILE    runs the frame-time script and prints what
//!                          each phase took, then closes
//! reader --keys            prints every chord it takes, for the launcher
//! reader --places          prints where each file was left, newest
//!                          first, for the launcher's Recent
//!
//! The measure reads when each frame reached the screen from the
//! compositor's presentation feedback, and holds the worst 1% of intervals
//! against one refresh interval.

mod anchor;
mod app;
mod blend;
mod ex;
mod eyebrow;
mod gl;
mod keys;
mod links;
mod menu;
mod motion;
mod moves;
mod outline;
mod page;
mod painter;
mod place;
mod prompt;
mod reading;
mod search;
mod select;
mod showing;
mod start;
mod text;
mod theme;
mod tiles;
mod visual;
mod walk;
mod watch;
mod web;
mod whole;
mod worker;
mod world;

use app::{Drawn, Following, Incoming, Loading, ReadsOn, Reload, Spot};
use painter::{Band, Key, Painter};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::channel::Event;
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle, channel};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::{Window, WindowDecorations};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::io::Write;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure, Help};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_keyboard;
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use wayland_protocols::wp::presentation_time::client::wp_presentation::WpPresentation;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;

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

/// How long a copy flashes what it took, as his Neovim's yank does
/// (`vim.highlight.on_yank({ timeout = 200 })`).
const FLASH: Duration = Duration::from_millis(200);
/// Lines kept between the cursor and the view's edge, as his Neovim's
/// scrolloff.
const SCROLL_OFF_LINES: f64 = 4.0;

/// How long this thread has waited, ready to run, for a processor, in
/// nanoseconds: schedstat's second figure. Zero where it can't be read.
fn run_delay_ns() -> u64 {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0)
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
    /// Whether the page shown is the whole file, rather than the first
    /// screen's page waiting for the rest, so how far down it the view is
    /// can be said.
    whole: bool,
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
    /// Whether Shift is held, which makes Tab Shift+Tab where the keymap
    /// gives Tab with Shift rather than its own back tab.
    shift: bool,
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
    last_jump: Option<Spot>,
    /// The lines of the file the last selection took, from 1, for :'<,'>.
    selected: Option<(usize, usize)>,
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
    lit: Option<[(gl::Texture, blend::Blend); 2]>,
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
    /// The window's width in points the key list was set for, and how far
    /// down it's scrolled, in device pixels, while taller than the window.
    keys_width_pt: f64,
    keys_scroll: f64,
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
    /// The page's words for selecting, with the file they were set from,
    /// read off the page on a thread; none until read.
    layout: Option<std::sync::Arc<select::Layout>>,
    layout_to: channel::Sender<(u32, select::Layout)>,
    /// Visual mode's selection, while there is one.
    selection: Option<select::Selection>,
    /// What the last copy took, flashed until a moment after it.
    flash: Option<(Vec<text::Piece>, Instant)>,
    /// The selection's mark, then the cursor's, which is drawn over it.
    chosen: Option<[gl::Mark; 2]>,
    /// The cursor's line's top, in points, as a help buffer keeps a cursor:
    /// where j and k, H, M and L, a landing or a match left it, for yy,
    /// v, zt, %, * and gf to act on, and marked by a rule in the margin.
    /// Until then, and once it's out of view, it's the view's nearest line
    /// (moves.rs).
    aim: Option<f64>,
    /// The rule's swatch, in the accent.
    rule: Option<gl::Texture>,
    /// The ground's swatch, laid over a line an overlay would cut.
    bare: Option<gl::Texture>,
    /// Which of the page's lines are a heading's, for the page's version
    /// they were read for, so a heading hides with its first line.
    heading_lines: Option<(u32, Vec<bool>)>,
    /// Whether the key being done put where it left from on the jump list,
    /// so a move longer than a screen that didn't is put there after.
    leapt: bool,
    /// A command being typed after :, and what the prompt's row says until
    /// the next key, as a command's error.
    commanding: Option<search::Typing>,
    said: Option<String>,
    /// The command line's menu while it's typed, whether Ctrl+E put it
    /// away until the next key typed, and the page's commands it lists,
    /// read from the language once. It's set and drawn where gO's outline
    /// is, as the two never show at once.
    menu: Option<menu::Menu>,
    menu_away: bool,
    menu_entries: Vec<menu::Entry>,
    /// The lines the command line ran, the newest last, for :his and @:.
    earlier: Vec<String>,
    /// gO's outline while it shows, a search through it being typed, and
    /// it set, waiting to be painted, and painted.
    outline: Option<outline::Outline>,
    outline_typing: Option<search::Typing>,
    outline_set: Option<prompt::Set>,
    outline_canvas: Option<gl::Texture>,
    /// Its chosen row's words reading on, where they're too long for it.
    reads_on: Option<ReadsOn>,
    /// Whether the last search was * or #'s, which ignores case whatever
    /// it holds, as vi's do.
    word_search: bool,
}

/// One line of the page's body text, what Ctrl+E and Ctrl+Y move by.
fn line_pt() -> f64 {
    page::pitch_pt()
}

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

struct Presentation;

/// The viewporter and its viewports send nothing.
struct Viewporting;

/// The compositor's scale for the window, in 120ths.
struct Scaling;

/// Which frame a presentation feedback is for.
struct Feedback(u64);

/// Where on the first page the view opens for a place kept, in points.
/// Kept at the end, it opens at the end, however the window or the page
/// has changed since: the first frame clamps it there.
fn start_at(block: usize, into_pt: f64, at_end: bool, starts: &[(usize, f64)]) -> Option<f64> {
    if at_end {
        Some(f64::INFINITY)
    } else {
        place::pt_of(starts, block, into_pt)
    }
}

/// What a place kept says of its marks, as the page takes them back.
fn kept_marks(place: &place::Place) -> Vec<(char, usize, f64)> {
    place
        .marks
        .iter()
        .map(|mark| (mark.letter, mark.block, mark.into_pt))
        .collect()
}

/// Where the view opens: the mark asked for, or where the file was left.
/// A letter with no mark kept says so, as Vim does, and opens where the
/// file was left, as a file opened plainly does.
fn opening_at(kept: Option<&place::Place>, letter: Option<char>) -> Option<(usize, f64, bool)> {
    let (opens, unmarked) = place::opening(kept, letter);
    if let Some(letter) = unmarked {
        eprintln!("reader: E20: Mark not set: {letter}");
    }
    opens
}

/// What the reader was started with: whether it measures, the mark it
/// opens at and the file; or nothing, once it has printed what was asked.
fn started_with(args: &[String]) -> Result<Option<(bool, Option<char>, String)>, Failure> {
    let usage = || {
        Failure::new(
            Exit::Usage,
            "the reader takes a file, with --measure or --at LETTER",
        )
        .try_this("reader --help")
    };
    Ok(Some(match args {
        [flag] if flag == "--keys" => {
            print_keys();
            return Ok(None);
        }
        [flag] if flag == "--places" => {
            // A reader of the list that stops early, as head does, ends it
            // quietly.
            let mut out = std::io::stdout().lock();
            for line in place::listed() {
                if writeln!(out, "{line}").is_err() {
                    break;
                }
            }
            return Ok(None);
        }
        [flag, file] if flag == "--measure" => (true, None, file.clone()),
        [flag, letter, file] if flag == "--at" => match place::letter(letter) {
            Some(letter) => (false, Some(letter), file.clone()),
            None => return Err(usage().because(format!("{letter} isn't a mark's letter"))),
        },
        [file] => (false, None, file.clone()),
        // Started with nothing to show, as from the launcher's Programs, it
        // shows what there is to read.
        [] => (
            false,
            None,
            start::page()
                .map_err(|error| {
                    Failure::new(Exit::Failed, "there's no page of files to read")
                        .because(error.to_string())
                })?
                .to_string_lossy()
                .into_owned(),
        ),
        _ => return Err(usage()),
    }))
}

/// The width the first screen is set at: the window's when the reader last
/// closed, which niri most likely opens it at again, so the first screen
/// seldom sets twice. Measuring sets the whole page each time.
fn first_width(measuring: bool) -> page::Width {
    if measuring {
        page::Width::whole()
    } else {
        place::window_pt().map_or_else(page::Width::whole, page::Width::for_window)
    }
}

fn help() -> Help {
    Help::new("reader", "shows a markdown file as a page to read")
        .example("reader notes.md", "shows notes.md where it was left")
        .command("[FILE]", "shows the file, or a page of files to read")
        .option("--at LETTER", "opens at the mark kept under LETTER")
        .option(
            "--measure",
            "says how long each frame's phases took, then closes",
        )
        .option("--keys", "lists every chord it takes, as JSON")
        .option("--places", "lists where each file was left, newest first")
        .exit(Exit::Failed, "the page couldn't be set or drawn")
        .exit(Exit::NotFound, "there's no such file")
        .exit(Exit::Unavailable, "there's no Wayland display to draw on")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let Some((measuring, letter, file)) = started_with(&context.args)? else {
            return Ok(());
        };
        if let Err(error) = std::fs::metadata(&file) {
            let exit = if error.kind() == std::io::ErrorKind::NotFound {
                Exit::NotFound
            } else {
                Exit::Failed
            };
            return Err(Failure::new(exit, format!("can't read {file}")).because(error.to_string()));
        }
        show(measuring, letter, &file).map_err(|error| {
            if error.is::<wayland_client::ConnectError>() {
                Failure::new(Exit::Unavailable, "there's no Wayland display to draw on")
                    .because(error.to_string())
            } else {
                Failure::new(Exit::Failed, "the reader stopped").because(error.to_string())
            }
        })
    })
}

/// Shows `file` in a window until it's closed, or measures its frames.
fn show(
    measuring: bool,
    letter: Option<char>,
    file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
        place::read(std::path::Path::new(file))
    };
    let width = first_width(measuring);
    let opens_at = opening_at(kept.as_ref(), letter);
    let first = page::set_first(file, opens_at.map(|(block, _, _)| block), width)?;
    let starts = place::starts(&first.page.frame);
    let start_pt =
        opens_at.and_then(|(block, into_pt, at_end)| start_at(block, into_pt, at_end, &starts));
    let kept_marks = kept.as_ref().map(kept_marks).unwrap_or_default();
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
        layout_to: heard(&handle, |app: &mut App, (version, layout)| {
            app.layout_read(version, layout);
        })?,
        prompt_to: heard(&handle, App::prompt_arrived)?,
    };
    let surface = surface(&globals, &qh)?;
    let opening = Opening {
        file: std::path::PathBuf::from(file),
        page,
        born,
        scale,
        measuring,
        start_pt,
        kept_marks,
        set_ms,
        set_parts,
        width,
        whole: rest.is_none(),
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
    let (reload_to, watch) = reload_on_save(&handle, file, width)?;
    app.reload_to = Some(reload_to);
    app.watch = watch;
    app.take_kept_marks();
    if !measuring && let Err(error) = showing::keep(std::path::Path::new(file)) {
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
        .map(|key| {
            serde_json::json!({
                "group": key.group,
                "chord": key.chord,
                "does": key.does,
                "act": key.act,
            })
        })
        .collect();
    // A reader of the list that stops early, as head does, ends it quietly.
    let _ = writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::Value::Array(keys)
    );
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
    layout_to: channel::Sender<(u32, select::Layout)>,
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
    /// Whether the first set was the whole file.
    whole: bool,
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
