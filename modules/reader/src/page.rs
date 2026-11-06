//! The page: the markdown set once in Typst as one tall page, its text one
//! measure of about 66 characters with room either side for a wide table to
//! spread into, on one pitch and one type scale. The window draws it
//! centred, so its margins are what the window leaves, and a window too
//! narrow for that has the page set again to fit it.

use std::path::PathBuf;
use std::time::Instant;

use typst_layout::{Page, PagedDocument};

use crate::walk::{self, Costs, Pieces};
use crate::world::PageWorld;

/// The measure: 580 px, about 66 characters of the text face, the most a
/// line should hold.
const MEASURE_PT: f64 = 435.0;

/// The least a measure holds: 45 characters of the text face, below which
/// lines break too often to read.
const LEAST_MEASURE_PT: f64 = 300.0;

/// Space's xxl, the least margin a window leaves either side of the words,
/// and its xl, which a window too narrow for both xxl and 45 characters
/// leaves instead.
const XXL_PT: f64 = 24.0;
const XL_PT: f64 = 18.0;

/// However narrow the window, a line still holds a few words, so Typst
/// always has a measure to set.
const NARROWEST_PT: f64 = 72.0;

/// How far a wide table or code may spread past the measure on each side,
/// which the page keeps as its side margins: its outer column, the glyph
/// column (theme.rs's `Space`), so everything that hangs out of the words,
/// a list's numbers among them, shares one edge.
fn spread_pt() -> f64 {
    crate::theme::get().space.outer * crate::theme::PT_A_PX
}

/// The narrowest window the whole page fits, its spread and xxl inside
/// each edge.
fn whole_from_pt() -> f64 {
    MEASURE_PT + 2.0 * (spread_pt() + XXL_PT)
}

/// How wide the page is set: its measure, and the room either side of it a
/// wide table may spread into, chosen for the window it shows in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Width {
    pub measure_pt: f64,
    pub spread_pt: f64,
}

impl Width {
    /// The whole page at a page without a theme's outer column, for what
    /// sets a page before the theme matters, such as the warm-up compile;
    /// a window takes `whole`.
    pub const WHOLE: Width = Width {
        measure_pt: MEASURE_PT,
        spread_pt: crate::theme::PLAIN_OUTER_PX * crate::theme::PT_A_PX,
    };

    /// The whole page, as a window wide enough for it shows it.
    pub fn whole() -> Width {
        Width {
            measure_pt: MEASURE_PT,
            spread_pt: spread_pt(),
        }
    }

    /// The page for a window this many points of the page wide at zoom 1.
    /// A narrower window than the whole page's has no room to spread a
    /// table into, and the measure gives way to keep xxl either side, down
    /// to 45 characters; then the margins give way to xl, and only then the
    /// measure again.
    pub fn for_window(window_pt: f64) -> Width {
        if window_pt >= whole_from_pt() {
            return Width::whole();
        }
        let measure_pt = if window_pt >= LEAST_MEASURE_PT + 2.0 * XL_PT {
            (window_pt - 2.0 * XXL_PT).clamp(LEAST_MEASURE_PT, MEASURE_PT)
        } else {
            (window_pt - 2.0 * XL_PT).max(NARROWEST_PT)
        };
        Width {
            measure_pt,
            spread_pt: 0.0,
        }
    }

    /// The page's width: its measure and the room on each side.
    pub fn page_pt(self) -> f64 {
        self.measure_pt + 2.0 * self.spread_pt
    }

    /// What Typst counts a hyphen as costing, in percent of its own: 200 on
    /// the whole measure, and dearer as the measure narrows, with the square
    /// of how much, since a narrow line has fewer spaces to take up the
    /// slack and Typst would otherwise hyphenate line after line.
    pub fn hyphen_cost(self) -> f64 {
        (200.0 * (MEASURE_PT / self.measure_pt).powi(2)).round()
    }
}

/// Above the first line's capitals: a line, as below the last baseline,
/// so the page has the same room at both ends. Lines start at their
/// capitals' height (Typst's top edge), so the margin is the gap.
fn top_pt() -> f64 {
    pitch_pt()
}

/// The page's line, from one baseline to the next, and what j and k move
/// by: the theme's, as home.nix bends it for a page (theme.rs's `Space`).
/// Every baseline lands on the page's beat, half of it.
pub fn pitch_pt() -> f64 {
    crate::theme::get().space.pitch * crate::theme::PT_A_PX
}

/// The page's look, after its measure, margins, size and line (template).
///
/// Every run of text is set at one of the theme's type levels, as the page
/// sets it (`level`, from the theme's file, and home.nix for which level
/// each is): the words at Words, a section in Title's cut at the section
/// step, the page's title at Display's step in the serif, code and figures
/// in Figure's mono, eyebrows at Label, the quieter words at Note. None
/// says a size or a weight of its own (type-is-set-by-level). Typst sets a
/// face's optical size to the text's by itself, and every other axis from
/// `variations`, the later winning, so a level names its optical size too.
/// A line is a capital's height and the leading below it, so the
/// leading is the pitch less the capitals, and a gap between paragraphs is
/// said as baseline to baseline (`gap`).
const PAGE: &str = r#"#import "/mitex/specs/mod.typ": mitex-scope
#let gap(n) = n * pitch - cap
#set text(..level.words.text, lang: "en", hyphenate: true, costs: (hyphenation: hyphen-cost, runt: 300%), top-edge: "cap-height", bottom-edge: "baseline")
#set par(justify: false, linebreaks: "optimized", leading: pitch - cap, spacing: gap(1.5))
// A word that holds a hyphen of its own breaks only there, as a typesetter
// would, never at one Typst adds, which would read as a third word.
#show regex("[\\p{L}\\p{N}]+(?:-[\\p{L}\\p{N}]+)+"): set text(hyphenate: false)
// Strong is the words at their strong span's weight, in the strong text:
// Typst's own adds 300, which draws this face black. Words whose level
// gives no strong weight keep Typst's.
#set strong(delta: level.words.strong - level.words.text.weight) if level.words.strong != none
#show strong: set text(fill: ink.strong)
// Code at its level, in the colour of what's around it and with no box.
// Typst's raw takes 0.8 em of its own, so the size is said whole.
#show raw: it => context text(..level.code.text, fill: text.fill, it)
// Headings, set by lines: more room above a heading than below it, so it
// belongs to what follows. A section sits a line over its first block, as
// a group's heading sits a pitch above its first thing, nearer it than one
// paragraph is to the next. A gap is baseline to baseline, rounded to the
// half line so every baseline stays on the grid. A block's top edge is its
// capitals (`caps` of an em in the words' face, the same at both sets of
// axes, and `code-caps` in the mono), so a gap takes the cap height of what
// follows off. A heavier or italic cut of a face that has cuts may stand
// its capitals a little higher or lower, so headings and the lede say
// their top edge as the regular's capitals, keeping every baseline where
// the gaps put it.
#let lines(n, c) = calc.round((n * pitch - cap + c) / beat) * beat - c
#let title-size = level.title.text.size
#let section-size = level.section.text.size
#let label-size = level.label.text.size
// An eyebrow: a label, as every eyebrow on the desktop is.
#let eyebrow(body) = at(level.label, body)
#show heading: set text(hyphenate: false)
// A title's lines two and a half lines apart, a section's one.
#show heading.where(level: 1): set text(..level.title.text, top-edge: caps * title-size)
#show heading.where(level: 1): set par(leading: 2.5 * pitch - caps * title-size)
#show heading.where(level: 1): set block(above: lines(3, caps * title-size), below: 1.5 * pitch - cap)
#show heading.where(level: 2): set text(..level.section.text, top-edge: caps * section-size)
#show heading.where(level: 2): set par(leading: pitch - caps * section-size)
#show heading.where(level: 2): set block(above: lines(3, caps * section-size), below: pitch - cap)
#show heading.where(level: 3): set text(..level.part.text, top-edge: cap)
#show heading.where(level: 3): set block(above: lines(2.5, cap), below: pitch - cap)
#show heading.where(level: 4): it => block(above: lines(2, code-caps * label-size), below: pitch - cap, eyebrow(it.body))
#show heading.where(level: 5): it => block(above: lines(2, code-caps * label-size), below: pitch - cap, eyebrow(it.body))
#show heading.where(level: 6): it => block(above: lines(2, code-caps * label-size), below: pitch - cap, eyebrow(it.body))
// A title of more than one line has its lines balanced: set at the
// narrowest width that keeps the lines it needs at the full measure, found
// in a few halvings, so no word is left alone on its last line.
#show heading.where(level: 1): it => layout(size => {
  let tall(width) = measure(block(width: width, {
    set par(leading: 2.5 * pitch - caps * title-size)
    text(..level.title.text, hyphenate: false, top-edge: caps * title-size, it.body)
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
// What the file is and when it changed, two lines over the title's
// baseline (four of the page's beats), or over the words when there's no
// title.
#show <title>: set block(above: 0pt)
#let about(titled: false, body) = block(above: 0pt, below: if titled { 4 * beat - caps * title-size } else { 1.5 * pitch - cap }, eyebrow(body))
// The paragraph after the title, in the voice, two lines over the words:
// the head widens outward, eyebrow to title, title to lede, lede to the
// words, and the last is wider than one paragraph is from the next, so
// the head reads as a group of its own.
#let lede(body) = block(above: 0pt, below: 2 * pitch - cap, par(text(..level.voice.text, top-edge: cap, body)))
#show math.equation: set text(font: "New Computer Modern Math")
#let mx(s, b) = math.equation(block: b, eval(s, mode: "math", scope: mitex-scope))
#let broken(s) = text(fill: ink.muted, raw(s))
// The page's thresholds carry its whimsy, never its reading. The ground
// glows faintly at the head: the background lifted 18% toward raised at
// most, so muted keeps Lc 60 on its lightest. It's an ellipse wider than
// it is tall, centred on the page's top edge: as wide as the page, which a
// measure and a half would pass, and gone 360 pt down, by the first
// section. A gradient relative to its rect is as wide and as tall as the
// rect, and its stop halfway eases it out, so it shows no edge.
#let glow = color.mix((ink.background, 82%), (ink.raised, 18%), space: rgb)
#let glow-half = color.mix((glow, 35%), (ink.background, 65%), space: rgb)
#set page(background: place(top + center, rect(width: 100%, height: 720pt, fill: gradient.radial((glow, 0%), (glow-half, 50%), (ink.background, 100%), center: (50%, 0%), radius: 50%, relative: "self"))))
// A thematic break is a fleuron: an asterism of the title's cut's
// asterisks in the accent, centred, standing as a line two lines from the
// words either side.
#let star = text(..level.ornament.text, top-edge: "bounds", bottom-edge: "bounds", "*")
#let fleuron = context {
  let s = measure(star)
  // Its ink stands evenly between the words either side: from the
  // baseline above, and to the capitals below.
  box(width: 2.3 * s.width, height: cap, {
    place(top + center, dy: -0.49 * s.height, star)
    place(bottom + left, dy: 0.66 * s.height, star)
    place(bottom + right, dy: 0.66 * s.height, star)
  })
}
#let rule() = block(above: 2 * pitch - cap, below: 2 * pitch - cap, width: 100%, align(center, fleuron))
// The page ends with a small accent lozenge after its last word.
#let the-end = box(polygon(fill: ink.accent, (0pt, 0.21em), (0.15em, 0pt), (0.3em, 0.21em), (0.15em, 0.42em)))
// A key, as code is set, framed: the frame stands outside the line's room,
// over the capitals and under the descenders, so a line with one still
// steps one line.
#let kbd(body) = box(stroke: 0.5pt + ink.rule, inset: (x: 3pt), outset: (top: 2pt, bottom: 0.3em), radius: 2pt, text(..level.code.text, body))
// Lists hang their marks in the margin, so the words keep the measure's
// edge: an accent bullet, a muted number, a task's box. A list in a list
// starts at its item's words.
//
// Things of a kind sit a line apart, and a beat further where any can take
// two lines, so a wrapped line is nearer its own item than the next item
// is, and each level stays wider than the one inside it. A list whose
// items all fit the measure steps a line from item to item and stands one
// and a half from the words round it. One where any item wraps sets its
// items one and a half apart, as paragraphs are, and stands two off, so it
// still reads as one group among them. The measure decides, so a list
// widens as the window narrows and its items come to wrap. A list stands
// off by the paragraphs' own rule, par spacing on a block left to it,
// which wins over a paragraph's by being larger but gives way to a set
// room such as a heading's, so a section sits a line over a list as over
// a paragraph.
#let hang = 1em
#set list(marker: box(width: 0.6em, align(right, text(fill: ink.accent)[•])), indent: -hang, body-indent: hang - 0.6em, spacing: pitch - cap)
#set enum(numbering: n => box(width: 1.6em, align(right, text(fill: ink.muted, number-width: "tabular")[#n.])), indent: -2 * hang, body-indent: 2 * hang - 1.6em, spacing: pitch - cap)
#let wraps(it) = it.children.any(item => measure(item.body, width: measure-width).height > measure(item.body).height)
#let listed(it) = {
  let inner = { set list(indent: 0pt); set enum(indent: 0pt); it }
  if it.indent >= 0pt { return inner }
  context {
    let wide = wraps(it)
    let items = if wide { gap(1.5) } else { pitch - cap }
    // The list is set again with its items' spacing, which comes back
    // here already holding it.
    if it.spacing != items {
      let fields = it.fields()
      let children = fields.remove("children")
      fields.insert("spacing", items)
      return it.func()(..fields, ..children)
    }
    // Typst ranks the room between two blocks by how weak it is before
    // how large: a weak v first, then a block's set above or below, then
    // auto and par spacing, the larger winning only between equals. Set
    // above and below would beat a heading's room, so the list leaves
    // them to the paragraphs' spacing, which a heading's set below wins
    // over and a paragraph's gives way to by being smaller.
    let off = if wide { gap(2) } else { gap(1.5) }
    set par(spacing: off)
    block(above: auto, below: auto, { set par(spacing: gap(1.5)); inner })
  }
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
// A quote in the voice, its words at the measure's edge and an
// accent rule hung 12 pt left of them, two lines from what's around it.
#let hung-rule(colour, first: cap, body) = block(above: lines(2, first), below: 2 * pitch - cap, stroke: (left: 1.5pt + colour), outset: (left: 12pt + 0.75pt), body)
#show quote.where(block: true): it => hung-rule(ink.accent, text(..level.voice.text, it.body))
// An alert is a quote's shape with its rule in its kind's hue and the kind
// as an eyebrow, one line over its words.
#let alert-hues = (note: ink.voice, tip: ink.accent, important: ink.program, warning: ink.action, caution: ink.machine)
#let alert(kind, body) = hung-rule(alert-hues.at(kind, default: ink.accent), first: code-caps * label-size, {
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
        // A table's heads are eyebrows, on the body's baseline, over
        // figures as over words; figures are the mono's, as everywhere.
        if head != none and k == 0 { eyebrow(it) } else if numeric.at(i, default: false) { text(..level.figure.text, it) } else { it }
      })).flatten()
    )
  }
  let records = {
    let names = if head == none { () } else { head }
    // One edge for every record's values, past the widest field's name,
    // each a note beside its value.
    let field(name) = at(level.note, name)
    let label = calc.max(0pt, ..names.map(name => measure(field(name)).width))
    let lead = pitch - cap
    set text(hyphenate: false, top-edge: cap, bottom-edge: "baseline")
    stack(..rows.enumerate().map(((k, row)) => block(
      width: 100%,
      inset: (y: half),
      stroke: if k > 0 { (top: 0.75pt + rule-ink) } else { none },
      stack(spacing: lead,
        strong(cell(row, 0)),
        grid(columns: (label, 1fr), column-gutter: 12pt, row-gutter: lead,
          ..range(1, n).filter(i => cell(row, i) != []).map(i => (field(cell(names, i)), cell(row, i))).flatten()))
    )))
  }
  let inside = fit(size.width)
  if inside != none { grid-of(inside) } else {
    let spread-out = fit(size.width + 2 * spread)
    if spread-out != none { pad(left: -spread, right: -spread, grid-of(spread-out)) } else { records }
  }
}))
// A code block: the listing's mono on a 15 pt line, its words on the
// column the paragraphs start at, as a quote's are. Its sunken ground
// hugs them as any chosen ground does, a mark out on every side, and
// hangs into the margin by that mark. Its height rounds up to the half
// line, shared above and below its lines, so the words after it stay on
// the page's grid. Its top stands one and a half lines under the baseline
// before it, and the next baseline two lines under its foot, so the room
// either side looks even. Code wider than the measure widens the block
// evenly, its ground no further out than the page's outer column. The
// language is an eyebrow at the top right, on the first line's baseline,
// or on a line of its own when the first line would come within the items
// rung of it.
#let code-size = level.listing.text.size
#let code-line = 15pt
#let code-inset = rung.mark
// The mono's set rules sit outside `layout`, since `measure` in its
// closure takes the styles `layout` was called with.
#let codeblock(lang: "", first: 0, longest: 0, body) = block(above: 1.5 * pitch, below: 2 * pitch - cap, width: 100%, {
 set text(..level.listing.text, top-edge: "cap-height", bottom-edge: "baseline", hyphenate: false)
 set par(leading: code-line - code-caps * code-size, justify: false, spacing: code-line - code-caps * code-size)
 layout(size => {
  let advance = measure([0]).width
  let inner = calc.min(calc.max(size.width, longest * advance), size.width + 2 * calc.max(0pt, spread - code-inset))
  let width = inner + 2 * code-inset
  let label = if lang == "" { none } else { eyebrow(lang) }
  let alone = label != none and first * advance + measure(label).width + rung.items > inner
  let lines = {
    if alone { block(width: 100%, below: code-line - code-caps * code-size, align(right, label)) }
    body
  }
  let h = measure(block(width: inner, lines)).height
  let tall = calc.ceil((h + 2 * code-inset) / beat) * beat
  let shown = block(fill: ink.sunken, radius: 4pt, width: width, inset: (x: code-inset, y: (tall - h) / 2), {
    if label != none and not alone { place(top + right, dy: code-caps * (code-size - label-size), label) }
    lines
  })
  let out = (width - size.width) / 2
  pad(left: -out, right: -out, shown)
 })
})
// A picture that can't be shown says what it shows, as a note never
// broken, as a caption would.
#let picture(alt) = text(..level.note.text, hyphenate: false, alt)
// A picture standing alone is centred, as wide as it is up to the measure
// and past it as far as a table spreads. Its height rounds up to the half
// line so the words after it stay on the grid, and its alt text is its
// caption, a note centred a line under it. It sits at
// the foot of its rounded height, so a line of words in its place, as a
// picture that can't be shown is, stands on a line too.
#let alone(body, caption) = block(above: 1.5 * pitch, below: 2 * pitch - cap, width: 100%, pad(left: -spread, right: -spread, layout(size => {
  let shown = align(center, body)
  let h = measure(block(width: size.width, shown)).height
  block(width: 100%, height: calc.ceil(h / beat) * beat, below: 0pt, align(bottom, shown))
  if caption != [] {
    block(above: pitch - cap, below: 0pt, width: 100%, pad(x: spread, align(center, text(..level.note.text, hyphenate: false, caption))))
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

/// What every page is set with: the theme's ink, then the page at `width`,
/// then its look, its words' capitals read from the face `world` finds for
/// them. All but the width is the same for every page, so it is made once.
pub fn template(world: &PageWorld, width: Width) -> String {
    static LOOK: std::sync::OnceLock<(String, String)> = std::sync::OnceLock::new();
    let (ink, look) = LOOK.get_or_init(|| look(crate::theme::get(), world));
    at_width(ink, look, width)
}

/// A theme's ink, and the page's look in it, its words' capitals read from
/// the face `world` finds for them.
fn look(theme: &crate::theme::Theme, world: &PageWorld) -> (String, String) {
    let words = &theme.levels.words;
    let size = words.size;
    let cap = world.capitals_pt(&words.family, size);
    // Each face's capitals in ems, for the gaps over a heading and an
    // eyebrow, so the page holds its line in whatever faces it finds.
    let caps = cap / size;
    let code_caps = world.capitals_pt(&theme.levels.code.family, 1.0);
    (
        theme.typst(),
        format!("#let cap = {cap}pt\n#let caps = {caps}\n#let code-caps = {code_caps}\n{PAGE}"),
    )
}

/// The page at `width`, in `ink` and `look`.
fn at_width(ink: &str, look: &str, width: Width) -> String {
    let Width {
        measure_pt,
        spread_pt,
    } = width;
    format!(
        "{ink}#set page(width: {page}pt, height: auto, margin: (x: {spread_pt}pt, top: {top}pt, bottom: {bottom}pt), fill: ink.background)\n#let measure-width = {measure_pt}pt\n#let spread = {spread_pt}pt\n#let hyphen-cost = {hyphen}%\n{look}",
        page = width.page_pt(),
        top = top_pt(),
        bottom = bottom_pt(),
        hyphen = width.hyphen_cost(),
    )
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

/// The bottom margin the page ends on: a line under the last baseline,
/// where Typst ends a line, as above the first capitals, so the page has
/// the same room at both ends and its descenders hang into it. A page set
/// from the file's first blocks lays out as the whole file's does down to
/// here.
pub fn bottom_pt() -> f64 {
    pitch_pt()
}

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
/// screens past it. The rest is set at the same `width`.
pub fn set_first(file: &str, reach: Option<usize>, width: Width) -> Result<First, String> {
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
        walk::program(&markdown, &about, base, &pieces, &world, width, &mut costs)
    });
    let (head, blocks) = walk::blocks(&program, &world, width);
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
    width: Width,
}

impl Setter {
    pub fn new(file: &str) -> Self {
        Setter {
            file: file.to_string(),
            world: new_world(),
            pieces: Pieces::new(),
            width: Width::whole(),
        }
    }

    /// Sets the page at another width from now on, for a window resized.
    pub fn fit(&mut self, width: Width) {
        self.width = width;
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
            self.width,
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
    use super::{Setter, Width, lives, when};

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

    /// The measure and spread a window this many points wide sets.
    fn width(window_pt: f64) -> (f64, f64) {
        let Width {
            measure_pt,
            spread_pt,
        } = Width::for_window(window_pt);
        (measure_pt, spread_pt)
    }

    #[test]
    fn a_window_too_narrow_for_the_whole_page_gives_way_measure_then_margin() {
        // The whole page: the measure, its outer column and xxl either side.
        let spread = super::spread_pt();
        let whole = 435.0 + 2.0 * (spread + 24.0);
        assert_eq!(width(1080.0), (435.0, spread), "the whole page");
        assert_eq!(width(whole), (435.0, spread), "the whole page, just");
        assert_eq!(
            width(whole - 1.0),
            (435.0, 0.0),
            "no spread, the whole measure"
        );
        assert_eq!(
            width(483.0),
            (435.0, 0.0),
            "the whole measure, xxl either side"
        );
        assert_eq!(
            width(360.0),
            (312.0, 0.0),
            "a third of the panel: xxl either side"
        );
        assert_eq!(width(348.0), (300.0, 0.0), "45 characters, xxl either side");
        assert_eq!(
            width(340.0),
            (300.0, 0.0),
            "45 characters, the margins giving way"
        );
        assert_eq!(width(336.0), (300.0, 0.0), "45 characters, xl either side");
        assert_eq!(
            width(300.0),
            (264.0, 0.0),
            "under 45 characters, xl either side"
        );
    }

    #[test]
    fn a_page_set_for_a_narrow_window_is_its_measure_wide_and_keeps_to_it() {
        let dir = std::env::temp_dir().join(format!("reader-narrow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, OPENING).expect("the file");
        let mut setter = Setter::new(file.to_str().expect("a path"));
        setter.fit(Width::for_window(360.0));
        let (page, _) = setter.set().expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        assert!(
            (page.frame.size().x.to_pt() - 312.0).abs() < 0.01,
            "the page is {} pt wide, not its measure",
            page.frame.size().x.to_pt()
        );
        let runs = crate::text::runs(&page.frame);
        let right = runs
            .iter()
            .flat_map(|run| run.chars.iter().map(|c| c.2))
            .fold(0.0, f64::max);
        assert!(
            right <= 312.01,
            "a line runs to {right} pt, past the measure"
        );
        let mut lines: Vec<f64> = runs.iter().map(|run| run.baseline_pt).collect();
        lines.sort_by(f64::total_cmp);
        lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        assert!(
            lines.len() > 2,
            "the words never wrapped on the narrow measure"
        );
    }

    /// How far across the page each shape it draws reaches, from its left
    /// edge to its right, stepping into every group it's set in.
    fn shapes_across(frame: &typst::layout::Frame, left: f64, out: &mut Vec<(f64, f64)>) {
        for (pos, item) in frame.items() {
            let x = left + pos.x.to_pt();
            match item {
                typst::layout::FrameItem::Group(group) => {
                    shapes_across(&group.frame, x, out);
                }
                typst::layout::FrameItem::Shape(shape, _) => {
                    let bounds = shape.bbox(true);
                    out.push((x + bounds.min.x.to_pt(), x + bounds.max.x.to_pt()));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn a_narrow_pages_code_keeps_its_ground_on_the_page() {
        // A narrow window's page is its measure alone, with no margin for a
        // block's ground to hang into, so a short block's ground keeps to
        // the measure rather than being cut flush with its words.
        let dir = std::env::temp_dir().join(format!("reader-narrow-code-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, format!("{OPENING}```\nlet code = here;\n```\n")).expect("the file");
        let mut setter = Setter::new(file.to_str().expect("a path"));
        setter.fit(Width::for_window(360.0));
        let (page, _) = setter.set().expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let wide = page.frame.size().x.to_pt();
        let mut shapes = Vec::new();
        shapes_across(&page.frame, 0.0, &mut shapes);
        assert!(!shapes.is_empty(), "the code block drew no ground");
        for (left, right) in shapes {
            assert!(
                left >= -0.01 && right <= wide + 0.01,
                "a ground runs from {left} pt to {right} pt, off the {wide} pt page"
            );
        }
    }

    #[test]
    fn a_narrow_page_hyphenates_as_a_typesetter_would() {
        // Every narrow window from 180 to 360 pt, so the rules hold at any
        // measure rather than at one that happens to break well.
        let markdown = "When you use two or more headings, GitHub automatically generates a table of contents that you can access by clicking the \"Outline\" menu icon {% octicon \"list-unordered\" aria-label=\"Table of Contents\" %} within the file header. Each heading title is listed in the table of contents and you can click a title to navigate to the selected section.\n";
        let dir = std::env::temp_dir().join(format!("reader-hyphens-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let mut setter = Setter::new(file.to_str().expect("a path"));
        let mut wrong = Vec::new();
        for window in (180..=360).step_by(6) {
            setter.fit(Width::for_window(f64::from(window)));
            let (page, _) = setter.set().expect("set");
            let lines: Vec<String> = crate::text::lines(crate::text::runs(&page.frame))
                .into_iter()
                .map(|line| line.text)
                .collect();
            let broken: Vec<bool> = lines.iter().map(|line| line.ends_with('\u{ad}')).collect();
            if broken.windows(3).any(|three| three.iter().all(|b| *b)) {
                wrong.push(format!("{window} pt: three hyphens in a row in {lines:?}"));
            }
            for (line, next) in lines.iter().zip(lines.iter().skip(1)) {
                let head = line.rsplit(' ').next().unwrap_or_default();
                let tail = next.split(' ').next().unwrap_or_default();
                if line.ends_with('\u{ad}') && (head.contains('-') || tail.contains('-')) {
                    wrong.push(format!(
                        "{window} pt: {head:?} {tail:?} broken off its hyphen"
                    ));
                }
            }
        }
        std::fs::remove_dir_all(&dir).expect("cleaned");
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// Every baseline from the first run holding `from` to the last holding
    /// `to`, top to bottom, and the page's pitch from its first paragraph,
    /// which is the page's line.
    /// The page a file holding `markdown` sets, as the window's is.
    fn page_of(name: &str, markdown: &str) -> super::Page {
        let dir = std::env::temp_dir().join(format!("reader-pitch-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, markdown).expect("the file");
        let (page, _) = Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        page
    }

    fn baselines(name: &str, markdown: &str, from: &str, to: &str) -> (f64, Vec<f64>) {
        let page = page_of(name, markdown);
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
            (pitch - super::pitch_pt()).abs() < 0.01,
            "the words step {pitch} pt, not the page's line of {} pt",
            super::pitch_pt()
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
    fn a_page_ends_a_line_under_its_last_baseline() {
        let page = page_of("foot", &format!("{OPENING}Omega.\n"));
        let last = crate::text::runs(&page.frame)
            .iter()
            .map(|run| run.baseline_pt)
            .fold(f64::NEG_INFINITY, f64::max);
        let foot = page.frame.height().to_pt() - last;
        assert!(
            (foot - super::pitch_pt()).abs() < 0.01,
            "the page ends {foot} pt under its last baseline, not a line of {} pt",
            super::pitch_pt()
        );
    }

    #[test]
    fn a_pages_first_capitals_stand_a_line_under_its_top() {
        let program = super::at_width("", "", super::Width::whole());
        let line = super::pitch_pt();
        assert!(
            program.contains(&format!("top: {line}pt, bottom: {line}pt")),
            "the eyebrow's capitals a line under the top and the last baseline a line over \
             the foot: {program}"
        );
    }

    #[test]
    fn code_sets_its_words_on_the_column_the_paragraphs_start_at() {
        let markdown = format!("{OPENING}```\nlet code = here;\n```\n");
        let page = page_of("code", &markdown);
        let runs = crate::text::runs(&page.frame);
        let start = |word: &str| {
            runs.iter()
                .filter(|run| {
                    run.chars
                        .iter()
                        .map(|c| c.0)
                        .collect::<String>()
                        .contains(word)
                })
                .map(|run| run.x_pt)
                .fold(f64::INFINITY, f64::min)
        };
        let (words, code) = (start("opening"), start("let"));
        assert!(
            (words - code).abs() < 0.01,
            "code's words start at {code} pt, off the column at {words} pt"
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
    fn a_theme_whose_words_give_no_strong_weight_still_sets_a_strong_span() {
        let theme = crate::theme::Theme::read(
            &serde_json::json!({"levels": {"words": {"strong": null}}}).to_string(),
        );
        let mut world = super::new_world();
        let (ink, look) = super::look(&theme, &world);
        let program = format!(
            "{}Words with #strong[a strong span] in them.",
            super::at_width(&ink, &look, super::Width::whole())
        );
        let page = super::set_page(&mut world, &program).expect("the page sets");
        let words: String = crate::text::runs(&page.frame)
            .iter()
            .flat_map(|run| run.chars.iter().map(|c| c.0))
            .collect();
        assert!(words.contains("strong"), "the page says its words: {words}");
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
        // The head widens outward: eyebrow to title, title to lede, lede
        // to the words, each wider than the last, the lede's widest of
        // all, past the paragraphs' one and a half.
        assert_eq!(
            steps[..3],
            [200, 150, 200],
            "the title two lines under the eyebrow, the lede one and a half under the title, \
             the words two under the lede: {steps:?}"
        );
        // A heading sits a line over its first block, nearer it than one
        // paragraph is to the next, and further from what's before it.
        assert_eq!(
            steps[steps.len() - 6..],
            [300, 100, 250, 100, 200, 100],
            "a section three lines under the words and one over its own, a part two and a \
             half and one, a fourth two and one: {steps:?}"
        );
    }

    #[test]
    fn a_list_steps_its_items_a_line_apart_or_a_beat_further_where_one_wraps() {
        let markdown = format!(
            "{OPENING}- One\n- Two\n- Three\n\nA paragraph between.\n\n- A first item long enough that it wraps onto a second line at the measure, as a little paragraph would\n- A second\n\nOmega.\n"
        );
        let (pitch, lines) = baselines("lists", &markdown, "opening", "Omega");
        let steps = steps(pitch, &lines);
        // Each level stays wider than the one inside it: a tight list's
        // items a line apart and the list one and a half from the words
        // round it; a wrapped list's items one and a half apart, as
        // paragraphs are, and the list two off. The opening's own lines
        // come first, as many as its face wraps it to.
        assert_eq!(
            steps[steps.len() - 8..],
            [150, 100, 100, 150, 200, 100, 150, 200],
            "a tight list a line a step and one and a half off, a wrapped list one and a \
             half a step and two off: {steps:?}"
        );
    }

    #[test]
    fn a_sections_heading_sits_a_line_over_a_list_however_its_items_wrap() {
        let markdown = format!(
            "{OPENING}## Tight\n\n- One\n- Two\n\n## Wrapped\n\n- A first item long enough that it wraps onto a second line at the measure, as a little paragraph would\n- Omega\n"
        );
        let (pitch, lines) = baselines("sectioned-lists", &markdown, "opening", "Omega");
        let steps = steps(pitch, &lines);
        // A group's heading sits a line over its first thing, a list's
        // first item as a paragraph, the list's own room giving way to it.
        assert_eq!(
            steps[steps.len() - 7..],
            [300, 100, 100, 300, 100, 100, 150],
            "each section a line over its list: {steps:?}"
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
