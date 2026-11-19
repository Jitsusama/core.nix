# The Mycelium Map's Tests

`model.test.ts` proves the map is made right from the node's records, `basins.test.ts`
that a basin's machines are drawn as one stand, `fed.test.ts` what the map keeps of
a receptor feeding a cell something, and for how long, `lineage.test.ts` who started
each cell, `reach.test.ts` how the map judges whether it reaches every cell, and
`keys.test.ts` what each key asks, `outline.test.ts` what hangs from what in the
tree beside the map and where its keys go, `words.test.ts` what each cell,
network and machine is in words, `when.test.ts` how the drawer says a time
against the reading's own, `health.test.ts` how each cell and machine stands
and why, `doing.test.ts` what each is doing and how much of the talk asked
something, and `spend.test.ts` what each spent today and how the drawer lists
it; all twelve run as the site builds
(`mycelium-site-holds`). `map-reads-live-test.py` proves the built page reads
live through the sites' server in Chromium, from the realm in `node.json`, with
a key of each kind pressed and the map's own page checks in `site-checks.js`
(`mycelium-site-reads-live`).
