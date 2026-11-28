# The Margin Cat

A pixel-art cat that lives in the margin above Joel's windows on niri: it
sits over the column the keys are in, walks, climbs and leaps to the next
one when they move, and plays a little while he rests. `MarginCat.qml` is
its surface, one per screen, and `MarginCatScreen.qml` follows niri's
layout for it. The scripts work out everything it does, so the Qt cases in
`tests/` reach all of it in Qt's own engine, in seconds:

| Script               | Works out                                                |
| -------------------- | -------------------------------------------------------- |
| `MarginCat.js`       | how lively it is, its plays, niri's spring, its drawings |
| `MarginCatPaths.js`  | its way to a seat: a walk, or a move between workspaces  |
| `MarginCatLedges.js` | where it can stand or hang on the columns, and its paces |
| `MarginCatRoutes.js` | the stops a way goes by: tops, sides, gaps and the floor |
| `MarginCatStacked.js` | a move down a workspace walked as one screen             |
| `MarginCatCarried.js` | riding its window as niri moves it, or walking on        |
| `MarginCatWays.js`   | a way's legs as beats in time, and where it is at a time |
| `MarginCatFalls.js`  | what it does as the window under it moves or goes        |
| `MarginCatFloats.js` | floating windows' tops as stops                          |
| `MarginCatPlays.js`  | a play's beats, as a route on its ways                   |
| `MarginCatPuppy.js`  | the puppy's visit: its track, and the cat's answer       |
| `MarginCatShown.js`  | how its walks and plays reached the screen               |
| `MarginCatTiles.js`  | niri's layout read as tiles                              |

How it looks and moves is the design's ([docs/design.md][design]); what it
must keep, whatever a ruling says, is Joel's.

The theme's checks read its walk too: `modules/theme/walk-read.nix` reads
the frozen `WALKS` and `STRIDING` in `MarginCatWays.js`, and the cat's and
the puppy's pose checks hold their strides to it. A change to `WALKS`'s
strides fails both until each animal's are drawn for it, a pair of feet a
step and each pair in its turn, and a gait added to `STRIDING` by a list
other than `WALKS` fails them until `walk-read.nix` reads it. The reader
takes the walk from those two declarations alone, and fails the file if
either is declared twice or `WALKS` is named anywhere else; a gait the
code strides by outside `STRIDING` passes unseen, as do the forms its
opening comment says it doesn't see.

## Joel's Words

Each design Joel has stated for the cat, in his words and dated, and the Qt
cases that hold it. A ruling on the cat's design is read against this list
first, so none quietly replaces what he asked for; one that crosses his
words fails a case before anyone builds it. The check
`the-cat-keeps-joels-words` reads this section: every design says when he
said it, every case it names is in that file of `tests/` outside a comment,
and a design with no case yet is listed as owed, with the date it was owed
from, so the list says what's still to do. A new design he states goes
here, owed, the moment it's taken up.

### Smooth, With Character

Joel, 2026-11-12 16:35: "the cat is not animated, it just jerkily moves, it
is not enjoyable with this change. we need proper smooth animations with
character to them"

- Held by `margin-cat.qml`: "it turns its head the way it's going first, standing where it sat"
- Held by `margin-cat.qml`: "it draws back a cell, crouched, before it sets off"
- Held by `margin-cat.qml`: "in its four strides, in turn"
- Held by `margin-cat.qml`: "each stride a stretch of ground, never a frame's time"

### Mild While He Works, More as He Rests

Joel, 2026-11-12: "mild movements when i'm active, increeeasing as I'm less
engaged"

- Held by `margin-cat.qml`: "while he types it's mild"
- Held by `margin-cat.qml`: "ten seconds with no keys, a little more"
- Held by `margin-cat.qml`: "a minute with none, livelier"

### Play Random, Gentle and Playful

Joel, 2026-11-12 21:22: "I would like for it to play a bit more and not wait
too long. it's play should be a bit random, gentle, but playful."

- Held by `margin-cat.qml`: "a pause rests 5 to 12 s"
- Held by `margin-cat.qml`: "a minute with no keys rests 3 to 8 s"
- Held by `margin-cat.qml`: "never the same play twice running"
- Held by `margin-cat.qml`: "each play short, under two and a half seconds"

### Walk Between Workspaces as One Screen

Joel, 2026-11-12 21:15: "it would be nice if when changing workspaces up or
down it walked up or down to the new location"

Joel, 2026-11-14 17:30: "when going up or down it should appear on the
screen from the position that would make sense for how it would have to
path if they were all on one screen. so the top of the left would have to
walk to the right, climb down the right side, drop down and grab with its
claws on the top of the window below, climb up, and then walk over."

Joel, 2026-11-15 21:20: "when transitioning screens it should be able to
poke its head up and then come in." And at 23:34: "the whole kitty should
not have to be rendered when you are seeing it come on to your screen."
It comes in head first, three rows over the screen's edge, its paws
moving; it peers and looks about, then comes on. Only the screen's edge
cuts it.

- Held by `margin-cat.qml`: "down a workspace to a window under another, never gliding"
- Held by `margin-cat-one-screen.qml`: "the drop ends in a grab at a top's corner in its reach"
- Held by `margin-cat-one-screen.qml`: "with no corner in its reach, the drop lands on the top
  under it, never the floor"
- Held by `margin-cat-one-screen.qml`: "it comes in at the top over the gap, head first"
- Held by `margin-cat-one-screen.qml`: "it comes in at the bottom beside a side, head first"
- Held by `margin-cat-one-screen.qml`: "no more of it than its head over the edge till it has
  looked"
- Held by `margin-cat-one-screen.qml`: "its head over the edge, it looks about for 400 ms"
- Held by `margin-cat-one-screen.qml`: "its head comes over the edge with its paws moving"
- Held by `margin-cat-one-screen.qml`: "then it comes on with a drop or a leap up"
- Held by `margin-cat-one-screen.qml`: "the leap up ends hanging on a side's foot, never its
  underside"
- Held by `margin-cat-one-screen.qml`: "up from a top under the crack above, it leaps from there
  straight up it"
- Held by `margin-cat-one-screen.qml`: "up with no side over any top, it comes in at the nearest
  side's foot, unmoved"
- Held by `margin-cat-one-screen.qml`: "told late, never ahead of niri's slide into its window"

### Walk the Strip to a Column Off the Edge

Joel, 2026-11-14 17:24: "is the kitty supposed to be climbing down the sides
of windows yet? ... workspace changes, or focusing to a window off the left or
right edge"

niri lays a workspace's columns on one strip and scrolls it. While the keys
rest on the new column it stays on the window it sat on, carried off the
screen with it on niri's spring if that's where niri scrolls it, and back
with it the same way if the keys come back first; then it comes back in at
that edge head first, as between workspaces, and walks the strip to its
seat, leaping the gap at the edge only where its window is all past it.
Caught mid-walk by the scroll standing on its window's top, it stops where
it's drawn and rides the same way, then walks on to its seat as the keys come
back; caught mid-leap or anywhere else, or carried by a drag or a column
moved beside it rather than a scroll, it walks on to that window where it
moved. Back on its column grown or shrunk across, it walks to its seat; back
on it as niri slides it aside for a column opening, closing or resizing
beside it, it rides there on niri's spring, as with a scroll. A
ride isn't a rest: no play sets off and no pet starts till it ends. A scroll
runs for niri's spring from when niri says it began, so a scroll's word left
on a window long after reads as ended. niri can say its window scrolled
wholly off the screen before it says the keys left that window; it rides off
with it then, as it would after, since that move is never heard again. Moved
to another workspace while it's off the edge, it holds out of sight through
niri's slide, then comes in at that edge head first over the new workspace's
windows; with its workspace emptied, it comes in at that edge along the floor.

- Held by `margin-cat-one-screen.qml`: "its window scrolled, it's carried as far"
- Held by `margin-cat-one-screen.qml`: "its window said off the screen first"
- Held by `margin-cat-tiles.qml`: "a column's move asked of long after keeps its begin and reads
  as ended"
- Owed since 2026-11-27: a case on niri that the cat rides back to its seat as a column
  opening beside its window slides it aside; the Qt cases hold only which word is heard.
- Held by `margin-cat-one-screen.qml`: "drawn where it was as niri sets off, nearer as it
  slides, there as it stops"
- Held by `margin-cat-one-screen.qml`: "it comes in at the edge on the top there, head first"
- Held by `margin-cat-one-screen.qml`: "then it leaps the gap at the edge onto the window inside
  it"
- Held by `margin-cat-one-screen.qml`: "on a window partly on the screen, it walks to its gap, then
  leaps only the gap"
- Held by `margin-cat-one-screen.qml`: "then it comes on facing in, never thinking again or going
  back out"
- Held by `margin-cat-one-screen.qml`: "and walks the tops to its seat"
- Held by `margin-cat-one-screen.qml`: "it holds out of sight as niri slides"
- Held by `margin-cat-one-screen.qml`: "it comes in at that edge, head first"
- Held by `margin-cat-one-screen.qml`: "a floating window dragged, it sits there at once"
- Held by `margin-cat-one-screen.qml`: "walking along its top"
- Held by `margin-cat-one-screen.qml`: "in the air of a leap onto it"
- Held by `margin-cat-tiles.qml`: "a scroll's word left on a window as another's layout comes
  keeps its begin"
- Held by `margin-cat-tiles.qml`: "a layout sent again mid-scroll keeps the scroll's begin"
- Held by `margin-cat-tiles.qml`: "a new scroll beside an old one's word left on another window
  is dated by the new"
- Held by `margin-cat-one-screen.qml`: "standing on its top as niri scrolls"
- Held by `margin-cat-one-screen.qml`: "standing on its top as a drag carries it"
- Held by `margin-cat-one-screen.qml`: "held short of this seat by a scroll, it walks on to
  it"
- Held by `margin-cat-one-screen.qml`: "its column grown across, it walks there"
- Held by `margin-cat-one-screen.qml`: "off the right under it"
- Held by `margin-cat-one-screen.qml`: "a pixel on the left, it comes straight down"
- Owed since 2026-11-25: on nested niri, a walk caught by a scroll off the edge rides its
  window, and a floor came to from off the edge.
- Owed since 2026-11-26: a Qt case that no play, wander or pet starts during a ride; the
  view's `still`, `play()` and `strollsNot()` each check `carry`, which nothing pins. A key's
  walk back home (`stroll("")`) and a stroll's walk back over its column are let through
  mid-ride, from where the cat is drawn as niri's spring slides; no case holds that either.
- Owed since 2026-11-27: a case of the view's own wiring, that follow() gives backOn
  `held` only for the seat a scroll held it short of, and `heldShortOn` is cleared once it
  plays; the choices are pinned in MarginCatCarried.js, not what the view passes them.
- Owed since 2026-11-27: a case of the view's own wiring, that its window said scrolled
  wholly off the screen before the keys' move rides it off (carriedOff); on nested niri
  the order can't be chosen, so `the-margin-cat-follows-the-keys-on-niri` reaches it only
  when the race goes that way, and the choice is pinned in MarginCatCarried.js alone.
- Owed since 2026-11-27: a case that a column moved before its tile record keeps a
  scroll's date; a layout object made afresh from the stale tile reads as a scroll begun.
- Owed since 2026-11-25: a word cached before the tiles watch goes live is dated as begun
  its `ago` before the shell heard it, not before niri sent it, so an old scroll may read as
  just begun and start a ride for one spring; unmeasured. niri's own stream, connecting as a
  stand-in and taken again after, dates its old words the same way.

### Climb, Path and Leap About the Windows

Joel, 2026-11-13: "it should stay on top of the previous workspaces window
... climb up ... pathfinding through the spaces between windows"; "when
crossing across gaps between windows, the kitty should jump"

Joel, 2026-11-13, by WCJQSG: "i might be on a window in the second row and
it needs to travel to it. or I am in a column on the left and change
workspaces where the last focused window is on the right"

Joel, 2026-11-13, by WCJQSG: "the cat should always be moving from where it
is to where my focus shifts, in a believable and adorable way. the windows
are the walls and roofs it is climbing or jumping off of"

A gap's leap is an arc that reads at 60 Hz: a cat in flight holds each pose for a third
of the way, and the arc is what moves (VKCF3Z, 2026-11-15, 5f8bf9e9 and 69bb37d0).

- Held by `margin-cat.qml`: "across a gap between columns it crouches, leaps and lands"
- Held by `margin-cat.qml`: "seen in at least 7 frames at 60 Hz, and quick as a leap"
- Held by `margin-cat.qml`: "rising as it stretches, held at the top tucked, falling as it
  reaches"
- Held by `margin-cat.qml`: "on a ledge under a row, no higher than the row's bottom"
- Held by `margin-cat.qml`: "to a window under another it goes down a side and leaps the gap to it"
- Held by `margin-cat.qml`: "windows come, it climbs to its seat by an edge, never up a face"
- Held by `margin-cat-journeys.qml`: "each ends on its seat, every frame of it a cat's"

### Insanely High Quality

Joel, 2026-11-13 17:15: "insanely high quality pixel art and the animations
are held to an insanely high quality"

- Held by `margin-cat-rides.qml`: "each frame of its own a whole number of cells from where its
  beat starts or ends"
- Held by `margin-cat.qml`: "each seen two frames at 60 Hz"
- Held by `margin-cat.qml`: "up and down again in an arc, never a glide"
- Held by `margin-cat.qml`: "never a jump between two frames"

### Climb at Walking Pace

Joel, 2026-11-13 17:22: "Going up the side of a window should be at the same
speed roughly as walking on top, it seems accelerated in the video"

- Held by `margin-cat-journeys.qml`: "it climbs a side at about its walking pace, within a fifth"

### Believable Routes

Joel, 2026-11-13 17:22: "it would not be able to drop down near the center
of a window below. we need it to be very believable"

- Held by `margin-cat-journeys.qml`: "falls from"
- Held by `margin-cat.qml`: "and in no face from there to its seat, never a fall"

### Resized Windows

Joel, 2026-11-13, by WCJQSG: "the kitten also needs to handle resizing
windows"

- Held by `margin-cat-rides.qml`: "told of the resize late, it rides as far into niri's spring
  as the edge has gone"
- Held by `margin-cat-rides.qml`: "its column narrowed out from under it, it falls"

### Closed Rows

Joel, 2026-11-14 17:24: "when a row in a column is deleted, it seems to hang
out in the middle then walk across, that is not believable"

- Held by `margin-cat-rides.qml`: "a row closed under it, it falls at once onto the floor, never
  shoved or thinking"
- Held by `margin-cat-rides.qml`: "and from the floor to its seat by an edge, in no face, never
  gliding"
- Held by `margin-cat-rides.qml`: "the window over it grown down too, it falls never above that
  window's edge"
- Held by `margin-cat-rides.qml`: "told by niri when the window began to grow, it goes as far into
  it as niri says"
- Held by `margin-cat-rides.qml`: "heard to close once the row below moved into its place, it
  falls all the same"

### Floating Windows

Joel, 2026-11-13 18:59: "we need to have the cat support jumping down on to
a floating window, and then jumping to the top of a non floating one when it
closes or loses focus"

It's never over a floating window's face but where the screen's top draws it there, as deep
as its feet sit, and at its corner: with no room over one for its hop, it steps down off the
end within its own height, paws moving, crossing the corner for a body-length's walk, under
a fifth of a second on its way over, under three tenths off a seat, where it eases to its
stop, or up to half a second on a short way, the smaller break than walking
one on the air (WCJQSG 1ee24751, 2026-11-18: "Stepping down a small ledge with its paws
moving is what a cat does"). With room for its hop, a drop of a cell or two steps from the
edge too, rather than walk a body-length on the air first (WCJQSG 3697e429); its feet a cell
over the top, it's never drawn over the face doing it. A deeper one, with room, hops clear
and falls. Its walk replanned partway down a step, it goes on clear of the end the near way,
in front of the tiled windows, before it settles, never behind the window it stepped off.
Sitting on the tiles under a floating window's corner, it stays under it as it sets off.

- Held by `margin-cat-floats.qml`: "onto a floating window it drops straight down from the top
  over it"
- Held by `margin-cat-floats.qml`: "off it, it crouches and leaps straight up onto the ledge
  in reach"
- Held by `margin-cat-floats.qml`: "its window closed, it leaps up at once, never thinking or
  walking first"
- Held by `margin-cat-floats.qml`: "on a floating window it's drawn over the tiled ones"
- Held by `margin-cat-floats.qml`: "dropping onto a floating window, it's drawn over the tiled
  ones"
- Held by `margin-cat-floats.qml`: "leaping off one, over them until it lands on the ledge"
- Held by `margin-cat-floats.qml`: "with no floating window on its way, never over them"
- Held by `margin-cat-floats.qml`: "never over its face, on or off it"
- Held by `margin-cat-floats.qml`: "crossing the corner of its face 3 deep, for a
  body-length's walk"
- Held by `margin-cat-floats.qml`: "cramped a step down, it steps off from its edge rather
  than walk clear first"
- Held by `margin-cat-floats.qml`: "cramped a step down on its way over one, it steps off
  from its edge too"
- Held by `margin-cat-floats.qml`: "a step down off one with room for its hop, it steps off
  from its edge"
- Held by `margin-cat-floats.qml`: "stepping down a step from its edge, it's never drawn over
  its face"
- Held by `margin-cat-floats.qml`: "replanned partway down its step, it's never behind the
  window it stepped off"
- Held by `margin-cat-floats.qml`: "and it goes on clear of the end it was stepping off, never
  back across"
- Held by `margin-cat-floats.qml`: "sitting on the tiles under a floating window's corner, it
  walks off under it"
- Held by `margin-cat-floats.qml`: "up to its own height down off one at the top, it steps
  off; a pixel more, drops"
- Held by `margin-cat-floats.qml`: "its own height down off it on a short way, it steps off
  as slowly as it ambles"
- Held by `margin-cat-floats.qml`: "at 1.5x it steps its own height down, never sunk; a pixel
  more, it drops"
- Held by `margin-cat-floats.qml`: "at 1.5x over it on its way, and off one a part-cell over
  the top, never sunk"
- Held by `margin-cat-floats.qml`: "off a seat its own height down, it steps; a pixel more,
  it walks clear and drops"
- Held by `margin-cat-floats.qml`: "at a cell of 2, its own height down off a seat it steps;
  a pixel more, it drops"
- Held by `margin-cat-floats.qml`: "at a cell of 2, an odd drop off a seat, it goes a cell
  along first, never sunk"
- Held by `margin-cat-floats.qml`: "one across the tops on its way, it walks over its top"
- Held by `margin-cat-floats.qml`: "a floating window a leap up on its way is a step"
- Held by `margin-cat-floats.qml`: "a floating window off its way changes nothing"
- Owed since 2026-11-16: on its way, straight up off a floating window onto a ledge over
  it, down onto one from a ledge over it, and from one floating window to another.
- Owed since 2026-11-13: its floating window's focus gone, it waits two seconds, then leaps up
  off it; the wait is the screen's, which no Qt case reaches yet.

### Unobtrusive and Cute

Joel, 2026-11-11 22:55, by P9R1KI: "i think having something crawl around
my screen in a way that is unobtrusive and cute is fun. i think making sure
the way things look on screen is joyful but not distracting is fun."

- Held by `margin-cat.qml`: "while he types it's mild"
- Held by `margin-cat.qml`: "each play short, under two and a half seconds"

### Never Skid

Joel, 2026-11-14 17:19: "i never want the kitty to skid into place, i always
want to see its paws moving when it is walking or running."

- Held by `margin-cat.qml`: "it walks onto its seat, never past it"
- Held by `margin-cat.qml`: "at a run, at most three cells a drawing"
- Held by `margin-cat.qml`: "a walk starts a pair on from its stand, and stops at most a
  pair off it"
- Held by `margin-cat-floats.qml`: "set off mid-stride into a turn and a think, it steps to its
  stand a pair at a time"
- Held by `margin-cat-floats.qml`: "re-planned as it turns in a stride before it's seen, it
  holds it a stride more"
- Held by `margin-cat-floats.qml`: "re-planned once the stride it turns in is seen, it steps a
  pair on at once"
- Held by `margin-cat-floats.qml`: "set off again easing in, it eases in; at its pace, it keeps
  it"
- Held by `margin-cat-floats.qml`: "a step back reads as a skid, but onto its stand as it holds
  there"
- Held by `margin-cat-floats.qml`: "a stride drawn under 25 ms flashes, set off in nothing too,
  but the one before"
- Held by `margin-cat-floats.qml`: "it walks out of its stand and back onto it a pair of feet
  at a time"
- Held by `margin-cat-floats.qml`: "nor fails to go, or skids or flashes a stride, any way,
  set off or climbing"
- Held by `margin-cat.qml`: "set off from another stride, it's never both pairs off the one
  it stops into"
- Held by `margin-cat-floats.qml`: "set off again every frame, a stride steps once a stride's
  ground"
- Held by `margin-cat-floats.qml`: "set off mid-stride into a turn, it turns in the stride drawn"
- Held by `margin-cat-floats.qml`: "stepped into early to stop, a stride counts from where it was
  first drawn"
- Held by `margin-cat-journeys.qml`: "each ends on its seat, every frame of it a cat's"
- Held by `margin-cat-floats.qml`: "onto and off one 2 above the tops, it steps within its
  walk, never leaping"
- Held by `margin-cat-floats.qml`: "at the screen's top, a float with no room for an arc is
  walked on, stepped off"
- Held by `margin-cat-floats.qml`: "never walking or falling above the screen's top, there or
  back"

### Paws Moving Up and Across Edges

Joel, 2026-11-14 22:40: "the kitten when crawling up or across window edges
still often does not look like its paws are moving."

- Held by `margin-cat-journeys.qml`: "each ends on its seat, every frame of it a cat's"

### Play With the Mouse

Joel, 2026-11-12 15:56: "i want to be able to also use my mouse to engage with my
animals."

- Held by `margin-cat-pets.qml`: "a pointer moved onto it as it sits strokes it"
- Held by `margin-cat-pets.qml`: "never one it came to under, still where it was"
- Held by `margin-cat-pets.qml`: "resting on it 300 ms, and leaning 600 ms after it goes"
- Held by `margin-cat-pets.qml`: "it leans toward the pointer's side, a lean left drawn from a
  cell out"
- Held by `margin-cat-pets.qml`: "leaning either way, its base is on sitting's cells, and leaning
  left its tail too"
- Held by `margin-cat-pets.qml`: "and only its head a cell toward the hand, eyes shut"
- Held by `margin-cat-pets.qml`: "a pointer crossing it swats about one time in three"
- Held by `margin-cat-pets.qml`: "never one that stopped on it, never twice within 20 s, never
  asleep"
- Held by `margin-cat-pets.qml`: "its paw is back down within 250 ms"
- Held by `margin-cat-pets.qml`: "swatting either way, its base and tail are on sitting's cells"
- Held by `margin-cat-pets.qml`: "a paw raised toward the side it left by, beside its head, the
  rest unmoved"
- Held by `margin-cat-pets.qml`: "either way, the same paw"
- Held by `margin-cat-games.qml`: "a click on the cat rolls its yarn in, noticed and eyed"
- Held by `margin-cat-games.qml`: "from just behind its column's edge, never appearing at rest"
- Held by `margin-cat-games.qml`: "a click on the yarn's near side rolls it four to eight cells
  away"
- Held by `margin-cat-games.qml`: "a cell short of its column's end, and with no room for a bat,
  it eyes it"
- Held by `margin-cat-games.qml`: "the cat's eyes follow it, it walks up to it and bats it on"
- Held by `margin-cat-games.qml`: "the cat never on the yarn"
- Held by `margin-cat-pets.qml`: "held by the scruff, it hangs under the pointer, wholly on the
  screen"
- Held by `margin-cat-pets.qml`: "its scruffed drawing hangs whole, as wide as it sits"
- Held by `margin-cat-pets.qml`: "over the windows held and falling, then by its landing, then
  its seat"
- Held by `margin-cat-games.qml`: "let go over a window, it falls onto its top, lands and flicks
  an ear"
- Held by `margin-cat-games.qml`: "falling the whole way, straight down and never up"
- Held by `margin-cat-games.qml`: "over a window's face, onto the next window's top, never up
  onto its own"
- Held by `margin-cat-games.qml`: "two pixels under a top, it scrambles up nothing and falls to
  the floor"
- Held by `margin-cat-games.qml`: "over a floating window, onto its top"
- Held by `margin-cat-games.qml`: "let go on a top, it only lands and flicks an ear"
- Held by `margin-cat-games.qml`: "a long fall moves every frame, as far each as the fastest
  fall goes"

### A Little Puppy Too

Joel, 2026-11-12 15:18: "I love the cat! So much! Can we have it move around
gently in between windows, bat balls of yarn and stuff? And maybe throw in a
little puppy too?"

The puppy visits only once he's left the keys, as the cat's other moves
come, and rests 20 to 40 minutes between visits so it stays a treat rather
than a second animal always about. The cat answers it where it sits. A key
ends a visit: the puppy fades where it is and the cat sits from where it's
got to, or, with the keys on another column, goes back over the window it
sat on until they rest there. Dozing off ends none by itself. A scroll that
carries the window it sat on away ends one too, dozing or not: the puppy
fades and the cat goes to its seat on that window, riding niri's scroll or
its column's slide aside, or there at once as a drag carries it, never left
on the air where the window was. Under the launcher's card the puppy goes at
once rather than fading.

- Held by `margin-cat-puppy.qml`: "from just behind its column's edge, drawn only within it,
  facing the cat"
- Held by `margin-cat-puppy.qml`: "it bows as the cat eyes it, as long as the cat thinks
  besides"
- Held by `margin-cat-puppy.qml`: "the cat answers with a hop where it sits, the puppy sitting"
- Held by `margin-cat-puppy.qml`: "then it turns and trots back behind the edge as the cat
  watches"
- Held by `margin-cat-puppy.qml`: "the cat never leaves its seat"
- Held by `margin-cat-one-screen.qml`: "its window scrolled away, it rides to its seat"
- Held by `margin-cat-one-screen.qml`: "cut mid-hop as a drag carries it, it sits on its seat
  at once"
- Held by `margin-cat-one-screen.qml`: "scrolled wholly off the right, it rides as far as
  niri carried it"
- Held by `margin-cat-puppy.qml`: "a new drawing each three cells it covers, in the cat's order"
- Held by `margin-cat-games.qml`: "and the puppy a few times, 20 minutes apart at the least,
  unevenly"
- Owed since 2026-11-12: the puppy answering his pointer as the cat does, for "engage with my
  animals" (Play With the Mouse)
- Owed since 2026-11-26: on nested niri, a focus moved mid-visit leaving the cat over the
  window it sat on till the keys rest, a doze mid-visit letting the puppy trot off, and the
  card coming up mid-visit taking the puppy at once; and a journal case that a cut visit is
  told as the puppy's, not the walk's.
- Owed since 2026-11-27: a case of the view's wiring, that follow() ends a visit as niri
  carries its window, and passes visitCut that window's seat; the choice is pinned in
  MarginCatCarried.js, not what the view gives it. Nor does a case say where the puppy fades
  as its window is carried off, or what a key cutting a hop before niri's scroll is heard
  does.

[design]: ../../docs/design.md
