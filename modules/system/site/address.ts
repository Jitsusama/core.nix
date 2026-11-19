// The system page's own address for a machine, `#machine=<name>`, which
// the launcher and the map link to: the machine it names, and the row the
// cursor lands on for it once the page is drawn.

import { MACHINES_DRAWN, MORE_ROW, machineRow } from "./draw.ts";

/** The page's address for a machine, its name still encoded. */
const ASKED = /^#machine=(?<name>.+)$/u;

/**
 * What a machine's name may be to be said back: a host's name, as a
 * node gives one. Anything else a link put there is never said in the
 * page's voice, beside the commands it copies.
 */
const SAYABLE = /^[\w.-]{1,63}$/u;

/**
 * Where a machine asked for lands: the row, a note when it isn't its own,
 * or a wait while the records that may name it are still coming.
 */
interface Landing {
  readonly row?: string;
  readonly note?: string;
  readonly waits?: true;
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

/**
 * Where the cursor lands for `name`, among `machines` in the order they are
 * drawn, while records that may name it are `coming` or once all are in.
 */
function landingOf(machines: readonly string[], name: string, coming: boolean): Landing {
  const at = machines.indexOf(name);
  if (at < 0 && coming) {
    return { waits: true };
  }
  if (at < 0) {
    return {
      note: SAYABLE.test(name)
        ? `No machine here is called ${name}.`
        : "No machine here has the name this address gives.",
    };
  }
  if (at >= MACHINES_DRAWN) {
    return {
      row: MORE_ROW,
      note: `${name} isn't among the first ${MACHINES_DRAWN} machines drawn here; the map has it.`,
    };
  }
  return { row: machineRow(name) };
}

export { askedOf, type Landing, landingOf };
