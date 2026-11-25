# The Stand-Ins

What stands in for a program or device a check's machine lacks, for the
checks in `tests/` and in modules' own folders, kept together so `tests/`
lists checks rather than their parts. Each file's opening comment says what
it answers and how a test drives it.

| File                         | Stands in for                                                  |
| ---------------------------- | -------------------------------------------------------------- |
| `battery-stand-ins/`         | what a service following the battery talks to, in its README  |
| `btvirt.nix` and its patch   | a Bluetooth radio, BlueZ's btvirt, with a buffer per client    |
| `busctl-stand-in.sh`         | busctl, closing the notify-send stand-in's notices             |
| `desk-light-stand-in.py`     | a Litra Glow or a Pebble's lights, made of uhid                |
| `notify-send-stand-in.sh`    | notify-send, keeping each notice and waiting as it waits       |
| `pi-stand-in.nix`, `.sh`     | pi, as the shell opens it, for the VM tests                    |
| `spotify-recent-stand-in.sh` | spotify-recent, for Spotify's own check and the desktop        |
| `stand-in-notices.py`        | whatever shows notices, on the session bus                     |
| `warp-cli-stand-in.sh`       | warp-cli, for a check of the shell's and the desktop           |
| `stand-in-backlight/`        | a laptop's panel and keyboard lights, as a kernel module       |
| `stand-in-lid/`              | a laptop's lid switch, as a kernel module                      |
| `stand-in-light/`            | an ambient light sensor, as a kernel module                    |
| `stand-in-monitor/`          | a desk monitor taking DDC/CI over an MST hub, as a kernel module |

Each kernel module is built by its folder's `default.nix` for whichever
kernel the test machine boots, called from that machine's kernel packages.
