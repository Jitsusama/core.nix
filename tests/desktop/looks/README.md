# The Desktop Test's Looks Areas

The areas of the desktop test named looks, each `desktop-looks-<x>-vm-test.py`,
run as the check `vm-desktop-looks-<x>`, and looks itself,
`desktop-looks-vm-test.py`, run as `vm-desktop-looks`.
[`../desktop.nix`][desktop] runs them, and finds them here by their names.
Five measure the launcher at Joel's scales, side by side: looks itself,
looks-rows, looks-vi, looks-plates and looks-file-picker. They're desktop.nix's
`looksAreas`, whose machines bring what looks brought (WARP, the node, a
stand-in for the screen's text and more) and
[`looks_session.py`][looks-session], which they import what they share from.
looks-machine and looks-programs aren't among them, so their machines have no
looks_session.py, WARP or screen-text stand-in, and run the real screen-text
daemon; looks-machine still runs the node. Each file opens with what its area
proves.

[desktop]: ../desktop.nix
[looks-session]: looks_session.py
