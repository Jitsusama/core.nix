# Code's Tests

What proves the Code page, each in the cheapest tier that can show it.

| File                      | Proves                                                 | Run by                 |
| ------------------------- | ------------------------------------------------------ | ---------------------- |
| `model.test.ts`           | the grove drawn from the record: seasons, order, cells | `code-site-holds`      |
| `draw.test.ts`            | every name the record says put in the markup as words  | `code-site-holds`      |
| `head.test.ts`            | what the foot says was read, and what couldn't be      | `code-site-holds`      |
| `listed.test.ts`          | which trees the cursor stood on a drawer keeps listed  | `code-site-holds`      |
| `pace.test.ts`            | how long a draw waits after the last, and its ceiling  | `code-site-holds`      |
| `address.test.ts`         | where an address lands, and what it says as it moves   | `code-site-holds`      |
| `homes.test.ts`           | which site is home to a kind, and where each thing is  | `code-site-holds`      |
| `asking.test.ts`          | when cells' homes are asked again, and the rows read   | `code-site-holds`      |
| `weight.test.ts`          | a git store's weight in words, and when upkeep is due  | `code-site-holds`      |
| `map.test.ts`             | the ties cells make, and the map's rows and threads    | `code-site-holds`      |
| `code-reads-live-test.py` | the page live in Chromium, beside stand-in daemons     | `code-site-reads-live` |
| `repos.json`              | the stand-in repos daemon's records, under test names  | `code-site-reads-live` |
| `node.json`               | the stand-in node's one cell at work                   | `code-site-reads-live` |

The node tests run in Node's own runner, with no browser; the live test serves
the page Nix builds through the real local sites' server
(`modules/local-sites/tests/live_page.py`).
