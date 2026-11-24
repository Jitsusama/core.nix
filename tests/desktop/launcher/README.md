# The Desktop Test's Launcher Areas

The areas of the desktop test that drive the launcher on the desktop's one
machine, each `desktop-launcher-<area>-vm-test.py` run as the check
`vm-desktop-launcher-<area>`. [`../desktop.nix`][desktop] runs them, and
they share [`../desktop_session.py`][desktop-session] with every other area.
Each file opens with what its area proves.

[desktop]: ../desktop.nix
[desktop-session]: ../desktop_session.py
