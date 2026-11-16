// What the network page says of the machines mycelium runs on, from the
// mycelium node's cells and machines as its feed tells them, so a
// machine's panel says what runs there and links to its place on the
// mycelium map. Nothing here touches the page, so each case is proven in
// node.

import { type Fields, number, object, text, texts } from "../../local-sites/kit/records.ts";

/** What waits on a machine, in percent of the last minute. */
interface Pressure {
  readonly cpu: number;
  readonly memory: number;
  readonly io: number;
}

/** A machine as mycelium sees it. */
interface Machine {
  readonly name: string;
  readonly pressure: Pressure | undefined;
  readonly available: number | undefined;
  readonly memory: number | undefined;
  readonly diskFree: number | undefined;
  readonly disk: number | undefined;
  /** What mycelium says it is short of. */
  readonly over: readonly string[];
  /** How many of its cells stand each way, by mycelium's word. */
  readonly cells: ReadonlyMap<string, number>;
}

/** Everything the page reads of mycelium. */
interface Mycelium {
  readonly machines: readonly Machine[];
}

function pressureOf(value: Fields | undefined): Pressure | undefined {
  const cpu = value === undefined ? undefined : number(value, "cpu");
  const memory = value === undefined ? undefined : number(value, "memory");
  const io = value === undefined ? undefined : number(value, "io");
  return cpu === undefined || memory === undefined || io === undefined
    ? undefined
    : { cpu, memory, io };
}

/**
 * Whether a cell runs on a machine: a session or a receptor. A machine is
 * a cell of its own too, and isn't one of those running on it.
 */
const runs = (cell: Fields): boolean => cell.type === "session" || cell.type === "receptor";

/** How many of `running` stand each way on the machine `name`. */
function standingOn(running: readonly Fields[], name: string): Map<string, number> {
  const standing = new Map<string, number>();
  for (const cell of running) {
    const stands = text(cell, "standing");
    if (text(cell, "machine") === name && stands !== undefined) {
      standing.set(stands, (standing.get(stands) ?? 0) + 1);
    }
  }
  return standing;
}

/** A machine record read, with the cells running on it, or none unnamed. */
function machineOf(machine: Fields, running: readonly Fields[]): Machine | undefined {
  const name = text(machine, "name");
  if (name === undefined) {
    return undefined;
  }
  // The node measures a machine's room as one thing, nested.
  const room = object(machine, "room") ?? {};
  return {
    name,
    pressure: pressureOf(object(room, "pressure")),
    available: number(room, "memory_available"),
    memory: number(machine, "memory"),
    diskFree: number(machine, "disk_free"),
    disk: number(machine, "disk"),
    over: texts(room, "over") ?? [],
    cells: standingOn(running, name),
  };
}

/** mycelium as the node's `cell` and `machine` records say it. */
function myceliumOf(cells: readonly Fields[], machines: readonly Fields[]): Mycelium {
  const running = cells.filter(runs);
  return {
    machines: machines.flatMap((machine) => machineOf(machine, running) ?? []),
  };
}

/** The machine's place on the mycelium map, from the sites' root. */
function homeOf(machine: string): string {
  return `mycelium/#machine=${encodeURIComponent(machine)}`;
}

export type { Machine, Mycelium, Pressure };
export { homeOf, myceliumOf };
