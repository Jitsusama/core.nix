# The Craft Behind the Rules

The measured facts about Joel's faces and colours in a browser, and the CSS and
markup that carry out each rule of the skill. The kit already does what every
site needs (`theme.css`, `base.css`); this is for what a site builds on it.
Values are named by the kit's properties, never as colours or pixel sizes a
theme would change.

## His Faces in a Browser

Measured from the font files themselves.

| Fact | Fraunces | Monaspace |
| --- | --- | --- |
| The zero's width (`1ch`) | 0.643 em | 0.620 em |
| Average letter of English text | 0.461 em | 0.620 em |
| Cap height | 0.700 em | 0.730 em |
| x-height | 0.482 em | 0.513 em |
| Numeric features | none: no `tnum`, `onum`, `lnum`, `smcp` | tabular by design |
| Axes | `opsz` 9 to 144, `wght` 100 to 900, `SOFT` 0 to 100, `WONK` 0 or 1 | `wght` 200 to 800 (the kit pins width and slant) |

What follows from them:

- **Measure in `em`, never `ch`.** In Fraunces, `1ch` holds about 1.39 letters,
  so `65ch` runs to about 90 characters a line. `30em` is about 66, `20em`
  about 45, `34.6em` about 75. Monaspace is exact: `1ch` is one character.
- **Figures that are data are Monaspace.** `font-variant-numeric:
  tabular-nums` on Fraunces does nothing at all; a column of Fraunces figures
  never lines up. Fraunces keeps figures in running prose.
- **WONK leans only at optical sizes of about 22 and up.** With
  `font-optical-sizing: auto`, headings drawn at 22 px or more lean; smaller
  ones stay upright whatever `WONK` says. A heading meant to lean is set big
  enough to.
- **Fraunces' axes go through custom properties.** `font-variation-settings`
  is one property, so an element that sets only `WONK` silently resets `SOFT`
  to its default. `base.css` sets both from `--soft` and `--wonk`, which any
  element may change:

```css
.ornate { --soft: 100; --wonk: 1; }   /* never font-variation-settings directly */
```

- **A label's tracking runs past its last letter**, which `base.css` takes
  back with a negative end margin, so a centred or right-aligned label sits
  true. Write labels in their real case and let `text-transform` set
  capitals, so a screen reader and a copy read words, not shouting.
- **Hanging punctuation is Safari's alone.** Elsewhere, a paragraph opening
  with a quotation mark hangs it by hand: `.opens-quote { text-indent:
  -0.4em }` (Fraunces' opening quote is 0.400 em wide).
- **Fraunces' and Monaspace's caps share a height** when the serif is drawn
  0.730 / 0.700 as large: `font-size-adjust: cap-height 0.73` on the serif
  beside a mono, as in a row of a name and a figure. `text-box: trim-both cap
  alphabetic` measures a band's padding from the ink, which is how a find box
  centres its text.
- **Matched stand-ins.** `theme.css` gives each face a local stand-in scaled
  to its metrics, so nothing moves when the real face arrives. Keep
  `font-display: swap`; `optional` would show the stand-in for good on a slow
  first visit.

## His Colours in a Browser

Contrast on the dark theme's ground, as WCAG ratios and APCA Lc, from the theme
files. The light theme's figures are in brackets.

| Role | Ratio | APCA Lc | Fit |
| --- | --- | --- | --- |
| `--text` | 11.7 (10.4) | 78.9 (92.8) | body text |
| `--strong` | 14.3 | 91.6 | headings, emphasis |
| `--muted` | 8.5 (5.7) | 60.8 (77.3) | notes and labels at 15 px and up, never a fact's only copy |
| `--accent` | 4.9 (6.4) | 36.9 (80.1) | the focus ring only, never text |
| `--rule` | 2.9 (2.0) | | hairlines, which carry nothing alone |
| `--edge` | 1.75 (1.55) | | fails 3:1, so never a field's edge |
| `--action` | 10.4 (5.6) | | the default tint |

- **APCA's floor for body text is Lc 75 at 18 px and weight 400** (Lc 90
  preferred), Lc 60 for other text at 24 px and 400 or 16 px and 700. That's
  why a page's body is 18 px or more and muted text never carries the
  argument.
- **A field's edge is `--field`** (`base.css`), a mix of muted and the ground
  that passes 3:1 on both themes, for WCAG 1.4.11.
- **The ring on a selected row:** the accent on the selection ground falls
  under 3:1, so draw the ring with an offset outside the row, or inset with
  the ground between.
- **A site's tint stays within reach of the theme.** `--turn` keeps each
  role's lightness, so contrast holds; check a large turn anyway, since chroma
  shifts with hue.

## Type Steps

The kit's steps are fluid between a 360 px and a 1280 px window, at ratio 1.2
narrow and the theme's 1.25 wide, in `rem` so zoom works.

| Step | A page, px | An interface, px |
| --- | --- | --- |
| `--step-small` | 15 to 16 | 12.5 to 12.8 |
| `--step-body` | 18 to 20 | 15 to 16 |
| `--step-large` | 21.6 to 25 | 18 to 20 |
| `--step-section` | 25.9 to 31.2 | 21.6 to 25 |
| `--step-title` | 31.1 to 39.1 | 25.9 to 31.2 |
| `--step-display` | 53.7 to 76.3 | 44.8 to 61 |
| `--step-hero` | 77.4 to 119.2 | 64.5 to 95.4 |

On a page to read, no prose is under 15 px on screen, `--step-small` at
its narrowest: a card's aside, a footnote, a table's head, a strip of
labels or a slide's number takes the step, not a fraction of it. Only
mono runs sit smaller, at 0.85 em, so code matches the serif beside it.
The probe holds this at every width.

Each step's largest is under 2.5 times its smallest, which keeps 200% zoom
growing text (WCAG 1.4.4). Leading is unitless, 1.5 to 1.6 for reading and
1.33 to 1.4 for rows, and nothing has a fixed height, so a reader's own
spacing (1.4.12) never clips.

## Measure, Grid and Space

One page grid with named lines, so "this lines up with that" is declared, not
coincidence:

```css
.page {
  display: grid;
  grid-template-columns:
    [full-start] minmax(var(--screen), 1fr)
    [wide-start] minmax(0, 6em)
    [content-start] min(var(--measure), 100%) [content-end]
    minmax(0, 6em) [wide-end]
    minmax(var(--screen), 1fr) [full-end];
  row-gap: var(--groups);
}
.page > * { grid-column: content; }
.page > :is(.wide, figure:has(table)) { grid-column: wide; }
.page > .bleed { grid-column: full; }
```

- **Values that line up down a list share columns through subgrid**, never
  fixed widths:

```css
.list { display: grid; grid-template-columns: [mark] auto [name] 1fr [figure] auto [when] auto; }
.list > .row { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; }
```

- **Columns come from the room**, set by a container query on the part's own
  width: one on a phone, two on a tablet or a laptop, three on a wide screen
  where that reads better, and never one under 256 px. A part changes shape
  by its own width, not the window's.
- **Things side by side are one grid row, ending level**: each spans the
  rows its parts need and takes them as a subgrid, so state, title, figure,
  line and fold each sit on one line across the row, and every box stretches
  to the tallest. Never padding or a fixed height to even them out:

```css
.steps { container-type: inline-size; }
.steps > ol { display: grid; gap: var(--groups) var(--items);
  grid-template-columns: 1fr; grid-auto-rows: auto; }
@container (min-width: 34em) { .steps > ol { grid-template-columns: repeat(2, 1fr); } }
@container (min-width: 56em) { .steps > ol { grid-template-columns: repeat(3, 1fr); } }
.steps li { display: grid; grid-row: span 4; grid-template-rows: subgrid;
  row-gap: var(--apart); } /* state, title, line, fold */
```
- **Space as hierarchy.** Gaps on the parent (`gap`, `row-gap`), each level's
  rung wider than the one inside it: `--tie` inside a mark, `--mark` between a
  mark and its words, `--apart` between parts of a row, `--items` between
  rows, `--groups` between groups, `--frames` between sections. A heading has
  more room above it than below, so it belongs to what follows. For vertical
  rhythm, `1lh` and `1rlh` are an empty line.
- **On a narrow screen a frame gives up its border and inset** and becomes the
  page (a container query under about 30em), rather than squeezing its text.
- **A drawer never moves the text.** Narrow, it slides over the page; wide, it
  docks into the margin the measure leaves, so the column neither moves nor
  rewraps. `scrollbar-gutter: stable` (in `base.css`) keeps a page from
  jumping sideways when it gains a scrollbar; an interface that opens drawers
  uses `stable both-edges`.
- **Sticky headers keep focus in sight** (WCAG 2.4.11):
  `html { scroll-padding-block-start: <the header's height> }`.
- **Centring on the ink, not the line box.** Flex and grid centre a line's box,
  which sits a little low or high against capitals. A mark beside words
  centres on their capitals, as the desktop's do: `vertical-align: calc(0.5cap -
  <mark height> / 2)` raises its foot so its middle meets the capitals' middle,
  or the mark is `1cap` tall on the baseline.

## Dark, Light and the Switch

Both looks are in `theme.css`. Give the page the scheme before the stylesheet
arrives, and the reader's choice before the first paint:

```html
<meta name="color-scheme" content="dark light">
<script>
  try {
    const t = localStorage.getItem("theme");
    if (t === "light" || t === "dark") document.documentElement.dataset.theme = t;
  } catch {}
</script>
```

The switch: three icon buttons in a `role="radiogroup"` labelled "Theme",
each a `role="radio"` with its accessible name ("Match System", "Light",
"Dark") and a visible name on hover and focus; arrow keys move between them,
and one tab stop reaches the group. An icon names nothing to a screen reader,
so every button carries its name, or the switch reads as three blank radios:

```html
<div class="switch" role="radiogroup" aria-label="Theme">
  <button type="button" role="radio" aria-checked="false" tabindex="-1"
          data-choice="light" aria-label="Light">
    <svg viewBox="0 0 24 24" aria-hidden="true">...</svg>
  </button>
  <button type="button" role="radio" aria-checked="true" tabindex="0"
          data-choice="system" aria-label="Match System">
    <svg viewBox="0 0 24 24" aria-hidden="true">...</svg>
  </button>
  <button type="button" role="radio" aria-checked="false" tabindex="-1"
          data-choice="dark" aria-label="Dark">
    <svg viewBox="0 0 24 24" aria-hidden="true">...</svg>
  </button>
</div>
```

Choosing "Match System" removes the stored
key and `data-theme`. The thumb slides between them on the theme's spring, and
the change may sweep in with a View Transition, both still when less motion is
asked for. The last one Joel kept: auto in the middle, the thumb on
`cubic-bezier(0.3, 1.4, 0.5, 1)`, applied before paint, a sweep on change:
"nope, this is good, thanks!" (2026-10-02).

## The Key Line and the Keys List

As omni-ref has them. The line sits in the head, under the parts it steps
between and above the switch, in Note and muted. It's drawn at every width
and on touch too, never hidden behind `(hover: none)`, since a tablet often
has a keyboard; and never in a footer, the length of the page away:

```html
<p class="keys"><kbd>j</kbd> and <kbd>k</kbd> step from part to part, and
  <kbd>?</kbd> lists every key</p>
```

? opens the list and ? closes it; Escape closes it too, since it holds no
work, and focus goes back where it was. A native `<dialog>`, opened with
`showModal()`, gives the focus and the Escape for free:

```html
<dialog class="keypanel" id="keys" aria-labelledby="keys-h">
  <h2 id="keys-h">Keys</h2>
  <dl>
    <div><dt><kbd>j</kbd></dt><dd>Next part</dd></div>
    <div><dt><kbd>k</kbd></dt><dd>Previous part</dd></div>
    <div><dt><kbd>g</kbd></dt><dd>First part</dd></div>
    <div><dt><kbd>G</kbd></dt><dd>Last part</dd></div>
    <div><dt><kbd>?</kbd></dt><dd>Show or hide this list; Escape closes it</dd></div>
  </dl>
</dialog>
```

A part is each numbered section the head lists; j and k scroll its heading
to the top and move focus to it, so a screen reader follows. Without script
the keys do nothing, so the line shows only once the head's script has set
`class="js"` on `<html>`, before the first paint, and never promises a key
that won't answer.

## Disclosure That Looks Like a Door

```html
<details class="branch">
  <summary>Why not keep both?</summary>
  <div>...</div>
</details>
```

```css
.branch > summary { list-style: none; display: grid; grid-template-columns: auto 1fr;
  gap: var(--apart); align-items: baseline; }
.branch > summary::before { content: ""; inline-size: 1.25em; block-size: 1.25em;
  border: 1px solid var(--tint); border-radius: 50%;
  background: /* a drawn plus, in --tint */; transition: rotate var(--arrive) var(--spring); }
.branch[open] > summary::before { rotate: 45deg; }  /* the plus becomes a cross */
```

- The summary is the reader's question, in their words, with no "open" or
  "fold". `base.css` grows the content open; nothing beside it moves.
- Several may be open at once; never an exclusive accordion that closes the
  one being read.
- `hidden="until-found"` on a long fold lets find-in-page open it.
- Whatever measures the page measures it with every fold open too.
- **Opening one grows only itself, downward**: nothing above moves, nothing
  beside resizes, and only what's below in its own column moves down. The
  probe opens each fold and measures every box before and after. A fold
  inside one of several things side by side would stretch the whole row and
  move its neighbours' parts, so it opens a side sheet instead.

## A Side Sheet

Evidence too long to open in place, or asked for from one of several things
side by side, opens over the page in a sheet, so the page under it never
moves:

```html
<button type="button" aria-haspopup="dialog" aria-controls="proof-3">The evidence</button>
<dialog id="proof-3" class="sheet" aria-labelledby="proof-3-title">
  <header><h2 id="proof-3-title">Suspend and resume</h2>
    <button type="button" class="close" aria-label="Close">...</button></header>
  ...
</dialog>
```

```css
.sheet { margin: 0 0 0 auto; block-size: 100dvh; max-block-size: none;
  inline-size: min(100vw, var(--measure) + 2 * var(--inset)); }
@media (max-width: 40em) { .sheet { inline-size: 100vw; } }
```

- **It opens with `showModal()`**, so focus goes in, the page under it is
  inert, and closing returns focus to the button that opened it.
- **Escape closes it**, as it closes the Keys list, since a sheet of
  evidence holds nothing to lose; so do its close button and :q. A sheet
  holding work (a form, a draft) cancels its `cancel` event and closes only
  by its own control or :q, as SKILL.md's Keys rule says.
- **From the right on a wide screen, the full screen on a phone**, sliding
  in on `--arrive` and still under reduced motion. The scrollbar's gutter is
  kept, so the page under it doesn't shift sideways.

## Motion

- **Arrival on a page to read, once:** gate it on a class the script sets only
  when motion is allowed, so a page without script, or asking for less, is
  whole and still.

```js
const quiet = matchMedia("(prefers-reduced-motion: reduce)").matches;
if (!quiet) {
  document.documentElement.classList.add("motion");
  const seen = new IntersectionObserver((entries) => {
    for (const e of entries) if (e.isIntersecting) {
      e.target.classList.add("arrived");
      seen.unobserve(e.target);
    }
  }, { rootMargin: "0px 0px -10% 0px" });
  document.querySelectorAll(".arrives").forEach((el) => seen.observe(el));
}
```

```css
.motion .arrives:not(.arrived) { opacity: 0; translate: 0 1rem; }
.motion .arrives { transition: opacity var(--large) var(--ease), translate var(--large) var(--ease); }
```

- **A scroll story** ties a drawing to the scroll with no script, which
  rewinds by construction: a tall section (200 to 300 vh) with a sticky stage
  and `animation-timeline: view()`, `animation-range: entry 0% cover 50%` (it
  ends while the stage is in plain view), `animation-fill-mode: both`, and a
  first frame that already shows the subject. Where `animation-timeline` isn't
  supported the static frame is the whole picture.
- **Never take the scroll**: never change its speed with script; lengthen the
  distance instead. Snap, if at all, `scroll-snap-type: y proximity`, never
  `mandatory`. Joel turned down snapping that jumped past the text.
- **State changes are transitions**, which reverse from wherever they are when
  interrupted; keyframes restart. Between two states of the page, a View
  Transition.
- **Animate only `transform`, `translate`, `opacity`, `clip-path` and
  `filter`.** Never `width`, `top` or `margin`, which lay the page out every
  frame, and never layout as a side effect.
- **Durations** are the theme's: `--leave` 100 ms, `--arrive` 150 ms, `--large`
  250 ms, on `--ease` or `--spring`. Feedback to a key is immediate, with no
  transition on the cursor at all.
- **Numbers change in place**, at once; at most their colour marks the change
  for a beat, and a live figure is announced politely, not on every tick.

## Keys

- **Roving focus in a list**: the list is one tab stop; j, k and the arrows
  move a real DOM focus (`tabindex="0"` on the current row, `-1` on the rest),
  so screen readers and the ring follow.
- **Single letters never fire in a text field**, and a setting switches them
  off:

```js
const typing = (e) => e.target.closest("input, textarea, select, [contenteditable]");
addEventListener("keydown", (e) => {
  if (typing(e) || e.altKey || e.ctrlKey || e.metaKey || keysOff()) return;
  // j, k, /, ? ...
});
```

- **: opens the palette** outside a text field, and **Ctrl+K or Cmd+K** opens
  it from anywhere, text fields included, with every command and its key. ?,
  Space ? and F1 open it at its list of keys.
- **Escape** closes the topmost popup the reader opened (the palette, a menu,
  a hover card, a question it answers "no" to) and returns focus to what
  opened it; with nothing open it does nothing. A native `<dialog>` closes on
  Escape by itself, so one holding work cancels its `cancel` event. A drawer
  closes with its own control or :q.
- **Keycaps follow the platform**, and chords are drawn as keys in Monaspace,
  joined by + with `--tie` either side:

```js
const mac = /Mac|iPhone|iPad/.test(navigator.userAgentData?.platform ?? navigator.platform);
const mod = mac ? "⌘" : "Ctrl";
```

## Tables

- **One-line cells only**; a row of sentences becomes a heading with labelled
  facts.
- **A wide table scrolls inside itself**, never the page, and the keyboard
  reaches it:

```html
<div class="table" role="region" aria-label="Fetch times by repository" tabindex="0">
  <table>...</table>
</div>
```

- **When narrow, a comparison stacks** rather than squeezing its columns:
  each row a small block, its cells labelled from the header through
  `data-label` and `content: attr(data-label)`, by a container query on the
  table's own width.
- Ratios as bars in CSS from a property (`--r: 0.42`), with the figure beside
  them in Monaspace.

## Diagrams

- **Inline `<svg>`, never an image**, so it takes the page's faces, roles and
  properties: text in `--serif` or `--mono` at a named level, and never
  under 12 px on screen at any width, once the drawing is scaled to fit;
  strokes in `--rule` or `currentColor`, marks in their kind's colour, the
  accent nowhere but a focusable node.
- **Hairlines stay hairlines**: `stroke-width: 1; vector-effect:
  non-scaling-stroke`.
- **Drawn twice**, wide and narrow, chosen by a container query, or with its
  labels as HTML over the drawing, so text never shrinks with it.
- **Labels placed, never only on hover**, with leader lines to their things,
  and nothing overlapping at any frame of an animation.
- **Accessible**: `role="img"` with a `<title>` and `<desc>` for a simple
  figure; a `<figure>` with a `<figcaption>` and a table or list of the same
  facts for a complex one.
- **Ornament is drawn the same way**: strokes in `--tint` at 1 to 1.2 px, never
  filled, drawn in once with `pathLength` when it arrives.

## A Deck

A talk kept as slides is a page to read that a big screen shows one slide
at a time.

- **A deck only where it reads.** It runs as slides only at a window
  large enough that, scaled to fit, no prose falls under 15 px,
  found by measuring rather than guessed (git's runs from 1240 by 790 px).
  Below that it reads as one page in normal flow, with nothing scaled.
- **Its software is the site's own.** The deck's script and the kit's fonts
  come from the deploy, never another server, with transitions off under
  reduced motion.
- **Each figure has two forms**: the drawing for the deck, and for the page
  either the drawing again narrow or, when it's only boxes in a row, a
  list or a table. A container query shows one and sets the other to
  `display: none`, so a reader hears it once, in the same words.
- **Its keys are Vim's**, as the deck's own already are: the key line says
  "h and l step from slide to slide"; Keys lists h j k l, Space, the
  overview, the speaker's view and full screen, and Escape closing Keys
  leaves the overview shut. q does nothing.
- **It is dated as a record**: one muted "A talk, as of" line on its title
  slide, from its last change, and its twin is the slides with what's said
  over each.
- **Its controls and progress are text too**: the slide's number, a strip
  naming its acts and the arrows sit in one band off the slides, at
  `--step-small` and 4.5:1, covering nothing.

## Live Data, Still

- **Hide hints with `visibility`, never `display`**, so showing them moves
  nothing.
- **Redraw only what changed**, keyed by id; the cursor stays on its item at
  its place; arrivals are batched to at most one redraw a second; a new item
  that would land in view waits behind a quiet "one new".
- **Presence with hysteresis**: someone appears on the first beat and leaves
  after two missed ones, fading over 200 ms; nothing flickers.
- **Fit before painting**: measure and cut text before the first frame, so no
  unfinished frame is ever drawn.
- **A new build never reloads a tab**; old clients keep working against the
  new server.
- **Idle means zero**: no frames, timers or polling while nothing changes; a
  hidden tab (`document.visibilityState`) checks but draws nothing.

## The Head and the Mark

Every page's head names it for a tab, a shared link and an agent, and points
at its mark:

```html
<title>World in a bottle.</title>
<meta name="description" content="The lede, in one or two sentences.">
<meta property="og:title" content="World in a bottle.">
<meta property="og:description" content="The lede, in one or two sentences.">
<meta property="og:type" content="website">
<meta name="theme-color" media="(prefers-color-scheme: dark)" content="...">
<meta name="theme-color" media="(prefers-color-scheme: light)" content="...">
<link rel="icon" href="favicon.svg" type="image/svg+xml">
<link rel="alternate" type="text/markdown" href="index.md">
```

The mark is drawn once, in the page's way of drawing ornament (strokes in
`--tint`, never filled), inline at the masthead's top with
`aria-hidden="true"`, since the title beside it names the page. The favicon
is the same drawing reduced to its plainest lines on a 32 grid, at 2 px
strokes, so it holds at 16 px. A favicon can't read the page's properties,
so it and `theme-color` are the one place a site writes a colour's value:
each look's `--tint` and `--background`, copied from `kit/theme.css`, and
copied again when the theme moves.

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <style>
    * { fill: none; stroke: #...; stroke-width: 2; stroke-linecap: round; }
    @media (prefers-color-scheme: dark) { * { stroke: #...; } }
  </style>
  <path d="..."/>
</svg>
```

## The Twin for Agents

Beside every page for people: the same facts as Markdown and JSON (`/page.md`,
`/page.json`), an `llms.txt` at the root saying what the site holds and where,
stable anchors on every section and item, and error text written as people
will paste it.

## Support

The kit and these patterns use what current Chrome, Firefox and Safari all
support, except: `hanging-punctuation` (Safari only), `text-box` and
`interpolate-size` (recent), scroll-driven animations (recent in Firefox and
Safari) and relative colour (the tint's turn). Each is written so a browser
without it gets a page a little plainer, never a broken one.
