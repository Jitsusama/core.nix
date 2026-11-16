# What Each Cell Is For

Every cell on this machine that works on a quest, with the purpose its
quest states, the cell that started it, and how far the plans, research,
briefs and reports it keeps have come. The page watches the mycelium
node's forage records live over the sites' server, and redraws a cell
the moment a step is ticked or a cell comes or goes. Nothing on it
changes anything.

A feed reaches one node, so the page shows this machine's cells. In a
terminal, `mycelium forage` gathers every machine's.

## How to Read It

- **One sentence for the whole:** how many cells here work on a quest,
  and how many of them have no purpose written yet. A cell with no
  purpose is the one worth a look, since a cell without one serves no
  use.
- **One row per cell,** each set in under the cell that started it, as
  a tree's branches are: its address, how many of its steps are done,
  its purpose, and what it keeps, such as "2 plans and one report". A
  cell started by one on another machine stands at the top, since its
  starter isn't drawn here.
- **Beside the list is the cell the cursor is on:** its purpose, each
  document it keeps with its kind, its stage (Thinking, Drafting,
  Building, Done or Put down) and how many of its steps are done, said
  in words beside a bar. Under them, where it sits: its quest, the cell
  that started it, the quest above its own, the machine it runs on and
  the command that says its forage in a terminal.
- **Under the list, whether what it shows is live:** when the node or
  the sites' server can't be reached, it says since when and why, and
  keeps showing how things stood then.

## Keys

| Key       | Does                                                          |
| --------- | ------------------------------------------------------------- |
| j, k      | Step to the next or the last cell                             |
| g, G      | Go to the first or the last                                   |
| -, h      | Step up to the cell that started the one you're on            |
| /         | Find a cell by its name, quest, purpose or a document's title |
| Return    | Open the cell on the map                                      |
| y         | Copy the command that says the cell's forage                  |
| ?, F1     | The list of keys                                              |

## What the Data Holds

The node serves one `forage` record for each of its live cells that
works on a quest, keyed `forage:<cell>` and gone with the cell. Each
has `cell` (its address), `session`, `machine`, `quest` (its id),
`purpose` (cut at 280 characters), `status`, `documents` and
`lineage`. Each document has `id`, `kind` (plan, research, brief or
report), `title`, `stage` (think, draft, build, concluded or retired),
`ticked` and `open`, its steps done and still to do. The `lineage`
has `started_by`, the cell that started this one, and `parent`, the
quest above its own.
