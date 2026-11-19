# The Mycelium Map's Tests

`model.test.ts` proves the map is made right from the node's records, `basins.test.ts`
that a basin's machines are drawn as one stand, `reach.test.ts` how the map judges
whether it reaches every cell, and `keys.test.ts` what each key asks; all four run as
the site builds (`mycelium-site-holds`). `map-reads-live-test.py` proves the built page
reads live through the sites' server in Chromium, from the realm in `node.json`, with
a key of each kind pressed and the map's own page checks in `site-checks.js`
(`mycelium-site-reads-live`).
