# The Loopback Holders Test's Programs

What [`../loopback-holders.nix`][test] builds to show which programs hold a
loopback camera open, run by their store paths rather than put on the machine's
path. Each is held to ruff and mypy through `nix fmt`.

- [`stand-in-relay.py`][relay]: holds a loopback open as the camera's relay
  does, writing nothing, run as the stand-in camera's service.
- [`renamed.py`][renamed]: takes the name it's given, whatever bytes it holds,
  then holds the camera open, to show how the holders' file names a program.

[test]: ../loopback-holders.nix
[relay]: stand-in-relay.py
[renamed]: renamed.py
