# Google's Other Programs

Two programs built beside google's daemon from its own code, so neither
keeps a copy of it that drifts:

- `google-people.rs` is a People source the people daemon runs as a child
  (../../../people): everyone Joel's meetings and mail name, followed from
  google's daemon socket as it comes and goes.
- `google-rows.rs` makes the rows and drawers a state's meetings make, as
  the daemon serves them, for the desktop tests' stand-in for the daemon,
  [stand-in-google.py](../../../../tests/desktop/stand-ins/stand-in-google.py).
