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
| `keys.ts`      | what each key asks of the drawing, decided apart from it    |
| `reach.ts`     | whether the drawing reaches every cell, said when asked     |
| `map-check.sh` | `mycelium-map-check`, asking the page this machine serves   |
| `index.html`   | the page, and `site.css` its styles                         |
| `index.md`     | its Markdown twin, with `llms.txt` for a model              |
| `favicon.svg`  | its icon, tinted by the theme as the page is built          |
| `tests/`       | the making of the map from the records, and the page, tried |

`view.js`, `index.html` and `site.css` are the drawing as it was written
by hand, served unchanged but for two seams: site.ts hands it each reading,
and keys.ts says what each key it hears asks, so those decisions are typed
and unit-tested while the drawing carries them out. They come under the
linters as the drawing is rewritten in TypeScript on the kit, and until
then `treefmt.nix` names them as kept out.

The `tests/` folder's `map-reads-live-test.py` reads the page as Nix builds
it in headless Chromium, through the real server beside a stand-in node holding
`node.json`'s small realm: live, moving, broken and stale, its keys doing
what its keys dialog says, and passing the web-design skill's page checks
(`mycelium-site-reads-live`). It also opens the map with `?reach`, on that
realm and on a twin of 10,000 cells, and holds it to missing none: no cell
of an orchestration or receptor drawn alone, no network named only by its
address, none the map holds nowhere. `live-reach-test.py` asks the same of
the page this machine serves, for `mycelium-map-check`.

[site]: ../../local-sites/site.nix
