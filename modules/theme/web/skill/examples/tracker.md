# An Interface: Slack Water

Slack Water is an interface to work in, a team's issue tracker, "where the
rivers meet, and leave faster", built for people and agents alike. It's made
up here, from a real tracker Joel is proud of, with its names and figures
changed; the design is as it was. Joel uses it all day. What makes it sing
is that one world runs through all of it. The team's areas are waters, the
ground is cold bathymetric teal, status is a tide rising in a ring, priority
is the wake an issue throws, the chart is islands on a calm sea, and the one
painted buoy, orange, appears only where the keys are. Yet the work stays
plain: a row is a title in a reading serif, a count, a face.

## What to Steal

- **Its own world, named once in a stylesheet comment.** The stylesheet
  opens: "Slack Water: cold bathymetric teal for the ground, warm sediment
  for what speaks, one painted buoy for where the keys go." Every token,
  mark and word after that has somewhere to come from.
- **Four faces, each with one job.** "Instrument Serif names things,
  Newsreader is read, Mona Sans works, Monaspace counts." A glance tells a
  name from prose from a control from a number.
- **A headline that is a sentence.** "38 open, 6 moving." Each count is a
  link to what it counts, and the second is italic in Silt. A lede under it
  names the hottest issue, so the screen says how things stand before the
  list starts.
- **Marks hung in a gutter.** Priority, number and status sit in a 128px
  gutter that ends 20px before "the main line", so every title, section
  head and headline starts on one edge. On a phone the gutter folds into a
  mono line under the title.
- **Places as a quiet row of words.** Triage, Mine, Moving, Open, Landed
  are serif words with mono counts, the current one in full ink and
  underlined, "so a switch changes colour and never width". Joel, 14 Nov
  2026: "the site's short navigation on a phone looks great!"
- **One accent, rationed to the keys.** `--accent` is commented "Buoy: only
  where the keys are": the focus ring, the caret, a focused field's
  keyline, the "4 NEW" press. A checked row gets a tick, "with no buoy
  orange".
- **Marks told by shape, not hue.** Urgent "changes shape as well as
  colour, a planing hull, so it's never told by hue alone."
- **Still water.** Someone else's change "lands without moving what a
  person is reading": new rows wait behind a press, the peek lies over the
  list rather than squeezing it, and a changed row washes once in Silt.
- **Whimsy only at thresholds.** "Warm at thresholds, quiet at work." The
  empty Triage says "The tide has turned; go build something."

## The Working Surface

A place is a word and, for Triage and Mine only, a count. The hero puts the
place's quiet line, the waters, the headline and the ledes in one block
(trimmed):

```js
const named = `<span class="nm">${esc(p.name)}</span>${placeCount(p)}`;
return `<a class="place" href="${placeHash(p.id)}"${glossed}
  ${here}>${named}</a>`;

return `<header class="hero">
  <p class="place-says">${esc(says)}</p>
  <div class="hero-gut" aria-hidden="true">...</div>
  <div class="copy">
    ${watersHtml()}
    <h1 class="kin-title headline" tabindex="-1">${hyphensBreak(h.html)}</h1>
    ${lede}
    ${currentFigure(scope, { ... })}
  </div>
</header>`;
```

The masthead is "the top of the page, not a bar laid over it", its rule
shown only once the page passes under it. Its places and the headline
(trimmed):

```css
.masthead { position: sticky; top: 0; height: var(--mast);
  background: var(--background); }
.masthead .place { height: 32px; padding-inline: var(--s);
  font: 400 var(--f-read)/1 var(--serif); color: var(--soft); }
.masthead .place[aria-current="page"] .nm {
  text-decoration: underline 1px var(--soft); text-underline-offset: .3em; }
.kin-n { font: 450 var(--f-mono)/1 var(--mono);
  font-variant-numeric: tabular-nums; color: var(--muted); }
.kin-title { font: 400 var(--f-d4)/.96 var(--display);
  letter-spacing: -.018em; color: var(--strong); text-wrap: balance; }
.kin-title em { font-style: var(--voice); color: var(--warning); }
.hero .copy > .headline { white-space: nowrap; font-size: 2.25rem; }
.waters a[aria-current="page"] { color: var(--warning); }
```

An issue row is one grid. The gutter's three columns (16, 52, 16) hold the
marks and the title starts after them (the row's markup, trimmed, and its
CSS):

```js
`<a class="row kin-item" role="option" id="row-${d.id}" href="#/${d.id}">
  <span class="p">${priorityMark(d.priority)}${glyph.check}</span>
  <span class="id kin-item-id" translate="no">${d.id}</span>
  <span class="s kin-item-mk">${statusMark(d.status, stands)}</span>
  <span class="t-cell">
    <span class="t kin-item-t">${whole(inline(d.title))}</span>
    <span class="meta" aria-hidden="true">...</span></span>
  <span class="who" aria-hidden="true">${avatar(who)}</span>
  <span class="when kin-item-end" aria-hidden="true">${age}</span>
</a>`
```

```css
.row { --kin-item-cols: 16px 52px 16px minmax(0, 1fr) auto auto 24px 40px;
  --kin-item-gap: 12px; }
.row .t { -webkit-line-clamp: 1;
  font: 430 var(--f-read)/var(--l-read) var(--serif); }
.kin-item.cursor { background: var(--selection); }
@container list (width < 640px) {
  .row { --kin-item-cols: 16px minmax(0, 1fr) auto 24px; }
  .row .p, .row .id, .row .when, .row .signals { display: none; }
  .row .meta { display: block;
    font: 450 var(--f-mono)/var(--l-mono) var(--mono); }
}
```

Note the one place it does fill a row: the cursor wears `--selection`, a
step of the ground's own water, one shade off and never a bright box. It's
the exception the faces, the gutter and the hairlines earn; a home or a
list of a dozen rows needs none of it.

## Type and Colour

`--display` is Instrument Serif, for place titles, section heads ("In
progress", in italic) and the wordmark. `--serif` is Newsreader, for row
titles, ledes, the places, placeholders ("Filter these 38 issues") and acts,
mostly in the italic reading voice, `--voice: italic`. `--sans` is Mona
Sans, the body's default, in which the primary act "stands up". `--mono` is
Monaspace Neon, for counts, ids, ages and the tracked capitals of waters and
eyebrows. Each face has a measured stand-in, "so a face that comes late
moves nothing", and `font-synthesis: none` forbids a faked bold or italic.

The scale is "a major third from the reading size, each step fluid":
`--f-mono: .6875rem`, `--f-ui: .875rem`, `--f-read: clamp(1rem, .96rem +
.18vw, 1.0625rem)`, up to `--f-d4: clamp(2.75rem, 1.4rem + 4.8vw, 5.5rem)`.
Line heights sit on the 4px grid (`--l-mono: 1rem`, `--l-read: 1.5rem`),
spaces run `--u: 4px` to `--vast: 96px`, and vertical rhythm is one ladder,
"each gap above a thing larger than the gap below it".

The palette is "Soundings, shoal to abyss: stepped, never blended", text
is "Foam to spume, warmed by the silt in it", and pigments come "each with
one job":

```css
--deepest: #070e12; --sunken: #0b161c; --background: #0f1d24;
--raised: #152731; --hover: #182d38; --selection: #1c3441;
--muted: #8e9c97; --soft: #b9b7a9; --text: #ddd6c5; --strong: #f5eedc;
--accent: #f39a45;   /* Buoy: only where the keys are */
--alert: #ef6f66;    /* Port */
--warning: #d6b86c;  /* Silt: what's arriving, and the voice that names it */
--success: #6cc290;  /* Starboard */
--program: #62b6cf;  /* Lagoon: flowing, River and agents */
--person: #b59de0;   /* Urchin: review, and people who are here */
```

The light ground is "chart paper", `--background: #f5f1e6`, each pigment
"ground down until it holds 4.5:1". The buoy shows in the one ring,
`outline: var(--ring) solid var(--accent)`, 2px wide and 2px off, settling
in from further out. A field is never a box, only a keyline, `inset 0 -1px
0 var(--edge)`, that turns to 2px of buoy while it holds focus.

## Marks, Charts and Whimsy

Status "is a tide: it rises in the ring as the work moves towards done":
Triage a Silt drop, Backlog a dashed ring, Todo a ring, In progress half
full of Lagoon, In review full of Urchin, Done a Starboard disc with a
tick, Cancelled a slashed ring. Priority is wake, up to three chevrons lit
"over a dim track so the scale's end always shows", and Urgent a red hull
(trimmed):

```js
const TIDE = {
  triage: [DROP, "st-triage"],
  in_progress: [`${ring()}<g class="tide">${water(HALF)}</g>`, "st-progress"],
  in_review: [`${ring()}<g class="tide">${water(1)}</g>`, "st-review"],
  done: [DONE, "st-done"],
};
export function priorityMark(id) {
  const p = priorityOf(id);
  if (p.id === 1) return markSvg(HULL, "pr pr-1", p.name);
  const waves = ["M2 14.5 8 10.5l6 4", "M2 9.5 8 5.5l6 4", "M2 4.5 8 .5l6 4"];
  const lit = { 2: 3, 3: 2, 4: 1 }[p.id];
```

The chart, on `v`, is the tracker "from above. Each water is an island on a
calm sea": a lit channel where work moves, open water out to the week's
ring, and paler shallows "where work silts up". Each issue is a buoy in its
status mark, greyed in the shallows. Beside the headline, the current is a
fortnight of arrivals and landings "drawn as a tide gauge". Faces are
portraits over mono initials at 16 to 28px; an agent's is a drawn mark,
"never its initials", and people here now wear a ring of Urchin.

The whimsy waits for thresholds. Landing an issue ripples its mark and the
toast picks from `["made port", "came ashore", "landed", "is in harbour",
"reached the sea", "tied up at the quay"]`, while a screen reader hears
plainly that it landed. An empty Triage reads "Clear water." under the
drawn crest, and the loading screen draws the crest over "Reading the
water".

## How It Works Under the Keys

Every action has a key, and `?` lists them in groups named in Title Case:
"Getting Around", "Changing", "Going". `j` and `k` step the list; `g i`,
`g m`, `g a`, `g o` and `g d` go to the places, and `g 0`, `g 1` onward to
a water; `/` filters, `c` files, `x` checks rows for a batch, and `s`, `p`,
`a`, `l` and `d` move, prioritize, assign, label and land. `⌘K` is one
palette for "Search or Do Anything". Keys print as the reader's keyboard
names them (`keyName` makes ⌘ Ctrl off a Mac), the headline's gutter shows
the key of the water pointed at, and on touch an act is its words alone.

Enter opens an issue as an article on the main line, its properties in the
margin; Space peeks, a drawer over the list's right (`--peek: min(46vw,
680px)`) that never reflows it; `[` and `]` walk to the previous and next
issue. A count in a headline opens a list of exactly that many rows:
"Every number leads to what it counts." The words carry the design:

- "Every issue not landed or let go." (Open's quiet first line)
- "Ten landed this fortnight, three let go."
- "Couldn't change #77: the server didn't answer. It's back as it was; make
  it again in a moment."
- "Clear water: nothing waits on you."

## What It Does Not Do

- No boxes round rows, no cards, no glass bar: the masthead is "the page's
  own water and voices, no glass, no box".
- No meaning in colour alone: the hull, a checked row's tick, a hollow buoy.
- No orange where the keys are not, not even on a selected row.
- No movement under the reader: no relayout beside the peek, no rows
  jumping in, no count sliding a place's name sideways.
- No menu on a phone, where each place stands "on a row of its own under
  the top row, never behind a menu"; no title cut mid-word.
- No "we", "oops", "invalid" or "something went wrong"; never asks to reload.
