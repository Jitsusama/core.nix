# The Reader's Scripted Tests

The scripts the reader's checks run (../checks.nix), beside the Rust tests
that live with its sources:

- `reader-takes-its-keys.sh` runs inside cage: it opens a long page, types
  Neovim's keys through wtype, reads where each took the view from the
  reader's key log and leaves `./passed` once every key did what it should.
- `presses.sh` is how it presses those keys and reads the log, sourced by
  it, so the script itself is the steps and what each should do.
- `places-list.sh` stands in for `reader --places`, printing the places a
  reader was left at, newest first, for the launcher's check that reads
  them.
