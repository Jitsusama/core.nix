# The ErgoDox

Joel's keyboard is an ErgoDox EZ Glow, and core.nix builds its firmware:
ZSA's QMK with the layout made in Oryx, which every workstation can write to
the board. [Decision 0011][decision] says why it's built here rather than in
Oryx.

Everything about it is in [`modules/ergodox/`][module]:

| File                            | What it holds                                                  |
| ------------------------------- | -------------------------------------------------------------- |
| `keymap/keymap.c`               | the layers, the macros and the lights, in QMK's own C          |
| `keymap/config.h`               | tapping, the RGB effects left out, and the macros' helpers     |
| `keymap/rules.mk`               | which QMK features are built in                                |
| `firmware.nix`                  | the build: the firmware, its layout as JSON, its serial number |
| `unused-config-result.patch`    | the one change to QMK, so GCC 16 builds it without warnings    |
| `ergodox.sh`                    | the `ergodox` command                                          |
| `home.nix`                      | the home-manager module: the command and zapp                  |
| `nixos.nix`                     | the NixOS module: the board's devices open to whoever is at it |

`flake.nix` offers the modules as `homeModules.ergodox` and
`nixosModules.ergodox`, and the `workstation` roles import both. QMK and the
two of its submodules the build needs are the `qmk`, `lufa` and `printf`
inputs.

## The Layout

Three layers, as Oryx named them:

- **Coal:** Colemak Mod-DH. On each hand's home row, held, the little finger
  is Ctrl, the ring finger Alt, the middle finger Punk, the index finger
  Super and the key inside it Numb.
- **Punk:** the symbols, with each bracket's pair on the other hand.
- **Numb:** a number pad on the right hand, the lights on the left, and the
  power key, the screen lock, Do Not Disturb, Print Screen, the layer lock
  and the bootloader key around them.

`keymap.c` is the layout as Oryx wrote it, byte for byte, so QMK's
[documentation][qmk] reads straight onto it. The build also reads it back
into `layout.json` with QMK's own `qmk c2json`, which is what the shell draws
the layers from.

## Changing It

Edit the files in `keymap/`, switch a machine to the change, as
[Testing][testing] says to try one, and flash the board from it. `ergodox`
writes the build the machine was switched to:

```sh
nixos-rebuild switch --flake .#<machine> --override-input core path:$HOME/src/core.nix
ergodox flash
```

`the-ergodox-layout-builds` catches a change that doesn't compile or no
longer fits in the board's 32 KiB; QMK reports what's left as it builds.

## Flashing

`ergodox flash` writes this machine's build with zapp. zapp says when to
press the bootloader key, Delete's key on either thumb while Numb is held,
then writes the firmware and restarts the board. `ergodox serial` prints the serial number
the board answers once it runs that build, made from the keymap's hash.

To put Oryx's layout back:

```sh
zapp flash https://configure.zsa.io/ergodox-ez/layouts/omjyr/XbWEA4/0
```

## Moving to a New ZSA Branch

ZSA's fork has a branch per firmware generation. The layout was made for
`firmware24`. To move, pin the `qmk` input to the new branch's latest commit,
and `lufa` and `printf` to the commits that commit records for its
submodules, which `git submodule status` in a clone of it prints. Then build
and flash, and try every key on every layer.

[decision]: decisions/0011-build-the-ergodox-firmware-here.md
[module]: ../modules/ergodox/
[qmk]: https://docs.qmk.fm/
[testing]: testing.md#the-loop
