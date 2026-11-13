# Daemons

A daemon here is a Rust program that holds one domain's records ready, such
as the machine's figures, people or open programs, and answers anyone who
asks over a socket. The shell draws from what daemons hold rather than
starting a program to learn something, so a daemon answers from memory and
never makes the launcher wait. This page says the shape every daemon takes,
so `ps`, `systemctl` and the socket all say one name, and every client asks
every daemon the same way.

## The Convention

A daemon for the domain `<name>` keeps six points, which the
`daemons-agree` check ([`tests/rules/daemons-agree.py`][check]) holds:

- **Held:** systemd holds its socket in a socket unit, and hands the daemon
  the listener as it starts (`LISTEN_FDS`). Asking while it starts or
  restarts waits on the socket rather than finding nothing there, and asking
  starts it should it have stopped.
- **Placed:** the socket is `%t/daemons/<name>.sock`, the user's runtime
  folder, where every client looks.
- **Named:** its socket unit, its service and its program are all named
  `<name>`, so `systemctl --user status <name>` and `ps` find it by the word
  its socket says.
- **Fits:** the name is at most 15 characters, all of a program's name that
  `ps -e` shows.
- **Shown:** its command line starts `@<program> <name>`, which systemd
  reads as the program to run and the name to run it by, so `ps -ef` reads
  `<name>` first rather than a store path.
- **Speaks:** it serves through [daemon-serve][serve], the one copy of the
  interface below, so no daemon answers in a dialect of its own.

The machine daemon's units ([`modules/machine/home.nix`][machine]) are the
model for the first four: a socket unit with `SocketMode = "0600"` and
`DirectoryMode = "0700"`, wanted by `sockets.target`, and a service that
`Requires` and comes `After` it, restarted by a switch
(`X-SwitchMethod = "restart"`).

A daemon that answers in a protocol another program defines, as the agent
proxy speaks ssh-agent's to ssh and git, keeps that protocol's ways, and the
check names it as such.

One speaks this interface without daemon-serve: pi-verbs, the launcher's AI
actions, is the harness's TypeScript, since the secret scrubber it refuses a
selection with lives there and guards every tool's output too, and a copy in
Rust would drift from it. It keeps the other five points, and the check
names it with that reason.

## The Interface

Version 1, one JSON object a line in UTF-8, either side writing at any time.
Every ask carries an `id`, which each answer to it repeats.

- `hello` answers the version, the domain and its state.
- `get` with `keys` answers the record held for each key.
- `list` with a `kind` answers every record of that kind.
- `watch` with a `kind`, and `keys` or a `place` to narrow it, answers the
  records whole, then tells only what changed, by key, and the keys that
  went. An `every` paces it, telling what moved at most once a pace.
  `unwatch` ends it, as closing the socket does.
- `health` answers the daemon's state, and each of its sources' state with
  its reason, where every other line says the state's word alone.
- `settings`, for a daemon whose rows are made of something the shell
  knows, as Programs' are of the hidden programs Joel lists and Capture's
  of the folders shots land in: each field named replaces the one before
  and is held until the next, and it answers every setting held. The shell
  says it before its watch and again as it changes. It's the daemon's, not
  a watch's, so every watch is told the same rows; with two watchers, the
  last to say a field wins it for both.

A record is `{"key": "<kind>:<id>", "kind": ..., ...}`: an identity key, and
each field as the [glossary][glossary] names it. Records hold facts, never
the words the shell draws them in. An ask the daemon doesn't know answers
`{"id": ..., "error": {"code": "unknown-ask", ...}}`, and a kind it doesn't
hold `unknown-kind`. The shell's side is [`Daemon.qml`][client]; a daemon
that follows another's records uses daemon-serve's `watch` module, and a
program asking one uses [daemon-client][daemon-client].

A daemon that reads only what's asked for, as Programs makes a preview
once it's got, hands daemon-serve a `fresh` that reads what each get, list
or watch names before it's answered, and hands what it read on like any
other change, so no answer says what a watch wasn't told. What else it
counts joins health through `health_too`, never in place of daemon-serve's
own fields, which the check reads alike for every daemon.

## When a Daemon Doesn't Agree Yet

Those that broke a point before the check wait in
[`tests/rules/daemons-agree-waiting.nix`][waiting], each with the points it
still breaks. The list only shrinks: a daemon that keeps a point takes it
off, and a new daemon keeps every point from its first commit.

[check]: ../tests/rules/daemons-agree.py
[serve]: ../modules/daemon-serve/src/lib.rs
[machine]: ../modules/machine/home.nix
[glossary]: glossary.md
[client]: ../modules/quickshell/Daemon.qml
[daemon-client]: ../modules/daemon-client/Cargo.toml
[waiting]: ../tests/rules/daemons-agree-waiting.nix
