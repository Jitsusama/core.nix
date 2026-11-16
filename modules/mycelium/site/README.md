# The Mycelium Map

The realm's map, served at `http://sites.localhost:8484/mycelium/` and drawn
live from the machine's mycelium node through the local sites'
`/feed/node`. It is built on the local sites' kit by [`site.nix`][site],
which leaves this note behind.

| File          | What it holds                                                   |
| ------------- | --------------------------------------------------------------- |
| `site.ts`     | the page's watches, handing each reading to the drawing         |
| `model.ts`    | the node's records made into what the map draws                 |
| `view.js`     | the drawing, as it was written by hand before the feed          |
| `index.html`  | the page, and `site.css` its styles                             |
| `index.md`    | its Markdown twin, with `llms.txt` for a model                  |
| `favicon.svg` | its icon, tinted by the theme as the page is built              |
| `tests/`      | the making of the map from the records, and the page, tried     |

`view.js`, `index.html` and `site.css` are the drawing as it was written
by hand, served unchanged but for the seam where site.ts hands it each
reading. They come under the linters as the drawing is rewritten in
TypeScript on the kit, and until then `treefmt.nix` names them as kept
out.

The `tests/` folder's `map-reads-live-test.py` reads the page as Nix builds
it in headless Chromium, through the real server beside a stand-in node holding
`node.json`'s small realm: live, moving, broken and stale, and passing the
web-design skill's page checks (`mycelium-site-reads-live`).

[site]: ../../local-sites/site.nix
