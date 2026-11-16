# A Site of Several Pages: The Spring

The Spring is a site of several pages about one thing, a sandbox service.
It's made up here, from a real site Joel is proud of, with its subject,
names and figures changed; the design is as it was. A front page shows the
sandbox's command in a live terminal replay. Eight chapters in two groups,
the sandbox and the record, cover how it works, its security, speed,
choices, what's next, evidence and history. A desk is where the project's
people and their agents leave notes and handoffs. Its readers are the
engineers and leads who weigh the project, plus anyone who wants a sandbox
today. The site got where it is through Joel's corrections: "it feels like
frankenstein right now", replays that were "sometimes too fast and sometimes
too slow", "a responsive layout design that does not look gross" and "there
are screen widths where the site looks kinda crappy". Each one is answered
by a dated comment in its stylesheet. What makes it sing is that the proof
is the hero. Beside "A machine of your own, in moments." the real command
runs: recorded on real hardware, captioned with its commit and linked to its
raw log. Every claim after that has a mark, a word and a link to its
evidence.

## What to Steal

- **A masthead of two named groups.** Under the mark and the name, a hairline
  and then every page, always in sight. "THE SANDBOX" and "THE RECORD" are
  small spaced mono labels, and each is followed by lowercase italic page
  names split by middots. The desk sits apart. The reader sees the whole site
  and how it divides without opening a menu ("two tiers, every place always
  shown", his pick of four).
- **A hero split, words beside a working replay.** From a 1040 px head the
  title, the lede and the contents take the left column and the replay takes
  1.2 times as much on the right, aligned to the bottom. The first screen
  makes the claim and proves it in the same glance.
- **A teal tint taken from the kit's glyph hues.** The kit's `--tint` is
  `--action` (gold). This site turns it to `--program`, the hue the theme uses
  for programs, because a spring is water held under stone. One variable
  colours the italic half of every heading, the labels, the play button and
  the ornaments.
- **One number under the title.** A muted "tier" line carries the single
  figure worth knowing (how long a guest takes to its first command) and
  links it to its raw run, so the headline promise is checkable at once.
- **A standing line.** A count of what works today, what works in part and
  what doesn't yet, in small mono, links down to the board, so a returning
  reader knows where things stand before reading a word of prose.
- **An italic contents list.** The page's five parts appear as lowercase
  italic links, on one line where they fit and as a short column where they
  don't. That is done with a `--fits` width and no breakpoint.
- **A title set in its designed lines.** Inline `--fw` and `--fp` give the
  longest line's width in em, and the size is `97cqi / --fw`. The title fills
  its container and never breaks anywhere but where it was designed to.
- **Three marks that always carry a word.** A full leaf dot is Works, a half
  amber dot Partly and a dashed muted ring Not yet. They are drawn once in a
  legend and reused in the board and in each way in, never as colour alone.
- **Wave rules from the mark.** A band of four strata closes the head across
  the wide line, and a small three-stroke fleuron, masked in the tint, sits at
  each join between parts. Both are drawn from the same curves as the logo.
- **Every section says where to go deeper.** Each part on the front page ends
  with "Go deeper: Speed →", and each chapter ends with "Next: The choices →".

## The First Screen

At 1440 px: a two-tier bar, then the head as two columns over a wave band. At
390 px the second tier scrolls sideways under edge fades, and the eyebrow
drops the site's name, which the bar already gives. Trimmed:

```html
<nav class="places" aria-label="Pages"><ul class="pl" role="list">
  <li class="g"><span class="gw" aria-hidden="true">The sandbox</span>
    <ul role="list" aria-label="The sandbox">
    <li><a href="get-started.html">get started</a></li>
    <li><a href="sandbox.html">how a sandbox works</a></li> ...</ul></li>
  <li class="g"><span class="gw" aria-hidden="true">The record</span> ...</li>
  <li class="g g-desk"><a href="desk.html" aria-label="The desk">...</a></li>
</ul></nav>
...
<p class="label"><span class="seg">The Spring</span><wbr><span class="seg">
  <span class="sep">·</span>the sandbox</span> ...</p>
<h1 class="fit split" style="--fw: 9.119; --fp: 4.808"><span class="w">
  <span class="p">A machine</span> <span class="p">of your own,</span></span>
  <span class="w"><em><span class="p">in moments.</span></em></span></h1>
<p class="tier">A guest boots to its first command in
  <a href="...">well under a second</a>, measured and linked.</p>
<p class="lede">The Spring gives you a sandbox: a Linux machine made from a
  Nix closure, that you pause, resume and throw away.</p>
<ol class="contents" aria-label="This page" style="--fits: 35.9em">
  <li><a href="#problem"><em>the problem</em></a></li> ...</ol>
<figure class="cast" id="rec-cli" style="--cols: 74"
  data-src="casts/hero.cast" aria-labelledby="rec-cli-cap"> ...
```

The CSS that sets it, trimmed:

```css
.places .g > ul > li:not(:last-child)::after {
  content: "\00b7" / ""; color: var(--muted); padding: 0 var(--mark); }
.places .gw { font-family: var(--mono); font-weight: 500; font-size: 12px;
  letter-spacing: 0.16em; text-transform: uppercase; color: var(--muted); }
.places .g > ul a { font-style: italic;
  font-variation-settings: "opsz" 24, "SOFT" 100, "WONK" 1; }
.places a[aria-current="page"] { color: var(--strong);
  box-shadow: inset 0 -2px 0 var(--jade); }
.head h1.fit { white-space: nowrap; line-height: 1.04;
  font-size: min(var(--hmax, 112px), var(--hone, 112px),
    calc(97cqi / var(--fw))); }
.head .contents > li {
  flex: 0 1 max(0px, calc((var(--fits, 0em) - 100%) * 999));
  min-width: max-content; }
@container head (width >= 1040px) {
  .head.front .head-in { display: grid; column-gap: var(--screen);
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr); align-items: end; }
  .head.front .head-in > .band { grid-column: 1 / -1; } }
```

## Type and Colour

Two faces from the kit: Fraunces for everything read and Monaspace Xenon for
whatever names a thing (labels, the standing line, figures, code and keys).
The body is Fraunces 380 at 18px on a 30px line. `font-optical-sizing: none`
is set and the axes are chosen per rank: `"opsz" 14, "SOFT" 50, "WONK" 0` for
text, `"opsz" 24` with `"SOFT" 100, "WONK" 1` for italics, and `"opsz" 96` for
the display. The scale is a major third from the body, on a 4px ladder:

```css
:root {
  --turn: 0;
  --tint: var(--program);
  --gold: var(--tint);
  --amber: var(--action);
  --leaf: var(--success);
  /* A major third from an 18 px body. */
  --note: 14.4px; --body: 18px; --large: 22.5px; --section: 28.13px;
  --title: 43.95px; --display: 85.83px;
  /* The ladder, from a 4 px unit. */
  --tie: 4px; --mark: 8px; --apart: 12px; --items: 16px; --groups: 20px;
  --inset: 24px; --frames: 32px; --screen: 48px; --huge: 64px;
  --measure: 36rem;
}
```

The palette is the kit's, with the grounds left unturned (`--turn: 0`). In
dark mode that is a `#111c18` ground, cream `--strong` titles and sage text.
The tint is `--program` (`#8CD3CB` dark, `#135651` light). Gold, in name
only, now means the tint, and real amber is kept for Partly. The dark look
alone gets a faint radial wash of 9% teal at the top. The current-page
underline, `--jade`, is `--accent` by night and `--program` by day.

Each page links only `kit/theme.css` and then its own stylesheet, and that
stylesheet does what `base.css` would not. It uses fixed pixel ranks in
place of base's fluid `--step` clamps, and a 36rem measure in place of 30em.
It sets the axes by hand in place of optical sizing. It turns the tint. And
it carries the site's own parts: the two-tier bar, the replay player,
diagram inks, status marks and container-sized titles. The cost is real:
base's `hanging-punctuation` and `font-synthesis: none` appear nowhere in
it, so a site that leaves base behind takes those with it.

## Figures, Replays and Ornament

The replay is an asciicast of a real run. Without the script, the pane holds
the whole transcript as plain text. The player writes each line into a pane
that keeps them all, so the reader can scroll back. It offers speeds of 0.25,
0.5, 1, 2 and 4 times, a step mode, tick marks on the seek bar at each step,
and a "The 8 steps" fold that lists them with times. A still stretch over 2 s
plays as 2 s, and the frame head says so. Under reduced motion, lines don't
fade and each step shows whole. The pane's type scales to keep the recorded
columns:

```css
.cast-frame { position: relative; border: 1px solid var(--edge);
  border-radius: 4px; background: var(--sunken); }
.cast-head { font-family: var(--mono); font-size: 12px; color: var(--muted);
  border-bottom: 1px solid var(--edge); padding: var(--mark) var(--items); }
pre.cast-pane { white-space: pre-wrap; line-height: 1.54em;
  font-size: clamp(10px, calc((100cqi - 58px) / ((var(--cols) + 1) * 0.62)),
    14px); }
.cast-bar .cast-play { width: 44px; height: 36px; background: var(--gold);
  border-color: var(--gold); color: var(--background); }
pre.cast-pane .ln.fresh { animation: cast-in var(--arrive) var(--ease) both; }
@media (prefers-reduced-motion: reduce) {
  pre.cast-pane .ln.fresh { animation: none; } }
```

Diagrams are hand-drawn inline SVG with a `<title>`. The host drawing uses
solid boxes for what is shared and dashed `--accent` outlines for each jail,
labelled in the serif with notes in mono. Wide figures are drawn twice, wide
and narrow, and a container query picks one, so no label shows under 12px.
A section with no figure says why in a `data-figure` attribute ("a drawing
would only decorate them"). Rules are wave bands and fleurons. The footer
ends on a line, a ring and a line.

## How It Moves the Reader

The front page is a walk of five stops, listed in the head: the problem,
what you get, try it, where it stands, go deeper. `j` and `k` step between
them, and `?` opens a key panel. Each stop links on to its chapter. The last
stop is a table of seven chapters with one line saying what each answers.
Every chapter ends on the next, so the record reads in order. The desk is
set apart in the bar as "the one place to work". The page the chapters
replaced sends each of its old ids on to the section that now says the same
thing. Every page has a markdown twin, and llms.txt points a model at them.

The headings use one shape throughout: a plain clause, then an italic turn in
the tint.

- "A machine of your own, *in moments.*"
- "Somewhere to try it, *that isn't your laptop.*"
- "Make one, run in it, *throw it away.*"
- "Seven chapters, *one story.*"
- "Speed, *as you wait for it.*"

Mono eyebrows say plainly what each part is ("The problem", "Where it
stands"). Status words are fixed sets: Works, Partly and Not yet on the front
page, and Proved, Partly proved, Not built and Leaning towards in the
chapters. A "What's rough" list says the bad news in plain sentences.

## What It Does Not Do

- No cards. The three ways in sit level in a subgrid with no fill or border.
  The board's rows are parted by hairlines. The only framed boxes are the
  replay and its controls.
- No colour without a word. Each mark is printed beside Works, Partly or Not
  yet, and the legend draws all three.
- No mono prose. Mono stays small, on labels, figures, code and keys.
- No hamburger and no hidden menu. Below 1280px the places scroll sideways in
  one line, and the bar sticks only from 80rem, where it fits.
- No claim without its raw. Figures link their run, and "not measured" is said
  as such.
- No autoplay. "Nothing plays until asked."
- No photographs or stock icons. Every picture is drawn from the mark's
  strata or the system's own boxes.
