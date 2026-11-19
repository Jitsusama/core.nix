// Who started each cell, from its forage (the node's `forage` kind): the
// cell that asked this machine's node to start it, else the orchestrator
// its presence names. Nothing of a forage crosses a link, so a node says
// it only of its own machine's cells, and the map grows one network out
// of another only from those until the cell itself carries it.

import { type Held, object, text } from "../../local-sites/kit/records.ts";

/** A cell and the cell that started it, each by its address. */
interface Started {
  readonly cell: string;
  readonly by: string;
}

/**
 * Who started each cell a forage is held for, in the order of the cells'
 * addresses; none at all when the node serves no forages, so a node too
 * old to serve them reads as one that says nothing of who started whom.
 * A cell said to have started itself names nobody.
 */
function startedOf(held: Held | undefined): Started[] | null {
  if (held === undefined) {
    return null;
  }
  const started: Started[] = [];
  for (const record of held.all()) {
    const cell = text(record, "cell");
    const by = text(object(record, "lineage") ?? {}, "started_by");
    if (cell && by && by !== cell) {
      started.push({ cell, by });
    }
  }
  return started.sort((one, other) => one.cell.localeCompare(other.cell));
}

export type { Started };
export { startedOf };
