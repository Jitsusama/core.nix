# The Desktop VM Test's Areas

The desktop test boots the graphical role's desktop in a virtual machine and
uses it as Joel does, as [`../desktop.nix`][desktop] says in its opening
comment. It's split into areas of a minute or two, each a check of its own,
and this folder holds each area's tests and what the test's machine runs to
drive the desktop. Every file opens with what it is.

| Here                                      | Holds                                                  |
| ----------------------------------------- | ------------------------------------------------------ |
| `desktop-<area>-vm-test.py`               | one area's tests, run as the check `vm-desktop-<area>` |
| `desktop_session.py`                      | the boot and the helpers every area shares             |
| `looks_session.py`                        | what the looks areas share: Joel's scales and measures |
| `stand-in-*.py`, `*-stand-in.sh`          | a daemon, a program or a server the VM can't run       |
| `stand-in-virsh.sh`, `stand-in-time.conf` | virsh for the launcher's VMs, and a time server        |
| `keyboard.py`, `touchscreen.py`           | a keyboard and a touchscreen that act when told        |
| `copilot-key.py`, `ergodox.py`            | a laptop's Copilot key, and the ErgoDox plugged in     |
| `ask-portal.py`, `choose-a-file.py`       | a program asking the portal to open a file, or for one |
| `tuple-window.py`, `window-storm.py`      | windows as Tuple names them, and a crowd of them       |
| `front-desk.py`                           | a hotel's front desk, which every page goes to         |
| `people-*.qml`, `people-*.sh`             | stand-in People sources, and what the daemon runs      |
| `search-ends.qml`                         | a launcher source that ends each search at the top     |

[desktop]: ../desktop.nix
