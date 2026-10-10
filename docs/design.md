# Design

How everything Joel sees and touches is meant to look, read and behave, from
the boot menu to a shell session, Neovim, pi, a browser or a chat app. Every
module that puts something on a screen follows this document, and every
module's `AGENTS.md` points at it, so an agent builds to it rather than to its
own taste.

It's a living document. No existing example is purely what Joel likes yet, so
the aesthetic is found surface by surface in studio rounds (below), and what
each round teaches is written here as it's learned. The document records the
aesthetic that was found, not the one that was assumed.

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

## The Principles

Every surface is held to the same eleven, and a surface isn't done until each
one is answered for it in [the surfaces table][surfaces].

1. **Act or wonder.** Something is shown only if Joel would act on it, or
   wonder about it if it were gone. What passes is precise and quiet; what
   doesn't is one step away, behind the thing it belongs to.
2. **One home.** A fact is drawn once on what can be on screen at the same
   time.
3. **The door.** Whatever is shown opens its own detail and controls, by Enter
   or a click.
4. **Found by typing.** Every action can be found by typing what it does, in
   its layer's palette, with its key shown beside it.
5. **Hints in place.** Keys sit beside the actions they trigger, in muted
   type, and each surface can turn them off once they're learned.
6. **No hidden modes.** A mode always shows that it's on, and Escape always
   leaves it.
7. **One gesture per meaning.** Arrows move, Enter opens or confirms, Escape
   backs out one level, typing filters, and each layer has one "what's here"
   key.
8. **Collapses on purpose.** As space shrinks, a surface gives way in a
   written order, never by truncation.
9. **The theme, whole.** Type, colour, glyphs, space and motion come from the
   theme and change with it.
10. **Customisable at no cost.** Every choice is an option, rendered into the
    program's own files when the machine is built.
11. **Fast.** Inside the latency budgets, at the panel's full refresh rate,
    with nothing spawned or polled that needn't be.

## The Quality Bar

A surface is done only when all of this is true, and nothing is shown to Joel
for judging before it's been judged here first.

- **Designed first.** Its three tiers, the facts it owns, its collapse order,
  its palette entries and its keys are written in the surfaces table, and Joel
  has read them.
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
  | body    | 0     | 10.5      | almost everything                         |
  | large   | 1     | 13.1      | a prompt's question, a notification title |
  | title   | 2     | 16.4      | a heading in a menu or dialog             |
  | display | 6     | 40.1      | the clock in the overview                 |
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
| xs   | 1     | 4      | between lines that belong together                 |
| s    | 2     | 8      | between a mark and its text                        |
| m    | 3     | 12     | inside a row, between windows, between cards       |
| l    | 4     | 16     | a terminal's padding above and below               |
| xl   | 6     | 24     | a terminal's padding at the sides, between columns |
| xxl  | 8     | 32     | a meter's height, a row of a prompt list           |
| xxxl | 12    | 48     | the overview's margin                              |
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
| accent     | where Joel's attention is: one focal point per screen state  |
| on_accent  | text drawn on the accent or on a state colour                |
| alert      | something that needs him: a failure, an urgent window        |
| warning    | something that will need him soon                            |
| success    | something that went well                                     |

Nothing else gets the accent, so it always means "here". Text is never drawn
on the wallpaper without a surface behind it. Muted text sits only on the
background, sunken and deepest; on the raised and selection surfaces hints
switch to text, since a muted that read there would be too bright to be
muted.

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

A console font holds at most 512 glyphs and none of Nerd Fonts' marks, so each
glyph also has a stand-in the console can draw, and a surface running on a
console (the prompt in an emergency shell) uses those.

The table is `jitsusama.theme.glyphs`, each mark with what it means and its
stand-in. Every program that can read it is given it: Quickshell as
`Theme.glyph`, Neovim as `require('theme').glyphs`, and anything else through
`~/.config/jitsusama/theme.json`, which holds the whole theme.

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
  waiting) belong to the shell alone. No terminal program, editor, prompt or
  app repeats them. The lock screen and the greeter show the time because
  they're never on screen with the rest.
- **A window's facts** (where it is, the repository and branch, the file, what
  failed) belong to the program in charge of that window. A program nested in
  another knows it (Neovim sets `$NVIM`) and leaves out what its host shows.
- **Frames repeat nothing their content says.** The compositor draws no title
  bars. A terminal's tab bar appears only with two tabs or more and names the
  tabs that aren't in view. An editor names a file once. Old prompts in
  scrollback collapse to their glyph.

| Fact                                                                   | Outside the overview                               | In the overview              |
| ---------------------------------------------------------------------- | -------------------------------------------------- | ---------------------------- |
| Time, date                                                             | the lock screen and greeter, when they're up       | the overview's status        |
| Battery, power                                                         | the margin mark, only when low or charging matters | the overview's status        |
| Network, Bluetooth, audio devices                                      | the margin mark, only when something's wrong       | the overview's status        |
| Volume, brightness                                                     | an overlay while it changes                        | the overview's status        |
| Do not disturb, caffeinate, recording, screen shared, microphone muted | the margin mark, while true                        | the same mark, in the status |
| Notifications waiting                                                  | the margin mark, as a count                        | the overview's status        |
| Directory, repository, branch, git state                               | the program running the window                     | nobody                       |
| Window titles                                                          | a terminal's unseen tabs; the window switcher      | the window switcher          |

The margin mark is the one always-visible desktop element: a small group of
glyphs in one corner, empty when nothing is true, so the screen is calm by
default. When the overview opens, it becomes part of the overview's status:
one element in two states, not two copies. It's the first thing the shell's
studio round tests, beside a version without it.

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
  palette: every action, mode, menu and setting the shell and the compositor
  offer is a result, found by what it does in plain words ("do not disturb",
  "rotate screen", "Bluetooth headphones"). Each result shows its key, so keys
  are learned by use rather than by study. Each program does the same inside
  its window: Neovim through which-key and its picker, pi through its command
  list, a browser through its own palettes, a chat app through its quick
  switcher. A key bound anywhere on the desktop that the palette can't find is
  a failing check.
- **What's shown is the door to its detail.** The battery opens the power
  menu, the network opens its list, a notification count opens the history.
  Nothing keeps its detail somewhere else.
- **Hints in place.** Wherever there are actions, their keys sit beside them in
  muted type, part of the design rather than a cheat sheet bolted on.
- **No hidden modes.** Do not disturb, caffeinate, recording, a resize, an
  editor's modes, a waiting gate: each shows that it's on, and Escape leaves.
- **One gesture per meaning.** A surface that needs a gesture of its own is a
  design problem to solve, not a feature.

## The Keyboard

Every layer's keys are designed together, so a chord means one thing and the
same idea gets the same key wherever it can.

- **Each layer owns its modifier.** Super is the desktop (the compositor and
  the shell). Ctrl+Shift is the terminal. Inside the terminal each program has
  its own (Neovim's Space leader, pi's chords). A chord a lower layer needs is
  never taken by a higher one.
- **The same gestures everywhere:** arrows for direction, Escape to back out,
  Enter to confirm, typing to filter.
- **The same verb on the same letter** across layers where it can be: new,
  close, find, the full view of something.
- **One "what's here" key per layer,** the same shape on each, showing that
  layer's actions and keys drawn from the real configuration, never a
  hand-kept copy.
- **The map is generated.** The whole map is built from the compositor's
  binds, the terminal's maps, Neovim's keymaps and pi's keybindings, and a
  check reports any chord two layers both claim.

## Customising

Every choice comes with deep room to change it, and changing it never slows
anything.

- **Options, not edits.** What a surface shows, the order, each piece's tier,
  its collapse rank, its threshold, its glyph and its colour role are
  `jitsusama.*` options with documented defaults. The prompt's pieces, the
  overview's status, the margin mark, the editors' status lines, the boot
  splash's lines and every chord are lists that can be reordered, extended or
  cut. A new piece is a small function in the same shape as the built-in ones.
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
anywhere goes), and its console font is rendered from its monospace face at
build, or named as a package.

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

# What each colour means, by role (see Colour). Each names a pigment.
[roles]
background = "forest"
accent = "jade"

# The terminal's sixteen colours and its own, by pigment.
[terminal]
background = "forest"
black = "forest"
bright_black = "sage"

# Code's roles (keyword, string, comment and the rest), by pigment and style.
[syntax]

# The type scale's ratio and steps, the space scale's unit and steps, motion's
# spring, curve and durations, and any glyph to change, as the theme module's
# options of the same names spell them.
[type]
ratio = "1.25"
[space]
unit = 4
[motion]
stiffness = 1600
[glyphs]
cursor = { glyph = "\u25b8", console = ">" }

# Still to come: the wallpaper and the console's font.
[wallpaper]
[console]
```

The shades, `palette`, `roles` and `terminal` are required. Every other table
falls back to the defaults the theme module carries, so a theme changes only
what it means to. Colours may carry alpha (`#rrggbbaa`). Roles always name a
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
  colours and font, the greeter, the compositor, the shell, the apps, the
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
shown side by side at full size on the panel they're for. Joel picks, the pick
is refined in rounds, and the findings go under [Decisions][decisions].

Each round compares a dense version beside a sparse one, the defaults beside a
customised configuration, and the panel beside a small monitor.

## Surfaces

Every interactive surface, from power on to the last app. Each row is filled
as its layer is built: its tiers, the facts it owns, how it collapses, its
palette and its keys.

| Surface                                   | Module                   | Faces          | Tier 1 | Tier 2 | Behind it | Palette | Collapse |
| ----------------------------------------- | ------------------------ | -------------- | ------ | ------ | --------- | ------- | -------- |
| The boot menu                             | lanzaboote, systemd-boot | the firmware's |        |        |           |         |          |
| The boot splash and unlock prompts        | Plymouth                 | both           |        |        |           |         |          |
| The console and emergency shell           | the console font         | console        |        |        |           |         |          |
| The greeter                               | Quickshell, greetd       | both           |        |        |           |         |          |
| The compositor                            | niri                     | none           |        |        |           |         |          |
| The overview's status and the margin mark | Quickshell               | both           |        |        |           |         |          |
| The launcher and palette                  | Quickshell               | both           |        |        |           |         |          |
| Notifications                             | Quickshell               | proportional   |        |        |           |         |          |
| Menus and overlays                        | Quickshell               | both           |        |        |           |         |          |
| The lock screen, polkit and PIN prompts   | Quickshell               | both           |        |        |           |         |          |
| GTK and Qt apps, file and print dialogs   | gtk, qt                  | proportional   |        |        |           |         |          |
| The browser                               | chrome                   | proportional   |        |        |           |         |          |
| Chat, passwords, calls                    | their own settings       | proportional   |        |        |           |         |          |
| The terminal                              | kitty                    | monospace      |        |        |           |         |          |
| The prompt                                | prompt                   | monospace      |        |        |           |         |          |
| Terminal programs                         | each tool's module       | monospace      |        |        |           |         |          |
| Neovim                                    | neovim                   | monospace      |        |        |           |         |          |
| pi                                        | its theme, its harness   | monospace      |        |        |           |         |          |

Where a surface can't take the design (web pages in a browser, a chat app's
messages), the row says so, and what's themed is what can be reached. The
firmware's setup screens and its own boot and update menus are the firmware's,
and stay outside.

## Adding a Program

Whatever is added later goes through the same questions before it's
considered done:

- Does it take the theme, or at least follow the dark preference?
- Do its notifications go through the shell, so do not disturb covers them?
- Are its keys in the map without a clash, and is its palette in the map too?
- Does it repeat a fact another surface owns, and can that be turned off?
- Can it show a secret, and if so, is it blocked out of screen shares?
- Does it start, draw and idle within the budgets?

A program that fails and can't be fixed is replaced, or written into its row
as an exception with the reason.

## Decisions

What each studio round settled, newest first, with the reason.

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
