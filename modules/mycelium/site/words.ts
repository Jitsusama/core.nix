// What each thing on the map is, in words: a cell, a network and a machine,
// as the tree's rows show them under their names and as their labels read
// them aloud, and the counts and sums those words are made of, which the
// panel says too. Decided from the reading alone, apart from the drawing
// (view.js), so each can be tried alone.

/** How a cell stands, worst first: the order the map lists and counts them in. */
export const STANDINGS = ["asks", "stuck", "moving", "quiet", "dormant"] as const;

/** How a cell stands. */
export type Standing = (typeof STANDINGS)[number];

/** Each standing as the map says it. */
export const STANDING_WORD: Readonly<Record<Standing, string>> = {
  asks: "needs you",
  stuck: "stuck",
  moving: "moving",
  quiet: "quiet",
  dormant: "dormant",
};

/** A cell as the map's reading holds it, of what its words read. */
export interface Cell {
  readonly address: string;
  readonly short: string;
  readonly standing: Standing;
  /** What it needs of the person, when it asks. */
  readonly wants?: string | null | undefined;
  /** What its turn is doing, when it moves. */
  readonly doing?: string | null | undefined;
  /** The questions waiting on it, and the holds it has standing. */
  readonly owes: readonly unknown[];
  readonly holds: readonly unknown[];
  readonly usage?: { readonly cost_today?: number | null | undefined } | null | undefined;
  readonly machine?: string | null | undefined;
  /** The network it is drawn in, or the one it works alone in where it is in none. */
  readonly hub?: Network | null | undefined;
  readonly loose?: Network | null | undefined;
}

/** A network as the map's reading holds it: its own cells, and its branches. */
export interface Network {
  readonly key: string;
  readonly name: string;
  /** The network it is a branch of, by key. */
  readonly within: string | null;
  readonly machine?: string | null | undefined;
  readonly children: readonly Network[];
  readonly cells: readonly Cell[];
}

/** A machine, or a basin standing for the machines in it. */
export interface Machine {
  readonly name: string;
  readonly state?: string | null | undefined;
  readonly members?: readonly Machine[] | null | undefined;
}

/** The map's reading, of what the words read. */
export interface Reading {
  readonly hubs: ReadonlyMap<string, Network>;
  readonly byAddress: ReadonlyMap<string, Cell>;
  readonly machines: readonly Machine[];
  readonly cells: readonly Cell[];
}

/**
 * One formatter for every count, made once: toLocaleString makes a new one
 * on every call, which at 10k cells was most of what a tree row cost.
 */
export const NUMBER = new Intl.NumberFormat("en-CA");
// Dollars to the cent below a hundred, where cents still tell cells
// apart, and whole above it, where they are noise.
const WHOLE_FROM = 100;
const WHOLE = new Intl.NumberFormat("en-CA", { maximumFractionDigits: 0 });
const CENTS = new Intl.NumberFormat("en-CA", {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

/** `n` of a thing, as "1 cell" or "3 cells", or as `many` says more than one. */
export const plural = (n: number, one: string, many = `${one}s`): string =>
  `${NUMBER.format(n)} ${n === 1 ? one : many}`;

/** Dollars, or nothing where there is no sum. */
export const money = (usd: number | null | undefined): string =>
  usd === null || usd === undefined ? "" : `$${(usd >= WHOLE_FROM ? WHOLE : CENTS).format(usd)}`;

/**
 * What a cell spent today: its session's own sum, since local midnight, of
 * the list price pi puts on every call it makes. The bill is spend-watch's
 * to say; this is what the sessions say of themselves.
 */
export const spentBy = (s: Cell): number => s.usage?.cost_today || 0;

/** Whether any of the cells says what it spent. */
export const spends = (cells: readonly Cell[]): boolean =>
  cells.some((s) => (s.usage?.cost_today ?? null) !== null);

// The sum is kept per list, and each reading brings new lists.
const SPENT = new WeakMap<readonly Cell[], number>();

/** What the cells spent today, together. */
export const spentOf = (cells: readonly Cell[]): number => {
  let sum = SPENT.get(cells);
  if (sum === undefined) {
    sum = cells.reduce((t, s) => t + spentBy(s), 0);
    SPENT.set(cells, sum);
  }
  return sum;
};

/** What the cells spent today, as words to follow theirs, or nothing where none says. */
export const spentWords = (cells: readonly Cell[]): string =>
  spends(cells) ? `, ${money(spentOf(cells))} today` : "";

/** How many questions wait, in words. */
export const waitingWords = (owes: readonly unknown[]): string =>
  `${owes.length === 1 ? "a question" : `${owes.length} questions`} waiting`;

/** How many of the cells stand each way. */
export const countsOf = (cells: readonly Cell[]): Record<Standing, number> => {
  const n: Record<Standing, number> = { asks: 0, stuck: 0, moving: 0, quiet: 0, dormant: 0 };
  for (const s of cells) {
    n[s.standing] += 1;
  }
  return n;
};

/** The cells, how they stand, and how many have questions waiting on them. */
export const countsWords = (cells: readonly Cell[]): string => {
  const n = countsOf(cells);
  const bits = STANDINGS.filter((s) => n[s]).map((s) => `${n[s]} ${STANDING_WORD[s]}`);
  const held = cells.filter((s) => s.owes.length).length;
  const asked = held === 1 ? "a question" : "questions";
  return (
    `${plural(cells.length, "cell")}${bits.length > 0 ? `: ${bits.join(", ")}` : ""}` +
    (held ? `; ${held} with ${asked} waiting` : "")
  );
};

/** How a cell stands, in words: what it needs or does where it says. */
const standingWords = (s: Cell): string => {
  if (s.standing === "asks") {
    return s.wants ? `needs you: ${s.wants}` : STANDING_WORD.asks;
  }
  if (s.standing === "moving") {
    return `moving: ${s.doing || "in a turn"}`;
  }
  return STANDING_WORD[s.standing];
};

/** What a cell is doing, and what waits on it or it holds. */
export const cellWords = (s: Cell): string => {
  const holds = s.holds.length === 1 ? "a hold" : `${s.holds.length} holds`;
  return (
    standingWords(s) +
    (s.owes.length > 0 ? `; ${waitingWords(s.owes)}` : "") +
    (s.holds.length > 0 ? `; ${holds} standing` : "")
  );
};

// A network with its branches, every cell under it, one list per network.
const ALL_CELLS = new WeakMap<Network, readonly Cell[]>();

/** Every cell of a network, its branches' included. */
export const allCells = (h: Network): readonly Cell[] => {
  let cells = ALL_CELLS.get(h);
  if (cells === undefined) {
    cells = h.children.length > 0 ? [...h.cells, ...h.children.flatMap(allCells)] : h.cells;
    ALL_CELLS.set(h, cells);
  }
  return cells;
};

/** What a network's own cells are doing, and what it spent, with its branches where it has any. */
export const hubWords = (h: Network): string => {
  const every = allCells(h);
  const spent =
    h.children.length > 0 && spends(every)
      ? `, ${money(spentOf(every))} today with its branches`
      : spentWords(h.cells);
  return `${countsWords(h.cells)}${spent}`;
};

/**
 * A basin's machines, or none where the node gave anything but a list, since
 * a machine with no basin reaches the reading as the node's own record.
 */
const membersOf = (x: Machine | null | undefined): readonly Machine[] =>
  Array.isArray(x?.members) ? x.members : [];

/** How many of a basin's machines are off while it runs, in words, or nothing where none is. */
export const offWords = (x: Machine | null | undefined): string => {
  const off = membersOf(x).filter((y) => y.state !== "ready").length;
  return off ? `${plural(off, "machine")} off` : "";
};

/** Each reading's cells by machine, so one list serves a reading and its sum is summed once. */
const ON = new WeakMap<readonly Cell[], Map<string, readonly Cell[]>>();

/** A machine's cells, as the reading lists them, the same list for the whole reading. */
export const cellsOn = (reading: Reading, id: string): readonly Cell[] => {
  let by = ON.get(reading.cells);
  if (by === undefined) {
    by = new Map();
    ON.set(reading.cells, by);
  }
  let cells = by.get(id);
  if (cells === undefined) {
    cells = reading.cells.filter((s) => s.machine === id);
    by.set(id, cells);
  }
  return cells;
};

/** Whether a machine is ready, and what its cells spent. */
export const machineWords = (
  reading: Reading,
  id: string,
  cells = cellsOn(reading, id),
): string => {
  const x = reading.machines.find((y) => y.name === id);
  const off = offWords(x);
  let state = "ready";
  if (x?.state !== "ready") {
    state = "off";
  } else if (off) {
    state = `ready, ${off}`;
  }
  return `${state}, ${plural(cells.length, "cell")}${spentWords(cells)}`;
};

/**
 * What a thing on the map is, in words, by its key (`c:` a cell, `h:` a
 * network, `m:` a machine): the name a tree row goes by, and what the keys
 * say as they land on it. Its name, then the words its row shows under it,
 * so what's read aloud holds what's seen; a thing the reading no longer
 * holds goes by its id.
 */
export const described = (reading: Reading, key: string): string => {
  const id = key.slice(2);
  if (key[0] === "c") {
    const s = reading.byAddress.get(id);
    return s ? `${s.short}, ${cellWords(s)}` : id;
  }
  if (key[0] === "h") {
    const h = reading.hubs.get(id);
    if (!h) {
      return id;
    }
    const branches =
      h.children.length > 0 ? ` with ${plural(h.children.length, "branch", "branches")}` : "";
    return `${h.name}, ${hubWords(h)}, a network${branches}`;
  }
  const x = reading.machines.find((y) => y.name === id);
  const what = x?.members ? `a basin of ${plural(x.members.length, "machine")}` : "a machine";
  return `${id}, ${machineWords(reading, id)}, ${what}`;
};
