// What is worth a look, across every machine and every part, as the list
// the system page opens with: the page reads top to bottom, what wants
// Joel first and then each machine (Joel, 2026-11-17). Each look is one
// of the daemon's own, a machine's or a part's side, so the page judges
// nothing the `system` command doesn't; each says whose it is and leads
// to where it can be seen whole. Nothing here touches the page.

import { capital, type Health, named, type Part, type System } from "./model.ts";

/** The most looks listed, so a bad night keeps the list a glance. */
const LOOKS_LISTED = 8;

/** How each health ranks in the list, the worst first. */
const RANK: { readonly [health in Health]: number } = { bad: 0, watch: 1, unread: 2, good: 3 };

/** One thing worth a look: how bad, whose it is, what it is and where it leads. */
interface Look {
  readonly health: Health;
  /** The machine or part it is in, as the page names it. */
  readonly whose: string;
  /** Whether `whose` is a machine's name, which no translator may change. */
  readonly machine: boolean;
  readonly words: string;
  /** Where it is seen whole: a machine's section here, or a part's own site. */
  readonly href: string;
  /** The row's key, the same across redraws while the look stands. */
  readonly row: string;
}

/** The list: the looks listed, how many more there are, and whether it's settled. */
interface Looks {
  readonly looks: readonly Look[];
  /** How many looks past those listed. */
  readonly more: number;
  /**
   * Whether every part has told, so an empty list says nothing wants a
   * look; one still to come may be the one with a look in it.
   */
  readonly settled: boolean;
}

/** A part's own site, from the sites' root as the daemon says it. */
function siteOf(part: Part): string {
  return `../${part.deeper.site}`;
}

/**
 * The machines' looks: each machine's side, a look a word, or why the
 * machines couldn't be read. A machine drawn whole on the page leads to its
 * section, its position among them told by `drawn`; one past those leads
 * to the map.
 */
function machineLooks(part: Part, drawn: number): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, 0)];
  }
  return part.things.flatMap((thing, at) =>
    thing.side.map((words, item) => ({
      health: thing.health,
      whose: thing.name,
      machine: true,
      words: capital(words),
      href:
        at < drawn
          ? `#machine-${at + 1}`
          : `../mycelium/#machine=${encodeURIComponent(thing.name)}`,
      row: `look:machine:${thing.name}:${item}`,
    })),
  );
}

/** One look in the part `part`, the `item`th of its own. */
function lookOf(part: Part, health: Health, words: string, item: number): Look {
  return {
    health,
    whose: named(part.part),
    machine: false,
    words: capital(words),
    href: siteOf(part),
    row: `look:part:${part.part}:${item}`,
  };
}

/** Another part's looks: each of its side's, or why it couldn't be read. */
function partLooks(part: Part): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, 0)];
  }
  if (part.health === "good") {
    return [];
  }
  return part.side.map((words, item) => lookOf(part, part.health, words, item));
}

/**
 * Everything worth a look in `system`, the worst first and in the daemon's
 * own order within a health, `drawn` of its machines drawn whole on the
 * page. Past what the list holds, the rest are counted.
 */
function looksOf(system: System, drawn: number): Looks {
  const all = system.parts
    .flatMap((part) => (part.part === "machines" ? machineLooks(part, drawn) : partLooks(part)))
    .map((look, at) => ({ look, at }))
    .sort((one, other) => RANK[one.look.health] - RANK[other.look.health] || one.at - other.at)
    .map(({ look }) => look);
  return {
    looks: all.slice(0, LOOKS_LISTED),
    more: Math.max(0, all.length - LOOKS_LISTED),
    // A part still to come, or a machine named before its record, may hold
    // the one look there is.
    settled: system.untold.length === 0 && system.parts.every((part) => part.untold === 0),
  };
}

export type { Look, Looks };
export { LOOKS_LISTED, looksOf };
