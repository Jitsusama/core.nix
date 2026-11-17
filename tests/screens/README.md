# The Screens VM Test's Areas

The screens test boots the graphical role with niri straight on its screen, so
screens can really go and come back, as [`../screens.nix`][screens] says in
its opening comment. Each area is a check of its own on a machine of its own,
`vm-screens-<area>`, so they run side by side.

- `screens-come-and-go-vm-test.py`: the shell's surfaces staying through the
  screens going and coming back, a console switch and a sleep.
- `screens-keep-places-vm-test.py`: niri starting with the lid shut, typing
  keeping the session awake, and sessions and workspaces going back where
  they sat.
- `screens-keep-the-launcher-vm-test.py`: the launcher working on each of two
  screens as either or both go and come back, and through a sleep.
- `screens_session.py`: the machine every area drives, and the steps it's
  taken through, which each area loads as a plugin.

[screens]: ../screens.nix
