# The Screens VM Test's Areas

The screens test boots the graphical role with niri straight on its screen, so
screens can really go and come back, as [`screens.nix`][screens] says in its
opening comment. Each area is a check of its own on a machine of its own,
`vm-screens-<area>`, so they run side by side.

- [`checks.nix`][checks]: the areas' checks, by name.
- [`screens-come-and-go-vm-test.py`][come]: the shell's surfaces staying through
  the screens going and coming back, a console switch and a sleep.
- [`screens-keep-places-vm-test.py`][places]: niri starting with the lid shut,
  typing keeping the session awake, and sessions and workspaces going back where
  they sat.
- [`screens-keep-the-launcher-vm-test.py`][launcher]: the launcher working on
  each of two screens as either or both go and come back, and through a sleep.
- [`screens_session.py`][session]: the machine every area drives, and the steps
  it's taken through, which each area loads as a plugin.
- [`niri-software-renderer.patch`][renderer] and
  [`niri-laptop-panel-named.patch`][panel]: what the test's niri takes, to draw
  with no GPU and to know which screen stands in for a laptop's own panel.

[screens]: screens.nix
[checks]: checks.nix
[come]: screens-come-and-go-vm-test.py
[places]: screens-keep-places-vm-test.py
[launcher]: screens-keep-the-launcher-vm-test.py
[session]: screens_session.py
[renderer]: niri-software-renderer.patch
[panel]: niri-laptop-panel-named.patch
