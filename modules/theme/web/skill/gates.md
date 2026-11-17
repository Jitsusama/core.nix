# The Gates

What a page must pass before anyone sees it. Load this at verification,
after the design is drawn and judged beside its model, never as the way a
page gets designed. A page shaped by these gates alone is fit to show and
still not his: on 15 November 2026 five pages passed them and he called
every one engineer-designed. `checks.md` runs them, with the probes beside
this file.

## What Must Pass

Five things are gates, not polish. A page that fails any of them isn't
published, however good it looks, and none is traded for another or for a
deadline. They are checked last, once the page is designed, and
`checks.md` runs them in this order before every publish. A gate that a
design fails changes the design's detail, never its world: fix the
contrast of the tint, not the tint into the kit's default.

1. **Accessibility, with WCAG 2.2 AA as the floor and AAA wherever text
   allows, measured, and every site worked by keyboard alone.** A page some
   readers can't use is broken for them, whatever it looks like to the rest:
   "accessibility cannot be a third class person" (2026-10-22). "I don't want
   to have to click. i want to support clicking, but all sites need to be
   able to be intuitively navigated by keyboard and meet insane levels of
   WCAG AA+ accessibility" (2026-11-14). It's proven by the browser's checks
   and by a cold walk with the keyboard alone, never by eye.
2. **The writing.** His voice, plain words for a reader who wasn't there,
   Title Case or whole sentences for headings, Canadian spelling and no
   emdash (`writing.md`). A page that isn't understood on its first read has
   failed at its one job.
3. **The parts.** The mark, the favicon, a figure for each part that
   explains, the head's names and the twin
   ([Every Page Has Its Parts](#every-page-has-its-parts)).
4. **The ways to move.** A key line, a Keys list and the three-way switch,
   set into the page's own composition as the Crossing sets them (under the
   contents, centred, quiet), never as a bar of chrome; a keyboard
   reaching everything, and nothing moving that the reader didn't move.
5. **The style.** The theme's colours and a space ladder, never a value retyped,
   and the layout's measure and edges. The faces and the tint are the
   site's own choice within `language.md`.

The gates make a page fit to show. Only the design makes it his, and that
was judged before this file was opened (SKILL.md, The Process).

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
  explanation](rulings.md#a-page-to-read)). A part with nothing to picture, a
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
- **The key line, the Keys list and the switch**, as `sites.md` says, with
  a skip link the first thing focus reaches and one `<h1>`,
  the title. The skip link is the kit's, `<a class="skip" href="#main">`
  from `base.css`, a whole target tall; a site never draws its own.

An interface keeps every part but the figures, which its rows and charts
replace.

## Judging Without Him

- **Gates aren't taste.** A page that passed AAA, a keyboard walk and every
  width still "looks like shit" (2026-09-10), and so did the Code page,
  every part of it passed and never the whole (2026-11-15). The first-sight
  walk asks of the whole what no probe does (`checks.md`). Pass the checks, then look at it
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
  ([Layout That Holds](rulings.md#layout-that-holds)).
- **Hold it beside its model.** Open the page beside the one it's built like
  (the loved site in `examples/` it followed, or its own last good version),
  part by part, and list what that one has that this one lacks: a figure, an
  ornament, a mark, a fold. Add each, or say why it's gone. A probe finds
  a part missing; only this finds one done thinly.
- **Readers before Joel.** Cold readers who weren't there say in a sentence
  what each section means and where they stumbled (`checks.md`). They find
  words; they don't find space or craft, so they never replace looking.
- **Don't polish for an empty room.** Ship small and often while the concept
  holds; stop and rethink when it doesn't.
