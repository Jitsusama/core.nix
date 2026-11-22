# The Forage Page's Tests

The tests of how the forage page reads the mycelium node's forage records,
run by Node's own test runner when the local sites build the page.

- `model.test.ts` tries the records read as cells: each under the cell that
  started it, even across machines or in a ring; documents plans first, with
  their steps added up; what can't be read said while the rest still draws;
  the counts of cells and of those with no purpose written; what waits on
  its person, counted once and said first; the find; the cells each
  one started; a start counted as the node's own only when its start
  book holds it; and a split cell grown under the cell it split from
  when nothing here started it.
- `joins.test.ts` tries the needs each cell serves and relies on, joined
  once a pair however both ends keep them.
- `records.ts` builds the forage records both of those read, as the node
  tells them.
- `draw.test.ts` tries which words the page marks as its quests' own: a
  written purpose and a document's title, never the page's own words for
  what's missing; a row's count of waits and the panel's waits, before
  its documents; and the panel's line of the cells a cell started, each
  linked and escaped, up to seven named, past that six named and the rest
  counted, and no comma left loose; and how a cell joins others: the
  cell it split from and into, each need it serves and relies on in the
  asking cell's words, one dotted arc a pair on the floor, and none of
  it said or drawn while the node sends none; and a branch drawn solid
  only for a start the node's book holds, dashed otherwise, and each tie
  the book doesn't hold said "as claimed" in the panel at either end.
- `floor.test.ts` tries which cells the floor draws of a list longer
  than it holds: the first 48 with no cursor, and otherwise the cell it's
  drawn around, the cells that one started and those nearest it, each
  with every cell it grew from, once round a ring, so a booked start far
  down the list still draws solid; and that a step draws the floor again
  only for a cell it doesn't draw.
- `goes.test.ts` tries where the open cell's panel goes: to the map, and
  to its Talk in the launcher by the link saying the machine it stands
  on, offered only where its address is one the launcher can reach.

Each of those tests builds the records it reads, so none needs a browser
or a node.

`forage-reads-live-test.py` is `forage-site-reads-live`, run by Nix rather
than as the page builds, and needs both: the page Nix builds, read in a
headless browser through the sites' server beside a stand-in node holding
`node.json`, four cells' forage on one machine with a wait, a plan, a
need served, a start the node's book holds and a split. It steps the
cursor by keys, follows the records as they change, stop and come back,
and keeps the page within its budget and passing the web-design skill's
page checks.
