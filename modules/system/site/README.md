# The System Page

The system page, served at `http://sites.localhost:8484/system/` and drawn
live from the system daemon through the local sites' `/feed/system`. It is
built on the local sites' kit by [`site.nix`][site], which leaves this note
behind.

| File         | What it holds                                                |
| ------------ | ------------------------------------------------------------ |
| `site.ts`    | the page: its watches, its keys and what it says             |
| `model.ts`   | the daemon's records read as the system's parts and machines |
| `draw.ts`    | the headline, and each part and its rows, drawn              |
| `index.html` | the page, and `site.css` and `keys.css` its styles           |
| `index.md`   | its Markdown twin, with `llms.txt` for a model               |
| `tests/`     | the reading of the records, tried                            |

[site]: ../../local-sites/site.nix
