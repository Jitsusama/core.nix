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
the link new work hands the Launcher, judging addresses by `addresses.txt`; all
twenty run as the site builds (`mycelium-site-holds`). `map-reads-live-test.py` proves the built page reads
live through the sites' server in Chromium, from the realm in `node.json`, with
a key of each kind pressed and the map's own page checks in `site-checks.js`
(`mycelium-site-reads-live`).

## The Address Cases

A session's address on the mesh is 1 to 12 capital letters and digits, in
ASCII. `addresses.txt` holds what is one and what isn't, a case a line: `is` or
`not`, a space, then the text to the end of the line, spaces and all, with
`not` alone the empty text. A line opening with `#` is a note. In the text,
`\u{HEX}` is the character with that code, so a case can hold a tab, a line
break or a space at its end where it can be seen; any other backslash, or a
carriage return anywhere, fails the test reading it.

Three copies of the rule read it in their tests, so none drifts from the
others unseen: the map's `isAddress` (`../sprout.ts`), the notices daemon's
`address_shaped` (`modules/notices/src/notice.rs`) and mycelium-link's
`addressed` (`modules/quickshell/mycelium-link/src/main.rs`). It holds the
bare rule alone, never the `@machine` and `#anchor` a whole cell link adds.

Other copies don't read it yet: the sites' kit (`modules/local-sites/kit/cell.ts`),
the launcher's Start New Work (`launcher/Sprouting.js`) and Explain It
(`launcher/Explaining.js`, which upper-cases first), the harness that sends
the addresses, and mycelium's own, which lets lowercase through. Session rows
and places in the launcher match letters and digits of any length, a looser
rule of their own.

Both Rust packages build with this file in their source, so editing it, its
notes included, rebuilds the notices daemon and mycelium-link; the notes are
kept here so that happens only when a case changes.
