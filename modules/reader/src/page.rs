//! The page: the markdown set once in Typst as one tall page, its text one
//! measure of about 66 characters with room either side for a wide table to
//! spread into, on one pitch and one type scale. The window draws it
//! centred, so its margins are what the window leaves.

use std::path::PathBuf;
use std::time::Instant;

use typst_layout::{Page, PagedDocument};

use crate::walk::{self, Costs, Pieces};
use crate::world::PageWorld;

/// The measure: 580 px, about 66 characters of the text face, the most a
/// line should hold.
pub const MEASURE_PT: f64 = 435.0;

/// How far a wide table may spread past the measure on each side, which
/// the page keeps as its side margins: in a window half the laptop's panel
/// wide (720 px, 540 pt), what's left once Space's xxl stays inside each
/// edge, so a spread table never reaches the window's edge.
pub const SPREAD_PT: f64 = 28.5;

/// The page's width: its measure and the room on each side.
pub const WIDTH_PT: f64 = MEASURE_PT + 2.0 * SPREAD_PT;

/// Above the first line's capitals: Space's xxl, 24 pt. Lines start at
/// their capitals' height (Typst's top edge), so the margin is the gap.
pub const TOP_PT: f64 = 24.0;

/// The page's line: 28 px, seven of Space's 4 px units, from one baseline
/// to the next, and what j and k move by. Every baseline lands on a half
/// of it.
pub const PITCH_PT: f64 = 21.0;

/// The page's look, after its measure, margins, size and line (template).
///
/// The words are the theme's serif, a variable face, at the axes it was
/// drawn for at a reading size: optical size 14, half soft and without its
/// wonky forms, at 380 (`text-axes`). Headings and ornaments take its
/// poster's cut, optical size 144, fully soft and wonky (`display-axes`).
/// Typst sets a face's optical size to the text's by itself, and every
/// other axis from `variations`, the later winning, so these name all
/// three. A line is a capital's height and the leading below it, so the
/// leading is the pitch less the capitals, and a gap between paragraphs is
/// said as baseline to baseline (`gap`).
const PAGE: &str = r#"#import "/mitex/specs/mod.typ": mitex-scope
#let text-axes = (opsz: 14, SOFT: 50, WONK: 0)
#let display-axes = (opsz: 144, SOFT: 100, WONK: 1)
#let half = pitch / 2
#let gap(n) = n * pitch - cap
#set text(font: face.text, size: body-size, weight: 380, variations: text-axes, fill: ink.text, lang: "en", hyphenate: true, costs: (hyphenation: 200%, runt: 300%), top-edge: "cap-height", bottom-edge: "baseline")
#set par(justify: false, linebreaks: "optimized", leading: pitch - cap, spacing: gap(1.5))
// Strong is the words a step heavier, at 480, in the strong text: Typst's
// own adds 300, which draws this face black.
#set strong(delta: 100)
#show strong: set text(fill: ink.strong)
#show heading: set text(font: face.wide, weight: 700, fill: ink.strong, hyphenate: false)
// A heading's lines keep their own leading, by their size, not the words'.
#show heading: set par(leading: 0.62em)
#show heading.where(level: 1): set text(size: 40pt)
#show heading.where(level: 2): set text(size: 16.4pt)
// Code in his mono at 0.93 of the words, so its x-height meets theirs
// (1035 and 964 of 2000), in the text's colour and with no box. Typst's
// raw takes 0.8 em of its own, so the size is said whole.
#show raw: set text(font: face.code, size: 0.93 * body-size)
// A link's words in the strong text with a hairline under them, as his
// Neovim sets a link, since a hue is a glyph's and never text's.
#show link: it => underline(stroke: 0.75pt + ink.rule, offset: 2.5pt, text(fill: ink.strong, it))
#show math.equation: set text(font: "New Computer Modern Math")
#let mx(s, b) = math.equation(block: b, eval(s, mode: "math", scope: mitex-scope))
#let broken(s) = text(fill: ink.muted, raw(s))
#let rule() = line(length: 100%, stroke: 0.5pt + ink.rule)
#let kbd(body) = box(stroke: 0.5pt + ink.rule, inset: (x: 3pt, y: 1pt), radius: 2pt, body)
#let task(done) = if done [☑ ] else [☐ ]
#let alert(kind, body) = block(stroke: (left: 2pt + ink.accent), inset: (left: 12pt, y: 4pt), [#text(weight: 700, upper(kind)) #parbreak() #body])
#let details(body) = block(body)
#let rule-ink = ink.rule
#let sheet-gap = 18pt
#let sheet(numeric: (), words: (), head: none, rows: ()) = layout(size => {
  let n = words.len()
  let all = if head == none { rows } else { (head,) + rows }
  let cell(row, i) = row.at(i, default: [])
  let wide(it) = measure(text(hyphenate: false, it)).width
  let mins = words.map(column => calc.max(0pt, ..column.map(word => measure(word).width)))
  let naturals = range(n).map(i => calc.max(0pt, ..all.enumerate().map(((k, row)) => wide(if head != none and k == 0 { strong(cell(row, i)) } else { cell(row, i) }))))
  let wants = range(n).map(i => naturals.at(i) - mins.at(i))
  // Every line of a table lands on the page's pitch: a body line's cap
  // height and its leading. A line in a cell steps one pitch, a field to
  // the next steps one, and a hairline takes half of one more, centred,
  // so the next first baseline comes one and a half after the last.
  let cap = measure([H]).height
  let pitch = cap + par.leading.to-absolute()
  let half = (1.5 * pitch - cap) / 2
  let fit(avail) = {
    let room = avail - sheet-gap * (n - 1)
    if mins.sum(default: 0pt) > room { return none }
    if naturals.sum(default: 0pt) <= room { return naturals }
    let widths = mins
    let open = range(n).filter(i => naturals.at(i) > mins.at(i))
    let rest = room - mins.sum(default: 0pt)
    while open.len() > 0 and rest > 0.01pt {
      let share = open.map(i => wants.at(i)).sum(default: 0pt)
      let given = 0pt
      let still = ()
      for i in open {
        let want = if share > 0pt { rest * (wants.at(i) / share) } else { rest / open.len() }
        let left = naturals.at(i) - widths.at(i)
        let take = calc.min(want, left)
        widths.at(i) = widths.at(i) + take
        given += take
        if take < left { still.push(i) }
      }
      rest -= given
      if still.len() == open.len() { break }
      open = still
    }
    widths
  }
  let grid-of(widths) = {
    set text(hyphenate: false, top-edge: cap, bottom-edge: "baseline")
    // The gap between columns is each cell's own right inset, so a row's
    // hairline runs across it unbroken.
    table(
      columns: range(n).map(i => widths.at(i) + if i < n - 1 { sheet-gap } else { 0pt }),
      inset: (x, y) => (left: 0pt, right: if x < n - 1 { sheet-gap } else { 0pt }, y: half),
      stroke: (x, y) => if y > 0 { (top: 0.75pt + rule-ink) },
      align: (x, y) => if numeric.at(x, default: false) { right } else { left },
      ..all.enumerate().map(((k, row)) => range(n).map(i => {
        let it = cell(row, i)
        let it = if numeric.at(i, default: false) { text(number-width: "tabular", it) } else { it }
        if head != none and k == 0 { strong(it) } else { it }
      })).flatten()
    )
  }
  let records = {
    let names = if head == none { () } else { head }
    // One edge for every record's values, past the widest header.
    let label = calc.max(0pt, ..names.map(name => measure(name).width))
    let lead = pitch - cap
    set text(hyphenate: false, top-edge: cap, bottom-edge: "baseline")
    stack(..rows.enumerate().map(((k, row)) => block(
      width: 100%,
      inset: (y: half),
      stroke: if k > 0 { (top: 0.75pt + rule-ink) } else { none },
      stack(spacing: lead,
        text(fill: ink.strong, weight: 500, cell(row, 0)),
        grid(columns: (label, 1fr), column-gutter: 12pt, row-gutter: lead,
          ..range(1, n).filter(i => cell(row, i) != []).map(i => (text(fill: ink.muted, cell(names, i)), cell(row, i))).flatten()))
    )))
  }
  let inside = fit(size.width)
  if inside != none { grid-of(inside) } else {
    let spread-out = fit(size.width + 2 * spread)
    if spread-out != none { pad(left: -spread, right: -spread, grid-of(spread-out)) } else { records }
  }
})
#let codeblock(body) = block(fill: ink.sunken, inset: 10pt, radius: 3pt, width: 100%, {
  // Code keeps its own line, by its size, not the words'.
  set par(leading: 0.62em)
  text(font: face.code, size: 11.5pt, body)
})
#let picture(alt, w, h) = box(width: calc.min(w, measure-width), height: h, fill: ink.raised, align(center + horizon, text(fill: ink.muted, alt)))
#let diagram(path) = block(width: 100%, image(path, width: 100%))
#let pictures = true
#let pic(path, alt, w) = if not pictures { picture(alt, 320pt, 160pt) } else {
  box(layout(size => {
    let natural = if w == none { measure(image(path, alt: alt)).width } else { w }
    image(path, alt: alt, width: calc.min(natural, size.width))
  }))
}
"#;

/// What every page is set with: the theme's ink, then the page's look,
/// its words' capitals read from the face `world` finds for them.
pub fn template(world: &PageWorld) -> &'static str {
    static TEMPLATE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let theme = crate::theme::get();
        let size = theme.sizes.text;
        let cap = world.capitals_pt(&theme.faces.text, size);
        format!(
            "{ink}#set page(width: {WIDTH_PT}pt, height: auto, margin: (x: {SPREAD_PT}pt, top: {TOP_PT}pt, bottom: {BOTTOM_PT}pt), fill: ink.background)\n#let measure-width = {MEASURE_PT}pt\n#let spread = {SPREAD_PT}pt\n#let body-size = {size}pt\n#let pitch = {PITCH_PT}pt\n#let cap = {cap}pt\n{PAGE}",
            ink = theme.typst(),
        )
    })
}

fn compile(world: &PageWorld) -> Result<PagedDocument, String> {
    typst::compile::<PagedDocument>(world)
        .output
        .map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
}

pub fn new_world() -> PageWorld {
    let fonts: Vec<String> = std::env::var("READER_FONTS")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let mitex = PathBuf::from(std::env::var("READER_MITEX").unwrap_or_default());
    PageWorld::new(&fonts, &mitex)
}

/// About two screens of source: enough for the first screen and a scroll
/// while the rest is set, in 50 to 170 ms.
const FIRST_CHARS: usize = 6000;

/// The bottom margin the page ends on: Space's xxl below the last line's
/// descenders, which hang 0.245 em of the words' 13.69 pt below their
/// baseline (a g's 490 of the serif's 2000), where Typst ends a line. A
/// page set from the file's first blocks lays out as the whole file's does
/// down to here.
pub const BOTTOM_PT: f64 = 27.4;

/// The file set as far as its first screen, and what sets the rest.
pub struct First {
    pub page: Page,
    pub ms: f64,
    pub rest: Option<Rest>,
    /// Each part of the set, with its time and the time of it spent waiting
    /// for a processor, in milliseconds.
    pub parts: Vec<(&'static str, f64, f64)>,
}

/// Times a part of the set into `parts`.
#[expect(
    clippy::cast_precision_loss,
    reason = "nanoseconds of a set, which never pass a mantissa's 52 bits"
)]
fn timed<T>(
    parts: &mut Vec<(&'static str, f64, f64)>,
    name: &'static str,
    part: impl FnOnce() -> T,
) -> T {
    let start = Instant::now();
    let waited = crate::run_delay_ns();
    let out = part();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let waited = crate::run_delay_ns().saturating_sub(waited) as f64 / 1e6;
    parts.push((name, ms, waited));
    out
}

/// The rest of the file, to set off the main thread once the first screen
/// shows.
pub struct Rest {
    world: PageWorld,
    program: String,
}

impl Rest {
    /// Sets the whole file as one tall page, and says how long that took.
    pub fn set(mut self) -> Result<(Page, f64), String> {
        let start = Instant::now();
        let page = set_page(&mut self.world, &self.program)?;
        drop(self);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        settled();
        Ok((page, ms))
    }
}

/// Sets the file's first blocks, up to about two screens of source, as a
/// page of their own; the rest comes back to set, unless that was all.
/// Given a block to open at, it sets as far as that block and about two
/// screens past it.
pub fn set_first(file: &str, reach: Option<usize>) -> Result<First, String> {
    let start = Instant::now();
    let mut parts = Vec::new();
    let markdown = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let mut world = timed(&mut parts, "world", new_world);
    let pieces = timed(&mut parts, "pieces", Pieces::new);
    let mut costs = Costs::default();
    let base = std::path::Path::new(file).parent().map(|dir| {
        if dir.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            dir
        }
    });
    let program = timed(&mut parts, "walk", || {
        walk::program(&markdown, base, &pieces, &world, &mut costs)
    });
    let (head, blocks) = walk::blocks(&program, &world);
    let mut first = head;
    let mut chars = 0;
    let mut all = true;
    // The program's first piece comes before any block, so block n is the
    // piece after it.
    let from = reach.map_or(0, |block| block + 1);
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            first.push_str("\n// block\n");
        }
        first.push_str(block);
        if i >= from {
            chars += block.len();
        }
        if chars > FIRST_CHARS {
            all = i + 1 == blocks.len();
            break;
        }
    }
    let page = timed(&mut parts, "compile", || set_page(&mut world, &first))?;
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let rest = if all {
        drop(world);
        settled();
        None
    } else {
        Some(Rest { world, program })
    };
    Ok(First {
        page,
        ms,
        rest,
        parts,
    })
}

/// Sets the file again each time it is saved, keeping its world and its
/// pieces from one save to the next, so diagrams that haven't changed
/// aren't drawn again and Typst's memo of the last set is there to use.
pub struct Setter {
    file: String,
    world: PageWorld,
    pieces: Pieces,
}

impl Setter {
    pub fn new(file: &str) -> Self {
        Setter {
            file: file.to_string(),
            world: new_world(),
            pieces: Pieces::new(),
        }
    }

    /// Sets another file from now on, keeping the world and its pieces, so
    /// a link followed costs no fonts read again.
    pub fn open(&mut self, file: &str) {
        file.clone_into(&mut self.file);
    }

    /// The file it sets.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The whole file as it is now, as one tall page, and how long that took.
    pub fn set(&mut self) -> Result<(Page, f64), String> {
        let start = Instant::now();
        let markdown =
            std::fs::read_to_string(&self.file).map_err(|e| format!("{}: {e}", self.file))?;
        let path = std::path::Path::new(&self.file);
        let base = path.parent().map(|dir| {
            if dir.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                dir
            }
        });
        let mut costs = Costs::default();
        let program = walk::program(&markdown, base, &self.pieces, &self.world, &mut costs);
        let page = set_page(&mut self.world, &program)?;
        Ok((page, start.elapsed().as_secs_f64() * 1000.0))
    }
}

/// Sets `program` in `world`. A picture Typst can't read stops the whole
/// compile, so when one fails the page sets again with every picture as a
/// box saying what it shows, rather than not at all.
fn set_page(world: &mut PageWorld, program: &str) -> Result<Page, String> {
    world.set_main(program);
    match first_page(world) {
        Ok(page) => Ok(page),
        Err(error) if program.contains("#pic(") => {
            eprintln!("reader: setting without pictures, since {error}");
            world.set_main(&program.replacen("#let pictures = true", "#let pictures = false", 1));
            first_page(world)
        }
        Err(error) => Err(error),
    }
}

fn first_page(world: &PageWorld) -> Result<Page, String> {
    let document = compile(world)?;
    document
        .pages()
        .first()
        .cloned()
        .ok_or_else(|| "the page set nothing".to_string())
}

/// The page is set and nothing compiles again until the file changes, so
/// Typst's memo of the compile is only held memory: let it go, and give what
/// malloc kept back to the system.
fn settled() {
    typst::comemo::evict(0);
    trim();
}

/// Gives the memory malloc holds free back to the system.
pub fn trim() {
    unsafe extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    // SAFETY: glibc's malloc_trim takes no pointers and only releases
    // memory that nothing holds.
    unsafe {
        malloc_trim(0);
    }
}

#[cfg(test)]
mod tests {
    use super::Setter;

    /// Every baseline from the first run holding `from` to the last holding
    /// `to`, top to bottom, and the page's pitch from its first paragraph,
    /// which is the page's line.
    fn baselines(name: &str, markdown: &str, from: &str, to: &str) -> (f64, Vec<f64>) {
        let dir = std::env::temp_dir().join(format!("reader-pitch-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let (page, _) = Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let runs = crate::text::runs(&page.frame);
        let text = |run: &crate::text::Run| run.chars.iter().map(|c| c.0).collect::<String>();
        let mut all: Vec<f64> = runs.iter().map(|run| run.baseline_pt).collect();
        all.sort_by(f64::total_cmp);
        all.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        let pitch = all[1] - all[0];
        assert!(
            (pitch - super::PITCH_PT).abs() < 0.01,
            "the words step {pitch} pt, not the page's line of {} pt",
            super::PITCH_PT
        );
        let top = runs
            .iter()
            .filter(|run| text(run).contains(from))
            .map(|run| run.baseline_pt)
            .fold(f64::INFINITY, f64::min);
        let bottom = runs
            .iter()
            .filter(|run| text(run).contains(to))
            .map(|run| run.baseline_pt)
            .fold(f64::NEG_INFINITY, f64::max);
        (
            pitch,
            all.into_iter()
                .filter(|b| *b >= top - 0.01 && *b <= bottom + 0.01)
                .collect(),
        )
    }

    /// Each step between baselines, in hundredths of a pitch.
    fn steps(pitch: f64, lines: &[f64]) -> Vec<i64> {
        lines
            .windows(2)
            .map(|pair| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a step of a few pitches, rounded"
                )]
                let hundredths = ((pair[1] - pair[0]) / pitch * 100.0).round() as i64;
                hundredths
            })
            .collect()
    }

    const OPENING: &str = "The opening paragraph runs long enough to wrap onto a second line and a third, so the page's pitch can be read from its first two baselines without guessing at what Typst set.\n\n";

    #[test]
    fn the_words_land_on_the_pitch_a_paragraph_one_and_a_half() {
        let markdown = format!(
            "{OPENING}The second paragraph runs long enough to wrap onto a second line as well, so a step between paragraphs sits among steps between lines, at Omega.\n"
        );
        let (pitch, lines) = baselines("words", &markdown, "opening", "Omega");
        let steps = steps(pitch, &lines);
        assert!(steps.len() >= 4, "too few lines: {steps:?}");
        assert_eq!(
            steps.iter().filter(|s| **s == 150).count(),
            1,
            "one step between paragraphs, of one and a half lines: {steps:?}"
        );
        assert!(
            steps.iter().all(|s| *s == 100 || *s == 150),
            "a line steps one pitch, a paragraph one and a half: {steps:?}"
        );
    }

    #[test]
    fn a_tables_lines_land_on_the_pitch() {
        let markdown = format!(
            "{OPENING}| Alpha | Wraps | Count |\n|---|---|---:|\n| one | a cell long enough that it wraps onto a second line in its column | 12 |\n| two | `code` in a cell | 3 |\n| three | short | Omega |\n"
        );
        let (pitch, lines) = baselines("table", &markdown, "Alpha", "Omega");
        let steps = steps(pitch, &lines);
        assert!(steps.len() >= 4, "too few lines: {steps:?}");
        assert!(
            steps.iter().all(|s| *s == 100 || *s == 150),
            "a line in a table steps one pitch or, past a hairline, one and a half: {steps:?}"
        );
    }

    #[test]
    fn a_records_lines_land_on_the_pitch() {
        let head = (1..=9)
            .map(|i| format!("Incomprehensibility{i}"))
            .collect::<Vec<_>>()
            .join(" | ");
        let rule = ["---"; 9].join(" | ");
        let row = |first: &str| {
            std::iter::once(first.to_string())
                .chain((2..=9).map(|i| format!("Counterrevolutionaries{i} and a value long enough to wrap onto a second line beside its header")))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let markdown = format!(
            "{OPENING}| {head} |\n| {rule} |\n| {} |\n| {} |\n",
            row("Alpha"),
            row("Omega")
        );
        let (pitch, lines) = baselines("records", &markdown, "Alpha", "Counterrevolutionaries9");
        let steps = steps(pitch, &lines);
        assert!(
            steps.len() >= 10,
            "set as records, too few lines: {steps:?}"
        );
        assert!(
            steps.iter().all(|s| *s == 100 || *s == 150),
            "a field or a wrap steps one pitch, a record one and a half: {steps:?}"
        );
    }
}
