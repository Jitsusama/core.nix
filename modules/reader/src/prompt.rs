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
/// outline alike, as the launcher's menu is (`CommandMenu.qml`): a
/// hairline in the rule the window across, a logical pixel snapped to the
/// device's, then a beat of the page's room above the first row and below
/// the last, and rows two beats apart, so every line of a list lands on
/// a line of the page it covers. `framed(side, body)` sets a list's rows
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
mod tests {
    use super::*;

    #[test]
    fn the_key_lists_groups_share_two_columns_as_evenly_as_they_can() {
        assert_eq!(columns_of(&[3, 3]), [0, 1]);
        // A tall group stands alone beside the rest.
        assert_eq!(columns_of(&[2, 10, 2, 2, 2]), [0, 1, 0, 0, 0]);
        let heights: Vec<usize> = crate::keys::listing()
            .iter()
            .map(|(_, rows)| rows.len())
            .collect();
        let columns = columns_of(&heights);
        let side = |column: usize| -> usize {
            heights
                .iter()
                .zip(&columns)
                .filter(|(_, c)| **c == column)
                .map(|(rows, _)| rows + 2)
                .sum()
        };
        assert!(
            side(0).abs_diff(side(1)) <= 4,
            "the reader's own list is {} rows beside {}",
            side(0),
            side(1)
        );
    }

    #[test]
    fn quoting_keeps_what_is_typed() {
        assert_eq!(quoted(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    /// The key list set for a window this many points wide: its width and
    /// height in points.
    fn key_list_set(width_pt: f64) -> (f64, f64) {
        let mut world = page::new_world();
        world.set_main(&key_list(width_pt));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        let size = document.pages()[0].frame.size();
        (size.x.to_pt(), size.y.to_pt())
    }

    #[test]
    fn the_key_list_fits_inside_its_window_at_any_width() {
        // A whole screen at 1.5 is 1920 points; half and a third are what
        // a window beside another takes, and 400 is narrower than either.
        for window in [400.0, 640.0, 960.0, 1920.0] {
            let (width, _) = key_list_set(window);
            assert!(
                width <= window - 2.0 * inset_pt(),
                "the key list is {width:.0} pt wide in a window {window} pt wide"
            );
        }
    }

    #[test]
    fn a_way_to_a_key_splits_into_its_keys_and_keeps_names_and_words_whole() {
        assert_eq!(keys_of("Space ?"), (vec!["Space", "?"], None));
        assert_eq!(keys_of("gg"), (vec!["gg"], None));
        assert_eq!(keys_of("Page Down"), (vec!["Page Down"], None));
        assert_eq!(keys_of("Space Page Up"), (vec!["Space", "Page Up"], None));
        assert_eq!(keys_of("m and a letter"), (vec!["m"], Some("and a letter")));
        assert_eq!(
            keys_of("Space m and a letter"),
            (vec!["Space", "m"], Some("and a letter"))
        );
    }

    /// The baselines runs stand on, each once, from the top down.
    fn lines_of<'a>(runs: impl Iterator<Item = &'a [f64; 4]>) -> Vec<f64> {
        let mut lines: Vec<f64> = runs.map(|t| t[1]).collect();
        lines.sort_by(f64::total_cmp);
        lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        lines
    }

    #[test]
    fn the_key_list_sets_its_lines_on_the_pitch_and_its_spaces_by_rung() {
        let space = &crate::theme::get().space;
        let pt = |px: f64| px * crate::theme::PT_A_PX;
        let (pitch, beat) = (pt(space.pitch), pt(space.beat));
        let rung = &space.rung;
        let mut world = page::new_world();
        world.set_main(&key_list(1920.0));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        let page = &document.pages()[0].frame;
        let (mut texts, mut rects) = (Vec::new(), Vec::new());
        drawn(page, (0.0, 0.0), &mut texts, &mut rects);
        let near = |a: f64, b: f64, by: f64| (a - b).abs() < by;
        assert!(
            !texts.iter().any(|t| t[2] < 0.1),
            "every run has ink: {texts:?}"
        );
        let left = texts.iter().map(|t| t[0]).fold(f64::INFINITY, f64::min);
        assert!(
            near(left, pt(rung.inset), 1.0),
            "its words start {left} pt in, not the inset rung's {}",
            pt(rung.inset)
        );
        // Lines by baseline, every one on the beat from the first, since
        // each column is a stack of its own whose groups stand a beat
        // apart, so one column's lines may fall a beat off the other's.
        let baselines = lines_of(texts.iter());
        for line in &baselines {
            let beats = (line - baselines[0]) / beat;
            assert!(
                near(beats, beats.round(), 0.001),
                "a line at {line} pt is off the beat from the first at {} pt",
                baselines[0]
            );
        }
        // Down the first column, its names and keys flush with the inset,
        // each line a pitch from the one before, or a pitch and a beat
        // where a group ends; the note under both columns may stand lower.
        let first = lines_of(texts.iter().filter(|t| near(t[0], left, 0.01)));
        for pair in first[..first.len() - 1].windows(2) {
            let step = pair[1] - pair[0];
            assert!(
                near(step, pitch, 0.01) || near(step, pitch + beat, 0.01),
                "lines at {} and {} pt stand {step} pt apart, not a pitch of {pitch} or a pitch and a beat",
                pair[0],
                pair[1]
            );
        }
        // The first line's baseline stands where a row of the page a pitch
        // tall centres a note's capitals, the last line's, from the inset.
        let last = baselines[baselines.len() - 1];
        let cap = texts
            .iter()
            .find(|t| near(t[1], last, 0.01))
            .expect("a last line")[3];
        assert!(
            near(
                baselines[0],
                pt(rung.inset) + f64::midpoint(pitch, cap),
                0.01
            ),
            "its first baseline is {} pt down",
            baselines[0]
        );
        // Along a line, one run from the next stands a chord's tie, a
        // key's apart, or the groups rung from a key to its words and from
        // one column to the next; no dot sets them apart.
        assert!(
            !texts.is_empty() && !key_list(1920.0).contains('\u{b7}'),
            "chords stand apart by space, not a dot"
        );
        let mut gaps = Vec::new();
        for &line in &baselines {
            let mut along: Vec<&[f64; 4]> =
                texts.iter().filter(|t| near(t[1], line, 0.01)).collect();
            along.sort_by(|a, b| a[0].total_cmp(&b[0]));
            // Runs that touch are one word's, set in two faces.
            for pair in along.windows(2) {
                let gap = pair[1][0] - (pair[0][0] + pair[0][2]);
                if gap > 0.5 {
                    gaps.push(gap);
                }
            }
        }
        let groups = pt(rung.groups);
        for gap in &gaps {
            assert!(
                [pt(rung.tie), pt(rung.apart)]
                    .iter()
                    .any(|r| near(*gap, *r, 0.05))
                    || *gap >= groups - 0.05,
                "two runs stand {gap} pt apart, no rung: {gaps:?}"
            );
        }
        for rung_pt in [pt(rung.tie), pt(rung.apart), groups] {
            assert!(
                gaps.iter().any(|gap| near(*gap, rung_pt, 0.05)),
                "no two runs stand {rung_pt} pt apart: {gaps:?}"
            );
        }
    }

    #[test]
    fn the_key_list_takes_two_columns_where_they_fit_and_one_where_not() {
        let (wide, wide_tall) = key_list_set(1920.0);
        let (narrow, narrow_tall) = key_list_set(640.0);
        assert!(
            wide > narrow,
            "{wide:.0} pt wide on a screen, {narrow:.0} pt in a third"
        );
        assert!(
            narrow_tall > wide_tall * 1.5,
            "one column stands taller than two"
        );
    }

    #[test]
    fn labels_set_one_page_each_in_the_themes_ink() {
        let mut world = page::new_world();
        world.set_main(&labels(&["a".into(), "sd".into()]));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        assert_eq!(document.pages().len(), 2);
    }

    #[test]
    fn a_labels_letters_stand_on_its_links_baseline_within_a_pixel() {
        let dir = std::env::temp_dir().join(format!("reader-label-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(
            &file,
            "# A Page\n\nWords before [a link](other.md) and after it.\n\n`code` then [another](#a-page).\n",
        )
        .expect("the file");
        let (page, _) = page::Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let found = crate::links::links(&page.frame);
        assert_eq!(found.len(), 2);
        // Each link's baseline is its words' own, as search reads them.
        let lines = crate::text::lines(crate::text::runs(&page.frame));
        for link in &found {
            assert!(
                lines
                    .iter()
                    .any(|line| (line.baseline_pt - link.baseline_pt).abs() < 0.01),
                "{link:?}"
            );
        }
        let mut world = page::new_world();
        world.set_main(&labels(&["x".into(), "sd".into()]));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .expect("the labels set");
        // Scales 1.5 and 2, at their own size and zoomed, and the view's
        // top anywhere between two pixels.
        for scale in [1.5, 2.0] {
            for zoom in [1.0, 1.1, 1.0 / 1.1, 1.21] {
                let px = scale / 0.75 * zoom;
                for offset in [0.0, 0.25, 0.5, 0.75, 13.37] {
                    for (link, label) in found.iter().zip(document.pages()) {
                        let label = rendered(label, px);
                        assert!(
                            label.baseline_px > 0.0 && label.baseline_px < f64::from(label.height)
                        );
                        let top =
                            f64::from(crate::links::label_top(link, label.baseline_px, px, offset));
                        let words = link.baseline_pt * px - offset;
                        assert!(
                            (top + label.baseline_px - words).abs() <= 1.0,
                            "at {px} px a point, {offset} down: the label's letters at {}, the link's at {words}",
                            top + label.baseline_px
                        );
                    }
                }
            }
        }
    }

    /// What a set page draws, in points from its top left: each run of
    /// text as its left, baseline, advance and capitals' height, and each
    /// rectangle as its left, top, width and height.
    fn drawn(
        frame: &typst::layout::Frame,
        (dx, dy): (f64, f64),
        texts: &mut Vec<[f64; 4]>,
        rects: &mut Vec<[f64; 4]>,
    ) {
        for (place, item) in frame.items() {
            let (x, y) = (dx + place.x.to_pt(), dy + place.y.to_pt());
            match item {
                typst::layout::FrameItem::Text(text) => texts.push([
                    x,
                    y,
                    text.width().to_pt(),
                    text.font.metrics().cap_height.at(text.size).to_pt(),
                ]),
                typst::layout::FrameItem::Shape(shape, _) => {
                    if let typst::visualize::Geometry::Rect(size) = shape.geometry {
                        rects.push([x, y, size.x.to_pt(), size.y.to_pt()]);
                    }
                }
                typst::layout::FrameItem::Group(group) => drawn(
                    &group.frame,
                    (
                        x + group.transform.tx.to_pt(),
                        y + group.transform.ty.to_pt(),
                    ),
                    texts,
                    rects,
                ),
                _ => {}
            }
        }
    }

    #[test]
    fn the_prompts_row_sets_its_words_and_its_count_on_one_baseline() {
        let ask = |mark: &'static str, typed: &str, cursor, count, note: &str| Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline: Vec::new(),
            menu: None,
            note: note.into(),
            mark,
            typed: typed.into(),
            cursor,
            count,
            width_pt: 1080.0,
            px_per_pt: 2.0,
            page: crate::page::Width::WHOLE,
        };
        for (what, asked) in [
            ("a search typed", ask("/", "stands", true, Some((1, 1)), "")),
            (
                "a search found",
                ask("/", "stands", false, Some((1, 1)), ""),
            ),
            ("the command line", ask(":", "b", true, None, "Tab choose")),
        ] {
            let mut world = page::new_world();
            world.set_main(&source(&asked));
            let document = typst::compile::<PagedDocument>(&world)
                .output
                .unwrap_or_else(|errors| {
                    panic!(
                        "{:?}",
                        errors
                            .iter()
                            .map(|e| e.message.to_string())
                            .collect::<Vec<_>>()
                    )
                });
            let (mut texts, mut rects) = (Vec::new(), Vec::new());
            drawn(
                &document.pages()[0].frame,
                (0.0, 0.0),
                &mut texts,
                &mut rects,
            );
            assert!(texts.len() >= 2, "{what}: {texts:?}");
            let baseline = texts[0][1];
            assert!(
                texts.iter().all(|[_, y, ..]| (y - baseline).abs() < 0.01),
                "{what}: its words and count stand on {:?} pt, not one baseline",
                texts.iter().map(|[_, y, ..]| *y).collect::<Vec<_>>()
            );
            // The largest level's capitals, the typed Figure's, are centred
            // in the row.
            let cap = texts.iter().map(|t| t[3]).fold(0.0, f64::max);
            assert!(
                (baseline - cap / 2.0 - ROW_PT / 2.0).abs() < 0.05,
                "{what}: its capitals' middle is {} pt down a row of {ROW_PT}",
                baseline - cap / 2.0
            );
        }
    }

    #[test]
    fn the_prompt_says_when_nothing_matches_only_once_something_is_typed() {
        let ask = |typed: &str, count| Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline: Vec::new(),
            menu: None,
            note: String::new(),
            mark: "/",
            typed: typed.into(),
            cursor: true,
            count,
            width_pt: 720.0,
            px_per_pt: 2.0,
            page: crate::page::Width::WHOLE,
        };
        assert!(source(&ask("cat", Some((0, 0)))).contains("Nothing matches."));
        assert!(!source(&ask("", Some((0, 0)))).contains("Nothing matches."));
        assert!(source(&ask("cat", Some((2, 5)))).contains("2 of 5"));
    }

    /// Where each run of text stands on a set page, in points from its
    /// top left.
    fn texts_at(frame: &typst::layout::Frame, dx: f64, dy: f64, found: &mut Vec<(f64, f64)>) {
        for (place, item) in frame.items() {
            let (x, y) = (dx + place.x.to_pt(), dy + place.y.to_pt());
            match item {
                typst::layout::FrameItem::Text(_) => found.push((x, y)),
                typst::layout::FrameItem::Group(group) => texts_at(
                    &group.frame,
                    x + group.transform.tx.to_pt(),
                    y + group.transform.ty.to_pt(),
                    found,
                ),
                _ => {}
            }
        }
    }

    /// A list over the page, set at each of Joel's scales, against the
    /// ladder: a hairline of whole device pixels the window across at its
    /// top, a beat of room inside it and at its foot, its rows two beats
    /// apart with their capitals centred, and the chosen row's ground a mark
    /// round its first and last ink, above its capitals and below its
    /// baseline.
    fn keeps_the_ladder(what: &str, ask: &dyn Fn(f64) -> String, rows: usize, chosen: usize) {
        let space = crate::theme::get().space;
        let beat = space.beat * crate::theme::PT_A_PX;
        let row = 2.0 * beat;
        let mark = space.rung.mark * crate::theme::PT_A_PX;
        for scale in [2.0_f64, 1.5] {
            let px = scale / crate::theme::PT_A_PX;
            let mut world = page::new_world();
            world.set_main(&ask(px));
            let document = typst::compile::<PagedDocument>(&world)
                .output
                .unwrap_or_else(|errors| {
                    panic!(
                        "{:?}",
                        errors
                            .iter()
                            .map(|e| e.message.to_string())
                            .collect::<Vec<_>>()
                    )
                });
            let frame = &document.pages()[0].frame;
            let (mut texts, mut rects) = (Vec::new(), Vec::new());
            drawn(frame, (0.0, 0.0), &mut texts, &mut rects);
            let near = |a: f64, b: f64| (a - b).abs() < 0.05;
            let at = format!("{what} at {scale}");
            let hair = rects
                .iter()
                .find(|[_, top, ..]| near(*top, 0.0))
                .unwrap_or_else(|| panic!("{at}: no hairline at its top in {rects:?}"));
            assert!(
                near(hair[2], frame.size().x.to_pt()),
                "{at}: its hairline is {} pt of {}",
                hair[2],
                frame.size().x.to_pt()
            );
            assert!(
                near(hair[3] * px, scale.round()),
                "{at}: its hairline is {} device pixels, not a logical pixel snapped",
                hair[3] * px
            );
            let mut baselines: Vec<f64> = texts.iter().map(|[_, y, ..]| *y).collect();
            baselines.sort_by(f64::total_cmp);
            baselines.dedup_by(|a, b| near(*a, *b));
            assert_eq!(baselines.len(), rows, "{at}: {baselines:?}");
            for pair in baselines.windows(2) {
                assert!(
                    near(pair[1] - pair[0], row),
                    "{at}: rows {} pt apart, not two beats ({row} pt)",
                    pair[1] - pair[0]
                );
            }
            let cap = texts[0][3];
            assert!(
                near(baselines[0], hair[3] + beat + row / 2.0 + cap / 2.0),
                "{at}: its first baseline is {} pt down, under {} of hairline and a beat of {beat}",
                baselines[0],
                hair[3]
            );
            #[expect(clippy::cast_precision_loss, reason = "a few rows")]
            let tall = hair[3] + 2.0 * beat + rows as f64 * row;
            assert!(
                near(frame.size().y.to_pt(), tall),
                "{at}: it stands {} pt, not {tall}",
                frame.size().y.to_pt()
            );
            let line: Vec<&[f64; 4]> = texts
                .iter()
                .filter(|[_, y, ..]| near(*y, baselines[chosen]))
                .collect();
            let first = line.iter().map(|t| t[0]).fold(f64::INFINITY, f64::min);
            let last = line
                .iter()
                .map(|t| t[0] + t[2])
                .fold(f64::NEG_INFINITY, f64::max);
            let ground = rects
                .iter()
                .find(|[_, top, ..]| !near(*top, 0.0))
                .unwrap_or_else(|| panic!("{at}: no ground for the chosen row in {rects:?}"));
            let round = [
                first - ground[0],
                ground[0] + ground[2] - last,
                baselines[chosen] - cap - ground[1],
                ground[1] + ground[3] - baselines[chosen],
            ];
            assert!(
                round.iter().all(|gap| (gap - mark).abs() < 0.5),
                "{at}: the chosen ground stands {round:?} pt round its ink (left, right, over the capitals, under the baseline), not a mark ({mark} pt)"
            );
        }
    }

    #[test]
    fn the_menu_and_the_lists_keep_the_ladder() {
        let ask = |px_per_pt: f64, outline: Vec<(u8, String, bool)>| Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline,
            menu: None,
            note: String::new(),
            mark: "",
            typed: String::new(),
            cursor: false,
            count: None,
            width_pt: 1080.0,
            px_per_pt,
            page: crate::page::Width::WHOLE,
        };
        let menu = Menu {
            lines: vec![
                ("bn[ext]".into(), "Next File".into(), "]b".into(), false),
                (
                    "bp[revious]".into(),
                    "Previous File".into(),
                    "[b".into(),
                    true,
                ),
            ],
            all: vec!["bn[ext]".into(), "bp[revious]".into()],
        };
        keeps_the_ladder(
            "the menu",
            &|px| menu_source(&ask(px, Vec::new()), &menu),
            2,
            1,
        );
        let rows = vec![
            (1, "The Reader's Cursor".into(), false),
            (2, "What Moves It".into(), true),
            (2, "What Acts There".into(), false),
        ];
        keeps_the_ladder("gO's outline", &|px| outline(&ask(px, rows.clone())), 3, 1);
    }

    #[test]
    fn a_chosen_row_too_long_for_the_outline_sets_its_words_whole_where_its_room_is() {
        let long = "A heading far too long for the outline's row, which goes on and on \
                    past where the page's measure ends, so it reads on in place";
        let ask = Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline: vec![(1, "Short".into(), false), (2, long.into(), true)],
            menu: None,
            note: String::new(),
            mark: "",
            typed: String::new(),
            cursor: false,
            count: None,
            width_pt: 500.0,
            px_per_pt: 2.0,
            page: crate::page::Width::WHOLE,
        };
        let mut world = page::new_world();
        world.set_main(&outline(&ask));
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .unwrap_or_else(|errors| {
                panic!(
                    "{:?}",
                    errors
                        .iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                )
            });
        let pages = document.pages();
        assert_eq!(
            pages.len(),
            3,
            "the list, the chosen row's words whole, a figure"
        );
        let room = outline_room(&ask).expect("a chosen row's room");
        assert_eq!(room.said, long);
        assert!(
            pages[1].frame.size().x.to_pt() > room.width_pt,
            "its words run past it"
        );
        assert!((pages[1].frame.size().y.to_pt() - row_pt()).abs() < 0.01);
        let figure = pages[2].frame.size().x.to_pt();
        assert!(figure > 2.0 && figure < 12.0, "a figure is {figure} pt");
        let mut texts = Vec::new();
        texts_at(&pages[0].frame, 0.0, 0.0, &mut texts);
        let chosen = texts
            .iter()
            .filter(|(_, y)| *y > room.top_pt && *y < room.top_pt + row_pt())
            .map(|(x, _)| *x)
            .fold(f64::INFINITY, f64::min);
        assert!(
            (chosen - room.left_pt).abs() < 0.5,
            "its words start at {chosen} pt, its room at {} pt",
            room.left_pt
        );
        let mut whole = Vec::new();
        texts_at(&pages[1].frame, 0.0, 0.0, &mut whole);
        let row = texts
            .iter()
            .filter(|(_, y)| *y > room.top_pt)
            .map(|(_, y)| y - room.top_pt)
            .fold(f64::INFINITY, f64::min);
        assert!(
            (whole[0].1 - row).abs() < 0.5,
            "the whole words stand on the row's baseline"
        );
    }

    #[test]
    fn only_a_chosen_row_has_a_room_to_read_on_in() {
        let mut ask = Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline: vec![(1, "One".into(), false)],
            menu: None,
            note: String::new(),
            mark: "",
            typed: String::new(),
            cursor: false,
            count: None,
            width_pt: 720.0,
            px_per_pt: 2.0,
            page: crate::page::Width::WHOLE,
        };
        assert_eq!(outline_room(&ask), None);
        ask.outline = Vec::new();
        assert_eq!(outline_room(&ask), None, "the menu's rows don't read on");
    }

    #[test]
    fn the_menu_sets_each_line_and_the_row_says_how_to_choose() {
        let mut ask = Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            outline: Vec::new(),
            menu: None,
            note: "Tab choose".into(),
            mark: ":",
            typed: "b".into(),
            cursor: true,
            count: None,
            width_pt: 720.0,
            px_per_pt: 2.0,
            page: crate::page::Width::WHOLE,
        };
        assert!(source(&ask).contains("\"Tab choose\""));
        let menu = Menu {
            lines: vec![
                ("bn[ext]".into(), "Next File".into(), "]b".into(), true),
                (
                    "bp[revious]".into(),
                    "Previous File".into(),
                    "[b".into(),
                    false,
                ),
            ],
            all: vec!["bn[ext]".into()],
        };
        let set = menu_source(&ask, &menu);
        assert!(set.contains("\"bn[ext]\"") && set.contains("\"Previous File\""));
        assert_eq!(
            set.matches("ink.selection").count(),
            1,
            "the chosen line's ground"
        );
        assert!(
            set.contains("(\"bn[ext]\", )"),
            "every command, as an array"
        );
        ask.menu = Some(menu);
        assert_eq!(slot(&ask), 3, "set where the outline is");
    }
}
