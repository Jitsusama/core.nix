// The system page's own address for a machine, `#machine=<name>`, which
// the launcher and the map link to: the machine it names, and the row the
// cursor lands on for it once the page is drawn.

import { MACHINES_DRAWN, MORE_ROW, machineRow } from "./draw.ts";
import type { System } from "./model.ts";

/** The page's address for a machine, its name still encoded. */
const ASKED = /^#machine=(?<name>.+)$/u;

/**
 * What a machine's name may be to be said back: a host's name, as a
 * node gives one. Anything else a link put there is never said in the
 * page's voice, beside the commands it copies.
 */
const SAYABLE = /^[\w.-]{1,63}$/u;

/**
 * How the records that may name a machine stand: some still coming, some
 * still coming past the wait, or all in.
 */
type Records = "coming" | "overdue" | "in";

/** What the page has heard of its machine watch. */
interface Watch {
  readonly told: boolean;
  readonly state: string | undefined;
}

/** What a note adds when the records it was waiting for haven't come. */
const BEHIND = "some machines' records haven't come.";

/**
 * How the records that may name a machine stand in `read`: coming while
 * the machine `watch` hasn't told them or isn't live, while the machines
 * part is untold, or while it names a machine whose record hasn't come,
 * as a new machine's can be one push behind its part's; `overdue` once
 * the page has waited long enough for them.
 */
function recordsOf(read: System, watch: Watch, overdue: boolean): Records {
  const machines = read.parts.find((part) => part.part === "machines");
  const coming =
    !watch.told ||
    watch.state !== "live" ||
    read.untold.includes("machines") ||
    (machines?.untold ?? 0) > 0;
  if (!coming) {
    return "in";
  }
  return overdue ? "overdue" : "coming";
}

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
 * drawn, as the `records` that may name it stand.
 */
function landingOf(machines: readonly string[], name: string, records: Records): Landing {
  const at = machines.indexOf(name);
  if (at < 0 && records === "coming") {
    return { waits: true };
  }
  if (at < 0 && records === "overdue") {
    return {
      note: SAYABLE.test(name)
        ? `No machine read here yet is called ${name}; ${BEHIND}`
        : `No machine read here yet has the name this address gives; ${BEHIND}`,
    };
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

export { askedOf, type Landing, landingOf, type Records, recordsOf, type Watch };
