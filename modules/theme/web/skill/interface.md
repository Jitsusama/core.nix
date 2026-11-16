# An Interface to Work In

What an interface (a tracker, a tool, a sheet, a viewer, a live map) needs
beyond a page to read: how it opens, how it stays still under its reader,
its keys, and the terminal replays some pages carry. Load this when the
site is an interface, or carries a replay or a walked map. Slack Water
(`examples/tracker.md`) is the model: an interface he uses all day, in a
world of its own.

## How It Works

- **The whole first, drawn, then the exceptions.** It opens on everything it
  holds as one picture, lets him narrow by stepping in or finding, and shows
  a record's detail only when asked (Shneiderman's overview, zoom and
  filter, details on demand). Its first words say what's wrong, busy or
  worth a look, a sentence each, never totals that read alike on every row:
  the network page's Worth a Look. Every mark carries what's true (how big,
  how busy, how stale) by its size, fill or weight, and alike things are
  drawn alike so one reads against the next. The Code page opened on two
  tables of 172 names in one weight, with figures that said nothing to act
  on, and he called it "complete shit" (2026-11-15) beside two drawn pages
  he liked.
- **What can be done, where it can be done.** The keys that act on the row
  under the cursor show on that row while it has the cursor, never behind ?
  alone, as the launcher's key line does. A page with one act, hidden, and
  nothing linking onward is a dead end however clean its parts.
- **Still water.** Nothing under the reader moves unless they moved it: a view
  switch, a drawer, a teammate's edit or a new build. Redraw only what
  changed, measure before painting, keep the cursor on the same item at the
  same place, batch arrivals, and let a new item that would land in view wait
  behind a quiet "one new". The cursor moves at once, with no transition. "i
  get a lot of flashing ... profile pics flash" (2026-11-05). A new build
  never reloads a tab under its reader, and old tabs keep working. A drawn
  place keeps its position between readings, so his memory of where a thing
  sits keeps working, and only what changed moves.
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

## Replays

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

## Keys

- **Nothing needs a click** (standing rule, 2026-11-14). The pointer is
  supported everywhere, and the keyboard comes first: a site is designed
  around its keys and then given the pointer, never the other way round, as
  the desktop's first principle says. A reader who has never seen the site
  finds their way with the keyboard alone, without being told how.
  - **A picture is walked, not only looked at.** A map, graph, chart or
    diagram that holds things is a structure the keys move through: the
    arrows or h, j, k and l step along its links to the nearest thing that
    way, Return opens what the cursor is on (centring it and showing its
    details), / finds and jumps, and - steps back out. Pan and zoom have
    keys, and ? lists them all. An arrow lands on the nearest thing on the
    level being walked within 45 degrees of it, or stays and says so, and
    the opposite arrow comes back; Tab leaves the map from any depth; no
    name sits over a line or another name, or runs past the map's edge.
    `map.js` measures each. The
    arrows on the mycelium map once jumped to whatever lay nearest at any
    level, and he said they "seem to pick at random" (2026-11-14). A map
    drawn on a canvas tells `map.js` where its links run, with
    `data-lines` and `lines()`, or the check fails: when the mycelium map
    moved onto a canvas (2026-11-15), the check found no lines and passed
    whatever lay under its names.
  - **A screen reader gets the same structure**, as a tree, grid or list
    with names and states, never a picture with a caption. Each thing says
    what it is, how it stands and what it links to.
  - **Focus always shows**, in the one ring, and is never lost: after an act,
    a close or a redraw it stays on the thing or moves to its nearest
    neighbour, never back to the top.
  - **Reduced motion stops what moves on its own**: pulses, drifting
    particles, autoplay. What the reader moved still moves, without the
    flourish.
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
