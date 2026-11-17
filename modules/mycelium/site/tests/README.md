# The Mycelium Map's Tests

`model.test.ts` proves the map is made right from the node's records, and runs as the
site builds (`mycelium-site-holds`). `map-reads-live-test.py` proves the built page reads
live through the sites' server in Chromium, from the realm in `node.json`, with the map's
own page checks in `site-checks.js` (`mycelium-site-reads-live`).
