# Code's Tests

What proves the Code page, each in the cheapest tier that can show it.

| File                      | Proves                                                 | Run by                 |
| ------------------------- | ------------------------------------------------------ | ---------------------- |
| `model.test.ts`           | the grove drawn from the record: seasons, order, cells | `code-site-holds`      |
| `address.test.ts`         | where an address lands, and which site is home to what | `code-site-holds`      |
| `weight.test.ts`          | a git store's weight in words, and when upkeep is due  | `code-site-holds`      |
| `code-reads-live-test.py` | the page live in Chromium, beside stand-in daemons     | `code-site-reads-live` |
| `repos.json`              | the stand-in repos daemon's records, under test names  | `code-site-reads-live` |
| `node.json`               | the stand-in node's one cell at work                   | `code-site-reads-live` |

The node tests run in Node's own runner, with no browser; the live test serves
the page Nix builds through the real local sites' server
(`modules/local-sites/tests/live_page.py`).
