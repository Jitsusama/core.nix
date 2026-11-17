# A Tour: The Crossing

The Crossing is a guided tour, a page to read and not an interface to work
in. It's made up here, from a real page Joel is proud of, with its subject,
names and figures changed; the design is as it was. It explains a new
deploy design to the people who build, review and run it, from why today's
deploys fail to what has been proven and which pull requests carry it, in
eight numbered stops on one long page. It shares type, ranks and space with
its sister sites but turns the palette to a harbour blue. Joel loved it, then
sent it back with "it is way too verbose, consider the heart of the other
designs more", after which two thirds of its words went and fourteen stops
became eight. What makes it sing is that one idea, start the new copy beside
the old and let the old one drain, is drawn everywhere: the title, the mark,
the favicon, the fleurons, the state colours and the stepped diagrams all
show a filled new box beside an outlined old one.

## What to Steal

- **One idea as the title, half in italic colour.** "Start new, drain old."
  sets its second half in italic azure, and every stop heading repeats the
  two-beat shape, so the reader has the thesis before the first paragraph.
- **A drawn mark in the site's own metaphor.** The masthead ornament is a
  filled azure box over an amber-outlined one, between curved flow lines.
  Amber is the draining state, so the mark is the design in miniature.
- **The family's palette turned to the site's tint.** The stylesheet says
  the night is blue where its sisters' is jade, and one `--hue` variable
  carries azure into labels, links, focus rings, drop caps and italics.
- **Numbered stops as an italic contents line.** All eight stops sit on the
  first screen as azure mono numbers before lowercase italic names, so the
  reader sees the whole route and its length before starting.
- **A standing line under the lede.** A gold half-filled circle, one
  sentence on where the work is, and the date it was updated tell a
  returning reader at once whether anything moved.
- **Diagrams that step, with words that change.** The hand-over figure has
  Back, Next and Play, a count, and a live caption saying each step in a
  sentence, so the reader sets the pace and never decodes motion.
- **Every state has a colour and always a word.** Starting is lavender and
  dashed, accepting azure, draining amber, gone grey, each with its name
  printed beside it, so colour never carries meaning alone.
- **Depth folded as questions.** Each deep dive is a `details` whose summary
  is a reader's question ("Can I still deploy my working tree to
  staging?"), so the walk stays short and the curious can open more.
- **Honest captions.** "The order is illustrative; the counts are the CI's."
  Saying what is drawn and what is measured earns belief.

## The First Screen

One centred column: ornament, mono label, display title, italic lede,
standing line, contents, key hint and theme switch, and nothing else; no
bar, no logo row, no sign-in. Trimmed, wrapped at 80:

```html
<header class="head">
  <svg class="ornament" width="240" height="56" viewBox="0 0 240 56"
       aria-hidden="true">
    <path d="M8 36 C 52 36, 78 34, 98 28" fill="none"
          stroke="currentColor" stroke-width="1"/>
    <!-- three more flow curves, mirrored, two at opacity 0.6 -->
    <rect x="104" y="12" width="32" height="11" rx="2" fill="currentColor"/>
    <rect x="104.75" y="29.75" width="30.5" height="9.5" rx="2" fill="none"
          style="stroke: var(--amber)" stroke-width="1.5"/>
  </svg>
  <p class="label">The project<span class="sep">·</span><wbr>its
    deploys<span class="sep">·</span><wbr>the crossing</p>
  <h1>Start new, <em>drain old</em>.</h1>
  <p class="lede">Today a deploy changes code under programs ...</p>
  <p class="standing"><span class="half" aria-hidden="true">◐</span>Three
    pull requests merged and one closed, ...</p>
  <ol class="contents" aria-label="Stops on the tour">
  <li><a href="#idea"><span class="num">02</span><em>the one idea</em></a></li>
  <!-- 01 to 08, each the same -->
  </ol>
  <p class="keys"><kbd>j</kbd> and <kbd>k</kbd> step from stop to stop, ...</p>
  <div class="switch" role="radiogroup" aria-label="Theme">...</div>
</header>
```

The CSS that sets it, trimmed and wrapped:

```css
.head { text-align: center; padding: var(--huge) var(--inset) 0;
  max-width: var(--wide); margin: 0 auto; }
.ornament { display: block; margin: var(--frames) auto var(--inset);
  color: var(--hue); }
h1 { font-weight: 600; font-size: var(--display); line-height: 88px;
  letter-spacing: -0.025em;
  font-variation-settings: "opsz" 96, "SOFT" 100, "WONK" 1;
  color: var(--strong); margin: var(--groups) auto 0; max-width: 14ch; }
h1 em { font-style: italic; font-weight: 500; color: var(--hue); }
.lede { font-style: italic; font-size: var(--large); line-height: 36px;
  font-variation-settings: "opsz" 24, "SOFT" 100, "WONK" 1;
  max-width: 34rem; margin: var(--frames) auto 0; color: var(--strong); }
.standing .half { color: var(--gold); padding-right: var(--mark); }
.contents { list-style: none; display: flex; flex-wrap: wrap;
  justify-content: center; gap: var(--mark) var(--groups); }
.contents .num { font-family: var(--mono); font-style: normal;
  font-weight: 500; font-size: var(--fine); letter-spacing: 0.1em;
  color: var(--hue); padding-right: var(--mark); }
```

At 390 px the title breaks after the comma, giving "drain old." its own
line; below 640 px `--display` drops to 54.93px and the contents rewrap.

## Type and Colour

Two faces, self-hosted as woff2. Fraunces, variable from 100 to 900 with a
separate italic file, sets everything read; Monaspace Neon at 400 and 500
sets what names a thing: labels, numbers, code, keys, buttons, table heads
and diagram small print. The body is Fraunces 380, 18px on a 30px line, with
`font-optical-sizing: none` and the axes set by hand: `"opsz" 14, "SOFT" 50,
"WONK" 0` for text, `"opsz" 24` to `96` with `"SOFT" 100, "WONK" 1` for
italics and headings, so the display is soft and a little wonky while the
body stays plain. The scale is a major third from the 18px body, by rank:

```css
--fine: 0.8rem; --note: 14.4px; --small: 1rem; --body: 18px;
--large: 22.5px; --section: 28.13px; --title: 43.95px; --display: 85.83px;
--tie: 4px; --mark: 8px; --apart: 12px; --items: 16px; --groups: 20px;
--inset: 24px; --frames: 32px; --screen: 48px; --huge: 64px;
--measure: 36rem; --wide: 64rem;
.label, .eyebrow, .kicker-h { font-family: var(--mono); font-weight: 500;
  font-size: var(--note); line-height: 20px; letter-spacing: 0.16em;
  text-transform: uppercase; font-feature-settings: "case";
  color: var(--hue); margin: 0; }
```

The night is `--background: #0b1422`, with `--raised: #132136` and
`--sunken: #070e19`. Text is `#d8dee7`; headings take `--strong: #f4ecd8`,
a warm cream that keeps the blue from going cold. The tint is `--hue:
var(--azure)` (`#62b1ff`), with `--hue-quiet` at 55%. Gold (`#e8c34f`) is
kept for what is running: the diagrams' lock, the standing mark, the
ladder's `where` lines. States are colours: `--starting: var(--lavender);
--accepting: var(--azure); --draining: var(--amber); --gone: var(--rule)`.
The light theme is the same colours in oklch, cast toward the night's hue 255
at a whisper of chroma (`--background: oklch(0.985 0.006 255)`), each glyph
hue darkened to hold contrast. A 10% radial azure wash tops the dark theme
only; the source says that on paper it reads as a smudge.

## Figures and Ornament

The diagrams are hand-written inline SVG, each with `role="img"` and a
`<title>` saying the whole picture in a sentence. Names are serif, all that
lines up is Monaspace, and wires, routes, files and cards have a class each.
A figure whose labels would shrink below 12px is drawn twice, wide and
narrow, and a container query picks one, so text never scales down:

```css
.panel, .decay { container-type: inline-size; }
.panel svg.narrow, .decay svg.narrow { display: none; }
@container (max-width: 26rem) {
  .panel svg.wide, .decay svg.wide { display: none; }
  .panel svg.narrow, .decay svg.narrow { display: block; }
}
svg text { font-family: var(--serif); fill: var(--strong); }
svg .sub { font-family: var(--mono); font-size: var(--fine);
  fill: var(--muted); }
svg .route.on { stroke: var(--azure); stroke-width: 3;
  stroke-dasharray: 7 9; }
svg .card.starting rect { stroke: var(--starting); stroke-dasharray: 6 5; }
svg .card.draining rect { stroke: var(--draining); }
svg .card.gone { opacity: .32; }
```

The rest are plain HTML: a strip of small bars, one per rollout, in
seaglass, vermilion and an outline for cancelled; a stacked bar of failure
causes keyed by count; big mono figures over a sentence each, under a
hairline. Captions are italic, muted and left-aligned under a plate. Motion
is earned: the tour's script adds `html.motion` only when
`prefers-reduced-motion` is `no-preference` and IntersectionObserver exists.
Then paragraphs fade up 14px on a 60ms stagger, the strip's bars rise 22ms
apart, the bar's segments grow from the left, the flow's dot hops from step
to step and an active route's dashes run. The page reads whole without the
script.

Ornament is one family: the masthead mark, a 120px fleuron of two small
boxes between rules above every stop in `--hue-quiet`, and the same two
boxes in the favicon and the return notice. A stop whose first paragraph is
ordinary prose opens it with an italic azure drop cap, 98px on an 80px line.

## How It Moves the Reader

The eight stops walk from problem to proof: why deploys fail, the one idea,
commit to host, watch and undo, when it breaks, what's proven, the pull
requests, and what changed here. Stop 02 holds the key line, alone in large
italic cream as `.big`: "A release never changes a running process's
code." Before it is the cost of not having it; after it, how it is kept.
Each stop has one skeleton: fleuron, a mono eyebrow ("Stop 03 · commit to
host"), a heading, a short opening paragraph, then figures, tables, folds.

Headings are plain sentences in two beats, the turn in italic azure:

- "Today's deploy *fights* its hosts."
- "Nothing reaches into a host. *Every host pulls.*"
- "Watch it happen. *Undo it in one command.*"
- "When it breaks, *what runs keeps running.*"
- "Proven in the lab. *Now on real hosts.*"

Folds keep the walk short: "Words you'll meet" is a glossary behind a mono
summary with a circled plus; each deep dive is a question under a kicker
("Deep dive · the host's agent"), its plus turning to a cross as
`::details-content` animates the height. Proof splits honestly into "Already
proven" (filled seaglass dots) and "Not yet run anywhere real" (hollow
ones). Keys `j`, `k`, `g`, `G` and `?` step between stops, and the foot
signs off in italic, naming its author and the agents who helped.

## What It Does Not Do

- **No cards.** Groups are hairline ledgers, a 1px `--edge` rule above each
  item; the comments say "hairline ledgers, no boxes", and the terminal
  board is "the one framed thing on the page".
- **No pills.** Pull request states use a class named `.pill` but render as
  uppercase mono words after a filled, half or hollow unicode circle, with
  no background or border.
- **No mono everywhere.** Monaspace is held to what names a thing; every
  heading, paragraph, figure caption and tab is Fraunces.
- **No app shell or chrome.** No nav bar, sidebar, sticky header, hero
  image or footer of links. The only sticky thing is a notice of what
  changed, hidden until there is news.
- **No ornament without a meaning.** Boxes round to 4px at most, the only
  circles are step numbers and the flow's moving dot, one soft shadow sits
  on that notice, and the one gradient is the faint harbour wash.
