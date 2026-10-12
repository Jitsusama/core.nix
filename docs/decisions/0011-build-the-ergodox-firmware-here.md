# 0011: Build the ErgoDox's Firmware Here

## Status

Accepted, 2026-10-12.

## Context

Joel types on an ErgoDox EZ Glow and plugs it into every machine he has. Its
layout was made in Oryx, ZSA's web configurator, which compiles the firmware
on ZSA's servers and hands back a file for Keymapp or zapp to flash. Oryx
keeps the layout: the source it gives out can't be imported back, and nothing
public writes a layout to it.

He wants the layout he has, every key, layer and macro, and to be able to
change it in code the way he changes everything else: in a repository, with
a review and a check, built the same on every machine.

## Decision

core.nix builds the firmware itself, in `modules/ergodox/`:

- **Source:** ZSA's fork of QMK, on the branch the layout was made for, with
  the two submodules an AVR board compiles in, all three pinned by commit.
- **Layout:** the `keymap.c`, `config.h` and `rules.mk` Oryx gave out,
  kept in QMK's own format, so QMK's documentation and tools apply to them.
  `keymap.c` came over byte for byte, and only the serial number changed.
- **Serial number:** one the build gives, from the keymap's hash, rather than
  Oryx's layout and revision, which is how Keymapp and `zapp update` find a
  board's layout to replace it.
- **Flashing:** `ergodox flash`, which writes this machine's build with zapp,
  so what's on the board is what core.nix says.
- **What the shell reads:** QMK's own reading of `keymap.c` back into JSON,
  built beside the firmware, so the board and anything drawing its layers
  can't disagree.

## Alternatives

- **Keep Oryx and flash its builds.** Nothing to build or pin. But the
  layout stays on ZSA's servers, a change is made by clicking rather than in
  code, and the shell would have to fetch the layout from Oryx to draw it.
- **Keep Oryx as the source and build its exports here.** The same layout,
  built locally. But every change still starts in Oryx, and one made in the
  repository can never go back to it.
- **The layout as Nix data, with `keymap.c` generated from it.** One
  description for the firmware and the shell. But it hides the layout from
  QMK's documentation and tools, which [Config Files Stay Files][files]
  rules out, and QMK can already read `keymap.c` back into data.
- **Mainline QMK.** Newer, and it supports the board. But it lacks ZSA's raw
  HID protocol, which tells the computer the layer and lets it change one,
  and it's months ahead of the fork the layout was made for. Moving to it is
  a later step, once this build is proven on the board.

## Consequences

- Oryx no longer describes the board. The layout it holds stays online, and
  `zapp flash` with its URL puts it back.
- Keymapp and `zapp update` no longer recognise the board, by design: the
  board's own commands and the shell take their place.
- Each machine builds the firmware once per change, with the AVR compiler,
  in about a minute.
- A new branch of ZSA's fork takes the three pins together, at the commits
  it records for its submodules.

[files]: ../conventions.md#config-files-stay-files
