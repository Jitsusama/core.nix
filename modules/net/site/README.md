# The Network Page

The home network's page, served at `http://sites.localhost:8484/network/`
and drawn live from net's daemon through the local sites' `/feed/net`. It
is built on the local sites' kit by [`site.nix`][site], which leaves this
note behind.

| File          | What it holds                                                  |
| ------------- | -------------------------------------------------------------- |
| `site.ts`     | the page: its watches, its keys, and what's open               |
| `model.ts`    | net's records read back as the network they describe           |
| `shape.ts`    | the network's shape: floors, who hangs from whom, the gateway  |
| `words.ts`    | how figures and devices are said                               |
| `map.ts`      | the floors drawn as SVG, stacked on a phone, with `wires.ts`   |
| `svg.ts`      | drawing SVG: elements, words fitted to room, the look mark     |
| `view.ts`     | the map's zoom, its edges, and walking it by arrow             |
| `elements.ts` | the panel's HTML: facts, tables and the look mark              |
| `panel.ts`    | the panel when nothing is open, and what a search finds        |
| `device.ts`   | one device in full, with `cable.ts` for the modem's channels   |
| `machine.ts`  | a machine as mycelium sees it, in its panel                    |
| `mycelium.ts` | the mycelium node's cells and machines, as `beside.ts` watches |
| `tree.ts`     | every device as a tree, walked by its own keys                 |
| `where.ts`    | what's said as the keys land, and how an address names one     |
| `index.html`  | the page, with `site.css`, `map.css` and `panel.css` its styles |
| `index.md`    | its Markdown twin, with `llms.txt` for a model                 |
| `tests/`      | the reading of the records, the words and the view, tried      |

In `tests/`, `network-reads-live-test.py` is `network-site-reads-live`: the page
Nix builds, read in a headless browser through the sites' server beside a
stand-in net holding `net.json` and a stand-in node holding `node.json`, as
the records change, stop and come back, every device within a phone's width,
within its budget and passing the web-design skill's page checks.

[site]: ../../local-sites/site.nix
