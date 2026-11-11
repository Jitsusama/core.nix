//! What the reader sets apart from the page: the search prompt and the
//! command line at the window's foot, as Neovim's command line, gO's
//! outline above it, as his location list, Space ?'s key list, as his
//! which-key, and g x's labels over links, as his flash. Each is set by Typst at the theme's
//! levels, what's typed at Figure and the rest at Note, on a thread of
//! their own, whose world is made once, so a key never waits on fonts or
//! setting.
use std::sync::mpsc;

use smithay_client_toolkit::reexports::calloop::channel::Sender;
use typst_layout::PagedDocument;

use crate::page;

/// The prompt's row: Space's xxxl, the pitch of a prompt's row, as 36 pt
/// at 0.75 pt a logical pixel.
pub const ROW_PT: f64 = 36.0;

/// What the prompt says: what is typed after its / or ?, and how many it
/// finds.
pub struct Ask {
    pub serial: u32,
    /// The key list rather than the prompt: only `px_per_pt` then counts.
    pub keys: bool,
    /// Labels for links rather than the prompt, each set apart: only
    /// `px_per_pt` then counts besides.
    pub labels: Vec<String>,
    /// gO's outline rather than the prompt: the rows shown, each a
    /// heading's level and words and whether it's chosen. `width_pt`,
    /// `px_per_pt` and `page` count besides.
    pub outline: Vec<(u8, String, bool)>,
    /// The command line's menu rather than the prompt; `width_pt`,
    /// `px_per_pt` and `page` count besides.
    pub menu: Option<Menu>,
    /// What the prompt's row says at its end where it counts no matches:
    /// how to choose from the menu, or what Return runs.
    pub note: String,
    /// What the prompt opens with: / or ? for a search, : for a command,
    /// and nothing for a message.
    pub mark: &'static str,
    pub typed: String,
    /// Whether it's still being typed, so the cursor shows.
    pub cursor: bool,
    /// The match the view is on and how many there are; none while the
    /// page's words are still being read.
    pub count: Option<(usize, usize)>,
    /// The window's width in points of the page at zoom 1, and device
    /// pixels a point.
    pub width_pt: f64,
    pub px_per_pt: f64,
    /// How wide the page is set for the window, so the prompt's words start
    /// where its measure does.
    pub page: crate::page::Width,
}

/// The command line's menu as it shows: each line's command, title and
/// key and whether it's chosen, and every line's command, which sets the
/// columns so they hold still as typing narrows it.
pub struct Menu {
    pub lines: Vec<(String, String, String, bool)>,
    pub all: Vec<String>,
}

/// The prompt set, as pixels.
pub struct Set {
    pub serial: u32,
    pub keys: bool,
    pub outline: bool,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Each label asked for, set on a page of its own, in the order asked.
    pub labels: Vec<Label>,
    /// gO's chosen row's words whole, where they're too long for it, to
    /// read on in place.
    pub reading: Option<Strip>,
}

/// A chosen row's words on one line, as long as they run, on the
/// selection's ground, and the room they read on in, all in device pixels:
/// where it stands in the outline's set, how wide it is, and how wide a
/// figure is, which sets the pace.
pub struct Strip {
    /// The words as asked for, so a set that arrives after another row
    /// was chosen never reads on.
    pub said: String,
    pub words: Label,
    pub left_px: f64,
    pub top_px: f64,
    pub room_px: f64,
    pub figure_px: f64,
}

/// Where a chosen row's words stand in gO's outline, in points: from the
/// set's left and top, and how wide their room is.
#[derive(Debug, PartialEq)]
struct Room {
    said: String,
    left_pt: f64,
    top_pt: f64,
    width_pt: f64,
}

/// One link's label, as pixels.
pub struct Label {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// How far below its top its letters stand, in device pixels.
    pub baseline_px: f64,
}

/// Where a set page's first text stands, in points down it.
fn baseline_pt(frame: &typst::layout::Frame) -> Option<f64> {
    frame.items().find_map(|(place, item)| match item {
        typst::layout::FrameItem::Text(_) => Some(place.y.to_pt()),
        typst::layout::FrameItem::Group(group) => {
            baseline_pt(&group.frame).map(|y| y + place.y.to_pt() + group.transform.ty.to_pt())
        }
        _ => None,
    })
}

/// A set page as pixels, `px_per_pt` device pixels a point.
fn rendered(page: &typst_layout::Page, px_per_pt: f64) -> Label {
    let options = typst_render::RenderOptions {
        pixel_per_pt: px_per_pt.into(),
        ..Default::default()
    };
    let pixmap = typst_render::render(page, &options);
    Label {
        width: pixmap.width(),
        height: pixmap.height(),
        baseline_px: baseline_pt(&page.frame).unwrap_or(0.0) * px_per_pt,
        pixels: pixmap.take(),
    }
}

/// Which of the four an ask is, so only the newest of each is set.
fn slot(ask: &Ask) -> usize {
    if !ask.labels.is_empty() {
        2
    } else if !ask.outline.is_empty() || ask.menu.is_some() {
        3
    } else {
        usize::from(ask.keys)
    }
}

pub struct Prompt {
    asks: mpsc::Sender<Ask>,
}

impl Prompt {
    pub fn new(done: Sender<Set>) -> Self {
        let (asks, waiting) = mpsc::channel::<Ask>();
        std::thread::Builder::new()
            .name("prompt".into())
            .spawn(move || {
                let mut world = page::new_world();
                // Sets an empty prompt once, so the first / finds the
                // fonts read and Typst's work memoised.
                world.set_main(&source(&Ask {
                    serial: 0,
                    keys: false,
                    labels: Vec::new(),
                    outline: Vec::new(),
                    menu: None,
                    note: String::new(),
                    mark: "/",
                    typed: String::new(),
                    cursor: true,
                    count: None,
                    width_pt: 720.0,
                    px_per_pt: 1.0,
                    page: crate::page::Width::WHOLE,
                }));
                let _ = typst::compile::<PagedDocument>(&world);
                while let Ok(first) = waiting.recv() {
                    // Only the newest ask of each is worth setting.
                    let mut newest: [Option<Ask>; 4] = [None, None, None, None];
                    let at = slot(&first);
                    newest[at] = Some(first);
                    while let Ok(newer) = waiting.try_recv() {
                        let at = slot(&newer);
                        newest[at] = Some(newer);
                    }
                    for ask in newest.into_iter().flatten() {
                        let room = outline_room(&ask);
                        let text = if !ask.labels.is_empty() {
                            labels(&ask.labels)
                        } else if !ask.outline.is_empty() {
                            outline(&ask)
                        } else if let Some(menu) = &ask.menu {
                            menu_source(&ask, menu)
                        } else if ask.keys {
                            key_list(ask.width_pt)
                        } else {
                            source(&ask)
                        };
                        world.set_main(&text);
                        let document = match typst::compile::<PagedDocument>(&world).output {
                            Ok(document) => document,
                            Err(errors) => {
                                let said: Vec<String> =
                                    errors.iter().map(|e| e.message.to_string()).collect();
                                eprintln!(
                                    "can't set the {}: {}",
                                    if ask.keys { "key list" } else { "prompt" },
                                    said.join("; ")
                                );
                                continue;
                            }
                        };
                        let mut pages = document
                            .pages()
                            .iter()
                            .map(|page| rendered(page, ask.px_per_pt));
                        let set = if ask.labels.is_empty() {
                            let Some(first) = pages.next() else {
                                continue;
                            };
                            let reading =
                                room.and_then(|room| strip(room, &mut pages, ask.px_per_pt));
                            Set {
                                serial: ask.serial,
                                keys: ask.keys,
                                outline: !ask.outline.is_empty() || ask.menu.is_some(),
                                width: first.width,
                                height: first.height,
                                pixels: first.pixels,
                                labels: Vec::new(),
                                reading,
                            }
                        } else {
                            Set {
                                serial: ask.serial,
                                keys: false,
                                outline: false,
                                width: 0,
                                height: 0,
                                pixels: Vec::new(),
                                labels: pages.collect(),
                                reading: None,
                            }
                        };
                        if done.send(set).is_err() {
                            return;
                        }
                    }
                }
            })
            .expect("the prompt's thread");
        Prompt { asks }
    }

    pub fn ask(&self, ask: Ask) {
        let _ = self.asks.send(ask);
    }
}

/// The prompt's Typst: its row sunken below the page, the typed words where
/// the page's text column starts, a cursor after them, and the count where
/// the column ends.
fn source(ask: &Ask) -> String {
    let mark = ask.mark;
    let count = match ask.count {
        Some((_, 0)) if !ask.typed.is_empty() => "Nothing matches.".into(),
        None => ask.note.clone(),
        Some((_, 0)) => String::new(),
        Some((at, all)) => format!("{at} of {all}"),
    };
    // The cursor stands a fifth of an em under the baseline, as his
    // terminal's block does, drawn in a box of no height, so it moves
    // neither the line nor its baseline as it comes and goes.
    let cursor = if ask.cursor {
        r"#box(width: 0.6em, place(left + bottom, dy: 0.2em, rect(width: 0.6em, height: 1.1em, fill: ink.strong, stroke: none)))"
    } else {
        ""
    };
    let width = ask.width_pt.max(290.0 + 72.0);
    // What's typed is his letters, at Figure, and raw, which Typst sets a
    // fifth smaller, so it's set back to its level's size; the count is a
    // note. Both are one line, so they stand on one baseline, and the line
    // is centred in the row by its largest capitals, the Figure's, as
    // every line is (docs/design.md, The Ladder).
    format!(
        r#"{ink}#set page(width: {width}pt, height: {row}pt, margin: (x: {side}pt, y: 0pt), fill: ink.sunken)
#let typing = (..level.figure.text, features: level.figure.text.features + ("calt",))
#set text(..typing)
#show raw: set text(..typing)
#align(horizon)[{lead}#raw({typed}){cursor}#h(1fr)#at(level.note, {count})]
"#,
        ink = crate::theme::get().typst(),
        // Its words start where the page's measure does, as the window
        // centres the page, and end where it ends.
        side = ((width - ask.page.page_pt()) / 2.0).max(0.0) + ask.page.spread_pt,
        row = ROW_PT,
        // The command line's : is in the accent, where his keys go, as
        // the spec has it; a search's / and ? are typed as they read.
        lead = if mark == ":" {
            // Its level's fill is the raw show rule's, so the accent is
            // set over it rather than around it.
            "#text(..(typing + (fill: ink.accent)), \":\")"
        } else {
            ""
        },
        typed = quoted(&if mark == ":" {
            ask.typed.clone()
        } else {
            format!("{mark}{}", ask.typed)
        }),
        count = quoted(&count),
    )
}

/// The command line's menu's Typst, as his completion menu rises over his
/// command line and the launcher's over its rows: in a list's frame, all
/// at Note in muted. Its commands stand under what's typed after the :,
/// so a command chosen reads straight down from the line; its titles on a
/// column the groups rung past the widest command it could hold; its keys
/// flush with the end of the page's measure, where the prompt's count
/// ends. The chosen line's ground hugs its ink from its command to its
/// key, as gO's chosen row's does, its words in the text, since muted
/// isn't drawn there.
fn menu_source(ask: &Ask, menu: &Menu) -> String {
    let rows: Vec<String> = menu
        .lines
        .iter()
        .map(|(command, title, key, chosen)| {
            let (ground, ink) = if *chosen {
                (
                    "ground(commands, 100% - commands); ",
                    "ink.text",
                )
            } else {
                ("", "ink.muted")
            };
            format!(
                "block(width: 100%, height: row, {{ set text(fill: {ink}); {ground}place(left + horizon, dx: commands, {command}); place(left + horizon, dx: titles, {title}); place(right + horizon, {key}) }})",
                command = quoted(command),
                title = quoted(title),
                key = quoted(key),
            )
        })
        .collect();
    // A Typst array, a trailing comma making one of a single command.
    let all = menu.all.iter().fold(String::new(), |mut all, command| {
        all.push_str(&quoted(command));
        all.push_str(", ");
        all
    });
    let width = ask.width_pt.max(290.0 + 72.0);
    format!(
        r#"{ink}#set page(width: {width}pt, height: auto, margin: 0pt, fill: ink.sunken)
#set text(..level.note.text, features: level.note.text.features + ("calt",))
{frame}#context {{
  let commands = measure(text(..level.figure.text, ":")).width
  let widest = calc.max(0pt, ..({all}).map(it => measure(text(it)).width))
  let titles = commands + widest + rung.groups
  framed({side}pt, stack({rows}))
}}
"#,
        ink = crate::theme::get().typst(),
        frame = list_frame(ask.px_per_pt),
        side = measure_left_pt(ask),
        rows = rows.join(", "),
    )
}

/// Where the page's measure starts across a window, in points, as the
/// window centres the page: where the prompt's words and every list's
/// start.
fn measure_left_pt(ask: &Ask) -> f64 {
    let width = ask.width_pt.max(290.0 + 72.0);
    ((width - ask.page.page_pt()) / 2.0).max(0.0) + ask.page.spread_pt
}

/// The Typst every list over the page is framed by, the menu and gO's
/// outline alike: a hairline in the rule the window across, a logical
/// pixel snapped to the device's, then a beat of the page's room above
/// the first row and below the last, and rows two beats apart, so every
/// line of a list lands on a line of the page it covers. `framed(side, body)` sets a list's rows
/// in it from `side` across, and `ground(from, wide)` lays the chosen
/// row's ground behind words `wide` across from `from`: a mark round
/// them, above their capitals and below their baseline, as the launcher's
/// chosen row is.
fn list_frame(px_per_pt: f64) -> String {
    format!(
        r"#let row = 2 * beat
#let framed(side, body) = stack(
  rect(width: 100%, height: {hair}pt, fill: ink.rule, stroke: none),
  block(width: 100%, inset: (x: side, y: beat), body))
#let ground(from, wide) = context place(left + horizon, dx: from - rung.mark,
  rect(width: wide + 2 * rung.mark, height: measure([H]).height + 2 * rung.mark,
    fill: ink.selection, stroke: none))
",
        hair = hair_pt(px_per_pt),
    )
}

/// A list's hairline, in points: a logical pixel, snapped to whole device
/// pixels so it's drawn whole at every scale.
fn hair_pt(px_per_pt: f64) -> f64 {
    (crate::theme::PT_A_PX * px_per_pt).round().max(1.0) / px_per_pt
}

/// The page's beat, in points: what a list's room and rows count in.
fn beat_pt() -> f64 {
    crate::theme::get().space.beat * crate::theme::PT_A_PX
}

/// A list's row, in points: two of the page's beats, its line.
fn row_pt() -> f64 {
    2.0 * beat_pt()
}

/// gO's outline's Typst, as his location list stands below a window: in
/// a list's frame, the full width of the window, each heading's words
/// where the page's measure starts, set in from it by two characters for
/// each level below the outline's highest, as Neovim's gO sets them in,
/// each a note in the text; the row chosen on the selection's ground in
/// the strong text, close round its words, as the launcher's chosen row
/// and his `CursorLine` in a list.
fn outline(ask: &Ask) -> String {
    let (width, side, highest) = outline_measure(ask);
    let rows: Vec<String> = ask
        .outline
        .iter()
        .map(|(level, words, chosen)| {
            let (ground, ink) = if *chosen {
                ("ground(0pt, measure(shown).width); ", "ink.strong")
            } else {
                ("", "ink.text")
            };
            // What `layout` sets shrinks to what it holds, and placed words
            // hold no room, so they're set in a block the row's whole size,
            // to be centred in it.
            format!(
                "block(width: 100%, height: row, inset: (left: {indent}pt), layout(size => block(width: size.width, height: size.height, {{ let shown = fit({words}, size.width); {ground}place(left + horizon, text(fill: {ink}, shown)) }})))",
                indent = indent_pt(*level, highest),
                words = quoted(words),
            )
        })
        .collect();
    // Past the list, the chosen row's words whole on a page as wide as
    // they run, a row tall, on the chosen ground as it stands round them,
    // and a figure, whose width sets the pace they read on at.
    let whole = ask
        .outline
        .iter()
        .find(|(_, _, chosen)| *chosen)
        .map(|(_, words, _)| {
            format!(
                "#page(width: auto, height: row, margin: 0pt, fill: ink.sunken, align(horizon, box(fill: ink.selection, outset: (y: rung.mark), text(fill: ink.strong, {words}))))\n#page(width: auto, height: auto, margin: 0pt, fill: none, [0])",
                words = quoted(words),
            )
        })
        .unwrap_or_default();
    format!(
        r#"{ink}#set page(width: {width}pt, height: auto, margin: 0pt, fill: ink.sunken)
#set text(..level.note.text, features: level.note.text.features + ("calt",))
{frame}// A row is one line: words too long for its room end in an ellipsis, at
// the last word that fits, as a long name does on the launcher's rows.
#let fit(words, room) = {{
  if measure(words).width <= room {{ return words }}
  let kept = words.split(" ")
  while kept.len() > 1 and measure(kept.join(" ") + "\u{{2026}}").width > room {{
    kept = kept.slice(0, -1)
  }}
  kept.join(" ") + "\u{{2026}}"
}}
#framed({side}pt, stack({rows}))
{whole}
"#,
        ink = crate::theme::get().typst(),
        frame = list_frame(ask.px_per_pt),
        rows = rows.join(", "),
    )
}

/// gO's outline's measure, in points: its set's width, where its words
/// start, where the page's measure does, and the highest level among its
/// rows.
fn outline_measure(ask: &Ask) -> (f64, f64, u8) {
    let highest = ask
        .outline
        .iter()
        .map(|(level, _, _)| *level)
        .min()
        .unwrap_or(1);
    (
        ask.width_pt.max(290.0 + 72.0),
        measure_left_pt(ask),
        highest,
    )
}

/// How far a row is set in: two characters, as 1.2 of the note's ems, for
/// each level below the outline's highest, as Neovim's gO sets them in.
fn indent_pt(level: u8, highest: u8) -> f64 {
    f64::from(level.saturating_sub(highest)) * 1.2 * crate::theme::get().levels.note.size
}

/// The room the chosen row's words stand in, when an outline is asked for
/// and a row of it is chosen: past the measure's start and the row's
/// indent to the measure's end, and down the list's hairline, its room and
/// the rows above it.
fn outline_room(ask: &Ask) -> Option<Room> {
    let at = ask.outline.iter().position(|(_, _, chosen)| *chosen)?;
    let (width, side, highest) = outline_measure(ask);
    let indent = indent_pt(ask.outline[at].0, highest);
    #[expect(
        clippy::cast_precision_loss,
        reason = "the outline shows ten rows at most"
    )]
    let above = at as f64;
    Some(Room {
        said: ask.outline[at].1.clone(),
        left_pt: side + indent,
        top_pt: hair_pt(ask.px_per_pt) + beat_pt() + above * row_pt(),
        width_pt: width - 2.0 * side - indent,
    })
}

/// The chosen row's words whole and a figure, the outline's next two
/// pages, as the strip that reads on in its room, when they run past it.
fn strip(room: Room, pages: &mut impl Iterator<Item = Label>, px_per_pt: f64) -> Option<Strip> {
    let words = pages.next()?;
    let figure = pages.next()?;
    let room_px = room.width_pt * px_per_pt;
    (f64::from(words.width) > room_px + 0.5).then(|| Strip {
        said: room.said,
        words,
        left_px: room.left_pt * px_per_pt,
        top_px: room.top_pt * px_per_pt,
        room_px,
        figure_px: f64::from(figure.width),
    })
}

/// The key list's Typst, as the launcher's drawer lists keys: on the
/// sunken ground, as notes, the keys in the text and what they do muted,
/// each group under its name as a label, in two columns. Every line is
/// the page's pitch, its baseline where a row's centres its capitals, so
/// a group's name stands a pitch over its first key and each key a pitch
/// under the last, a beat more between groups. Its spaces are rungs: the
/// inset round it, groups between a key column and its words and between
/// the columns, apart between a key's chords and tie between a chord's
/// keys. The groups in a column share one edge for their words, measured
/// from the widest keys in it. Each column is a stack of its own, the
/// groups shared out so the two stand as near one height as they can.
///
/// It is set for a window `width_pt` wide, Space's xl clear of each side:
/// two columns where they fit, else one, every group in order down it,
/// whose words wrap only where even one column is wider than the room.
/// Taller than the window, it scrolls there rather than here.
fn key_list(width_pt: f64) -> String {
    let listing = crate::keys::listing();
    let heights: Vec<usize> = listing.iter().map(|(_, rows)| rows.len()).collect();
    let columns = columns_of(&heights);
    let mut groups: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    let mut alone: Vec<String> = Vec::new();
    let mut widest: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for ((group, rows), &column) in listing.into_iter().zip(&columns) {
        let mut cells = Vec::new();
        for (ways, does) in rows {
            let chords: Vec<String> = ways.iter().map(|way| chord(way)).collect();
            let name = format!("({},)", chords.join(", h(rung.apart), "));
            widest[column].push(name.clone());
            cells.push(format!("[#{name}.join()]"));
            cells.push(format!("[#{}]", quoted(does)));
        }
        // A grid's rows take their lines' height, and every line is the
        // pitch, so the rows need no height of their own; alone in a
        // column that may be narrower than its words, a row's words wrap
        // onto lines a pitch each.
        let listed = |keys: &str| {
            format!(
                "stack(at(level.label, {}), grid(columns: ({keys}, auto), column-gutter: rung.groups, {}))",
                quoted(group),
                cells.join(", ")
            )
        };
        groups[column].push(listed(&format!("keys.at({column})")));
        alone.push(listed("calc.max(..keys)"));
    }
    let stacks: Vec<String> = groups
        .iter()
        .map(|column| format!("stack(spacing: beat, {})", column.join(", ")))
        .collect();
    format!(
        r"{}#set page(width: auto, height: auto, margin: rung.inset, fill: ink.sunken)
#set text(..level.note.text)
#set par(leading: 0pt)
#context {{
  let cap = measure([H]).height
  // Each line a pitch, its baseline where a row's of the page centres
  // its capitals, whatever its level, so names and keys share the lines.
  set text(top-edge: (pitch + cap) / 2, bottom-edge: -(pitch - cap) / 2)
  let keys = (({}), ({})).map(column => calc.max(0pt, ..column.map(it => measure(it.join()).width)))
  let note = [A number first does it that many times: 5j goes five lines down.]
  let two = stack(spacing: beat, grid(columns: 2, column-gutter: rung.groups, {}), note)
  let one = stack(spacing: beat, {}, note)
  let room = {room}pt
  if measure(two).width <= room {{ two }} else {{ block(width: calc.min(measure(one).width, room), one) }}
}}
",
        crate::theme::get().typst(),
        widest[0].join(", ") + ",",
        widest[1].join(", ") + ",",
        stacks.join(", "),
        alone.join(", "),
        room = (width_pt - 4.0 * inset_pt()).max(KEYS_LEAST_PT),
    )
}

/// One way to a key's action as the key list sets it, in Typst: each of
/// its keys a run in the text, a chord's keys the tie rung apart, as the
/// drawer sets a chord.
fn chord(way: &str) -> String {
    let (keys, words) = keys_of(way);
    let keys: Vec<String> = keys
        .iter()
        .map(|key| format!("text(fill: ink.text, {})", quoted(key)))
        .collect();
    let words = words.map_or(String::new(), |words| {
        format!(" + text(fill: ink.text, {})", quoted(&format!(" {words}")))
    });
    format!("(({},).join(h(rung.tie)){words})", keys.join(", "))
}

/// A way to a key's action as its keys, in the order they're pressed,
/// and the words after them, if any: the keys a space apart as
/// `keys::listing` spells them, Page Up and Page Down one key each, and
/// "and a letter" after a key that takes one words rather than keys.
fn keys_of(way: &str) -> (Vec<&str>, Option<&str>) {
    const WORDS: &str = "and a letter";
    let (pressed, words) = match way.strip_suffix(WORDS) {
        Some(pressed) => (pressed.trim_end(), Some(WORDS)),
        None => (way, None),
    };
    let mut keys = Vec::new();
    let mut rest = pressed;
    while !rest.is_empty() {
        let whole = ["Page Up", "Page Down"]
            .into_iter()
            .find(|name| rest.starts_with(name));
        let end = whole.map_or_else(|| rest.find(' ').unwrap_or(rest.len()), str::len);
        keys.push(&rest[..end]);
        rest = rest[end..].trim_start();
    }
    (keys, words)
}

/// The inset rung, in points: round the key list's words and between it
/// and the window's sides.
fn inset_pt() -> f64 {
    crate::theme::get().space.rung.inset * crate::theme::PT_A_PX
}

/// The narrowest the key list's words are set, however narrow the window:
/// its keys and a few words beside them.
const KEYS_LEAST_PT: f64 = 240.0;

/// Which column each group of the key list goes in, the first always the
/// left: the share whose taller column is shortest, counting a group's
/// name as one and a half rows, as it stands with its gap; groups keep
/// their order down each column.
fn columns_of(rows: &[usize]) -> Vec<usize> {
    let height = |n: usize| n * 4 + 6;
    let n = rows.len().min(16);
    let tallest = |share: u32| {
        let mut sides = [0, 0];
        for (i, &count) in rows.iter().take(n).enumerate() {
            sides[(share >> i & 1) as usize] += height(count);
        }
        sides[0].max(sides[1])
    };
    let best = (0..1u32 << n)
        .filter(|share| share & 1 == 0)
        .min_by_key(|&share| tallest(share))
        .unwrap_or(0);
    (0..rows.len())
        .map(|i| if i < n { (best >> i & 1) as usize } else { 1 })
        .collect()
}

/// The labels' Typst, as his flash draws a label: the letters, chords to
/// type, as notes in the colour on the accent, on the accent, each on a page of its
/// own just round its letters, with Space's xs at its sides.
fn labels(each: &[String]) -> String {
    let pages: Vec<String> = each
        .iter()
        .map(|label| format!("#text({})", quoted(label)))
        .collect();
    format!(
        r#"{}#set page(width: auto, height: auto, margin: (x: 3pt, y: 0pt), fill: ink.accent)
#set text(..level.note.text, fill: ink.on_accent, top-edge: "ascender", bottom-edge: "descender")
{}
"#,
        crate::theme::get().typst(),
        pages.join("\n#pagebreak()\n")
    )
}

/// A Typst string holding `text` as it is.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
