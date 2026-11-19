# The Desktop VM Test's Areas

The desktop test boots the graphical role's desktop in a virtual machine and
uses it as Joel does, as [`../desktop.nix`][desktop] says in its opening
comment. It's split into areas of a minute or two, each a check of its own,
and this folder holds each area's tests and what the test's machine runs to
drive the desktop. Every file opens with what it is.

| Here                                                       | Holds                                                  |
| ---------------------------------------------------------- | ------------------------------------------------------ |
| `desktop-<area>-vm-test.py`                                | one area's tests, run as the check `vm-desktop-<area>` |
| [`desktop_session.py`][desktop-session]                    | the boot and the helpers every area shares             |
| [`looks_session.py`][looks]                                | what the looks areas share: Joel's scales and measures |
| [`keeps_pace.py`][keeps-pace], `keeps-pace-test.py`        | what keeps-pace judges by, and its cases off any VM    |
| `checks.nix`                                               | the checks of what the areas judge by, off any VM      |
| `stand-in-*.py`, `*-stand-in.sh`                           | a daemon, a program or a server the VM can't run       |
| [`stand-in-virsh.sh`][virsh], [`stand-in-time.conf`][time] | virsh for the launcher's VMs, and a time server        |
| [`keyboard.py`][keyboard], [`touchscreen.py`][touchscreen] | a keyboard and a touchscreen that act when told        |
| [`copilot-key.py`][copilot], [`ergodox.py`][ergodox]       | a laptop's Copilot key, and the ErgoDox plugged in     |
| [`ask-portal.py`][portal], [`choose-a-file.py`][choose]    | a program asking the portal to open a file, or for one |
| [`tuple-window.py`][tuple], [`window-storm.py`][storm]     | windows as Tuple names them, and a crowd of them       |
| [`front-desk.py`][front-desk]                              | a hotel's front desk, which every page goes to         |
| `people-*.qml`, `people-*.sh`                              | stand-in People sources, and what the daemon runs      |
| [`search-ends.qml`][search-ends]                           | a launcher source that ends each search at the top     |

[desktop]: ../desktop.nix
[desktop-session]: desktop_session.py
[looks]: looks_session.py
[keeps-pace]: keeps_pace.py
[virsh]: stand-in-virsh.sh
[time]: stand-in-time.conf
[keyboard]: keyboard.py
[touchscreen]: touchscreen.py
[copilot]: copilot-key.py
[ergodox]: ergodox.py
[portal]: ask-portal.py
[choose]: choose-a-file.py
[tuple]: tuple-window.py
[storm]: window-storm.py
[front-desk]: front-desk.py
[search-ends]: search-ends.qml
