# The Mycelium Map

The realm's map, served at `http://sites.localhost:8484/mycelium/` and drawn
live from the machine's mycelium node through the local sites'
`/feed/node`. It is built on the local sites' kit by [`site.nix`][site],
which leaves this note behind.

| File           | What it holds                                               |
| -------------- | ----------------------------------------------------------- |
| `site.ts`      | the page's watches, handing each reading to the drawing     |
| `model.ts`     | the node's records made into what the map draws             |
| `view.js`      | the drawing, as it was written by hand before the feed      |
| `layout.ts`    | where everything stands, kept in place while the page is up |
| `canvas.ts`    | what the ground layer paints, says, and a pointer touches   |
| `shape.ts`     | a reading's shape, which says when the map draws whole      |
| `keys.ts`      | what each key asks of the drawing, decided apart from it    |
| `outline.ts`   | the tree beside the map: what hangs from what, and its walk |
| `words.ts`     | what each cell, network and machine is, in words            |
| `when.ts`      | each time, said against the reading's own                   |
| `health.ts`    | how each set of cells and each machine stands, and why      |
| `doing.ts`     | what each is doing, and how much of the talk asks something |
| `spend.ts`     | what each spent today, most first, as the drawer lists it   |
| `waits.ts`     | what waits on each cell and what holds stand, in words      |
| `reach.ts`     | whether the drawing reaches every cell, said when asked     |
| `lineage.ts`   | who started each cell, from its forage                      |
| `balance.ts`   | the realm's balance, and each machine's share of the work   |
| `lines.ts`     | how wide a line is drawn as the map steps in and out        |
| `motion.ts`    | how the map's motion is timed, the legend's samples' too    |
| `map-check.sh` | `mycelium-map-check`, asking the page this machine serves   |
| `index.html`   | the page, and `site.css` its styles                         |
| `index.md`     | its Markdown twin, with `llms.txt` for a model              |
| `favicon.svg`  | its icon, tinted by the theme as the page is built          |
| `tests/`       | the making of the map from the records, and the page, tried |

`view.js`, `index.html` and `site.css` are the drawing as it was written
by hand, served unchanged but for its seams: site.ts hands it each reading,
layout.ts says where everything stands, canvas.ts what the ground layer
touches and says, shape.ts when a reading draws the whole map again,
keys.ts what each key it hears asks, outline.ts which rows the tree shows
and where its keys go, words.ts what each thing is in words, when.ts how a
time is said, health.ts how each thing stands, doing.ts what each is
doing, spend.ts what each spent, waits.ts what waits and what holds,
lines.ts how wide a line is drawn and motion.ts how the map's motion is
timed, so those decisions are typed and unit-tested while the drawing
carries them out. They come under the linters as the drawing is rewritten
in TypeScript on the kit, and until then `treefmt.nix` names them as kept
out.

The `tests/` folder's `map-reads-live-test.py` reads the page as Nix builds
it in headless Chromium, through the real server beside a stand-in node holding
`node.json`'s small realm: live, moving, broken and stale, a key of each
kind keys.ts decides answering, and passing the web-design skill's page checks
(`mycelium-site-reads-live`). It also opens the map with `?reach`, on that
realm and on a twin of 10,000 cells, and holds it to missing none: no cell
of an orchestration or receptor drawn alone, no network named only by its
address, none the map holds nowhere. `live-reach-test.py` asks the same of
the page this machine serves, for `mycelium-map-check`.

[site]: ../../local-sites/site.nix
