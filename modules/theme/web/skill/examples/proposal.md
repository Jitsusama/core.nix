# A Proposal: The Ledger

The Ledger is a page to read, a one-page proposal that a storage service keep
each of its records as one append-only log. It's made up here, from a real
page Joel is proud of, with its subject, names and figures changed; the
design is as it was. It was written for the engineers fighting that
service's fires, from everything they'd said about them. What makes it sing
is that the idea and the ornament are one drawing. The log is a jade stem of
numbered records, every reader of it is a branch ending in a leaf, the same
stem with two leaves joins each chapter to the next, and an opened aside
grows down a stem to a gold bud. It reads as a dark green book set in a
soft, wonky Fraunces, with gold kept for the word that turns each title. Its
eyebrow says what it is and who it's for, its status line says when it was
last true with an italic postscript under it, and a quiet line says j and k
step from part to part.

## What to Steal

- A centred drawn crest above a spaced eyebrow. A gold line flourish with a
  letter in a double ring, then one mono line in capitals tracked 0.24em
  saying what this is and who it's for. The reader knows the register, a
  book with a seal, before reading a word.
- A two-part title with a tinted italic turn: "The log `<em>is</em>` the
  record." The roman is cream at weight 560 and the turn is gold italic at
  420. Every chapter title repeats it, so each heading is a sentence with one
  stressed word, and the eye lands on the claim.
- An italic lede that is the whole argument. Three sentences in linen,
  balanced, at most 33rem wide. A reader who stops at the first screen still
  has the proposal.
- A status line with a glyph. A dot half filled with gold means "under way",
  and the same mark comes back in the stages list beside a filled mint dot
  (done) and a sage ring (next), each with hidden words for screen readers.
  One mark, learned once, says where the work is.
- Contents as a run of italic words with gold roman numerals, centred and
  wrapping, not a nav bar. It shows how long the walk is and lets the reader
  jump, with no chrome.
- Then and now side by side. Each fire is a two-column grid: "then" under a
  vermilion label with a mono date line, "with the ledger" under a mint
  label. A container query folds it to one column. The problem and its
  answer are read together, never pages apart.
- A bridging lead at every chapter's end: a centred italic line that asks the
  next chapter's question ("who gets to add the next one?"). The walk pulls
  itself along.
- Folds drawn as branches. Depth for the reviewer sits in `<details>` whose
  summary is a question, behind a gold ring whose plus turns to a cross, and
  the opened text hangs from a jade stem ending in a leaf. A separate page of
  details was folded back into the walk, and its address now only redirects
  there.
- Numbers set as display figures: lining, tabular, at up to 3.4rem, with the
  unit small, italic and gold. Running text uses oldstyle figures, so a
  measured result looks like a result and prose stays quiet.

## The First Screen

Everything is centred: crest, eyebrow, title, lede, status, contents, then
the whole idea as one plate, still inside the header.

```html
<header class="hero">
  <svg class="orn crest" viewBox="0 0 260 60" aria-hidden="true">
    <g fill="none" stroke="currentColor" stroke-width="1.2"
       stroke-linecap="round">
      <path d="M8,34 C40,34 62,34 82,30 C98,27 104,16 96,11 ..."/>
      <circle cx="130" cy="30" r="17"/>
      <circle cx="130" cy="30" r="21" stroke-opacity="0.45"/>
    </g>
    <text x="130" y="38.5" text-anchor="middle" fill="currentColor"
      style="font: italic 500 23px var(--serif)">L</text>
  </svg>
  <span class="eyebrow">the ledger &middot; a design for its engineers</span>
  <h1>The log <em>is</em> the record.</h1>
  <p class="lede">With the ledger, the bucket is the record ...</p>
  <p class="status"><span><span class="dot" aria-hidden="true"></span>
    Aiming for review on Monday</span> ...</p>
  <ol class="contents" aria-label="Chapters">
    <li><a href="#fires"><span>I</span>the fires</a></li>
    ...
  </ol>
  <figure class="plate">...</figure>
</header>
```

```css
.eyebrow { font: 500 0.72rem/1.2 var(--mono); letter-spacing: 0.24em;
  text-transform: uppercase; color: var(--gold); }
.hero h1 { margin-top: calc(var(--u) * 6);
  font-size: clamp(2.9rem, 1.6rem + 6.2vw, 5.8rem); line-height: 0.96;
  font-weight: 560; letter-spacing: -0.022em;
  font-variation-settings: "SOFT" 100, "WONK" 1, "opsz" 144; }
.hero h1 em { font-style: italic; font-weight: 420; color: var(--gold); }
.hero .lede { margin: calc(var(--u) * 8) auto 0; max-width: 33rem;
  font-size: clamp(1.08rem, 1rem + 0.4vw, 1.24rem); line-height: 1.55;
  color: var(--linen); font-style: italic; font-weight: 360;
  text-wrap: balance; }
.dot { width: 8px; height: 8px; border-radius: 50%;
  border: 1.5px solid var(--gold);
  background: linear-gradient(90deg, var(--gold) 50%, transparent 50%); }
.contents { font: italic 400 0.98rem/1.6 var(--serif); }
.contents span { font: 500 0.66rem/1 var(--mono); font-style: normal;
  letter-spacing: 0.14em; color: var(--gold); margin-right: 0.45em; }
```

## Type and Colour

Two faces, self-hosted as woff2 and the serif preloaded. Fraunces, variable
from 100 to 900 with its italic, carries everything read. Monaspace Neon, at
400 and 500 only, is kept for small furniture: eyebrows, numerals, labels,
dates, terms in a definition list and the quiet words inside diagrams. The
body is light and soft; headings turn the wonk on, and the optical size
follows the role:

```css
html { font: 380 clamp(17px, 15.6px + 0.32vw, 19.5px)/1.62 var(--serif);
  font-optical-sizing: auto; font-variation-settings: "SOFT" 50, "WONK" 0; }
h1, h2, h3 { color: var(--cream); text-wrap: balance;
  font-variation-settings: "SOFT" 100, "WONK" 1; }
p, li, dd { text-wrap: pretty; hanging-punctuation: first last; }
```

The scale: the title at `clamp(2.9rem, ..., 5.8rem)` weight 560; chapter
titles at `clamp(2.05rem, ..., 3.15rem)` weight 520, both at `"opsz" 144`;
fire titles and fold questions in italic at 1.55rem and up to 1.5rem at
`"opsz" 72`; figures up to 3.4rem; the lede up to 1.24rem; captions in
italic at 0.95rem; mono furniture between 0.66rem and 0.82rem. Spacing is one
4px unit, `--u`, always written `calc(var(--u) * n)`. Reading text holds to a
36rem measure inside a 50rem column.

```css
--night: #090f0d; --forest: #111c18; --fern: #23372b; --moss: #32473b;
--sage: #718779; --quiet: #8a9f8f; --seafoam: #81b8a8;
--parchment: #c1c497; --linen: #d6d5bc; --cream: #f7e8b2;
--jade: #509475; --mint: #63b07a; --gold: #e5c736; --vermilion: #ff5345;
```

The ground is forest, with one faint radial glow of `#17261f` at the top of
the page. Text climbs in three steps: parchment for prose, linen for ledes
and the "now" answers, cream for headings and bold. Quiet greys out captions
and meta. Gold is the tint and it is rationed: the turned word, numerals,
eyebrows, the crest, the focus ring and the text selection. Links are
seafoam over a moss underline that turns cream over gold on hover. Vermilion
means "then" or the hot path and mint means "now" or done; neither appears
anywhere else.

## Figures and Ornament

Every figure is a hand-drawn inline SVG, a "plate", on the same metaphor. One
hidden SVG at the top of the body defines a leaf and three arrow tips, gold,
red and jade, reused by every plate. Strokes are named by class, not drawn
with inline colour: `.stem` jade at 2.25, `.vine` jade at 1.5, `.gilt` gold
at 1.25, `.hot` vermilion dashed `3 4`, records filled fern with a moss edge,
and the next record forest with a dashed gold edge. Labels are two voices:
`.name` in italic Fraunces at 21px and `.sub` in mono at 12.5px in quiet.
Below 42rem the subs are hidden and the names grow to 30px, so the words
carry on a phone. Each plate is `role="img"` with a title and a full sentence
of description, and an italic one-line caption ("One stem, and everything
else a branch that reads it.").

```html
<g id="leaf">
  <path fill="#509475" d="M0,0 C9,-11 25,-11 32,0 C25,11 9,11 0,0 Z"/>
  <path fill="none" stroke="#111c18" stroke-width="1" d="M3,0 L27,0"/>
</g>
...
<path pathLength="1" class="vine" d="M403,332 C460,332 498,294 552,294"/>
<use href="#leaf" transform="translate(552,294)"/>
<text class="name" x="596" y="292">relay</text>
<text class="sub" x="596" y="312">out to the mirror</text>
```

Every drawn path carries `pathLength="1"`, so one rule draws any of them.
Motion runs only once a short script has added `html.motion`, which it does
when reduced motion isn't asked for: paths draw in, leaves bud, labels fade,
each once on arrival. The CSS comment says it outright: "nothing is scrubbed
by the scroll". Without the script the page is whole. Lists use a gold
fleuron, `content: "\2766"`, and the footer closes on a small gold sprout.

## How It Moves the Reader

The walk is ten numbered chapters, and the first screen gives the whole idea
before chapter I, as the tree plate. Between chapters sits the joining stem.
Every chapter opens on an eyebrow with its numeral and what it is about ("I ·
what it ends", "VIII · what's the catch") over a title with its turn. Then
it draws on a small set of parts: an opening paragraph with a gold italic
drop cap, prose at the measure, a plate, and a list of rules whose items
open on a bold claim ("The create is the decision."). Some add one centred
italic thesis line ("An acknowledged write is in the log, exactly once."),
and some add folds whose summaries are questions ("What if the answer never
comes back?", "And if a builder falls behind, or dies?"). The first nine end
on a bridging lead.

The words are short and plain. Chapter titles are phrases of two to four
words, with the tail after the turn set in gold:

- "Two weeks of fires"
- "One log, and nothing else"
- "Maintenance, off the path"
- "Measured, not promised"
- "Where it stands"

Leads are spoken sentences that hand over: "Look closely and the fires share
one root. ... So start there." and "None of this is free. Here is what it
costs." What a switch removes is a list of struck-through italic words with
a vermilion line. It owns its limits under "What the ledger won't fix on its
own", and ends on a signature and a motto in italic.

## What It Does Not Do

- No cards. Fires and pull requests are parted by 1px fern hairlines, with no
  fill, no shadow (its stylesheet has no `box-shadow`) and no rounded box.
- No pills, badges or chips. Status is a glyph and a sentence; stages are
  dots and bold words.
- No mono everywhere. Mono never sets a heading or running prose; it stays
  small, gold or quiet, on labels, dates and numbers.
- No app shell or chrome: no top bar, logo strip, sticky header, sidebar,
  hamburger, buttons, call to action or theme switch. The contents line is
  the only navigation, and it scrolls away with the page.
- No tables, icon set or photographs; every picture is drawn on one metaphor.
- No framework. One script of about 25 lines, which only adds motion, and one
  stylesheet.
- No chevron accordion. A fold is a branch, opened by a ring that turns.
