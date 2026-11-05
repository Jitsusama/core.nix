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

/// A row of the key list: the page's 25 px rhythm, as 18.75 pt.
pub const KEY_ROW_PT: f64 = 18.75;

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
/// command line and the launcher's over its rows: a hairline in the rule
/// splitting it from the page, then each line on the page's rhythm, all
/// at Note in muted. Its commands stand under what's typed after the :,
/// so a command chosen reads straight down from the line; its titles on a
/// column Space's xl past the widest command it could hold; its keys
/// flush with the end of the page's measure, where the prompt's count
/// ends. The chosen line takes the selection's ground over the measure,
/// as gO's chosen row does, its words in the text, since muted isn't
/// drawn there.
fn menu_source(ask: &Ask, menu: &Menu) -> String {
    let rows: Vec<String> = menu
        .lines
        .iter()
        .map(|(command, title, key, chosen)| {
            let (fill, ink) = if *chosen {
                ("ink.selection", "ink.text")
            } else {
                ("none", "ink.muted")
            };
            format!(
                "block(width: 100%, height: {row}pt, fill: {fill}, inset: (x: {SPREAD_PT}pt), {{ set text(fill: {ink}); place(left + horizon, dx: commands, {command}); place(left + horizon, dx: titles, {title}); place(right + horizon, {key}) }})",
                row = KEY_ROW_PT,
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
#context {{
  let commands = measure(text(..level.figure.text, ":")).width
  let widest = calc.max(0pt, ..({all}).map(it => measure(text(it)).width))
  let titles = commands + widest + {xl}pt
  block(width: 100%, stroke: (top: 0.75pt + ink.rule), inset: (x: {side}pt, y: {xs}pt), stack({rows}))
}}
"#,
        ink = crate::theme::get().typst(),
        // The hairline runs the window across, and the rows the measure,
        // the chosen one's ground reaching Space's s past it on either
        // side, as gO's chosen row and a selection on the page do.
        side = (((width - ask.page.page_pt()) / 2.0).max(0.0) + ask.page.spread_pt - SPREAD_PT)
            .max(0.0),
        xl = SIDE_PT,
        xs = SPREAD_PT / 2.0,
        rows = rows.join(", "),
    )
}

/// gO's outline's Typst, as his location list stands below a window: on
/// the sunken ground the prompt's row is on, the full width of the window,
/// each heading's words where the page's measure starts, set in from it by
/// two characters for each level below the outline's highest, as Neovim's
/// gO sets them in, each a note in the text; the row chosen on the selection's ground in the strong
/// text, as the launcher's chosen row and his `CursorLine` in a list. Its rows stand on the key list's
/// 18.75 pt, the page's 25 px rhythm, with Space's l above and below.
fn outline(ask: &Ask) -> String {
    let (width, side, highest) = outline_measure(ask);
    let rows: Vec<String> = ask
        .outline
        .iter()
        .map(|(level, words, chosen)| {
            let (fill, ink) = if *chosen {
                ("ink.selection", "ink.strong")
            } else {
                ("none", "ink.text")
            };
            format!(
                "block(width: 100%, height: {KEY_ROW_PT}pt, fill: {fill}, inset: (x: {SPREAD_PT}pt), align(horizon, pad(left: {indent}pt, text(fill: {ink}, fit({words})))))",
                indent = indent_pt(*level, highest),
                words = quoted(words),
            )
        })
        .collect();
    // Past the list, the chosen row's words whole on a page as wide as
    // they run, and a figure, whose width sets the pace they read on at.
    let whole = ask
        .outline
        .iter()
        .find(|(_, _, chosen)| *chosen)
        .map(|(_, words, _)| {
            format!(
                "#page(width: auto, height: {KEY_ROW_PT}pt, margin: 0pt, fill: ink.selection, align(horizon, text(fill: ink.strong, {words})))\n#page(width: auto, height: auto, margin: 0pt, fill: none, [0])",
                words = quoted(words),
            )
        })
        .unwrap_or_default();
    format!(
        r#"{ink}#set page(width: {width}pt, height: auto, margin: (x: {side}pt, y: {OUTLINE_MARGIN_PT}pt), fill: ink.sunken)
#set text(..level.note.text, features: level.note.text.features + ("calt",))
// A row is one line: words too long for it end in an ellipsis, at the
// last word that fits, as a long name does on the launcher's rows.
#let fit(words) = layout(size => {{
  if measure(words).width <= size.width {{ return words }}
  let kept = words.split(" ")
  while kept.len() > 1 and measure(kept.join(" ") + "\u{{2026}}").width > size.width {{
    kept = kept.slice(0, -1)
  }}
  kept.join(" ") + "\u{{2026}}"
}})
#stack({rows})
{whole}
"#,
        ink = crate::theme::get().typst(),
        rows = rows.join(", "),
    )
}

/// gO's outline's measure, in points: its set's width, its side margin,
/// where the words start where the page's measure does with the chosen
/// row's ground reaching past them on either side, and the highest level
/// among its rows.
fn outline_measure(ask: &Ask) -> (f64, f64, u8) {
    let highest = ask
        .outline
        .iter()
        .map(|(level, _, _)| *level)
        .min()
        .unwrap_or(1);
    let width = ask.width_pt.max(290.0 + 72.0);
    let side =
        (((width - ask.page.page_pt()) / 2.0).max(0.0) + ask.page.spread_pt - SPREAD_PT).max(0.0);
    (width, side, highest)
}

/// How far a row is set in: two characters, as 1.2 of the note's ems, for
/// each level below the outline's highest, as Neovim's gO sets them in.
fn indent_pt(level: u8, highest: u8) -> f64 {
    f64::from(level.saturating_sub(highest)) * 1.2 * crate::theme::get().levels.note.size
}

/// The room the chosen row's words stand in, when an outline is asked for
/// and a row of it is chosen: past the margin, the ground's spread and the
/// row's indent, and down the list's top margin and the rows above it.
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
        left_pt: side + SPREAD_PT + indent,
        top_pt: OUTLINE_MARGIN_PT + above * KEY_ROW_PT,
        width_pt: width - 2.0 * (side + SPREAD_PT) - indent,
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

/// The outline's margin above and below its rows: Space's l.
const OUTLINE_MARGIN_PT: f64 = 12.0;

/// How far a chosen row's ground reaches past its words on either side:
/// Space's s, so its fill sits close round its ink.
const SPREAD_PT: f64 = 6.0;

/// The key list's Typst, as his which-key draws it: on the sunken ground,
/// as notes, the keys in the text and what they do muted, each group
/// under its name as a label, in two columns. Its gaps are Space's steps in points:
/// xl round it, xxl between a key and its words and between the columns,
/// l between groups and s under a group's name; its rows stand on the
/// page's 25 px rhythm. The groups in a column share one edge for their
/// words, measured from the widest keys in it. Each column is a stack of
/// its own, the groups shared out so the two stand as near one height as
/// they can.
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
            widest[column].push(quoted(&ways.join(" \u{b7} ")));
            let keys: Vec<String> = ways
                .iter()
                .map(|way| format!("text(fill: ink.text, {})", quoted(way)))
                .collect();
            cells.push(format!(
                "[#({})]",
                keys.join(" + text(fill: ink.muted, \" · \") + ")
            ));
            cells.push(format!("[#{}]", quoted(does)));
        }
        groups[column].push(format!(
            "stack(spacing: 6pt, at(level.label, {}), grid(columns: (keys.at({column}), auto), rows: 18.75pt, align: horizon, column-gutter: 24pt, {}))",
            quoted(group),
            cells.join(", ")
        ));
        // Alone in a column that may be narrower than its words, a row
        // grows for words that wrap, each line the rhythm's 18.75 pt with
        // its capitals where a fixed row centres them.
        alone.push(format!(
            "stack(spacing: 6pt, at(level.label, {}), {{ set text(top-edge: (18.75pt + cap) / 2, bottom-edge: -(18.75pt - cap) / 2); set par(leading: 0pt); grid(columns: (calc.max(..keys), auto), column-gutter: 24pt, {}) }})",
            quoted(group),
            cells.join(", ")
        ));
    }
    let stacks: Vec<String> = groups
        .iter()
        .map(|column| format!("stack(spacing: 12pt, {})", column.join(", ")))
        .collect();
    format!(
        r"{}#set page(width: auto, height: auto, margin: {SIDE_PT}pt, fill: ink.sunken)
#set text(..level.note.text)
#context {{
  let cap = measure([H]).height
  let keys = (({}), ({})).map(column => calc.max(0pt, ..column.map(it => measure(text(it)).width)))
  let note = [A number first does it that many times: 5j goes five lines down.]
  let two = stack(spacing: 12pt, grid(columns: 2, column-gutter: 24pt, {}), note)
  let one = stack(spacing: 12pt, {}, note)
  let room = {room}pt
  if measure(two).width <= room {{ two }} else {{ block(width: calc.min(measure(one).width, room), one) }}
}}
",
        crate::theme::get().typst(),
        widest[0].join(", ") + ",",
        widest[1].join(", ") + ",",
        stacks.join(", "),
        alone.join(", "),
        room = (width_pt - 4.0 * SIDE_PT).max(KEYS_LEAST_PT),
    )
}

/// Space's xl, round the key list's words and between it and the window's
/// sides.
const SIDE_PT: f64 = 18.0;

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
                width <= window - 2.0 * SIDE_PT,
                "the key list is {width:.0} pt wide in a window {window} pt wide"
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
        assert!((pages[1].frame.size().y.to_pt() - KEY_ROW_PT).abs() < 0.01);
        let figure = pages[2].frame.size().x.to_pt();
        assert!(figure > 2.0 && figure < 12.0, "a figure is {figure} pt");
        let mut texts = Vec::new();
        texts_at(&pages[0].frame, 0.0, 0.0, &mut texts);
        let chosen = texts
            .iter()
            .filter(|(_, y)| *y > room.top_pt && *y < room.top_pt + KEY_ROW_PT)
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
