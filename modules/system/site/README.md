# The System Page

The system page, served at `http://sites.localhost:8484/system/` and drawn
live from the system daemon through the local sites' `/feed/system`. It is
built on the local sites' kit by [`site.nix`][site], which leaves this note
behind.

| File         | What it holds                                                |
| ------------ | ------------------------------------------------------------ |
| `site.ts`    | the page: its watches, its keys and what it says             |
| `model.ts`   | the daemon's records read as the system's parts and machines |
| `ties.ts`    | a machine's ties to the other parts, read from its record    |
| `machine.ts` | the words each machine and its ties are said in              |
| `bridge.ts`  | the machines drawn as the piers of a bridge                  |
| `draw.ts`    | the marks, the headline, each machine and the parts, drawn   |
| `index.html` | the page; `site.css`, `parts.css` and `keys.css` its styles  |
| `index.md`   | its Markdown twin, with `llms.txt` for a model               |
| `tests/`     | the reading tried, and the page read live in Chromium        |

[site]: ../../local-sites/site.nix
