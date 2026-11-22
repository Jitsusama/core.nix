//! The page: the markdown set once in Typst as one tall page, its text one
//! measure of about 66 characters, up to 80 in a wide window, with room
//! either side for a wide table to spread into, on one pitch and one type
//! scale. The window draws it centred, so its margins are what the window
//! leaves, and a window of another width has the page set again to fit it.
//!
//! Long because the Typst template the page is set by is held here whole,
//! half the file, as one program whose parts lean on each other's
//! measures, and the type check (`type-is-set-by-level`) reads it here.

use std::path::PathBuf;
use std::time::Instant;

use typst_layout::{Page, PagedDocument};

use crate::eyebrow::about;
use crate::numbers::ms_of_ns;
use crate::walk::{self, Costs, Pieces};
use crate::world::PageWorld;

/// The measure: 580 px, about 66 characters of the text face, the line a
/// page is set to in a window just wide enough for it.
const MEASURE_PT: f64 = 435.0;

/// The widest measure, about 80 characters of the text face: the most a
/// line should hold, past which the eye loses its way back to the next
/// line's start.
const WIDEST_MEASURE_PT: f64 = MEASURE_PT * 80.0 / 66.0;

/// How much of a window's width past the whole page's the measure takes,
/// the rest going to the margins, so a wider window reads wider without
/// the words running to its edges.
const WIDENS_BY: f64 = 0.5;

/// A character of the text face, as the measure counts them.
const CHARACTER_PT: f64 = MEASURE_PT / 66.0;

/// The measure widens three characters at a time, so a wide window being
/// dragged sets a handful of pages rather than one every few pixels.
const WIDENS_IN_CHARACTERS: f64 = 3.0;

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

    /// The page the first screen is set at: for the window's width when
    /// the reader last closed, `kept_pt`, which niri most likely opens it
    /// at again, or else for the one a window has until niri names it, so
    /// the first configure finds the page already set for it. A window
    /// niri opens at another width, with none kept, sets it once more.
    pub fn first(kept_pt: Option<f64>) -> Width {
        let unnamed_pt = f64::from(crate::app::UNNAMED_SIZE.0) * crate::theme::PT_A_PX;
        Width::for_window(kept_pt.unwrap_or(unnamed_pt))
    }

    /// The page for a window this many points of the page wide at zoom 1.
    /// A wider window than the whole page's widens the measure by half of
    /// what it has past it, three characters at a time, up to 80. A narrower one has no
    /// room to spread a table into, and the measure gives way to keep xxl
    /// either side, down to 45 characters; then the margins give way to
    /// xl, and only then the measure again.
    pub fn for_window(window_pt: f64) -> Width {
        if window_pt >= whole_from_pt() {
            let step_pt = WIDENS_IN_CHARACTERS * CHARACTER_PT;
            let steps = ((window_pt - whole_from_pt()) * WIDENS_BY / step_pt).floor();
            let measure_pt = MEASURE_PT + steps * step_pt;
            return Width {
                measure_pt: measure_pt.min(WIDEST_MEASURE_PT),
                spread_pt: spread_pt(),
            };
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
    /// the whole measure or a wider one, and dearer as the measure narrows,
    /// with the square of how much, since a narrow line has fewer spaces to
    /// take up the slack and Typst would otherwise hyphenate line after
    /// line. A wider line has spaces to spare, so it is no cheaper there.
    pub fn hyphen_cost(self) -> f64 {
        (200.0 * (MEASURE_PT / self.measure_pt.min(MEASURE_PT)).powi(2)).round()
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
// Where the capitals of the words around stand: a line's top edge, which
// every run on it keeps, so a line steps one pitch whatever face its runs
// are in. Read in a context outside what it's for, since inside a show
// rule on raw the face is raw's own.
#let line-top() = measure([H]).height
// Code at its level, in the colour of what's around it and with no box.
// Typst's raw takes 0.8 em of its own, so the size is said whole.
#show raw: it => context text(..level.code.text, fill: text.fill, it)
// Code in a line stands at the line's top edge rather than the mono's
// lower capitals, so a line that is all code steps as far as one of
// words.
//
// Code short enough to keep whole, under a third of the measure, never
// breaks. Longer code breaks only at its spaces and after a slash,
// never at a hyphen or inside a name, so a flag or a path's piece is
// read whole; and a piece under four characters stays with the one
// beside it, so no line ends or opens on a stray `-o`, `|` or `/`. A
// piece wider than two thirds of the measure is left to break where
// Typst can, so it never runs out of a nested list's narrower room,
// and so is code past a few thousand bytes, which isn't worth the
// splitting.
//
// Each piece is boxed alone and held to the one before it at the join,
// by a space that doesn't break or a word joiner after a slash, rather
// than glued into one string, so a scrap stays held even beside a
// piece too wide to box. A short piece holds to the one before it, or
// the first to the one after, and the one after a held piece stands
// free, so held pieces go in pairs and a span of short flags still
// breaks between them. A short path's opening, `~/` or `./`, holds to
// the rest of its path too, so no line ends on it; and a short last
// piece holds to the pair before it rather than standing alone. The
// page reads both joins back as what the code has (text.rs).
#let after-slash = regex("[^/]*/|[^/]+")
// A span's pieces: each with whether it's kept whole, what comes
// between it and the one before (a space, nothing after a slash, or
// none for the first), and whether it's held to that one.
#let code-pieces(s) = {
  if s.len() > 4000 { return ((piece: s, whole: false, by: none, held: false),) }
  if measure(raw(s)).width < measure-width / 3 { return ((piece: s, whole: true, by: none, held: false),) }
  let out = ()
  let short-last = false
  let held-last = false
  let opens-last = false
  for word in s.split(" ") {
    let parts = word.matches(after-slash).map(m => m.text)
    // A space beside another is a word of none, kept as it's written.
    if parts.len() == 0 { parts = ("",) }
    for (j, piece) in parts.enumerate() {
      let first = out.len() == 0
      let short = piece.clusters().len() < 4
      let held = not first and (opens-last or (not held-last and (short or short-last)))
      let by = if first { none } else if j == 0 { " " } else { "" }
      out.push((piece: piece, whole: measure(raw(piece)).width <= measure-width * 2 / 3, by: by, held: held))
      short-last = short
      held-last = held
      opens-last = short and j == 0 and parts.len() > 1
    }
  }
  if out.len() > 1 and out.last().piece.clusters().len() < 4 { out.at(-1).held = true }
  out
}
#let code(s) = context {
  let set-piece(p) = {
    let piece = if p.whole { box(raw(p.piece)) } else { raw(p.piece) }
    if p.by == none { piece } else if p.held and p.by == " " { raw("\u{a0}") + piece } else if p.held { sym.wj + piece } else if p.by == " " { raw(" ") + piece } else { piece }
  }
  text(top-edge: line-top(), code-pieces(s).map(set-piece).join())
}
// What an unboxed piece can't break inside: a run up to a space, or up
// to and taking a hyphen, after which a line may break, but for one
// opening a word, as in --release, or before a figure, as in -1, which
// Unicode's line breaking holds to what follows.
#let unbroken = regex("-*[^ -]*(?:-+[0-9][^ -]*)*-*")
// The least room a table's column needs for a span: its widest run of
// pieces held together, since only between those can it break. A piece
// too wide to box is unboxed, so it breaks where its words or a hyphen
// let it, and counts its widest stretch between those: a name with
// nowhere to break counts whole, as the cell can't break it either.
#let least-code(s) = context {
  let widest = 0pt
  let run = 0pt
  for p in code-pieces(s) {
    let by = if p.by == " " { measure(raw(" ")).width } else { 0pt }
    if p.whole {
      let width = measure(raw(p.piece)).width
      run = if p.held { run + by + width } else { width }
    } else {
      // Its first stretch joins what's held to it, and its last goes on
      // to the piece held after.
      let stretches = p.piece.matches(unbroken).filter(m => m.text != "").map(m => measure(raw(m.text)).width)
      if stretches.len() == 0 { stretches = (0pt,) }
      let first = if p.held { run + by + stretches.first() } else { stretches.first() }
      widest = calc.max(widest, first, ..stretches)
      run = if stretches.len() == 1 { first } else { stretches.last() }
    }
    widest = calc.max(widest, run)
  }
  box(width: widest)
}
// An emoji in the colour face, whichever face the words around it are
// set in, as the desktop draws them: a pictograph drawn as one, with
// its skin tone, its joined people and its keycap, or a flag's pair of
// letters. A pictograph that reads as text unless asked, an arrow or a
// trademark, stays in the words' face. Its top edge is the line's, so
// its taller box never pushes its line down.
#let emoji-face = "Noto Color Emoji"
#let dressed = "\\p{Emoji_Modifier}?\\x{FE0F}?[\\x{E0020}-\\x{E007F}]*"
#let pictograph = "(?:\\p{Emoji_Presentation}|\\p{Extended_Pictographic}\\x{FE0F})" + dressed
#let joined = "(?:\\x{200D}\\p{Extended_Pictographic}\\x{FE0F}?" + dressed + ")*"
#let emoji = regex("\\p{Regional_Indicator}{2}|[#*0-9]\\x{FE0F}?\\x{20E3}|" + pictograph + joined)
#show emoji: it => context text(font: emoji-face, top-edge: line-top(), bottom-edge: "baseline", it)
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
// The page's first paragraph of words opens on a drop cap, two lines
// tall, when it sets three lines or more at the measure, as a chapter
// opens; a shorter one, or one opening on anything but a capital, stands
// as any paragraph. The letter is the title's cut in the title's ink, its
// capitals reaching from the first line's to the second's baseline, with
// the first two lines beside it. Typst flows no words round a shape, so
// the paragraph is set in two: as many of its words as fill two lines
// beside the letter, then the rest at the whole measure, a line under.
// Its size is read from the title's own capitals, which an optical size
// or another face sets apart from the words'.
#let drop-size() = (pitch + cap) / (measure(text(..level.title.text, top-edge: "cap-height", bottom-edge: "baseline", "H")).height / title-size)
// A paragraph's pieces: each word and each space of its words alone, and
// whatever else it holds whole, so it can be cut between two words.
#let pieces-of(it) = if it.func() == [].func() {
  it.children.map(pieces-of).flatten()
} else if it.func() == text and it.fields().keys() == ("text",) {
  it.text.split(" ").map(word => (text(word),)).intersperse(([ ],)).flatten().filter(piece => piece != text(""))
} else { (it,) }
#let opening(body) = layout(size => {
  let plain = par(body)
  if measure(block(width: size.width, plain)).height < 2 * pitch + cap - 0.01pt { return plain }
  let pieces = pieces-of(body)
  let first = pieces.position(piece => piece.func() != metadata)
  let word = if first == none { none } else { pieces.at(first) }
  if word == none or word.func() != text or word.text.match(regex("^\\p{Lu}")) == none { return plain }
  let letter = word.text.clusters().first()
  let after = word.text.slice(letter.len())
  let size-of = drop-size()
  let big = text(..level.title.text, size: size-of, top-edge: "cap-height", bottom-edge: "baseline", letter)
  // A letter that is a word of its own stands a space from the next. The
  // letter's air is known to the reader's lines too (text.rs), which read
  // a word of one letter apart from the next by it.
  let alone = after == ""
  let indent = measure(big).width + 0.08 * size-of + if alone { 0.25em } else { 0pt }
  let pieces = pieces.slice(0, first) + if alone { () } else { (text(after),) } + pieces.slice(first + 1)
  let pieces = if alone and pieces.at(first, default: none) == [ ] { pieces.slice(0, first) + pieces.slice(first + 1) } else { pieces }
  let beside(k) = measure(block(width: size.width - indent, par(pieces.slice(0, k).join()))).height
  let fits(k) = beside(k) < pitch + cap + 0.01pt
  // Two lines hold far fewer pieces than this, so a long opening is
  // searched over its start alone, not laid out whole at every step.
  let (low, high) = (first, calc.min(pieces.len(), first + 400))
  while low < high {
    let middle = calc.ceil((low + high) / 2)
    if fits(middle) { low = middle } else { high = middle - 1 }
  }
  // Cut at a space between two words, never inside a run of strong or
  // code; past the last word, nothing is left for the whole measure.
  let cut = low
  while cut > first and pieces.at(cut, default: [ ]) != [ ] { cut -= 1 }
  // Beside the letter stand two lines, or the rest would run into it: a
  // word too long to share the second line leaves the letter alone.
  if cut <= first or beside(cut) < pitch { return plain }
  let rest = pieces.slice(calc.min(cut + 1, pieces.len())).join()
  block({
    set par(spacing: pitch - cap)
    place(top + left, big)
    pad(left: indent, par(pieces.slice(0, cut).join()))
    if rest != none { par(rest) }
  })
})
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
// The page ends with its mark, centred under its last block as a break
// stands between two: the accent lozenge, as tall as the capitals, between
// two hairlines in the rule's ink that thin away from it. It reads as the
// end at a glance and never as a break, which is stars.
#let the-end() = block(above: 2 * pitch - cap, below: 2 * pitch - cap, width: 100%, align(center, {
  let (diamond, hair, air, thick) = (0.72 * cap, 2.5em, 0.5em, 0.9pt)
  let wide = 2 * (hair + air) + diamond
  box(width: wide, height: cap, {
    place(left + horizon, polygon(fill: ink.rule, (0pt, 0pt), (hair, -thick / 2), (hair, thick / 2)))
    place(center + horizon, polygon(fill: ink.accent, (0pt, cap / 2), (diamond / 2, 0pt), (diamond, cap / 2), (diamond / 2, cap)))
    place(right + horizon, polygon(fill: ink.rule, (0pt, -thick / 2), (hair, 0pt), (0pt, thick / 2)))
  })
}))
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
// A table's first baseline stands two lines under the one before it,
// past its first row's half-line inset, so the page's grid runs through
// it. It ends on its last baseline, as a paragraph does, the last row's
// inset taken back, so what follows it, a heading as much as words,
// stands where its own room puts it from a baseline.
#let sheet-air = 2 * pitch - (1.5 * pitch + cap) / 2
#let sheet(numeric: (), words: (), head: none, rows: ()) = block(above: sheet-air, below: 2 * pitch - cap, layout(size => {
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
  v(-half)
}))
// A code block: the listing's mono on a 15 pt line, its words on the
// column the paragraphs start at, as a quote's are. Its sunken ground
// hugs them as any chosen ground does, a mark out on every side, and
// hangs into the margin by that mark. Its height rounds up to the half
// line, shared above and below its lines, so the words after it stay on
// the page's grid. Its top stands one and a half lines under the baseline
// before it, and the next baseline two lines under its foot, so the room
// either side looks even. Code wider than the measure widens the block
// evenly, its ground no further out than the page's outer column. A
// narrow window's page has no margin to hang into, so there the ground
// keeps to the measure and its words stand a mark inside it. The
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
  let inner = calc.min(calc.max(size.width, longest * advance), size.width + 2 * (spread - code-inset))
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
// A diagram is drawn as wide as the room it stands in, but no wider than
// the whole page's measure, so a wide window's longer lines never grow its
// labels past the words beside them, and one in a list keeps to the list.
#let diagram(path) = block(width: 100%, layout(room => image(path, width: calc.min(room.width, whole-measure))))
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
        "{ink}#set page(width: {page}pt, height: auto, margin: (x: {spread_pt}pt, top: {top}pt, bottom: {bottom}pt), fill: ink.background)\n#let measure-width = {measure_pt}pt\n#let whole-measure = {MEASURE_PT}pt\n#let spread = {spread_pt}pt\n#let hyphen-cost = {hyphen}%\n{look}",
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

/// The file set as far as its first screen, and what sets the rest.
pub struct First {
    pub page: Page,
    pub ms: f64,
    pub rest: Option<Rest>,
    /// Each part of the set, with its time and the time of it spent waiting
    /// for a processor, in milliseconds.
    pub parts: Vec<(&'static str, f64, f64)>,
    /// The window's title for the file.
    pub title: String,
}

/// Times a part of the set into `parts`.
fn timed<T>(
    parts: &mut Vec<(&'static str, f64, f64)>,
    name: &'static str,
    part: impl FnOnce() -> T,
) -> T {
    let start = Instant::now();
    let waited = crate::run_delay_ns();
    let out = part();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let waited = ms_of_ns(crate::run_delay_ns().saturating_sub(waited));
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
/// screens past it, and the block after it unless that is longer still. The rest is set at the same `width`.
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
    let title = crate::title::of(std::path::Path::new(file), &markdown);
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
        // At least the block after the one it opens at, whose top says how
        // tall that one stands, so a place kept in a long block lands as
        // far through it, unless that block is itself more than a first
        // screen, whose marks then wait for the whole page.
        let next_too = i == from
            && blocks
                .get(i + 1)
                .is_some_and(|next| next.len() <= FIRST_CHARS);
        if chars > FIRST_CHARS && !next_too {
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
        title,
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
    title: String,
}

impl Setter {
    pub fn new(file: &str) -> Self {
        Setter {
            file: file.to_string(),
            world: new_world(),
            pieces: Pieces::new(),
            width: Width::whole(),
            title: String::new(),
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

    /// The window's title for the file, as it was last set.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The whole file as it is now, as one tall page, and how long that took.
    pub fn set(&mut self) -> Result<(Page, f64), String> {
        let start = Instant::now();
        let markdown =
            std::fs::read_to_string(&self.file).map_err(|e| format!("{}: {e}", self.file))?;
        let path = std::path::Path::new(&self.file);
        self.title = crate::title::of(path, &markdown);
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
#[path = "page_tests.rs"]
mod tests;
