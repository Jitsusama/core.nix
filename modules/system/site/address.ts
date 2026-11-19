// The system page's own address for a machine, `#machine=<name>`, which
// the launcher and the map link to: the machine it names, and the row the
// cursor lands on for it once the page is drawn.

import { MACHINES_DRAWN } from "./draw.ts";

/** The page's address for a machine, its name still encoded. */
const ASKED = /^#machine=(?<name>.+)$/u;

/** Where a machine asked for lands: the row, and a note when it isn't its own. */
interface Landing {
  readonly row?: string;
  readonly note?: string;
}

/** The machine `hash` asks for, when it is the page's address for one. */
function askedOf(hash: string): string | undefined {
  const name = ASKED.exec(hash)?.groups?.["name"];
  if (name === undefined) {
    return undefined;
  }
  try {
    return decodeURIComponent(name);
  } catch {
    // A name that can't be decoded names no machine the page could have.
    return undefined;
  }
}

/** Where the cursor lands for `name`, among `machines` in the order they are drawn. */
function landingOf(machines: readonly string[], name: string): Landing {
  const at = machines.indexOf(name);
  if (at < 0) {
    return { note: `No machine here is called ${name}.` };
  }
  if (at >= MACHINES_DRAWN) {
    return {
      row: "machines:more",
      note: `${name} isn't among the first ${MACHINES_DRAWN} machines drawn here; the map has it.`,
    };
  }
  return { row: `machine:${name}` };
}

export { askedOf, type Landing, landingOf };
