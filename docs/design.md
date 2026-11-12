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
   In the launcher the pointer never moves the selection, so there's no
   hover selection (Joel, 2026-10-29: "the launcher window is not meant to
   be mouse navigated").
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
7. **Modal, as vi is, and never hidden.** Every list and text box has vi's
   modes, each with one purpose ([Vi's Keys][vi]). A mode other than normal
   always shows that it's on, and Escape always leaves it.
8. **One gesture per meaning.** j and k move, Return goes in or confirms
   and - goes up, the only keys that step a level, Ctrl+O goes back, /
   finds, : names a command, and each layer has one "what's here" key.
   Where vi has a key for it, the key is vi's.
9. **Collapses on purpose.** As space shrinks, a surface gives way in a
   written order, never by truncation.
10. **The theme, whole.** Type, colour, glyphs, space and motion come from
    the theme and change with it.
11. **Customisable at no cost.** Every choice is an option, rendered into the
    program's own files when the machine is built.
12. **Fast.** Inside the latency budgets, at the panel's full refresh rate,
    with nothing spawned or polled that needn't be.

## Written for Joel

Anything put in front of Joel to read is written for him, not for the
agents that made it: a look on his look list, a page, a brief, a landing's
line, a notice. Joel, 2026-11-11, of a research synthesis sent him: "anything
that is sent for me to look at needs to be written for me, like what the UX
canon says is not written for humans to read. i tried, it is too hard for
me to parse."

- **What it means for him comes first.** It opens with what changes for him
  or what he's asked to do, then why, in short sentences of plain words.
- **An example of his own.** Each point carries one thing from his own
  machine (a row, a count, a window), never an abstract case.
- **The working stays one step away.** Citations, identifiers, codes,
  numbered rules and field names stay out of what he reads, kept in a
  record for agents beside it: a research document, a drawer's facts, a
  page's text twin.
- **Why it's there comes with it.** A look says what it is, why it matters
  to him now, what he's asked to do, or that nothing is and it's just to
  see, and which quest it belongs to, so he can place it days later.

## The Quality Bar

A surface is done only when all of this is true, and nothing is shown to Joel
for judging before it's been judged here first.

- **Driven by keys.** Everything it does has been done with the keyboard
  alone, from the chord that reaches it to :q, and its check does it
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
- **As deep as the deepest.** Every program, device and service the desktop
  touches is integrated with the same care as the best of them (Joel,
  2026-11-07: "every integration has an insanely high bar of nerdiness and
  integration and each should be given the same care"). It shows every true
  figure its source has, live and in its own units, in its place, its
  drawer, its : commands and its command line; it's found by typing as the
  rest are and laid out to this document, none a thinner cousin.
  **Check:** every figure the source gives is shown or named as
  left out with a reason, and it holds up against the deepest integration
  of its kind, row for row. Printing is left out whole, with no printer
  set up (Joel, 2026-11-07: "i don't really care about printing at this
  point"), so a print dialog shows only what GTK and Qt give it.
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
- **Accounted for.** Every check in [Every Pixel Accounted For][pixel] has
  its answer, and the answers are shown with it.
- **Judged whole, before it merges.** A change that moves anything on a
  surface carries its shots at 1.5 and 2, cropped where it changed, and a
  table of what it changed measured to the ink: each gap by its rung, each
  edge by its column, each baseline by its pitch, each run by its level, each
  ink by its colour role. A design review judges type, space, alignment and
  colour together on those shots and answers within half an hour, and the
  change merges on its pass (Joel, 2026-11-05: "you are not considering
  enough design elements when changing things"). If no answer comes within
  half an hour of the shots, it merges on its owner's checks and is judged
  after, a miss fixed forward, so nothing waits on the review. A change that
  draws nothing differently says so and needs none.
- **Said, where it's unfinished.** Whatever hasn't met all of this is called
  unfinished when it's shown, never done.

## Every Pixel Accounted For

Joel's bar for everything he sees and touches, set on 2 November 2026,
after the second look at the desktop's keys and type failed:

> i want greater emphasis on typography as well as excellent use of colour,
> insanely tasteful UI design and UX. I want something you could put up in
> an art museum that is also insanely functional, that fits my aesthetics
> for form and function. no detail can be ignored. every pixel of
> whitespace must be accounted for. alignments, available space, all
> interactions, each must be amazing. the same story with how i interface
> with a keyboard.

The quality bar says when a surface is done. This section says how each part
of it is shown, so a design arrives with its proof rather than a claim. Each
check is answered with a measurement, a file and line, or Joel's own words.
A check left unanswered is a fault, not a gap to explain.

Measurements come from full-size screenshots at the scales Joel's screens
use, 1.5 and 2 today and any he adds, cropped to the part being measured
before they're read, never judged by eye. A grey is read as the mean of a
flat patch of it (`-colorspace gray -format '%[fx:mean]' info:`). Ink is
found by thresholding the crop midway between the grey of its ground and
the grey of the faintest ink in it, so muted text and hairlines count,
then trimming it and printing what's left, in device pixels, as width,
height and offset (here at 30%):

```sh
magick crop.png -colorspace gray -threshold 30% -trim -format '%wx%h%O\n' info:
```

On the wallpaper, where no ground is flat, ink is found by its colour
instead: `-fill white -fuzz 6% -opaque '<ink>' -fill black +opaque white`
before the trim. A baseline is the bottom of a crop holding only letters
that don't descend, such as capitals and figures. A line's pitch is the
distance from its baseline to the next line's: a surface names the pitch
of each kind of line it sets, and a row's pitch is its height.

### Typesetting

- **Type carries the design.** Hierarchy comes from the type levels (size,
  weight, face, case and colour role, each with its own job), set as a
  typesetter sets a page, never from a stock
  template's sizes and boxes (the first look, 2026-11-02: "the typography is
  middling ... not leveraging type and typesetting enough"). **Check:** a
  table of every run of text on the surface, giving its face, step, weight,
  colour role, OpenType features and line pitch, and why it differs from the
  run nearest it. Two runs that differ for no written reason are a fault.
- **Set, not styled.** Figures, case, ligatures, punctuation and breaks are
  chosen: tabular figures where a number changes in place, a face's case
  forms beside capitals, real punctuation, a name cut only at a whole word,
  no widow and no hyphen break. A figure keeps three significant digits,
  taking the next unit past 999 ("370 Mb/s", "1.20 Gb/s"), and a negative
  one the real minus ("−48 dBm"), never a hyphen; a figure that names
  rather than measures keeps every digit and gains no zeros (a mode's
  "120.001 Hz", a channel's "5,745 MHz", a version, a charger's offered
  "5 V · 20 V at 3 A", a learned set point's "2 klx"), while a measure keeps
  its zeros ("0.500 A", "96.0 ppi"), and a measure at zero reads 0
  ("0 lx"). Sizes are decimal, worked out
  from bytes ("1.52 GB of 4.08 GB", "915 MB"), never a GiB beside a GB;
  thousands take a comma ("4,000 K", "2,400 rpm"); a scale reads "2×",
  never "200%"; and a pair says which is which ("12 kB/s in · 0 B/s
  out"), in and out at this machine's own links, down and up only for
  the internet line's speed, as every speed test says it. A range reads
  "118 ms to 136 ms", and one that holds still "127 ms · steady". A
  moment reads as When writes it: today its bare time ("15:20"), then
  "yesterday 23:40", a weekday within six days, "28 Oct", and forward
  the same ("tomorrow 00:10"), with "at" only before a bare time. A
  count says what it counts, never its source's name ("24 known · 15 on
  Wi-Fi"), a part of it that's zero gives way, and one its row
  already says isn't said again ("all on Wi-Fi" under "9 devices"). A run
  of items joins with dots, never a comma beside a dot, and a qualifier
  follows its figure after a dot ("−60 dBm · strong", "Wed 2 Dec · a
  guess"). **Check:** each feature named in the table
  and seen in a crop at the device scale.
- **At least what it replaces.** A redesign keeps whatever today's surface
  does better (2026-11-02: "the existing launcher design has better
  typographic presence in some ways than what you have presented").
  **Check:** today's and the new, cropped side by side at one scale, with
  what each does better written beneath them.
- **What he likes stays.** A face, a size or a weight he hasn't turned down
  changes only for a reason he gave (2026-11-02: "the lock clock font face
  is nicer as is, why the font change?"). Where this and a change he'd feel
  pull apart, this wins: the change is made in what he hasn't liked.
  **Check:** every change to something already on his screen names its
  reason and whose it is.

### Colour's Use

- **Every pair measured.** **Check:** each pair of ink and ground on the
  surface, by role, with its contrast in every theme, measured on the
  colours the screenshot shows, since a floating ground is translucent.
  Each passes what the theme's build holds (`modules/theme/home.nix`, which
  [Themes][themes] describes): 4.5:1 by WCAG for text, and by APCA Lc 75
  for text and strong, Lc 60 for muted and Lc 45 for a mark's hue. On the
  raised and selection surfaces muted isn't drawn, as [Colour][colour]
  says, so what would be muted is measured as text there.
- **Colour says something.** The accent marks only where the keys go, once
  per screen state; a glyph's hue says its kind; text keeps to the text
  roles. **Check:** a list of every coloured thing on the surface and the
  kind or state it says. Anything coloured that says nothing is a fault, and
  so is a second accent.
- **Structure in type, warmth in colour.** Colour gives the focal point and
  the warmth, never the structure. **Check:** a greyscale render of the
  surface, in which the runs ranked by grey and weight fall in the type
  table's order of hierarchy.

### Whitespace

- **Every gap has a name.** Every space between two inks, inside a frame
  and outside it, is a rung of [the ladder][space], a level's pitch, a
  number of beats, or what a row leaves above and below its text once the
  text is centred in it, measured to the ink (a capital's top, a baseline, a
  mark's edge), never to a text box. **Check:** the screenshot with each gap
  measured in device pixels and named by its rung. A gap off its rung by more
  than a device pixel, or one nobody can name, is a fault.
- **Equal where it says it is.** The room round a thing is the same on every
  side, measured to the ink: a frame's padding, a notice's, a band's room
  over and under its text. **Check:** its four measurements, within a device
  pixel.
- **Space groups.** What belongs together sits closer than anything else
  does to it: each gap is its level's rung, and every rung is wider than the
  one inside it, so two levels never share a space and nothing inside a thing
  is wider than the space round it. **Check:** each group's inner and outer
  gaps, by rung. A pair equal or inverted is a fault, and the surface reads as
  cluttered.
- **Never cluttered.** A surface has only what Joel would act on or wonder
  about (2026-11-02, of the notices: "whitespace issues ... feeling quite
  cluttered"). **Check:** every element listed with what he'd do with it or
  wonder about it. An element that answers neither goes.

### Alignment

- **Few edges, every one named.** Every left and right edge lies on a named
  column: the frame's inset, the mark column, the text edge, the flush-right
  edge. **Check:** the columns listed, and every ink edge on one within a
  device pixel. Where a shape's optical edge differs from its ink (a round
  letter's overshoot, a mark drawn small in its cell), the column takes the
  optical edge, and the check says so.
- **Baselines on the pitch, marks one height.** **Check:** every baseline
  measured against the surface's line pitch, and every mark's ink one optical
  size, centred on the capitals' middle, as the skill's "Marks one height"
  asks. One size is equal area, not equal height, since a wide mark made as
  tall as a narrow one turns heavy: each is scaled so the square root of its
  ink's width by height is 15, no wider than its cell and no taller than 15,
  measured at load from the font, with no table.
- **Shared across neighbours.** A surface beside another (the launcher and
  its drawer, a notice and the margin) lines up with it. **Check:** the
  shared edges named and measured.

### Space Available

- **Room used on purpose.** Every region is content or a named space. An
  empty region taller or wider than huge needs its reason written beside
  it, and text cut short while room is free beside it is a fault.
  **Check:** the screenshot with its empty regions marked and their reasons.
- **At every size.** **Check:** every state at every scale his screens use,
  the longest name, the empty list and the full one among them.

### Interactions

- **Every interaction written before it's built.** **Check:** a table of
  every state and every key in it: what it does, what the screen does in
  answer, how long it takes against [Latency Budgets][budgets], and which of
  the theme's motions carries it. A mode Escape can't leave, a key with no
  visible answer, or an answer that waits is a fault. So is a surface with
  no way out, which :q, ZZ and Space w don't close, and so is one Escape
  closes.
- **Felt, not just correct.** **Check:** each transition timed frame by
  frame at the panel's full refresh rate, from the program's own frame
  times or a screen recording stepped through a frame at a time, with no
  dropped frame, no jump and no jitter.

### The Keyboard, on Both Keyboards

- **Every key named where it sits.** **Check:** each key a design uses,
  with its place on the laptop's keyboard (a letter from
  `modules/keyd/colemak-dh.conf`, any other key where the laptop's keyboard
  has it) and on the ErgoDox (layer, hand and finger, from
  `modules/ergodox/keymap/keymap.c`).
- **The ErgoDox's facts.** On its base layer each hand's home row is
  tap-hold: held, the little finger is Ctrl (A and O), the ring finger Alt
  (R and I), the middle finger the symbols (S and E), the index finger Super
  (T and N) and the key inside it the number pad (G and M). Super is bare on
  both thumbs as well, beside Space and Backspace, and Escape is bare in
  both top corners. Shift is bare on each outer column, under the little
  finger, so Ctrl and Shift held on one hand ask one little finger for two
  keys. F1 to F10 are bare across its top row; the laptop reaches them as
  its own keyboard does.
- **Neovim's way: modes and a leader, not new chords.** What's done often
  is a single key in a mode, or Space and then a letter, as in his Neovim
  (2026-11-02: "all of those ctrl keys do not feel very vim to me"). On the
  ErgoDox a Ctrl chord pins the weakest finger on a letter, where Super's
  index finger is the strongest, which is why the desktop's Super chords are
  comfortable. A key held to repeat, such as a motion, is never a home-row
  letter there, since those are modifiers when held. **Check:** every chord
  led by Ctrl or Alt in the design, each with its reason. Neovim's own (the
  Ctrl+D, Ctrl+U, Ctrl+F and Ctrl+B that [Vi's Keys][vi] keeps), the
  desktop's copy and paste, Super's second modifiers ([The
  Keyboard][keyboard]) and the terminal's Ctrl+Shift layer, which he uses
  and which waits for its own review against the little-finger rule, stand;
  a new one for something done often is a fault.
- **Bare keys first.** What's done often ends on a key both keyboards have
  bare, as [The Keyboard][keyboard] lists them. **Check:** each key for
  something done often, and that both keyboards have it bare.
- **Only the keyboards plugged in show** (Joel, 2026-11-07: "seeing ergodox
  layouts and shit when it isn't plugged in is weird as fuck"). Every
  surface that names or draws one keyboard's keys (the layout card, the
  glance's layer mark, the Keyboards place, hints, key lists, the guide,
  the greeter and the lock) reads one list of the keyboards connected now,
  asked of the input stack rather than guessed from config, and the
  laptop's own is always on it. A keyboard not plugged in is absent, never
  listed as not connected; a key only it has is named only while it's
  there, and keys are otherwise named as the laptop has them. Plugging one
  in or out changes an open surface within a second, with no notice, so
  Super+K shows the laptop's Colemak keys while the ErgoDox is away and the
  ErgoDox's layers while it's in.

### Where Each Choice Came From

- **Every choice has a source.** **Check:** each choice names where it came
  from: Joel's words with their date, his configuration by file and line,
  his keyboards, this document or [the aesthetic skill][taste], or research,
  cited. A choice with none is marked as the designer's own, so he sees it
  (2026-11-02: "what actually has impacted your design choices? you don't
  seem to have dug very deep").
- **Changed enough to feel.** A redesign is shown beside what it replaces,
  with the differences he'd notice at a glance listed first (2026-11-02:
  "the UI still doesn't feel materially changed"). **Check:** that list. If
  it holds only measurements, it isn't a redesign yet.

### Who Judges

A design is still judged without him, by this document and [the aesthetic
skill][taste], and never handed to him to find its faults. A reviewer checks
facts, measurements and these rules, and nobody stands in for his eye: a
panel judging in his voice passed the second look, and he failed it. Once
every check here is answered and its designer would hang it in a museum, a
design that changes what he sees is shown to him, and his verdict stands
over any reviewer's.

## Type

Two faces do the work: Monaspace, a monospace, and Fraunces, a serif. Beside
them, a script face draws the letters neither has, and Mona Sans is kept only
for other people's programs.

- **No sans-serif unless it earns it** (Joel, 4 November: "I don't like
  seeing sans-serif fonts in the launcher, the current one feels so out of
  place. like if the sans serif has some more class to it maybe, but
  generally, not a fan of sans-serif"). Nothing of ours sets a sans: what
  the mono doesn't set, the serif does. A sans comes back only if it has real
  class and beats both, side by side in shots at 1.5 and 2. The shell's
  theme gives it no sans to name, so one can't come back by accident.
- **Monospace** lives where characters line up or the content is code: the
  terminal, Neovim, a launcher's input and a row's name, numbers that change
  in place (a clock, a volume, a battery), keyboard chords, paths and commit
  hashes.
- **Serif upright, for words to read** (Fraunces roman, weight 380, opsz 14,
  SOFT 50, WONK 0, as the reader's page sets its text): a launcher row's
  words and notes, a notice's words, the lock's day and what it says, the
  greeter's words and the words of a password prompt. It's at the name's
  size, on the mono's baseline, so a row stays one line: the name leads by
  its weight, and the words follow by face and colour. A line holding a
  strong span (a prompt's owner, the greeter's
  name) sits at Regular 400 with the span at SemiBold 600, since Qt's span
  styles can't move the weight axis and Fraunces names no 380 or 500.
- **Sans** (Mona Sans) stays only where another program draws its
  own interface: GTK and Qt programs, Spotify, GIMP and web pages that ask
  for a sans, whose layouts are built for one. Programs whose words we set
  (mpv, zathura, swayimg, the boot screen, the camera, inlyne) move to the
  mono and the serif too.
- **Monaspace's own art, on purpose.** Texture healing and the coding
  ligatures wherever monospace is drawn, but for texture healing in kitty,
  which shrinks a glyph wider than its cell to fit it. The other Monaspace
  voices only where they mean something, such as a second voice for italics
  so comments read as handwriting, chosen by looking.
- **Serif italic, the voice** (Fraunces, italic, weight 380, soft and wonky, drawn for 24
  points) is the desktop's voice: what the launcher says to Joel rather than
  what he typed, such as its hint while nothing's typed and its words when
  nothing is shown. Upright, at the Title level below, it names a place
  and the thing a launcher's drawer is about. Joel, 2 November: "i love
  serif, i'm willing to give fraunces a try".
  It's sized by capitals, not by its em: Monaspace's stand 0.730 of an em
  and Fraunces' 0.700, so it's drawn 0.730 / 0.700 as large and sits on the
  line Monaspace would, aligned by baseline.
- **Script** (Noto Sans Hebrew, Noto Naskh Arabic, Noto Sans CJK) draws the
  letters the other faces lack, script by script, such as a calendar's date
  in its own script. A letter missing from the face a surface names comes
  from it, before any other font. Words a surface knows are in one script
  name that script's cut beside their face, `Theme.script.serif.hebrew` or
  `Theme.script.mono.chinese`: serif cuts beside Fraunces, sans beside
  Monaspace, and Chinese in its own cut, since a fallback would give its
  Han the Japanese forms.
- **Numbers are typography too.** Tabular figures wherever a number changes
  in place, so it never jiggles; units set smaller than the number; the same
  precision for the same quantity everywhere.
- **One scale.** Sizes come from one base and one ratio,
  `jitsusama.theme.type`, so every surface draws from the same few steps. The
  ratio is a major third, 1.25, from a 10.5 point body. Only some powers are
  used, so every step taken reads: a surface that seems to need a size
  between two of them needs a weight or a colour instead. No level of the
  shell's takes section; it's kept for the reader's page.

  | Step    | Power | Size (pt) |
  | ------- | ----- | --------- |
  | small   | -1    | 8.4       |
  | body    | 0     | 10.5      |
  | large   | 1     | 13.1      |
  | section | 2     | 16.4      |
  | title   | 3     | 20.5      |
  | display | 6     | 40.1      |
  | hero    | 8     | 62.6      |

### The Levels

Joel, 4 November: "our typography game is still weak. we are not levraging
different weights and sizes enough to build a good hierarchy of information."
The scale had been used as a zoom, one size for a whole surface, so within a
surface everything was one size and one weight, and colour alone ranked it.
Now every run of text names one of eight levels, and the levels rank it.

- **Each dimension has one job.** Size says the tier: a title over content,
  content over a surface's furniture. Weight says what to read first in a
  line. Face says what kind of thing it is: the mono for a name or a figure,
  the serif upright for words to read, its italic for the voice. Capitals
  with tracking say "label". Colour says focus and state; it never ranks
  alone. Space groups.
- **Two cues a step.** Neighbouring levels differ in at least two of size,
  weight, face, case and colour. One 1.25 step of size alone, or 100 units of
  weight alone, reads as texture, not rank, and light ink on a dark ground
  hides it further. **Check:** the typesetting table names the two cues
  between each run and the one nearest it.
- **At most four levels on a surface** at once, and three in a row, since
  working memory holds about four things.
- **Selection never changes weight.** The fill and the strong colour show it,
  so weight means only a level.
- **One strong for each state.** Strong is the one focal thing: the selected
  row's name, a drawer's title while it's shown, or a notice's sender.
- **Figures are the mono's**, the lock's clock and a drawer's time among
  them; figures inside a sentence are the serif's.
- **Optical size follows the size drawn.** Never a poster cut (opsz 72 or
  144) at a title's size or below.
- **Italic means the voice**, nothing else, in the shell: not emphasis,
  not a clock's seconds. A document the reader shows keeps a document's
  italic, its emphasis and its third-level headings.
- **No raw size or weight.** A run of text names its level, never a step, a
  size or a weight; `type-is-set-by-level` fails on one that doesn't.

The serif's sizes are the mono's grown by 0.730 / 0.700, so both share a cap
height. Weights are Monaspace's (it has 400, 500 and 700) or Fraunces' wght
axis.

| Level   | Face and setting                                         | Step    | Case                | Colour                                                   | What goes at it                                                                                                               |
| ------- | -------------------------------------------------------- | ------- | ------------------- | -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Display | Monaspace 400, tabular figures                           | hero on the lock and greeter, display on a plate | as is, -0.01 em | text on a wallpaper, strong on a plate                 | a clock or a time shown large                                                                                                 |
| Title   | Fraunces upright, wght 640, opsz 14, SOFT 100, WONK 1    | large   | sentence            | strong                                                   | a drawer's title, a place's name in the launcher's head, a dialog's heading, a page's section                                 |
| Name    | Monaspace Medium 500                                     | large   | as written          | text; strong when selected or as a notice's sender       | a row's name, a notice's sender, a window's title                                                                             |
| Figure  | Monaspace 400, tabular figures                           | large   | as written          | text; strong while typed                                 | a fact's value, a figure in a row, the typed query                                                                            |
| Words   | Fraunces upright, wght 380, opsz 14, SOFT 50, WONK 0     | large   | sentence            | muted beside a name; text alone                          | a row's words, a notice's words, a drawer's prose, the lock's day, a prompt's ask                                             |
| Voice   | Fraunces italic, wght 380, opsz 24, SOFT 100, WONK 1     | large   | sentence            | muted as a hint; text when it speaks                     | a prompt's hint while nothing's typed, what's said when nothing is found, the drawer's line on what needs him               |
| Note    | Monaspace 400                                            | body    | as written          | muted; text on the selection and for a chord             | a row's side note, the count, chords and their hints, the foot, a fact's label, a notice's program and time; units at small |
| Label   | Monaspace Medium 500, the `case` feature                 | body    | capitals, +0.16 em  | muted                                                    | a drawer's eyebrow, a group's heading in a list                                                                               |

Label is Note told apart by case, so a surface counting its levels counts
them as one. Label's capitals are for our own words: a name from the world,
an address, a repository, a folder or a file, is never re-cased, nor is a
figure's unit ("on 2.4 GHz · network"), so an eyebrow carrying one
is set in Note as written ("8IDU4E · core.nix · since 21:52"). The
eyebrows of one place share one shape, where the thing
is and then its kind ("via Example ISP", "eero Pro 6 · node", "on Study
· Wi-Fi"), and say no state, which its first fact says. Nothing the
launcher shows names a daemon: it says where a thing came from ("set by
hand · location", "via Wi-Fi · location", "via cell towers ·
location"), never what told it, such as GeoClue, and never a source it
can't tell ("found · location"). Short of room,
an eyebrow gives way by whole items from its end, as a note does. Each
surface says which levels it shows, in the surfaces table.

## Space

One unit and a scale built from it, `jitsusama.theme.space`, so the padding in
a menu, the gap between windows and the margin of a notification are the same
few values. At a display scale of 2, every edge lands on a whole device pixel.
The unit is 4 logical pixels, and the steps sit one unit apart where a pixel
shows and further apart as they grow:

| Step | Units | Pixels | Taken by                                           |
| ---- | ----- | ------ | -------------------------------------------------- |
| xs   | 1     | 4      | the tie rung                                       |
| s    | 2     | 8      | the mark rung                                      |
| m    | 3     | 12     | the apart rung, the beat, niri's gap between tiles |
| l    | 4     | 16     | the items rung                                     |
| xl   | 6     | 24     | the inset rung, the large pitch                    |
| xxl  | 8     | 32     | the frames rung                                    |
| xxxl | 12    | 48     | the screen rung, a row of the launcher or a prompt |
| huge | 16    | 64     | the bound past which empty room needs a reason     |

Whitespace groups things before borders or boxes do. Start with too much space
and take it away. Density follows the moment: dense and scannable where Joel is
mid-task (a launcher, a status line, a prompt), roomier where he reads (a
notification, a dialog, the greeter).

### The Ladder

A step says how big a space is; a rung says why it's there. Joel, 2026-11-05:
"do more research on design, particularly around not just how typography but
white space gives topology and hierarchy awareness." Space is how the eye
learns what belongs together and what outranks what, ahead of colour or a box,
so every gap belongs to one level of structure, and **each level's gap is wider
than the one inside it**: inner at one level is outer at the next, from a mark
to the screen's edge, the same on every surface (Bureau Gorbunov's inner and
outer; NN/g on proximity). A surface names the rung, `Theme.space.rung.items`,
never the step, and `space-is-set-by-rung` refuses a step where a rung belongs;
the theme refuses a ladder that doesn't widen outward.

| Rung | Pixels | Between |
| ---- | ------ | ------- |
| tie | 4 (xs) | the parts of one token: a chord's keys, "Space ?", the gutter's numbers and the ground |
| mark | 8 (s) | a mark and its text, a key and its word, "? keys", a dot and what it joins |
| apart | 12 (m) | two runs of one thing: a name and its words, typed text and its count, a title and its hint, chord and chord |
| items | 16 (l) | things on one line: hint and hint, fact and fact |
| groups | 20 | groups on one line (the facts, the state, the hints), and column and column |
| inset | 24 (xl) | a frame and its text, on every side, to the ink |
| frames | 32 (xxl) | floating frames that belong together: the launcher and its drawer, notices in a stack |
| screen | 48 (xxxl) | a floating frame and the screen's edge, every frame alike |

Groups take 20, the one rung off the scale, since the ladder needs one between
things on a line and a frame's own room: no gap inside a frame passes the room
round its text. A drawn boundary outranks space, so a region may take a hairline
rather than a wider gap, and tiled windows, each bounded by its border, keep
niri's gap between them with their own text at the inset.

Down a surface, space counts in lines, as a page's does (Bringhurst;
Müller-Brockmann). The **beat** is 12, half the large pitch, and every space between
blocks is a whole number of beats, so a row (48), a drawer's line (24) and
home's group gap (12) fall on one grid and lines at two pitches side by side
meet on it. Each level carries its **pitch**, baseline to baseline,
`Theme.level.note.pitch`: Note and Label 20; Title, Name, Figure, Words and
Voice 24; Display 56. A Title is its line's size, told apart by weight and
colour, never by size (Joel, 2026-11-06: "use weights and not size for
that"), so a head is a band like the rows.

- **Lines of one thing** sit at their level's pitch.
- **Things of a kind** sit a row or a pitch apart, and a beat further where any
  can take two lines, so a wrapped line is nearer its own thing than the next
  thing is.
- **Groups** take a beat more between groups of rows, an empty line between
  groups of lines. A group's heading sits a pitch above its first thing, its
  group's gap above it: a heading belongs to what follows.
- **Regions** of a frame part at a hairline: the head over the rows and the
  foot under them alike, the foot's baseline a row below the last row's. The
  hairline runs inset to inset, the width of the ink, and never meets the
  frame, since a rule that meets it cuts the card into boxes. A
  plate's head is a region of its own: its eyebrow a Title pitch above the
  title, the body two below it, back on the beat.
- **A frame's bottom is set**, never what whole lines happen to leave: where
  a status line closes it, the bottom falls where that band's text centres.

The rest follows from the ladder:

- **The frame's edges keep the inset; text in a band centres in it.** The
  inset is measured from the frame's inside edge, its border left out, to the
  ink: the side's first and last ink always, and the first cap's top and the
  last baseline where no band edge is drawn. A band, the room between two
  edges (the frame and a hairline, or two hairlines), sets its text optically
  centred between them: its baseline at the band's middle plus a quarter of
  its capitals and x-height together (Joel, 2026-11-05: "the launcher find
  box text is not vertically aligned within its box, it is more heavily top
  aligned"). A band's height comes from the rows' rhythm, the head's
  baseline a row above the first row's, so the head, a field and the status
  line read alike. A head with no rows under it, a prompt's or the picker's,
  has no rhythm to size it, so its capitals sit the inset under the frame
  and its hairline falls where the text centres, the same room under it.
- **A glyph's column is its cell and a mark.** A mark sits in a square cell
  one line pitch wide, its text the mark rung after it, so a list's names
  start at `Theme.level.name.pitch + Theme.space.rung.mark` from its glyphs,
  an indent is the same again, and the column follows the type.
- **A stroke isn't a space.** A meter's bar, a wave's bars and the room in a
  texture are counted in hairlines, `4 * Theme.border`, never named by a rung,
  and each says in a comment that it's a stroke or a texture.

- **One baseline a line.** A line's pieces share one baseline, found by
  centring its largest level's capitals in the line; a smaller level takes that
  baseline, never the centre of its own capitals. A note beside a head sits on
  the head's baseline (Joel, 2026-11-05: "which? &etc. need to be baselined
  with the text to their left"). In a band, a line centres the largest level
  it holds now, so the head's line settles a few pixels as a place's Title
  arrives with the rest of the change, and every state stays centred; the
  band and the rows never move.
- **Held room at the far end.** A place that keeps the widest room it's needed
  keeps it after its cluster's last item, never between items, so every gap is
  its rung. Return's slack sits inside its own place, between the dot that
  joins it and its key, so the dots run evenly a rung from each hint, and as
  the selection moves and Return's words change, nothing before the dot moves
  (VKCF3Z, 2026-11-08).
- **Rows tile, as Vim's lines do** (Joel, 2026-11-06: "rows in the launcher
  should have padding instead of margins"): a row is a band with its room
  inside it, and bands meet with no gap. A one-line band is 48, its line
  centred by the band rule; a taller row keeps an apart (12) over its first
  line and under its last. The cursor row's ground fills its whole band,
  from one inside edge of the frame to the other, gutter included, square,
  while its ink stays on the inset. The list's first band starts a tie
  under the head's hairline and its last ends a tie over the foot's, so a
  ground never touches a rule. One shape for the launcher's rows, the
  command line's, the held list and the drawer's lines. A selection inside a
  line covers its characters exactly, as Vim's Visual does, and so does a
  find's wash.
- **One hairline**: a logical pixel snapped to the device, from the frame's
  inside edge to its inside edge, never touched by a ground.

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
| the radius, `shape.radius`  | a whole thing: a window, or anything floating; square at its foot where it stands on another, as the launcher's drawer over its card |
| a part's radius             | a part of a thing, always the radius less the border and `shape.inset`, never below square; at one unit's radius, square, since a smaller curve inside a small one wobbles |
| square                      | not a thing at all: text, a hairline, a bar's fill              |
| a hairline across a surface | splits what's typed from what comes back                        |
| cloudy, lifted by a shadow  | it floats above the windows, which stay opaque                  |
| monospace                   | typing, or the machine being precise: commands, chords, figures, programs' names |
| the serif upright           | words to read: what a thing holds or says                       |
| the serif italic            | the system speaking to Joel                                     |
| size                        | the tier, by level: a title over content, content over a surface's furniture |
| weight                      | what to read first in a line, by level; never a state           |
| colour                      | focus and state, and pushing back; never a rank alone           |

No surface picks a radius or a border width of its own; a check refuses any
that does. The kinds that follow from it:

| Kind    | For example            | Border                    | Where                 | Type              | What sets it apart |
| ------- | ---------------------- | ------------------------- | --------------------- | ----------------- | ------------------ |
| window  | a program              | the accent when focused, the edge otherwise | tiled | its own | opaque |
| palette | the launcher           | the accent                | a quarter down; its command line over a picture or a video low, a twelfth of the screen up, so what it acts on stays in view | large, monospace  | a command line over a list |
| prompt  | polkit, the lock screen | the accent, the alert after a wrong answer | dead centre over windows; the lock screen and the greeter low, a twelfth of the screen up, so the picture's sky stays clear | large | a scrim stops what's behind it; who asks and why, in words, then the padlock beside the dots |
| notice  | a notification         | the edge, the accent while it holds the keys, the alert when critical | the frames rung inside the window at the top right | large, both | three groups, the room from frame to ink the same on every side: who first, then the message, then a signature of what sent it and when; Super+N holds the stack as lines, Vim's quickfix list |
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
glyph muted, since what it's for is inside. A figure, a value an answer's
part only says, wears none, even where Return copies it, and its words keep
the column's edge; a colour's swatch stays, since it is the figure. A row
that says how it stands by its mark, as a budget and its windows do, keeps
a warning's or an alert's hue on it, holding more or not, and a figure of
those wears its mark. Return's own Copy wears the copy glyph, as every
Copy row does, never its row's. In a
block of text a glyph never starts a line, since it would leave a hole under
itself or push the text off its edge. It sits inline before the word it
marks, or in a signature, as a notice signs what sent it: its mark in a
column of its own, then its name, and the time at the right. Edges line
up to the ink, not the box, so a glyph's left edge and the letters above
it share a column.

**Chords are drawn key by key:** a key's own glyph where a proper one exists
(⇧, ↵ and the arrows), its name where none does (Super, Ctrl, Alt, Esc,
Tab, Space), and a letter as itself, joined by + with the same room on each
side, all muted, even on the selected row. A chord is spaced the same
wherever it's drawn, in a row, the foot or a drawer's last line ("17 more
· Ctrl + W l"), by one piece that draws every chord, never as a string
typed out (VKCF3Z, 2026-11-07). Super is named, since the NixOS
mark at Note's size reads as a gear (Joel's shot of 2026-11-06 read "⚙ +
0"), and a chord's figures take Monaspace's slashed zero (`cv01`), in
chords, the foot's keys and the key list alone, so Super + O never reads as
a zero. A chord sits at the right of its row, at least `space.xxl` from the
name, but a place's row, at home or in Desktop, hands its chord to the
foot, which teaches Now's "Super + O next time" while the cursor is on it,
furthest from Return and first to give way, since its right says how it
stands. Nothing says what Escape does, since it only ever leaves a mode.

**A row says what it is.** A thing's row (a program, a window, a pi
session, a network, a device, a copy) says its kind at its right edge, muted:
`program`, `window`, `pi`, from one closed list, so a kind is learned once.
Its detail, a window count, a workspace or what the program is, follows its
name, muted. How it stands goes at its right, the state first and then
its figure ("paused · 9:50", "ready · short of memory", "this machine ·
−75 dBm"), never after its name. The key that carries an act ending
something is the foot's alone, never the row's, so the cursor row says how
it stands as its neighbours do (VKCF3Z, 2026-11-09). A note is a run of items, lowercase
where the words are ours, never a sentence with a capital and a stop
("found by its meaning"), written most-needed first, so that short of
room it gives way by whole items from its end, never to an ellipsis
(VKCF3Z, 2026-11-08). A run that wraps, in a drawer or a plate,
breaks between its items, the break taking the dot's place, so no line
ends or starts on one. An identifier wraps at its own marks, each dot
kept with the part before it ("com.onepassword." over "OnePassword"),
and a run of 20 or more letters and digits with no mark, a hash, may
break at any letter once no slash or hyphen fits (VKCF3Z, 2026-11-09).
How it stands never gives way: short of room the
note goes first, then the name ends as a long name does. Two rows that
would read alike (one track in two players) each say in their note what
tells them apart, and a file's name kept there gives way from its middle,
keeping its start and its ending ("second-…take.mp3"), as a path does;
that is the one "…" a note holds. A file in the Nix store is named
without its hash. A row for an act or a set already says what it is in its name
("Lock the screen", "Windows"), so its chord stays at the right. A thing's
chord is taught in the foot instead, "next time" and the keys. Inside a
collection the path in the field says the kind, so its rows don't. What
the rows say at their right takes one column's room, as wide as the widest
said there since the last key and growing only, within each row's share,
so where a name is cut holds still as "working" becomes "needs you" and
back.

**A long name reads on.** A name too long for its row ends in an ellipsis,
and once the selection rests on it, it reads on in place: after half a
second it slides until its end shows, rests a second, then glides back to
its start. It reads on twice, then rests at its start, ellipsis showing,
until the selection moves, since motion that never ends ranks nothing and
costs frames while he reads; the start is what names the row, moving off
and back reads it again, and the drawer holds the whole name where the row
has one. Only a selection at rest sets it going, so holding an arrow never
does. The slide is a reading motion, not an
arrival: a steady twelve characters a second with soft ends, gathering
speed from rest and settling back in at most a fifth of a second each, so
the eye follows one speed and nothing starts or stops hard. It moves on
the render thread, so a busy shell never stutters it. Only the name
moves, inside its own room, so nothing beside it does, and given up by
the selection it's back at its start at once (Joel, 2026-10-25: titles
"get elipsed really quick"; 2026-10-29: the slide stuttered, was too
slow and started and stopped abruptly). What a row says at its right,
cut to its share, reads on the same way, by the same piece, and never
opens the drawer to be read (VKCF3Z, 2026-10-30). With both cut they
take turns, the name first and then what it says, so only one thing
moves at a time. Nothing reads on unseen: a closed launcher keeps its
selection, but its rows rest at their start until it opens again.

**Text that doesn't fit reads at a glance** (Joel, 2026-11-11: "i hate
gross line wraps, particularly wheen they make it hard to parse
information"). What's too long is made shorter by design, in a written
order, so every line still says one whole thing:

- **A row is one line, always.** Short of room its note gives way by whole
  items from its end, then its right shortens to its short form ("about
  $604 in all" to "$604"), then its name ends at a whole word in an
  ellipsis and reads on at rest. How it stands never gives way.
- **A source's raw text is made a name before it's a row.** A mail's
  subject, a meeting invite or a window's title loses what the row says
  elsewhere or the drawer holds: "Accepted:", "Invitation:", "Re:" and
  "Fwd:" become the row's state; a schedule after "@", a time zone or a
  sender in brackets, an address and a program's name after a dash go to
  the drawer. "Accepted: Joel / Chad 1:1 @ Every 2 weeks from 11:30am to
  12pm on Monday (AST) (Chad Cromwell)" is the row "Joel / Chad 1:1",
  its right "accepted · 2 Nov". An agent's window is named by its
  session, never its title's "π - " and folder.
- **A link reads as what it points at.** A pull request is "world#2396409"
  and a review on it "world#2396409 review", a page its site and title; the
  address itself is in the nerd fold, whole, where y copies it.
- **Only the drawer wraps, and only where a line is prose or a value.**
  Its title takes two lines at most, broken between words and balanced,
  never one word alone below, and past two ends at a whole word in an
  ellipsis, its whole kept in the nerd fold. A fact sits beside its label
  while it fits, else under it, whole, breaking between words or at an
  identifier's marks; a label never wraps. Its why and its body wrap as
  prose at the serif's measure, five lines at most, the rest in the fold.
- **Nothing that reads as one is split.** A figure and its unit, a date's
  parts, a chord's keys, a name and its count, and a time and its zone are
  joined so no break falls between them.
- **Never wrapped:** a row, the foot, the path, a head, a label, a
  figure, a key.

**A figure with a range draws a graph,** set in braille in the monospace
face and always ten characters wide, so every graph is the same size: a
bar for a level (the charge, a light, the volume), filling from the left;
a spark for a history, filling from the right with the newest last, a step
brighter; a strip for a reading of each member, such as each core, from
the left. Every dot lies under it in the edge colour at half its opacity,
so an empty graph and a full one both show where they'd end. It's muted,
text on the selected row, and the warning colour goes on a reading of 95%
or more, or all of it when the row says it warns; a row that's never a
warning however high, as a full battery isn't, says so. While a row says it
warns, what it says at its right takes the warning colour with its graph, one
rule for every such row: an update waiting, a device's charge running low, a
short disk. Its mark keeps its kind's hue, or it stops saying what it is, and
its note in the foot stays plain, since the words already say it. A row's graph sits
between its words and what it says at the right, and the graphs in a list
stand in one column, clear of the widest of those, the name giving way to
it before anything else does.

**The space from text to its frame is the same on every side, and on every
surface:** the inset rung, measured to the ink, so letters start the same
distance from every frame (a row, a field, a footer, a notice's message, a
drawer's facts). Nothing is nudged by a number off the scale to even out the
ink. Large type sits in rows of `space.xxxl`, as a meter's row does; lines of
one thing sit at their level's pitch, Note's 20 and the large levels' 24. A
footer is a region, a hairline over it, its baseline a row below the last
row's in the launcher and a large pitch below the last line on a surface with
no row pitch of its own, as the picker, the keyboard map and the prompts.

**A notice sits the frames rung inside the window under it,** a frame on a
frame, so it lies on one window and never over the wallpaper or across the
border between two columns, and never nearer the screen's edge than the
screen rung; with no window there, or one too narrow, it sits the screen
rung from the corner. Notices in a stack sit the frames rung apart. The
margin's marks sit at the top centre instead, on the line the launcher is
centred on.

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
already true as the desktop started, says nothing, since he knows. Every
battery runs low at 20% and needs him now at 10%, the laptop's and each
Bluetooth device's alike: each device is a notice, a mark and a row of
its own, signed with its name and drawn with its kind's glyph, so a
headset running low never reads as the laptop. Its row in Now is its own
row in Bluetooth, saying "12% · running low" at its side, never a second
way of drawing it. Once low, a battery only
goes deeper until it charges or rises five points past the mark, so a
charge wavering about a mark is never said twice. The
desktop or its apps kept waiting on the same thing, memory, the disk, the
processor or niri, for a minute shows quietly, as something left on, from
lag-watch's windows; a moment's wait never does, since Joel feels it
himself, and the launcher's "lag" says what waited.

What each is, in words, is the launcher's Now, opened by Super+O on its
first row, For You, or by clicking a mark, on that mark's row: a
row each, its glyph, its name, how it stands at the right, and what more
there is to know in the footer, grouped by how much it asks with what
wants Joel first. Return does there what it
does on that thing's row anywhere, and Ctrl+] goes to it in its own place. While it's open, the mark
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
look at one movement closely. The launcher's selection, and the list's
scroll with it, lands within 50 ms of the key, a short glide on the curve,
since a highlight still moving once Joel has read its row is lag
(2026-11-10): 30 ms, shorter after a key already late, and none after one
too late for a frame of it. A held key moves the highlight without a glide,
so each repeat stands on its row in its own frame (2026-11-11). Nothing
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

## First, and Light

The shell is how the desktop is driven, so it answers before anything else
wants the machine, and it asks little of the machine in return (Joel,
2026-10-29: "it is the main control interface for this OS, so it needs to
always be responsive over anything else. it also needs to use a small
amount of resources so it doesn't put too much load on the system").

**First.** The shell comes before every program, build and agent for the
processor, memory and the disk, and after niri alone, which draws it. Its
share is a weight above theirs and its memory is protected from being
reclaimed, so the budgets above hold with every core busy, memory under
pressure and the disk saturated. That's measured, not assumed: a key's
frame and a chord's keys are timed while the machine is loaded that way,
and a check fails when either passes its budget. Coming first is never a
cap on anyone else; what's idle is theirs.

**Light.** Coming first is only fair while the shell takes little, so
what it takes is held to numbers, measured on the running desktop:

| Measure                                  | Holds to                                      |
| ---------------------------------------- | --------------------------------------------- |
| processor, launcher closed, nothing live | under 1% of one core, over five minutes       |
| frames, launcher closed, nothing live    | none from any window, over 30 seconds         |
| wakeups, launcher closed, nothing live   | under 5 a second on the shell's main thread   |
| processor, launcher open and still       | under 3% of one core, over a minute           |
| processor, a key typed                   | its work done inside the 4 ms search budget   |
| memory resident                          | under 300 MB, and no more than 10% over a day |
| threads and open files                   | the same after a day as after an hour         |

What isn't on screen isn't read: a reading is taken while something shows
it, no more often than it changes, and stops when it goes. Anything that
keeps the shell busy while nothing changes is a bug, found by profiling and
given a check that fails without its fix.

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

Importance shows by order and by the type levels, never by boxes. Calm never
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
  Bluetooth, audio, brightness, do not disturb, Keep Awake, notifications
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
| Do Not Disturb, Keep Awake, recording, sharing, mic muted, caps lock   | the margin mark, while true                        | Now; the session's rows |
| Keyboard layer                                                         | the margin mark, while locked; nothing while a key holds it | Keyboard            |
| Music playing                                                          | the margin mark while it plays; a notice for a moment when the track changes or a media key is pressed | Now |
| What waits on Joel: an agent's want, a to-do he can do now, a look, a source's to-do, a critical notice unread; a landing to try once the to-do daemon streams it | For You's tray, the margin's one mark for it, while any waits, counting as For You's row does; an agent's want a notice once | For You (Super+O); Agents ("pi" finds them) |
| A camera open, and by what                                            | the margin mark, while a program reads one         | "camera"; Now               |
| In a call                                                              | the margin mark, while in one                      | Programs › Tuple; Now        |
| Notifications waiting                                                  | nothing in the margin: they're what came lately, and a critical one is For You's | Notifications (Super+N); Now |
| A password or one-time code copied                                     | the margin mark, until the clipboard clears it, since the launcher goes as it copies | Passwords' foot; Now |
| A password reused or breached                                          | nothing, and nothing guessed, until Joel presses Check Every Password, which reads every password, so never on its own | Passwords › Needing Attention, counted, shown again after the vaults in its manager's program; each entry's Health in the warning's role, "reused in 3 other entries", "seen in 1,204 breaches" |
| Disk space                                                             | the margin mark, only while a disk is short of room, a writer is frozen or nothing watches it; a notice once a disk is nearly full, again when the guard has made room or can't, and when it freezes a writer, each saying what to do or that nothing is needed | Now |
| RAM type, speed, maker and part                                        | nothing, since it never changes                    | Memory's Type                |
| AI spend and its caps                                                  | the margin mark, only while a finding wants a look or nothing watches it; a notice once when one starts to; a line each morning; the launcher's foot | AI spend, a part of Agents (Super+U); Now |
| AI tokens running out                                                  | the margin mark, only while they run out within four hours at the last hour's rate, or are out; one notice, its words following the minutes in place, rising again only as the time left steps down | Now; AI Spend, a part of Agents |
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
The launcher's field gives way in its path's steps: a folder's path is cut
first, in its middle, so the program's ask and the place he's in stay whole
until the paths can give no more (VKCF3Z, 2026-11-04).

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
  once, in its vocabulary (`modules/quickshell/launcher/words/`, the
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
  Show opens a place or a program on one thing it holds ("Show in
  Bluetooth", "Show in 1Password"), and it's the one word for looking, so
  no act says See. An act naming one part of its thing takes its article
  ("Copy the Title", "Copy the Link", "Copy the Password"). A password's
  Return copies it, since that's its use rather than a copy of what
  describes it, so Copy last doesn't move it; the secret itself reads
  "hidden", and nothing shows it. A login's Open the Website opens a
  web page in the browser its password manager's extension fills in,
  wearing that browser's mark, and the launcher never fills it: where
  the manager can't be asked to fill it, as 1Password can't from outside
  its app, the foot says the extension offers to ("1Password offers to
  fill it on the page"), and no secret passes through the launcher. With
  no such browser, it opens in the default one with the globe's mark,
  and the foot says nothing more; it's one act either way, never a
  second beside it doing the same (VKCF3Z, 2026-11-08).
  A quest's Go To Its Session is the one that names where, as it pairs
  with Start a Session.
  Ending the session asks once more, saying what it closes ("Reboot now,
  closing 14 windows").
- **Things hold their parts.** A thing is one place for all of it, and
  what's part of it is inside: Agents holds the agents, then their
  Quests, Runs and AI spend; the Keyboard holds its layout and keys, the
  keyboards plugged in and every shortcut; Slack holds its theme, so
  "slack" is Slack and "slack theme" the theme. A part is still found
  from anywhere by its own name. A program holds what controls it, the
  same for every program and each only while it applies, made as it's
  stepped into: first what can be done (Go To the window used last,
  New Window, New Window Below, and its desktop entry's own actions such
  as Chrome's New Incognito Window), then what it has open now, each
  the thing it is (a file, a site, every tab behind the front one too),
  newest looked at first, saying so first at its right and where it is
  after ("open in 2 windows · ~/Downloads"), so a side cut short loses
  where and never the state, then its other windows by title, only
  those no open thing stands for, then its Recent, only what's closed,
  its Notifications, newest first, its Shortcuts, any part of
  its own, such as the theme, and Quit, asked once
  since a window may hold unsaved work. Media while it has a player
  (Play or Pause, Next, Previous, and what's playing) and Sound while it
  has a stream open (Muted, Louder, Quieter, and its volume as a
  bar) come last, since they come and go. Next, Previous, Louder and Quieter keep the
  launcher open to be pressed again, its figures following; Media, Sound
  and Notifications are found only inside it.
  A setting is a part saying how it's set, its choices a step in, so going
  into a thing lists a few parts rather than every choice: a
  display's Scale "1.75×", Refresh, Resolution and Rotation, then what
  its EDID says (Colour depth "10 bit"). What's typed beside them finds
  their choices too, so "scale 1.5" finds 1.5× without a step in, the
  setting's name a hidden word on each of its choices.
  A part is found from the top by its name, three letters or more,
  saying where it lives (Rotation, Desktop › Screens › Built-in screen;
  Health, Machine › Power › Battery), and Return goes there, the path
  filled, so - climbs back out. A part named as something
  asked whole at the top (the processor's Cores) is that answer instead.
  Only what a thing reads or holds is found so, never what it does,
  except a program's own acts (Meet's New Meeting), each found by its
  name with the program's mark and its path (Programs › Meet), below the
  program itself and below anything else found as well, as a call's
  copied link is. Return does the act, and Tab goes into the program
  onto it. A program found by its name lists what it holds under it, a
  few of each in its page's order: Chrome's open tabs and recent sites,
  kitty's folders, Calendar's next meetings, Mail's newest. Each sits one
  depth in under the program, as a session sits under its orchestration,
  so the program stays first and Return still switches to it in one press
  (Joel, 2026-11-07: "keep"). Its say counts what it holds by kind ("1 tab
  · 2 recent", "3 today") and names one ("next 14:00 · Design Review") only
  where none is listed under it, so it never says a row below it again.
- **Recent means one thing.** Recent is the last nine things Joel did
  with a thing, newest first, each once, by what it is (a file by its
  path, a site by its page), never what's open now or already shown
  above it (VKCF3Z, 2026-10-30). A program's holds what it opened or
  saved and has closed since, a person's what he had with them (a
  meeting held, a call, a conversation), Capture's what he took, and
  each is laid out by the one rule (Recent.js). Calendar's holds the
  meetings he had in the past 30 days, never one he said no to, a
  series once at its latest, its drawer saying how many came before;
  Mail's what he sent or read, a conversation once by its newest and
  none still waiting on him, one he sent naming who it went to but
  him, two at most and then how many more, giving way by whole names
  to one and how many more, then to how many in all, "to 3 people",
  before its name gives way, while its time never does. A meeting
  over says when it was, "yesterday 15:00", and notes nothing (VKCF3Z,
  2026-11-09). What a program holds now,
  its open rows or today's meetings, comes before its Recent, in its
  note and in the rows under it found from the top. It always
  stands where its thing keeps any, so it never arrives late at the
  bottom: how many at its right, "none" while it holds nothing and an
  ellipsis while its source first answers, nothing after its name, the
  names it holds newest first in its drawer, and "Nothing yet." stepped
  into empty. Every part inside a thing that holds things, Unread too,
  is a place's row: nothing after its name, how many at its right, its
  members in its drawer, nothing in the foot (VKCF3Z, dafc2199).
  What waits on Joel is
  never Recent. A person has no Recent, which named when rather than
  what: their page says what's between them by what it is. Waiting
  (their mail he hasn't read, and a conversation owing either an
  answer) comes first, above the ways to reach them, since it needs
  him; then their next meeting, Last Met (meetings and calls, its
  side when the last was, "yesterday · 1:1" for the two of them),
  then Messages (its side "they wrote · 14:02"). Each stands only
  while it holds anything, and one thing alone stands as itself, its
  own act on Return, saying at its side what its heading would have:
  "needs you · mail · 14:02", "met · yesterday" (VKCF3Z, 0cea8a93
  and 17e0f150, 2026-11-12).
- **Acts in one order.** Every kind of thing lists what's inside it in
  the same order, so three anchors are all there is to learn: the top row
  does it, what ends it comes near the bottom, and Copy is last. Between
  them, what leaves the thing as it is comes before what changes it,
  then who and where it's with (a mail's sender, a window's program),
  then its parts, settings before readings; then what ends it (Quit,
  Stop, Forget, Close), each asking once when it can't be taken back,
  saying what it ends; then Copy. A switch is one row that keeps its name
  as it flips, its note saying how it stands (Keep Awake, "on, lid shut
  too"; Night Light, "on"), and a setting's two states are never
  a place holding both. Return flips it and the launcher stays; one that
  would cut off something in use asks once first ("Turn Bluetooth Off,
  Disconnecting the Keychron"). An ask names the act on its thing, from
  the act's own words, never a place's name joined to it: "Reboot the
  Machine, Closing 12 Windows?", never "Reboot Power?". A choice is said by its value alone
  ("100%", "60 Hz", "Upright"), the setting it's for named in the path
  above it, the one chosen saying "chosen", whoever picked it; its
  note says only what marks it out ("preferred", "custom", the default's
  name), and its foot "↵ set", never the value again. "In use"
  is only ever what a program holds, as a camera Chrome has open. Rows
  that come and go, such as Media, sit at the end of their group. A check walks every row to hold
  the order.
- **Each kind of list keeps one order.** With nothing typed, a list
  moves only by what Joel does, never by a guess, and a choice never
  moves when it's chosen; only typing ranks, and what's picked is
  learned wherever typing searches, inside a thing's page too, for that
  page alone (RSCH-20261030-GB56HX). Every list is one of these kinds:

  | Kind               | With nothing typed                            | Where                             |
  | ------------------ | --------------------------------------------- | --------------------------------- |
  | A fixed set        | the order it was designed in                  | home, a thing's parts, its acts   |
  | Things returned to | open or favourite first, the last act, A to Z | Programs, People                  |
  | A stream           | newest first                                  | Recent, Notices, Clipboard, mail  |
  | Live states        | how each stands, then when it started         | Sessions, Windows, devices, Wi-Fi |
  | Amounts            | the worst first                               | Processor, Memory, Traffic, disks |
  | Choices            | their natural order, the chosen one said so   | Scale, Refresh, sound's devices   |
  | A search           | the same letters land on the same row         | everywhere typed                  |

  A to Z is only ever the last tie, or the order of many peers alike, as
  a large folder's files.
- **Under what it belongs to, a step in.** A session orchestrated by
  another sits right under it, its glyph and its name a glyph's column
  in, so its glyph stands where the name above it starts, and two steps
  at most. No line, box or fold says so, and the selection keeps the
  whole row. An orchestration stands where its most urgent row would,
  and every list of sessions keeps one order of how they stand: needs
  you, working, in line, listening, paused; one idle is listening, never
  waiting. Only typing ranks, so a search's rows, and a row that arrives
  while the list is open, stand at no depth, whom each is under leading
  its note ("under QX7RTA · ..."). An orchestrator's figure is its
  whole orchestration's ("$14 today in all"), its own and its sessions'
  inside it. A live session past 80% of its context says how full first
  ("84% full · working · $4.20"), its glyph in the warning's hue, the
  figure kept longest while room runs short and the money giving way
  before how it stands, since its drawer says the spend. Its drawer's
  first screen holds what matters of it, never only its title (Joel,
  2026-11-07): Doing, its step alone ("edit Drawer.qml"), since how it
  stands is its row's, and none where it has no step; Needs You For;
  its Last Landing; what it spent; its figures; Context Rebuilt; its
  Quest, how that stands said only where it isn't active; its Machine;
  and Working On last, as words, without the title the drawer already
  says (VKCF3Z, 2026-11-07). A row's note says no title again either,
  and gives way at a whole word, never a letter and an ellipsis. Agents
  is a place with sources, as Passwords is (Joel, 2026-11-07: "password
  management should be a universal concern with 1password being a
  particular implementation. same with agents"): a session's source is
  named only while two or more are on the machine, first in its note and
  never in its name, its glyph the agent's mark whatever runs it; its
  drawer's eyebrow always says it; typing a source's name finds its
  sessions; and an act its source can't do is absent, never dimmed.
  Its figures go by
  OpenTelemetry's gen_ai names: Context ("142k of 200k") and Last Turn
  first, then Model, Turns Today, Tokens and Cache, today's only on the
  day the harness counts them; none is money, so sharing hides none.
  Context draws Meter's stroke of how full after its value, in warning
  from 80%, and its own spend today Graph's spark of its hours, scaled
  to the day's busiest hour of any session, so a quiet one reads quiet;
  Context Rebuilt says what rebuilding its context cost today, an amount
  hidden while sharing and unsaid under a cent. A drawer's fact may carry a figure so, a stroke or Graph's
  spark, after its value on the value's baseline, its room counted in
  how the drawer lays out; where it still won't fit, it gives way before
  the value. It is no line for the cursor.
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
  ⟨unit⟩ in ⟨unit⟩"), and Ctrl+Y or Return on one puts its start in the
  field, out at the top, to finish there. A command is the one row
  written as it's typed, in lowercase; every other row's name, and every
  step of the path, is in Title Case as the glossary sets it, and what an
  empty list says is a sentence, above the one act that fills it where
  there is one ("No sessions running.", then Start a Session), or above
  the one way elsewhere when that is all it holds ("Nothing unread from
  the desktop.", then History). The sentence is never a row the cursor
  lands on.
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
  Programs, People and Passwords, what Joel works with; then Desktop,
  Machine, Network and Keyboards, what he works on; always in that order,
  nine rows, so home's shape never changes, and Passwords only on a
  machine with a password manager, beside People (VKCF3Z). Disk is a
  place in Machine, beside Processor, Memory and Heat, with Lag after it,
  since what slowed the desktop is one of them, and a disk filling
  fast shows in Now. Now's top never changes: For You first, always,
  nothing after its name, as a place's row, its side what each of its
  bands holds as a run ("1 to do · 44 to look at · 2 to try"), or
  "nothing now";
  then its own
  rows, always there in this order: the time, the date, the location,
  the weather and the next meeting, with no Today to step into, each the
  one row that thing is anywhere it shows, never a version Now draws of
  it. Below them, only while true, come what else wants Joel (a low
  battery; mail and notices are For You's), what reaches other people,
  what's playing and the modes left on, by name within each (the power
  profile only once picked by hand over the one the machine chose for
  the charger and the battery, since its own choice isn't a mode Joel
  left on). For You is a place, so Return steps into it whatever it
  holds, and Return on what's first goes where it's done; Super+O,
  Return, Return reaches the pressing thing, each press foreseen. Every place
  and list opens on its first row, Now and For You included, so the same
  keys land on the same row every time: what's pressing earns the top by
  order, never by the selection travelling to it (Joel, 2026-10-29:
  "when opening the launcher it selects seemingly random things"). A
  place's row says nothing after its name, not even "none", as no other
  row does there (Joel, 2026-11-09: "they still have a lot more text
  right beside the title than most do"; VKCF3Z, dae7fb67). What it holds
  is in its drawer, which comes out as it's selected, so one key shows
  it: its eyebrow "place", then its members by their own names in its
  own order, a run of middle dots laid as a fact's run is, breaking
  only between members with the dot going with the break, so no line
  ends or starts on one ("Power · Bluetooth · Sound"; VKCF3Z,
  8e0712aa). Its members are what it holds, never
  what it does: Passwords holds "1Password", not Check Every Password. A
  place holding things of one kind says what they are in a sentence
  instead: Keyboard's "Every key on the keyboards plugged in, by its
  names, and its layout" (VKCF3Z, 00ca9231). At its right, in Note, it
  says how it stands, as that row reads inside its parent (Processor's
  "99% busy", Passwords' "locked", how Agents' sessions stand, "none
  running" with none), else "none" or "nobody lately" while it holds
  nothing, a lowercase run and never the sentence that stays inside it,
  else how many it holds ("78 keys"); a place that only groups others
  (Machine, Network, Desktop) says nothing there. What's pressing leads
  that run: Timers reads "rang 21:20 · 1 running" once one has rung and
  "paused · 1:00 · 1 running" while its first is held, else "2 running ·
  23:44", the soonest's time left (VKCF3Z, 158ac71f). Return on a place
  opens it, and its foot says so ("↵ open"). A thing keeps its detail
  after its name, as every row does: a call names who's in it, a
  meeting its time, a track its artist. Only a row that opens a list of
  its own goes bare (VKCF3Z, 2026-11-09). Now's right names the
  first thing waiting on Joel, its minutes and how many more he can do
  now ("Answer the ask · 2 min · 2 more"); short of room its title gives
  way by whole words behind an ellipsis, its minutes and counts never
  (VKCF3Z, bd5cb965). Calm, it says what waits by its band ("44 to look
  at · 2 to try"), and nothing only when all are empty; For You's own
  row inside Now counts every band, needed most first ("1 to do · 44 to
  look at · 2 to try"), To Try giving way first, and as a place's row
  says nothing after its name, its drawer naming the heaviest it holds
  in a fact, Heaviest ("Answer the ask · 2 min"; VKCF3Z, a31a0e1b,
  3a92c450, 16efb1dc). Now's drawer is its moment plate, holding only
  the moment (the date, the clock, the week), never what waits, which
  home's Now row already says, and nothing more when all is calm
  (VKCF3Z, 8e0712aa, 25c253d3). A place's
  row is the same row at home and inside its parent, made by one
  function (Joel, 2026-11-08: "i don't like how the title and values on
  the home screen differ from every other sub-menu"; VKCF3Z, 2a429f67),
  and home's head is a place's: its mark, then "Home" in Title where
  what's typed goes.
  What a row holds, and a found row's note ("1 window · 14 tabs"), is set
  in the serif upright at the name's size, on its baseline and a step after it,
  while the name stays in the mono, so the eye finds the name first; it's
  muted, and the text colour on the selection, where muted reads under Lc
  60. What's on the right, a kind, a chord or a count, stays in the mono.
  Inside a place, things come by importance and relation, by their
  kind's order, A to Z only the last tie, and a typed command comes
  last in the place whose answer it gives (Network's "ip ⟨address or
  network⟩").
  Tab goes to the next row, as in his Neovim's completion menu, but into
  a row that holds parts, a program or anything else with a page of its
  own, since Down and Ctrl+N already go to the next row (VKCF3Z,
  2026-11-07), so "chrome" and Tab reach Chrome's page.
- **For You is the one place for Joel's attention, fed by sources**
  (Joel, 2026-11-08: "Why would we not just have a single place to grab
  my attention from that grabs data from various sources?"). Sources
  plug in as Passwords' do, each giving items of one shape, so a new
  source adds rows, never a place for him to watch. A source raises one
  item for each thing that happened, never one for each thing it found,
  so nothing floods it: a password check finding fifteen logins is one
  to-do, "Change the Passwords the Check Found", its side "3 breached ·
  12 reused", Return opening Passwords' list of them. What needs him now
  leads, one by one, by weight, heaviest first, so "the most critical
  stuff bubbles up": to-dos, what only he can do, each raised by the
  session that needs him through the one tool every session uses, and
  any look or landing that blocks a session or is due within the day.
  Then each other kind is one row holding the rest, shown only while it
  holds something, a place's row: nothing beside its name, its side a
  bare count, its drawer naming the heaviest (VKCF3Z, 2026-11-09; Joel:
  "there is no to look at section"). To Look At holds what a session
  points him at, never opened for him; To Try, landings whose line asks
  him to try something. Inside each, by weight, ties oldest first.
  Weight comes from what it holds up (a session parked on him counts
  most), a deadline near, its source's urgency and, slowly, how long
  it's waited; the order is
  worked out as the place opens and holds still while it's open. A row's
  mark and eyebrow say its kind ("from Reef Quest · look"), and its side
  the strongest reason it's there, state first ("session ended · 12 min
  ago", "blocks 2 sessions", "due 15:00", else "12 min ago", and before
  today its day, "yesterday"). For You
  wears an in-tray, full while it holds something and empty when it
  doesn't; a look wears an eye, a landing to try a flask, a to-do and
  History the person and clock, and a source with no mark of its own
  the tray (VKCF3Z, 2026-11-08). Lately
  follows, what came: a place per source ("Mail", "Slack"), nothing
  after its name, its side "12 to read", its drawer its senders newest
  first (VKCF3Z, 2026-11-09), ordered by its heaviest, and a notice its
  source calls critical lifts into the
  first band. A message inside is one row wherever it shows, Search
  Slack's hits included: named by its sender, its note its first line,
  or from the first word a find matched, its side its time, its eyebrow
  where it came ("in #deploys · message", "direct message").
  Then Later, what was put off or can't be done yet, each
  saying what for; then History. A source may raise one to-do of its own
  for what one of its checks found, never one a finding, so a check can't
  flood For You: Passwords' Change the Passwords the Check Found, "3
  breached · 12 reused", stands until a later check finds none (VKCF3Z,
  WCJQSG).
- **For You keeps itself true, and forgets nothing.** Nothing in it times
  out. An item leaves the moment its source says it no longer needs him:
  a check passes, a sign comes (a boot, a switch, a sign-in, a device
  seen, a file written), a pull request merges, a meeting ends, a
  message is read where it lives, or he clears it with dd; where nothing
  can tell, only Joel saying so, which its drawer says. History keeps
  everything resolved for good, no timer clearing it; a store short of
  room lets its oldest read item go first, never an unread one, as the
  desktop's notices keep their last 1,000. It reads newest first, each
  side saying how and when ("merged 14:02", "read in Slack 14:02",
  "cleared by you 13:10"),
  and `/` finds any. A kind or a source is a step-in, never a mode:
  `:filter look` or `:filter slack` goes in, Tab completing the kinds and
  sources present, each with its mark, its name and its count, the head
  names the kind by its name ("For You › To Look At", "› To-Dos", "› To
  Try"), and - comes back out. Every row has a drawer, "nerdy and
  useful" (Joel, 2026-11-08): its why whole in the serif; Weight, as a
  figure and its parts ("82 · blocks 2 sessions · waiting 2 h · due
  15:00"), so he sees why it's where it is; Takes, Waiting Since, Last
  Checked and Clears When; its source's own facts (a pull request's
  checks and lines, a mail's sender and labels); its session, the quest
  being the eyebrow's; identifiers last. Every moment in it reads as
  When writes it, forward as back ("tomorrow 15:00"), one moment one
  way. Return on a look opens it in its own program; on a
  to-do it steps in, where the first act is where it's done: Go To its
  window when it names one, else Deal With It, bringing its session
  forward with a prompt filled in and never sent; then Mark It Done, Put
  It Off and Open Its Quest (VKCF3Z, 2026-11-08).
- **Ctrl+Y only completes.** It takes the grey completion the field shows,
  as his Neovim's completion menu takes its selection, and does nothing
  while none shows; Tab moves through the rows, as that menu's does, but
  for going into one that holds parts. The
  field says no keys of its own: it holds only the find and how far it
  narrows, and its hints are the foot's, by key and lowercase word
  (VKCF3Z, 2026-11-05). So the aside ends at the field's end, where the
  rows' sides end (Joel, 2026-11-04: "the text in being inset left from
  the right edge of the launcher"). Nor does an arrow clear what's typed
  or step anywhere: the arrows move the caret, Return goes in and - goes
  up. The aside has all the room the path and what's typed with its
  completion leave it, and says less as they grow.
- **Every row is a thing with verbs.** Return goes into a thing that holds
  more, as oil's Return opens a folder, so a few letters and Return reach
  inside a thing from anywhere: what can be done to it,
  searched the same way, each with its chord, and a verb needing a target
  asks for it in the same field. What was typed found the row, so going
  in takes it away, and everything inside shows. Where Joel is shows as
  one path in the field, each step named as the row he picked was, joined
  by a muted › ("Shortcuts › Desktop › Workspaces"), and a step named as
  the one before it (the Shortcuts category's Shortcuts) is said once, so
  it reads as a place, not as tags side by side. In normal mode - goes up
  to what holds it, onto the row it came from with what was typed before,
  as oil's and netrw's - do; Ctrl+O and Ctrl+I go back and forward through
  every place been in, and Ctrl+] on a row found from elsewhere goes to it
  in its own place, Ctrl+T coming back (Joel, 2026-11-05: "where in vi
  does that step into shit?"). h, l and Backspace step nowhere: on a row
  they mean nothing, and the foot says so with Vim's way, "l not here · ↵
  open". On the find line the arrows move the caret, as in Vim. Escape
  cancels, then closes the launcher (Vi's Keys), and only in the launcher:
  the window behind never hears it. The foot names
  every key that works on the selection by its letter: what Return does,
  and - wherever there's somewhere to go up to. A step in lists what
  Return does first, where the row's own list left it out, so going in
  never hides the act Return would have done (Joel,
  2026-10-28: "enter is supposed to be the default action, but that means
  that action should be selectable within"). Every row has a Return act,
  the same wherever the row shows, and a check walks every row to hold
  it. A thing with one obvious act that does no harm does it: a program
  or a window is gone to, a mail or a file opens, a closed orchestrator
  carries on where it left off. Anything else goes in on Return, its
  likeliest act first inside, so Return twice does it: a person, a quest,
  a device, a network, a setting, a place. A network's Return only shows
  it, since connecting to one lets go of another. Return never calls
  someone or starts something new by itself: a call in Tuple's Recent
  goes in, Call Ada Again first, so Return twice rings her (VKCF3Z,
  2026-11-08). A row with nothing under it but acts never opens a step of
  them (Joel, 2026-11-11: "we have a number of menu items that i open to
  only see verb:copy ... if they have nothing under them, we shouldn't
  open a submenu for them"). Return does its act, y copies it, and any
  other act is named in its drawer and its foot by its key, so Return never
  lands him on a list of verbs alone. Where the act mustn't happen on one
  key, as a call or something new, the first Return arms it and the foot
  says so, "↵ again calls Ada", and any other key lets it go; Return twice
  still does it, with no list between. A check per place fails on a step
  whose rows are all acts. A meeting joins on Return
  only from ten minutes before it starts, and
  before then goes in, Join first, so Joel is never made to join early
  (2026-10-26: "they force me to join, and what if I want to cancel");
  the foot says which. A value copies on Return only where it holds
  nothing more; one holding more, as the battery's answer or the
  weather's Rain, goes in, copying it a row inside: "rain" asked alone
  and Rain inside the weather work the same. Copy is never the first row
  of a step, and is always the last. Return
  closes the launcher only
  when its act takes Joel out of it: to a window, a program or a file, a
  call, or something copied to paste. An act that changes something (the
  volume, a screen's brightness or scale, Wi-Fi, Bluetooth, Do Not
  Disturb, Keep Awake, the power profile, the desk light, a keyboard's
  light or layer, a program's playing) keeps it open, and its row says
  the new state at once (Joel, 2026-11-07), before the system confirms
  it. A refusal, or no answer in 5 s, puts the row back and says so in
  the foot until the next key, naming the act that didn't happen and
  why: "Home-5G not disconnected · not connected", or "Home-5G not
  disconnected yet · no answer", the yet saying it may still come.
  The failure's mark and what didn't happen are in alert and the why
  muted, and short of room the why gives way from its end. Going into
  one lists its acts, led by Return's own, which there does it and
  closes, so
  there's no Ctrl+Return to learn (Joel, 2 November: "all of those ctrl
  keys do not feel very vim to me"); a power profile's stays even there,
  since a pick lost to a closing launcher goes unseen (Joel, 8
  November). **An act that ends or takes away
  never runs on Return**, from any mode: Disconnect It, Forget It, Close
  It, Turn Off, Stop, an agent's Close, Done, and the session's Lock,
  Sleep, Log Out, Reboot and Power Off. An agent's Close always asks,
  naming the session by its address, since the path above says its title,
  and its ask says what stays, "Close 8IDU4E? Its worktree and 1 to-do
  stay. (y/n)", so ending one never reads as losing its work.
  Return on its row puts its command on the command line,
  `:disconnect`, the menu saying what it acts on, and a fresh Return there
  runs it; the row and the foot name the key that carries it, "d
  disconnect" where d's table gives it, else ": disconnect", as Vim's
  changes are operators and commands, never Return. Which acts end comes
  from the language's `ends`, never a list kept by hand. A copy closes
  saying so: the keys go back to the
  window at once, and the card stays a moment with "copied" where
  Return's key was, its done mark in the success colour, then leaves. A
  password's copy closes too, so it pastes at once, and while the
  clipboard holds it the margin wears a mark until it clears. What the
  foot says is lowercase, as a said value is ("copied", "told reef").
  A refusal names the thing, what wasn't done, then why: "GitHub's
  password not copied · 1Password is locked", "HP Z27 not unplugged ·
  the last lit screen". The thing is what was asked ("1.3× not set · HP
  offers 1× to 3×"), and the why is the world's own words, the first
  letter lowered and no closing full stop; an act that gave up says so
  the same way, "didn't install · stopped waiting after 30 min". Asked
  for what already holds, it says what's true, as Vim's `:set` does:
  "HP Z27 is plugged in".
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
  of bare figures. The weather's parts are Conditions, Rain, then the
  seven days from today, and away from home Local Time follows Conditions
  ("22:45 Saturday"). Conditions holds every figure of this moment: the
  temperature, what
  it feels like, the humidity, the dew point, the precipitation, the wind
  and its gusts, the cloud cover, the UV index, the visibility and the
  pressure. Rain says when it next rains ("from 15:00 · 70%") beside the
  next twelve hours' chance, and holds those hours; with no rain due in
  them, it gives up its place rather than saying so. A day holds its own
  figures, its high and low, the most it rains and blows and its sun, then
  its hours; an hour ("14:00") holds its figures, the same thing in Rain as
  in its day, and a day never says a figure its hours do. "rain",
  "umbrella" or "when will it rain", here or in a place, answers with the
  next rain, its hours a step in. The time
  here is the clock's, which follows the laptop into another zone once a
  fix says it's there to within a region's fifty kilometres and the zone
  keeps its time all round the fix's circle, never for a fix a country
  wide, nor one whose circle crosses a zone's line; the time then says
  so ("clock moved to Edmonton time, as the laptop is in Kelpwater"), and
  Put It Back leads
  what's inside, holding the zone it kept until the laptop is somewhere
  else. The time here holds the clock and never the date: its Zone, a
  thing saying its offset, abbreviation, next change and tzdata's
  version, then UTC, Unix time, TAI, GPS time and sidereal time, each to
  the second and counting while it's shown, then how the clock is kept.
  The date holds the calendar alone: the ISO date, ISO week date and
  ordinal date, the quarter, the Unix day, the Julian day and the
  modified one, then the season where the laptop is and the moon, each a
  thing. Nothing one says, the other does (VKCF3Z, 2026-10-30), and each
  reads the same on Now's rows, its plate and in a search, from one
  place (`launcher/Moment.js`). So is the
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
  too), "work", "warp", "vpn" or "tunnel" (the Work Network, Joel's
  link to work through WARP: its switch first, Disconnect WARP or Connect
  WARP, which :disconnect and :connect name and which leave a switch the
  organisation holds as it is, saying who holds it, and say when WARP
  always on comes back; then WARP's State and why, an Emergency Disconnect
  while one holds, the network's Health as WARP judges it, its state
  alone at the side and its note saying it's "WARP's measure" before the
  figures, so whose gives way last, its Settings a step in, the Mode,
  Always On and Split Tunnel first, each saying who set it, the split
  tunnel's every host and network a step further, then where names are
  looked up, sign-in pages and what the organisation allows, an override
  among them, the tunnel's Round Trip and Loss
  as WARP estimates them, its Protocol, Data Centre, Endpoint and Last
  Handshake, its Traffic each way ("1.52 GB in · 312 MB out"), Round
  Trip, Loss and Traffic sparked while shown, each with a drawer whose
  eyebrow says once whose figures they are ("via WARP · tunnel"), its TLS a
  step in, what its DNS proxy answered, each count a step in, and
  Through, WARP, named once, listed in Network after the Home Network
  where the system runs WARP) and "uptime". The
  queue is Agents', beside the sessions whose heavy work it holds, and
  "queue" finds it: each job named in words, its time against its last
  run or what it waits for, and its quest and memory below. The devices hold as much: "heat"
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
  grey, and Ctrl+Y finishes it. A row whose name starts the same way is
  chosen first, so "pow" is still Power Off. A colour's first row wears
  it, and so does the theme's nearest pigment. Return copies a sum, Ctrl+Y
  puts it in place of the question, and Return goes into
  an answer with parts, each copied the same way. A level typed after its name, "brightness 40" or "volume
  40", is an answer that does: Return sets it. The field's end says where
  an answer came from. An answer that comes to nothing says why once, as
  its one quiet row with the nothing-found mark, where its asking row
  stood; the field finishes nothing, and its end says nothing more.
  Nothing leaves the machine unless the question needs the network, never
  for a private address, and never with the launcher's `lookups` off.
- **Large sets are one row each.** The clipboard's history, past notices
  and the Wi-Fi networks in reach are too many to mix into the search: each
  that has a match adds one row, its best match with a count of the rest,
  so there's no haystack and no mode to enter first. Return uses that
  match, and Ctrl+] goes straight inside it, the set's crumb above it
  saying where it lives; Ctrl+O or Ctrl+T comes back to the search, since
  it goes where Joel came from. The whole set is found by
  its own name. The row sits below the other matches. Emoji add none, since a
  name like what was typed ("turkey" for "tu") is a row to rule out; they
  are found in their own place. A collection is found whole by its name or
  what a person would call it ("audio" is Sound, "notices" the
  Notifications), and Return goes into it; found that way, it stands alone for
  what it holds, so neither its members nor things that live in it, nor the
  shortcut that opens it, sit beside it. Each thing's row says what kind
  of thing it is at its right edge (Shape and Kinds).
  The windows, the copies and the notices are whole at home, and most have
  a chord that opens them: Super+V the
  copies, Super+E the emoji, Super+N the notices (the stack on screen,
  by the keys, where :e Notices opens the history), Super+Shift+/ the
  shortcuts.
- **A key's action is its row.** A key's action that a row already does
  is that row, drawing the key's chord: Super+L is the session's Lock,
  Super+Return is kitty, and Shift+Print and Alt+Print are Capture's rows.
  A row draws the shortest key that does it from where Joel stands, so in
  Capture the whole screen's row reads Print.
- **A capture key takes the screen as it is.** Print and Super+P never
  change what's on the screen first. With nothing of the shell's open they
  open Capture, the whole screen first; pressed again there, they close it
  and take what's underneath, so Print twice always captures the work. Over
  the launcher showing anything else they take the screen with the launcher
  in it, and it stays as it was, the keys on the same row. Ctrl+Print
  records the screen as it stands, the launcher included, and stops on the
  same chord; the margin's recording mark shows the moment it's pressed,
  and the recording begins once the mark is drawn, so every frame holds
  it. The launcher is kept out of a call's share and the notices out
  of screenshots, never the launcher out of his own pictures.
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
- **Every row says a little more beside it** (Joel, 2026-11-11: "i would
  like to have side drawers for just about every item giving me more
  details, not as much as if i opened it up, but enough to give me
  slightly more than we could fit in the line and to reduce how often i
  feel i need to dive in deeper. in fact, this is a concept that should
  probably be universally applied"). Every row's drawer gives what its
  line couldn't fit and a glance would want next, so he steps in less:
  Weather's tonight and next days, Date's week, moon and season,
  Location's how it knows, a disk's rate and when it fills, a person's
  next meeting with him. It's a little more, never the place itself: a
  handful of facts and a line of words, while the whole list stays
  inside. A drawer that would only repeat its row is a row still missing
  its facts, never a reason to show none. Now's rows come first, then
  every place's, and a check per place fails on a row with no drawer.
- **The drawer is the card continued** (Joel, 2026-10-28: "it should come
  out of the right side like a drawer, making sure it never passes the
  bound of the screen"). What a row has to show beyond its line (a
  picture, a notice's whole words, a mail, a meeting's details) shows
  beside the card's right side, a floating surface like the card, top
  aligned and as tall as it, 12 (m) from it. It's set as a plate, a small
  page about the selected thing (look 3): an eyebrow saying where and
  then what kind of thing it is, as Type says, at Label while every word
  is ours and in Note as written once one is the world's, a notice's
  sender by the name its card signs it with;
  the thing's name at Title, strong; its facts as a name at Note and a
  figure at Figure, the names whole and the
  figures on one edge 24 (xl) beyond the longest, a figure wrapping with
  each number kept to its unit and a run of items between them, never
  inside one ("38 s · 4.1k out ·", then "0.9 s first token"). A drawer
  lays its facts out one way: each beside its name while every one fits,
  else each under its name, since a mix reads as neither a table nor a
  list (VKCF3Z, 2026-11-07); a fact that's a sentence stands under its
  name as a paragraph and doesn't decide which. A figure's spark or meter
  draws in its text's ink on a muted track, in warning only past its
  limit's warn, as Context's past 80%. The facts run from what moves
  most, through what describes the thing, to its identifiers last in the
  Name face (a node, a bus name, a model's code, the name Linux gives a
  part, "Linux Name coretemp"), so the first screen
  holds what he came for (VKCF3Z, 2026-11-07). A fact still being read a
  fixed placeholder, and none that only says again the name, or what the
  row's side or note already says: a speaker's row saying "volume 100%"
  leaves its Volume fact "0 dB", and a fact equal in every part says it
  once ("stereo · balanced"). Said once runs the other way too: a row
  says what its neighbours' rows say, and what more only its own would
  is the drawer's, so with the drawer out the selected row's side reads
  as the rows around it ("9 devices", its backhaul in Backhaul), and a
  note leaves out what its drawer's facts say (a cap's "$0 spent", its
  Resets in the drawer). What's the same on every row of a list, as a
  folder every screenshot shares, is said once, in the eyebrow. A thing
  has one drawer wherever it's reached: Now's Microphone opens Sound's
  drawer for it, and Now draws its own only for what no place holds,
  as Disk or Lag (VKCF3Z, 2026-11-08). What kind of thing it is goes in
  its eyebrow, never a fact named Mode or Kind ("time server", "reference
  clock"), and a state of ours is lowercase wherever it's said ("normal",
  "off"). A figure takes the unit that fits it, "3.00 mV", never "0.00300
  V"; then its words,
  a notice's in the serif upright its card reads them in; then its
  picture, only one that is the thing itself (album art, a screenshot, a
  photo), never a program's own icon, for which its mark stands, and
  never on a notice's card, while History keeps a notice's words alone,
  since a picture can hold what's private (VKCF3Z, 2026-11-09). Its
  text is as far from its frame as the card's (the inset, l and the
  hairline) on every side, its foot included, so it ends at the last
  whole line inside that, and every line lies on one of
  the card's, so the two read as one page: the eyebrow on the field's
  centre, the name on the first row's baseline (the serif's own line box
  would set it 1.72 lower), and the rest on a pitch of 24 (xl), half a
  row, so each line falls on a row's baseline or halfway between two,
  one empty line between the name, the facts and the words. A name that
  wraps sets its lines 32 (xxl) apart, a heading's leading tighter than
  the pitch, so it reads as one title, and its block ends on the next
  whole line of the pitch, so what follows stays on the card's. Home's Now
  has a plate of the moment instead (look 3): the date as its eyebrow;
  the time at Display, in the mono as the lock's clock is, its capitals
  centred on Now's row, and its seconds counting after it at Note,
  never what waits, which home's row says (VKCF3Z, 25c253d3); and the
  facts behind the moment in two groups that follow the card's,
  the weather, the next meeting and where Joel is from Programs' line,
  the clock's keeping and the week from Desktop's, each figure's
  qualifier muted after a dot, and each fact in its place's words, as
  Time says synchronised. A group with nothing to show gives up its
  place to the next, so no empty lines wait in the middle for it; a
  row that's missing sets its part one empty line below the one before.
  The card's room and the drawer's are worked out for the screen as
  the launcher opens, the card centred unless the screen is too narrow
  for the drawer beside it, so the card never moves and never changes
  size for what's selected. A screen without room for even the
  drawer's least beside the card shows it only once the keys go in
  with Ctrl+W l, in the card's place and at its width, as Vim's preview
  opens in the one window there is, never squeezed until a word breaks.
  It's out only while the selected row is a thing (a program, a session, a
  quest, a mail, a meeting, a person, a file, a folder, a notice, a
  window, a device, a network, a copy, a value with a history) or a
  place, whose drawer names what it holds, and the drawer holds
  something the row doesn't already say: facts, a picture,
  words beyond its name, side and note, or more. A drawer that would
  only repeat the row is never shown (Joel, 2026-10-29: "we should
  never show the side panel when it is just repeating the currently
  selected text"). It's the same at home, in a search and in every
  place, and a switch or a choice has one saying what it changes and
  since when; only an act alone, which its row says whole, has none (Joel,
  2026-10-29: "the side drawer shows when it's not needed, and only
  shows in certain places"; and 2026-11-11, in Every Row Says a Little
  More). Between two things it stays and what it
  shows changes in place; onto or off one it fades in place once the
  selection rests, so holding an arrow never flickers it. It's Vim's
  preview window, a split the keys go into: Ctrl+W l takes them in, or
  his Neovim's Space and the right arrow over it, where its frame takes
  the accent, an accent rule in its left margin marks the cursor's line
  as the reader's does, and the reader's keys move it (j k, counts, Ctrl+E Ctrl+Y and the rest), the
  view following at his scrolloff by whole lines, its eyebrow and title
  among them, so no line is ever shown cut at its top or foot. It stops
  above the card's foot, which stays and, while the drawer has the keys,
  reads "Space ? keys · Ctrl+W h back", the way out at the right end as
  Return is on the card. Ctrl+W h or w, Space and the left arrow, :q, ZZ
  or Space w bring them back, and Escape and q do nothing there. Inside,
  the arrows move as j and k do, as in any Vim buffer (Joel, 2026-11-05),
  and Shift with an arrow scrolls nothing. Another program's words are shown plain, never as markup.
- **A file picked shows what it is** (Joel, 2026-11-03: "i see no
  previews for files ... no clue about sizes or when last updated"). In
  the file picker, a row in the folder shown says its size in SI units,
  three significant digits ("940 kB", "2.41 MB"), or a folder its count, and
  when it changed, as the launcher says when on a row: "just now", then
  minutes ago under an hour, its time today, "yesterday", its weekday
  within six days, its day this year, and its day and year before that,
  the locale's names for days and months, muted in the side slot. A
  row listed away from its folder says its folder instead. The drawer is
  its preview (VKCF3Z, 2026-11-03): the eyebrow its kind with its numbers
  ("PNG · 2400 × 1600", "PDF · 12 pages", "MP4 · 3:41", a text's by
  its kind's own name, "Markdown · 4 lines", or "over N lines" past
  the first 64 MB), its name, then
  a picture as wide as the plate in its own shape (a picture itself, a
  PDF's first page, a video's frame a tenth of the way in), a text's
  first whole lines or a folder's first items, quiet, in the mono on the
  pitch, then Size (a folder's Items, its number alone), Changed, said
  as its row says it so the two never read as two dates, and Where.
  Anything else shows its facts
  alone, with no stand-in. The facts draw at once from the folder read;
  the preview fades in once it's made, kept by path and change, and no
  key ever waits on it, nor starts a program: one file-preview answers
  the whole ask, a line at a time, and what it's made is pruned past
  128 MB, the longest unshown first. Every word the rows and the plate
  say is the launcher's own, in its words.
- **A picker says what it takes** (Joel, 2026-11-03: "no clue when
  something only accepts one file what happens"). In every folder of
  the file picker, the field asks for what the program takes, in its own
  word where it gives one: "open which file?", "open which files?",
  "open which folder?", "attach which files?", or "save it where?".
  Taking one, Return on a file picks it and closes, as the foot says
  ("↵ open"), with no mark: opening is undone by opening another, so
  nothing asks first (VKCF3Z, 2026-11-04).
- **What's gathered is in sight** (Joel, 2026-11-03: "it is hard when
  marking files to know what to do once mmarked"). Where a program takes
  several files, Return takes the cursor row's file and answers, and
  :arga gathers the cursor row's file, or a range's, across folders, as
  Vim's argument list does (VKCF3Z's vi-paradigm spec, The File
  Picker). A gathered row's glyph is the mark, in an act's hue, the one
  colour on its row, and Return then answers what's gathered, the foot
  saying how many ("↵ open 2"). :args steps into Gathered: the files,
  each with its folder, then Forget Them, then Copy the Paths, its plate
  counting them with their Size together. A folder's Copy the Path comes
  after its files and filters, held back until the folder's read so it
  never heads the list.
- **A folder lists in the order he chose** (Joel, 2026-11-03: "no ...
  ability to sort by anything"). :sort name (A to Z), :sort changed
  (newest first) and :sort size (largest first) list every folder so,
  :sort! turning it round, the folder shown made afresh on its first row,
  since a list shown otherwise keeps its rows where they are. Name is A
  to Z, case aside, 2 before 10; a folder sizes by its items. Folders
  come first in every order and what was opened lately stays apart on
  top. One order serves every ask and outlasts a restart, starting on
  name (VKCF3Z, 2026-11-04). :set hidden lists the hidden files and :set
  nohidden leaves them out; :e and :cd go to a folder, their paths
  completed from folders already read, as if the ask opened there; :e
  a file opens it; :mkdir makes a folder; :q answers nothing.
- **A save is named on the line** (VKCF3Z's vi-paradigm spec, The File
  Picker). Asked to save, the line holds the name the program gave, a
  prompt's words in insert rather than a find, the caret before its
  extension so typing renames it; Ctrl+W, Ctrl+U and Ctrl+H edit it, and
  Escape gives normal on it, where h l w b e 0 $ x dw cw and u edit it
  as Vim's do. The line is the row Return stands on, above every
  folder's rows, as an answer's headline is, so the name is never said
  again below it, and the rows are never narrowed by it: j goes down
  into the folders and k back up to the name, Return on a folder opens
  it and - steps out, as oil's do, the name kept, and Return on the name
  saves there. A name already there asks once in the line's place, "Replace
  notes.md? (y/n)", as Vim's :w says E13: y saves over it, n, Escape or
  Ctrl+C let it go, and any other key, or one in its first 300 ms, does
  nothing. :w [name] and :x save from anywhere, asking the same, and :w!
  saves over a name already there without asking.
- **Google Drive is a folder among his own** (Joel's Mac mounted it for
  every dialog, 2026-10-29: "i've used the folder a little bit but not a
  lot"; here it comes through the google program's own sign-in, never a
  client of its own, a mount or a second sign-in, 2026-11-07). Home lists
  Google Drive after its own files, apart, and typing finds it from any
  folder: My Drive, Shared with Me (not when saving, since another's
  folder takes nothing of his) and Shared Drives once he's on one. Each
  folder there lists as one here does, folders first, a row saying its
  size and when it changed as a file here says them, a folder only when,
  since Drive counts no folder's items; its plate's eyebrow says where it
  is then its kind, as Drive's own drawer does ("in My Drive/Trips ·
  PDF"), then its name, Size, Changed and Drive's Owner, with no
  preview, since nothing is fetched to be looked at, and nothing Drive
  doesn't give. Google's own kinds are the one row each is everywhere,
  noted "as .docx", ".xlsx" or ".pptx", the file a program is handed,
  its side saying when it changed, as every row in a folder does, and
  left out where the program takes none of those (VKCF3Z, 2026-11-08). Return
  fetches it and hands its path, the foot saying "Fetching it."; gx opens
  it the same way in its own program, one of Google's own kinds by its
  link in its program's window, never one that runs. A file fetched is
  kept in the cache home's google/drive/<id>/<name>, as Drive last
  changed it, so one asked again while Drive's modifiedTime holds is
  never fetched twice, and past 2 GB the copies handed out longest ago
  go. It's readable only, since nothing written to it goes back to Drive:
  a program opening it says so rather than losing a change. Saving into
  Drive is the picker's own save, never the cached copy: a name in My
  Drive or a shared drive's folder is staged in the state home's
  google/drive-saves and sent up once written, a new version of a file
  this program made, and a notice says if Drive wouldn't take it, since
  Joel's sign-in grants replacing only those.
- **Ctrl+C copies the selected row.** Any row with a Copy inside copies
  it with Ctrl+C from wherever it's listed, without stepping in, as
  Ctrl+C copies everywhere else on the desktop; while text is selected
  in the field, it copies that text instead.
- **A notice gone is not lost.** Every notice of the session stays in the
  desktop's memory. One clicked, or withdrawn by its program because it was
  read there, was seen; one that left by itself was missed, and the notices'
  row at home says how many until they're opened. One from a conversation
  with one open, the same program saying the same title, takes its place,
  so a conversation holds one card, its side saying how many came; the one
  it took over is in History, read "into a newer one" (Joel's wish that
  notices clear themselves, 2026-11-12). Return on one goes to the
  program that sent it, its window when one is open. Its row is named by
  its title alone, or by its words' first line when it has none; its note
  is the line of its words holding what's typed, else their first, giving
  way by whole words from its end, so one notice reads the same in every
  list. Its words are found but never its name (VKCF3Z, 2026-11-08). Its
  drawer's eyebrow says who sent it, then its kind ("from Slack ·
  message"), a kind the glossary doesn't name left out. Among the notices its
  row's acts are in the notices daemon's order (`modules/notices`' row.rs):
  going to what it's about,
  what its sender offers, Mark It Read (Mark It Unread once read), an act
  with nothing at its side, Forget It, and
  Copy What It Said last. Held, a card's acts are commands, never a row
  stepped into: the cursor card names what its sender offers under its
  sender, ":reply · :play", and : puts them first in the menu, after
  Return's going to it; d, :forg and yy do the rest. A
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
  sleeping so work left running goes on, and it keeps running with the lid
  shut, on battery as on the charger. Away from a desk, a shut lid locks the
  screen, as the sleep it holds back would have. While it holds, Keep Awake shows in Now,
  saying it's on, and the margin's mark says so.
- **What's shown is the door to its detail.** The battery opens the power
  menu, the network opens its list, a notification count opens the history.
  Nothing keeps its detail somewhere else.
- **Hints in place.** Wherever there are actions, their keys sit beside them in
  muted type, part of the design rather than a cheat sheet bolted on.
- **No hidden modes.** Do Not Disturb, Keep Awake, recording, a resize, an
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
  | Escape       | cancel and leave a mode, never close     | everywhere     | everywhere        | everywhere          |

- **Friendly on every keyboard Joel types on.** The laptop reaches Page Up,
  Home and End through Fn, and the ErgoDox reaches the symbols and Print
  through a layer, so what's done often ends on a key both have bare: a
  letter, an arrow, Tab, Return, Space or Escape. Page Up, Home, Print and
  the symbols are for what's done rarely, and anything on them has another
  way in (Super+P for Print). A chord holds its modifiers on one hand and
  presses its key with the other, and never asks one little finger for two
  keys, which is why the terminal opens a window on N rather than Return.
  On the ErgoDox, Ctrl and Alt are held on the little and ring fingers'
  home keys and Super on the index finger's, so nothing new for what's done
  often is a Ctrl chord ([The Keyboard, on Both Keyboards][pixel]).
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
Ctrl+S below, Ctrl+V beside, Ctrl+T in a tab, Escape and :q close it and ?
shows its keys; q closes nothing ([Vi's Keys][vi]).

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
the margin's glyphs in words, and Super+N takes the notices on screen as
lines, where :e Notices gathers every notice into the launcher. Either
way Return goes to who sent one, as the quickfix list's Return goes to
where a line came from, and its verbs are commands: its own actions
(:reply, Mark as read), copying what it said, and forgetting it, listed
under its words by their names and first in the : menu.

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

### Vi's Keys

Every surface of ours is a Vim buffer, and a Vim user is at home on each
with no training (Joel, 2026-11-04: "i want a vi user to feel at home in
every interface without a bunch of training"). The keys go by letter,
wherever they sit on Colemak Mod-DH.

**When vi and a key here disagree, do what Vim does, as his Neovim has
it** (Joel, 2026-11-04: "fuck, vi like ffs"). The answer is his config in
`modules/neovim` where it gives the key a meaning, and Vim's own where it
doesn't. Where a surface has no exact Vim equivalent, it takes the whole
behaviour of the nearest kind of Vim buffer, never half of one, and a key
that departs from Vim is in the Departures below with its reason. No such
call goes to Joel: it's already made. His Neovim's values are kept: a key
that could begin a pair waits his timeoutlen, 300 ms; which-key lists what
can follow after 200 ms; the mode is named in the accent when it isn't
normal; and what's copied flashes for 200 ms. Where his status line names
it in a word, ours name it in three capitals at the foot's start (Joel,
2026-11-05: "the mode should be 3 letters, in caps, and in a place where
they are more obvious and not lost with other things").

The surfaces move to these keys a piece at a time. Until a surface's piece
lands, its own paragraphs above say what it does today; where they
disagree with this section, this section is the design.

**What each surface is, in Vim:**

| Surface | In Vim, it's |
| --- | --- |
| the launcher | an oil buffer over a tree, home its root and a place a folder, found through Vim's `/` line |
| the file picker | the same buffer over real folders, its first line the name when it saves |
| the notices, held | the quickfix list, a line a card |
| the reader | a help buffer, read-only, with a cursor |
| the drawer | the preview window |
| the gallery, a PDF, a video | a buffer whose lines are pictures, pages and seconds |
| a question, the lock, the greeter, a Wi-Fi password | `inputsecret()`: a secret, with no modes |
| a reply, a new name, a save name | a prompt buffer |
| the launcher, held notices, which-key, a menu | popups, closed by :q, never by Escape |

The launcher is oil rather than netrw or Telescope. netrw's keys are
one-letter shortcuts that aren't Vim's, and Telescope's pickers are right
for how it looks but have no levels, ranges or operators. Oil is already
his (`keys.nix`, `inWindow "files"`), and its listing is a buffer, so dd,
yy, V and :3,5 mean something.

**Each mode has one purpose** (Joel, 2026-11-04: "we need the modes to be
separate and have separate purposes"):

| Mode | Its one purpose | It shows by |
| --- | --- | --- |
| normal | standing on a thing and acting on it | no caret; the cursor row's ground; relative numbers in the gutter |
| the find line, `/` | typing what to find, the rows narrowing | the accent's `/` at its start; the mono's bar caret |
| the command line, `:` | naming a command from a menu of every one | the accent's `:`; the bar caret; the menu |
| insert | writing a prompt's words | the bar caret; INS |
| visual, `v` `V` | choosing several things | the selection's ground; VIS, and "3 chosen" |
| operator-pending | an operator waiting for what it acts on | the cursor row tinted; "d · disconnect Home-5G" |

This is Vim's own split: `/` and `:` aren't insert in Vim but its command
line, and his status line names no mode for them, since the command line
says its own (`status.lua`).

**Normal.** Nothing types, and no caret shows: the cursor row's ground is
the cursor. A list shows relative numbers in a gutter at Note, muted, the
cursor row's own absolute and in text, as his `number`, `relativenumber`
and `cursorlineopt=number` draw them; Space u l switches them to absolute.
On the find and the command line the numbers are absolute, as his
numbertoggle draws them in insert, so a list is never shown without them
(Joel, 2026-11-05: "when not present it looks like shit"). The numbers
are a block starting on the card's inset, on the x where the head's mark,
the hairlines and the mode word start, as wide as the digits this place
needs now and right-aligned in it, as Vim makes room only for the largest
(Joel, 2026-11-06: "why leave constant room for three when we often have
only 1?"): one figure on home in normal. A find holds the digits of the
most rows it lists at opening and only grows within that find. The glyph
column stays where three figures and a tie would put it, on every page and
in every mode, so the spare room lies between the numbers and the glyphs,
never before the numbers, and no page or mode moves a glyph or a name
sideways (Gutter.qml, Columns.js). The head's mark, its Title and what's
typed stand on the rows' glyph and name columns. While a find narrows the
rows, the head keeps its count after what's typed: "/ Home slack · 3
found". The status line says where the cursor stands ("1 of 3"), so the
two never both read "of".

- **Moving:** j k, gg G and {n}G, H M L, Ctrl+D Ctrl+U Ctrl+F Ctrl+B, and
  { } to the next and previous group of rows, each taking a count. Only a
  motion repeats while its key is held.
- **In and out, by Vim's own keys:** Return opens the cursor row, going
  into a place or a thing as oil's Return opens a folder; - goes up to
  what holds it, onto the row it came from, with its find; Ctrl+O and
  Ctrl+I go back and forward through the jump list, every place reached by
  Return, -, :e, :b, Tab, Ctrl+] or a mark; Ctrl+] on a row that lives
  elsewhere goes to it in its own place, and Ctrl+T comes back.
- **Tab goes in** on any row that holds parts, where Return does the row's
  own act, as switching to a program: Tab reaches the program's page, its
  tabs, folders and recent files, with Go To first (Joel, 2026-11-07:
  "keep"). Ctrl+] is its twin, for Vim's sake, and no hint names it. On a
  row that holds nothing it says "Tab not here". Ctrl+I alone goes forward
  through the jump list, beside Ctrl+O; the two keys are one in a terminal
  Vim, but Neovim under kitty tells them apart, as the launcher does.
- **h and l move along a line, and nothing else.** A row has no column to
  move along, so on a list they say "l not here · ↵ open" and "h not here
  · - up"; Backspace is h, as Vim's is. No surface steps in or out on h, l
  or Backspace, since only plugins bind them so (Joel, 2026-11-05: "where
  in vi does that step into shit?").
- **Return** opens the cursor row (Return, below).
- **The lines:** / and ? start a fresh find, going down and up; n and N
  go to the next and previous match, wrapping as Vim's do with "search hit
  BOTTOM, continuing at TOP"; f and F and a letter go to the next or
  previous row starting with it, and ; and , again; i, a, I and A open
  the find line on the current find, the caret at its start, after the
  caret, at the start and at the end; : opens the command line.
- **Operators and visual:** d y c, dd yy cc, ., u Ctrl+R, v V and gv.
- **Marks,** where a place lasts: m and a letter sets one, ' and the letter
  goes back, and '' goes back to the last jump.
- **Only Return and - step a level,** on every surface (Joel, 2026-11-05:
  "enter and - should be the only level stepping keys in anything,
  right?"). Ctrl+O and Ctrl+I walk the jump list, which is history rather
  than a level; Ctrl+W moves between windows; Tab and Ctrl+] follow a row
  to its own page, as Ctrl+] follows a tag, and Ctrl+T comes back.
- **Escape** does what Vim's does: it cancels a count, a pending operator
  or a waiting key, and leaves insert, visual, the find line or the command
  line, closing which-key or the menu with them. At rest it does nothing,
  on every surface: it never closes or quits anything of ours, the
  launcher, notices, the drawer, the picker and the viewers included, and
  never goes up a level (Joel, 2026-11-05: "fuck escape quitting"), so no
  hint or key list names it. Closing is :q, ZZ or Space w alone. Escape
  still leaves a secret's field for its row, and takes mpv and swayimg out
  of full screen or a slideshow, since neither closes anything.
- **q does nothing.** There are no macros, since recording is a mode no one
  sees. The foot says "q does nothing · :q closes" until the next key, as
  every key with no meaning on a surface says "not here", never silent.

**The find line, `/`.** The launcher and the file picker open on it. It is
a list's one way to find (Joel, 2026-11-04: "it is weird that the text box
is a search, but i also have the / command"): the rows narrow as it's
typed, the letters each row matched in Neovim's Search wash, the selected
row's on rule, a shade past the selection, since raised sinks into it, and
none on the accent, since a find has no match the cursor is on (Wash.qml).
What's typed is
the mono at Figure after the place's Title, the accent's `/` at the head's
start, with the mono's bar caret at every scale.

- Letters, Backspace, Ctrl+W, Ctrl+U and Ctrl+H type and delete; ← →, Home
  and End move the caret, as on Vim's command line.
- ↓ ↑, Tab and Shift+Tab, and Ctrl+N and Ctrl+P choose the next row and the
  one before, as his completion menu does, and Ctrl+Y takes the chosen
  row's name into the line; Ctrl+R + puts the clipboard in.
- `:` as the first letter opens the command line.
- Return opens what's chosen. Escape or Ctrl+C gives normal with the rows
  as found, and Backspace on an empty line gives normal.
- Earlier finds are Space s h. A held key never crosses a mode: a held
  Backspace stops at empty.

**The command line, `:`.** From normal or visual, or as the find's first
letter, it takes the find line's place, which comes back on Escape; on the
notices it takes the acts' place, and in the reader the bottom line.

- **The menu shows as `:` is typed** (Joel, 2026-11-04: "Ex mode makes no
  sense, i don't know what I can do and there is no autocomplete"). In
  the launcher its lines are the list, in the rows' place, each drawn as
  a row, its command on the names' column under what's typed, never
  indented under the line (Joel, 2026-11-06: "they signal the wrong
  organizational concept"); the place's rows come back as it's let go,
  the cursor where it was. Over a viewer it holds ten lines at most, as
  his `pumheight`; on the notices it rises over their bottom. It lists the row's or
  the selection's acts first, Return's leading, then the surface's, then
  :q, :help, :set and :{n}. Each line reads `disc[onnect]   Disconnect It
  d`, the command in Vim's help notation, the act's name and its key, so
  the menu teaches all three. While a line is chosen the drawer names what
  it acts on: "on 3 notices: Slack, Slack, Mail".
- Typing narrows the menu; Tab, Shift+Tab, ↓ ↑, Ctrl+N and Ctrl+P choose a
  line and write it in, as his cmp does; Ctrl+Y takes it and Ctrl+E puts the
  menu away; a space after a name lists its arguments (`:sort `'s orders,
  `:e `'s places and paths, `:set `'s options).
- Return runs exactly what's typed, unless Tab or an arrow chose a line. A
  word that isn't a command says `E492: Not an editor command: wifi`,
  whatever the menu shows. Escape and Ctrl+C drop the line; :his lists
  earlier commands and @: runs the last again.
- **Names:** a Vim command keeps Vim's name and meaning (:q :x :w :e :h :d
  :y :u :red :noh :set :sort :g :v :sp :b :m :clo :sus :his); any other act
  is its row's verb in lowercase, Disconnect It's :disconnect. A name or
  short form Neovim defines with another meaning is refused, so Forget is
  :forg[et] (:fo is :fold), a call :dial, a meeting :meet, the lock
  :lockscreen, short locks (:lock is :lockmarks), and stopping an agent
  mid-turn :interrupt, Vim's word for Ctrl+C; a notice has no :read (Vim's
  reads a file in): reading a notice is d. One name an act, with no
  synonyms; a switch is an option only, so Bluetooth connects with :set
  bluetooth+=Earbuds, as Wi-Fi with :set wifi=Home-5G. Two acts on one row
  never share a name but by argument: :new and :new ., :res third, :res
  half and :res full. Short forms are
  written once beside each act, never computed, so :disc never changes
  meaning when a command is added.
- **Ranges** count the rows as listed, read off the gutter: none is the
  cursor row; visual gives '<,'>; 3: gives .,.+2; % is every row listed;
  3,5, .,+2 and 'a,'b; and :g/pat/cmd and :v/pat/cmd on every list
  (`:g/Slack/d`). A range holds the things in it, never their places, so a
  row arriving can't shift it. A command that takes no rows says `E481: No
  range allowed`.
- **Options are :set**, as Vim's are: :set wifi, :set nowifi, :set wifi!,
  :set wifi=Home-5G, :set volume=40, :set volume+=5, :set brightness=60,
  :set background=light; :set volume? shows one and :set all lists them.
- **:setl[ocal] sets the thing's own,** as Vim's sets the buffer's: the
  thing under the cursor, or the nearest that holds an act's row (Muted
  inside a program sets the program's): :setl mute, :setl volume+=5, :setl
  port=Speakers, :setl trust. Choosing a default stays global, :set
  output=Speakers. A thing without the option says "E518: Unknown option:
  port" and never falls back to the machine's, since muting all sound when
  one program was meant is the worse mistake.
- **!** is "don't ask" or "the other way": :forget!, :sort!, :w!, :q!.

**Insert** is only in a prompt: a reply, a new name (cc), the file picker's
save name. Vim's insert keys work, Ctrl+O runs one normal command, and
Return hands the words over from insert or from normal. Escape gives normal
over the words, with Vim's editing (w b e 0 $ x dw cw u), and :q drops the
prompt. **A secret,** a question's password, the lock, the greeter or a
Wi-Fi password, is `inputsecret()`: one line with no modes and no mode
word. Return answers, Ctrl+W Ctrl+U and Ctrl+H edit, and Escape and Ctrl+C
cancel it, or clear it where it can't be cancelled, at the lock and the
greeter.

**Visual** works wherever there are rows, cards, files, pictures or words
(Joel, 2026-11-04: "imaging visual mode a number of notfications and then
dd to mark them as read"). On a list a row has no letters to choose among,
so v is V, the mark says VIS and the mode word "3 chosen"; on a page v takes
characters. o goes to the other end, gv brings the last back, and : gives
:'<,'>. The same operator pressed again within 300 ms of a visual
operator is taken with it, so V j j d d acts on three. While a surface is
in visual, operator-pending or on the command line, its rows hold still,
and what arrives waits until the mode ends. The foot counts it as "2 new",
its last status fact before the mode word, which never gives way, so it
shows on a full list too; then the rows come in at the end.

**Operator-pending** follows d, y or c: a motion, a count, a text object
or the operator again. On a list a paragraph is a run of rows that ends at
a gap or where the kind or sender changes, and { } move by those runs. ip
is the run, and ap the same rows, since a list has no blank rows to add, as
ap in a file with no blank lines. The cursor row is tinted at once as
visual tints it, and the mode word's room says what it will hit. Operators,
., u and Ctrl+R act only on a fresh press, never on a held key's repeats.

**The operators** do one thing to each kind of row, named by the foot:

| Key | Launcher rows | File picker | Notices | Reader | Gallery |
| --- | --- | --- | --- | --- | --- |
| d, x in visual | the row's ending act: Close It, Disconnect It, Forget It | to the trash | done: read where the sender offers it, then forgotten | `E21: Cannot make changes` | to the trash |
| y | the row's copy: a path, an address, a name | the paths | what it said | the markdown, flashing | the picture |
| c | a new name, where the name is Joel's | rename | not here (:reply) | E21 | rename |
| . | the last change again | the same | the same | nothing | the same |
| u, Ctrl+R | every change with an inverse, while the surface is open | the same | the cards back, still read | nothing | from the trash |

What can't be undone asks once on the command line, as Vim's `confirm()`
does ("Forget Home-5G and its password? (y/n)"), taking the whole head while
it stands, the path giving way whole and back once it's answered. Where even
the head can't hold it, what stays gives way from its end at a whole word,
behind an ellipsis, never what it does, to what, or its keys. Only y answers
yes, keys
pressed in the first 300 ms are ignored, and ! skips the ask; what asks
comes from the language's `undoable`, never a list kept by hand. An act
the language marks guarded keeps its own work, so one doesn't ask, though it can't be
undone: closing a window, whose program asks before losing anything, as
Vim's :close refuses a modified buffer, and interrupting an agent, whose
session :reopen brings back. A range of more than one guarded thing asks
once, with the count, and a prompt's Return, as :reply's, is its own
asking. Anything else acts at once, and the foot says how to take it back: "3 done · u
brings them back".

**Return** opens, from the find line and from normal alike, but for one
case (Joel, 2026-11-04: "it is quite dangerous how in insert mode i can
type in a name of a wifi network and then hitting enter disconnects from
it"):

| The cursor row | Return |
| --- | --- |
| a place, or a thing with acts, a network among them | goes in, onto a first row that never changes state |
| a program, window, file, page, link or notice | opens it, or goes where it came from |
| an act that changes nothing: Copy, Show, :sort, :rotate | does it |
| an act that changes state and ends nothing: Connect, Turn On, Start, Pause | from the find line, lands on it in normal, its side saying "↵ connect"; from normal, does it |
| an act that ends or takes away: Disconnect It, Forget It, Close It, Turn Off Wi-Fi, Stop, the session's acts | never runs: Return puts its command on the command line, the menu saying what it acts on, and a fresh Return there runs it; its side says "d disconnect" or ": disconnect" |

Return acts only on a fresh press, so a held Return can't run what the
first only reached. Stepping in from the find line keeps the find line,
empty, inside: "net", Return, "home", Return. No modified Return has an
act; Vim's :sp opens below.

**The arrows move, as in a Vim buffer** (Joel, 2026-11-05: "In normal
mode, in a buffer, arrows move the cursor"). In normal and visual, Down
and Up are j and k and Left and Right are h and l, counts and all, so on
a list Left and Right mean what h and l do: nothing. On a line or in a
prompt Left and Right move the caret and Down and Up choose. No arrow
steps in, acts or makes a chord, but for the leader's Space and an arrow
into the drawer and back, and no hint or key list names one: hints name
the key that does it, "↵ open" and "- up".

**The leader** is Space, in normal and visual, as his Neovim's
(`keys.nix`, "What every layer shares"):

| Keys | Here |
| --- | --- |
| Space ?, F1, :help | this surface's keys and commands, by mode |
| Space w | close, as :q |
| Space Tab | the place, folder, page or picture used last |
| Space Space | a fresh find across everything |
| Space / | the reader's and the drawer's lines and headings |
| Space . | the launcher on its last place and find |
| Space → ← | into the drawer and back, over Ctrl+W l and h |
| Space Return | a terminal in this folder |
| Space f r, Space f Space | files opened lately, and finding a file |
| Space s h | earlier finds |
| Space u b, u l, u w | light or dark, the numbers, the reader's wrap |
| Space Q | close every surface of ours, as :qa |

His other groups have nothing to act on here, and none is faked.

**The launcher and held notices are one family** (Joel, 2026-11-05: "we
need for notifications and the laucnher to feel more similar ... i like the
status line in notifications better"). Each is the same card: a head, a
hairline, the rows, a hairline, the status line.

- **The head** is the card's one line for typing, a 48 band like the rows.
  It draws nothing over the gutter in any mode. The place's glyph stands in
  the rows' glyph cell and its Title over their names, home's too ("⌂ Home",
  "Notices", "Time › Servers"). The glyph is its path's first place's,
  however deep it goes, so "Now › For You › Slack" wears Now's mark.
  The mode's mark takes the glyph's cell while the mode lasts, centred by
  its ink: the accent's `/` on the find,
  `:` on the command line (Joel, 2026-11-06: "we have an extra space in
  front of the launcher heading when in command mode that we don't have in
  find"). So the Title and what's typed start on one x in every mode. What's
  typed follows the Title, and a find's count sits beside it, as Vim's
  search count sits beside the search. The head holds one size on one
  baseline, the Title told from Figure by the serif's weight and strong
  (Joel, 2026-11-06: "it is like a bumpy hill and a bit too chaotic"): the
  launcher's find has no bare ask, since the mark and the caret say to type,
  and a prompt keeps its particular one ("open which file?").
- **The rows** are the launcher's: the gutter, the glyph, the name and its
  words, the side at the far end, each a band meeting the next, the cursor's
  ground filling its band. A row that holds more draws its mark muted, never
  in a rule's or the ground's colour, so it reads on the cursor, but for a
  warning's or an alert's, which it keeps. What a row
  is, holds or does follows its name, in Words ("Shortcuts" then "Tabs,
  Tab Numbers"; "brightness ⟨percent⟩" then "Sets the screen's
  brightness"). A figure
  or a state, a count, a size, a time or a chord, sits at the far end in
  Note, ending on the inset, a count before its chord across a dot. The
  cursor row alone says what nothing else draws, a toggle's state, a
  piece's context ("full in 40 min") or where an answer came from: at the
  far end in Note, before its side across a dot, cut from its end and
  never narrowing the name, and whole in the drawer. Its note follows
  its name, after what it holds across a dot, in Words and muted, never
  at the right (Wi-Fi's "Lantern" then "a password", "connected" at
  its right). It has the room the rest leave and gives way first, by whole
  items from its end, so the right never gives way to it (VKCF3Z,
  2026-11-09). A held notice's card
  opens under its line, typed as the drawer's plate is.
- **A path** gives way from its middle: a middle step shortens to its last
  folder ("…/more"), then to "…", while the ask and the last step stay
  whole at every width. What's typed after it scrolls inside its own room
  past that point, a muted "…" at its start saying some went off.
- **The command line** takes the head, the find line's place, as Vim's
  `:` and `/` are one line (Joel, 2026-11-06: "it is basically / by
  default"): its `:` where the find's `/` stands, what's typed on the
  rows' names, over the line it completes. Its completions take the rows'
  place as rows, never indented, in held notices too.
- **One set of pieces** draws them, and the notice that pops up: Card.qml
  is the frame, its edge, radius and fill and the inset on every side, and
  Columns.js finds the gutter, the mark column and the text column from
  what stands in them (Joel, 2026-11-06: "they need to share more code and
  UI/UX design to stop them from constantly drifting"). A glyph in the mark
  column is followed by a mark (8), a picture, as music's 48 cover, by an
  apart (12). The pop-up has no gutter, as Vim's `nonumber`, and is as wide
  as forty letters of Words beside its mark column, so a notice breaks the
  same popped as held. `every-card-is-the-one-card` fails on a surface that
  draws its own frame, reads the inset or sets its own x.
- **The status line** is one component both read (StatusLine.qml), below.

**The foot, the mode mark, the mode word and the key list.** The foot is
the status line, and it starts with the mode's mark, three capitals at Label, NOR, FND, CMD, INS
or VIS: muted in normal and in the accent otherwise, first where the eye
starts the line, as his status line starts with its mode, and every mark
the same width, so nothing beside it moves. Pending, a prompt's normal and
the key list are NOR. It then gives its surface's own facts, the glance
where the launcher has it and the position ("1 of 4") where held notices
do, the mode word's room, then the cursor row's keys
in this mode by letter and lowercase verb, each key in text and its word
muted, joined by a muted dot a mark each side, "d disconnect · Space ?
keys · ↵ go to": Return
at the right end, as Joel chose, and Space ? just before it, so the way to
the keys is always on screen. Short of room, once the facts read their
least, the keys drawn furthest from Return give way first, never Return
nor Space ?: in visual the command line, then d, then y. A key that comes
and goes with the row is drawn furthest from Return, so nothing else moves
when it does, and which gives way follows need rather than place: on the
find the completion first, then next time, and never Tab in, which is
named wherever Tab goes in and nowhere it doesn't, Ctrl+] never; in
normal Tab in first, then d and y, which Space ? lists, so - up and :
outlast them, - being the one way back out of a step such as a filter
(VKCF3Z, 2026-11-08). On held
notices d stays too, and the position gives way whole before any of them.
Return's place holds the widest verb among the rows it may name: on the
find those the text typed now lists, measured again at each letter or
erasure and growing only while the cursor moves among them, so a passing
row leaves no gap; in normal and visual the place's own rows, ending lines
and all. Arrows, n and nothing pressed move no key; a letter may, since it
changes the rows.
It's the only room held: a hint keeps none for a key it no longer draws.
On the command line the foot names the line's own keys: "Tab choose" until
a line is chosen or typed, then what Return runs. Its hints come from the
table that handles the keys, filtered by mode and row, so none names a key
that does nothing, and a check presses every hint drawn. The mode word's
room is as wide as its widest
word in its own font, a count in as many figures as the place has rows,
set before the hints, which give way around it,
so it's never cut (Joel, 2026-11-04: "the mode text is truncated so I
only see ...rt instead of insert"). Room is held only for what this mode
draws, never for keys another mode would show. While an operator waits no hint applies, so the
mode word takes the hints' room too and names its target whole,
"d · close Notes for Tuesday", cut only at a whole word with an ellipsis
when even that won't hold it. A landing is normal, so its foot keeps
normal's hints, only its Return in the accent. Space ?, F1 and :help open
the key list, grouped by mode under its purpose ("Normal: moving and
acting", "The find: the /
line", "Commands: the : line", "Visual: choosing several", "Insert: writing
words"), each line its keys at Note in text and what they do at Note,
muted: a saying of about 17 letters at most and a chord of 14, so a line
fits whole on the laptop's plate.

**On each surface:**

- **The launcher** opens on the find line, the first row chosen, so the
  fast path is Super+Space, letters and Return, as before. :q, ZZ and
  Space w close it, and Escape at rest does nothing. Its commands are its
  rows' verbs, :e with
  places as paths (`:e Machine/Power`), :b for a window, :sp, :m to a
  workspace, :sort, :set and the session's (:lockscreen, :sus, :logout,
  :reboot,
  :poweroff), each of the session's asking once.
- **The file picker** opens on the find line. Return takes the cursor row
  or a V selection; :arga, or :'<,'>arga, gathers files across folders,
  the mode word saying "2 gathered" and :args listing them, and Return then
  answers with them. A save dialog's first line is the name, a prompt in
  insert with the caret before the extension; Return there saves, a taken
  name asks once, and :w, :w!, :x and ZZ save from anywhere. :sort name
  (A to Z), :sort changed (newest first) and :sort size (largest first),
  with :sort! the other way, are kept across asks; :e, :cd, :set hidden
  and :mkdir go with them, ]b [b to the folders beside, gx to a file's own
  program. :q cancels the dialog, and Escape never does.
- **The notices,** held on Super+N, are the quickfix list: a line a card,
  the sender's glyph, who, the first words and the time, the cursor card
  whole under its line, the newest first. / finds among them; its acts are
  commands, listed under its words and first in the : menu; Return goes to
  whoever sent it. d is done, its
  read sent at once, and the foot says "d read" where the sender offers a
  read and "d forget" where it doesn't; :forg forgets a card unread, :reply
  writes one, and :g/Slack/d takes all of one sender's. `:e Notices` opens
  their history. :q, ZZ, Space w and Super+N let them go, and Escape at
  rest does nothing. It's a launcher page in shape (2026-11-06, so the two
  can't drift apart): its numbers on the inset, as wide as it numbers, the
  sender's mark in the mark column and every line on the text column, a
  band a line, meeting the next with no gap, the cursor's ground filling
  its band from one inside edge of the frame to the other. The cursor's
  card makes its band taller, keeping as much room under its last line as
  over its first, and signs it with when at the far end. Its head is a
  band of the same height, its mark in the mark column, where the find's
  / and the command line's : take its cell while they last, and the Title
  on the text column. Its : line is the launcher's: it takes the head, what's
  typed on the text column, and its completions are the list's rows in the
  lines' place, each a launcher row, never a menu indented under the line.
- **The reader** has a cursor, as a help buffer does: the cursor line marked
  by a thin accent rule in the left margin, as his `cursorlineopt=number`
  lights only the number, and a block only in visual, which starts on the
  line's first word, never on a list's number or bullet: those are the
  margin's, as Vim's number column is, and the selection's ground leaves
  them bare. j and k move it by
  the line as shown, the view following at his `scrolloff=4`; Ctrl+E and
  Ctrl+Y scroll the view alone. /, n N, ]] [[, { }, gO, marks, jumps and
  :{n} put the cursor where they land, so / and then yip copies the
  paragraph found. Escape at rest and q do nothing.
- **The drawer** is the preview window: Ctrl+W l goes in and the reader's
  keys work there, and Ctrl+W h or :q comes back, Ctrl+W w either way, as
  Vim goes between windows, with his Neovim's Space and an arrow over
  them (Joel, 2026-11-05: "leader stuff can overlay default stuff");
  inside, the arrows move as j k h l do.
- **The viewers** close on :q and ZZ, swayimg and zathura on Space w too
  (mpv's Space pauses at once and leads nothing), never on Escape or q, and
  their arrows are their letters, each with one meaning. swayimg's and mpv's
  : opens the launcher's command line aimed at the picture or the video
  (`:d`, `:y`, `:rotate`, `:{n}`, `:q`); zathura keeps its own, which is
  Vim's already.

**His Neovim is held to the same bar:** q closes none of its windows, and :q
closes its floats. Its Escape stays as he set it, since his own editor
changes only through his config (Joel, 2026-11-05).

**The Departures.** Each key here that doesn't do what Vim's does, and why:

1. A list's / and ? narrow the rows as they're typed, where Vim's only
   move to a match; n and N then step through what's left and wrap, as
   Vim's do, never searching afresh.
2. f and F go by a row's first letter on a list, a row being one line;
   on a page they go by a letter in the line, as Vim's.
3. % means the rows listed, fewer than all while a find narrows them.
4. The launcher and the picker open on the find line, as Telescope opens on
   its prompt, which keeps the fast path.
5. Escape and Ctrl+C on the find line keep what was found, since on a list
   the find is what the list is.
6. `:` as a find's first letter opens the command line, since a find never
   starts with one.
7. i, a, I and A reopen the find line, since a list has no text of its own.
8. A row whose act ends something puts its command on the command line
   rather than doing it, since a list has rows that are acts where Vim's
   buffers have only text.
9. v is V on a list, since a row has no letters to choose among.
10. q records nothing: recording is a mode no one sees.
11. The same operator pressed again just after a visual operator is taken
    with it: Joel's "visual ... then dd".
12. A pending operator shows at once, though his showcmd is off, since an
    armed d is the riskiest state no one sees.
13. Return from the find line opens what opens and goes into what holds
    more, where Vim's lands on the match, to keep the fast path; on an act
    that changes state it lands, as Vim's does.
14. A key with no meaning says so in the foot, where Vim beeps.
15. The picker's sort is kept across asks, a folder's order being a
    preference.
16. mpv's Space pauses at once and leads nothing, as a player's does.
17. :setl takes a range, :%setl mute setting each thing's own, where Vim's
    :setlocal takes none, since a list holds many things where a window
    shows one buffer.
18. Ctrl+] on the find goes into the chosen row's parts, as in normal,
    where Vim's command line would expand an abbreviation; there are none
    to expand.
19. Tab goes into a row that holds parts, in normal and on the find, and
    Ctrl+I alone goes forward through the jump list, where Vim's Tab is its
    Ctrl+I: going in is done often, and Ctrl+] asks both hands and two
    holds on the ErgoDox (Joel, 2026-11-07).

**Where Vim's keys live.** Each act is named once in
`modules/keys/language.toml`, with its keys in Neovim's notation, the
modes it's pressed in (normal, find, command, insert, visual), the kinds of
surface it belongs to (field, list, page, player, prompt, secret), its
command name and fixed short form, and whether it changes state, ends or
takes something away, can be undone and takes a range, beside how long a waiting key waits, the
which-key list's delay and the yank flash's, which Neovim reads from there
too. Each surface reads its keys from it by the acts' names, names its
kinds in `jitsusama.surfaces`, and is proved by a VM test pressing every
key it adds. `keys-speak-the-language` fails on a surface that leaves out
an act its kinds hold, gives a key of the language to another act or binds
an act by a key not its own, and on a change to Neovim's keys the language
doesn't agree with. As the language takes in the commands it fails too on a
short form that isn't unique or a name Neovim defines otherwise, on an act
that changes state reachable by Return from the find line, on one that
ends reachable by Return at all, on h, l or Backspace going in or out, and
on a hint
whose key isn't in its mode's table. Until a surface
reads from it, its keys sit in one table of key and act, never scattered
through its code.

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
  `jitsusama.quickshell.margin.pieces`.
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
opacity = "0.9"

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
  contrast as the machine is evaluated: 4.5:1 for every pair a surface draws
  text in. The text roles are measured by APCA as well, which weighs how light
  the background is: on the background, sunken and deepest, text and strong
  need Lc 75, as body text does, and muted Lc 60. Each mark's hue needs Lc 45,
  as a fine mark does, on those grounds and on the selection as a floating
  surface draws it. A theme that fails doesn't build.
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
| The launcher's glance and the margin mark | Quickshell               | both           | the status line after its mark: the time, the date, the battery with its time and watts, the network, the volume, the day's AI spend, pi, each after its glyph | the battery running low, offline, a network wanting a sign-in, muted | step 2's doors | step 2 | volume, network, battery, date; the time stays |
| The launcher and palette                  | Quickshell               | monospace      | what's typed; the selected row: its glyph, its name, its kind or its chord; Return's verb at the foot's right | the foot's note: "sl is enough", the chord that opens a thing next time, where an answer came from | a thing's verbs, a collection's members and an answer's parts, a step in by Return | the accent frame, a command line over a list split by a hairline, a quarter down | past eight rows the list scrolls, keeping the height it opened at; a note takes the glance's place in the foot |
| Notifications                             | Quickshell               | both           |        |        |           |         |          |
| Menus and overlays                        | Quickshell               | both           |        |        |           |         |          |
| The lock screen, polkit and PIN prompts   | Quickshell               | both           |        |        |           |         |          |
| GTK and Qt apps, file and print dialogs   | gtk, qt                  | proportional   |        |        |           |         |          |
| The browser                               | chrome                   | proportional   |        |        |           |         |          |
| Images and drawings                       | swayimg, gimp, inkscape, Quickshell | proportional | nothing | nothing | nothing | the launcher's rows, in Capture after what it captures: edit the newest screenshot in GIMP; look through the screenshots in swayimg's gallery; Programs › swayimg leading with that gallery while there are screenshots, else its Recent's newest folder's, then a gallery of each folder its Recent's pictures are in, newest first, each saying how many pictures and where and opening on Recent's newest there, each found from the top by its name, and its drawer saying when the newest screenshot was taken and how many there are; in swayimg, e edits, y copies, Delete trashes, ZZ closes; in Programs › Inkscape, New Drawing, New Drawing From the Copied Picture while an SVG or a picture is copied, and Recent, its drawings lately, each saying when and its drawer its size | swayimg's name and place go two seconds after each picture opens |
| Documents                                 | zathura                  | proportional   | nothing | nothing | nothing | zathura's own keys | its bar and command line show only while they're asked for |
| Music                                     | spotify, Quickshell      | both           | nothing | a note in the margin while any player plays; for a moment, the track as a notice | What's on: the track, artist, album and player; pause, next, previous, the player's window, and pausing a second player | Play or pause the music, Play the next track, Play the previous track, on the media keys | the mark carries no words; a title wraps to two lines, then gives way |
| Pairing                                   | tuple, Quickshell, niri  | both           | nothing | a phone in the margin while in a call, and wanting Joel while one rings; for a moment, who joined or left, and that he's the only one left; a ring, and a missed call kept | What's on: how long, and who with or who's sharing; leave, share the screen, mute him in Tuple, copy the room's link; while ringing, show the call | the launcher's rows: show a ringing call, first at home; open the room and copy its link; join a copied call's link, first at home once copied; leave, share, mute; Tuple's Recent in Programs › Tuple, the last nine calls ended or missed, newest first, each by who, how long or missed, and when, a missed one's mark in the alert, its drawer saying each thing once (who rang whom where tuple-call saw it, with for two or more, then their addresses), Return going in and Call Them Again first inside; a tuple: link from the browser joins its room | the mark carries no words; What's on says how long and who with |
| AI spend                                  | spend-watch, Quickshell  | both           | the launcher's foot: today's spend against usual ("$214 today · usual") | the margin's glyph while a finding wants a look: a cap near, over or refused, an override ending, a cap lowered or lapsed, a credential nobody knew spending, a source signed out, or the watch stopped or on hold; a notice once for each, at most three a week beside a cap's, and one line each morning | What's on: how it stands, the day's likely end, the week, and how the figures were got; go to the session, ask for more, sign in, it's expected, read the week's report, show where it went | the launcher's rows: "spend", "cap" and a report by its words ("spend last week by quest"), Return copying it as markdown and Ctrl+] going into its rows; Super+U opens the collection: what wants a look first, then today, each cap longer than a day and earlier, a day's cap behind today and the next seven days behind the week's cap, each a figure of a few words with the rest in the footer and its parts a step in (a period's cuts and their rows, a cap's changes and its sign-in), then what was found | the row gives way before the network; the mark carries no words; while sharing, every amount gives way and the words stay; a source that can't be read says nothing for its first fifteen minutes, as a machine waking finds its network, and no more than a note unless only a sign-in fixes it; a history a newer watch keeps, as when an older system boots again, is left as it is, the watch on hold and saying so until that one runs; a cap's allowance that changes for one reading and comes back says nothing, a change being told once it has stood ten minutes unless the cap warns or refuses at it, and a fall stops asking once the cap is back |
| The home network                          | eero, Quickshell         | both           | nothing | a notice once for each change while the daemon watches: the internet out and back, a node offline and back online, critical while out; a device new to the home network; a firmware update waiting and the nodes updated; an event eero lists that nothing else says; the access point in the margin, warning, and a line in Now while something ails (the internet down since when, a node offline) | Network › Home Network: the internet as its head (how it stands, its internet provider, the last speed test and when, conferencing and gaming, outages, the nodes by state, the devices eero counts), each node after the one its backhaul hangs from with its devices under it, strongest first, then the profiles; each a thing whose plate holds every figure eero's contract places there, sparked where a history is kept, and whose step in lists them as rows Return copies | Run a Speed Test (:speedtest, Return twice) and Optimize for Conferencing and Gaming (:set sqm), both started through niri as the eero program; every node, device and profile found by its name, a device by its hardware address, hostname or address too | a row's note gives way at a whole word from its end; a plate lays its facts beside their names while all fit, else each under its name; a figure eero didn't give is left out, never guessed |
| The work network                          | warp, Quickshell         | both           | nothing | the tunnel in the margin, warning, and a line in Now while it ails (down since when, degraded, connecting for over 30 s, WARP's service not answering), gone within seconds of healing; a disconnect Joel chose says nothing | Network › Work Network, after the Home Network: how WARP's link stands, its health and round trips, Through: WARP last | Disconnect WARP and Connect WARP (:disconnect, :connect); found by work, warp, vpn and tunnel | a switch the organisation holds is left as it is, saying who holds it |
| VMs                                       | libvirt, Quickshell      | both           | nothing | nothing | Programs › VMs, listed only while a VM exists on the session's connection or the system's: each machine by name, "system" after one on the system's, its state at the right in the launcher's words, never libvirt's (running, waiting, paused, powering off, off, crashed, asleep, unknown), the program's row counting them by state; its mark a cube's outline, never a screen's monitor; its plate, its eyebrow where then what, "on this machine · VM" or "on the system · VM", every figure dominfo gives (processor time and memory in use while it runs, processors, most memory, whether it starts with the machine, is kept when off, a saved state only where it has one, its security model and label only where it has one, never its DOI, messages, then ID and UUID), each size in decimal units from its bytes ("4.29 GB"), each a row a step in that Return copies, and in its words each figure dominfo left out, named with why ("Known only while it runs: Processor Time, Memory in Use and ID.") | by its state, Open the Console (:open, in virt-manager), Start It (:boot), Resume It (:resume, u pausing it again), Power It Off (:off, Return twice) and Force It Off (:forceoff, d, asking once); one on the system's says it asks for his password and is done only by his own Return | a figure dominfo leaves out is named as left out, never guessed; a state reads … until the launcher's opening has read it |
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

- **A shutdown says what it waits on plainly, in the serif, and names the
  generation running** (2026-11-07, from Joel's photographs of a restart).
  The restart's screen read "waiting on User Manager for UID 1000" in a
  sans, under a foot naming the generation booted days before rather than
  the one running, which the restart boots. A slow job is now said by its
  kind in our words: "waiting on your session to close" for a person's
  session, "waiting on something to stop" for anything else, and to start
  at a boot; the unit's name goes to the journal, for whoever looks. A
  shutdown's splash starts from the system, whose fonts lacked the serif's
  cut, so the cut joins them. The foot at a shutdown names the newest
  generation that is the running system; a boot keeps the entry it chose.

- **kitty draws every letter at its own size** (2026-11-04, from the m
  of "limit" drawn visibly smaller in the terminal at 1.5). Texture
  healing gives a wide letter beside narrow ones, such as that m between
  two i's, a glyph reaching into its neighbours' room. kitty draws each
  glyph inside its own cell and shrinks one that's wider to fit, so the
  healed m came out small (githubnext/monaspace#222). kitty alone draws
  the face without its contextual alternates, which are texture healing,
  in all four styles; the ligature sets stay, as does texture healing
  everywhere else monospace is drawn. A kitty that drew a healed glyph
  whole, over its narrow neighbours, would bring it back.

- **A notice reads in three groups, at the launcher's size** (2026-11-03,
  look 3, from Joel's "the differing font sizes, and too much information
  on lines really felt gross and unrefined" of 24 October and "i'm also
  seeing some whitespace issues with the notifications, feeling quite
  cluttered" of 3 November). Who or what it's about, strong in monospace;
  a unit below, what it says, in the serif upright; a step below, the signature,
  muted in monospace: the name Joel knows it by (AI Spend, never a
  process's name) and, on the right, when. The mark of what sent it stands
  in the card's mark column beside who, centred on its capitals, as a
  launcher row's mark does (2026-11-06, so the two can't drift apart). All
  three are at the large step, as the launcher is. The
  frame stands an inset and a step from the ink on every side, to the
  capital's top, the first letter, the last and the signature's baseline,
  as the launcher's card does. The card is as wide as what it says needs to
  hold about 40 of the serif's letters on a line at that size, beside its
  mark and inside its room, worked out from the card's columns. The
  line of offers and the held card's key hints went, as the clutter he
  named: a click does what the card is about, and the keys reach every
  act. Held, the card shows its acts under a rule at once, each by its
  command name, so it shows what : offers without a hint. The track
  playing, beside it in the same column, reads the same way.

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
  twice the gap in Now, they crossed the windows' borders wherever
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
  each asks, with their words in the launcher's Now, a surface Joel
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
  says the keys where it said when. Since 3 November it says none: held,
  its acts show at once, each with its key. Since 5 November those keys
  are commands, since no Vim key steps into a line (Joel: "where in vi
  does that step into shit?").

- **The splash says what it waits on** (2026-10-21, judged against the
  taste without a round). A boot or shutdown that stalled said starting or
  shutting down and nothing more, and a restart's screen froze when its last
  stage began. Once a job has run three seconds, the line under those words
  names it, in the body face and in the description systemd gives it:
  waiting on Docker Application Container Engine (since 7 November, in our
  words rather than systemd's, above). One is named at a time,
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

- **A drawing begun from the launcher** (2026-11-09). Inkscape's page in
  Programs holds New Drawing, a fresh copy of the account's template opened
  in Inkscape, and New Drawing From the Copied Picture while the clipboard
  holds an SVG, or a picture whose size its history keeps, which it's made
  the size of. Each is a file in ~/Pictures/Drawings, beside the camera's
  photos and the screenshots, rather than in the runtime folder as GIMP's
  copied picture is: a drawing is saved where it was opened, and one in the
  runtime folder would go at the next boot. Its Recent is the drawings
  Inkscape opened lately, from the list GTK keeps for it, each saying when
  and its drawer its size as its SVG says it, in its own units.

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
  and a notice signed in its footer. Look 3's first gap (2026-11-03) gave
  a row's words back to Mona Sans, at the name's size and on its baseline
  beside the mono name, so the name is found first without a second size.
  On 4 November those words went to the serif upright, as every sans of
  ours did, when Joel said a sans felt out of place in the launcher.

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
- **Text is linen and muted is reed** (2026-10-31). Measured by APCA on the
  background, parchment, Omarchy's foreground, is Lc 67.6, under the 75 body
  text needs, and sage Lc 34.3, under the 60 any text needs; and sage's hue
  sat 46° from parchment's, so rank read as a change of hue. Text is now
  linen (Lc 78.9), Omarchy's light foreground, and muted is reed (Lc 60.8),
  linen's own hue and chroma made darker, so rank is lightness alone. Sage
  stays for the terminal's bright black. Judged by screenshot in Osaka Jade.
  Flexoki Light passes as it was, its faintest muted pewter on flax at Lc 63.1,
  so nothing in it changed.

[surfaces]: #surfaces
[decisions]: #decisions
[taste]: ../modules/theme/SKILL.md
[pixel]: #every-pixel-accounted-for
[themes]: #themes
[space]: #space
[budgets]: #latency-budgets
[colour]: #colour
[vi]: #vis-keys
[keyboard]: #the-keyboard
