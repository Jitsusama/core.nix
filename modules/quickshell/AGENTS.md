# The Shell

Quickshell draws everything on the desktop that isn't a window: the
launcher, notifications, the margin, meters,
prompts, the lock screen. [`home.nix`][home] installs these files as the
account's Quickshell config and writes the ones that come from settings
beside them: `Theme.qml` from `jitsusama.theme`, `Providers.qml`,
`Options.qml`, `Pieces.qml` and `Shortcuts.qml`.

Every surface is held to [`docs/design.md`][design] and the aesthetic skill
beside it ([`../theme/SKILL.md`][skill]). Read both before changing anything
Joel sees.

Every case the shell is proven by goes in the lowest tier of [the
pyramid][pyramid] that can show it, as [Checking](#checking) below sets out:
its logic in Qt's engine or offscreen Quickshell in under a second, what it
does on niri on a nested niri in seconds, and in a VM test only what needs a
machine, one path through each connection.

## How It's Built

- **State and drawing.** A singleton (`pragma Singleton`) holds what the
  shell knows: `Niri`, `Notices`, `Music`, `Wifi`, `Quiet` and the rest. A
  surface only draws from them; a surface holding state another needs is a
  singleton waiting to be split out.
- **Values from the theme.** Every colour, face, size, space, radius and
  duration is `Theme`'s. The `nothing-names-its-own-look` lint fails a file
  naming one of its own.
- **Events, never polling.** `Niri.qml` is the shell's one connection to
  niri, which every surface shares: a socket for asking and one hearing
  niri's events. The rest listen on D-Bus, PipeWire or a native helper's
  socket, and nothing spawns a process to ask. What the clipboard holds
  now is said by the clipboard's history over its socket (`Copied.qml`),
  which everything acting on a copy reads rather than watching again.
- **Built once, then shown.** A surface is made at start and shown or hidden,
  so it's on screen the frame after its chord.
- **Programs through niri.** The shell asks niri to start a program, never
  starts one itself, so restarting the shell takes nothing down
  (`quickshell-opens-programs-through-niri`).
- **Keys first.** Every surface can be driven from the keyboard alone, on the
  laptop and the ErgoDox, in the language docs/design.md sets out.
- **Sessions reopen when Joel asks.** After a boot, the sessions it ended wait
  in Agents' Sessions to Reopen until he presses Reopen Them All or types
  `:reopen`, which brings them all back in one act, each where it sat; the
  shell starting never reopens one. He chose that on 2026-11-01, and the
  shell reopening them by itself after a reboot was undone on 2026-11-07
  ("no, i never wanted them automatic"). A session that dies while the
  machine is up is cadence's to start again, not the shell's.
- **Agents come from sources.** Each kind of agent the machine runs is a
  source behind one shape (`AgentSources`), so Agents is one place whatever
  runs its sessions. pi, read from the realm's mycelium node, is the first,
  in `PiSource`; another agent is a source beside it, named in
  `AgentSources.sources`. `Sessions` keeps what the shell does with pi's
  sessions here: their windows, reopening and notices.

## Layout

- `*.qml` at the top: the singletons and the surfaces, `shell.qml` making
  each surface.
- `*.js` at the top: what a surface works out apart from drawing, such as
  `Stack.js` for the notices' order or `Picking.js` for the region picker,
  each held by a test in Qt's own engine, as its header says.
- Programs the shell runs: `askpass.sh`, OpenSSH's askpass asked through
  the shell; `pi-helper.nix`, which builds a helper of one Rust file,
  Tuple's among them; `keyboard-lights.rs`, which says whether caps
  lock is on, run by `nixos.nix`; [`record-window/`][record], one
  window's recording for the recorder; and [`whereabouts/`][whereabouts],
  the daemon that holds where the laptop is, from GeoClue, and the
  clock's zone, over `daemons/location.sock`.
- [`maths/`][maths]: libqalculate as a QML module, `Maths`, which the
  launcher's sums and measures are worked out by in the shell's own
  process, its definitions read once and each key asked with a time limit;
  `nixos.nix` puts it on Quickshell's import path.
- `*.patch`: Quickshell's own fixes, each saying in its header what it
  mends.
- [`launcher/`][launcher]: one provider per kind of thing the launcher finds,
  each starting from [`Provider.qml`][provider], whose header says what a
  row holds. Every name, word and glyph a provider shows or is found by is
  said once, in its place's file under [`launcher/words/`][words], read through `Words.qml`:
  what sits beside a row in `said`, lowercase, and a sentence its drawer
  tells in `tells`;
  `the-launcher-means-one-thing-a-word` fails on a word that finds two
  things, and prints each. What it can do is asked from outside through
  its catalogue, `qs ipc call launcher list`, `run` and `runWith`, which
  walks the same rows Return takes by `launcher/Catalogue.js`'s rules, so
  pi-ask acts by what the launcher holds and gains whatever it gains.
  What it shows is asked the same way, by `show`, walked by
  `launcher/Showing.js`'s rules.
- [`in-a-terminal/`][terminal]: `launcher`, a Rust program printing any
  place, row or drawer the launcher holds, as text or as JSON, from the
  shell's `show`, so every integration has a command line at once; zsh
  completes its steps by asking the shell the same way. `launcher do`
  does what a row offers through `doing`, asking the launcher's question
  on the terminal before an act that ends something.
- [`pieces/`][pieces]: the pieces the margin and the launcher's foot show,
  each starting from `Piece.qml`.
- [`clipboard/`][clipboard]: the clipboard's history, a Rust daemon.
- [`checks/`][checks-folder]: the shell's checks, one file a place, named
  as the launcher names it. `checks.nix` lists each file by name, so only a
  new place edits it, and two places' changes never share a file.
- [`home/`][home-folder]: the files the shell links as they are, a list a
  place, which `home.nix` joins by name; a new file goes in its place's list.

## QML

A file whose delegates read the ids around them says `pragma
ComponentBehavior: Bound`, and every delegate declares what it takes from
its model as `required` properties. A nested item names its delegate by id
rather than reading the delegate's properties bare.

A ListView that sizes itself to its rows takes its height from its model's
length, never from its own `count`: bound to that, the held notices' menu
stood no taller than nothing with eleven rows listed. A check that reads a
view's rows counts only those wholly inside it, since `itemAtIndex` also
answers for rows made but clipped away.

`quickshell-passes-qmllint` runs qmllint over the account's files, and any
warning fails. It reads Quickshell's own type files, with three of their
faults corrected, and leaves out two kinds of report that are gaps in those files
rather than the shell; its comment in [`checks/shell.nix`][checks] says which. A
line qmllint still can't read because of them says so with a `qmllint
disable` comment and the reason above it.

## Checking

A case goes in the first of these that can show it, cheapest first:

1. **Qt's engine or offscreen Quickshell**, over cases in `tests/`, in under
   a second each, as [testing][logic] lists: a ranking, an order, what a key
   does to a list, how a row or a drawer is laid out and cut. Logic that sits
   in a view file moves into a `.js` beside it, where those checks can reach
   it, and a surface's drawing is held offscreen in a `FloatingWindow`, as
   `tests/rows.qml` does.
2. **A nested niri** in the build sandbox ([testing][nested]), in seconds:
   a surface landing as niri's layer, what a bind's command does, what the
   shell reports as niri's windows change. Nothing types there, so a check
   runs what a key runs.
3. **A VM test**: only what needs a machine, such as keys pressed through
   niri's binds, a system service, screens coming and going or Joel coming
   back, and one path through each connection, never each case of it.

The desktop's VM tests ([`tests/desktop.nix`][desktop]), one per area, each
named `vm-desktop-<area>`, start niri and the shell in a virtual machine and
drive each surface by its keys. A case only a running machine shows gets a
subtest in its area's file under `tests/desktop/`, and a helper two areas use
goes in its prelude. An area past about two minutes or twenty subtests splits,
as [Earning a Check][earning] says, and `every-vm-test-keeps-to-the-pyramid`
fails on one that grows.
A picture is taken with `shoot`, once the screen holds still, after
waiting on what the shell reports for the state it shows. Only an area's
gallery, `vm-desktop-<area>.gallery`, takes them, so a subtest never leans
on `shoot` to let the screen settle; one that measures the screen uses
`settled_screen`.

To see a surface while changing it, run `shell-preview` from the dev shell.
It opens the desktop test's shell in a niri window of its own, built from the
working copy, and an edit to any QML or JavaScript file shows as Quickshell
reloads it; `--type sl` opens the launcher on a query, with the test
machine's stand-in programs. What Nix generates, such as `Theme.qml`, takes a
new preview.

To prove it, run its cases in the tiers below first, then only its subtests,
with `run-subtests vm-desktop-<area> <words>` from the dev shell, in under
half a minute; each runs alone. Before pushing, run the area it's about once,
and compare its gallery with main's using `shots-changed`, as [testing][testing]
says, looking at every picture it draws: a change to one surface can move
another.

[checks]: checks/shell.nix
[checks-folder]: checks/
[home-folder]: home/
[clipboard]: clipboard/
[design]: ../../docs/design.md
[desktop]: ../../tests/desktop.nix
[home]: home.nix
[record]: record-window/
[whereabouts]: whereabouts/
[terminal]: in-a-terminal/
[launcher]: launcher/
[maths]: maths/
[pieces]: pieces/
[provider]: launcher/Provider.qml
[words]: launcher/words/README.md
[skill]: ../theme/SKILL.md
[logic]: ../../docs/testing.md#the-desktops-logic
[nested]: ../../docs/testing.md#the-desktop-on-niri
[pyramid]: ../../docs/testing.md#the-pyramid
[earning]: ../../docs/testing.md#earning-a-check
[testing]: ../../docs/testing.md#what-changed-on-screen
