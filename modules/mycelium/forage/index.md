# What Each Cell Is For

Every cell standing on this machine that works on a quest, with the
purpose its quest states, the cell that started it, and how far the plans,
research, briefs and reports it keeps have come. The page watches the
mycelium node's forage records live over the sites' server, and redraws
a cell the moment a step is ticked or a cell comes or goes. Nothing on
it changes anything.

A feed reaches one node, so the page shows the cells on this machine. In
a terminal, `mycelium forage` gathers the realm's.

## How to Read It

The page opens with the head every live page shares: what it shows and
where, one sentence for the whole, whether it's live, and the keys. The
floor is drawn under it, across the page, then the list with the cell
open beside it.

- **The floor, cut through:** the ground line, the hyphae under it and
  each cell a compartment along them. A cell with its purpose written is
  whole; one without is hollow, told by its shape. A cell started by
  another, or split from it, branches down from it: solid when the node's
  own start book holds the start, dashed when it's only claimed. No book
  holds a split, so the panel calls every split claimed, even one whose
  branch is solid as a booked start. The panel says "as claimed" of each
  tie the book doesn't hold, at either end, and before a list when all of
  it is. A dotted arc, bowing below both, joins a cell to one serving a
  need of its. A cap rises over a cell whose plans
  have landed steps, its gills lit as they land. The machine the cells
  stand on is named under the ground. It draws up to 48 cells, and the
  list holds every one. Past 48 it's drawn around a cell, the one you're
  on until you step to another it draws: that cell, as many of the cells
  that grew straight from it as fit (those the node's book holds first) and those
  nearest it in the list, each with every cell it grew from, so no
  branch is left hanging. Only that cell's own line is cut at its top,
  whenever it's longer than the room left once the cells the node's book
  holds as grown from it have theirs, up to half the floor, so even a
  line shorter than 48 is cut when enough of them grew from its cell.
  Cells that only claim to have grown from it take what room is left.
  Another line longer than the room left is left off, unless none fits,
  as in a ring of starts longer than the room, when the
  floor is filled with lines cut at their tops. Where it skips cells of
  the list, before, between or after those it draws, the main line breaks
  at an ellipsis, unless all it skips between two is the rest of the
  first's brood, the second not of it: those are folded under the first,
  so a brood drawn apart by its own broods reads as one row. A ring's ties
  never count for it. A list that grows past 48 is drawn around the cell you're on. As
  records arrive over a list past 48, or a find narrows the list, it
  stays drawn around the same cell, though a cell added or ended near it
  shifts the cells beside it. It draws again around the cell you're on when it
  leaves that cell off, or a cell that grew from it, unless it's drawn
  around that cell already, and when the cell it's drawn around ends.
  A find that hides the cell you're on moves you to its first match, and
  the floor follows as it does a step; a find that matches nothing keeps
  you, and the floor, where you were. If your cell ends meanwhile, nothing
  is ringed, and the floor stays put unless it was drawn around that cell,
  when it goes back to the list's first 48. Opening the cells without
  purpose while that find holds you on a cell with one lets go of it too:
  nothing is ringed, and the floor stays where it was, since the cell
  still lasts. Once every cell has gone, the floor holds the page's crest
  again, until a cell is heard again.
- **One sentence for the whole:** how many cells here work on a quest,
  and how many have no purpose written yet. Each count opens the cells
  it counts. The line under it first says how many things wait on you
  and from which cells, then names the cells without purpose and the
  plan furthest from done. It names two cells at most and counts the
  rest, so it keeps to the lines it opens on: four on a phone, three
  at a desk. A cell with no purpose is the one worth a look, since a
  cell without one serves no use.
- **One row per cell,** each set in under the cell the records say it
  was started by or split from. The set-in says only where a row
  stands; the floor and the panel say which starts the node's book
  holds. Each row shows its mark, its address, how many of its steps
  are done, its purpose, and what it keeps, such as "2 plans and one report", and how many things
  its quest waits on you for, when any. A cell started by, or split
  from, one not standing here stands at the top, since that cell isn't
  drawn here.
- **Beside the list is the cell the cursor is on:** its purpose, then
  what its quest waits on you for, each with why, about how long it
  takes, when it's for and whether it's ready now, then each
  document it keeps with its kind, its stage (Thinking, Drafting,
  Building, Done or Put down) and how many of its steps are done, said
  in words beside gills. Under them, where it grows: its quest, the cell
  that started it, the cells it started, the cell it split from and the
  cells it split into, the needs it serves and relies on, each with the
  cell at its other end, what it was about and since when, the quest
  above its own, the machine it stands on and the command that says its forage in a
  terminal. Each cell named there opens when pressed, and a link under it talks to
  the cell in the Launcher.
- **A cell opens by its link too:** the page opened at `#cell=` with a cell's whole
  link, as a notice gives it, or its address on its machine, opens on that cell.
- **Under the sentence, whether what it shows is live:** when the node
  or the sites' server can't be reached, it says since when and why,
  and keeps showing how things stood then.

## Keys

| Key       | Does                                                          |
| --------- | ------------------------------------------------------------- |
| j, k      | Step to the next or the last cell                             |
| g, G      | Go to the first or the last                                   |
| -, h      | Step up to the cell that started the one you're on, or it split from |
| /         | Find by name, quest, purpose, or a document's or wait's title |
| Return    | Open the cell on the map                                      |
| y         | Copy the command that says the cell's forage                  |
| ?, F1     | The list of keys                                              |

## What the Data Holds

The node serves one `forage` record for each of its live cells that
works on a quest, keyed `forage:<cell>` and gone with the cell. Each
has `cell` (its address), `session`, `machine`, `quest` (its id),
`purpose` (cut at 280 characters), `status`, `documents`, `waits`
(only when its quest waits on its person), `lineage`, and `serves` and
`relies` (only when it serves or relies on a need). Each wait
has `id` and `title`, and may have `why`, `minutes`, `when`, `ready`
and `raised`. Each document has `id`, `kind` (plan, research, brief or
report), `title`, `stage` (think, draft, build, concluded or retired),
`ticked` and `open`, its steps done and still to do. The `lineage`
has `started_by`, the cell that started this one, `started_by_from`,
`book` when the serving node's start book holds that start or
`presence` when it's the cell's own word, which the node works out from
its own book and never takes from a presence (any other word, or none
sent beside a starter, is said among what couldn't be read), `parent`, the
quest above its own, and, when its purpose was split, `split_from`, the
cell it split from, or `split_into`, the cells it split into as it
ended. Each of `serves` (needs it took from other cells) and `relies`
(needs it handed other cells) is a list of `cell`, the cell at the
other end, `need`, what the need was about, and `since`, an ISO time.
The page draws each only once the node sends it.
