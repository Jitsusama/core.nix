# The IR Camera Test's Programs

What [`../ir-camera.nix`][test] installs on its machine to show the IR camera is
relayed only to the programs reading it. Each is held to ruff and mypy through
`nix fmt`, and takes the frames' size from the test's values.

- [`stand-in-camera.py`][camera]: stands in for the IR camera, sending a known
  pattern to a loopback over and over, and pausing as a relay waiting for its
  next reader does.
- [`frames.py`][frames]: names each frame a reader saved, as the camera's
  pattern in grey, blank, or other, so the test can say what each reader got.

[test]: ../ir-camera.nix
[camera]: stand-in-camera.py
[frames]: frames.py
