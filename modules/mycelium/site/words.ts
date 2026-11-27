// What each thing on the map is, in words: a cell, a network, a machine and
// a builder, as the tree's rows show them under their names and as their
// labels read them aloud, and the counts and sums those words are made of,
// which the panel says too. Decided from the reading alone, apart from the
// drawing (view.js), so each can be tried alone.

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
  /** The questions waiting on it, oldest first, and the holds it has standing. */
  readonly owes: readonly Owed[];
  readonly holds: readonly Hold[];
  readonly usage?: Usage | null | undefined;
  readonly machine?: string | null | undefined;
  /** The network it is drawn in, or the one it works alone in where it is in none. */
  readonly hub?: Network | null | undefined;
  readonly loose?: Network | null | undefined;
}

/** A question waiting on a cell: who asked it, and when. */
export interface Owed {
  readonly from: string;
  readonly at: string;
}

/**
 * A hold a cell has standing: where it went, as the mesh writes it, since
 * when, and until when at most.
 */
export interface Hold {
  readonly to: readonly string[];
  readonly at: string;
  readonly until?: string | null | undefined;
}

/** What a cell's session says it has used: today's turns and spend, and its context. */
export interface Usage {
  readonly cost_today?: number | null | undefined;
  readonly turns_today?: number | null | undefined;
  /** When its last turn ended. */
  readonly last_turn?: string | null | undefined;
  /** How many tokens its context holds, of how many it can. */
  readonly context?:
    | { readonly used: number; readonly limit?: number | null | undefined }
    | null
    | undefined;
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

/**
 * A builder the machine sends Nix builds to. It is no machine of the realm:
 * nothing runs on it but builds, and no node of its own says how it stands,
 * so the node says it from what reaching it, or its lender, told.
 */
export interface Builder {
  readonly name: string;
  /**
   * How it stood when last heard: `answering` or `not answering` where it
   * was reached, `running` or `not running` where its lender said, and
   * `unknown` where nothing could be heard.
   */
  readonly state: string;
  /** The systems it builds for, such as x86_64-linux. */
  readonly systems: readonly string[];
  /** How many builds it takes at once. */
  readonly jobs: number;
  /** Whether it is lent: made for a while by a lender, and paid for while it runs. */
  readonly lent: boolean;
  /** Why it stands as it does, or its lender's word, where either says. */
  readonly said: string | null;
}

/** The map's reading, of what the words read. */
export interface Reading {
  readonly hubs: ReadonlyMap<string, Network>;
  readonly byAddress: ReadonlyMap<string, Cell>;
  readonly machines: readonly Machine[];
  readonly builders?: readonly Builder[] | undefined;
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

/**
 * How many questions wait on a cell, in words said of the cell: the answers
 * it owes. Said as a question waiting, it read as one waiting on the reader,
 * when it is another cell's question and the cell's to answer.
 */
export const waitingWords = (owes: readonly Owed[]): string =>
  `owes ${owes.length === 1 ? "an answer" : `${owes.length} answers`}`;

/** How many of the cells stand each way. */
export const countsOf = (cells: readonly Cell[]): Record<Standing, number> => {
  const n: Record<Standing, number> = { asks: 0, stuck: 0, moving: 0, quiet: 0, dormant: 0 };
  for (const s of cells) {
    n[s.standing] += 1;
  }
  return n;
};

/**
 * The cells, how they stand, and how many answers they owe between them. The answers are
 * counted rather than the cells owing them, as a network's health counts them, so one cell
 * owing three never reads as one answer beside a drawer saying three.
 */
export const countsWords = (cells: readonly Cell[]): string => {
  const n = countsOf(cells);
  const bits = STANDINGS.filter((s) => n[s]).map((s) => `${n[s]} ${STANDING_WORD[s]}`);
  const owed = cells.reduce((sum, s) => sum + s.owes.length, 0);
  return (
    `${plural(cells.length, "cell")}${bits.length > 0 ? `: ${bits.join(", ")}` : ""}` +
    (owed ? `; ${plural(owed, "answer")} owed` : "")
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

/** How a builder stands, as the map says it: each as the node says, but one heard by nobody. */
const BUILDER_STATE_WORD: Readonly<Record<string, string>> = { unknown: "unheard" };

/** Whether a builder takes builds now: reached and answering, or run by its lender. */
export const builds = (b: Builder): boolean => b.state === "answering" || b.state === "running";

/**
 * How much a builder asks of the reader, as the peek grades it: nothing while
 * it takes builds or rests (a lent builder its lender doesn't run, which is
 * how it waits for work), most when it didn't answer, and between for one
 * nobody heard or in a state the map doesn't know.
 */
export const builderTone = (b: Builder): 0 | 1 | 2 => {
  if (builds(b) || b.state === "not running") {
    return 0;
  }
  return b.state === "not answering" ? 2 : 1;
};

/**
 * What a builder's mark and row say under its name: lent or not, how it
 * stands, how much at once.
 */
export const builderParts = (b: Builder): readonly string[] =>
  [
    b.lent ? "lent" : "",
    BUILDER_STATE_WORD[b.state] ?? b.state,
    b.jobs > 0 ? `${plural(b.jobs, "build")} at once` : "",
  ].filter(Boolean);

/** A builder's parts, as a row's words. */
export const builderWords = (b: Builder): string => builderParts(b).join(", ");

/**
 * How a builder stands, as its drawer says it: who said so, and why where
 * the node gave a reason, since a builder nobody could hear is told apart
 * from one that didn't answer.
 */
export const builderSays = (b: Builder): string => {
  const why = b.said ? `: ${b.said}` : "";
  switch (b.state) {
    case "answering":
      return "It answered when last reached, so it takes builds.";
    case "running":
      return b.said
        ? `Its lender runs it, so it takes builds; the lender's word for it is ${b.said}.`
        : "Its lender runs it, so it takes builds.";
    case "not answering":
      return `It didn't answer when last reached${why}.`;
    case "not running":
      return `Its lender doesn't run it now${why}.`;
    case "unknown":
      return `Nobody has heard how it stands${why}.`;
    default:
      return `The node says it is ${b.state}${why}.`;
  }
};

/**
 * What a thing on the map is, in words, by its key (`c:` a cell, `h:` a
 * network, `m:` a machine, `b:` a builder): the name a tree row goes by, and what the keys
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
  if (key[0] === "b") {
    const b = reading.builders?.find((y) => y.name === id);
    return b ? `${id}, ${builderWords(b)}, a builder` : id;
  }
  const x = reading.machines.find((y) => y.name === id);
  const what = x?.members ? `a basin of ${plural(x.members.length, "machine")}` : "a machine";
  return `${id}, ${machineWords(reading, id)}, ${what}`;
};

/**
 * What each kind of shared thing a network gathers round is called in words.
 * The model keeps only the networks whose kind is named here, so the panel
 * and the map's drawing never meet a kind without its word.
 */
export const SHARED_WORD: Readonly<Record<string, string>> = {
  project: "quest",
  repository: "repository",
  group: "group",
};

/** A network as its name is told: its key, its name and what it carries. */
export interface Named {
  readonly key: string;
  readonly name: string;
  readonly carries: string;
}

/**
 * Each network's name, told apart from the others': networks whose titles
 * share their name, as several slices of one project do, are named with
 * what each carries too, so no two read alike on the map, in the tree or in
 * Now; whole titles that repeat are told apart by their ids, the part of
 * the key after its kind, and any still alike, as a title written to read
 * as another's told name is, by their whole keys until none are.
 */
export const toldApart = (networks: readonly Named[]): ReadonlyMap<string, string> => {
  const names = new Map(networks.map((n) => [n.key, asDrawn(n.name)]));
  tell(networks, names, ": ", (n) => {
    const carries = asDrawn(n.carries);
    return carries === asDrawn(n.name) ? "" : carries;
  });
  tell(networks, names, " ", idHint);
  // Keys are unique, so each pass leaves fewer alike; one per network
  // bounds it whatever the titles say.
  for (let pass = 0; pass < networks.length; pass++) {
    if (!tell(networks, names, " ", (n) => `[${n.key}]`)) {
      break;
    }
  }
  return names;
};

/**
 * Each network's label on the map, which clips a name to `room`: a told
 * name clipped alike to another's, its tell cut off, is the network's own
 * name clipped shorter with its id after it, so none read alike there
 * either. A label kept whole that reads like another's id-hinted one, as a
 * title written to does, takes its own id in turn; hinted labels still
 * alike, as two long ids clipped alike are, are numbered in order.
 */
export const labelsApart = (
  networks: readonly Named[],
  told: ReadonlyMap<string, string>,
  clip: (text: string, room: number) => string,
  room: number,
): ReadonlyMap<string, string> => {
  const labels = new Map(networks.map((n) => [n.key, clip(told.get(n.key) ?? n.name, room)]));
  const hinted = new Set<string>();
  // Each pass hints at least one more network or ends, so one per network
  // bounds it.
  for (let pass = 0; pass < networks.length; pass++) {
    const seen = timesEach(labels.values());
    const alike = networks.filter(
      (n) => !hinted.has(n.key) && (seen.get(labels.get(n.key) ?? "") ?? 0) > 1,
    );
    if (alike.length === 0) {
      break;
    }
    for (const n of alike) {
      labels.set(n.key, hintedLabel(n, clip, room));
      hinted.add(n.key);
    }
  }
  numberAlike(networks, labels, clip, room);
  return labels;
};

/**
 * Each label in `labels` still reading like another's clipped short enough
 * to end on a number, the least that leaves it reading like no label
 * already standing, so none read alike. Two sets clipped to one stem share
 * its numbers, and each set is numbered in its networks' key order, so a
 * network keeps its number while the networks alike with it stay the same.
 */
const numberAlike = (
  networks: readonly Named[],
  labels: Map<string, string>,
  clip: (text: string, room: number) => string,
  room: number,
): void => {
  const seen = timesEach(labels.values());
  const isAlike = (n: Named): boolean => (seen.get(labels.get(n.key) ?? "") ?? 0) > 1;
  const alike = networks.filter(isAlike).toSorted((a, b) => byKey(a.key, b.key));
  if (alike.length === 0) {
    return;
  }
  const taken = new Set(networks.filter((n) => !isAlike(n)).map((n) => labels.get(n.key)));
  // No number passes the count of networks, so the widest is kept room for.
  const stemRoom = room - ` ${networks.length}`.length;
  for (const n of alike) {
    const stem = clip(labels.get(n.key) ?? "", stemRoom);
    let place = 1;
    while (taken.has(`${stem} ${place}`)) {
      place++;
    }
    const label = `${stem} ${place}`;
    taken.add(label);
    labels.set(n.key, label);
  }
};

/** Two keys in the order their characters sort, the same in every locale. */
const byKey = (a: string, b: string): number => {
  if (a === b) {
    return 0;
  }
  return a < b ? -1 : 1;
};

// The fewest characters of its name a label keeps beside its id, so a
// clip is never asked for less room than its own three dots take.
const LEAST_NAME = 8;

/**
 * A network's name clipped to leave room for its id after it, or, where
 * the id is too long to leave a name any room, the id alone clipped to the
 * room, so a label never runs past it.
 */
const hintedLabel = (
  n: Named,
  clip: (text: string, room: number) => string,
  room: number,
): string => {
  const hint = idHint(n);
  const left = room - hint.length - 1;
  const name = asDrawn(n.name);
  // A name that draws as nothing leaves the id alone, so no label starts on a space.
  if (left < LEAST_NAME || name === "") {
    return clip(hint, room);
  }
  return `${clip(name, left)} ${hint}`;
};

// Any run of the white space a page runs together: spaces, tabs and line breaks, as CSS has
// it. JavaScript's \s takes in a no-break space too, which a page keeps as it is, so two names
// apart only by one would be counted alike though a page never draws them so.
const SPACES = /[ \t\n\r\f]+/gu;
// The space a page drops at either end, once each run is one; trim() would drop a
// no-break space there too.
const EDGE_SPACES = /^ | $/gu;

/**
 * A name as a page draws it, each run of spaces as one and none at its
 * ends, so two that read alike there are alike here too.
 */
const asDrawn = (text: string): string => text.replace(SPACES, " ").replace(EDGE_SPACES, "");

/**
 * A network's id as a hint: the part of its key after its kind, as drawn,
 * since a key can hold spaces or a line break the map runs together.
 */
const idHint = (n: Named): string =>
  `(${asDrawn(n.key.slice(n.key.indexOf(".") + 1)).toUpperCase()})`;

/** How many times each text comes. */
const timesEach = (texts: Iterable<string>): ReadonlyMap<string, number> => {
  const seen = new Map<string, number>();
  for (const text of texts) {
    seen.set(text, (seen.get(text) ?? 0) + 1);
  }
  return seen;
};

/**
 * Adds what `more` tells to each name that repeats, after `between`, and
 * says whether any did; a network `more` has nothing for keeps its name,
 * and a name that draws as nothing is its tell alone, with no `between`
 * to start on.
 */
const tell = (
  networks: readonly Named[],
  names: Map<string, string>,
  between: string,
  more: (n: Named) => string,
): boolean => {
  const seen = timesEach(names.values());
  let any = false;
  for (const n of networks) {
    const name = names.get(n.key) ?? n.name;
    if ((seen.get(name) ?? 0) > 1) {
      const told = more(n);
      if (told !== "") {
        names.set(n.key, name === "" ? told : `${name}${between}${told}`);
      }
      any = true;
    }
  }
  return any;
};
