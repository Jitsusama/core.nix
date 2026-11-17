# The Murmurs Page

The murmurs page, served at `http://sites.localhost:8484/web/` and drawn
live from the mycelium node's murmur records through the local sites'
`/feed/node`. It shows what the quests' sessions murmured into their
webs over the last day: the words, the cell that said each, the quests it is tied to and
how it was settled. It is built on the local sites' kit by
[`site.nix`][site], which leaves this note behind, and served at
`/web/`, the address the home has always given it.

| File         | What it holds                                             |
| ------------ | --------------------------------------------------------- |
| `site.ts`    | the page: its watch, its keys and what it says            |
| `model.ts`   | the murmur records read, and the sentence saying them     |
| `draw.ts`    | the flock over its wire, and the rows                     |
| `index.html` | the page, and `site.css` its styles                       |
| `index.md`   | its Markdown twin, with `llms.txt` for a model            |
| `tests/`     | the reading of the records, and the page read live, tried |

[site]: ../../local-sites/site.nix
