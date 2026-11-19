// The network as a tree, for keys and screen readers, decided apart from
// the drawing (view.js) that writes it into the page. Machines hold the
// networks their cords carry, networks hold their branches and then their
// cells, worst first. Only open branches are shown, so the tree stays small
// however large the network grows. Every thing goes by its key: `m:` and a
// machine's name, `h:` and a network's, `c:` and a cell's address.

import type { TreeAct } from "./keys.ts";
import {
  type Cell,
  cellWords,
  hubWords,
  type Machine,
  machineWords,
  type Network,
  type Reading,
  STANDINGS,
} from "./words.ts";

const collate = new Intl.Collator("en-CA").compare;

/**
 * Networks in the order the tree lists them under a machine or a network: by
 * name. At the top, with machines hidden, they keep the reading's order.
 */
export const byName = (a: { readonly name: string }, b: { readonly name: string }): number =>
  collate(a.name, b.name);

/** Cells in the order the tree lists them: worst first, then by name. */
export const worstFirst = (a: Cell, b: Cell): number =>
  STANDINGS.indexOf(a.standing) - STANDINGS.indexOf(b.standing) || collate(a.short, b.short);

/**
 * What hangs from a thing, in the tree's order: a machine's networks, a
 * network's branches and then its cells.
 */
export const kidsOf = (reading: Reading, key: string): readonly string[] => {
  const id = key.slice(2);
  if (key[0] === "m") {
    return [...reading.hubs.values()]
      .filter((top) => !top.within && top.machine === id)
      .sort(byName)
      .map((top) => `h:${top.key}`);
  }
  const h = key[0] === "h" ? reading.hubs.get(id) : undefined;
  if (!h) {
    return [];
  }
  return [
    ...[...h.children].sort(byName).map((c) => `h:${c.key}`),
    ...[...h.cells].sort(worstFirst).map((s) => `c:${s.address}`),
  ];
};

/**
 * Whether anything hangs from a thing, known for a network without sorting
 * what does, since a closed branch needs only to know it has any.
 */
export const hasKids = (reading: Reading, key: string): boolean => {
  if (key[0] === "h") {
    const h = reading.hubs.get(key.slice(2));
    return h !== undefined && h.children.length + h.cells.length > 0;
  }
  return key[0] === "m" && kidsOf(reading, key).length > 0;
};

/**
 * What a thing hangs from: a cell its network, a branch the network it is a
 * branch of, a network its machine while machines are shown; nothing for
 * the rest, or for what the reading no longer holds.
 */
export const upOf = (reading: Reading, key: string, machinesShown: boolean): string | null => {
  const id = key.slice(2);
  if (key[0] === "h") {
    const h = reading.hubs.get(id);
    if (!h) {
      return null;
    }
    if (h.within) {
      return `h:${h.within}`;
    }
    return h.machine && machinesShown ? `m:${h.machine}` : null;
  }
  if (key[0] === "c") {
    const s = reading.byAddress.get(id);
    const home = s?.hub ?? s?.loose;
    return home ? `h:${home.key}` : null;
  }
  return null;
};

/** The tree's top: the machines while they're shown, else every network that is no branch. */
export const rootsOf = (
  reading: { readonly hubs: ReadonlyMap<string, Network>; readonly machines: readonly Machine[] },
  machinesShown: boolean,
): readonly string[] =>
  machinesShown && reading.machines.length > 0
    ? reading.machines.map((x) => `m:${x.name}`)
    : [...reading.hubs.values()].filter((h) => !h.within).map((h) => `h:${h.key}`);

/** The words a tree row shows: its name, in mono for a machine's, and what it does under it. */
export interface RowWords {
  readonly name: string;
  readonly fact: string;
  readonly mono: boolean;
}

/** What a thing's row shows, by its key; a thing the reading no longer holds goes by its id. */
export const rowWords = (reading: Reading, key: string): RowWords => {
  const id = key.slice(2);
  if (key[0] === "c") {
    const s = reading.byAddress.get(id);
    return s
      ? { name: s.short, fact: cellWords(s), mono: false }
      : { name: id, fact: "", mono: false };
  }
  if (key[0] === "h") {
    const h = reading.hubs.get(id);
    return h
      ? { name: h.name, fact: hubWords(h), mono: false }
      : { name: id, fact: "", mono: false };
  }
  return { name: id, fact: machineWords(reading, id), mono: true };
};

/** A row the tree shows, where it sits among its siblings, and its rows while it's open. */
export interface Row {
  readonly key: string;
  /** How deep it sits, the top being 1. */
  readonly level: number;
  /** Its place among its siblings, from 1, and how many they are. */
  readonly at: number;
  readonly of: number;
  /** Whether it is open, or null where nothing hangs from it to open. */
  readonly open: boolean | null;
  readonly kids: readonly Row[];
}

/** How the tree reads what hangs from what, so the drawing can keep what it has sorted. */
export interface Hanging {
  readonly kids: (key: string) => readonly string[];
  readonly any: (key: string) => boolean;
}

/** The rows the tree shows from `roots`, with every branch in `expanded` open. */
export const rowsOf = (
  roots: readonly string[],
  expanded: ReadonlySet<string>,
  hanging: Hanging,
): readonly Row[] => {
  const rows = (keys: readonly string[], level: number): readonly Row[] =>
    keys.map((key, i) => {
      const open = hanging.any(key) ? expanded.has(key) : null;
      return {
        key,
        level,
        at: i + 1,
        of: keys.length,
        open,
        kids: open ? rows(hanging.kids(key), level + 1) : [],
      };
    });
  return rows(roots, 1);
};

/** Every thing a key needs opened above it for its row to show, nearest first. */
export const openedTo = (key: string, up: (key: string) => string | null): readonly string[] => {
  const above: string[] = [];
  for (let at = up(key); at; at = up(at)) {
    above.push(at);
  }
  return above;
};

/**
 * Every step a key in the tree can come to, listed so the drawing, which no
 * typechecker reads, can say at load that it takes each one.
 */
export const EVERY_TREE_STEP = ["focus", "expand", "collapse", "open"] as const;

/**
 * A step a key in the tree comes to: focus on a row, open or close a
 * branch, or open a thing in the drawer.
 */
export interface Step {
  readonly step: (typeof EVERY_TREE_STEP)[number];
  readonly key: string;
}

/** A shown row, with the row it hangs under. */
interface Placed {
  readonly row: Row;
  readonly up: Row | null;
}

// The shown rows in the order they read, top to bottom.
const flat = (rows: readonly Row[], up: Row | null = null): Placed[] =>
  rows.flatMap((row) => [{ row, up }, ...flat(row.kids, row)]);

/**
 * What a key's act in the tree comes to, from the row it's on: a step to
 * the next row or the one before, the first or the last, into a branch
 * (opening it if it's closed) or out of one (closing it if it's open), or
 * the thing opened. Nothing where there is nowhere to go.
 */
export const walked = (act: TreeAct, rows: readonly Row[], key: string): Step | null => {
  const shown = flat(rows);
  const at = shown.findIndex((x) => x.row.key === key);
  const here = shown[at];
  if (!here) {
    return null;
  }
  const focus = (to: Row | null | undefined): Step | null =>
    to ? { step: "focus", key: to.key } : null;
  switch (act) {
    case "next":
      return focus(shown[at + 1]?.row);
    case "prev":
      return focus(shown[at - 1]?.row);
    case "first":
      return focus(shown[0]?.row);
    case "last":
      return focus(shown.at(-1)?.row);
    case "in":
      if (here.row.open === false) {
        return { step: "expand", key };
      }
      return focus(here.row.kids[0]);
    case "out":
      return here.row.open ? { step: "collapse", key } : focus(here.up);
    case "open":
      return { step: "open", key };
    default:
      return act satisfies never;
  }
};
