# The Desktop Test's Stand-Ins

What the desktop test's machine runs in place of something a VM can't have:
a daemon, a program, a server, a device or a page. [`../../desktop.nix`][desktop]
names each where it puts it on the machine, and some shell checks and the
screens test borrow one. Every file opens with what it stands in for.
Those other tests share are in [`../../stand-ins/`][shared]; the few that
shell checks and the screens test borrow stay here, as the desktop's own.

| Here                                                       | Stands in for                                           |
| ---------------------------------------------------------- | ------------------------------------------------------- |
| `stand-in-*.py`, `*-stand-in.sh`                           | a daemon, a program or a server the VM can't run        |
| [`stand-in-virsh.sh`][virsh], [`stand-in-time.conf`][time] | virsh for the launcher's VMs, and a time server         |
| [`keyboard.py`][keyboard], [`touchscreen.py`][touchscreen] | a keyboard and a touchscreen that act when told         |
| [`copilot-key.py`][copilot], [`ergodox.py`][ergodox]       | a laptop's Copilot key, and the ErgoDox plugged in      |
| [`ask-portal.py`][portal], [`choose-a-file.py`][choose]    | a program asking the portal to open a file, or for one  |
| [`tuple-window.py`][tuple], [`window-storm.py`][storm]     | windows as Tuple names them, and a crowd of them        |
| [`front-desk.py`][front-desk]                              | a hotel's front desk, which every page goes to          |
| `people-*.qml`, `people-*.sh`                              | People sources, and what the daemon runs                |
| [`search-ends.qml`][search-ends]                           | a launcher source that ends each search at the top      |

[desktop]: ../../desktop.nix
[shared]: ../../stand-ins/README.md
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
