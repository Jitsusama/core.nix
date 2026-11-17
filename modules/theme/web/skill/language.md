# The Design Language

The materials a site is made from, as his loved sites use them. Load this
when choosing a site's world (SKILL.md, The Process, step 2). Each part
says what to choose, the range the loved sites span, and what the
engineered pages did instead. The CSS for each technique is in `craft.md`;
his verdicts behind each are in `rulings.md`.

## A World of Its Own

Every site takes a metaphor from its subject and lets it run through
everything. Write it as the stylesheet's opening comment, then make every
choice below come from it.

| Site | Its world | Where it shows |
| --- | --- | --- |
| The Ledger | a book with a seal; the log as a jade stem, readers as leaves | crest with a letter in a double ring, leaf joins between chapters, folds that hang from a stem to a gold bud |
| The Crossing | a harbour at night; new boxes in as old ones drain | a mark of a filled box over a draining one, azure ground, states named starting, accepting, draining, gone |
| The Spring | water held under stone | teal tint, wave rules, strata in the mark |
| Slack Water | slack water where rivers meet | teams as waters, status as a tide rising in a ring, priority as a wake, the chart as islands, one buoy for the keys |

The world names states and empties too: Slack Water's empty Triage says
"The tide has turned; go build something." A site with no world falls back
to the kit's defaults, and every such site looks alike.

## Type

Type is the main material: most of what made the loved sites sing.

- **A display serif with character, set large and soft.** Fraunces at its
  display axes ("SOFT" 100, "WONK" 1, optical size high) at 4 to 6 rem for
  a page's title, weight 520 to 560, letter-spacing slightly tight, line
  height under 1. Slack Water sets its names in Instrument Serif instead. A
  site may choose another serif that beats Fraunces side by side in its
  world; it never sets a sans for reading.
- **The title has two parts: a plain lead and an italic turn in the
  tint.** The turn is lighter (weight 400 to 420) and coloured: "The log
  *is* the record.", "Start new, *drain old.*" Chapter titles repeat
  the shape, so each heading has one stressed phrase.
- **The lede is italic**, balanced, at 1.1 to 1.25 rem on a measure of
  about 33 rem, and it is the whole argument in two or three sentences.
- **Reading text is soft and quiet**: Fraunces at weight 380, "SOFT" 50,
  "WONK" 0, 17 to 19.5 px on a line of about 1.6, oldstyle figures in
  running text. Slack Water reads in Newsreader. Either way it's a serif.
- **Mono is furniture, small and tracked**: eyebrows, numerals, dates,
  labels and figures that are data, at 0.66 to 0.82 rem, often uppercase
  with 0.14 to 0.24 em of tracking, in the tint or muted. It never sets a
  heading, a row's title written in words, or a paragraph.
- **Figures that are results are display numbers**: lining and tabular,
  large, with the unit small, italic and in the tint ("23.6/s").
- **Rank by voice and weight before size**: roman against italic, cream
  against parchment, 560 against 380.

The engineered pages did the opposite: titles in mono at body size, every
label and status in tracked mono, the title broken huge over two lines with
an upright lede, and a sentence in place of a title ("Machines, Mycelium,
Network and Code are worth a look.").

## Colour

- **Turn the palette to the subject.** Each loved site has its own ground
  and tint: gold on forest green (the Ledger), azure on blue night with a
  warm cream for headings (the Crossing), teal from the kit's program hue
  (the Spring), buoy orange on bathymetric teal (Slack Water). With the kit,
  `--turn` rotates the grounds and `--tint` picks the emphasis hue
  (`sites.md`, The Kit); a site may also name its own pigments as
  Slack Water does, each with one job written beside it.
- **Text climbs in steps**: muted for captions and meta, a soft colour for
  prose, a lighter one for ledes, the strongest for headings. The heading's
  strongest is warm, never pure white.
- **The tint is rationed**: the title's turn, eyebrows, numerals, the
  crest, drop caps and ornament. Links are their own quiet colour with an
  underline that warms on hover. The accent, where it differs from the
  tint, is only where the keys are: the focus ring, the caret.
- **Every state is a colour and always a word**, the colours named in the
  world's terms (starting lavender and dashed, draining amber), never a
  legend to remember.
- **One faint radial glow at the top of the ground** gives the dark look
  depth; skip it in the light look, where it reads as a smudge.

## Space and Layout

- **A page to read is centred and composed.** Crest, eyebrow, title, lede,
  standing line and contents stack on one centre axis; the body below
  holds a measure of about 36 rem inside a column of 50 to 64 rem.
- **A site of several pages puts every page in sight**: two named groups
  under spaced mono labels, each a run of italic links (the Spring), with
  the hero split as claim beside proof from about 1040 px.
- **An interface hangs its marks in a gutter** so every title, heading and
  headline starts on one edge, places sit as a quiet row of words at the
  top, and detail opens beside the list without moving it (Slack Water).
- **One unit, one ladder.** Spacing is multiples of a 4 px unit, each gap
  above a thing larger than the gap below it. Generous air on a page to
  read; scanned density in an interface.
- **Hairlines and space, never boxes.** No card, no filled row, no pill,
  no shadow round content. A selected row in an interface shows by weight
  and a quiet ground, never a bright box with rounded corners.

## Marks and Ornament

- **The site's mark is drawn in its world**: a letter in a double ring between
  gold flourishes, a box over a draining box between flow lines, strata.
  It sits centred above the eyebrow at about 200 to 260 px wide on a page
  to read, and its simplest form is the favicon. A 20 px glyph in the
  corner is not a mark.
- **Ornament at thresholds and joins**: a joining stem or rule between
  chapters, a drop cap, a fleuron for list items (`\2766`), a sprout at
  the close. Thin strokes, the tint, the world's own shapes.
- **Marks for kinds of thing**, in their kind's hue, telling by shape as
  well as colour (Slack Water's urgent mark is a planing hull).

## Figures

- **One drawing carries the idea**, on or just under the first screen,
  inline SVG in the page's faces and colours.
- **Strokes by class, not inline colour**: a stem, a vine, a gilt line, a
  hot dashed path; records filled with an edge, the next one dashed.
- **Labels in two voices**: a name in italic serif, a note in small muted
  mono; on a phone the notes go and the names grow.
- **Captions are honest and italic**: what's drawn and what's measured
  ("The order is illustrative; the counts are the CI's.").
- **A stepped diagram** has Back, Next and Play, a count, and a caption
  that says each step in a sentence (the Crossing).
- **The proof can be the hero**: a real terminal replay beside the claim,
  captioned with its commit and linked to its raw log (the Spring).

## Motion

Things arrive once and settle: paths draw in, leaves bud, labels fade, on
arrival and never again. Nothing is scrubbed by the scroll, nothing loops,
nothing autoplays, and a page whose script doesn't run is complete
(`rulings.md`, Motion).

## Navigation and Keys

- **The way through is typography**: an italic line of numbered stops with
  the numerals in mono and the tint ("01 why deploys fail · 02 the one
  idea"), or the named groups of a several-page site, or a row of place
  words.
- **The keys are said once, quietly, inside the composition**: a line of
  keycaps under the contents ("j and k step from stop to stop, and ? lists
  every key"), the three-way switch small beneath it, centred. Never a
  strip across the top of the page, and never at the top right as chrome.
- **Folds are the reader's questions** ("Can I still deploy my working
  tree to staging?"), opened by one quiet mark that turns.

## Words

- **Titles have a turn**: three to six words, Title Case or a whole
  sentence with its full stop.
- **Eyebrows say what this is and for whom**: "the ledger · a design for
  its engineers", "the project · its deploys · the crossing".
- **The standing line is one sentence with a glyph**: a half-filled gold
  dot, where the work is, and when it was updated.
- **Each part ends on a lead that hands over**: "None of this is free. Here
  is what it costs."
- **It owns its limits** ("What the ledger won't fix on its own") and may
  close on a signature and a motto.

Everything else about words is `writing.md`.
