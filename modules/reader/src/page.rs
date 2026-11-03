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
// Code in his mono at 0.93 of the words, as the spec has it: its x-height
// (1035 of 2000) stands a little over theirs (938 at optical size 14), in
// the text's colour and with no box. Typst's
// raw takes 0.8 em of its own, so the size is said whole.
#show raw: set text(font: face.code, size: 0.93 * body-size)
// Headings, set by lines: more room above a heading than below it, so it
// belongs to what follows. A gap is baseline to baseline, rounded to the
// half line so every baseline stays on the grid. A block's top edge is its
// capitals (`caps` of an em in the words' face, the same at both sets of
// axes, and `code-caps` in the mono), so a gap takes the cap height of what
// follows off. A heavier or italic cut of a face that has cuts may stand
// its capitals a little higher or lower, so headings and the lede say
// their top edge as the regular's capitals, keeping every baseline where
// the gaps put it.
#let lines(n, c) = calc.round((n * pitch - cap + c) / half) * half - c
#let title-size = 42pt
#let section-size = 17.1pt
// An eyebrow: the mono's capitals, small, tracked as every eyebrow on the
// desktop is.
#let eyebrow(body) = text(font: face.code, size: 8.4pt, weight: 500, tracking: 0.16em, fill: ink.muted, upper(body))
#show heading: set text(font: face.text, variations: display-axes, fill: ink.strong, hyphenate: false)
// A title's lines two and a half lines apart, a section's one.
#show heading.where(level: 1): set text(size: title-size, weight: 540, tracking: -0.02em, top-edge: caps * title-size)
#show heading.where(level: 1): set par(leading: 2.5 * pitch - caps * title-size)
#show heading.where(level: 1): set block(above: lines(3, caps * title-size), below: 1.5 * pitch - cap)
#show heading.where(level: 2): set text(size: section-size, weight: 520, top-edge: caps * section-size)
#show heading.where(level: 2): set par(leading: pitch - caps * section-size)
#show heading.where(level: 2): set block(above: lines(3, caps * section-size), below: 1.5 * pitch - cap)
#show heading.where(level: 3): set text(variations: text-axes, style: "italic", size: body-size, weight: 460, top-edge: cap)
#show heading.where(level: 3): set block(above: lines(2.5, cap), below: pitch - cap)
#show heading.where(level: 4): it => block(above: lines(2, code-caps * 8.4pt), below: pitch - cap, eyebrow(it.body))
#show heading.where(level: 5): it => block(above: lines(2, code-caps * 8.4pt), below: pitch - cap, eyebrow(it.body))
#show heading.where(level: 6): it => block(above: lines(2, code-caps * 8.4pt), below: pitch - cap, eyebrow(it.body))
// A title of more than one line has its lines balanced: set at the
// narrowest width that keeps the lines it needs at the full measure, found
// in a few halvings, so no word is left alone on its last line.
#show heading.where(level: 1): it => layout(size => {
  let tall(width) = measure(block(width: width, {
    set par(leading: 2.5 * pitch - caps * title-size)
    text(font: face.text, variations: display-axes, size: title-size, weight: 540, tracking: -0.02em, hyphenate: false, top-edge: caps * title-size, it.body)
  })).height
  let full = tall(size.width)
  if full < 2.5 * pitch { return it }
  let (low, high) = (size.width / 2, size.width)
  for _ in range(8) {
    let middle = (low + high) / 2
    if tall(middle) > full { low = middle } else { high = middle }
  }
  set block(width: high)
  it
})
// What the file is and when it changed, over the title and one line above
// it, or over the words when there's no title.
#show <title>: set block(above: 0pt)
#let about(titled: false, body) = block(above: 0pt, below: if titled { lines(1, caps * title-size) } else { 1.5 * pitch - cap }, eyebrow(body))
// The paragraph after the title, in the text face's italic.
#let lede(body) = par(text(style: "italic", weight: 360, top-edge: cap, body))
#show math.equation: set text(font: "New Computer Modern Math")
#let mx(s, b) = math.equation(block: b, eval(s, mode: "math", scope: mitex-scope))
#let broken(s) = text(fill: ink.muted, raw(s))
// The page's thresholds carry its whimsy, never its reading. The ground
// glows faintly at the head: the background lifted 18% toward raised at
// most, so muted keeps Lc 60 on its lightest, fading to the background
// before the page's sides and 900 pt down.
#let glow = color.mix((ink.background, 82%), (ink.raised, 18%), space: rgb)
#set page(background: place(top + center, rect(width: 100%, height: 1800pt, fill: gradient.radial(glow, ink.background, center: (50%, 0%), radius: 50%, relative: "self"))))
// A thematic break is a fleuron: an asterism of Fraunces Display's
// asterisks in the accent, centred, standing as a line two lines from the
// words either side.
#let star = text(font: face.text, variations: display-axes, weight: 500, size: 1.3 * body-size, fill: ink.accent, top-edge: "bounds", bottom-edge: "bounds", "*")
#let fleuron = context {
  let s = measure(star)
  // Its ink stands evenly between the words either side: from the
  // baseline above, and to the capitals below.
  box(width: 2.3 * s.width, height: cap, {
    place(top + center, star)
    place(bottom + left, dy: 1.15 * s.height, star)
    place(bottom + right, dy: 1.15 * s.height, star)
  })
}
#let rule() = block(above: 2 * pitch - cap, below: 2 * pitch - cap, width: 100%, align(center, fleuron))
// The page ends with a small accent lozenge after its last word.
#let the-end = box(polygon(fill: ink.accent, (0pt, 0.21em), (0.15em, 0pt), (0.3em, 0.21em), (0.15em, 0.42em)))
// A key, in the mono, framed: the frame stands outside the line's room,
// over the capitals and under the descenders, so a line with one still
// steps one line.
#let kbd(body) = box(stroke: 0.5pt + ink.rule, inset: (x: 3pt), outset: (top: 2pt, bottom: 0.3em), radius: 2pt, text(font: face.code, size: 0.93 * body-size, body))
// Lists hang their marks in the margin, so the words keep the measure's
// edge: an accent bullet, a muted number, a task's box. An item steps one
// line and a list stands one and a half from what's around it. A list in a
// list starts at its item's words.
#let hang = 1em
#set list(marker: box(width: 0.6em, align(right, text(fill: ink.accent)[•])), indent: -hang, body-indent: hang - 0.6em, spacing: pitch - cap)
#set enum(numbering: n => box(width: 1.6em, align(right, text(fill: ink.muted, number-width: "tabular")[#n.])), indent: -2 * hang, body-indent: 2 * hang - 1.6em, spacing: pitch - cap)
#let listed(it) = {
  let inner = { set list(indent: 0pt); set enum(indent: 0pt); it }
  if it.indent < 0pt { block(above: 1.5 * pitch - cap, below: 1.5 * pitch - cap, inner) } else { inner }
}
#show list: listed
#show enum: listed
// A task list's items hang their boxes where the bullets would: a hairline
// square in rule, or done, an accent tick and its words muted, never struck.
#let no-mark = box(width: 0.6em)
#let task(done, body) = {
  let side = 0.62em
  let mark = if done {
    box(width: side, height: side, curve(stroke: 1.2pt + ink.accent, curve.move((0.08em, 0.33em)), curve.line((0.25em, 0.55em)), curve.line((0.6em, 0.05em))))
  } else {
    box(width: side, height: side, stroke: 0.75pt + ink.rule)
  }
  h(-hang)
  box(width: hang, mark)
  if done { text(fill: ink.muted, body) } else { body }
}
// A quote in the text face's italic, its words at the measure's edge and an
// accent rule hung 12 pt left of them, two lines from what's around it.
#let hung-rule(colour, first: cap, body) = block(above: lines(2, first), below: 2 * pitch - cap, stroke: (left: 1.5pt + colour), outset: (left: 12pt + 0.75pt), body)
#show quote.where(block: true): it => hung-rule(ink.accent, text(style: "italic", weight: 360, it.body))
// An alert is a quote's shape with its rule in its kind's hue and the kind
// as an eyebrow, one line over its words.
#let alert-hues = (note: ink.voice, tip: ink.accent, important: ink.program, warning: ink.action, caution: ink.machine)
#let alert(kind, body) = hung-rule(alert-hues.at(kind, default: ink.accent), first: code-caps * 8.4pt, {
  block(below: pitch - cap, eyebrow(kind))
  body
})
// A link's words stay text, an accent hairline under them.
#show link: it => underline(stroke: 0.75pt + ink.accent, offset: 0.22em, it)
#let details(body) = block(body)
#let rule-ink = ink.rule
#let sheet-gap = 18pt
// A table's first baseline stands two lines under the one before it and
// the next two lines under its last, past each row's half-line inset, so
// the page's grid runs through it.
#let sheet-air = 2 * pitch - (1.5 * pitch + cap) / 2
#let sheet(numeric: (), words: (), head: none, rows: ()) = block(above: sheet-air, below: sheet-air, layout(size => {
  let n = words.len()
  let all = if head == none { rows } else { (head,) + rows }
  let cell(row, i) = row.at(i, default: [])
  let wide(it) = measure(text(hyphenate: false, it)).width
  let mins = words.map(column => calc.max(0pt, ..column.map(word => measure(word).width)))
  let naturals = range(n).map(i => calc.max(0pt, ..all.enumerate().map(((k, row)) => wide(if head != none and k == 0 { eyebrow(cell(row, i)) } else { cell(row, i) }))))
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
        // A table's heads are eyebrows, on the body's baseline.
        if head != none and k == 0 { eyebrow(it) } else { it }
      })).flatten()
    )
  }
  let records = {
    let names = if head == none { () } else { head }
    // One edge for every record's values, past the widest header.
    let label = calc.max(0pt, ..names.map(name => measure(eyebrow(name)).width))
    let lead = pitch - cap
    set text(hyphenate: false, top-edge: cap, bottom-edge: "baseline")
    stack(..rows.enumerate().map(((k, row)) => block(
      width: 100%,
      inset: (y: half),
      stroke: if k > 0 { (top: 0.75pt + rule-ink) } else { none },
      stack(spacing: lead,
        text(fill: ink.strong, weight: 500, cell(row, 0)),
        grid(columns: (label, 1fr), column-gutter: 12pt, row-gutter: lead,
          ..range(1, n).filter(i => cell(row, i) != []).map(i => (eyebrow(cell(names, i)), cell(row, i))).flatten()))
    )))
  }
  let inside = fit(size.width)
  if inside != none { grid-of(inside) } else {
    let spread-out = fit(size.width + 2 * spread)
    if spread-out != none { pad(left: -spread, right: -spread, grid-of(spread-out)) } else { records }
  }
}))
// A code block: the mono at 10.5 pt on a 15 pt line, on the sunken ground
// inset m by l. Its height rounds up to the half line, shared above and
// below its lines, so the words after it stay on the page's grid. Its top
// stands one and a half lines under the baseline before it, and the next
// baseline two lines under its foot, so the room either side looks even. Code wider than the measure
// widens the block evenly, as far as a table may spread. The language is
// an eyebrow at the top right, on the first line's baseline, or on a line
// of its own when the first line would reach it.
#let code-size = 10.5pt
#let code-line = 15pt
#let code-inset = (x: 12pt, y: 9pt)
// The mono's set rules sit outside `layout`, since `measure` in its
// closure takes the styles `layout` was called with.
#let codeblock(lang: "", first: 0, longest: 0, body) = block(above: 1.5 * pitch, below: 2 * pitch - cap, width: 100%, {
 set text(font: face.code, size: code-size, top-edge: "cap-height", bottom-edge: "baseline", hyphenate: false)
 set par(leading: code-line - code-caps * code-size, justify: false, spacing: code-line - code-caps * code-size)
 layout(size => {
  let advance = measure([0]).width
  let width = calc.min(calc.max(size.width, longest * advance + 2 * code-inset.x), size.width + 2 * spread)
  let inner = width - 2 * code-inset.x
  let label = if lang == "" { none } else { eyebrow(lang) }
  let alone = label != none and first * advance + measure(label).width + code-inset.x > inner
  let lines = {
    if alone { block(width: 100%, below: code-line - code-caps * code-size, align(right, label)) }
    body
  }
  let h = measure(block(width: inner, lines)).height
  let tall = calc.ceil((h + 2 * code-inset.y) / half) * half
  let shown = block(fill: ink.sunken, radius: 4pt, width: width, inset: (x: code-inset.x, y: (tall - h) / 2), {
    if label != none and not alone { place(top + right, dy: code-caps * (code-size - 8.4pt), label) }
    lines
  })
  let out = (width - size.width) / 2
  if out > 0pt { pad(left: -out, right: -out, shown) } else { shown }
 })
})
// A picture that can't be shown says what it shows, in the italic, muted
// and never broken, as a caption would.
#let picture(alt) = text(style: "italic", fill: ink.muted, hyphenate: false, alt)
// A picture standing alone is centred, as wide as it is up to the measure
// and past it as far as a table spreads. Its height rounds up to the half
// line so the words after it stay on the grid, and its alt text is its
// caption, in the italic, muted and centred, a line under it. It sits at
// the foot of its rounded height, so a line of words in its place, as a
// picture that can't be shown is, stands on a line too.
#let alone(body, caption) = block(above: 1.5 * pitch, below: 2 * pitch - cap, width: 100%, pad(left: -spread, right: -spread, layout(size => {
  let shown = align(center, body)
  let h = measure(block(width: size.width, shown)).height
  block(width: 100%, height: calc.ceil(h / half) * half, below: 0pt, align(bottom, shown))
  if caption != [] {
    block(above: pitch - cap, below: 0pt, width: 100%, pad(x: spread, align(center, text(style: "italic", fill: ink.muted, hyphenate: false, caption))))
  }
})))
#let diagram(path) = block(width: 100%, image(path, width: 100%))
#let pictures = true
#let pic(path, alt, w) = if not pictures { picture(alt) } else {
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
        // Each face's capitals in ems, for the gaps over a heading and an
        // eyebrow, so the page holds its line in whatever faces it finds.
        let caps = cap / size;
        let code_caps = world.capitals_pt(&theme.faces.code, 1.0);
        format!(
            "{ink}#set page(width: {WIDTH_PT}pt, height: auto, margin: (x: {SPREAD_PT}pt, top: {TOP_PT}pt, bottom: {BOTTOM_PT}pt), fill: ink.background)\n#let measure-width = {MEASURE_PT}pt\n#let spread = {SPREAD_PT}pt\n#let body-size = {size}pt\n#let pitch = {PITCH_PT}pt\n#let cap = {cap}pt\n#let caps = {caps}\n#let code-caps = {code_caps}\n{PAGE}",
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

/// The eyebrow's words over the title: what the file says it is in its
/// front matter, its kind and id, or else where it lives; and when it last
/// changed. The title says the file's own name, and the front matter
/// itself never shows.
fn about(file: &std::path::Path, markdown: &str) -> String {
    let name = said(markdown).unwrap_or_else(|| lives(file));
    let changed = std::fs::metadata(file)
        .and_then(|it| it.modified())
        .ok()
        .and_then(|at| jiff::Timestamp::try_from(at).ok());
    let Some(changed) = changed else { return name };
    let zone = jiff::tz::TimeZone::system();
    let now = jiff::Timestamp::now().to_zoned(zone.clone());
    format!(
        "{name} \u{b7} changed {}",
        when(&changed.to_zoned(zone), &now)
    )
}

/// Where a file lives: in a repository, its name and the folders down to
/// the file, each after a middot; elsewhere, its folder's name.
fn lives(file: &std::path::Path) -> String {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let Some(folder) = file.parent() else {
        return String::new();
    };
    let named = |path: &std::path::Path| {
        path.file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    };
    let Some(root) = folder.ancestors().find(|it| it.join(".git").exists()) else {
        return named(folder);
    };
    let below = folder
        .strip_prefix(root)
        .unwrap_or(std::path::Path::new(""));
    std::iter::once(named(root))
        .chain(below.iter().map(|part| part.to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// What a file's front matter says it is: its kind and its id, whichever
/// it gives, joined as the eyebrow joins its parts.
fn said(markdown: &str) -> Option<String> {
    let rest = markdown.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let field = |key: &str| {
        rest[..end].lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
            let value = value.trim_matches(|c| c == '"' || c == '\'');
            (!value.is_empty()).then(|| value.to_string())
        })
    };
    let parts: Vec<String> = [field("kind"), field("id")].into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
}

/// When a file changed, as briefly as says it: a clock's time today, a day
/// this year, a day and its year before that.
fn when(changed: &jiff::Zoned, now: &jiff::Zoned) -> String {
    let form = if changed.date() == now.date() {
        "%H:%M"
    } else if changed.year() == now.year() {
        "%-d %b"
    } else {
        "%-d %b %Y"
    };
    changed.strftime(form).to_string()
}

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
    let about = about(std::path::Path::new(file), &markdown);
    let program = timed(&mut parts, "walk", || {
        walk::program(&markdown, &about, base, &pieces, &world, &mut costs)
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
        let program = walk::program(
            &markdown,
            &about(path, &markdown),
            base,
            &self.pieces,
            &self.world,
            &mut costs,
        );
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
    use super::{Setter, lives, when};

    fn at(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> jiff::Zoned {
        jiff::civil::date(year, month, day)
            .at(hour, minute, 0, 0)
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(-5)))
            .expect("a time in a fixed zone")
    }

    #[test]
    fn a_file_lives_in_its_repositorys_folders_or_else_its_own() {
        let dir = std::env::temp_dir().join(format!("reader-lives-{}", std::process::id()));
        let repo = dir.join("core.nix");
        std::fs::create_dir_all(repo.join(".git")).expect("a repository");
        std::fs::create_dir_all(repo.join("docs/notes")).expect("its folders");
        std::fs::create_dir_all(dir.join("loose")).expect("a folder");
        assert_eq!(lives(&repo.join("README.md")), "core.nix");
        assert_eq!(lives(&repo.join("docs/design.md")), "core.nix \u{b7} docs");
        assert_eq!(
            lives(&repo.join("docs/notes/a.md")),
            "core.nix \u{b7} docs \u{b7} notes"
        );
        assert_eq!(lives(&dir.join("loose/page.md")), "loose");
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn a_file_says_what_it_is_in_its_front_matter() {
        assert_eq!(
            super::said("---\nid: QEST-1\nkind: subquest\nstatus: active\n---\n# Title\n"),
            Some("subquest \u{b7} QEST-1".into())
        );
        assert_eq!(
            super::said("---\ntitle: \"Tables\"\nid: 'tables'\n---\n"),
            Some("tables".into())
        );
        assert_eq!(super::said("---\ntitle: Tables\n---\n"), None);
        assert_eq!(super::said("# No front matter\n\nkind: words\n"), None);
    }

    #[test]
    fn a_change_is_said_as_briefly_as_says_when() {
        let now = at(2026, 11, 3, 9, 40);
        assert_eq!(when(&at(2026, 11, 3, 9, 14), &now), "09:14");
        assert_eq!(when(&at(2026, 11, 2, 23, 5), &now), "2 Nov");
        assert_eq!(when(&at(2025, 10, 30, 8, 0), &now), "30 Oct 2025");
    }

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
        // The opening paragraph's first two lines, past the eyebrow and any
        // title over it.
        let opening = runs
            .iter()
            .filter(|run| text(run).contains("opening"))
            .map(|run| run.baseline_pt)
            .fold(f64::INFINITY, f64::min);
        let next = all
            .iter()
            .find(|b| **b > opening + 0.01)
            .expect("a second line");
        let pitch = next - opening;
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
            "{OPENING}The second paragraph runs long enough to wrap onto a second line and a third as well, so a step between paragraphs sits among steps between lines, whichever face and size the page sets its words in, plain or the theme's, before it comes to an end at Omega.\n"
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
    fn a_line_holding_a_key_steps_one_line_as_any_other() {
        let markdown = format!(
            "{OPENING}Press <kbd>Ctrl</kbd>+<kbd>O</kbd> to go back to where you were, and <kbd>Ctrl</kbd>+<kbd>I</kbd> to come forward again, in a paragraph long enough to wrap onto a second line and a third, whichever face it's set in, before it ends at Omega.\n"
        );
        let (pitch, lines) = baselines("keys", &markdown, "opening", "Omega");
        let steps = steps(pitch, &lines);
        assert!(steps.len() >= 4, "too few lines: {steps:?}");
        assert!(
            steps.iter().all(|s| *s == 100 || *s == 150),
            "a key's frame grew its line: {steps:?}"
        );
    }

    #[test]
    fn a_pages_headings_land_on_the_half_line() {
        let markdown = format!(
            "---\nkind: page\n---\n# Title\n\nThe lede.\n\n{OPENING}## Section\n\nWords.\n\n### Part\n\nMore words.\n\n#### Fourth\n\nOmega.\n"
        );
        // The eyebrow says the kind the front matter gives.
        let (pitch, lines) = baselines("headings", &markdown, "PAGE", "Omega");
        let steps = steps(pitch, &lines);
        assert!(
            steps.iter().all(|s| s % 50 == 0),
            "every baseline lands on the half line: {steps:?}"
        );
        assert_eq!(
            steps[..2],
            [200, 150],
            "the title two lines under the eyebrow, the lede one and a half under the title"
        );
    }

    #[test]
    fn a_tables_lines_land_on_the_pitch() {
        let markdown = format!(
            "{OPENING}| Alpha | Wraps | Count |\n|---|---|---:|\n| one | a cell long enough that it wraps onto a second line in its column, however narrow the face its words are set in | 12 |\n| two | `code` in a cell | 3 |\n| three | short | Omega |\n"
        );
        // The heads are eyebrows, in capitals.
        let (pitch, lines) = baselines("table", &markdown, "ALPHA", "Omega");
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
