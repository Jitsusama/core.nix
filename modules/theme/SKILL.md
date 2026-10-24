---
name: aesthetic
description: >
  Joel's aesthetic and how to build to it without asking him: his brief, the
  character of the design (modernist, Nordic and Baja, with whimsy), what
  he's liked and turned down, and the questions to judge a design by. Use
  before designing, building or changing anything he sees, types or presses:
  a window, a launcher, a notice, a prompt, a status line, terminal or
  command output, a theme, colours, type, spacing, glyphs, motion, key
  chords, a page or a document laid out for him.
---

# Joel's Aesthetic

How what Joel sees and touches should feel, found over many rounds of
building things and his verdicts on them. The system built from it, the type
and space scales, the colour roles, the glyphs, the shapes and kinds of
surface, motion and the keyboard's language, is the design document:
`design.md` beside this file where the theme module installs it, and
`docs/design.md` in core.nix. Read both before designing anything.

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
- Every choice comes with deep room to customise it, and performance always.
- He has a very poor memory, so everything has to be intuitive and work the
  same way everywhere.
- Things collapse cleanly and on purpose when space is short.
- The keyboard is the way in to everything, without exception, and it has
  to be comfortable; the pointer comes second.
- "Anything you build has to meet an insanely high quality bar. No half
  assed shit can happen. Everything has to follow my design aesthetic which
  has been refined through our work."

## The Character

In his words, a modernist, "with a bit of baja/nordic leanings, that also
loves whimsy", wanting "functionality and focus on simplicity that doesn't
constrain".

- **Modernist.** Form follows function. Type is the main material, set on a
  grid of one unit, with hierarchy made by order, weight and colour rather
  than by boxes or size. Nothing is ornament that doesn't also work: a
  border says a thing is a thing, a radius says whether it's whole or a
  part, and a glyph names an idea.
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
- **Whimsy.** The warmth that keeps minimalism from going cold: glyphs in
  their kind's colour, the padlock beside the password dots, a strong
  picture behind everything, a row of glyphs in the corner saying what's
  on, a suggestion in the field that changes each time. It
  lives at thresholds and in marks, never on the numbers or the controls he
  needs mid-task. Pure text with no glyphs grows dull.
- **Simple, never constraining.** Few ideas, each reaching far. Nothing
  useful is taken away to look clean: numbers stay, made quiet by type.
  Every choice is an option underneath, and the default is the one that
  needs no setting.

## What He's Liked

- **The text is the star, the glyphs add the impact.** A glyph leads only a
  line to act on, and text below a glyph lines up with its ink, never
  indented from it. A figure wears none, since "it is hard to tell actions
  from stats" when both lead alike (2026-10-24).
- **Braille graphs, ten characters wide.** Over a dim track of every dot,
  so a graph's ends show; block characters were "too blocky".
- **One frame, shared radii.** The launcher won when it became one border
  round a field and its results, split by a hairline, its selected row an
  inset whose curve runs parallel to the frame's.
- **Cloudy, mostly opaque.** A floating surface that "only hints at what is
  beneath", lifted by light shadows, its rows told apart by weight and
  colour.
- **The picture shines.** A strong wallpaper, never dimmed. Text on it is
  lifted by offset shadows of its own letters, and text that may lie over
  windows too by a halo, never by a dark block.
- **Things that feel physical.** Motion carries a thing the way it would
  move.
- **Glyphs one height.** A row of glyphs stands level, every one's ink the
  same height on one middle line, however tall its font draws it; uneven
  heights set off "my autism infused OCD" (2026-10-24).
- **Margin round a highlight, not padding inside it.** A selected item's
  fill sits close round its ink, with room outside it, so it doesn't float.
- **A step inside the edge.** Things in a corner sit a little inside the
  windows' edge, so nothing lines up with something else by chance.
- **Chords drawn as keys.** A key's glyph where a good one exists, its name
  where none does, joined by + with room on each side, muted.
- **Signatures, not headers.** A notice says who in strong type, then the
  message, then a muted footer: the program's glyph and name at the bottom
  left, the time at the bottom right.
- **The calm prompt.** Where he is, where his keys go, and only what's true
  besides; the rest a step away.
- **Found by typing.** The launcher as the way into everything: a few
  letters, the handful of things they could mean, each with its key beside
  it.

## What He's Turned Down

- **Boxes in boxes.** Two rounded boxes nested in one dialog. Use space, a
  hairline or weight instead.
- **Inconsistency.** Spacing, sizes or weights that differ for no reason, a
  font that isn't quite Monaspace, typed text smaller than what it
  completes, a hint colliding with the line above.
- **Jarring alignment.** A heading indented from the glyph above it, an edge
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

Before anything is shown or shipped, look at it full size, in every state,
and answer these for it:

- Is there one focal point, and does the accent mark only where the keys go?
- Could anything be taken away without losing something he'd act on or
  wonder about? Is anything he'd wonder about missing?
- Is every edge on the grid and aligned to the ink, and is the space round
  each thing the same on every side?
- Is anything in a box, a border or a shade that space would group better?
- Does it read at a glance: colour on glyphs and text in its roles, contrast
  passing, nothing told by colour alone?
- Is it held to its kind, so it looks like what it is and nothing else?
- Does it hold up at his scale, with dozens of windows, a long title, an
  empty list and a full one?
- Can it be done from the keyboard alone, comfortably on the laptop and on
  the ErgoDox, and found by typing what it does?
- Is there warmth at its threshold, and none in the way of the work?
- Is it smooth: no jump, no jerk, no wait it could avoid?
