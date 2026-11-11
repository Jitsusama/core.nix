---
name: web-design
description: >
  How to build any website Joel sees or shares so it meets his taste without
  being told: the spirit and the reasons, the two kinds of site (a page to
  read and an interface to work in), the rules that held over dozens of his
  verdicts on real sites, what he turned down, the theme as a ready kit of
  CSS and fonts, and the checks to pass before anyone looks. Use before
  designing, building, restyling or writing any web page, site, web app,
  dashboard, status page, proposal, tour, teaser, docs site, changelog, HTML
  or CSS, a chart or diagram for a page, or the words on one.
---

# Building Joel's Websites

How a website should look, read and behave for Joel, drawn from his verdicts
on the sites agents built for him from August to November 2026: proposals,
status pages, tours, a docs site, a character sheet, a code viewer and two
team tools. It builds on his aesthetic, which comes first: read the
`aesthetic` skill and the design document beside it before this one. They
give the character (modernist, Nordic and Baja, with whimsy), the type and
space systems and the colour roles. This skill adds only what a browser
changes, and why.

Beside this file:

- `writing.md`: how the words on a site read, for a reader who wasn't there.
- `craft.md`: the measured facts about his faces and colours on the web, and
  the CSS that does each rule here.
- `checks.md`: what to measure and look at before anyone sees a site.
- `gate.js`: the probe that opens the gate before every publish, finding
  what a page lacks of its parts ([Every Page Has Its
  Parts](#every-page-has-its-parts)) and what its words get wrong.
- `layout.js`: the probe run after it, finding rows and folds that don't
  hold their layout ([Layout That Holds](#layout-that-holds)).

The theme itself, ready for a site, is the kit at `~/.local/share/jitsusama/web/`
([The Kit](#the-kit)). Never retype a colour, a size or a face.

- **Build to it, and judge by it yourself.** Don't hand a site back to Joel to
  look at, and never ask him to list its faults again: "I've done that before,
  and you keep missing the mark" (2026-09-22). A question this skill doesn't
  answer is a gap in it: once he answers, write the answer here, in core.nix's
  `modules/theme/web/skill/`.
- **Say what's unfinished.** Whatever fails a check below is shown as
  unfinished, never as done.

## What Matters, and How Much

Five things are gates, not polish. A page that fails any of them isn't
published, however good it looks, and none is traded for another or for a
deadline. A model reading this cold weighs them first, before the craft
that follows; `checks.md` runs them in this order before every publish.

1. **Accessibility, with WCAG 2.2 AA as the floor, measured.** A page some
   readers can't use is broken for them, whatever it looks like to the rest:
   "accessibility cannot be a third class person" (2026-10-22). It's proven
   by the browser's checks, never by eye.
2. **The writing.** His voice, plain words for a reader who wasn't there,
   Title Case or whole sentences for headings, Canadian spelling and no
   emdash (`writing.md`). A page that isn't understood on its first read has
   failed at its one job.
3. **The parts.** The mark, the favicon, a figure for each part that
   explains, the head's names and the twin
   ([Every Page Has Its Parts](#every-page-has-its-parts)).
4. **The ways to move.** omni-ref's key line, Keys list and three-way switch,
   a keyboard reaching everything, and nothing moving that the reader didn't
   move.
5. **The style.** The kit's faces, colour roles and space ladder, never a
   value retyped, and the layout's measure and edges.

Then the look, judged against [The Spirit](#the-spirit): the gates make a
page fit to show, and only the look makes it his.

## The Spirit

He has said these things to session after session, and is tired of it: "i'm
tired of repeating my desires over and over" (2026-11-07). They're why every
rule below exists.

- **The bar is a museum piece that works as hard as he does.** "I want
  something you could put up in an art museum that is also insanely
  functional" (2026-11-03), and of a site, "this should make a design god or
  goddess shit their pants in ectasy, while also being an insanely good UX"
  (2026-11-02). When he names Apple, he names the bar, never the look: the
  site built from Apple's white tiles, big radii and frosted bars "looks like
  it was made with fuckign bootstrap ... I could hvae done that design in my
  teens" (2026-10-20). Generic competence is the failure.
- **Type is the material, and the serif leads.** "serif or GTFO"
  (2026-11-03); "use great typography for styling, like, fucking art nuevo
  fucking amazing typographic design" (2026-10-29); "everything is a
  typographic joy" (2026-10-29). The page he called "the best design i have
  seeen you produce so far, i'm not saying to copy this, but you have to at
  least be that fucking amazing" (2026-11-02) was set entirely in Fraunces,
  with Monaspace only for labels, after a first draft in a sans was thrown
  out.
- **It's for a human brain that holds about four things.** A site has to be
  "approachable, readable, easily understood" (2026-09-03), "written in
  human" (2026-10-19), and must not show "no empathy for the human brain and
  how much context it can hold" (2026-10-01). When a page asks too much, "there
  are times when my brain shuts down" (2026-11-07). Density, length and jargon
  are the same fault: they spend the reader's attention on the page instead
  of on the point.
- **Accessibility is first class, and as beautiful as the rest.**
  "accessibility cannot be a third class person. it must be given the same
  attention as everything else, including the cosmetics of it, even any skip
  links" (2026-10-22); "anything accessible should be just as beautiful"
  (2026-11-02). People with motion sickness count, and so does layout that
  jumps, not only animation.
- **Whimsy is demanded, and never gaudy.** "whimsy is fucking important"
  (2026-11-02); "it needs to give joy" (2026-11-02); "all interactivity is
  functional or purely whimsical" (2026-10-22). Design is "a lot like
  architecture. you design the space around the flow of people that inhabit
  it ... with whimsy place purposefully where it will enhance the journey"
  (2026-09-27).
- **Fast, light and still.** "these pages need to be highly performmant and
  not a tax on a users machine" (2026-11-06). Nothing jerks, flickers, lags,
  waits or moves under the reader.
- **The present, never the history.** "I don't want the site to be a history
  lesson in making the site" (2026-09-03); "a holistic view of where we are at
  which communicates well without being a log book" (2026-11-06).
- **One whole, every part with the same care.** "any ui change must be looked
  at holistically" (2026-11-04); "make sure that every screen and screen
  component is given the same amount of care and attention" (2026-10-22). A
  section in a style of its own "does not fit well" (2026-11-06), and a fix
  that costs craft elsewhere is a regression: "some of the ideas on change are
  good, but they were not kept to our insanely high quality standard"
  (2026-11-05).

## Two Kinds of Site

Decide first which one it is, since they're judged differently. A page to read
is judged in one pass, top to bottom. An interface is judged over hours, under
the keys, while the data changes under it. An interface carries pages to read
of its own, a guide or a lexicon, and each is held to the first kind.

| | A page to read | An interface to work in |
| --- | --- | --- |
| What it is | a proposal, a status page, a tour, a teaser, a study, docs | a tracker, a tool, a sheet, a viewer |
| Its job | carry one argument or one picture of now, to a reader who wasn't there | let someone do their work all day without thinking about the page |
| Body type | Fraunces at 18 to 20 px, line 1.55 | Fraunces at 15 to 16 px, rows at 1.4; `data-kind="interface"` |
| Shape | one column, one measure, one story, folds for depth | places at the top, rows, a drawer for detail, keys for everything |
| Whimsy | pervasive on a showcase, at thresholds on a page read under pressure | in marks and thresholds (an empty list, a landing), never on the controls |
| Its worst failure | a fact dump, a history lesson, a hodgepodge | overwhelm, relayout, flicker, hidden places |

## The Kit

The theme module writes the theme for the web on every machine, at
`~/.local/share/jitsusama/web/`. Copy the folder whole into the site (as
`kit/`), and load it before the site's own CSS:

```html
<link rel="stylesheet" href="kit/theme.css">
<link rel="stylesheet" href="kit/base.css">
<link rel="stylesheet" href="site.css">
<link rel="preload" href="kit/fonts/fraunces.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="kit/fonts/fraunces-italic.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="kit/fonts/monaspace.woff2" as="font" type="font/woff2" crossorigin>
```

Preload the faces the first screen draws: the heading's italic turn and a
label put all three on most of them.

- `theme.css` is written from the theme, so it's never edited: every colour
  role (`--background`, `--text`, `--muted`, `--strong`, `--accent`, the glyph
  hues `--action`, `--program`, `--machine`, `--person`, `--voice`, the
  states) for both looks, dark and light; the faces (`--serif`, `--mono`,
  `--mono-features`) with stand-ins metric-matched so nothing moves when the
  fonts arrive; the fluid type steps (`--step-small` to `--step-hero`); each
  level's settings (`--title-weight`, `--words-soft`, `--label-tracking`);
  the space ladder's rungs (`--tie`, `--mark`, `--apart`, `--items`,
  `--groups`, `--inset`, `--frames`, a fluid `--screen`); the radius and
  border; and the motion (`--ease`, `--leave`, `--arrive`, `--large`).
- `base.css` sets the type as a typesetter would, one focus ring, one
  selection, links, code, tables, disclosures, fields, and what the reader's
  system asks for. Override it only for a reason you can name.
- `fonts/` holds Fraunces upright and italic and the theme's Monaspace, cut to
  what a page in English uses: about 125, 150 and 60 KB.

A site names properties, never a value. Where it needs one the kit lacks,
derive it from the roles (`color-mix()` of two roles) and say why in a
comment; if every site would need it, it belongs in the kit, in core.nix's
`modules/theme/web/`.

**A site's own tint.** Each site may have its own feel within the palette,
as two of his favourites did. Two properties do it, and nothing else should:

- `--turn` turns the grounds, the text and the accent round the hue wheel, in
  degrees, keeping their lightness, so contrast holds: `:root { --turn: 85 }`
  gives a harbour blue in place of the forest green ("with a blue tint instead
  of green", 2026-11-03).
- `--tint` is the hue the site marks its emphasis in: the italic turn of a
  heading, eyebrows, drop caps, ornaments, a link's underline on hover. It's
  gold (`--action`) unless the site takes another glyph hue as its own:
  `:root { --tint: var(--program) }` for sea glass. Gold then keeps its own
  meaning, something running or partly so.

**Dark and light.** Both looks are in `theme.css`, chosen by the reader's
system, dark when it says nothing. Give every site the three-way switch he
asked for, "a 3way with icons toggle, auto/dark/light, and make sure it looks
spiffy" (2026-10-02): Light, Match System and Dark, in the page's head,
applied before the first paint so nothing flashes (`craft.md` has it). A site
that is dark only sets `data-theme="dark"` on `<html>`.

**The page to build like is omni-ref**, the proposal headed "From one ref to
many.": "i like how [it] updated itself with different light styles and the
? help, i think that should be good general guidance for any page we create"
(2026-11-08). Every page, either kind, does what it does:

- **One quiet line under its head says how to move**: "j and k step from part
  to part, and ? lists every key", in Note, muted, the keys drawn as keycaps.
- **? opens a Keys list** and closes it again: j next part, k previous, g
  first, G last, ? this list, Escape closing it too. An interface's list adds
  its own keys after these.
- **The three-way switch sits in the head**, under that line, Match System
  chosen until the reader picks, and remembered across visits.

## Every Page Has Its Parts

A page built from this skill by a model that drafts plainly kept its type and
its keys and dropped everything drawn: no figure, no favicon, no mark in its
head. Set beside the page it followed, "all imagery is gone, no icon, a lot
missing" (2026-11-09). Words are what a model writes first, and pictures are
what it leaves out, so the parts are listed here, whoever builds the page,
and `gate.js` finds each one missing before it's published (`checks.md`).

- **The site's mark, drawn once and shown twice.** A small drawing in the
  site's own metaphor (a bottle, a crest, a spring), inline in the masthead
  above the title, and again as `favicon.svg`, so the tab says which site it
  is. The favicon is the mark's simplest form on a 32 grid at 2 px strokes,
  since a hairline vanishes at 16 px, in the tint of each look (`craft.md`).
- **A head that names it wherever it's shared**: a `<title>` that is the
  pithy title, the lede as `description`, `og:title`, `og:description` and
  `og:type`, and a `theme-color` per look, so a link in a chat or a tab
  strip reads as the page does.
- **A figure for every part that explains.** Each part under a heading holds
  a drawing or a table that carries its point ([Figures carry the
  explanation](#a-page-to-read)). A part with nothing to picture, a
  changelog or a list of links, says so on its section in `data-figure`,
  as a suppression says why, and every page to read draws at least one.
  A dated record kept as written says why once, on its `main` or its
  `article`, rather than gaining a drawing only to have one.
- **Ornament at the joins**: a fleuron or rule drawn between parts, a drop
  cap or a crest at the threshold, in the story's own metaphor. The probe
  can't judge these; holding the page beside its model can ([Judging
  Without Him](#judging-without-him)).
- **The twin for agents**: `index.md` linked as the page's Markdown
  alternate, and `llms.txt` at the root.
- **The key line, the Keys list and the switch**, as omni-ref has them
  (above), with a skip link the first thing focus reaches and one `<h1>`,
  the title.

An interface keeps every part but the figures, which its rows and charts
replace.

## What Holds on Every Site

Each rule says why, in his words where he said it.

### Type

- **Fraunces for every word to read and every heading; the theme's Monaspace
  for names that are identifiers (a file, a branch, a host, a command), figures
  that are data, code, keys and labels; no sans.** A row's name is Monaspace at
  the Name level's weight when it's an identifier, and Fraunces when it's a
  title written in words. "I
  don't like seeing sans-serif fonts ... generally, not a fan of sans-serif"
  (2026-11-04). A sans earns a place only by beating these two side by side.
  Figures that are data are Monaspace because Fraunces has no tabular or
  old-style figures at all (`craft.md`).
- **Use Fraunces' axes, not just its letters.** Reading text is soft and
  straight (SOFT 50, WONK 0, weight 380); headings are softer and wonky (SOFT
  100, WONK 1), which at display sizes gives the Art Nouveau leaning n, m and
  h. That switch is most of what made the best page sing. Let the browser set
  optical size from the size drawn.
- **Rank by weight first, then size.** "font weights and spacing for
  hierarchy" (2026-11-05); "use weights and not size" (2026-11-07). At most
  four levels on a screen and three in a row, as the design document says.
- **A heading is a title in Title Case, or a whole sentence with its
  punctuation, never a sentence without it.** "i generally prefer title case
  for titles, i hate thigns that look like sentences without punctuation"
  (2026-10-26); "headings/subheadings/actions should be title case"
  (2026-10-21). So "Measured, Not Promised" or "From one ref to many.", never
  "From one ref to many". Actions and buttons are Title Case.
- **The best pages give each heading two parts: a plain lead and an italic
  turn in the site's tint.** "Two weeks *of fires*", "World *in a bottle.*"
  This is the web's one exception to colour staying off words: the display
  italic of a heading, a drop cap and an eyebrow may wear `--tint`. Body text,
  links and labels never do.
- **Set it as a typesetter would**: `text-wrap: balance` on headings and short
  blocks, `pretty` on paragraphs, hanging punctuation, real quotes and
  apostrophes, hyphens only on narrow screens, Canadian English (`lang="en-CA"`).
  `base.css` does all of it.

### Measure, Grid and Space

- **Prose holds a measure of 45 to 75 characters: `max-inline-size:
  var(--measure)`, 30em in Fraunces, never `65ch`**, which runs to about 90.
  "we still have so little horizontal space for text" (2026-09-22) on one
  side; on the other, "only a few words of text in a row is shit. What
  newspaper layout editor would approve thta?" (2026-10-20). No column of
  sentences narrower than `--measure-least` (about 45 characters, or 500 px):
  below it the layout changes shape, never squeezes.
- **One column, one set of edges, one container width on every page.** Prose,
  figures, code and tables share the column's edges; a wider thing takes a
  named wider line for a reason, never its own width. "text widths that are
  incongruent with their containers" (2026-09-22); "why is the table so much
  wider than the main text" (2026-09-22); "I also don't like the different
  viewport widths on the landing page and the other pages" (2026-09-10).
  Columns come from the measure, not from a count: "the column count has to
  adjust to screen sizes" (2026-10-20). Use named grid lines, subgrid for
  values that line up down a list, container queries for parts that reflow.
- **Space is hierarchy.** Every gap is a rung of the theme's ladder, each wider
  than the one inside it, set as `gap` on the parent so it belongs to the level
  it separates. "around the heading there are so many types of whitespace
  choices that seem chaotic" (2026-09-22); "things are a little too tight. our
  aesthetics needs to consider whitespace more and how it relates to hierarchy"
  (2026-11-05). On a narrow screen a frame gives up its inset and becomes the
  page rather than squeezing its text.
- **Alignment is exact.** Equal inset on both sides; every mark centred on its
  words' capitals; a number on the line of the thing it measures. "definitions
  and numbers don't vertically align with the text they identify" (2026-09-10);
  "glyphs and text needs to be pleasantly aligned, nothing off the proper
  center line" (2026-10-21). Flex centres on the line box, not the ink, so
  check it (`craft.md`).
- **Nothing wraps, breaks or spills in the wrong place.** A word or a token
  never breaks; a title is cut at a whole word; dates, ids and numbers with
  units never split; no text runs out of its box or over another at any
  width. "wrapping in shitty spots" (2026-09-10); "text overflowing into other
  text" (2026-11-07).

### One Site, Thought Whole

"the website has not been holistically re-thunk. it feels like a mash of
things and good UX has not been considered consistently in so many ways. it
feels like frankenstein right now" (2026-11-11).

- **A site that grows is rethought as a whole, never appended to.** A new
  section is the moment to read the whole site cold and reshape it, not a
  block added at the end or wedged between two others. Its structure
  follows the reader's journey: what they need first, what next, and where
  they'll want to come back to, walked as a reader who wasn't there. A part
  that no longer fits that walk moves, merges or goes, however recent.
- **One set of components, used the same way everywhere.** Each kind of
  thing (a step, a figure, a fold, a callout, a table, a player) is drawn
  by one component, with one look and one behaviour on every page and in
  every section, and its keys, controls and words mean the same in each.
  A section that needs more changes the shared component for all of them;
  a new component is only for a new kind of thing.
- **Consistency is checked, not hoped for.** Before publishing a change to a
  site of several sections, list each kind of thing it shows and the forms
  each takes; a kind in two forms is a fault to fix (`checks.md`).

### Layout That Holds

"we can't have cards beside each other with different heights looking like
shit", "we need adaptive layouts, we also can't have opening a disclosure
causing the page to reformat" (2026-11-10). `layout.js` measures all three.

- **Things side by side end level, by layout.** Steps, panels or
  comparisons set beside each other are one grid row: each stretches to the
  row's height, and their parts (state, title, figure, line, fold) share
  rows through subgrid, so every title starts on one line and every fold on
  another. Never padding, a fixed height or a spacer to even them out.
- **Opening a fold never reflows the page.** Nothing above it moves and
  nothing beside it resizes, jumps or reorders. Evidence opens in place,
  growing only itself downward, so only what's below it in its own column
  moves down. A fold inside one of several things side by side would
  stretch the row, so it opens in a side sheet instead: from the right on a
  wide screen, the full screen on a phone, focus going in and coming back
  to its summary on close.
- **Layouts adapt; they never squeeze.** Columns come from the room there
  is: one on a phone, two on a tablet or a laptop, three on a wide screen
  where that reads better, and none under 256 px. Each part changes shape by
  its own width through a container query. A table stacks into labelled
  rows when narrow, and a figure is drawn wide and narrow.
- **Read the whole page cold before every change, and redesign what
  doesn't come across.** "we need to constantly evaluate whether it is
  holistically communicating well and not be afraid of redesigning parts or
  all of it at any time" (2026-11-10). If the big picture doesn't land at a
  glance, a part or the whole page is redesigned, however much work it
  undoes. Fewer words isn't the measure; a reader getting it is.

### Colour and Surfaces

- **Roles, never colours,** and colour on glyphs, marks and bars, never on
  words (but for the heading exception above). The accent is only the focus
  ring, the one place the keys are. Nothing is told by colour alone: "right
  now people have to remember colour associations" (2026-09-03). Every colour
  in a chart means what it means everywhere else: "it makes no sense what the
  gold on a node means" (2026-10-22).
- **No cards, pills, chips, badges, banners, frosted bars or boxes round
  prose.** Use space, a hairline, a heavy rule for a section or weight. "way
  too many visual elements ... those pills help no-one" (2026-09-10); a page
  built as rounded white tiles was "bootstrap" (2026-10-20). A section in a
  different style "does not fit well" (2026-11-06): a new part is set with the
  page's existing parts.
- **Tables for one-line cells only.** A row whose cells hold sentences becomes
  a title with labelled facts under it. "the tables on that site are looking
  like shit" (2026-10-19), of a table of sentences. Tables are welcome for
  comparisons: "clean illustrations and tables to visualize stuff" (2026-11-07).
- **No chrome the reader didn't ask for.** "I don't want a lot of visual chrome
  on the screen" (2026-09-08); "The footer at the bottom of every page adds
  nothing" (2026-09-28); a masthead that doesn't eat the fold.

### Marks

- **A mark for each kind of thing, wherever there's room for one.** Of the
  desktop's bar he said "i want icons to be able to be there as much as
  possible" (2026-10-28), and the same holds on a site: a person, a place, a
  state, a kind of item each has its mark, from one set (the theme's icon
  font, or one drawn set), never mixed.
- **One optical size**: every mark's width times height, square-rooted, is
  the same (15 px at the desktop's scale, about 1.1em of the text beside it),
  so a wide mark and a tall one look equal.
- **In its kind's glyph hue** (`--program`, `--person`, `--machine` and the
  rest), centred on the capitals of its words, and a row's mark in a column of
  its own, so the words all start on one edge.

### Disclosure

- **A fold is the reader's question, where they'd ask it, worth opening.** Its
  summary says what's inside, in words a reader would use, with one quiet,
  unmistakable mark that turns as it opens, and no "open" or "close" words.
  "it is not obvious that those branches can be opened, they are also often
  aligned weird" (2026-10-29), then "we don't need the open this branch link
  or fold it back with the + and x icon there" (2026-10-29). "make expanding
  stuff meaningful and useful" (2026-10-20). What's hidden is hidden whole:
  no cut-off preview.
- **Native `<details>`, growing open rather than jumping** (`base.css`), so the
  keyboard, a screen reader and find-in-page all work. Opening one never shifts
  the text beside it: "opening and closing disclosures causes text to shift
  which is jarring" (2026-09-28). Where it would, among things side by side,
  it's a side sheet ([Layout That Holds](#layout-that-holds)).

### Motion

- **Things arrive once and settle; nothing loops at rest.** Fluid and timed
  well, on the theme's curve and durations (`--ease`, `--arrive`).
- **Nothing moves under the reader unless the reader moved it.** "People with
  motion sickness will have a very hard time coping with this site as is, not
  due to animations, but due to responsive layout causing screen transitions
  or drawer pull outs to relayout a lot of the page" (2026-11-02). A drawer
  slides over or into room already left; the scrollbar's gutter is kept;
  fonts swap into matched room; a live change never moves what's in view.
- **A scroll animation is visible in its first frame, finished while still in
  plain view, paced by the story, and rewound by scrolling back.** "i need
  animations to start at some state that is visible immediately when i scroll
  into them" (2026-10-22); "make sure that scrolling back up reverses
  animations" (2026-10-22); "our main front page animation goes so fast with
  natural scrolling that most detail is lost" (2026-10-22). Never take the
  reader's scroll: no snapping that jumps past what they came for.
- **Numbers never move.** A figure changes in place, at once; no count-ups,
  which show false values on the way.
- **Less motion asked for means still and whole**, and a page whose script
  doesn't run is still complete.

### Replays

"playback needs to have speed, progress, and i should be able to scroll up
to see previous things that were outputted. sometimes it is too fast and
sometimes it is too slow" (2026-11-11); "I think we need a .25 speed as
well, it still feels very fast, or maybe even just some CSS transitions to
slow down how the text appears. Anyone that gets dizzy easy would be
destroyed by it" (2026-11-11).

- **Reach for a replay where a reader learns by watching output unfold.**
  "i think the concept of terminal replays is something to put in there in
  general, it is great medium for certain things" (2026-11-11). A flow at
  the command line, a build or a deploy running, an agent's run, a
  terminal before and after a change: where the order and the pace of what
  prints is the point, a replay shows it as no still can. Where only the
  end state matters, a still of it, or a table, says it quicker.
- **Make it well.** Record a real run, or script one that types and prints
  on a timed plan, so it can be made again when the tool changes; either
  way the output is real, never mocked. Trim it to what teaches, cut each
  wait to a beat, and keep secrets, paths, ids and machine names out of it.
  Name its chapters in the cast (a step, a stage, a verdict), set its
  columns to fit the measure in the kit's mono, and give it the controls
  below, a reduced-motion form and a twin of its transcript.
- **A replay that's evidence is never trimmed.** Where it proves a run
  happened as the page says, it stays whole and in real time: no output
  cut, the printed timings as they were, and gaps over 2 s shortened only
  in playing, with a note on the frame that says so. A secret in it is
  masked where it stands, marked as masked, never cut out.
- **A terminal replay is a player the reader drives.** It opens paused, at
  1×, and plays only when asked. Its speed always shows beside it and
  changes in steps (0.25×, 0.5×, 1×, 2×, 4×), so a reader slows what rushes
  and hurries what drags; a long idle gap in the recording is capped, never
  sat through.
- **Output arrives gently.** New lines fade in on the theme's `--arrive`,
  and the scroll that follows them glides rather than snapping, so text
  never jumps under the eye. With less motion asked for, nothing fades or
  glides: each step's output shows whole, at once.
- **A step mode that waits.** Switched on, it plays one step (a command
  and what it printed) and stops until the reader asks for the next, so
  anyone can go at reading pace, however fast the recording ran.
- **Its progress is a bar the reader can seek**, with a marker at each
  chapter, and beside it the chapter it's in, by name, the time played and
  the whole. It counts only time the page is seen: a reader who leaves the
  tab comes back to where they left it, never further on. A replay that's
  evidence reads one clock, the recording's, in its chapters and its
  readout alike, so a step's time matches what it printed; how long it
  takes to play is said once, in its head.
- **Scrollback through everything printed so far.** The reader scrolls up
  in it to read what went by, whether it plays or is paused, and new output
  never pulls them back down; a quiet "latest" returns them.
- **Its keys, while it has focus, are the player's and Vim's**: Space
  plays and pauses, h and l step back and on, [ and ] jump chapter to
  chapter, < and > change speed, s turns step mode on and off, n plays
  the next step, k and j scroll its output, and gg and G go to its top
  and its latest line, all listed in Keys. How
  it's built is in `craft.md`.

### Keys

- **Everything works from the keyboard, and a Vim user feels at home.** "we
  need to think about insane accessibility, mobile support, and vi support
  together" (2026-09-05). Tab reaches each list once, and j and k move in it;
  Enter goes in and - goes up; / finds. On an interface, : opens the one
  palette of everything, each with its key, as the desktop's command line,
  and Ctrl+K (Cmd+K on a Mac) opens the same palette, the web's own key for
  it.
  Space ? and F1 open the keys, as on the desktop. On a page to read, j and k
  step part to part, g and G go to the first and last, and ? lists them.
- **? opens the keys too, a departure from Vim**, where it finds upward: a
  site's / narrows without a direction, which leaves ? free, and he reaches
  for it there: "? doesn't seem to work on every page and works differently
  on both systems" (2026-11-05). On a page to read, where Space scrolls, ? is
  the key.
- **Escape closes what a mode opened, as Vim's closes its completion menu**:
  the palette, a menu, a hover card, a side sheet of evidence, and a question
  it answers "no" to. A sheet of evidence holds nothing to lose, like the Keys
  list, so Escape closes it too. It
  never closes a drawer or a dialog holding work, which close with their own
  control or :q; it never leaves a page, steps a level, clears work or quits.
  That's the desktop's rule ("fuck escape quitting", 2026-11-05), and it
  meets WCAG 1.4.13, which asks only that hover and focus content can be
  dismissed. It departs from the many web apps whose side panel closes on
  Escape, on purpose: work held in a panel is never lost to a stray key.
- **Single letters never fire in a text field, and can be switched off**, for
  screen readers and speech input (WCAG 2.1.4).
- **Keycaps follow the platform**: Ctrl off a Mac, ⌘ on one. "it should detect
  if you are not on mac and show a ctrl for linux" (2026-11-05).

### Access, Speed and Agents

- **WCAG 2.2 AA is the floor, with APCA beside it**, for every disability, on
  a phone and everything between: "the latest WCAG is like a minimal bar.
  Screen readers, assistive device users, keyboard only users, colour blind of
  various sorts" (2026-09-09). One focus ring everywhere, which never moves
  layout ("we now have different focus highlighting than the rest of the site,
  FFS", 2026-09-08); a skip link set as carefully as the masthead; targets of
  24 px, 44 on touch; content at 320 px wide with no sideways scroll; 200% and
  400% zoom; forced colours and more contrast.
- **Light on the machine.** No request to anyone else's server: fonts,
  scripts and images come from the site. Budgets in `checks.md`. Idle means
  zero: no frames, timers or polling while nothing changes, and a background
  tab checks but draws nothing.
- **A site for people has a twin for agents.** "it needs to not look like shit
  and be agent friendly" (2026-10-19): the facts as Markdown and JSON beside
  the page, an `llms.txt`, stable anchors, and error text written as people
  will paste it. "human and AI agent empathy needs to be a critical tenet"
  (2026-11-07).

## A Page to Read

- **An argument with data under it, not data.** "this needs to be readable,
  not a fact dump. we need supporting data, but the data is not the story
  itself" (2026-09-15). It's "completely self contained" and "a proposal, not
  a ask" (2026-09-15): it states its choices, never poses its owner's
  decisions as questions. A converted document is never the page: "No one is
  going to read all of that ... why the fuck did you use pandoc?"
  (2026-09-10). Link the depth for those who want it.
- **A walk, in the order a sceptic asks.** Open with what it ends for the
  reader, in their own terms, then the idea, how it works, what it costs, the
  evidence, where it stands and what it won't fix. Each section opens with its
  point and ends on the question the next answers. "we need to connect with
  the reader, bring them in, and help them follow a chain of thought. we don't
  want to assume they have context that we have not shared" (2026-09-10);
  "the entire flow of the page builds up and helps solidify understanding. i
  need it dressed to impress and to be easy to skim through as well"
  (2026-11-04). Reading only the headings gives the argument.
- **A pithy title and a first screen that carries the whole idea.** Three to
  six words; a lede of one or two short sentences; then the presence. "fuck,
  the site went into novel mode and lost its pithy title" (2026-11-06).
- **One focal point, made of type.** A huge plain sentence, perhaps one huge
  number with its comparison, a name and a voice line in the masthead, heavy
  rules between sections: "I insanely love the visual presence of it"
  (2026-10-20). Presence first; then make it easy to process under it.
- **Figures carry the explanation, and words caption them.** "don't be too
  miserly with diagrams, and don't be too verbose with prose" (2026-09-22);
  "the visuals do not connect with the narrative" (2026-09-10) is the fault to
  avoid. Inline SVG in the page's own faces, roles and hairlines; labels
  placed, never only on hover; drawn again for a narrow screen rather than
  shrunk; a caption saying what it shows; a table or list with the same facts
  beside it; text in it edited like prose (2026-10-22).
- **Ornament is drawn, thin and part of the story's own metaphor**, at
  thresholds and joins: a crest, a vine between chapters, a fleuron, a drop
  cap. "some ornaments and serif is desired" (2026-11-05). Whimsy is
  pervasive on a showcase and kept off the facts on a page read under
  pressure.
- **The present, with the past in its place.** The body says where things
  stand now; nothing in it is dated but one true "updated" time. A changelog,
  if any, is grouped by day, newest first, each day a terse overview with its
  entries folded: "what changed on the site is taking over the site"
  (2026-11-07). A living page keeps a fixed structure and a word budget, and a
  new result changes a number or moves an item rather than adding a
  paragraph: otherwise it becomes "a hodge podge of loosely organized ideas"
  (2026-11-07). Every part agrees with every other: "parts of it are in
  different states of agreement with the rest" (2026-11-06).
- **Posted early and kept current.** "posted quick, eveen if it barely has
  anything on it" (2026-11-05), then updated as the work moves, and readers
  told when the picture changes (a notice, a count in the tab's title),
  never when only a typo moved.

## An Interface to Work In

- **Still water.** Nothing under the reader moves unless they moved it: a view
  switch, a drawer, a teammate's edit or a new build. Redraw only what
  changed, measure before painting, keep the cursor on the same item at the
  same place, batch arrivals, and let a new item that would land in view wait
  behind a quiet "one new". The cursor moves at once, with no transition. "i
  get a lot of flashing ... profile pics flash" (2026-11-05). A new build
  never reloads a tab under its reader, and old tabs keep working.
- **Density between two failures, judged by measurement.** "wastes way too
  much horizontal space. you have way over-corrected" (2026-11-05) and, an
  hour later, "things are a little too tight". Rows are scanned, prose is
  read: a row keeps its facts beside its title, not at the window's far edge;
  prose keeps its measure. One row grammar for the same thing everywhere.
  Teaching aids, key hints and legends sit a step away, shown on approach, not
  at rest on the working surface. The one key line in a page's head is no
  such hint: it's said once, above the work and apart from it, so a reader
  who wasn't told learns ? exists, and it wins over this rule there alone.
  Beside a row, a chart or a control, a hint still waits to be approached.
- **Layers kept apart.** Places always show, at the top, at every width, never
  behind a menu or in a bottom bar: "ANY WINDOW WIDTH!" (2026-11-04). A way of
  seeing (list or chart) is a layer below places, secondary pages a quieter
  one, and detail opens in a drawer with its own room, never in a popup over
  content: "hovering on anything puts shit on top of other UI elements"
  (2026-11-04). The heading you're in always shows. Every view links to every
  other, and the way back names where it goes.
- **One palette, one meaning per key.** The palette (: or Ctrl+K) is the only
  popup, and the keys (? or Space ?) open on every page; a key means the same
  in every view.
- **Acts in one order, and nothing assumed.** In every list of acts, the one
  that does the thing is at the top, what ends something near the bottom, and
  Copy last. No state is assumed at load (a filter, a selection, a scroll),
  and every place opens on its first row.
- **Live and present, gently.** Who is here and what they're editing, with
  nothing flashing: presence that arrives on the first beat and leaves after
  missed ones; an edit of someone else's never overwrites yours. "showing who
  is looking at a thing at the same time as you is also awesome"
  (2026-11-02).
- **Words that work.** A domain word keeps its character and its meaning is a
  hover, focus or tap away, in clear space; every number leads to what it
  counts; people are shown as people, with full names a step away. One name
  for each thing, never a synonym, from a glossary the site keeps. Nothing
  waits on a person visiting a page: "I never want to poke users to do
  things, that is horrific" (2026-11-07).
- **Whimsy in marks and thresholds**: an empty list, a landing, someone
  arriving, a chart's islands. Never on the controls, and a change for meaning
  never costs it: "it lost a lot of whimsy" (2026-11-05).

## What He's Turned Down

Each is a verdict on a real site; the rule it gave is above.

- A converted document as the site, and a page that narrates its own making.
- Apple's surface copied as the bar: white tiles, radii, frosted bars, pulses.
- Columns of a few words a line, and tables of sentences.
- Pills, chips, card grids, legends a reader must remember, verdict tags.
- A wall of components with no walk through it, and "the design links at the
  end are a garbage bag" (2026-09-10).
- Agents' code on the page: stream numbers, ids, "6a" suffixes, file names,
  paths, their own metaphors.
- "novel mode": sentences for titles and paragraphs for a lede, from a request
  to be clearer. Clear means plain and short together.
- A page that grows by accretion until it's a hodgepodge, and a changelog that
  takes it over.
- A page of words alone, with no figure, no favicon and no mark: "all imagery
  is gone, no icon, a lot missing" (2026-11-09).
- A section in a different style, and text running over text.
- A site grown section by section until "it feels like frankenstein right
  now" (2026-11-11): parts that each work, in no one design, with good UX
  "not been considered consistently in so many ways".
- A terminal replay with no speed, no progress and no way back: "sometimes it
  is too fast and sometimes it is too slow" (2026-11-11).
- A command-line flow told in still screenshots or prose, where a replay
  would have shown it: "it is great medium for certain things" (2026-11-11).
- Replayed text that snaps onto the screen as fast as it ran, with nothing
  slower than half speed: "Anyone that gets dizzy easy would be destroyed by
  it" (2026-11-11).
- Branches that don't look openable; "open" and "close" words on them.
- Scroll animations that finish off screen, rush past, or snap past the text.
- Counting numbers, and a freshness time that measures publishing, not facts.
- A generic app shell, "too corporate and the UX is weeak. And totally not
  enough design or typography magic" (2026-11-02).
- Navigation at the bottom, places in a menu, hover cards over content, help
  text everywhere, flashing presence, screen jumping between views.
- A focus look, hover or selection that differs from the rest of the site, or
  grows a column.

## Judging Without Him

- **Gates aren't taste.** A page that passed AAA, a keyboard walk and every
  width still "looks like shit" (2026-09-10). Pass the checks, then look at it
  whole, at every width, every fold open, against this skill and the
  aesthetic skill's questions.
- **Turn each complaint into a measurement** before saying it's fixed, and
  keep the measurement as a check, so the fault can't come back: the short-line
  count, the overlap probe, the still-water sweep (`checks.md`).
- **Fix the concept before the pixels.** When two rounds of polish don't
  register, the fault is structural: the focal point, the pacing, the type's
  roles. "having a hard time recognizing any improvements" (2026-10-21).
- **Judge the whole page after every fix.** One site swung from crowded to
  cramped in an afternoon because each fix was checked on its own and nobody
  judged the whole page. Every update starts with a cold read of the whole,
  and a part that doesn't come across is redesigned, not patched
  ([Layout That Holds](#layout-that-holds)).
- **Hold it beside its model.** Open the page beside the one it's built like
  (omni-ref, his best page of its kind, or its own last good version), part
  by part, and list what that one has that this one lacks: a figure, an
  ornament, a mark, a fold. Add each, or say why it's gone. A probe finds
  a part missing; only this finds one done thinly.
- **Readers before Joel.** Cold readers who weren't there say in a sentence
  what each section means and where they stumbled (`checks.md`). They find
  words; they don't find space or craft, so they never replace looking.
- **Don't polish for an empty room.** Ship small and often while the concept
  holds; stop and rethink when it doesn't.
