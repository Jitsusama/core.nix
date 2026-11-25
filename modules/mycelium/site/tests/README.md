# The Mycelium Map's Tests

`model.test.ts` proves the map is made right from the node's records, `builders.test.ts`
that the builders are read from their own records and never as cells, `basins.test.ts`
that a basin's machines are drawn as one stand, `fed.test.ts` what the map keeps of
a receptor feeding a cell something, and for how long, `lineage.test.ts` who started
each cell, `reach.test.ts` how the map judges whether it reaches every cell, and
`keys.test.ts` what each key asks, `outline.test.ts` what hangs from what in the
tree beside the map and where its keys go, `words.test.ts` what each cell,
network and machine is in words, `when.test.ts` how the drawer says a time
against the reading's own, `health.test.ts` how each cell and machine stands
and why, `doing.test.ts` what each is doing and how much of the talk asked
something, `spend.test.ts` what each spent today and how the drawer lists it,
`waits.test.ts` what waits on each cell and what holds stand, `lines.test.ts`
how wide a line is drawn at each zoom, `motion.test.ts` how the map's motion is
timed, `layout.test.ts` where everything stands and that it stays there,
`canvas.test.ts` what the ground layer says and which line a pointer touches,
`shape.test.ts` when a reading draws the whole map again, and `sprout.test.ts`
the link new work hands the Launcher, judging addresses by `addresses.txt`, and
`acts.test.ts` what a cell's drawer offers, Go to It first, and its question
said whole; all twenty-one run as the site builds (`mycelium-site-holds`). `map-reads-live-test.py`
proves the built page reads live through the sites' server in Chromium, from
the realm in `node.json`, with a key of each kind pressed and the map's own
page checks in `site-checks.js` (`mycelium-site-reads-live`).

## The Address Cases

A session's address on the mesh is 1 to 12 capital letters and digits, in
ASCII. `addresses.txt` holds what is one and what isn't, a case a line: `is` or
`not`, a space, then the text to the end of the line, spaces and all, with
`not` alone the empty text. A line opening with `#` is a note. In the text,
`\u{HEX}` is the character with that code, written as 1 to 6 capital hex
digits naming a character (never a surrogate, D800 to DFFF), so a case can
hold a tab, a line break, a backslash (`\u{5C}`) or a space at its end where
it can be seen. Any other backslash, or a carriage return anywhere, fails
each test reading it, and each reader has a case seeing it refuse them.

Four copies of the rule read it in their tests, so none drifts from the
others unseen: the map's `isAddress` (`../sprout.ts`), the notices daemon's
`address_shaped` (`modules/notices/src/notice.rs`), mycelium-link's
`addressed` (`modules/quickshell/mycelium-link/src/main.rs`) and the
launcher's `shaped` (`modules/quickshell/launcher/Address.js`), which Start
New Work and Explain It read, Explain It upper-casing first. The launcher's
reader is its own, in `modules/quickshell/tests/address.qml`, since Qt's
engine takes in no TypeScript. The two Rust
copies share one reader of it, `addresses.rs` beside it, which their tests
take in with `include!`. It holds the bare rule alone, never the `@machine`
and `#anchor` a whole cell link adds.

Other copies don't read it yet. A grep under `modules/` for the `[A-Z0-9]`
patterns and `is_ascii_uppercase` found these, so there may be more:

- The same rule: the sites' kit (`modules/local-sites/kit/cell.ts`).
- Looser, any length: For You's reader of the notices daemon's cell links
  (`modules/quickshell/launcher/ForYouCells.js`, which takes either case and
  upper-cases it), session rows (`modules/quickshell/launcher/SessionRow.js`),
  places (`modules/quickshell/Places.js`), the sessions list's orchestrator
  and workspace (`modules/quickshell/Sessions.qml`), and cadence's sender of a
  hold (`modules/cadence/src/watch/held.rs`, after trimming and dropping an
  `@`).
- Stricter, exactly 6: rhythm's reading of a session's message
  (`modules/rhythm/src/seed.rs`), so a message from a shorter or longer
  address counts there as typed by Joel; and nix-offload's gate
  (`plain_session` in `modules/nix-offload/offload/src/gate.rs`), which
  refuses a gate request, and `nix-offload ask --session`, from any other
  length as no mesh address.

Outside this tree, the harness that sends the addresses and mycelium hold
copies too.

Three builds read this file. The notices daemon and mycelium-link hold it in
their source, and only graphical NixOS machines build them; the map site
builds this whole folder and runs these tests, on any machine with local
sites in a realm. So editing the file, its notes included, rebuilds all
three, and the next switch restarts the notices daemon, which is safe since
the bus holds notices meanwhile. A bad line fails all three, so those
machines' switches fail until it is reverted. The launcher's check
(`the-launcher-reads-the-address-cases`) reads it too, and fails with them,
though no switch builds it. The notes kept in this README spare the two Rust
packages a rebuild, though not the map site.
