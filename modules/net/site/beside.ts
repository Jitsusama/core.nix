// mycelium's cells and machines, watched live from the machine's mycelium
// node through the sites' feed, as mycelium's own map watches them
// (modules/mycelium/site). They're mycelium's, not net's, so a machine in
// no realm has no node to watch: its watches never go live, and a
// machine's panel says nothing of mycelium.

import { follow } from "../../local-sites/kit/feed.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { type Mycelium, myceliumOf } from "./mycelium.ts";

/** What a reading says, so the same reading told again changes nothing. */
const said = (mycelium: Mycelium): string =>
  JSON.stringify(mycelium.machines.map((machine) => ({ ...machine, cells: [...machine.cells] })));

/**
 * Watches the node's cells and machines, telling `heard` each reading that
 * says something new. A cell moves far more often than what a panel says
 * of it, and a panel redrawn under its reader loses their place.
 */
function followMycelium(heard: (mycelium: Mycelium) => void): void {
  const held = new Map<string, Held>();
  let last = "";
  const tell = (): void => {
    const cells = held.get("cell");
    const machines = held.get("machine");
    if (cells === undefined || machines === undefined) {
      return;
    }
    const read = myceliumOf(cells.all(), machines.all());
    const now = said(read);
    if (now !== last) {
      last = now;
      heard(read);
    }
  };
  for (const kind of ["cell", "machine"]) {
    const hearing = { moved: tell, stands: () => undefined, problem: () => undefined };
    held.set(kind, follow({ feed: "node", kind }, hearing).held);
  }
}

export { followMycelium };
