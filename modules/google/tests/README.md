# Google's Tests

The tests that reach beyond the crate's sources: Cargo runs the Rust ones,
and google's checks (../checks.nix) run the script and the shell's cases.

- `daemon.rs` asks the daemon over its socket as the shell asks it.
- `stand_in/` runs the program's look at the account against a stand-in
  for Google on this machine: the stand-in in `main.rs`, and a file to
  each of signing in, mail, the calendar, Drive and the receptors.
- `people.rs` holds that google-people finds the daemon the moment its
  socket is made.
- `rows.rs` runs google-rows against a meeting told as the daemon tells one.
- `google-opens-its-links.sh` holds that each of a meeting's links opens
  where it should, with a stand-in for each program.
- `mail.qml` runs Google.js and Mail.js in Qt's own engine: what the shell
  reads of the program's records, and how mail is said.
- `calendar.qml` runs Calendar.js, Repeats.js and Meet.js there: which
  meeting is next, how it's said, and which Meet call a window is.
- `drive.qml` runs Drive.js, DrivePlaces.js, Google.js and Saves.js there: the
  documents looked at lately, Drive's own places, and which saves into
  Drive are told.
