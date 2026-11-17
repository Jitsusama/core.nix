# The Notices Daemon's Tests

The tests that reach beyond the crate's sources. Cargo runs them as the
daemon builds, in `notices-holds-what-the-bus-says` (../checks.nix).

- `bus.rs` meets the daemon as senders and the shell do: a session bus of
  the test's own, the daemon serving the notifications name on it, and a
  client asking it over its socket.
- `asked.rs` asks the daemon in ways its unit never does (for its help, a
  flag it doesn't know, one it needs left out or left bare, levels it can't
  read), and holds that each ends in the terminal kit's shape, with the
  code docs/commands.md gives it, before it listens.
