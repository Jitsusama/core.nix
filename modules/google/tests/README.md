# Google's Tests

The tests that reach beyond the crate's sources: Cargo runs the Rust ones,
and google's checks (../checks.nix) run the script.

- `daemon.rs` asks the daemon over its socket as the shell asks it.
- `stand_in/` runs the program's look at the account against a stand-in
  for Google on this machine: the stand-in in `main.rs`, and a file to
  each of signing in, mail, the calendar, Drive and the receptors.
- `people.rs` holds that google-people finds the daemon the moment its
  socket is made.
- `rows.rs` runs google-rows against a meeting told as the daemon tells one.
- `google-opens-its-links.sh` holds that each of a meeting's links opens
  where it should, with a stand-in for each program.
