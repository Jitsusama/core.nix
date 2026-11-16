// What the network page says of the machines mycelium runs on, read from
// mycelium's own files beside it (its now.json and homes.json), so a
// machine's panel says what runs there and links to its home. Nothing here
// touches the page, so each case is proven in node.

import { isObject, number, object, text, texts } from "../../local-sites/kit/records.ts";

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
  /** Each machine's home address from the sites' root, by its name. */
  readonly homes: ReadonlyMap<string, string>;
}

const objects = (from: unknown, field: string): readonly { readonly [key: string]: unknown }[] => {
  const value = isObject(from) ? from[field] : undefined;
  return Array.isArray(value) ? value.filter(isObject) : [];
};

function pressureOf(
  value: { readonly [field: string]: unknown } | undefined,
): Pressure | undefined {
  const cpu = value === undefined ? undefined : number(value, "cpu");
  const memory = value === undefined ? undefined : number(value, "memory");
  const io = value === undefined ? undefined : number(value, "io");
  return cpu === undefined || memory === undefined || io === undefined
    ? undefined
    : { cpu, memory, io };
}

/** mycelium as its two files say it, either of which may be missing. */
function myceliumOf(now: unknown, homes: unknown): Mycelium {
  const cells = objects(now, "cells");
  const machines = objects(now, "machines").flatMap((machine) => {
    const name = text(machine, "name");
    if (name === undefined) {
      return [];
    }
    const standing = new Map<string, number>();
    for (const cell of cells) {
      const stands = text(cell, "standing");
      if (text(cell, "machine") === name && stands !== undefined) {
        standing.set(stands, (standing.get(stands) ?? 0) + 1);
      }
    }
    return [
      {
        name,
        pressure: pressureOf(object(machine, "pressure")),
        available: number(machine, "available"),
        memory: number(machine, "memory"),
        diskFree: number(machine, "disk_free"),
        disk: number(machine, "disk"),
        over: texts(machine, "over") ?? [],
        cells: standing,
      },
    ];
  });
  const homed = new Map<string, string>();
  for (const thing of objects(homes, "things")) {
    const key = text(thing, "key");
    const address = text(thing, "address");
    if (text(thing, "kind") === "machine" && key !== undefined && address !== undefined) {
      homed.set(key, address);
    }
  }
  return { machines, homes: homed };
}

/**
 * The machine's home as mycelium lists it, or until mycelium writes its
 * homes.json, the address its page answers to today.
 */
function homeOf(mycelium: Mycelium, machine: string): string {
  return mycelium.homes.get(machine) ?? `mycelium/#machine=${encodeURIComponent(machine)}`;
}

export type { Machine, Mycelium, Pressure };
export { homeOf, myceliumOf };
