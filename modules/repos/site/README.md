# The Code Page

The Code page, served at `http://sites.localhost:8484/code/` and drawn live
from the repos daemon (`repos serve`) through the local sites' `/feed/repos`.
It is built on the local sites' kit by [`site.nix`][site], which leaves this
note behind.

| File          | What it holds                                                  |
| ------------- | -------------------------------------------------------------- |
| `site.ts`     | the page: its watches, its keys and what it says               |
| `model.ts`    | the daemon's records and the mycelium's cells read as a grove  |
| `draw.ts`     | the grove, the repositories as rows, and the one open          |
| `using.ts`    | what works in each repository, and how its use is said         |
| `head.ts`     | the head's lede, its counts first, and its standing line       |
| `address.ts`  | where the address lands, and what it says as the cursor moves  |
| `mycelium.ts` | the cells at work, live from the node, and their homes         |
| `page.ts`     | the page's parts marked and copied from, given what to act on  |
| `index.html`  | the page, and `site.css` its styles                            |
| `grove.css`   | the grove and the marks every part draws                       |
| `drawer.css`  | the repository open beside the list                            |
| `index.md`    | its Markdown twin, with `llms.txt` for a model                 |
| `favicon.svg` | its mark, a tree on its ground                                 |
| `tests/`      | the reading of the records and the address, tried              |

[site]: ../../local-sites/site.nix
