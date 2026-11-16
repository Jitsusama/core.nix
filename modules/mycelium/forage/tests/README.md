# The Forage Page's Tests

The tests of how the forage page reads the mycelium node's forage records,
run by Node's own test runner when the local sites build the page.

- `model.test.ts` tries the records read as cells: each under the cell that
  started it, even across machines or in a ring; documents plans first, with
  their steps added up; what can't be read said while the rest still draws;
  the counts of cells and of those with no purpose written; the find; and
  the cells each one started.
- `draw.test.ts` tries which words the page marks as its quests' own: a
  written purpose and a document's title, never the page's own words for
  what's missing; and the panel's line of the cells a cell started, each
  linked and escaped, six named and the rest counted, no comma left loose.

Each test builds the records it reads, so none needs a browser or a node.
The page as drawn is walked by keys alone in a headless browser until the
local sites' page test covers it.
