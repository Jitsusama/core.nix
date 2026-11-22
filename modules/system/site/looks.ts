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
// Worst first as the daemon ranks them: a part not read at all before a
// look, since nothing it holds can be told.
const RANK: { readonly [health in Health]: number } = { unread: 0, bad: 1, watch: 2, good: 3 };

/** The item a side ends with when it had more than it lists, as the daemon says it. */
const MORE = /^and \d+ more worth a look$/u;

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
   * Whether every part has told and every record could be read, so an
   * empty list says nothing wants a look; one still to come, or one that
   * couldn't be read, may be the one with a look in it.
   */
  readonly settled: boolean;
}

/**
 * Each of `side`'s looks with its key: how many the same came before it, then
 * its own words, so a look keeps its key while those before it come and go.
 * The count leads, since no count holds the colon after it and words may.
 */
function keyed(side: readonly string[]): readonly { words: string; key: string }[] {
  const seen = new Map<string, number>();
  return side.map((words) => {
    const before = seen.get(words) ?? 0;
    seen.set(words, before + 1);
    return { words, key: `${before}:${words}` };
  });
}

/** A part's own site, from the sites' root as the daemon says it. */
function siteOf(part: Part): string {
  return `../${part.deeper.site}`;
}

/**
 * The machines' looks: each machine's side, a look a word, or why the
 * machines couldn't be read. A machine drawn whole on the page leads to its
 * section, its position among them told by `drawn`; one past those leads
 * to the map. Then the part's own, such as the note it says when the
 * machines were read from their owner's last answer.
 */
function machineLooks(part: Part, drawn: number): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, "unread")];
  }
  return [...thingLooks(part, drawn), ...ownLooks(part)];
}

/**
 * What the machines part says worth a look that no machine does. Its side
 * names each machine worth a look, "kestrel on battery at 9%", then how many
 * more; those are left to the machines' own looks, which also clear first
 * when a machine's record changes before the part's.
 */
function ownLooks(part: Part): readonly Look[] {
  const machines = part.things.map((thing) => `${thing.name} `);
  const own = part.side.filter(
    (words) => !(MORE.test(words) || machines.some((name) => words.startsWith(name))),
  );
  return keyed(own).map(({ words, key }) => lookOf(part, part.health, words, key));
}

/** Each machine's side, a look a word, leading to its section or the map. */
function thingLooks(part: Part, drawn: number): readonly Look[] {
  return part.things.flatMap((thing, at) =>
    keyed(thing.side).map(({ words, key }) => ({
      health: thing.health,
      whose: thing.name,
      machine: true,
      words: capital(words),
      href:
        at < drawn
          ? `#machine-${at + 1}`
          : `../mycelium/#machine=${encodeURIComponent(thing.name)}`,
      row: `look:machine:${encodeURIComponent(thing.name)}:${key}`,
    })),
  );
}

/** One look in the part `part`, keyed `key` among its own. */
function lookOf(part: Part, health: Health, words: string, key: string): Look {
  return {
    health,
    whose: named(part.part),
    machine: false,
    words: capital(words),
    href: siteOf(part),
    row: `look:part:${encodeURIComponent(part.part)}:${key}`,
  };
}

/** Another part's looks: each of its side's, or why it couldn't be read. */
function partLooks(part: Part): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, "unread")];
  }
  if (part.health === "good") {
    return [];
  }
  return keyed(part.side).map(({ words, key }) => lookOf(part, part.health, words, key));
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
    // A part still to come, a machine named before its record, or a record
    // that couldn't be read may hold the one look there is.
    settled:
      system.untold.length === 0 &&
      system.unread.length === 0 &&
      system.parts.every((part) => part.untold === 0),
  };
}

export type { Look, Looks };
export { LOOKS_LISTED, looksOf };
