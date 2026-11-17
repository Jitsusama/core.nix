---
name: web-design
description: >
  How to design any website Joel sees or shares so it's as good as the ones
  he's proud of: a way of thinking and a process, drawn from four of them
  (a proposal, a tour, a site of several pages and a tracker), with a design language,
  worked examples, his rulings, the kit and the gates beside it, each
  loaded when the task needs it. Use before designing, building,
  restyling or writing any web page, site, web app, dashboard, status
  page, proposal, tour, teaser, docs site, changelog, HTML or CSS, a chart
  or diagram for a page, or the words on one.
---

# Designing Joel's Websites

Joel is proud of four sites, studied here under made-up names: a proposal
(the Ledger), a tour (the Crossing), a site of several pages (the Spring)
and a tracker (Slack Water). Each was designed. Each has a world of its own (a book with a
seal, a harbour at night, water under stone, slack water where rivers
meet), a first screen that carries the whole idea in type and one drawing,
and a voice. On 15 November 2026 five newer pages, built from a long list
of rules on one shared kit, all came out the same: left-aligned, a tiny
mark, the key line as a bar of chrome, mono labels everywhere, rows in
filled boxes. He said they looked designed by an engineer, and asked
for this instead: a way of thinking first, and resources pulled in as
needed rather than a rulebook to drown in.

So this file teaches how to think about a page. Everything else is a file
beside it, loaded when the step below names it.

## Think First

Answer these before writing any HTML, in a sentence each, and keep the
answers: every later choice is judged against them.

1. **What is it for?** One argument, one picture of now, or a place to work.
   A page to read carries one idea to a reader who wasn't there; an
   interface lets someone work all day without thinking about the page.
   Decide which, because they're designed differently (`interface.md` for
   the second).
2. **Who reads it, and in what moment?** A CEO with two minutes, an
   engineer weighing a proposal, Joel mid-task with forty windows open.
   What do they already know, and what's the one thing they must leave
   with?
3. **What's the one sentence?** The title is that sentence, three to six
   words, with a turn: "The log *is* the record.", "Start new, *drain
   old.*", "A machine of your own, *in moments.*", "38 open, *6
   moving.*" If you can't write it, you don't know what the page is yet.
4. **What world does it live in?** Every loved site took a metaphor from
   its subject and let it run through everything: the colours, the mark,
   the ornament, the figures, the words for states. A log is a stem with
   leaves; a deploy is a harbour where the new boat comes in as the old
   one leaves; a tracker is water with tides and buoys. Name the world in
   one line, as Slack Water's stylesheet does ("Slack Water: cold
   bathymetric teal for the ground, warm sediment for what speaks, one
   painted buoy for where the keys go"). A page with no world falls back
   to the kit's defaults, and that's the engineered page.
5. **What's the one drawing?** The figure that explains the idea at a
   glance, drawn in the world's own terms. It's on the first screen or
   just under it. A page with words alone has failed before it starts.

## The Process

1. **Open a model and study it.** Pick the loved site closest to the job
   and read its worked example in `examples/` in full, then look at its
   pictures: a proposal or an essay starts from `proposal.md`, a guided
   walk from `tour.md`, a site of several pages with a live proof from
   `site-of-pages.md`, an interface or anything worked in all day from
   `tracker.md`, and a home, an index or a list of places from the
   Your Sites home itself (`modules/local-sites/home/`, in `rulings.md`'s
   What He's Loved). Where another skill layers the real sites' studies over
   these, read those too. Read `what-went-wrong.md` once, so you know the
   page you must not build.
2. **Choose the world's materials**, from `language.md`: the tint and the
   ground's turn, the faces and how each is used, the mark, the ornament,
   the figure's style, and the words for states. Write them as the
   stylesheet's opening comment before any rule.
3. **Draw the first screen whole**, before any other part: the crest or
   mark, the eyebrow, the title with its turn, the lede, the standing line,
   the way through (contents, places), and the one drawing. Look at it at
   1440 and 390 beside the model's first screen. If it doesn't stand next
   to the model, redesign it now, while it's cheap.
4. **Walk the rest** in the order a sceptical reader asks, each part
   opening with its point, carrying its own figure, and ending on the
   question the next answers. Depth goes in folds whose summary is the
   reader's question. The words follow `writing.md`.
5. **Judge it beside the model**, screenshot against screenshot, at both
   widths, part by part. List what the model has that this lacks (a
   drawing, an ornament, a turn of phrase, a quiet line where this has
   chrome) and add each, or say why it's gone. Then hold it to the
   aesthetic skill's questions. This is the step that makes it his.
6. **Then verify**, with `gates.md` and `checks.md` and the probes beside
   them: accessibility, keys, parts, layout, words. Fix what fails in its
   detail, never by retreating to the kit's defaults.
7. **Ship small, and say what's unfinished.** Post early, keep it
   current, and call anything that hasn't passed both the judging and the
   gates unfinished, never done.

## What His Loved Sites Share

Whatever the world, these held on all four, and none of the five he
turned down had them:

- **Type is the design.** A huge, soft, wonky serif title with an italic
  turn in the tint; an italic lede; small tracked mono only for eyebrows,
  numbers and labels. Rank by weight and voice before size.
- **A composed first screen.** Centred and symmetrical for a page to read
  (crest, eyebrow, title, lede, standing line, contents), or a split of
  claim beside proof (the Spring), or a sentence over the work (Slack
  Water).
  Never a generic header bar with the page underneath.
- **Drawn, not boxed.** Inline SVG in the world's metaphor, hairlines and
  space instead of cards, ornament at thresholds and joins.
- **Their own colour.** A tint and a ground turned to the subject (gold on
  forest, azure on night, teal for water, buoy orange on slack water),
  with every state a colour and always a word.
- **Navigation as typography.** An italic line of numbered stops, or two
  named groups of italic links, or a quiet row of place words. The keys
  are there, said in one quiet line set into the composition.
- **A voice.** Titles with a turn, ledes that are the whole argument, leads
  that hand over to the next part, folds that ask the reader's question,
  and a little whimsy at the thresholds.

## Beside This File

Load each when its step needs it, not before.

| File | Load it when |
| --- | --- |
| `examples/proposal.md` | modelling a proposal, an essay or any page to read |
| `examples/tour.md` | modelling a guided tour, a walk with stepped diagrams |
| `examples/site-of-pages.md` | modelling a site of several pages, a replay as the hero |
| `examples/tracker.md` | modelling an interface, a tracker, a live dashboard |
| `examples/what-went-wrong.md` | once per site, to know the engineered page on sight |
| `language.md` | choosing the world's materials: type, colour, space, ornament, figures, motion, words |
| `writing.md` | writing any words on the page |
| `interface.md` | the site is an interface, or holds a replay or a walked map |
| `rulings.md` | a question the language doesn't settle, or checking against his past verdicts |
| `craft.md` | the CSS for a technique: faces' axes, the switch, folds, side sheets, replays, diagrams |
| `sites.md` | making, placing or publishing a site: the local home, links home, the kit |
| `gates.md` | verifying, after the design is judged |
| `checks.md` | running the verification, with `gate.js`, `layout.js`, `map.js`, `homes.js` and `svgtext.js` |

The kit (`~/.local/share/jitsusama/web/`, `sites.md`) gives a site its
colours, faces, space ladder and a sensible base. It's a palette to
start from, not a uniform: a site sets its own world on top, with its own
tint, turn, type scale and ornament, and overrides the base wherever its
design says so.

## Judging Without Him

- **Don't hand it back to him to judge.** "I've done that before, and you
  keep missing the mark" (2026-09-22). A question this skill doesn't
  answer is a gap in it: once he answers, write it into the right file
  here, in core.nix's `modules/theme/web/skill/`.
- **Gates aren't taste.** All five pages he turned down on 15 November
  passed the probes. The only judge of the design is the model beside it
  and the aesthetic skill's questions.
- **Fix the concept before the pixels.** When polish doesn't register, the
  fault is the world, the first screen or the type's roles. Redesign
  those, however much it undoes.
- **Would it hang beside the Ledger?** If not, it isn't done.
