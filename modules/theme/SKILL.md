---
name: aesthetic
description: >
  Joel's aesthetic and how to build to it without asking him: his brief, the
  character of the design (modernist, Nordic and Baja, with whimsy), what
  he's liked and turned down, and the questions to judge a design by. Use
  before designing, building or changing anything he sees, types or presses:
  a window, a launcher, a notice, a prompt, a status line, terminal or
  command output, a theme, colours, type, spacing, marks, motion, key
  chords, a page or a document laid out for him.
---

# Joel's Aesthetic

How what Joel sees and touches should feel, found over many rounds of
building things and his verdicts on them. The system built from it, the type
and space scales, the colours, the marks, the shapes and kinds of
surface, motion and the keyboard's language, is the design document:
`design.md` beside this file where the theme module installs it, and
`docs/design.md` in core.nix. Read both before designing anything. A website,
any page or app in a browser, is built to the `web-design` skill as well,
which says what a browser changes and gives the theme as CSS and fonts.

- **Build to it, and judge by it yourself.** Don't hand a design back to
  Joel to look at. A question this skill and the design document answer
  isn't put to him. A question they can't answer is a gap in them: once he
  answers it, write the answer here, in core.nix's
  `modules/theme/SKILL.md`, so it's never asked again.
- **Change a habit only for something clearly better.** A key, a gesture or
  a flow he already uses changes only when the change is better by these
  terms, and the change is said plainly when it ships: what moved, where it
  went, and why.
- **Say what's unfinished.** Whatever hasn't met the design document's
  quality bar is called unfinished when it's shown, never done.

## The Brief

In Joel's words:

> I am a minimalist but also an artist. I love form and function and text and
> glyphs elevated to art. Spacing, padding, margins, typography, design
> language, ligatures, monospace where it deserves to live and non-monospace
> where it deserves to live. Every pixel matters. I want everything to feel
> cohesive, I hate wasted space but I also hate too much business.

And, as the design grew:

- Strongly consistent; giving him what he needs; helping him see what's
  important; never cluttered, and never leaving him looking for more.
- No redundancy: nothing told twice, and everything where it makes sense and
  only there. Conservation and intent.
- Legibility is vital, whimsy is appreciated, anything buggy is anathema.
  Smooth, never jerky; clean, with beautiful proportions and careful
  whitespace.
- Type and colour run through everything and are easy to swap, end to end.
- He's very nerdy: useful information is always present. But no useless
  information and no busyness for its own sake.
- "every integration has an insanely high bar of nerdiness and integration
  and each should be given the same care" (2026-11-07, of the eero: "i want
  insanely nerdy integration with the eero"). Every program, device and
  service the desktop touches goes as deep as the deepest: every true
  figure it has, live, in its place, its drawer, its : commands and its
  command line. With a poor memory, a thinner one teaches him that what he
  wants may not be there, and he stops looking.
- Every choice comes with deep room to customise it, and performance always.
- He has a very poor memory, so everything has to be intuitive and work the
  same way everywhere.
- Things collapse cleanly and on purpose when space is short.
- The keyboard is the way in to everything, without exception, and it has
  to be comfortable; the pointer comes second.
- "Anything you build has to meet an insanely high quality bar. No half
  assed shit can happen. Everything has to follow my design aesthetic which
  has been refined through our work."

And the bar for everything since, set on 2 November 2026 after two looks at
the desktop's keys and type failed:

> i want greater emphasis on typography as well as excellent use of colour,
> insanely tasteful UI design and UX. I want something you could put up in
> an art museum that is also insanely functional, that fits my aesthetics
> for form and function. no detail can be ignored. every pixel of
> whitespace must be accounted for. alignments, available space, all
> interactions, each must be amazing. the same story with how i interface
> with a keyboard.

The design document's Every Pixel Accounted For turns it into checks: type,
colour, whitespace, alignment, the space available, every interaction and
the keyboard on both his keyboards. A design is still judged without him,
and shown to him as finished only once every check is answered; anything
shown sooner is called unfinished.

## The Character

In his words, a modernist, "with a bit of baja/nordic leanings, that also
loves whimsy", wanting "functionality and focus on simplicity that doesn't
constrain".

- **Modernist.** Form follows function. Type is the main material, set on a
  grid of one unit, with hierarchy made by order, size, weight and colour
  rather than by boxes: a few named levels, each two cues from the next,
  as docs/design.md's Levels set them. Nothing is ornament that doesn't also work: a
  border says a thing is a thing, a radius says whether it's whole or a
  part, and a mark names an idea.
- **Nordic.** Calm, restrained and made to be lived with. Generous air, few
  things on screen, each made with care, and a surface that recedes until
  it's wanted. Light shadows rather than heavy ones, opaque windows, and
  floating things cloudy enough to hint at what's beneath without showing
  it.
- **Baja.** Warm, natural and unhurried. A palette is pigments, as Osaka
  Jade's are forest and jade, parchment and cream, gold and vermilion, never
  neon and never glowing. Edges are softened, by the theme's radius and
  feathered shadows, but never bubbly, and motion arrives easily and settles
  without a wobble.
- **Whimsy.** The warmth that keeps minimalism from going cold: marks in
  their kind's colour, the padlock beside the password dots, a strong
  picture behind everything, a row of marks in the margin saying what's
  on, a suggestion in the field that changes each time. It
  lives at thresholds and in marks, never on the numbers or the controls he
  needs mid-task. Pure text with no marks grows dull.
- **Simple, never constraining.** Few ideas, each reaching far. Nothing
  useful is taken away to look clean: numbers stay, made quiet by type.
  Every choice is an option underneath, and the default is the one that
  needs no setting.

## What He's Liked

- **The text is the star, the marks add the impact.** A mark leads only a
  line to act on, and text below a mark lines up with its ink, never
  indented from it. A figure wears none, since "it is hard to tell actions
  from stats" when both lead alike (2026-10-24).
- **Braille graphs, ten characters wide.** Over a dim track of every dot,
  so a graph's ends show; block characters were "too blocky".
- **One frame.** The launcher won when it became one border round a field
  and its results, split by a hairline, its selected row an inset. The
  frame's corners are softened by one unit and the row inside it is square,
  since a smaller curve inside a small one wobbles.
- **Cloudy, mostly opaque.** A floating surface that "only hints at what is
  beneath", lifted by light shadows, its rows told apart by weight and
  colour.
- **The picture shines.** A strong wallpaper, never dimmed. Text on it is
  lifted by offset shadows of its own letters, and text that may lie over
  windows too by a halo, never by a dark block.
- **Things that feel physical.** Motion carries a thing the way it would
  move.
- **Marks one height.** A row of marks stands level, every one's ink the
  same height on one middle line, however tall its font draws it; uneven
  heights set off "my autism infused OCD" (2026-10-24).
- **Rows that tile, as Vim's lines do.** A row's room is inside it and rows
  meet, so the selected one's fill spans its whole row, edge to edge ("rows
  in the launcher should have padding instead of margins", 2026-11-06). It
  replaced a fill hugging the ink, which left the chosen line against the
  next. A selection inside a line still covers its characters exactly.
- **A step inside the edge.** Things in a corner sit a little inside the
  windows' edge, so nothing lines up with something else by chance.
- **Chords drawn as keys.** A key's mark where a good one exists, its name
  where none does, joined by + with room on each side, muted.
- **Signatures, not headers.** A notice says who in strong type, then the
  message, then a muted footer: the program's mark and name at the bottom
  left, the time at the bottom right.
- **The calm prompt.** Where he is, where his keys go, and only what's true
  besides; the rest a step away.
- **Found by typing.** The launcher as the way into everything: a few
  letters, the handful of things they could mean, each with its key beside
  it.

## What He's Turned Down

- **Middling type, and a stock template.** The first look at the desktop's
  keys and type: "the typography is middling. you are not leaning into my
  aesthetic very well, and are not leveraging type and typesetting enough",
  and "the existing launcher design has better typographic presence in some
  ways than what you have presented. yours feels very much like something
  lifted off of shit" (2026-11-02). Type is set as a typesetter sets it,
  from his faces and this skill, and a redesign keeps what today's does
  better.
- **Everything at one size and one weight.** "our typography game is still
  weak. we are not levraging different weights and sizes enough to build a
  good hierarchy of information" (2026-11-04). The shell had set nearly
  everything at 13 points and one weight, and left colour to rank it. Every
  run of text names a level, a title is larger than what it heads, a name
  is heavier than its words, and furniture is a size smaller.
- **Keys that aren't Neovim's.** "the keymap feels way too alien from vim"
  and, of the second look, "all of those ctrl keys do not feel very vim to
  me" (2026-11-02). Keys start from Neovim's modes and his leader, Space,
  never from a text box's Ctrl chords; on his ErgoDox, Ctrl is a held little
  finger. "you need to keep in mind my ergodox keyboard for shortcuts."
- **A key that isn't Vim's, where Vim has one.** Asked whether pictures,
  PDFs and video should keep their own keys or take vi's: "fuck, vi like
  ffs" (2026-11-04). When vi and a key disagree, do what Vim does, as his
  Neovim has it, and never bring him the question again.
- **A list or a text box without modes.** "i want to be able to use vi
  motion and search and shit everywhere a list can come up. i think i want
  everything modal"; and of the reader, "should feel like we are using vi
  in every way that makes sense" (2026-11-04). Every list and text box has
  vi's modes; every app of ours reads its keys from one key language,
  checked, as docs/design.md's Vi's Keys says.
- **A mode doing half of another's job.** "it seems like insert has some
  command mode functionality and not others ... we need the modes to be
  separate and have separate purposes" (2026-11-04). Each mode has one
  purpose: normal acts on what the cursor is on, / finds, : names a command
  from a menu of all of them, insert only writes words handed over. Return
  from the find line never changes anything, an arrow is its letter with no
  act of its own, Escape cancels and leaves a mode but never closes
  anything ("fuck escape quitting", 2026-11-05), and q does
  nothing: "make esc work like vi, q should do nothing by itself. :q to
  quit". A Vim user is at home with no training.
- **A sans-serif without class.** Of Mona Sans on the launcher's rows: "I
  don't like seeing sans-serif fonts in the launcher, the current one feels
  so out of place. like if the sans serif has some more class to it maybe,
  but generally, not a fan of sans-serif" (2026-11-04). His non-monospace
  is the serif: nothing of ours sets a sans unless it earns it, beating
  Fraunces and Monaspace side by side in shots at 1.5 and 2.
- **Words written for agents.** Of a research synthesis sent him to read:
  "anything that is sent for me to look at needs to be written for me, like
  what the UX canon says is not written for humans to read. i tried, it is
  too hard for me to parse" (2026-11-11). And of the looks waiting for him:
  "by the time i find them the context of why they exist is lost". Whatever
  is put in front of him to read opens with what it means for him, in short
  plain words, with an example from his own machine, and says why it's
  there and what it asks of him; sources, identifiers and numbered rules
  stay one step away, in a record for agents. docs/design.md's Written for
  Joel says how.
- **Polish that changes nothing he'd feel.** "i think you do have some
  general improvements here, but the UI still doesn't feel materially
  changed. what actually has impacted your design choices? you don't seem
  to have dug very deep" (2026-11-02). A redesign changes what he sees at a
  glance, and every choice says where it came from.
- **Changing what he likes for no reason of his.** "the lock clock font
  face is nicer as is, why the font change?" (2026-11-02). What's on his
  screen and not turned down stays, unless he gave the reason to change it.
- **Cluttered notices.** "i'm also seeing some whitespace issues with the
  notifications, feeling quite cluttered" (2026-11-02).
- **Space that ranks nothing.** "we have alignment and whitespace issues for
  padding and margins that really don't look great ... you are not
  considering enough design elements when changing things. do more research
  on design, particularly around not just how typography but white space
  gives topology and hierarchy awareness" (2026-11-05). Space is how the
  eye learns what belongs together before colour or a box tells it, so
  every gap is its level's rung on docs/design.md's ladder, each wider than
  the one inside it: a chord's keys closer than a key to its word, a hint to
  a hint closer than the hints to the facts, nothing inside a frame wider
  than the frame's own room. Two levels sharing one space, or a thing roomier
  inside than round it, reads as clutter however tidy the numbers. A change
  is judged on type, space, alignment and colour together, never one alone.
- **Too much roundness.** "you also have issues with consistency in
  padding and margin, and have often too much roundness" (2026-11-02). A
  corner is softened by the theme's radius, one unit, and parts are
  square; nothing picks a corner of its own.
- **Boxes in boxes.** Two rounded boxes nested in one dialog. Use space, a
  hairline or weight instead.
- **Inconsistency.** Spacing, sizes or weights that differ for no reason, a
  font that isn't quite Monaspace, typed text smaller than what it
  completes, a hint colliding with the line above.
- **Jarring alignment.** A heading indented from the mark above it, an edge
  that nearly lines up with another.
- **Darkness under text and boxes over things.** A shaded block behind text
  on the picture, an opaque box over the overview, an overlay covering his
  windows at rest.
- **Glow, and colour without contrast.** Glowing marks, and coloured bands
  that sit on the picture without an edge to hold them.
- **Signs to learn.** Short lines of a few colours and widths for what's on:
  "having different width lines of a few colours does not tell me
  anything". A mark pictures its thing, or its words are one key away.
- **A surface no other looks like.** The margin's drawer "really feels out
  of place with the rest of the launcher and lock screen". Where a notice
  or the launcher will do, nothing new is made.
- **Taking pixels.** "every pixel of my screen is precious": what's always
  there lives in room the windows already leave, and never moves them.
- **Floating in the sky.** Too much space round a thing, so it loses what it
  belongs to. Space groups; too much of it scatters.
- **Busy lines.** A prompt carrying everything at once, a line that repeats
  what another surface says, the machine's name where it isn't needed.
- **Wordiness.** Hints that say the obvious, such as what Escape does.
- **Haystacks and gymnastics.** A list that grows with everything he has, a
  switcher used by holding one key while hunting with another. Typing a few
  letters finds a thing; holding keys to walk to it doesn't.
- **Forgetting and spilling.** Going back and losing his place, a title
  escaping its window, a wait where an answer should be, anything that
  jerks, lags or polls.

## Judging Without Him

Judge it yourself, but don't mistake anyone for him. A panel judging in
his voice passed the second look at the desktop's keys and type, and he
failed it (2026-11-02). A reviewer checks facts, measurements and the
design document's rules; the bar is whether it could hang in a museum and
work as hard as he does, and once it's shown to him his verdict stands.

Before anything is shown or shipped, answer every check in the design
document's Every Pixel Accounted For, look at it full size, in every state,
and answer these for it:

- Is there one focal point, and does the accent mark only where the keys go?
- Could anything be taken away without losing something he'd act on or
  wonder about? Is anything he'd wonder about missing?
- Is every edge on the grid and aligned to the ink, and is the space round
  each thing the same on every side?
- Is anything in a box, a border or a shade that space would group better?
- Does it read at a glance: marks and text in the theme's colours, contrast
  passing, nothing told by colour alone?
- Is it held to its kind, so it looks like what it is and nothing else?
- Does it hold up at his scale, with dozens of windows, a long title, an
  empty list and a full one?
- Can it be done from the keyboard alone, comfortably on the laptop and on
  the ErgoDox, and found by typing what it does?
- Is there warmth at its threshold, and none in the way of the work?
- Is it smooth: no jump, no jerk, no wait it could avoid?
- Is every gap its level's rung, a line pitch, a number of beats or what a
  row leaves round its text, measured to the ink, each rung wider than the
  one inside it, and every edge on a named column?
- Does each choice say where it came from, his words, his configuration or
  his keyboards, and is a choice of the designer's own marked as one?
- Would he feel the change at a glance, beside what it replaces?
