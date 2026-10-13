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
| `keymap/host.c`                 | the commands the computer can send the board                   |
| `firmware.nix`                  | the build: the firmware, its layout as JSON, its serial number |
| `unused-config-result.patch`    | a change to QMK, so GCC 16 builds it without warnings          |
| `host-commands.patch`           | a change to ZSA's code, handing `host.c` the commands it takes |
| `layers.nix`                    | the layers' names, in the keymap's order                       |
| `ergodox.sh`                    | the `ergodox` command                                          |
| `ergodox-watch.rs`              | the watcher that tells the shell which layer the board is on   |
| `home.nix`                      | the home-manager module: the command and zapp                  |
| `nixos.nix`                     | the NixOS module: the board's devices open to whoever is at it, and the watcher |

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

`ergodox flash` restarts the board ready to be flashed and writes this
machine's build with zapp. A board running anything else, such as Oryx's
layout, can't be restarted from the computer, so zapp says when to press the
bootloader key instead: Delete's key on either thumb while Numb is held. On a
Mac the key is always needed, since only Linux's hidraw lets a script reach
the board's commands. `ergodox serial` prints the serial number the board
answers once it runs that build, made from the keymap's hash.

To put Oryx's layout back:

```sh
zapp flash https://configure.zsa.io/ergodox-ez/layouts/omjyr/XbWEA4/0
```

## Talking to the Board

ZSA's firmware listens on a raw HID channel of its own, which Oryx and
Keymapp use to read the board's layer and set its lights. Those tools pair
first, and a paired board reports every key pressed to whoever paired, until
it restarts. `host.c` adds commands that need no pairing, numbered from 0x80
so they stay clear of any Oryx adds:

| Command        | What the board does                                             |
| -------------- | --------------------------------------------------------------- |
| 3, Disconnect  | ends a pairing, which Oryx's own code never does               |
| 0x80           | restarts ready to be flashed                                    |
| 0x81, on/off   | tells the computer its layer whenever it changes, and never keys |
| 0x82, layer    | locks that layer on, as the layer lock key does; 0 unlocks      |
| 0x83, on/off   | turns its lights on or off, until it restarts                   |
| 0x84, up/down  | steps its lights' brightness, until it restarts                 |

Each is one 32-byte report: the command, then what it takes. A watched
board answers with reports of its own: 0x81, the layer, then 1 if it's
locked and 0 if a key holds it. It sends one only once the last was read,
since writing past a report nobody has read holds the board up for about
10 ms, so a computer that stops reading costs it nothing and hears the
layer as it is when it reads again.

On NixOS the `ergodox-watch` user service watches the board and writes its
layer to `keyboard-layer` in the runtime directory, such as `Numb locked`,
or `-` while no board is plugged in. The shell's margin shows a locked
layer, which is easy to forget, and leaves alone one a key is holding.

The `ergodox` command sends them on Linux:

```sh
ergodox bootloader
ergodox layer numb      # or coal, punk, or a layer's number
ergodox lights off      # or on
ergodox brighter        # or dimmer
```

`the-ergodox-command-reaches-the-board` checks it finds the channel by ZSA's
vendor number and Oryx's usage page together and writes each command whole.

## Moving to a New ZSA Branch

ZSA's fork has a branch per firmware generation. The layout was made for
`firmware24`. To move, pin the `qmk` input to the new branch's latest commit,
and `lufa` and `printf` to the commits that commit records for its
submodules, which `git submodule status` in a clone of it prints. Then build
and flash, and try every key on every layer. `host-commands.patch` has to
apply to the new branch's `oryx.c`, and the commands have to stay clear of
any Oryx has added.

[decision]: decisions/0011-build-the-ergodox-firmware-here.md
[module]: ../modules/ergodox/
[qmk]: https://docs.qmk.fm/
[testing]: testing.md#the-loop
