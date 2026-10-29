# Design

How everything Joel sees and touches is meant to look, read and behave, from
the boot menu to a shell session, Neovim, pi, a browser or a chat app. Every
module that puts something on a screen follows this document, and the
`AGENTS.md` of core.nix and of every repository built on it points at it, so
an agent builds to it rather than to its own taste.

It's a living document. No existing example is purely what Joel likes, so the
aesthetic was found surface by surface, and what each surface taught is
written here. It's written so that it can be applied without him: a new
surface is judged against [the aesthetic skill][taste] and the rules here, not
handed back to him to look at. The document records the aesthetic that was
found, not the one that was assumed.

## The Taste

Joel's brief in his own words, the character of the design, what he's liked
and turned down, and the questions a design is judged by are in [the
aesthetic skill][taste], which the theme module gives every agent on the
machine, so an agent working anywhere reads it before designing. This
document is the system built from it.

## The Principles

Every surface is held to the same twelve, and a surface isn't done until each
one is answered for it in [the surfaces table][surfaces].

1. **Keys first.** Everything can be done with the keyboard alone, and done
   comfortably: a popup, a notice, a menu, the margin, a prompt, every
   surface. Each is designed around the keys and then given the pointer,
   never the other way round, and nothing can be reached only by pointing.
   Chords are short, keep to the hands' strong positions and favour a
   mnemonic letter, which keeps its meaning on any layout.
2. **Act or wonder.** Something is shown only if Joel would act on it, or
   wonder about it if it were gone. What passes is precise and quiet; what
   doesn't is one step away, behind the thing it belongs to.
3. **One home.** A fact is drawn once on what can be on screen at the same
   time.
4. **The door.** Whatever is shown opens its own detail and controls, by Enter
   or a click.
5. **Found by typing.** Every action can be found by typing what it does, in
   its layer's palette, with its key shown beside it.
6. **Hints in place.** Keys sit beside the actions they trigger, in muted
   type, and each surface can turn them off once they're learned.
7. **No hidden modes.** A mode always shows that it's on, and Escape always
   leaves it.
8. **One gesture per meaning.** Arrows move, Enter opens or confirms, the
   left arrow takes away what's typed and then backs out one step, Escape
   leaves, typing filters, and each layer has one "what's here" key.
9. **Collapses on purpose.** As space shrinks, a surface gives way in a
   written order, never by truncation.
10. **The theme, whole.** Type, colour, glyphs, space and motion come from
    the theme and change with it.
11. **Customisable at no cost.** Every choice is an option, rendered into the
    program's own files when the machine is built.
12. **Fast.** Inside the latency budgets, at the panel's full refresh rate,
    with nothing spawned or polled that needn't be.

## The Quality Bar

A surface is done only when all of this is true, and nothing is shown to Joel
for judging before it's been judged here first.

- **Driven by keys.** Everything it does has been done with the keyboard
  alone, from the chord that reaches it to Escape, and its check does it
  that way too.
- **Designed first.** Its three tiers, the facts it owns, its collapse order,
  its palette entries and its keys are written in the surfaces table before
  it's built, and judged against [the aesthetic skill][taste].
- **Seen in every state.** Each state it can be in (empty, one item, many, too
  many, long text, an error, two monitors, a monitor plugged in and out, after
  suspend, every theme) at every breakpoint, as a full-size screenshot, read
  pixel by pixel: alignment to the grid, optical spacing, baselines, how the
  ligatures fall, whether anything could be taken away.
- **Measured.** Its first frame and latency against the budgets, its motion at
  the full refresh rate without a dropped frame, no CPU at rest, no process
  started where one isn't needed. Measured with the defaults and again with a
  heavily customised configuration.
- **Whole.** A theme switch leaves nothing of the old theme behind, the
  legibility check passes, and its keys are in the map without a clash.
- **Proven.** Every bug found gets a check before its fix, and the check is
  seen failing first.
- **Tried as he works.** It's been used the way Joel uses it and at his
  scale: dozens of windows over many workspaces, on the laptop's keyboard and
  on the ErgoDox. A design that only holds up for a few things isn't done,
  however tidy its reasons.
- **Better, where a habit changes.** A key, a gesture or a flow he already
  uses changes only for something clearly better by this document's own
  terms, and the change is said plainly when it ships: what moved, where it
  went, and why.
- **Said, where it's unfinished.** Whatever hasn't met all of this is called
  unfinished when it's shown, never done.

## Type

Two typefaces from one family. Monaspace and Mona Sans were drawn together by
GitHub Next, so they share proportions, x-height and voice.

- **Monospace** lives where characters line up or the content is code: the
  terminal, Neovim, a launcher's input, numbers that change in place (a clock,
  a volume, a battery), keyboard chords, paths and commit hashes.
- **Proportional** (Mona Sans) lives where people read: GTK and Qt
  interfaces, a browser's interface, notification text, menu labels, dialogs
  and the greeter.
- **Monaspace's own art, on purpose.** Texture healing and the coding
  ligatures wherever monospace is drawn. The other Monaspace voices only where
  they mean something, such as a second voice for italics so comments read as
  handwriting, chosen by looking.
- **Numbers are typography too.** Tabular figures wherever a number changes
  in place, so it never jiggles; units set smaller than the number; the same
  precision for the same quantity everywhere.
- **One scale.** Sizes come from one base and one ratio,
  `jitsusama.theme.type`, so every surface draws from the same few steps, and
  a surface names the step, never a size. The ratio is a major third, 1.25:
  far enough apart to read as different, near enough that a heading doesn't
  shout. From a 10.5 point body:

  | Step    | Power | Size (pt) | For                                       |
  | ------- | ----- | --------- | ----------------------------------------- |
  | small   | -1    | 8.4       | units beside a number, a footnote         |
  | body    | 0     | 10.5      | almost everything: notices, meters, menus |
  | large   | 1     | 13.1      | the launcher and every prompt             |
  | title   | 2     | 16.4      | a heading in a menu or dialog             |
  | display | 6     | 40.1      | a clock shown large                       |
  | hero    | 8     | 62.6      | the clock on the lock screen              |

Each surface names which face it uses and why, in the surfaces table.

## Space

One unit and a scale built from it, `jitsusama.theme.space`, so the padding in
a menu, the gap between windows and the margin of a notification are the same
few values. At a display scale of 2, every edge lands on a whole device pixel.
The unit is 4 logical pixels, and the steps sit one unit apart where a pixel
shows and further apart as they grow:

| Step | Units | Pixels | For                                                |
| ---- | ----- | ------ | -------------------------------------------------- |
| xs   | 1     | 4      | between lines that belong together, a part's inset |
| s    | 2     | 8      | between a mark and its text                        |
| m    | 3     | 12     | between windows, between notices, a field's glyph  |
| l    | 4     | 16     | text inside its row, a terminal's padding          |
| xl   | 6     | 24     | a terminal's padding at the sides, between columns |
| xxl  | 8     | 32     | a glyph's column in a list, a chord from its name  |
| xxxl | 12    | 48     | a row of the launcher or a prompt, a meter         |
| huge | 16    | 64     | an overlay's distance from the edge                |

Whitespace groups things before borders or boxes do. Start with too much space
and take it away. Density follows the moment: dense and scannable where Joel is
mid-task (a launcher, a status line, a prompt), roomier where he reads (a
notification, a dialog, the greeter).

## Colour

The palette's colours are given meanings that hold on every surface, and a
surface names a meaning, never a colour value:

| Role       | Meaning                                                      |
| ---------- | ------------------------------------------------------------ |
| background | behind everything: windows, the terminal, menus              |
| sunken     | below the background: a sidebar, an inactive tab, a list     |
| deepest    | the very back: the overview's backdrop, a tab bar            |
| raised     | above the background: a hovered row, an indicator            |
| selection  | behind what's selected                                       |
| text       | what he reads and acts on                                    |
| strong     | text that stands out: a heading, a selected row, the cursor  |
| muted      | text that gives context: labels, units, hints                |
| rule       | separators and marks that aren't read as text                |
| edge       | the outline of what isn't focused: a window, a field at rest |
| shadow     | the shadow under a floating surface, drawn faint             |
| accent     | where Joel's attention is: one focal point per screen state  |
| on_accent  | text drawn on the accent or on a state colour                |
| alert      | something that needs him: a failure, an urgent window        |
| warning    | something that will need him soon                            |
| success    | something that went well                                     |

Nothing else gets the accent, so it always means "here". The wallpaper is
never dimmed or faded to make room for text: text set straight on a picture
is lifted off it by the theme's text shadows (`wallpaper.shadows`), a crisp
one set off down and right and a soft one around it. The margin's marks,
six pixels tall, take a shade of their own (`wallpaper.marks`): one pixel,
centred, so it's even on every side and stays in the mark's cell rather
than spilling onto the window below. Text that may lie over a
window as well gets no box either: the theme's halo
(`wallpaper.halo`) holds it off whatever is behind, shadows of the letters
in the shadow colour, centred, stacked from tight to wide so the shade is all
but solid at the ink and feathers out with no edge of its own. Muted text
sits only on the background, sunken and deepest; on the raised and selection surfaces hints
switch to text, since a muted that read there would be too bright to be
muted.

**Colour goes on glyphs, never on text.** Text keeps to the text, strong and
muted roles, so it always reads, and glyphs carry the colour, which is where
the whimsy is. Each glyph hue is a kind of thing, the same kind on every
surface:

| Role    | A glyph for                    | In Osaka Jade | For example                     |
| ------- | ------------------------------ | ------------- | ------------------------------- |
| action  | something to do                | gold          | "Open a terminal" in the launcher |
| program | a program                      | aqua          | kitty's cat, a browser's notice |
| machine | the machine itself             | rose          | the padlock, power, the volume  |
| person  | a person                       | orchid        | a message from someone          |
| voice   | the desktop speaking for itself | turquoise    | an empty list, nothing found    |

The accent is never a glyph's: it stays where the keys go, on a border and
the caret. Glyph hues are held to the same contrast as text.

A terminal program draws with the terminal's sixteen ANSI colours, which
carry no meaning of their own, and something that draws in shades, such as a
graph, takes the theme's eight shades from the background to the brightest
text.

## Glyphs

One mark per idea, in a single table in the theme that every surface draws
from, aligned to the text's baseline and never decoration. The seed is pi's:
its interface spells its marks in one place, a small geometric set (`▸ · ◆ ◇ ◈
✕ ●`) chosen because it exists in the monospace font and keeps columns
aligned, and its package tests refuse any surface that spells one itself. That
idea holds everywhere now: a mark is looked up by meaning, never typed.

The console's font holds 256 glyphs, since a font of more costs the console its
bright colours, and none of Nerd Fonts' marks, so each glyph also has a
stand-in the console can draw, and a surface running on a console (the prompt
in an emergency shell) uses those. The consoles signed in on are kmscon's,
which draws the face itself, so the prompt there has its real marks.

The table is `jitsusama.theme.marks`, each mark with what it means and its
stand-in. Every program that can read it is given it: Quickshell as
`Theme.glyph`, Neovim as `require('theme').marks`, and anything else through
`~/.config/jitsusama/theme.json`, which holds the whole theme.

## Shape and Kinds

Shape is a language too. Every visual property means one thing, the same
thing on every surface, so a glance says what kind of thing is on screen and
where the keys go. Different things look different, drawn from the same few
words, so Joel never wonders what he's talking to.

| Property                    | What it says                                                    |
| --------------------------- | --------------------------------------------------------------- |
| a border, `shape.border`    | this is a thing; its colour says where the keys go: the accent holds them, the edge doesn't, the alert needs Joel |
| the radius, `shape.radius`  | a whole thing: a window, or anything floating                   |
| a smaller radius            | a part of a thing, always the radius less the border and `shape.inset`, so the curves run parallel |
| square                      | not a thing at all: text, a hairline, a bar's fill              |
| a hairline across a surface | splits what's typed from what comes back                        |
| cloudy, lifted by a shadow  | it floats above the windows, which stay opaque                  |
| monospace                   | typing, or the machine being precise: commands, chords, figures, programs' names |
| Mona Sans                   | the system saying something in words                            |
| size                        | how much attention a kind asks, set for the kind and never changed within it |
| weight and colour           | the hierarchy within a surface                                  |

No surface picks a radius or a border width of its own; a check refuses any
that does. The kinds that follow from it:

| Kind    | For example            | Border                    | Where                 | Type              | What sets it apart |
| ------- | ---------------------- | ------------------------- | --------------------- | ----------------- | ------------------ |
| window  | a program              | the accent when focused, the edge otherwise | tiled | its own | opaque |
| palette | the launcher           | the accent                | a quarter down        | large, monospace  | a command line over a list |
| prompt  | polkit, the lock screen | the accent, the alert after a wrong answer | dead centre | large | a scrim stops what's behind it; who asks and why, in words, then the padlock beside the dots |
| notice  | a notification         | the edge, the accent while it holds the keys, the alert when critical | a step inside the window at the top right | body, Mona Sans | who first, then the message, then a footer of where from and when; Super+N gives it the launcher's keys, the footer saying them where it said when |
| meter   | the volume, the microphone, the brightness, the charge when power comes or goes, dictation while it's on | the edge | bottom centre | body, monospace | reads a level back while it changes; dictation's says what it's doing in a word, with a wave rather than a bar, so it's never the microphone's volume |
| map     | the keyboard's layout  | the edge                  | bottom centre         | body, monospace   | the keys where they sit, their layer named, following the board while typing carries on |

**Floating surfaces** are the deepest colour at `floating.opacity`, cloudy
enough to hint at what's beneath without showing it, over a blur of the
windows there. Each asks niri to blur exactly its own rounded shape, so the
blur never spills past a corner. Two light shadows lift it
(`floating.shadows`), and a prompt's scrim is the deepest colour at
`floating.scrim`. The launcher's selected row is the selection colour at
`floating.selection`, an inset inside the border.

**A glyph leads a line only when the line is something to act on:** a field,
such as the padlock beside a password or the magnifier on the command line,
or a result in a list, where the glyphs form a column. In a list a row's
glyph says what sort of row it is, so a thing to do never reads like a
figure. An act (a thing with verbs, a verb, anything Return does) wears its
own glyph, or else its thing's, in its hue. A row that holds more (an
answer, whose parts are a step in, a row of rows, a collection) wears its
glyph in the edge colour, since what it's for is inside. A figure, a value
an answer's part only says, wears none, even where Return copies it, and
its words keep the column's edge; a colour's swatch stays, since it is the
figure. In a
block of text a glyph never starts a line, since it would leave a hole under
itself or push the text off its edge. It sits inline before the word it
marks, or in a footer, as a notice signs where it came from: its glyph, a
step apart, then its name at the bottom left, and the time at the bottom
right. Edges line up to the ink, not the box, so a glyph's left edge and the
letters above it share a column.

**Chords are drawn key by key:** a key's own glyph where a proper one exists
(the NixOS mark for Super, ⇧, ↵ and the arrows), its name where none does
(Ctrl, Alt, Esc, Tab, Space), and a letter as itself, joined by + with the
same room on each side, all muted, even on the selected row. A chord sits at
the right of its row, at least `space.xxl` from the name. Nothing says what
Escape does, since it always leaves.

**A row says what it is.** A thing's row (a program, a window, a pi
session, a network, a device, a copy) says its kind at its right edge, muted:
`program`, `window`, `pi`, from one closed list, so a kind is learned once.
Its detail, a window count, a workspace or what the program is, follows its
name, muted. A row for an act or a set already says what it is in its name
("Lock the screen", "Windows"), so its chord stays at the right. A thing's
chord is taught in the foot instead, "next time" and the keys. Inside a
collection the path in the field says the kind, so its rows don't. What
the rows say at their right takes one column's room, as wide as the widest
said there since the last key and growing only, within each row's share,
so where a name is cut holds still as "working" becomes "asks you" and
back.

**A long name reads on.** A name too long for its row ends in an ellipsis,
and once selected it reads on in place: after a second's rest it slides at
a steady reading pace, eight characters a second, until its end shows,
rests, then glides back to its start and rests again, for as long as it's
selected. Only the name moves, inside its own room, so nothing beside it
does, and given up by the selection it's back at its start at once (Joel,
2026-10-25: titles "get elipsed really quick").

**A figure with a range draws a graph,** set in braille in the monospace
face and always ten characters wide, so every graph is the same size: a
bar for a level (the charge, a light, the volume), filling from the left;
a spark for a history, filling from the right with the newest last, a step
brighter; a strip for a reading of each member, such as each core, from
the left. Every dot lies under it in the edge colour at half its opacity,
so an empty graph and a full one both show where they'd end. It's muted,
text on the selected row, and the warning colour goes on a reading of 95%
or more, or all of it when the row says it warns; a row that's never a
warning however high, as a full battery isn't, says so. A row's graph sits
between its words and what it says at the right, and the graphs in a list
stand in one column, clear of the widest of those, the name giving way to
it before anything else does.

**The space from text to its frame is the same on every side,** measured to
the letters rather than their boxes. In the palette and a prompt, large type
sits in rows of `space.xxxl`, each region an inset inside the border and its
text `space.l` inside its row. A notice keeps `space.l` from its letters to
its frame, and a meter is `space.xxxl` tall with `space.l` at its sides.

**A notice sits a step inside the windows' edge,** `space.s` past their
gap, so it never lines up with a window's border by chance and reads as its
own thing. The margin's marks sit at the top centre instead, on the line
the launcher is centred on.

**The margin** is how the desktop shows what's on that Joel would otherwise
forget, without covering a window or taking a pixel from one. It isn't a
surface at all: each thing that's true is its glyph alone, set straight on
the picture in the gap the windows already leave above them, lifted off it
by a tight shade of its own that never reaches a window. Nothing true,
nothing drawn, so an empty strip says all is well.

Each mark stands in a square cell the gap's height, drawn half that, so it
has the same room above, below and on either side, and the cells touch. The
row is centred on the screen, as far from either side, on the launcher's
line, and about half as far from where Joel looks as a corner; with two
windows side by side it sits where the gap between them meets the gap
above. One coming or going slides the rest along to stay centred. A mark is
the theme's drawing of it (a glyph's `drawn`, a path on a twelve by twelve
grid), made simple enough to read at six pixels, since a font's glyph is
drawn for text and goes to mush that small; six is also whole pixels at
both of Joel's screens' scales. Every drawing touches its grid's top and
bottom and is centred across it, so a row of them is level and nothing
jumps when one comes. They read from the right by how much each asks:
what's on air rightmost (a camera, a share, a recording, a call),
then what wants Joel, then what was left on (Keep Awake, do not disturb, the
mic muted, caps lock), and within each in the overview's order. Colour says
only how much: strong for what's on air or wants him, muted for what was
left on, and the alert for what needs him now, the battery about to die or
a call ringing. Nothing else in the margin has a hue, no glyph carries a
badge, and no line underlines one, since a glyph that pictures its thing
needs nothing told twice. Something on air that goes stays fifteen
seconds in its place, muted (`margin.linger`), so a camera that was on
for a moment is still seen to have been on; nothing else lingers. A card
or a mark, never both: while a program's notice is on screen saying the
same thing, as pi's want or a call ringing does, its glyph waits for the
card to go. What comes true by itself, a camera a program opened or the
battery running low, arrives as one notice of its own, signed with its
glyph and name and carrying what can be done about it, and again, urgent,
when it comes to need Joel now; what his own keys turned on, and what was
already true as the desktop started, says nothing, since he knows. The
desktop or its apps kept waiting on the same thing, memory, the disk, the
processor or niri, for a minute shows quietly, as something left on, from
lag-watch's windows; a moment's wait never does, since Joel feels it
himself, and the launcher's "lag" says what waited.

What each is, in words, is the launcher's Now, opened by Super+O on its
most pressing row or by clicking a mark, on that mark's row: a row each, its glyph, its name, how it
stands at the right, and what more there is to know in the footer, grouped
by how much it asks with what wants Joel first. Return does its first
action and the right arrow steps into the rest. While it's open, the mark
on its screen of the row the keys are on wears the accent, as where the
keys are always does, straight up the line the list and the marks share,
so the eye joins the two. Nothing grows: anything past the gap would lie
over a window. It
stays while the overview is open, which draws nothing of the desktop's own,
and it shows in a screenshot or a share: niri can only black out a layer
it's asked to keep from a capture, which would put a black patch at the
top of every one.

What takes the keys appears on the next frame, a notice slides in from its
corner, and a prompt's scrim fades in.

## Motion

`jitsusama.theme.motion` holds the curves, durations and springs, and every
program that animates renders them, so one change retimes everything.
Anything a gesture can carry rides a critically damped spring (stiffness
1600, 98% of the way in about 150 ms), which keeps a swipe's speed and never
wobbles. Everything else runs on one ease-out curve (0.23, 1, 0.32, 1), mostly
there at once, for 100 ms when leaving, 150 ms when arriving and 250 ms for
something large and rare. `motion.slowdown` stretches all of it at once, to
look at one movement closely. Nothing
moves in a way that delays input or focus, every animation holds the panel's
refresh rate, the first frame of every surface is complete, and nothing jumps
when its content changes size. Anything that stutters is a bug.

## Latency Budgets

Joel types the moment he presses a chord, so a surface that takes the keys
has them before his first letter lands, and every key after shows its result
in the next frame. These are the budgets the principle and the quality bar
hold every surface to:

| From    | To                             | Budget                                   |
| ------- | ------------------------------ | ---------------------------------------- |
| a chord | the surface has the keys       | 30 ms, one round trip with niri          |
| a key   | what it changed, drawn         | 50 ms, and 100 ms at the 99th percentile |
| a key   | the launcher's search answered | 4 ms, leaving the frame for drawing      |

A surface meets the first by never being made at the chord. The launcher's
surface stays mapped, a clear pixel that takes no keys while closed, and a
chord reaches it over the desktop's own socket rather than a program started to
carry it. On the laptop it was measured on, a bare surface built that way
had the keys 17 to 26 ms after the chord, where the launcher making its
surface at the chord took 122 to 401 ms and lost the first letters to the
window below. niri hands a surface the keys a frame after it asks, so a
letter typed within that frame still goes beneath; the desktop check holds
that none is ever lost between the two.

## What Matters: Three Tiers

Every surface sorts what it could show into three tiers and draws them the
same way:

1. **Always:** the few things needed to act right now, such as the directory
   in a prompt, the file in an editor, the query in a launcher.
2. **When it's true:** state that changes what Joel would do, shown only while
   it holds: a failed command, a dirty tree, a low battery, do not disturb, a
   muted microphone, a failing test. Each has its glyph and its colour role.
3. **On request:** everything else, one consistent step away, behind the thing
   it belongs to.

Importance shows by order, weight and colour, not by size or boxes. Calm never
means empty: tier 1 is generous with what's genuinely useful, numbers
included, and made quiet by typography rather than by removing it. A surface
with nothing in tier 2 is calm. Something that merely interests (a temperature
while nothing is hot) sits in tier 3 and moves into tier 2 by itself the
moment it matters (the machine throttling). A thing that matters never hides in
tier 3, and a thing that doesn't never climbs into tier 1.

## One Home for Each Fact

Redundancy is the same fact drawn twice on what can be on screen at once. Two
surfaces that can never be seen together may each show it: a prompt and an
editor can both name the branch, because the editor covers the shell while it
runs, unless the shell runs inside the editor, and then only one of them says
it. Owners go by layer:

- **The desktop's facts** (the time and date, battery and power, the network,
  Bluetooth, audio, brightness, do not disturb, caffeinate, notifications
  waiting) belong to the desktop alone. No terminal program, editor, prompt or
  app repeats them. The lock screen and the greeter show the time because
  they're never on screen with the rest.
- **A window's facts** (where it is, the repository and branch, the file, what
  failed) belong to the program in charge of that window. A program nested in
  another knows it (Neovim sets `$NVIM`) and leaves out what its host shows.
- **Frames repeat nothing their content says.** The compositor draws no title
  bars. A terminal's tab bar appears only with two tabs or more and names the
  tabs that aren't in view. An editor names a file once. Old prompts in
  scrollback collapse to their folder and glyph.

| Fact                                                                   | At a glance                                        | In detail, in the launcher   |
| ---------------------------------------------------------------------- | -------------------------------------------------- | ---------------------------- |
| Time, date                                                             | the launcher's foot; the lock screen and greeter, when they're up | "time", a place's time |
| Battery, power                                                         | the margin mark, only when low; the launcher's foot | "battery", "charger"; Now |
| Network, Bluetooth, audio devices                                      | the network in the launcher's foot, in the warning colour when something's wrong | Wi-Fi, Bluetooth and Sound |
| Volume, brightness                                                     | an overlay while it changes; the volume in the launcher's foot | Sound                |
| Do Not Disturb, caffeinate, recording, sharing, mic muted, caps lock   | the margin mark, while true                        | Now; the session's rows |
| Keyboard layer                                                         | the margin mark, while locked; nothing while a key holds it | Keyboard            |
| Music playing                                                          | the margin mark while it plays; a notice for a moment when the track changes or a media key is pressed | Now |
| An agent wanting Joel, and for what                                   | the margin mark while one waits, a notice once     | Agents ("pi" finds them); Now |
| A camera open, and by what                                            | the margin mark, while a program reads one         | "camera"; Now               |
| In a call                                                              | the margin mark, while in one                      | Calls; Now                   |
| Notifications waiting                                                  | the margin mark                                    | Notifications (Super+N); Now |
| Disk space                                                             | the margin mark, only while a disk is short of room, a writer is frozen or nothing watches it; a notice once a disk is nearly full, again when the guard has made room or can't, and when it freezes a writer, each saying what to do or that nothing is needed | Now |
| RAM type, speed, maker and part                                        | nothing, since it never changes                    | Memory's Type                |
| AI spend and its caps                                                  | the margin mark, only while a finding wants a look or nothing watches it; a notice once when one starts to; a line each morning; the launcher's foot | AI spend, a part of Agents (Super+U); Now |
| Directory, repository, branch, git state                               | the program running the window                     | nobody                       |
| Window titles                                                          | a terminal's unseen tabs; the launcher's windows   | the launcher's windows       |

The margin mark is the one always-visible desktop element: a glyph at the
top centre for each thing that's true (Shape and Kinds), nothing at all when nothing is
true, so the screen is calm by default. The launcher's foot is the other
glance: one line, its keys on the right, and on the left how things
stand, always, whatever's typed, held or asked, since Joel looks there
for the battery and the network; what the selection has to say goes at
the field's right end instead. It's the
quietest thing in the launcher, below the field, the rows and the keys:
all muted but a fact that wants Joel, each fact its glyph and the least
that tells him something, as a key hint is its key and what it does, at
the hints' size and step. The battery is its charge, and its time left
while it drains; the network and the volume are their glyph alone while
all is well; the agents are what's moving. It's the time, the date, the
battery, the network, the volume, the day's AI spend and the agents, the
last two both AI's and so under the agents' glyph, drawn once for both.
Short of room, each says its least first, the battery its charge
alone; then each shrinks to its glyph alone by collapse rank, the
volume first, so every fact still shows it's there, and a glyph never
gives way: the marks have first claim on the foot, and the keys make room
for them, the arrows said by their marks alone (Joel, 2026-10-28: "i
want icons to be able to be there as much as possible"). The date, with
no glyph, goes where it would shrink, and the time never goes. Each
keeps the room it has needed since the launcher opened, so one that
reads shorter, the battery going from 100% to 99%, moves nothing after
it. The volume and the date shrink first since they're not what Joel looks there for, and
each fact's detail is a word away ("battery", Wi-Fi, Sound, Agents). Both draw from the same pieces,
so a piece with a mark is in each. The overview draws nothing of the
desktop's own: it's for finding windows (2026-10-21, at Joel's word: "having
any widgets on Super A can be abolished, it is fucking annoying relying on
it for anything i care about outside of cool window navigation shit.").

## Collapsing

Every surface writes down how it gives way as its space shrinks (a narrow
terminal split, a third-width column, a small monitor, a short editor window,
a full menu). Tier 3 goes first, then tier 2 shortens to its marks, then tier 1
abbreviates in a designed way (a path to its last parts, never cut mid-word).
The most important thing is the last one standing, and nothing wraps, overlaps
or jumps on the way down. Each surface is shown at each of its breakpoints.

## Intuitive Without Memory

Nothing may depend on remembering it. Everything is learnable on the spot and
works the same way everywhere.

- **Everything is found by typing.** The launcher is the desktop's command
  palette: every action, mode, menu and setting the desktop and the compositor
  offer is a result, found by what it does in plain words ("do not disturb",
  "rotate screen", "Bluetooth headphones"). Each result shows its key, so keys
  are learned by use rather than by study. Each program does the same inside
  its window: Neovim through which-key and its picker, pi through its command
  list, a browser through its own palettes, a chat app through its quick
  switcher. A key bound anywhere on the desktop that the palette can't find is
  a failing check.
- **Few rows, and learned.** A query leaves the handful of things it could
  mean, not a wall: each word typed has to start a word, its letters
  unbroken, so nothing is found by letters scattered through it.
- **One row a thing.** A thing found several ways is one row: a program and
  its windows are the program, saying how many, with its windows a step in;
  a pi session's window, its quest and its Agents row are the session. A
  window is found by its own title, and two titled alike say where each is.
  A desktop check types everything Joel has picked for, and fails on a thing
  listed twice.
- **The same letters land on the same row.** The name typed whole comes
  first, then a name it starts, then a later word it starts, then a word
  that only finds it, typed whole before only started: an act's name starts
  with its verb, so "quiet" is do not disturb before the volume's
  "quieter". What's
  picked most for what was typed, counting the longer words it starts, is
  first there once it's been picked two more times than anything else, so
  "sl", "sla" and "slac" all mean Slack, and "l" means what its words were
  picked for most, never what one stray pick for "light" was. One slip
  teaches nothing: it neither takes a place nor loosens one, and another
  row takes the place only once it's been picked there two more times than
  the row holding it. A name typed whole is first over anything else
  learned, unless that was picked for exactly those letters twice, so
  "slack" is Slack. A row that's gone (a notice, a closed window) counts
  for nothing, and its picks are forgotten once it's been gone and unpicked
  for a half-life. Learning only orders: every match still shows.
  Something that can't be taken back, such as powering off,
  is learned only once its first word is typed whole. When fewer letters
  would do, the field's end says so ("sl is enough"), so the habit gets
  quicker by itself, and there are no aliases to set up or remember.
- **One word each.** Every name and word the launcher knows is said
  once, in its vocabulary (`modules/quickshell/launcher/words.json`, the
  option `jitsusama.quickshell.launcher.words`): each thing's name as
  shown, the place it's in and the glyph it wears. A thing is named by
  its context's one term for it, the glossary's (`docs/glossary.md`), and
  that term is what finds it: there are no synonyms and no hidden words
  (Joel, 2026-10-25: "i don't want 1,000 synonyms for similar ideas").
  A rename never loses a row: the names it had (`once`, each one the
  glossary replaces) still find it, never shown, and its note says "Once
  called Caffeinate." so the new name is learned as it's used. A word
  may mean its own thing in another context, the crumbs saying which. A
  set is a plural noun ("Windows", "Quests"); an act is short, in
  Title Case, saying what it does ("Lock", "New Window", "Bring
  Here"), and a switch is named for what it is ("Keep Awake", "Read"),
  never for what it will do next. Implementation names are never shown or typed. An
  answer is asked by its term, or a part's name after its thing's
  ("battery health"). A word finds one thing at each strength (a name
  typed whole, an answer's question), or the vocabulary lists it as meant
  to find several ("pair" is a call and a Bluetooth device), and a check
  fails on any other. Return's word in the footer is one: the act's own verb (go,
  start, lock, mute, close, copy, set, off), a turn by the way it goes.
  Going to anything already open, a window, a program, a session or a
  notice's program, is Go To, its name never changing shape, with where
  it goes at its side ("Go To · Slack"); Open only starts what's closed.
  A quest's Go To Its Session is the one that names where, as it pairs
  with Start a Session.
  Ending the session asks once more, saying what it closes ("Reboot now,
  closing 14 windows").
- **Things hold their parts.** A thing is one place for all of it, and
  what's part of it is inside: Agents holds the agents, then their Tasks,
  Helpers and AI spend; the Keyboard holds its layout and keys, the
  keyboards plugged in and every shortcut; Slack holds its theme, so
  "slack" is Slack and "slack theme" the theme. A part is still found
  from anywhere by its own name. A program holds what controls it, the
  same for every program and each only while it applies, made as it's
  stepped into: first what can be done (Go To the window used last,
  New Window, New Window Below, and its desktop entry's own actions such
  as Chrome's New Incognito Window), then its Recent, what it had open
  lately, newest first, each the thing it is (a file, a folder, a site),
  then its other windows by title, its Notifications, newest first, its
  Shortcuts, any part of its own, such as the theme, and Quit, asked once
  since a window may hold unsaved work. Media while it has a player
  (Play or Pause, Next, Previous, and what's playing) and Sound while it
  has a stream open (Muted, Louder, Quieter, and its volume as a
  bar) come last, since they come and go. Next, Previous, Louder and Quieter keep the
  launcher open to be pressed again, its figures following; Media, Sound
  and Notifications are found only inside it.
  A setting is a part saying how it's set, its choices a step in, so a
  thing's right arrow lists a few parts rather than every choice: a
  display's Scale "175%", Refresh, Resolution and Rotation, then what
  its EDID says (Colour depth "10 bit"). What's typed beside them finds
  their choices too, so "scale 150" is Scale to 150% without a step in.
  A part is found from the top by its name, three letters or more,
  saying where it lives (Rotation, Desktop › Screens › Built-in screen;
  Health, Machine › Power › Battery), and Return goes there, the path
  filled, so the left arrow climbs back out. A part named as something
  asked whole at the top (the processor's Cores) is that answer instead.
  Only what a thing reads or holds is found so, never what it does.
- **Acts in one order.** Every kind of thing lists what's inside it in
  the same order, so three anchors are all there is to learn: the top row
  does it, what ends it comes near the bottom, and Copy is last. Between
  them, what leaves the thing as it is comes before what changes it,
  then who and where it's with (a mail's sender, a window's program),
  then its parts, settings before readings; then what ends it (Quit,
  Stop, Forget, Close), each asking once when it can't be taken back,
  saying what it ends; then Copy. A switch is one row that keeps its name
  as it flips, its note saying how it stands (Keep Awake, "on, lid shut
  too"; a notice's Read, "no"), and a setting's two states are never
  a place holding both. Return flips it and the launcher stays; one that
  would cut off something in use asks once first ("Turn Bluetooth Off,
  Disconnecting the Keychron"). A choice is said by its value alone
  ("100%", "60 Hz", "Upright"), the setting it's for named in the path
  above it, the one chosen saying "chosen", whoever picked it. "In use"
  is only ever what a program holds, as a camera Chrome has open. Rows
  that come and go, such as Media, sit at the end of their group. A check walks every row to hold
  the order.
- **Places are levels.** Desktop, Machine and Network gather what
  they're about, and a place may hold a smaller one, as Machine holds
  Power (Battery, the power profile, Keep Awake, Sleep, Lock, Log Out,
  Reboot and Power Off). Each is a row found by its word, stepping in
  lists what it holds, and its word and a space steps into it, through
  the places it sits in, so what's typed after it searches only inside
  it ("machine " and then "heat"). When a name goes on from the word and
  a space, as Power off does from Power, it's searched for where it's
  typed instead, so nothing moves under the keys; Power found by its
  name opens as Machine, then Power. A place named as a thing is gives way to the thing, and its word
  and a space still step into it; what's typed after the word that asks
  something whole ("time in tokyo", "time left") is given back whole, as
  the question it is. A place lists last the typed commands whose
  answers it gives, each with its form muted ("ip ⟨address or network⟩", "time in ⟨place⟩", "⟨amount⟩
  ⟨unit⟩ in ⟨unit⟩"), and Tab or Return on one puts its start in the
  field, out at the top, to finish there. A command is the one row
  written as it's typed, in lowercase; every other row's name, and every
  step of the path, is in Title Case as the glossary sets it, and what an
  empty list says is a sentence, above the one act that fills it where
  there is one ("No sessions running.", then Start a Session).
  The-launcher-means-one-thing-a-word holds the vocabulary to it.
- **Nothing moves that Joel didn't move.** Rows that arrive late go below
  the ones of their kind already shown, the selection stays on its thing
  as rows come and go, and the list keeps the height it opened at, so the
  foot never jumps. Only what he types orders a list by rank: a list that
  changes while he looks at it (the agents starting and stopping work, a
  window renamed) refreshes in place, each row saying what it says now in
  the order it's shown, a new one at the end and one gone staying, dimmed
  and saying so, until the list is opened again, so no row slides up
  under the keys; the selection stays on the same row, and the list
  scrolled where it was. A category held, as Agents in "ai", refreshes
  the same way. A collection says it changed only when what its rows show
  did, not each time what they're made from moves.
- **Home is a few places.** With nothing typed, the launcher shows Now,
  then after a gap the places, each a context with its own words: Agents,
  Programs, Desktop, Machine, Network and Keyboard, always in that order,
  seven rows in a card of eight, so home's shape never changes. Disk is a
  place in Machine, beside Processor, Memory and Heat, with Lag after it,
  since what slowed the desktop is one of them, and a disk filling
  fast shows in Now. Now's top never changes: Waiting on You first,
  always, saying how many wait now and later or that nothing does, a
  session that wants Joel among them, drawn as the session; then its own
  rows, always there in this order: the time, the date, the location,
  the weather and the next meeting, with no Today to step into, each the
  one row that thing is anywhere it shows, never a version Now draws of
  it. Below them, only while true, come what else wants Joel (unread
  mail, missed notices, a low battery), what reaches other people,
  what's playing and the modes left on, by name within each (the power
  profile only once picked by hand over the one the machine chose for
  the charger and the battery, since its own choice isn't a mode Joel
  left on). Waiting on You is a place, so Return steps into it whatever waits, to
  the most pressing first or to "Nothing waits on you.", and into each
  thing waiting, going to its window first inside; Super+O, Return,
  Return reaches the pressing thing, each press foreseen. Now opens on
  its most pressing row: Waiting on You while something waits now, else
  the first that wants Joel or is live, else Waiting on You; never a
  mode left on, which Return would turn off. Each place says what it holds
  after its name, muted, by its members' own names in the place's own
  order (Desktop: "This Window, Windows, Screens, Notices"), as many whole
  as fit, so a glance reminds Joel what there is; a place that measures
  something says how it stands instead, as Processor says "99% busy".
  Inside a place, things come by importance and relation, never A to Z,
  and a typed command comes last in the place whose answer it gives (Network's "ip ⟨address or network⟩").
  Tab does nothing at home, since an arrow reaches every place.
- **Tab only completes.** It takes the grey completion the field shows, and
  does nothing while none shows. The field says so at its right edge,
  "⇥ complete", only then, rather than the foot, where its coming and
  going with each letter moved the other keys or took room from how things
  stand. Its place there is kept while nothing shows in it, so nothing in
  the field moves as it comes and goes; flush after the completion, a bare
  ⇥ was neither clear nor at rest (Joel, 2026-10-28: "flush against the
  text feels wrong and they are not super obvious what they mean"). The
  keys' words take a third of the field, so they're said only while the
  path, what's typed with its completion and the aside all fit whole
  beside them; otherwise the keys stand alone, and once a step runs out of
  room they stay alone until it ends, rather than coming and going. What's
  typed and its completion come before the aside, which says less as they
  grow.
- **Every row is a thing with verbs.** The right arrow at the end of the line
  steps into the selected row, so a few letters reach inside a thing from
  anywhere: what can be done to it,
  searched the same way, each with its chord, and a verb needing a target
  asks for it in the same field. What was typed found the row, so stepping
  in takes it away, and everything inside shows. Where Joel is shows as
  one path in the field, each step named as the row he picked was, joined
  by a muted › ("Shortcuts › Desktop › Workspaces"), and a step named as
  the one before it (the Shortcuts category's Shortcuts) is said once, so
  it reads as a place, not as tags side by side. The left arrow takes
  away what's typed, whole, where the field says so, "← clear" at its
  right edge after Tab's place; what it took then stands in grey, the
  field saying "→ restore" in the same place, and the right arrow right
  after puts it back. Any other key lets it
  go, and with Ctrl the left arrow moves the caret as in any field. With
  nothing typed it steps back out,
  putting back what was typed before with the same row selected;
  Backspace in an empty field does the same. Escape always closes, and
  only the launcher: the window behind never hears it. The foot names
  every key that works on the selection: what Return does, Shift+Return
  where a row opens below, and the right arrow wherever there's something
  inside, a step in as well as at the top. The arrows are said by their
  marks alone, → and ←, which Joel knows. Where Return steps in just as
  the right arrow does, the two are said as one, ↵ →, so stepping in
  reads the same wherever it's done, and no key is said twice. Where Return
  goes somewhere else in the launcher, a spot to where it is, a command
  into the field, one that opens a collection into it, the right arrow
  goes there too, said with Return's word: ↵ → go. A step in lists
  what Return does first, where the row's own list left it out, so the
  right arrow never hides the act Return would have done (Joel,
  2026-10-28: "enter is supposed to be the default action, but that means
  that action should be selectable within"). Every row has a Return act,
  the same wherever the row shows, and a check walks every row to hold
  it. A thing with one obvious act that does no harm does it: a program
  or a window is gone to, a mail or a file opens, a network connects, a
  closed orchestrator carries on where it left off. Anything else steps
  in on Return, as the right arrow does, its likeliest act first inside,
  so Return twice does it: a person, a quest, a device, a setting, a
  place. Return never calls someone or starts something new by itself. A
  meeting joins on Return only from ten minutes before it starts, and
  before then steps in, Join first, so Joel is never made to join early
  (2026-10-26: "they force me to join, and what if I want to cancel");
  the foot says which. A value copies on Return only where it holds
  nothing more; one holding more, as the battery's answer or the
  weather's Rain, steps in, copying it a row inside: "rain" asked alone
  and Rain inside the weather work the same. Copy is never the first row
  of a step, unless it's the only one, and is always the last. Return
  closes the launcher only
  when its act takes Joel out of it: to a window, a program or a file, a
  call, or something copied to paste. An act that changes something (the
  volume, a screen's brightness or scale, Wi-Fi, Bluetooth, Do Not
  Disturb, Keep Awake, the power profile, the desk light, a keyboard's
  light or layer, a program's playing) keeps it open, and its row says
  the new state at once; Ctrl+Return does it and closes, and the foot
  says so on those rows. A copy closes saying so: the keys go back to the
  window at once, and the card stays a moment with "copied" where
  Return's key was, its done mark in the success colour, then leaves.
- **Answers first.** A sum is answered in grey right after what's typed,
  and that's its only home. Sums, bases, bits ("0b1011 << 3", "~5", "5
  xor 3"), "20% of 50", "3 squared" and measures worked out together ("5
  ft + 3 in in cm") are libqalculate's, asked in the shell's own process
  on each key, and a measure alone ("25ms") is said in its kind's base
  unit. A base, a measure in another unit, a colour in its other forms, an
  address or network (its subnet's parts, whether it holds an address, the
  subnets it splits into), a character and its codes, a hardware address,
  money, the time or the weather somewhere, or how the machine is doing is
  one row
  first in the list, named by what it says and saying it's an answer at its
  right edge, with its parts a step in, so a question never lists a column
  of bare figures. The weather's parts are Now, Rain, Wind, Today, Tomorrow
  and the day after, Rain saying when it next rains ("from 15:00, 70%")
  beside the next twelve hours' chance, and holding those hours, each its
  chance and how much; "rain", "umbrella" or "when will it rain", here or
  in a place, answers with the next rain, its hours a step in. The time
  here is the clock's, which follows the laptop into another zone once a
  Wi-Fi fix says it's there to within ten kilometres, never for a fix from
  the network's address alone; the time then says so ("clock moved to
  Edmonton time, as the laptop is in Lighthouse"), and Put It Back leads
  what's inside, holding the zone it kept until the laptop is somewhere
  else. So is the
  machine itself, in its own words: "battery" or
  "charger", "temperature" or "fans", "date" or "epoch", "brightness",
  "volume", "keyboards", and "lag" or "slow" for whether anything waited
  in the last ten seconds. The machine's own things answer the same way,
  each with its parts a step in: "cpu" (its Cores, one reading each in
  order, which are Maxed, the Clock, the Temperature as "temperature"
  says it, the programs Using It, the Load, the Kinds of core and the
  Model), "memory" (Used, Pressure, Swap, Cached, Using It and the Type
  read at boot), "disk" (the Space the disk guard says, Speed, Busy,
  Latency, Temperature, Health as UDisks has it, and the Model), "speed"
  (what the cards carry In and Out now, the Connection as the shaper last
  measured it and how full it is, the Round Trip while it's quiet, each
  server a step in, what's Dropped, and each of the Connections, VPNs
  too) and "uptime". The queue is Agents', beside the sessions whose
  heavy work it holds, and "queue" finds it: each job named in words,
  its time against its last run or what it waits for, and its quest and
  memory below. The devices hold as much: "heat"
  (each part against its critical point, the processor's Cores, the fans
  against what they're rated for, and how often it Throttled), Bluetooth
  (each device's Battery, Signal and Link Quality, Codec and Connection,
  and the Adapter), Sound (each speaker's or microphone's Level, its Volume
  or Gain, whether it's Muted, what's Playing through it, its Port where
  it has several, its Tuning and its card; an unplugged port isn't listed),
  each camera
  ("built-in camera": In Use, its lights, Streaming, Sensor and Modes),
  "keyboards" (each one's locks, Repeat and Lights, which are its own and
  not Light's, and the ErgoDox's Layer as a setting), "ip" (each
  connection's Address, Gateway and DNS, and a joined network's Signal,
  Speed, Protection and Access Points) and "battery" (its Charger, Health
  and Charge Limit). A part answers alone by its own
  words, "disk speed", "ram type" or "ping". A rate is the difference between two reads, read again
  once a second while it's shown and never for a key, and the footer says
  whether it's just now or since last asked. A figure copies what it
  shows on Return when it holds nothing more, one holding more steps in,
  and one still being read copies nothing. The start of one of those words, three letters
  or more, asks it too: "batt" shows the battery, the rest of the word in
  grey, and Tab finishes it. A row whose name starts the same way is
  chosen first, so "pow" is still Power Off. A colour's first row wears
  it, and so does the theme's nearest pigment. Return copies a sum, Tab puts
  it in place of the question, and Return or the right arrow steps into
  an answer with parts, each copied the same way. A level typed after its name, "brightness 40" or "volume
  40", is an answer that does: Return sets it. The field's end says where
  an answer came from.
  Nothing leaves the machine unless the question needs the network, never
  for a private address, and never with the launcher's `lookups` off.
- **Large sets are one row each.** The clipboard's history, past notices
  and the Wi-Fi networks in reach are too many to mix into the search: each
  that has a match adds one row, its best match with a count of the rest,
  so there's no haystack and no mode to enter first. Return uses that
  match, and the right arrow goes straight inside it, the set's crumb
  above it saying where it lives; the left arrow comes back to the
  search, since it goes where Joel came from. The whole set is found by
  its own name. The row sits below the other matches. Emoji add none, since a
  name like what was typed ("turkey" for "tu") is a row to rule out; they
  are found in their own place. A collection is found whole by its name or
  what a person would call it ("audio" is Sound, "notices" the
  Notifications), and Return steps into it; found that way, it stands alone for
  what it holds, so neither its members nor things that live in it, nor the
  shortcut that opens it, sit beside it. Each thing's row says what kind
  of thing it is at its right edge (Shape and Kinds).
  The windows, the copies and the notices are whole at home, and most have
  a chord that opens them: Super+V the
  copies, Super+E the emoji, Super+N the notices (the stack on screen
  first, by the keys, then the history), Super+Shift+/ the shortcuts.
- **A key's action is its row.** A key's action that a row already does
  is that row, drawing the key's chord: Super+L is the session's Lock,
  Super+Return is kitty, and the screenshot keys are Capture's rows.
- **Return pastes where the launcher opened.** On a copy in Super+V or an
  emoji in Super+E, Return puts it on the clipboard and pastes it into the
  window the launcher opened over, Ctrl+Shift+V in a terminal and Ctrl+V
  elsewhere (Pasting.qml); a step in are Paste and Copy, for only copying.
  Over no window, Return only copies, and the foot says which it does.
- **Copies stay in memory.** One program hears every copy for the session,
  text and pictures, and the desktop asks it for them, so none is written to
  disk, a picture's file sitting in the runtime directory, and none is kept
  that a password manager marks secret. A copy's glyph says what it is:
  several lines, a link, an address, a path and a number each wear their
  own, plain text the clipboard's, and a picture a picture's. A picture's
  row is named by its size, and the picture itself shows in the drawer
  while it's selected, in its own shape with a part's corners.
- **The drawer is the card continued** (Joel, 2026-10-28: "it should come
  out of the right side like a drawer, making sure it never passes the
  bound of the screen"). What a row has to show beyond its line (a
  picture, a notice's whole words, a mail, a meeting's details) slides
  out of the card's right side, a floating surface like the card, top
  aligned and as tall as it: its title in strong type, its facts as a
  name and a figure, a fact still being read a fixed placeholder, then
  its words, then its picture. The card's room and the drawer's are
  worked out for the screen as the launcher opens, the card centred
  unless the screen is too narrow for the drawer beside it, so the card
  never moves and never changes size for what's selected. It's out
  inside a place while any of its rows has one, so it slides only as
  Joel steps in or out, and never for a search typed at home; stepping
  between two places that both have one, it stays, and what it shows
  changes in place. Ctrl+Up and Ctrl+Down scroll it, said in the foot
  while it holds more than fits, since Page Up and Page Down page the
  list. Another program's words are shown plain, never as markup.
- **Ctrl+C copies the selected row.** Any row with a Copy inside copies
  it with Ctrl+C from wherever it's listed, without stepping in, as
  Ctrl+C copies everywhere else on the desktop; while text is selected
  in the field, it copies that text instead.
- **A notice gone is not lost.** Every notice of the session stays in the
  desktop's memory. One clicked, or withdrawn by its program because it was
  read there, was seen; one that left by itself was missed, and the notices'
  row at home says how many until they're opened. Return on one goes to the
  program that sent it, its window when one is open. A notice is one thing
  on its card and among the notices, its acts the same in the same order
  (`launcher/Notice.js`): going to what it's about, what its sender offers,
  Read, one switch saying yes or no, Forget It, and Copy What It Said last;
  the card adds what's done to the whole stack before the copy, and leads
  with Forget It where its Return forgets one with nothing to open. A
  notice on screen waits while Joel's away, thirty seconds with nothing
  typed or moved (`notices.away`), whatever keeps the screen awake, so one
  that comes while he's gone is there when he's back, counting again in
  full. Each
  program can be kept to how far its notices reach (`notices.senders`):
  the history alone, counted as waiting but never shown, a card, or, when
  it's critical, a card that stays; a notice asking for less reaches less
  far, and none reaches further than its program is kept to. The desktop
  makes one sound, softly: when a pi session first wants Joel and its
  notice is said (`sounds.wants`), never while notices are held.
- **Connecting waits to say how it went.** Return on a Wi-Fi network
  connects it and the launcher stays, its footer saying how far it has got,
  until the network takes it or says why not; connected, a password asked for
  is put away and the network's row says it's connected. Join is a call's
  word, never a network's. A locked network met for the first
  time, or one whose password changed, asks for the password in the field,
  the padlock beside the dots. The radio looks for networks only while the
  launcher is open.
- **Signing in is one door.** A network that wants a sign-in turns the
  network's piece to a warning saying so, one notice says it once for that
  network, and Sign in to it comes after the focused window at home. Each
  opens the sign-in window; nothing opens it by itself.
- **Awake on purpose.** Keep Awake, in the launcher, stops the machine
  sleeping so work left running goes on, and with power it keeps running
  with the lid shut. Without power the lid is let go, since a laptop shut in
  a bag on its battery would cook. While it holds, Keep Awake shows in Now,
  saying it's on, and the margin's mark says so.
- **What's shown is the door to its detail.** The battery opens the power
  menu, the network opens its list, a notification count opens the history.
  Nothing keeps its detail somewhere else.
- **Hints in place.** Wherever there are actions, their keys sit beside them in
  muted type, part of the design rather than a cheat sheet bolted on.
- **No hidden modes.** Do Not Disturb, caffeinate, recording, a resize, an
  editor's modes, a waiting gate: each shows that it's on, and Escape leaves.
- **One gesture per meaning.** A surface that needs a gesture of its own is a
  design problem to solve, not a feature.

## The Keyboard

Every layer's keys speak one language, so a key known in one layer tells the
key in the next. A chord says three things, in order: who hears it, what
comes along, and what to do.

- **Who hears it is the modifier.** Super is the desktop (the compositor and
  what it shows). Ctrl+Shift is the terminal. Inside the terminal each program
  has its own: Space starts Neovim's keys, and pi, where everything typed is the
  message, keeps the plain Ctrl keys every text box has. Neovim and pi sit
  side by side, never heard at once, so one may use a chord the other does. A
  chord a lower layer needs is never taken by a higher one. Print and the
  media keys do their one job bare, and each has another way in.
- **What comes along is a second modifier.** Nothing moves only focus. Shift
  carries the thing at that level, Alt just the smaller thing inside it, and
  Ctrl reaches across to another screen.
- **What to do is the same word in every layer:**

  | Key          | Means                                    | Desktop        | Terminal          | Neovim, after Space |
  | ------------ | ---------------------------------------- | -------------- | ----------------- | ------------------- |
  | arrows       | that way                                 | windows        |                   | splits              |
  | Tab          | the one used last, the others by typing  | windows        | the tab used last | the file used last  |
  | Page Up Down | the level above                          | workspaces     | tabs              |                     |
  | 1 to 9       | that one, by number                      | workspaces     | tabs              | tabs                |
  | Return       | open a new one                           | a terminal     |                   | a terminal          |
  | T            | tabs                                     | column as tabs | a new tab         | tabs                |
  | W            | close the one in focus                   | the window     | the tab           | the split           |
  | Space        | find anything here                       | the launcher   | the scrollback    | anything            |
  | ?            | what's here: every key, by what it does  | every layer's  | the terminal's    | Neovim's            |
  | Escape       | back out                                 | everywhere     | everywhere        | everywhere          |

- **Friendly on every keyboard Joel types on.** The laptop reaches Page Up,
  Home and End through Fn, and the ErgoDox reaches the symbols and Print
  through a layer, so what's done often ends on a key both have bare: a
  letter, an arrow, Tab, Return, Space or Escape. Page Up, Home, Print and
  the symbols are for what's done rarely, and anything on them has another
  way in (Super+P for Print). A chord holds its modifiers on one hand and
  presses its key with the other, and never asks one little finger for two
  keys, which is why the terminal opens a window on N rather than Return.
- **One "what's here" key per layer,** the same shape on each, showing that
  layer's actions and keys drawn from the real configuration, never a
  hand-kept copy.
- **The map is generated.** The whole map is built from the compositor's
  binds, the terminal's maps, Neovim's keys and pi's, and a check reports any
  chord two layers both claim.

In Neovim, Space and then a letter opens a group (b the open files, c code, f
files, g git, s search, t tests, T tabs, u looks), and the group then Space
finds among what it holds: Space b Space finds an open file, Space g b Space
a branch. Inside a group the shared words keep their meaning, so Space b w
closes the file, Space T Return opens a tab and Space g b Return makes a
branch. A capital letter is that letter with Shift, and does more of the
same, or more forcefully: Space b W closes the file without saving it. [ and
] step to the one before and after (a problem, a git change, a test), and a
key that only makes sense somewhere, in code a language server reads or a
file git tracks, is there only then. A window a plugin opens, the outline,
the file browser or the problems list, uses the pickers' words: Return opens,
Ctrl+S below, Ctrl+V beside, Ctrl+T in a tab, q closes and ? shows its keys.

Each program's shortcuts are data: a chord, what it does in plain words, and
the program's action, in `jitsusama.niri.binds` and `jitsusama.kitty.keys`,
and in Neovim's and pi's own `keys.nix`, rendered into each program's own
file. They gather in `jitsusama.keys`, layer by layer from the desktop down,
which refuses to build a machine where two layers claim one chord, a
sequence starts on a chord a layer above takes, or one layer claims a chord
twice in the same place, and writes the whole map to
`~/.config/jitsusama/keys.json` for the launcher and the "what's here" views.

Any key is found by what it does. Super+Shift+/ opens the launcher's
collection of every layer's shortcuts in levels, each flat, and opens it on
the program Joel came from (Joel, 2026-10-24: "ideally context aware with
the app open"): over Neovim, Neovim's groups; over a terminal, the
terminal's, a terminal running Neovim or an agent counting as that
program. At its top the layers (that program's first, then Desktop, then
the rest), each with its count and its first groups; inside a layer its
groups, by what a chord acts on (the desktop's Windows, Columns,
Workspaces, Displays, Launcher, Typing, Capture and Sound; Neovim's by its
leader's letters, and GitHub's by the window a key works in); inside a
group its chords, each with its chord and where it works, or, for one too
big to read whole, groups of its own. A level aims at the card's eight
rows, a group never holds fewer than three chords, and no layer is more
than two levels of groups deep; one chord doing one thing in several
places is one row naming each. A group found by typing below the next
level says what holds it. A layer of eight chords or fewer is its chords
alone. The groups come from a small table
(launcher/KeyGroups.js) reading each chord's action, so a new chord falls
into one without being told, and a bind the table puts wrong names its own
(`group`). Typing at any level finds a group below it by its name, then any
chord below it by what it does, flat. Inside a layer its own "what's here"
shows only its keys: Ctrl+Shift+/ in the terminal, Space ? in Neovim.
A program holds its own layer as Shortcuts, a step into its row, Neovim's
by its desktop entry since its windows are the terminal's; found from the
top beside a layer named as it is, the program stands for both, so one
Neovim is listed.

Nothing the desktop shows is reached only by pointing. Super+O opens Now,
the margin's glyphs in words, and Super+N takes the notices on screen by
the launcher's keys, then gathers every notice into the launcher. Either
way Return goes to who sent one, and the right arrow steps into its verbs:
its own actions (Reply, Mark as read), copying what it said, and letting it
go; Up and Down choose, Left steps back.

On the desktop, windows sit in a grid: columns side by side, each holding
windows stacked above and below. Super with the arrows moves focus through
it, and focus carries on over an edge the way niri lays things out, up and
down into the next workspace, left and right onto the next screen. A
modifier says what comes along:

| Held       | Carries         | With the arrows                                  |
| ---------- | --------------- | ------------------------------------------------ |
| nothing    | focus           | to the next window, workspace or screen          |
| Shift      | the column      | along the row, or onto the workspace past it     |
| Alt        | just the window | into the next column, or up and down and past it |
| Ctrl       | focus           | to the next screen, past any window              |
| Ctrl+Shift | the column      | to the next screen                               |
| Ctrl+Alt   | just the window | to the next screen                               |

The numbers take the same modifiers to the workspaces. Page Up and Page Down
walk through them, and with Shift carry the workspace itself, since it's
the thing at that level; Ctrl+Shift carries it to the screen before or
after. Super+A shows every workspace, and Super+Tab finds a window in the
launcher: every other window, the one used last first, so Super+Tab and
Return go back to it and a few letters find any other, with nothing held. A
letter keeps its meaning whatever is held: F fills, R sizes, M widens (and
with Alt stretches the window to the edges), C centres (a floating window
too), T shows a column as tabs, G lifts a window out to float, and Shift
turns a width into a height. A window opens in a column of its own, as
niri opens it; Super+Shift+Return opens a terminal just below the focused
window instead, and Super+Alt+Return does that for whichever window opens
next, from the launcher or anywhere else.

## Customising

Every choice comes with deep room to change it, and changing it never slows
anything.

- **Options, not edits.** What a surface shows, the order, each piece's tier,
  its collapse rank, its threshold, its glyph and its colour role are
  `jitsusama.*` options with documented defaults. The prompt's pieces, the
  launcher's foot, the margin mark, the editors' status lines, the boot
  splash's lines and every chord are lists that can be reordered, extended or
  cut. A new piece is a small function in the same shape as the built-in ones.
- **One shape for every surface's pieces.** `modules/theme/pieces.nix` gives
  it: pieces by name, so a machine changes one in a line or adds its own
  beside the rest, each with its `order`, its `tier` (`always`, `when` it's
  true, on `request`, or null to leave it out), its `collapse` rank (lowest
  gives way first, null never), the theme glyph it wears while it's true as
  its `mark`, its `source` (a built-in piece by name, or a file of the same
  shape) and settings of its own, such as a format or a threshold. The
  built-in settings are defaults, so a machine's own win without a fight. The
  launcher's foot and the margin's are the first,
  `jitsusama.quickshell.overview.pieces`, named for where they once stood.
- **The launcher's kinds of thing are named the same way.**
  `jitsusama.quickshell.launcher.providers` lists what it finds (windows,
  applications, actions, the session), each a file of its own with an
  `order`, a `weight` against the rest and a `source`, so a kind is turned
  off, reweighted or added in a line; `jitsusama.quickshell.launcher.learning`
  says whether and how fast it learns. `synonyms` gives a row more words
  that find it, by the key it's learned by; `home` says how many likely
  picks home shows, which lead them, and its shelf of collections; `kinds`
  says the word each kind of thing's row ends with.
- **Rendered at build.** Nix fills the options into each program's own files,
  which stay in that program's format beside the module. Nothing parses a
  setting or looks one up while running, so a customised surface is exactly as
  fast as the default.
- **Live while experimenting.** A surface can be tried from a working copy
  before a choice is committed, so tuning a detail takes seconds rather than a
  switch.
- **Documented.** Each option says what it changes, its default and why that
  default was chosen.

## Themes

A theme is one file, `modules/theme/<name>.toml`, beside the module that reads
it, in a format of our own. `jitsusama.theme.name` picks one. Nothing else
lives with it: everything a theme uses that isn't text is named, not copied.
Its typefaces and icons are packages, its wallpaper is a path in the
`wallpapers` input (Joel's own collection, where an image or a shader from
anywhere goes), and its console font is drawn from its monospace face at
build.

The format is built for how this design works rather than borrowed from
another project's. It's TOML, because Nix reads it without help and a person
reads it without a manual, and it's layered so a theme can be written quickly
and refined deeply:

```toml
name = "Osaka Jade"
appearance = "dark"   # or "light", which the desktop tells every program
icons = "Yaru-sage"  # one of Yaru's icon themes

# Eight pigments from the background to the brightest text.
shades = ["forest", "fern", "moss", "lichen", "seafoam", "parchment", "linen", "cream"]

# The pigments: every colour the theme uses, named once.
[palette]
jade = "#509475"
forest = "#111c18"

# What each colour means (see Colour). Each names a pigment.
[colours]
background = "forest"
accent = "jade"

# The terminal's sixteen colours and its own, by pigment.
[terminal]
background = "forest"
black = "forest"
bright_black = "sage"

# Code's roles (keyword, string, comment and the rest), by pigment and style.
[syntax]
string = "mint"
comment = { pigment = "sage", italic = true }

# The type scale's ratio and steps, the space scale's unit and steps, motion's
# spring, curve and durations, and any glyph to change, as the theme module's
# options of the same names spell them.
[type]
ratio = "1.25"
[space]
unit = 4
[motion]
stiffness = 1600
[marks]
cursor = { glyph = "\u25b8", console = ">" }

# How cloudy a floating surface is, its scrim and selection, and its shadows.
[floating]
opacity = "0.85"

# The pictures behind the desktop and the lock screen, by their path in the
# wallpapers input, and the shadows that lift text set on them.
[wallpaper]
desktop = "landscapes/glowing-city-green.png"
lock = "landscapes/mountain-woods-green.png"

# Still to come: the console's font.
[console]
```

The shades, `palette`, `colours` and `terminal` are required. Every other table
falls back to the defaults the theme module carries, so a theme changes only
what it means to.

Code has nine roles, each falling back to a role of the theme's: `comment`
(muted, in italics), `keyword` (strong), `punctuation` (muted), `operator`
(text), `string` (success), `constant` (machine), `function` (program),
`type` (voice) and `special` (person) for escapes, macros and attributes.
Structure goes by brightness and meaning by a few hues, so most of a file
reads as text, and the accent stays where the keys go. Neovim's colour scheme
is drawn from them, and the legibility check holds each to 4.5:1 on the
background and the sunken shade. Colours may carry alpha (`#rrggbbaa`). Roles always name a
pigment, never a value, so changing one pigment recolours everything that
means it. A pigment's name says what it looks like (jade, parchment, night),
never what it's for, which is the role's job.

**Bringing a theme in from elsewhere** is a translation into this format, done
once by an agent and committed, so evaluation only ever reads one format.
There's no importer to maintain: the `translate-a-theme` skill, in
`.agents/skills/`, guides an agent through it for whatever the source is
(Omarchy's `colors.toml`, a base16 or base24 scheme, a kitty or Ghostty theme,
an iTerm2 `.itermcolors`, a VS Code colour theme, a Neovim colour scheme, or a
screenshot). It says how to name the pigments, how to choose each role and
where the usual formats keep each colour, to mark every role that was a
judgement call with a comment, to run the legibility check, and to screenshot
the result beside the source before calling it done.

- **End to end.** A switch changes everything on [the surfaces
  list][surfaces]: the boot splash and the disk unlock prompt, the console's
  colours and font, the greeter, the compositor, the desktop, the apps, the
  terminal, the editors and pi.
- **Nothing names its own.** No surface names a colour or a font; it names a
  role or a face. A check holds that across every module, listing each
  remaining exception with the reason until it's gone.
- **Legibility is a check.** Each theme's text pairs are measured for WCAG
  contrast as the machine is evaluated: 4.5:1 for text, 3:1 for large text and
  glyphs. A theme that fails doesn't build.
- **Proven whole.** Swapping themes rebuilds the initrd, so the proof goes the
  whole way: switch, reboot, the TPM still opens the disk, and the new theme is
  on the first frame after the firmware's.

## The Studio

For each layer, before it's built, two or three distinct directions for its
most-seen surfaces are built for real, in the theme's type and colour, and
judged side by side at full size on the panel they're for, against [The
Taste][taste]. The best is refined in rounds and built, and what was learned
goes under [Decisions][decisions] and into [the aesthetic skill][taste]. A
round is shown to
Joel only where the document can't decide it.

Each round compares a dense version beside a sparse one, the defaults beside a
customised configuration, and the panel beside a small monitor.

## Surfaces

Every interactive surface, from power on to the last app. Each row is filled
as its layer is built: its tiers, the facts it owns, how it collapses, its
palette and its keys, the keys first, since a surface that can't be driven
by them isn't done.

| Surface                                   | Module                   | Faces          | Tier 1 | Tier 2 | Behind it | Palette | Collapse |
| ----------------------------------------- | ------------------------ | -------------- | ------ | ------ | --------- | ------- | -------- |
| The boot menu                             | lanzaboote, systemd-boot | the firmware's |        |        |           |         |          |
| The boot splash and unlock prompts        | Plymouth                 | both           | the machine's name, what's happening (the disk is locked, starting, restarting, shutting down), the prompt and what it wants: PIN, passphrase or recovery key | a refused one, framed in the alert colour, saying try again; a message the boot sends | Escape, Plymouth's log of the boot | nothing | nothing gives way; drawn at one or two to one by the screen's height, so the type stays sharp |
| The console and emergency shell           | the console font         | console        |        |        |           |         |          |
| The consoles signed in on                 | kmscon                   | monospace      |        |        |           |         |          |
| The guide                                 | guide                    | monospace, the console's stand-ins on a console | the journey's name and how far along it is, muted at the right; the step's title; the one thing it asks now, in words, with its field: the padlock beside dots for a secret, the cursor for a word typed back, which stands strong above it, or a list of choices, the cursor in the accent beside the one the keys are on, in strong; the keys that work now, muted, at the foot | what the step before showed, muted, under the name; a refusal under the field, by the failure mark in alert; a step that can't run, by the caution mark, with what to do under it; what to do while a step works, such as touching a key | the raw log, by Tab once a step fails | nothing | what the step before showed, then how far along; words wrap between themselves, and a mark's text wraps under its ink |
| The greeter                               | Quickshell, greetd       | both           | the time, the date, who's signing in, the session it starts | a refused password, what PAM says | Tab to the next account | the lock screen's | the boot's timings; the time stays |
| The compositor                            | niri                     | none           |        |        |           |         |          |
| The wallpaper                             | Quickshell, niri         | none           |        |        |           |         |          |
| Dictation                                 | dictation, Quickshell    | monospace      | nothing | while it listens, at the bottom centre in a meter's frame: a person speaking in the voice colour, listening, a wave of what the microphone hears, and the time, counting down the last five of Whisper's thirty seconds with the glyph in the warning colour; typing, once it lasts long enough to see; then for a moment heard nothing, still writing, or, with the muted microphone, the microphone is muted | nothing | Super+D starts and stops it | nothing; it's never in the margin, and never a notice while the shell watches |
| The launcher's foot and the margin mark   | Quickshell               | both           | the foot, when the selection says nothing: the time, the date, the battery with its time and watts, the network, the volume, the day's AI spend, pi, each after its glyph | the battery running low, offline, a network wanting a sign-in, muted | step 2's doors | step 2 | volume, network, battery, date; the time stays |
| The launcher and palette                  | Quickshell               | monospace      | what's typed; the selected row: its glyph, its name, its kind or its chord; Return's verb at the foot's right | the foot's note: "sl is enough", the chord that opens a thing next time, where an answer came from | a thing's verbs, a collection's members and an answer's parts, a step in by the right arrow | the accent frame, a command line over a list split by a hairline, a quarter down | past eight rows the list scrolls, keeping the height it opened at; a note takes the glance's place in the foot |
| Notifications                             | Quickshell               | both           |        |        |           |         |          |
| Menus and overlays                        | Quickshell               | both           |        |        |           |         |          |
| The lock screen, polkit and PIN prompts   | Quickshell               | both           |        |        |           |         |          |
| GTK and Qt apps, file and print dialogs   | gtk, qt                  | proportional   |        |        |           |         |          |
| The browser                               | chrome                   | proportional   |        |        |           |         |          |
| Images and drawings                       | swayimg, gimp, inkscape, Quickshell | proportional | nothing | nothing | nothing | the launcher's rows, in Capture after what it captures: edit the newest screenshot in GIMP; look through the screenshots in swayimg's gallery; in swayimg, e edits, y copies, Delete trashes, q closes | swayimg's name and place go two seconds after each picture opens |
| Documents                                 | zathura                  | proportional   | nothing | nothing | nothing | zathura's own keys | its bar and command line show only while they're asked for |
| Music                                     | spotify, Quickshell      | both           | nothing | a note in the margin while any player plays; for a moment, the track as a notice | What's on: the track, artist, album and player; pause, next, previous, the player's window, and pausing a second player | Play or pause the music, Play the next track, Play the previous track, on the media keys | the mark carries no words; a title wraps to two lines, then gives way |
| Pairing                                   | tuple, Quickshell, niri  | both           | nothing | a phone in the margin while in a call, and wanting Joel while one rings; for a moment, who joined or left, and that he's the only one left; a ring, and a missed call kept | What's on: how long, and who with or who's sharing; leave, share the screen, mute him in Tuple, copy the room's link; while ringing, show the call | the launcher's rows: show a ringing call, first at home; open the room and copy its link; join a copied call's link, first at home once copied; leave, share, mute; a tuple: link from the browser joins its room | the mark carries no words; What's on says how long and who with |
| AI spend                                  | spend-watch, Quickshell  | both           | the launcher's foot: today's spend against usual ("$214 today · usual") | the margin's glyph while a finding wants a look: a cap near, over or refused, an override ending, a cap lowered or lapsed, a credential nobody knew spending, a source signed out, or the watch stopped or on hold; a notice once for each, at most three a week beside a cap's, and one line each morning | What's on: how it stands, the day's likely end, the week, and how the figures were got; go to the session, ask for more, sign in, it's expected, read the week's report, show where it went | the launcher's rows: "spend", "cap" and a report by its words ("spend last week by quest"), Return copying it as markdown and the right arrow stepping into its rows; Super+U opens the collection: what wants a look first, then today, each cap longer than a day and earlier, a day's cap behind today and the next seven days behind the week's cap, each a figure of a few words with the rest in the footer and its parts a step in (a period's cuts and their rows, a cap's changes and its sign-in), then what was found | the row gives way before the network; the mark carries no words; while sharing, every amount gives way and the words stay; a source that can't be read says nothing for its first fifteen minutes, as a machine waking finds its network, and no more than a note unless only a sign-in fixes it; a history a newer watch keeps, as when an older system boots again, is left as it is, the watch on hold and saying so until that one runs; a cap's allowance that changes for one reading and comes back says nothing, a change being told once it has stood ten minutes unless the cap warns or refuses at it, and a fall stops asking once the cap is back |
| Chat, passwords, calls                    | their own settings       | proportional   |        |        |           |         |          |
| The terminal                              | kitty                    | monospace      |        |        |           |         |          |
| The prompt                                | prompt, kitty            | monospace      | where Joel is, told from the nearest root (the root in text, the folders between muted, the folder he's in strong); the glyph where his keys go, in the accent, red after a failure, # in gold as root | another machine, @ and its name; a rebase, merge or pick under way, a rebase drawn as its steps, red while a conflict waits; a development shell and jobs as glyphs; a long command's time; the branch, muted, then git's notation: * not staged, + staged, ? untracked, together, then ↑N ahead and ↓N behind, together, and nothing when clean and in step; over ssh in kitty, the window a little towards the machine hue | git status, and a wider window | nothing | laid out per prompt, never per keystroke: a long run's time, the shell's glyphs, the branch's middle, the path's middle, the branch to its glyph, the right side, then what's under way to its glyph; what gives way goes without a trace; the left keeps to a third of the window; zsh clears the right side once typing reaches it |
| Terminal programs                         | each tool's module       | monospace      |        |        |           |         |          |
| Neovim                                    | neovim                   | monospace      | the status line's one line at the bottom, on Neovim's own background: the file told from its root as the prompt tells a folder (the root in text, the folders between muted, the file strong), or a buffer's kind by name (help, terminal, quickfix, a plugin's window) | Vim's [+] while the buffer has edits not yet written, a padlock on a file that can't be changed; what git is in the middle of, drawn as the prompt draws it; the mode, in the accent, whenever it isn't Normal; what's selected, in lines or characters, or which match of a search; errors and warnings, each its mark and count; a language server at work; the agents' glyph while an agent is paired; the branch, muted, with the prompt's git notation; a tab line only with two tabs or more | the problems list, the outline, git's status | nothing | the language server's words, the branch's middle, the path's middle, the branch to its glyph, what's under way to its glyph, the path to the file's name, the right side but for the problems, then the name's own middle; what gives way goes without a trace |
| pi                                        | its theme, its harness   | monospace      | the status line's one line at the bottom: where pi works, told from its root as the prompt tells a folder (the root in text, the folders between muted, the folder strong); the model, muted; context's mark and reading, and what the next turn costs | what git is in the middle of, drawn as the prompt draws it; how hard the model thinks, its mark in the level's colour and the word muted; each extension's mark while its state holds; the panel's height when it isn't the usual one; the session's total; the branch, muted, with the prompt's git notation | each mark's own command (`/guards`, `/jobs`), `/settings` | nothing | the total, the branch's middle, the path's middle, context to a share, the model to its short name, how hard it thinks, the branch to its glyph, what's under way to its glyph, the path to the folder's name, the model, then the name's own middle; the next turn's cost never; what gives way goes without a trace |

Where a surface can't take the design (web pages in a browser, a chat app's
messages), the row says so, and what's themed is what can be reached. The
firmware's setup screens and its own boot and update menus are the firmware's,
and stay outside.

## Adding a Program

Whatever is added later goes through the same questions before it's
considered done:

- Does it take the theme, or at least follow the dark preference?
- Do its notifications go through the desktop, so do not disturb covers them?
- Are its keys in the map without a clash, and is its palette in the map too?
- Does it repeat a fact another surface owns, and can that be turned off?
- Can it show a secret, and if so, is it blocked out of screen shares?
- Does it start, draw and idle within the budgets?

A program that fails and can't be fixed is replaced, or written into its row
as an exception with the reason.

## Decisions

What each studio round settled, newest first, with the reason.

- **Actions read apart from figures, and a figure draws its range in
  braille** (2026-10-24, at Joel's word: "it is hard to tell actions from
  stats"). Every row wore a glyph in its kind's hue, so the battery's
  health and Turn on do not disturb led the same way. Now the glyph says
  the sort of row: an act its own in its hue, a row holding more its glyph
  in the edge colour, and a figure none. In the approved spike graphs were
  drawn in braille, ten characters over a dim track of every dot, after
  block characters read as "too blocky"; the track shows a graph's ends
  whatever it holds, and one width keeps every graph in one column.

- **The margin's marks sit at the top centre, lit rather than grown**
  (2026-10-24, at Joel's word on the drawn marks: "i don't like the line
  under the teacup"; "they also seem to be shadowing on top of a window
  which i don't like. they seem to still be breaking my aeshetic
  somehow"). The marks wore the lock screen's text shadows, one set off
  down and right, which is lopsided at six pixels, and one sixteen pixels
  soft, which reached past the twelve pixel gap onto the window. Grown to
  twice the gap in What's on, they crossed the windows' borders wherever
  they sat, as anything past the gap must. So each mark has a shade of
  its own, a pixel round it, and nothing grows: the accent alone points,
  since Joel is already looking once he's asked. Research on placement
  found the top centre about half as far from where he looks as a corner,
  less crowded and symmetric, and the iPhone's island grows from there;
  against it, status sits at the right on every desktop, and a centred
  row shifts when one comes, so it slides. The cup lost its saucer, whose
  line read as a mark of its own.

- **The margin's marks are drawn, each in a square, and grow in What's
  on** (2026-10-24, at Joel's word on the built glyphs: "the icons are super
  small and feel very tight"; "everythng has to be symmetrical for my
  OCD"; "i want them to be inset from the right edge to where a maximal
  windows right edge should be and slightly inlayed from that"; "when i
  open what's on, they enlarge and overlay"). Measured on the laptop, the
  gap above the windows and the one beside them were both twelve pixels;
  the glyphs looked tight because they filled the strip, two pixels off
  each side, eight apart, and ended twenty pixels in, lining up with
  nothing. A taller gap would take room from every window, which he
  wouldn't give, so each mark got a square cell with the same room all
  round, the row ending on the windows' edge. Font glyphs went to mush at
  six pixels in the mock, so the theme draws its own, and the struck bell
  and microphone cut a gap along one side of the slash, since one filled
  over the mark read as neither.

- **The margin is glyphs alone, and the drawer goes** (2026-10-24, at
  Joel's word: "having a little square in my top right doesn't really let
  me know what is happening"; "having different width lines of a few
  colours does not tell me anything"; "every pixel of my screen is
  precious"). The research behind it (calm and peripheral displays, the
  aircraft's dark cockpit) found a colour or a length can't say which
  thing is which out at the edge of the eye, and an unlearned sign is
  slower than a picture of its thing. Three rounds were mocked at full
  size in the theme. Pulls, badges and coloured wants read "too busy", and
  the drawer "really feels out of place with the rest of the launcher and
  lock screen", since no other surface was shaped like it. Glyphs at
  their font's own heights set off "my autism infused OCD", so each now
  stands the same height, inside the gap the windows already leave, since
  a taller gap would move every window. The third round, "it seems nice i
  think", is what's built: glyphs alone, level, coloured only by how much
  each asks, with their words in the launcher's What's on, a surface Joel
  already knows. A sixteen-pixel gap read better in the mock but cost
  every window four pixels, so twelve stays.

- **A row says its kind, and a thing is one row** (2026-10-23, at Joel's
  word: "seeing multiples of the same thing gives me executive
  dysfunction"; "i have an impossible time knowing what i'm looking at").
  Four ways of saying a row's kind were drawn at full size in the theme's
  own type: the kind at the right edge on every row, its chord moved to
  the foot; the kind in a narrow column after the glyph; no kind, as
  before; and the kind at the right only on a thing's row. A column of
  kinds pushed every name right and read as a table, and no kind left
  "Slack" the program and "Slack" the window looking alike. A sentence
  ("Lock the screen") already says what it is, so only a thing says its
  kind, muted, and an act keeps its chord where it was. Typing "tuple" had listed
  the program, its window, its room and people by scattered letters; the
  program and its windows are one row now, found by words, and a window
  only by its own title.

- **The launcher has one foot, and a held notice speaks its keys**
  (2026-10-22, at Joel's word: "i don't like losing the context hints in
  the status bar, the look of the status bar breaks my aesthetics hard.
  some of the new interactions feel slightly out of place with the other
  ones that i have been liking."). A second line under a hairline, in
  small type, with a word before every value and dots between, was a
  second foot and a busy line, and took the hints' place. The foot is one
  line again: the keys on the right, the selection's note on the left, and
  when it has none, the glance, each fact its glyph and the least that
  tells something, at the hints' size and step and all muted, the two
  crossfading; at Joel's word ("it needs to feel fucking polished ... and
  also not steal visual focus"; "feel free to reduce what it shows
  depending on context"), it's whole only at home, and mid-task only the
  time and what wants him. The held
  notice chose its offers with the side arrows, which in the launcher step
  in and out, and said its keys in a second footer; now the right arrow
  steps into its verbs as rows, the history's own, and the one footer
  says the keys where it said when.

- **The splash says what it waits on** (2026-10-21, judged against the
  taste without a round). A boot or shutdown that stalled said starting or
  shutting down and nothing more, and a restart's screen froze when its last
  stage began. Once a job has run three seconds, the line under those words
  names it, in the body face and in the description systemd gives it:
  waiting on Docker Application Container Engine. One is named at a time,
  the longest running, until it ends, so the line never flits; a job that
  ends in time is never named, so a calm boot shows nothing new. A
  shutdown's last stage says what it's doing in the theme's words, sent as
  keywords. The line sits in the prompt's place, a step under the words it
  adds to, and goes while something is asked. Live words are lifted as the
  pictures' are, the wide shadow drawn once as a halo stretched to their
  width, since Plymouth can't blur, so the boot's own messages now share the
  body face and the lift. systemd's status lines are kept off the console,
  which they woke over the firmware's logo before the splash could draw.

- **A failed boot says what stopped it** (2026-10-18, judged against the
  taste without a round). A boot that drops to the emergency shell, the
  initrd's or the system's, showed systemd's status lines and sulogin's
  words, and with root locked sulogin opens nothing, so nothing said which
  unit failed. Before the shell asks, each failed unit is listed: the
  failure mark in alert, the unit strong with its description beside it,
  and the last its own programs said muted under it, wrapped between words
  rather than cut. A line of air parts the units, and the way out comes
  last with its keys strong: Ctrl+Alt+Delete restarts, and Space held as it
  starts gives an older generation. A shell asked for with nothing failed
  adds nothing.
- **A generation that won't boot is passed over, and said** (2026-10-17,
  judged against the taste without a round). The boot menu stays hidden:
  systemd-boot never shows it for a generation out of tries, and a menu
  is a question asked at the worst moment. A new generation gets three
  boots to bring the machine's services up; after that the last good one
  boots by itself, and once the session can show notices it says which
  generation failed, as a critical notice that waits to be read.
- **The console wears the theme's face** (2026-10-17, judged against the
  taste without a round). The console drew in the kernel's Terminus, square
  and technical beside everything else. It now draws in the theme's
  monospace face at Medium, drawn at build into the kernel's own 16 by 32
  cell: at one bit a pixel, the regular weight's stems fall on one pixel in
  some letters and two in others, where Medium's stay at two. Box drawing is
  the face's, carried to the cell's edges so lines join; the blocks are
  drawn exact. It's loaded in the initrd, so the disk's prompt wears it too.
- **The consoles signed in on are kmscon's** (2026-10-21, at Joel's word:
  "the glyphs aren't working on console, so the prompt looks like shit,
  also, the font looks very dissimilar and ideally would be slightly
  larger. also there is no padding on the edges of the screen"). The
  kernel's console can't hold the prompt's marks, draws a bitmap that only
  approaches the face, and runs its text to the screen's edge. kmscon draws
  the face with Pango at the terminal's size, padded as the terminal is and
  in its colours, so a console reads as the terminal does. The kernel's
  console keeps the boot, the disk's prompt and the emergency shell, where
  kmscon can't run. Above the login prompt, either console greets the
  account as the greeter does and gives the keys back to the desktop, drawn
  as the failed boot's are, in place of NixOS's banner, whose console name
  kmscon gives as its pseudo-terminal.
- **niri's own screens never show** (2026-10-17, judged against the taste
  without a round). niri draws its screenshot screen, its key list, its
  question before quitting and its banner for a configuration it can't
  read in near-black boxes with white words and red or green borders, and
  no theme reaches them. Rather than patch four screens to wear the theme,
  none opens: the launcher shows every layer's keys, Picker.qml picks a
  region by the keys, Log Out quits without niri's question, and the
  build refuses a configuration niri can't read. A check holds the keys
  and Log Out to it.
- **The panic screen wears the theme** (2026-10-17). The log a panic draws
  for ten seconds before the reboot is in the theme's text on its
  background, like the console and the boot, not white on black. The
  kernel takes colours only as it's built, so another theme rebuilds it.
- **The rest of Neovim's screen goes quiet** (2026-10-16, judged against
  the taste without a round). The line's own number sat a column left of
  the counts around it, in muted; it now stands in their column, in
  strong. Tildes past a buffer's end, a fold's "+--" and row of dots, and
  the dashes in a diff's gaps are gone: a closed fold shows its first line
  coloured as code on the fold's shade. A line cut at a window's edge or
  foot ends in the more mark, not the prompt's mark or @@@. A terminal is
  named by its program, not its term:// address, and a file fugitive
  opens at a version is told from its root with the version after it,
  beside its branch. Every file's mark takes the text's colour, as the
  finder's already did, and trouble lists problems flat under each file,
  named from where Neovim is, without the whole path or tree lines.
- **pi's panels keep the accent for keys** (2026-10-16, judged against the
  taste without a round). A gate's edges, title and scrollbar were all in
  the accent, beside the one key that was, while the footer's keys were as
  muted as their words. Now the edges are hairlines in the rule colour, the
  title is bold in strong, the scrollbar takes the theme's scrollbar
  shades, and every key the footer names is drawn as a chord: the key in
  the accent, its word muted after it. Tab and Ctrl+# gained the words
  they lacked, and the view on screen is marked by its word in text. The
  review gates' rows moved one column right, onto the margin every other
  row keeps. A gate docked above pi's editor leaves its closing rule to
  the editor's border rather than lay a second hairline on it. Three
  accents stay because they're keys or a state: a code to type, a
  pipeline's current stage and the active tab's number.
- **kitty's cursor rests** (2026-10-16, judged against the taste without a
  round). The block in strong, fading on WezTerm's 800 ms, stays: it's
  legible and quiet while typing. It used to blink for as long as a window
  had focus, motion on a screen nobody's touching and an animation kitty
  draws all the while; it now rests, solid, 15 seconds after the last key.
- **octo's keys speak the language** (2026-10-16, judged against the taste
  without a round). octo's buffers held 117 keys of its own in words like
  "Buffer close (close issue)" and emoji, with its defaults still bound
  beside them, and none in the key map. Every octo action is now in
  keys.nix in plain words, and octo binds nothing else. Where the
  language has a word, octo's keys use it: Return adds and w takes away or
  closes, Ctrl+L reads again, and [ and ] step with the thing's letter,
  [ C a comment or thread and [ v a changed file, so octo no longer gives
  the quickfix list's and the symbols' steps other meanings. The
  notifications' keys, which never worked behind Space, are the finder's
  Ctrl+R, Ctrl+X and Alt+X.
- **Space finds in every one of Neovim's groups** (2026-10-16, judged
  against the taste without a round). The language says a group then Space
  finds among what it holds, but eighteen groups, git, search, tests and
  looks among them, had no Space. Where a group holds things, Space finds
  one; where it holds things to do, Space finds one of its keys by what it
  does and runs it as if pressed, listing only the keys that work in this
  buffer. Found by typing, so nothing has to be remembered past the group's
  letter. neovim-finds-in-every-group holds that every group has one.
- **Neovim pairs with one agent, pi** (2026-10-16, judged against the
  taste without a round). Neovim carried three: Claude Code's plugin on
  Space a, which started a server in every Neovim for a Claude Code that
  never came, opencode's, which nothing ever loaded, and neovim-pi. pi is
  the agent in use, so neovim-pi stays and the other two go, and Space a
  with them until neovim-pi has keys worth giving it. fzf-lua was already
  the one picker; Snacks' picker is never turned on.
- **pi's status line speaks the prompt's language** (2026-10-16, judged
  against the taste without a round). It drew the folder from home in a
  colour of its own, the branch beside it with no marks, fields between bars,
  the panel's usual height as a square, and how hard the model thinks as an
  emoji and a small number. It now tells the folder from its root, by the
  prompt's own markers, and keeps the branch at the right end, muted, with
  the prompt's marks from the prompt's git-engine, asked when something may
  have changed it, so nothing polls. Fields are two spaces apart. How hard
  the model thinks sits beside the model, a mark in the level's colour and
  the level's word muted; the panel's height shows only when it isn't the
  usual one. pi-tells-where-as-the-prompt-does holds that the two agree.
- **Neovim's finder is a float like the rest** (2026-10-16, judged against
  the taste without a round). fzf-lua draws on the floats' shade with a step
  of it round the list and the preview, dims nothing behind it, and takes
  the theme's colours: the prompt's mark in the accent, a hairline under
  what's typed, the chosen row in the selection's colour. Its title starts
  at the list's edge, over the prompt. A file reads as its name in strong
  and then its folder in text, since muted words can't be read on the
  selection. The folder isn't in the prompt, nor the file over the preview:
  the status line names the folder, and goes on naming the file under a
  float rather than the float itself. neovim-floats-by-shade holds it.
- **Neovim's floats stand apart by their shade** (2026-10-16, judged against
  the taste without a round). A problem in full, a language server's note, a
  notice, the suggestions, the outline and a change's preview are each drawn
  on the floats' shade with a step of it round their words, never a line
  round them, and nothing behind shows through. A notice is its mark and then
  its words, the mark in its level's colour. A suggestion is its word and the
  mark of its kind; the kind's name and its source said the same on every
  row, so they went. neovim-floats-by-shade holds it.
- **Neovim starts with only what draws its first screen** (2026-10-16, on
  Joel's word that it be as fast as the rest). Six plugins load as it
  starts; the rest wait for a module, a command, an action or an event,
  and the background ones load once the screen is drawn. Nothing it does
  as it starts waits on another program. The Lua language server reads
  Neovim's own runtime and roots at the prompt's markers, so it answers in
  seconds rather than a minute. neovim-starts-with-only-what-it-draws
  holds it.
- **Neovim's status line speaks the prompt's language** (2026-10-16,
  judged against the taste without a round). lualine drew powerline arrows,
  a mode block on a shade, a tab line repeating the root and branch, and
  breadcrumbs, and couldn't draw the prompt's path or marks. The status line
  is now Neovim's own: the file told from its root as the prompt tells a
  folder, the mode as a word in the accent after it (where the prompt puts
  its glyph, so nothing to its left moves), and on the right the branch,
  muted, with the prompt's marks. Fields on the right are two spaces apart,
  a mark and its count one, so each count reads with its own mark. Git's
  state comes from git-engine, asked when something may have changed it,
  so nothing polls. Hints and notes stay in the sign column and the problems
  list, since they change nothing Joel would do. Where the cursor is goes
  unsaid: the number column gives the line, and Ctrl+G tells the rest when
  asked.
- **Eight of Monaspace's ten ligature sets** (2026-10-16). The sets are off
  until asked for, so kitty drew texture healing alone. Now fontconfig gives
  the face equals, comparisons, arrows, tags, Markdown's marks, the centred
  colon, double arrows and the other tags, each still reading as the operator
  it stands for. The F# shapes stay off, since they draw |> and <| as
  triangles, which are characters of their own; so does the centred period,
  which draws a range such as 1..=10 in middle dots. kitty still shows the
  plain characters under the cursor.
- **Told only what costs access or money nobody chose** (2026-10-15,
  from a replay of every detector over 26 weeks after the burn). A new
  model sent a notice ten times, each a model chosen on purpose, and a
  day leaning on one model was listed 32 times, every one the week after
  a switch to a newer one. A new model or tool is now listed, a day's
  mix of models is the report by model's, and what's told is a cap, a
  refusal, what the provider says, a sign-in only Joel can give,
  a day on course for twice the costliest of eight weeks, pi spending
  from sessions no log here knows, and a credential nobody knew
  spending, which may be a leaked key.

- **A burn is a working hour's pace, and only a note** (2026-10-15, from
  Joel reading the findings). Agents left working overnight raised the
  loudest alert, since the burn judged 4 a.m. against the $3 that hour
  usually costs, though they spent what a busy afternoon does. Now an
  hour runs hot only past twice a working hour, whatever the time, and a
  burn is listed in the collection, never told: running into a cap is
  what costs access, and the caps already say when that's likely.

- **The AI spend collection, in three rows** (2026-10-15, from three
  directions drawn side by side: the facts as a list, the cap first, and
  the fewest rows). The first screen said the week twice, this week since
  Monday beside the cap's last seven days, and held a premium day at 0%
  and a forecast nobody had asked about. Now it's today, each cap longer
  than a day, then the periods before: a day's cap is behind today and the
  next seven days behind the week's cap, whose days are the week that can
  stop work. The periods once sat behind an Earlier row, which put a
  period's cuts four steps deep, so they're in AI spend itself. What's near a
  cap leads as a finding, so the rows can stay plain. The margin's header
  stays how today stands, since a cap's share there would run past it.

- **The AI spend collection, laid out as the others** (2026-10-14, after
  its first boot). The first collection drew whole sentences beside its
  rows, which ran over the names, said the week three ways, and listed
  every report and every source's error at the top, so it read as a mess
  and a boot without network sent three notices. Now each row is a name
  and a figure of a few words, its detail in the footer and its parts a
  step in, each fact once: a cap's dollars in its footer, the basis only
  when it's weaker than a bill. A finding that wants a look leads, since
  the cursor starts there. The glyph says the kind of row: the dollar for
  how things stand, a trend line for what was found, a tick for what's
  expected, the chart for a report and the door for a sign-in. A source
  down a moment is nothing, since boots and wakes outpace the network.

- **What AI use costs** (2026-10-14, chosen without a round, to be seen).
  The question is asked at a glance and answered when something's wrong,
  so today's spend is in the launcher's foot, against what a usual day
  had spent by this hour, and reaches the margin only when a finding wants
  a look: a cap close or reached, a raise about to end, a burn. The daemon
  works out every string, the desktop draws them, and while the screen is
  shared it draws a second set with no amounts. The glyph is a dollar sign
  (md-currency-usd), plain where a console draws it. The chord is Super+U,
  for usage, since $ is a layer away on the ErgoDox. No sparkline:
  a row's words say whether today is usual, and the collection holds the
  days for whoever wants them.

- **A quick look at a picture or a document** (2026-10-12). Opening a
  picture or a document is most often a glance, so each opens in a viewer
  that's quick and shows nothing but it: swayimg for pictures and drawings,
  zathura for documents, since no one program does both well. A picture
  floats in the middle of the screen at four fifths of its size, a look
  rather than a place to work, with its name and place in the folder for
  two seconds and the accent only on the gallery's selected picture, and
  e takes it to GIMP, or a drawing to Inkscape, and closes the look. A
  document tiles, since reading sits beside work, with its pages recoloured
  into the theme, hue kept, and Ctrl+R to see them as printed. SVG opens
  in swayimg rather than Inkscape for the same reason a PNG doesn't open in
  GIMP. Both programs go on past a setting they can't take, saying so only
  as they start, so each has a check that runs it.

- **A screenshot to mark up** (2026-10-12). A screenshot is most often
  taken to point at something in it, so the launcher offers the newest to
  edit in GIMP, at home just below the window in front for two minutes
  after it's taken, and found by typing after that. Every screenshot lands
  in one folder, niri's own and the region ones the desktop takes with grim,
  and niri tells the desktop only of its own, so the folder is what's
  watched; it's made at login, since niri makes it only with its first
  screenshot. niri and the desktop already say a screenshot was taken, so
  GIMP's row adds no notice of its own. GIMP's and Inkscape's dialogs float
  by niri's own rule for windows with a parent.

- **An editor gets the screen's width** (2026-10-12). At half a column,
  GIMP's docks cover the picture and Inkscape's leave a sliver of the
  drawing, so their main windows open as a full-width column, gaps kept,
  and Inkscape no longer covers the screen edge to edge as it asks to.
  Super+R still narrows them. Each main window is matched by the title it
  opens with, so its splash and dialogs keep floating.

- **Who's in a call, and a call that rings** (2026-10-12). Tuple on Linux
  leaves unfinished the notices its Mac app gives: someone joining,
  everyone else gone, a ring. Its daemon still writes each to its log, so
  the desktop says them. Who joined and who left are notices that matter only
  as they happen, since the panel names who's there; the last one leaving
  offers to leave as well; a ring wants Joel in the margin and offers to
  bring forward Tuple's window, where it's answered, since the command
  can't; and a ring nobody answered is kept, as a missed call. Names come
  from Tuple's contacts, asked once for anyone not yet known. Muting is now
  Tuple's own, which the others see, rather than the microphone's, which
  has its own mark, and Tuple's answer says how Joel stands even when the
  desktop had it wrong. Tuple's windows reach niri under one name with no
  title, and most are small, so all of them float at the size Tuple asks
  for. Its cursor over a share is a layer no rule reaches, and nothing of
  Tuple's is kept out of a screencast, which would draw the share black. A
  call link opened in the browser comes back as tuple://join, which
  Tuple's Linux command doesn't take, so its desktop entry does. What
  fills each message's blank was read from the program, not yet seen with
  a second person in the call, so a blank that isn't a number shows as
  "Someone".

- **A call, while it lasts** (2026-10-11). Tuple on Linux is a command and
  a daemon with no window to look at, so the desktop is where a call shows:
  a phone in the margin while Joel's in one, as recording and sharing are
  marked, and a panel that says how long and offers what a call needs. A
  call's link arrives pasted in a chat, so a copied one puts a row to join
  it first on the launcher's home, and Joel's own room is opened from there
  with its link copied to hand on. Sharing starts from the panel only while
  nothing's shared, since the sharing mark is where a share stops; muting
  is the microphone's, since Tuple says nothing of its own mute. Who's in
  the call, a ring, and Tuple's transcripts wait for Tuple to say them on
  Linux.

- **What's playing, for a moment and while it plays** (2026-10-11). Joel
  plays music from Spotify, spotify-player and his phone, and wanted the
  track only when it changes or a media key is pressed, then gone. Of three
  directions on the panel, a meter at the bottom, a notice in the corner
  with the cover, and words lifted by the halo with no box, he chose the
  notice: it's where he looks for what just happened, and the cover is
  recognised before the words are read. Then he asked for something to say
  music is playing at all, from any player, so the margin carries a note
  while one plays and none while paused. Both come from MPRIS, so a
  browser's tab or mpv counts as Spotify does, and the media keys go
  through the desktop, so they act on the player it shows. Seen in every
  state with two players, the card had to follow the player a key acted
  on, since pausing one hands the music to the other; play resumes the
  last player paused, not the last started; two of one name are told apart
  by what they play; and a player is offered to be shown only when it has
  a window, since opening its program would start a second one.

- **A launcher that finds what's meant** (2026-10-11). Research into
  Raycast, the launchers before it and the work on learning keys found two
  complaints above the rest: modes entered before typing, and lists too long
  to read. The launcher matched as fzf does, a word's first letter starting
  a word and a match more scattered than the best dropped, which is what
  Alfred and LaunchBar knew. On 2026-10-23 Joel asked to turn even that off:
  scattered matches, and the same thing listed twice, left him reading rows
  to rule them out. Each word typed now starts a word of what it finds,
  unbroken, ranked by fzf's bonuses, and a collection found whole stands
  alone for what it holds. It learns from picks as LaunchBar and
  Firefox's address bar do, keyed by what was typed. A check holds both its
  rankings and its speed, run in Qt's own engine.

- **A wallpaper that stands still** (2026-10-10). Of eight wallpapers shown
  on the panel, from none to tiled damask, Joel kept Omarchy's two Osaka
  Jade landscapes: the glowing city behind the desktop, being darker and
  calm behind windows, and the green woods and mountains on the lock
  screen, a threshold where a brighter picture belongs. The picture stands
  still while the workspaces move over it, as niri's guide describes, since
  a picture sliding with each workspace is motion that means nothing.
  Dimming the picture, or fading it into the night behind text, hid the
  wallpaper Joel picked it for; text set on it is lifted by offset shadows
  instead, and the picture shows whole. The overview's status moved above
  the overview so the workspaces no longer cover it, and the lock screen's
  time and field sit low, under its sky.

- **The overview's status wears a halo** (2026-10-11). A card behind the
  status, mostly opaque, read well over the workspaces but was "the large
  block overlapping", and Joel asked for shadowing instead, letting text
  and glyphs overlap with no opaque box. Of a card, the plain wallpaper
  shadows and a halo, he chose the halo: twelve shadows centred on the
  letters, from 1 px to 40 px, each the deepest colour at 0.95.

- **One language for every surface** (2026-10-10). The launcher took a
  dozen rounds. Boxes inside boxes, mixed sizes and spacing, and Mona Sans
  names beside a monospaced command line all read as clutter. Joel chose
  the launcher set wholly in Monaspace at one size, a command line that
  completes in place over a list, a hairline between them, the accent frame,
  and a cloudy surface that only hints at what's beneath. From it came the
  language above: one meaning per property, a kind for each sort of
  surface, colour on glyphs only, chords drawn as glyphs, and glyphs leading
  only the lines to act on. A glyph hanging over indented text, a badge
  beside the block, a glyph at the end of its first line, a drop cap and a
  sender written into the border were all tried for notices and prompts,
  and each read as slightly off. What stayed: the padlock beside the dots,
  and a notice signed in its footer.

- **Windows are framed by a hairline, rounded a little** (2026-10-10). Of
  seven directions shown on the panel, from square 2 px borders to a ring
  outside the window, Joel chose the one between: a 1 px border in the
  accent on the focused window and in the edge colour on the rest, corners
  rounded to `space.s`, gaps of `space.m`, on the deepest background. Every
  window keeps an outline, so the grid reads, and only the focused one
  carries the accent.

- **The monospace face stays Nerd Fonts' MonaspiceXe** (2026-10-10). Nerd
  Fonts' patched Monaspace and Monaspace's own NF builds keep exactly the same
  OpenType features: texture healing's eleven contextual lookups, the ten
  ligature sets and every character variant. Monaspace's own build adds the
  wide and semi-wide widths, but lacks 1,579 characters the patched one has,
  among them every Braille pattern, which btop draws its graphs with, and
  those would fall back to another face. The variable font has neither the
  icons nor anything the static faces lack but its width axis.
- **Muted text is sage, not Osaka Jade's own muted** (2026-10-10). Omarchy's
  muted measures 2.90:1 on the background, too faint to read, so muted text
  is its hue made just light enough to pass (4.52:1), and the original
  stays for rules and marks that aren't read. Still to be judged by eye.

[surfaces]: #surfaces
[decisions]: #decisions
[taste]: ../modules/theme/SKILL.md
