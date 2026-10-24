# The Shell

Quickshell draws everything on the desktop that isn't a window: the
launcher, notifications, the margin, meters,
prompts, the lock screen. [`home.nix`][home] installs these files as the
account's Quickshell config and writes the ones that come from settings
beside them: `Theme.qml` from `jitsusama.theme`, `Providers.qml`,
`Options.qml`, `Overview.qml` and `Shortcuts.qml`.

Every surface is held to [`docs/design.md`][design] and the aesthetic skill
beside it ([`../theme/SKILL.md`][skill]). Read both before changing anything
Joel sees.

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
  socket, and nothing spawns a process to ask.
- **Built once, then shown.** A surface is made at start and shown or hidden,
  so it's on screen the frame after its chord.
- **Programs through niri.** The shell asks niri to start a program, never
  starts one itself, so restarting the shell takes nothing down
  (`quickshell-opens-programs-through-niri`).
- **Keys first.** Every surface can be driven from the keyboard alone, on the
  laptop and the ErgoDox, in the language docs/design.md sets out.

## Layout

- `*.qml` at the top: the singletons and the surfaces, `shell.qml` making
  each surface.
- [`launcher/`][launcher]: one provider per kind of thing the launcher finds,
  each starting from [`Provider.qml`][provider], whose header says what a
  row holds.
- [`pieces/`][pieces]: the pieces the margin and the launcher's foot show,
  each starting from `Piece.qml`.
- [`clipboard/`][clipboard]: the clipboard's history, a Rust daemon.

## QML

A file whose delegates read the ids around them says `pragma
ComponentBehavior: Bound`, and every delegate declares what it takes from
its model as `required` properties. A nested item names its delegate by id
rather than reading the delegate's properties bare.

`quickshell-passes-qmllint` runs qmllint over the account's files, and any
warning fails. It reads Quickshell's own type files, with two of their faults
corrected, and leaves out two kinds of report that are gaps in those files
rather than the shell; its comment in [`checks.nix`][checks] says which. A
line qmllint still can't read because of them says so with a `qmllint
disable` comment and the reason above it.

## Checking

The shell's logic is proven where it's cheapest: in Qt's JavaScript engine
or offscreen Quickshell, over cases in `tests/`, in under a second each, as
[testing][logic] lists. Logic that sits in a view file moves into a `.js`
beside it, where those checks can reach it.

`vm-desktop-works` ([`tests/desktop.nix`][desktop]) starts niri and the shell
in a virtual machine and drives each surface by its keys, keeping a
screenshot of each; what only a running desktop shows gets a subtest there.
A picture is taken with `photograph`, once the screen holds still, after
waiting on what the shell reports for the state it shows.

While changing a surface, run only its subtests, with `run-subtests
vm-desktop-works <words>` from the dev shell, in under half a minute; a
subtest that uses what an earlier one left needs that one named too. Before
pushing, run the whole test once, and compare its pictures with main's using
`shots-changed`, as [testing][testing] says, looking at every one it draws:
a change to one surface can move another.

[checks]: checks.nix
[clipboard]: clipboard/
[design]: ../../docs/design.md
[desktop]: ../../tests/desktop.nix
[home]: home.nix
[launcher]: launcher/
[pieces]: pieces/
[provider]: launcher/Provider.qml
[skill]: ../theme/SKILL.md
[logic]: ../../docs/testing.md#the-shells-logic
[testing]: ../../docs/testing.md#what-changed-on-screen
