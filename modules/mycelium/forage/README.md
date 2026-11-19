# The Forage Page

The forage page, served at `http://sites.localhost:8484/forage/` and drawn
live from the mycelium node's forage records through the local sites'
`/feed/node`. It shows what each of this machine's cells is for: the
purpose its quest states, the cell that started it and how far the plans,
research, briefs and reports it keeps have come. It is built on the local
sites' kit by [`site.nix`][site], which leaves this note behind.

| File         | What it holds                                                   |
| ------------ | --------------------------------------------------------------- |
| `site.ts`    | the page: its watch, its keys and what it says                  |
| `model.ts`   | the forage records read as cells, and the sentence saying them  |
| `joins.ts`   | how forages join: needs shared, cells split and starts booked   |
| `draw.ts`    | the floor cut through, the rows, and the cell open as its panel |
| `index.html` | the page, and `site.css` and `panel.css` its styles             |
| `index.md`   | its Markdown twin, with `llms.txt` for a model                  |
| `tests/`     | the reading of the records, tried                               |

[site]: ../../local-sites/site.nix
