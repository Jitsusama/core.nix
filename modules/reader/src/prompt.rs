//! What the reader sets apart from the page: the search prompt at the
//! window's foot, as Neovim's command line, Space ?'s key list, as his
//! which-key, and g x's labels over links, as his flash. Each is set by Typst in the page's code face on a thread of
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
    pub forward: bool,
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
}

/// The prompt set, as pixels.
pub struct Set {
    pub serial: u32,
    pub keys: bool,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Each label asked for, set on a page of its own, in the order asked.
    pub labels: Vec<Label>,
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

/// Which of the three an ask is, so only the newest of each is set.
fn slot(ask: &Ask) -> usize {
    if ask.labels.is_empty() {
        usize::from(ask.keys)
    } else {
        2
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
                    forward: true,
                    typed: String::new(),
                    cursor: true,
                    count: None,
                    width_pt: 720.0,
                    px_per_pt: 1.0,
                }));
                let _ = typst::compile::<PagedDocument>(&world);
                while let Ok(first) = waiting.recv() {
                    // Only the newest ask of each is worth setting.
                    let mut newest: [Option<Ask>; 3] = [None, None, None];
                    let at = slot(&first);
                    newest[at] = Some(first);
                    while let Ok(newer) = waiting.try_recv() {
                        let at = slot(&newer);
                        newest[at] = Some(newer);
                    }
                    for ask in newest.into_iter().flatten() {
                        let text = if !ask.labels.is_empty() {
                            labels(&ask.labels)
                        } else if ask.keys {
                            key_list()
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
                            Set {
                                serial: ask.serial,
                                keys: ask.keys,
                                width: first.width,
                                height: first.height,
                                pixels: first.pixels,
                                labels: Vec::new(),
                            }
                        } else {
                            Set {
                                serial: ask.serial,
                                keys: false,
                                width: 0,
                                height: 0,
                                pixels: Vec::new(),
                                labels: pages.collect(),
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
    let mark = if ask.forward { "/" } else { "?" };
    let count = match ask.count {
        Some((_, 0)) if !ask.typed.is_empty() => "Nothing matches.".into(),
        None | Some((_, 0)) => String::new(),
        Some((at, all)) => format!("{at} of {all}"),
    };
    let cursor = if ask.cursor {
        r"#box(width: 0.6em, height: 1.1em, baseline: 0.2em, fill: ink.strong)"
    } else {
        ""
    };
    let width = ask.width_pt.max(290.0 + 72.0);
    format!(
        r#"{ink}#set page(width: {width}pt, height: {row}pt, margin: (x: {side}pt, y: 0pt), fill: ink.sunken)
#set text(font: face.code, size: 11.5pt, fill: ink.text, features: ("calt",))
#align(horizon, grid(columns: (1fr, auto),
  [#raw({typed}){cursor}],
  text(fill: ink.muted, {count})))
"#,
        ink = crate::theme::get().typst(),
        // Its words start where the page's measure does, as the window
        // centres the page, and end where it ends.
        side = ((width - crate::page::WIDTH_PT) / 2.0).max(0.0) + crate::page::SPREAD_PT,
        row = ROW_PT,
        typed = quoted(&format!("{mark}{}", ask.typed)),
        count = quoted(&count),
    )
}

/// The key list's Typst, as his which-key draws it: on the sunken ground,
/// keys in the accent and what they do in the text, each group under its
/// name in strong, in two columns. Its gaps are Space's steps in points:
/// xl round it, xxl between a key and its words and between the columns,
/// l between groups and s under a group's name; its rows stand on the
/// page's 25 px rhythm. The groups in a column share one edge for their
/// words, measured from the widest keys in it.
fn key_list() -> String {
    let mut groups = Vec::new();
    let mut widest: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for (index, (group, rows)) in crate::keys::listing().into_iter().enumerate() {
        let column = index % 2;
        let mut cells = Vec::new();
        for (ways, does) in rows {
            widest[column].push(quoted(&ways.join(" \u{b7} ")));
            let keys: Vec<String> = ways
                .iter()
                .map(|way| format!("text(fill: ink.accent, {})", quoted(way)))
                .collect();
            cells.push(format!(
                "[#({})]",
                keys.join(" + text(fill: ink.muted, \" · \") + ")
            ));
            cells.push(format!("[#{}]", quoted(does)));
        }
        groups.push(format!(
            "stack(spacing: 6pt, text(fill: ink.strong, weight: 700, {}), grid(columns: (keys.at({column}), auto), rows: 18.75pt, align: horizon, column-gutter: 24pt, {}))",
            quoted(group),
            cells.join(", ")
        ));
    }
    format!(
        r#"{}#set page(width: auto, height: auto, margin: 18pt, fill: ink.sunken)
#set text(font: face.code, size: 11.5pt, fill: ink.text)
#context {{
  let keys = (({}), ({})).map(column => calc.max(0pt, ..column.map(it => measure(text(it)).width)))
  stack(spacing: 12pt, grid(columns: 2, column-gutter: 24pt, row-gutter: 12pt, {}),
    text(fill: ink.muted, "A number first does it that many times: 5j goes five lines down."))
}}
"#,
        crate::theme::get().typst(),
        widest[0].join(", ") + ",",
        widest[1].join(", ") + ",",
        groups.join(", ")
    )
}

/// The labels' Typst, as his flash draws a label: the letters in strong
/// weight in the colour on the accent, on the accent, each on a page of its
/// own just round its letters, with Space's xs at its sides.
fn labels(each: &[String]) -> String {
    let pages: Vec<String> = each
        .iter()
        .map(|label| format!("#text({})", quoted(label)))
        .collect();
    format!(
        r#"{}#set page(width: auto, height: auto, margin: (x: 3pt, y: 0pt), fill: ink.accent)
#set text(font: face.code, size: 11.5pt, weight: 700, fill: ink.on_accent, top-edge: "ascender", bottom-edge: "descender")
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
    fn quoting_keeps_what_is_typed() {
        assert_eq!(quoted(r#"a "b" \c"#), r#""a \"b\" \\c""#);
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

    #[test]
    fn the_prompt_says_when_nothing_matches_only_once_something_is_typed() {
        let ask = |typed: &str, count| Ask {
            serial: 1,
            keys: false,
            labels: Vec::new(),
            forward: true,
            typed: typed.into(),
            cursor: true,
            count,
            width_pt: 720.0,
            px_per_pt: 2.0,
        };
        assert!(source(&ask("cat", Some((0, 0)))).contains("Nothing matches."));
        assert!(!source(&ask("", Some((0, 0)))).contains("Nothing matches."));
        assert!(source(&ask("cat", Some((2, 5)))).contains("2 of 5"));
    }
}
