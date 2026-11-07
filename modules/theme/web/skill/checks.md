# Before Anyone Sees It

What to measure and look at before a site is shown or published, and after
every change to it. Each check below came from a fault Joel found by eye that a
check could have found first. Run them in a real browser, headless where it can
be, at every width and with every fold open; keep each as a script beside the
site, so it runs on every publish and a fault can't come back.

Gates aren't taste: the docs site that "looks like shit" passed AAA contrast
and a keyboard walk. Pass the gates, then look ([Looking](#looking)).

## The Widths

Check at 320, 390, 640, 768, 1024, 1280, 1440, 1512 and 1920 px wide, and on an
interface also 834 by 1000 (a tablet held upright) and 1512 by 900 (a laptop).
At each, check with every fold closed and every fold open, and in both looks,
dark and light. A layout that only holds at the width it was built at is the
fault "stretched and grotesque" names.

## Layout

- **No short lines.** Count blocks of prose of 40 characters or more whose
  widest line is under 35 characters (under 25 below 1024 px wide). Any is a
  column too narrow: "only a few words of text in a row is shit"
  (2026-10-20). A change that raises the count doesn't ship.
- **No long lines.** No line of prose over 75 characters.
- **Nothing overlaps.** For every text node, take its line boxes with a
  `Range` and check none crosses another's or leaves its container; every
  `scrollWidth` is at most its `clientWidth` where it doesn't scroll on
  purpose. "text overflowing into other text" (2026-11-07).
- **One set of edges.** The left and right edges of prose, figures, code and
  tables each sit on a named line: list the distinct edges per width and
  expect the few the grid declares. "why is the table so much wider than the
  main text" (2026-09-22).
- **Gaps on the ladder.** Measure ink to ink between neighbours; every gap is
  a rung, and each level's is wider than the level inside it. Equal inset on
  both sides of every frame.
- **Nothing wraps in the wrong place.** No word broken across lines; no date,
  id, number and unit, or key chord split; headings not orphaning a word.
- **No sideways scroll at 320 px** and no lost content at 200% and 400% zoom
  (WCAG 1.4.10, 1.4.4).

## Still Water

- **Nothing moves that the reader didn't move.** Toggle every control, open
  every fold and drawer, switch every view and both looks, and measure the
  layout shift of what was outside the thing changed: it's zero. Fonts
  arriving shift nothing (the stand-ins are matched).
- **No unfinished frame.** Record the first load and each view switch at 60
  frames a second for 30 seconds; no frame shows text uncut, a layout
  unsettled or an empty that later fills in place.
- **No flicker.** In the same recording, nothing appears and disappears within
  a few frames, and no image or avatar reloads.
- **Live changes land still** (an interface): feed it a burst of changes and
  check the cursor's item keeps its place on screen.
- **Idle is zero.** With nothing changing for 30 seconds, no animation frames
  run and no timers fire; a hidden tab draws nothing.

## Motion

- **A reader-speed scroll walk.** Scroll the page at a reader's pace, about a
  screen every few seconds, with a natural-scrolling trackpad's flicks too,
  recording: every animation is visible in its first frame, finishes while in
  plain view, and isn't skipped past. Scroll back: each rewinds.
- **Less motion asked for**: with `prefers-reduced-motion: reduce`, nothing
  moves and the page is complete.
- **Without script**: a page to read is whole with JavaScript off.

## Keys and Access

- **axe-core finds nothing** at each width, in both looks, with folds open.
- **A keyboard walk.** Tab from the top: the skip link shows, designed; every
  control is reached once, in reading order, and shows the one ring; every
  scroll container is reachable; nothing traps focus; Escape closes only a
  popup and returns focus, never a drawer or work. On an interface, j, k, /,
  :, ?, Space ? and Ctrl+K (Cmd+K on a Mac) do what the skill says on every
  page, and single letters do nothing in a text field.
- **A screen reader pass** on the landmarks, headings and one fold, a figure
  and a table: each says what it is.
- **Forced colours and more contrast**: every control keeps an edge, focus
  shows, figures stay legible.
- **Targets** of 24 px at least, 44 on touch, but for links in running text.
- **Nothing by colour alone**: a greyscale screenshot still tells every state.

## Speed

On a throttled middling phone (4 times slower CPU, a fast 4G network):

| Measure | A page to read | An interface |
| --- | --- | --- |
| Largest paint | 1.5 s | 1.0 s at 1512 px, 2.5 s at 390 px |
| Layout shift | 0.02 | 0 |
| Response to input | 100 ms | a key drawn in 50 ms, 100 ms at most |
| Fonts for the first view | 350 KB | 350 KB |
| Script for the first view | none needed | 100 KB compressed |
| A quiet tab | | under 1% CPU and 60 MB |

- **No request leaves the site.** List every request's origin on a load; there
  is one.
- **The first view's fonts** are the kit's three faces: Fraunces upright,
  its italic, which the heading's turn draws on most first screens, and the
  mono, about 340 KB together. Any further cut costs an axis the type system
  sets, or the kerning around an accented letter, so a site adds no face of
  its own to the first view rather than cutting these. Each swaps in over a
  stand-in matched to its metrics, so none holds the first paint.

## Words and Facts

- **Cold readers** read each changed section (`writing.md`) and say what it
  means; rewrite until each gets it on first read.
- **A guard fails the publish** on any private reference (internal ids, task
  names, file paths, branch names), a retired phrase, an emdash, or a word
  spelled other than in Canadian English.
- **Every fact from one source**, and every part of the page agrees with it.
  The updated time is the time the facts last changed.
- **The published copy is the local copy.** After publishing, fetch every file
  back and compare it; open the live page and look, never trusting the
  publish's own report.

## Looking

After the gates, look at the whole page yourself, as he would, at each width
with folds open and closed, in both looks, and judge it against the skill and
the aesthetic skill's questions:

- Is there one focal point, made of type, and does the eye know where to go
  next?
- Would a typesetter or a broadsheet's layout editor sign it? Is every pixel
  placed, every edge on a line, every gap on the ladder?
- Does it feel like his other sites, a family, without copying one?
- Is there whimsy at the thresholds, and none in the way?
- Could a reader who wasn't there get the whole idea from the first screen?
- Is anything there that nobody asked for?
- Would it hang in a museum, and could he work in it all day?

When a judge's or his complaint names a fault, measure it before calling it
fixed, add the measurement here or to the site's checks, and judge the whole
page again after the fix.
