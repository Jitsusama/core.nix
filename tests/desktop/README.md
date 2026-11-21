# The Desktop VM Test's Areas

The desktop test boots the graphical role's desktop in a virtual machine and
uses it as Joel does, as [`../desktop.nix`][desktop] says in its opening
comment. It's split into areas of a minute or two, each a check of its own,
and this folder holds each area's tests and what the test's machine runs to
drive the desktop. Every file opens with what it is.

| Here                                                | Holds                                                  |
| --------------------------------------------------- | ------------------------------------------------------ |
| [`launcher/`][launcher]                             | the areas named `launcher-*`                           |
| [`looks/`][looks]                                   | looks and the areas named `looks-*`                    |
| `desktop-<area>-vm-test.py`                         | any other area's tests, the check `vm-desktop-<area>`  |
| [`stand-ins/`][stand-ins]                           | what the VM runs in place of a service, device or page |
| [`desktop_session.py`][desktop-session]             | the boot and the helpers every area shares             |
| [`heard.py`][heard], `heard-test.py`                | how a wait on what's reported ends, and its cases      |
| [`reports.sh`][reports]                             | the reader that wait reads the shell's journal through |
| [`reports-tools.nix`][reports-tools]                | the tools that reader runs on, on the VM and off it    |
| [`keeps_pace.py`][keeps-pace], `keeps-pace-test.py` | what keeps-pace counts its builder took, and its cases |
| `checks.nix`                                        | the checks of the areas' own logic, off any VM         |

[desktop]: ../desktop.nix
[launcher]: launcher/README.md
[looks]: looks/README.md
[stand-ins]: stand-ins/README.md
[desktop-session]: desktop_session.py
[heard]: heard.py
[reports]: reports.sh
[reports-tools]: reports-tools.nix
[keeps-pace]: keeps_pace.py
