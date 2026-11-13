# Commands

A command here is a program of ours typed into the shell, by Joel or by an
agent: `night-shift`, `nix-offload`, `update-to-main`, `launcher`, `mycelium`
and every other one core.nix, a machine repository or the harness builds.
Each has two readers. Joel reads it in kitty, so it's a surface like any
other and is held to [design.md][design] and [the aesthetic skill][taste];
an agent or a script reads it through a pipe, so it's a contract, and a
contract doesn't move. This page says the shape every command takes for
both, so learning one teaches them all, and [the terminal kit][kit] gives
each Rust command that shape for free.

It's built from the research in the quest that wrote it: [clig.dev][clig],
Heroku's style guide, no-color.org and force-color.org, gh, cargo, ripgrep,
jj, kitty and libxo. Where this page and one of them differ, this page
says why.

## The Convention

Every command keeps these points. The terminal kit's tests hold them for
every command built on it, and the `commands-agree` check holds every
command to the kit, as [When a Command Doesn't Agree Yet](#when-a-command-doesnt-agree-yet)
says:

- **Answers:** stdout carries the answer and nothing else: the records, the
  value asked for, or the JSON. Everything said to a person on the way,
  progress, warnings, what changed, an error, goes to stderr.
- **Formats:** `--format text|json|jsonl` chooses the answer's form, and
  `--json` is `--format json`. `text` is the default, into a terminal or a
  pipe alike; a pipe changes how it's drawn, never what it is.
- **Inks:** colour and marks follow `--color`, then `NO_COLOR`, then
  `FORCE_COLOR`, then whether the stream is a terminal, each stream on its
  own.
- **Helps:** `-h` and `--help` print the help to stdout and exit 0,
  whatever else is on the line, and never start anything; `help <command>`
  does the same for a subcommand. `--version` prints one line. A command
  run for a person declares its help, which `--help --format json` prints
  for the check to read.
- **Exits:** with a code from [the table](#exit-codes), and only those.
- **Fails in three parts:** what failed, why, and what to do, as
  [Errors](#errors) sets them.
- **Waits visibly:** anything that takes longer than a moment says so on
  stderr, and draws nothing in motion off a terminal.
- **Completes:** a command with subcommands or named values gives zsh its
  completion, installed by its module.

## Two Readers

clig puts people first and machines second, and Heroku's guide says the
same. Here neither waits for the other, because each gets its own form:

- **The text form** is free to get better. It's judged by the aesthetic,
  changed whenever a change is clearly better, and never parsed by
  anything of ours. A script that needs a fact asks for JSON.
- **The machine forms** are schemas. They change only by adding: a field
  is never renamed, retyped or removed without a new version.

The text form still reads well through `grep`, since Joel pipes it:
every line that holds a record says which record it is, as a table's row
names its thing, so no line depends on a heading above it (Heroku's
`heroku regions`). Into a pipe it draws no colour, no motion and cuts
nothing short, since a pipe has no width to fit.

An agent reads the text form too when it runs a command for a person,
so it's written to be read by one: plain words, no escape codes in a
pipe, nothing only its author understands. An agent that keeps or acts
on a fact asks for `--json`.

## Formats

| Format  | What stdout holds                                           | For                       |
| ------- | ----------------------------------------------------------- | ------------------------- |
| `text`  | the answer laid out for a person, as [How It Reads][reads] sets it | Joel, and an agent reading for him |
| `json`  | one JSON object, the whole answer                           | a script, an agent        |
| `jsonl` | one JSON object a line, as each part is known               | a stream, a long run      |

- **One object, never a bare array**, so a field can be added beside a
  list (`{"schema": ..., "items": [...]}`). Pretty-printed into a terminal,
  one line into a pipe, as gh's `--jq` does.
- **Every object names its schema**: `"schema": "<command>.<answer>/<n>"`,
  such as `"launcher.show/1"`. The number rises only when a field
  changes meaning or goes; adding one keeps it.
- **Lines are typed**, as ripgrep's and cargo's are: each `jsonl` line is
  `{"type": ..., ...}` and the last is `{"type": "end", ...}`, carrying
  the outcome and the exit code, so a reader knows the stream finished
  rather than broke. A command with nothing to stream answers `jsonl` with
  its `json` object as the one line before `end`.
- **Facts, never words.** Keys are snake_case and say their unit
  (`elapsed_ms`, `size_bytes`); times are RFC 3339 with their offset;
  states are lowercase words from the [glossary][glossary]; a field the
  command can't know is `null`, never missing, so every record of a kind
  has one shape. Nothing in JSON is a sentence written for a person, but
  an error's three parts.
- **Bytes stay bytes.** A path or a value that isn't UTF-8 goes under
  `bytes`, base64, beside `text`, as ripgrep keeps them, rather than being
  lost to a replacement character.

A command may offer a form of its own beside these, named for what it
is, as spend-watch offers `markdown` and `csv`; it never renames one of
the three. `--format` applies to the answer, and to help for the check;
version and completion are always text.

## Streams

stdout is the answer, stderr is the conversation (clig, and Heroku's
"actions are displayed on stderr"). Under `json` and `jsonl`, stdout holds
JSON alone from its first byte to its last, and an error is written to
stderr as JSON as well, so an agent learns why without reading words:

```json
{
  "schema": "error/1",
  "kind": "try-again",
  "exit": 75,
  "what": "couldn't reach the builder",
  "why": "connection refused after 10 s",
  "try": "nix-offload there --here-when-idle . checks.x86_64-linux.example-nixos"
}
```

stderr isn't a log. It carries no levels (`ERROR`, `WARN`), no times and
no module paths; `--verbose` adds detail and a command's journal or log
file keeps the rest. Where stderr is the journal's own stream, as a
unit's is (`JOURNAL_STREAM` names it), the kit opens each line of a
failure with `<3>`, sd-daemon's mark for an error, so `journalctl -p err`
finds it; journald takes the mark off.

## Colour and Marks

Ink is decided for each stream on its own, first match winning:

1. `--color=never|always|auto`, the flag, as clig and jj give it.
2. `NO_COLOR` set and not empty: none ([no-color.org][nocolor]).
3. `FORCE_COLOR` set and not empty: ink, even into a pipe
   ([force-color.org][force]). `NO_COLOR` wins over it, as bixense's merge
   of the two asks, since being asked for none is the safer reading.
4. The stream is a terminal and `TERM` isn't `dumb`: ink. Otherwise none.

Where there's ink, it's the theme's, read at each run from
`~/.config/jitsusama/theme.json`, so a theme switch reaches every command
without a rebuild:

- **Roles, never colours.** A command names a role (text, strong, muted,
  alert, warning, success, and the glyph kinds action, program, machine,
  person and voice), never a colour or an ANSI number, as jj names labels.
  It's drawn in 24-bit colour where `COLORTERM` says the terminal draws
  it, kitty among them, and otherwise in the one of the terminal's sixteen
  colours holding the role's pigment or near it, as the console and the
  guide read a theme ([nearest.nix][nearest]).
- **Colour goes on marks, never on text** ([Colour][colour]). Text keeps
  to text, strong and muted; a mark wears its kind's hue or its state's:
  done in success, failed in alert, caution in warning. The accent is
  where the keys go, so a command uses it only on its own question's
  prompt, while it waits for an answer.
- **Nothing told by colour alone.** Every state colour comes with its
  mark, so the form reads the same with `NO_COLOR`.
- **Marks by meaning.** A mark is looked up in the theme's table
  (`jitsusama.theme.marks`) by what it means, never typed. On the console,
  `TERM=linux`, and where the theme can't be read, each mark is its
  stand-in, which every font draws.
- **A mark and its space.** A mark is always followed by one space, and
  takes two cells: kitty draws a Nerd Font mark whole only when a space
  follows it, and a list's text then starts on one column whichever marks
  it wears. Where one row of a list wears a mark, every row holds its
  cells, blank where it has none.
- **Links.** A URL or a path shown into a terminal with ink is a kitty
  hyperlink (OSC 8) on the words that name it, so it opens on a click
  without being printed twice.

## How It Reads

A terminal has one size and one face, so the [levels][levels] move onto
the cues a cell has: weight, the colour roles, case, italic, and space
counted in cells and lines. The launcher in a terminal (`launcher show`)
was the first command set this way, and this generalises it.

| Level  | In a terminal                     | What goes at it                                        |
| ------ | --------------------------------- | ------------------------------------------------------ |
| Title  | bold, strong                      | the head: the thing the answer is about, once          |
| Name   | text                              | a row's name, a command or a flag in help              |
| Figure | text, right-aligned in its column | a fact's value, a count, a time                        |
| Words  | muted, after its name             | what a row holds, what a flag does                     |
| Note   | muted                             | a row's side, units, a hint, a time beside a line      |
| Label  | muted capitals                    | a group's heading: USAGE, EXAMPLES, OPTIONS            |
| Voice  | italic, muted                     | what the command says of itself: nothing found, a tip  |

- **Only the title is bold.** Weight is a level's alone, so bold never
  marks emphasis or a selection, and there's one title on an answer at
  most.
- **One strong.** Strong is the one thing the eye should land on: the
  title, or the thing a command just changed. A list of records has none
  but its title.
- **Fewer cues than a page.** Name and Words differ by colour alone, since
  the grid has no second face, so position does the rest: words always
  follow their name, a column apart.

### Space

Space groups before anything else does ([the ladder][ladder]), counted in
cells across and lines down:

| Between                                            | Room                                    |
| -------------------------------------------------- | --------------------------------------- |
| a mark and its text                                | one cell                                |
| a figure and its unit, a name and its words        | one cell                                |
| items on one line                                  | " · ", a dot with a cell on each side   |
| columns of a table, a row's text and its side      | two cells at least                      |
| lines of one thing                                 | none: the next line                     |
| groups                                             | one empty line                          |

- **The left edge is column 0**, the terminal's own inset given by kitty.
  A row's detail sits under its text, two cells in, past its mark's cells.
- **No rules and no boxes.** A table is aligned by space, never drawn
  round (design.md turned down boxes in boxes); a head is a Label over
  its column, not underlined. Groups part by an empty line, never a line
  of dashes.
- **The side is flush right**, in one column for the whole list, as the
  launcher's rows are: aligned to the widest row, never to the terminal's
  edge, so the eye doesn't travel across empty room.
- **Prose wraps at 80.** Help and words are set at a reading measure of
  80 cells, however wide the terminal is.

### Width and What Gives Way

Into a terminal, an answer fits its width, and no line of it runs past
the edge for the terminal to fold. These choices come from setting
27 commands of 15 well-regarded tools at 60, 80, 100 and 140 columns
(git, ripgrep, gh, jj, cargo, uv, eza, ls, fd, systemd, glow, gum, dust,
duf and bat) and from what they and their guides say:

- **The width is `COLUMNS`, then the terminal's, then 80**, rather than
  the terminal's first as `ls` reads it, because neither zsh nor bash
  exports `COLUMNS`: it's in a command's environment only when someone
  asked for that width, which is how [git][git-width] and
  [man][man-width] read it. It's read once, as the command starts, as git
  does, rather than again on each resize, since an answer already printed
  can't be set again; only a program that redraws the screen, like
  `less`, follows a resize.
- **Into a pipe nothing gives way.** Each record stays whole on its line
  at any length, rather than fitting 80 as git's `--stat` does, because
  whoever reads a pipe wants a record a line ([clig][clig]), as gh's
  tables turn tab-separated and `ls` lists one name a line
  ([gh][gh-table], [ls][ls-width]). The width only lays out the text
  form; the JSON forms never change with it.
- **Prose is set at a measure** of at most 80 cells, however wide the
  terminal, rather than across its whole width as jj, uv and cargo set
  their help (lines of up to 140 cells at 140 columns), because a line
  past about 90 characters tires the eye ([Butterick][measure]). clap's
  next version caps help at 100 and glamour wraps at 80 for the same
  reason ([clap][clap-width], [glamour][glamour]). At 140
  columns an answer reads as it does at 80, with the room past it left
  empty rather than stretched into.
- **Prose wraps rather than being cut**, at whole words, and each line
  keeps the inset of the line it continues rather than starting back at
  column 0 as the terminal's own fold does, because the inset says which
  thing a line belongs to ([the ladder][ladder]). Words set once for 80
  and left to fold are the worst case: ripgrep's help, set at 79, folds
  787 of its lines at 60 columns, and glow, wrapping lines already broken
  at 80, leaves a long line and a short one in turn.
- **A word too long for its line is shortened in its middle**, rather
  than broken over two lines, where glow leaves a path whole on none, or
  left for the terminal to fold. It's shortened to the room left on its
  line where that's 24 cells or more, rather than taking a line of its
  own and leaving the word before it alone. The whole word is in the JSON
  form. A path keeps its first part and as many of its last as fit, parted at
  slashes (`~/.cache/…/tmp/build-state-48`), rather than losing its start
  as git's `--stat` and starship's prompt do, because our paths share no
  root: the head says where a thing lives and the tail names it. git can
  drop the start because every path in a diffstat starts at the same
  repository. systemd and macOS shorten in the middle too
  ([systemd][systemd-ellipsize], [macOS][apple-middle]). Any other word
  is cut in the middle by letters, `…` between its halves.
- **A table gives way by whole columns**, in an order the command writes
  down, least needed first, rather than cutting every column a little as
  gh does, because a cut cell says nothing: at 60 columns each row of
  `gh pr list` reads "Move disk-guar...", "jitsusama/dis..." and "about
  18 minu...". What identifies a record is never dropped, and the last
  column left is cut only at a whole word, ending in the theme's `more`
  mark. A list gives way the same way, as the prompt does: a row's words
  after its name, then its side, each by whole items from its end, and
  then its name.
- **A label goes above its value when room is short.** Facts keep each
  label beside its value while the labels take less than half the room,
  and stack each over its value past that, rather than squeezing values
  into a column too narrow for a phrase. Help does the same: a flag's
  words go under its name when fewer than 24 cells are left beside it, as
  clap moves them once the names take 40 per cent of the width and the
  words no longer fit beside them, which uv and cargo do at 60. Beside
  is the better form wherever it fits, since a pair reads at a glance in
  half the lines; above is how it collapses on purpose.

Widths are counted in cells, by Unicode's width, never in bytes or
characters, and a mark and its space are two.

### Figures and Words

Figures are set as [Typesetting][typesetting] sets them on the desktop,
since they're the same facts: three significant digits taking the next
unit past 999 ("370 Mb/s", "1.20 GB"), decimal sizes, a real minus,
thousands with a comma, a scale as "2×", a range as "118 ms to 136 ms",
and a moment as the desktop writes one ("15:20", "yesterday 23:40",
"28 Oct"). Durations read "2 m 14 s", their largest two units.

The words are the launcher's:

- **Plain and short.** Sentence case, no closing full stop on a line, and
  no "Error:", "Success!" or "Warning:" ahead of what happened; the mark
  says which it is.
- **Say what changed.** A command that changes something ends by saying
  what it changed, in the past tense, naming the thing ("switched to
  1e6657e"). One that changed nothing says so ("already on 1e6657e"), so
  silence never means either.
- **The glossary's words.** A thing is called what the [glossary][glossary]
  calls it, never what a daemon or a library calls it.

## Help

Help is a surface too, set in the levels above, with examples first, since
people copy an example before they read a flag (clig). The launcher's, as
it reads keeping this page:

```text
launcher  any place, row or drawer the launcher holds, in a terminal

EXAMPLES
  launcher show machine disk     the disk's row and what it holds
  launcher show --drawers        home, with every row's drawer
  launcher do programs kitty     what Return does on kitty's row

COMMANDS
  show         a place or a row, as the launcher shows it
  do           what a row does on Return, or an act a place offers
  completion   zsh's completion

OPTIONS
  --yes                       do an act that asks first, unasked
  --format text|json|jsonl    the answer's form; --json is json
  --color never|always|auto   whether to ink; NO_COLOR and FORCE_COLOR too
  -h, --help                  this

EXIT
  0 done · 64 asked wrongly · 66 no such row · 69 the shell isn't answering
  77 answered no, or no one to ask
```

- **The first line** is the command's name, then what it's for in its own
  words, muted. The groups are Labels, their names Names, their words
  Words, one column for all of them.
- **A bare command** shows what's true now where it has a state to show,
  as `git status` and `launcher` do, and otherwise this help's first line,
  its examples and a pointer to `--help`. It never changes anything: a
  change always takes a verb.
- **A mistake suggests**, never runs: an unknown subcommand or flag names
  the nearest one and exits 64, and is never warned about and run on
  anyway.
- **Common flags mean one thing everywhere**: `-h/--help`, `--version`,
  `-q/--quiet`, `-v/--verbose`, `-n/--dry-run`, `-y/--yes`, `--no-input`,
  `-o/--output`, `--format`, `--json`, `--color`.

## Errors

An error is a conversation, not a trace (clig), and says three things in
the order they're read, the most useful last, where the eye ends:

```text
✕ couldn't reach the builder
  connection refused after 10 s
  try  nix-offload there --here-when-idle . checks.x86_64-linux.example-nixos
```

- **What failed**, after the failed mark in alert: one line, naming the
  thing in the glossary's words.
- **Why**, muted: the cause as the system said it, made plain where it can
  be, under the words, two cells in.
- **What to do**, where there's something: a command to type after "try",
  or the one thing to check. A command that can't say what to do leaves
  the line out rather than guess.

No stack, no `anyhow` chain and no source path by default; `--verbose`
adds the chain. Errors of one kind are grouped under one line saying how
many, rather than printed one by one.

## Exit Codes

One table for every command, so a script learns it once. Past 0 and 1
they're sysexits.h's, which about a quarter of our commands used when
this page was written, and which sit apart from the small numbers a
command passes through from a program it runs:

| Code | Kind          | Means                                                                    |
| ---- | ------------- | ------------------------------------------------------------------------ |
| 0    |               | done, or nothing was wrong                                               |
| 1    | `failed`      | it tried and failed                                                      |
| 64   | `usage`       | asked wrongly: an unknown word or flag, a value it can't take            |
| 65   | `bad-input`   | what it was given to read isn't what it reads                            |
| 66   | `not-found`   | the thing named isn't there                                              |
| 69   | `unavailable` | what it needs isn't answering: a daemon, the shell, a builder            |
| 70   | `bug`         | it broke in a way it shouldn't, and says where to report it              |
| 75   | `try-again`   | not now: a lock is held, a builder is busy, the network is down          |
| 77   | `refused`     | not done on purpose: a guard, a permission, a question answered no or with no one to answer it |
| 78   | `config`      | its configuration is wrong, and it names the setting                     |
| 130  |               | interrupted by Ctrl+C, as 128 plus the signal                            |

- **A question answers in 0 and 1.** A command that asks yes or no, as
  `test` and `grep -q` do (is a call on, is this commit known), exits 0
  for yes and 1 for no, and from the table for anything else, so "no"
  never reads as "broken".
- **A stand-in passes through.** A command that runs another program as
  its stand-in (a wrapper, `run` on another machine) exits with that
  program's code, and says so in its help.
- **Each lists its own.** A command lists the codes it uses in its help's
  EXIT group, and the kind is the `kind` of its JSON error.

## Waiting

Responsive is more important than fast (clig): a command says something
within 100 ms of starting anything that may take longer, on stderr.

- **Into a terminal**, a wait is one line that rewrites in place: the
  running mark, what it's doing, and how far along it is where that's
  known, as a ten-cell braille meter over a dim track, the graph Joel
  liked on the desktop. When it's over, the line gives way to its outcome,
  the done or failed mark and what happened, and stays.
- **Into a pipe**, the same events print once each as plain lines, with no
  carriage return and no meter, so a log reads as a list.
- **A failure prints what the wait hid**: the output it was holding back,
  after the error, since that's what Joel will want next.
- **Ctrl+C says so at once**, bounds its clean-up, and a second Ctrl+C
  skips it, saying what that leaves behind.

## Asking

A command asks only when stdin is a terminal, and never needs to: every
answer can be given by a flag, `--yes` answers yes and `--no-input` makes
any question an error that names the flag. Its prompt wears the accent,
the only place a command does. What can't be undone asks once, and what
ends something asks by its name, as the launcher does. A secret is never
read from a flag, only from a file or stdin.

## The Terminal Kit

[The terminal kit][kit] is the Rust crate every command of ours builds
on, as daemons build on daemon-serve, so none keeps a dialect of its own.
It depends on serde_json and unicode-width alone, which most of our
crates hold already, since a parser crate would be most of a small
command's dependencies, as disk-guard's reading of its arguments says.
It gives:

- the shared flags (`--format`, `--json`, `--color`, `-h`, `--help`,
  `--version`), taken from the arguments before the command reads the
  rest its own way;
- the ink decision above for each stream, and the theme's roles and marks;
- help set in the levels from what the command declares, and the same
  declaration as JSON for the check;
- tables and lists set to How It Reads, fitting the width and giving way;
- the error's three parts, as text and as JSON, and the exit codes as a
  type;
- the wait line and its meter, drawn only into a terminal.

A command that isn't Rust keeps the convention by hand, and the check
holds a script to the points it can keep without the kit, below.

A repository built on core takes the kit from core's git, pinned to a
commit of main, since the kit isn't published to crates.io:

```toml
[dependencies]
terminal-kit = { git = "https://github.com/Jitsusama/core.nix", rev = "<commit>" }
```

Its Nix package then names the kit's hash beside its lock, in
`cargoLock.outputHashes."terminal-kit-0.1.0"`: set to `lib.fakeHash`
first, a build refuses it and prints the real one. It holds its commands
to this page with `core.lib.commandsAgree`, as core does, given the
commands it builds on the kit and a waiting list of its own.

## When a Command Doesn't Agree Yet

The `commands-agree` check ([`tests/rules/commands-agree.py`][check])
reads every program a crate builds, and holds it to five points:

- **kit:** its crate builds on the terminal kit;
- **exits:** its own code never ends the process, with `process::exit`
  or an `ExitCode` made from a number, so its codes are the table's; a
  thread that hears a signal ends it with the kit's `Exit::end_now`;
- **inks:** its own code spells no escape sequence and takes no colour
  crate, so its colour is the kit's decision;
- **helps:** run with `--help --format json`, it answers `help/1` under
  its own name, says nothing on stderr and exits 0 within ten seconds, and
  `--version` answers one line. The check is given each command on the kit
  built, to run it;
- **fits:** its `--help`, set in a terminal 60, 80, 100 and 140 columns
  wide, runs no line past the edge, keeps each line's inset when it wraps,
  parts no figure from its unit across lines, and reads at 100 and 140 as
  at 80, as [Width and What Gives Way](#width-and-what-gives-way) says.
  What a command answers from a machine's state is held to the same rules
  by its crate's own tests, which set it at 60 and 100 from made-up state,
  as disk-guard's status tests do.

It reads every script a Nix file outside the tests wraps as a program,
with `writeShellApplication` or `writeShellScriptBin`, too: the files its
`text` reads or runs and the strings it spells. A script is held to four
of the points by hand:

- **exits:** every exit it spells with a number is the table's;
- **inks:** it spells no escape sequence and sets no colour with `tput`;
- **helps:** run with `--help`, it opens its answer on stdout with its own
  name, says nothing on stderr and exits 0 within ten seconds, as the
  check runs it when given its package;
- **fits:** its help fits each width as a crate's does, which a script
  meets by setting its help in lines of at most 60 cells.

A script by a crate program's own name, telling it its settings, is that
program as it's typed, so its exits and inks count as the program's.

Commands written before this page wait in
[`tests/rules/commands-agree-waiting.nix`][waiting], each with the points
it still breaks. The list only shrinks: a command that keeps a point takes
it off, and a new command keeps every point from its first commit.

[design]: design.md
[taste]: ../modules/theme/SKILL.md
[kit]: ../modules/terminal-kit/Cargo.toml
[check]: ../tests/rules/commands-agree.py
[waiting]: ../tests/rules/commands-agree-waiting.nix
[clig]: https://clig.dev/
[nocolor]: https://no-color.org/
[force]: https://force-color.org/
[glossary]: glossary.md
[nearest]: ../modules/theme/nearest.nix
[colour]: design.md#colour
[levels]: design.md#the-levels
[ladder]: design.md#the-ladder
[typesetting]: design.md#typesetting
[reads]: #how-it-reads
[git-width]: https://github.com/git/git/blob/master/pager.c
[man-width]: https://man7.org/linux/man-pages/man1/man.1.html#ENVIRONMENT
[gh-table]: https://github.com/cli/go-gh/blob/trunk/pkg/tableprinter/table.go
[ls-width]: https://www.gnu.org/software/coreutils/manual/html_node/General-output-formatting.html
[measure]: https://practicaltypography.com/line-length.html
[clap-width]: https://docs.rs/clap/latest/clap/struct.Command.html#method.max_term_width
[glamour]: https://github.com/charmbracelet/glamour
[systemd-ellipsize]: https://github.com/systemd/systemd/blob/main/src/basic/string-util.c
[apple-middle]: https://developer.apple.com/documentation/uikit/nslinebreakmode/bytruncatingmiddle
