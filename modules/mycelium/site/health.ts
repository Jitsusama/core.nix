// How things stand, as the drawer's health rows say it: a verdict of well,
// watch or tend for a set of cells, one cell or a machine, the words that
// say why, what form a machine takes, and the realm's balance in the words
// `mycelium balance` says it in. Decided from the reading alone, apart from
// the drawing (view.js), which only wraps each in its row.

import { leftWords } from "./when.ts";
import {
  type Cell,
  countsOf,
  type Machine,
  money,
  NUMBER,
  plural,
  STANDING_WORD,
  spentOf,
} from "./words.ts";

/** A context this full is worth a look. */
export const CONTEXT_WATCH = 0.75;
/** And this full wants tending. */
export const CONTEXT_TEND = 0.9;
// Percent of a machine's cpu or memory worth a look.
const PRESSURE_WATCH = 85;
// Percent of battery left that wants tending.
const CHARGE_TEND = 20;
// A sandbox this near its end is worth a look.
const LEASE_WATCH_MS = 30 * 60_000;
// A lease this far out is one the person keeps, not one that runs out.
const KEPT_MS = 365 * 86_400_000;
const PERCENT = 100;
const GIB = 2 ** 30;
const THOUSAND = 1000;
const MILLION = 1e6;

/** How a thing stands: well, watch or tend, as 0, 1 and 2. */
export type Verdict = 0 | 1 | 2;

/** Each verdict in words, by its number. */
export const VERDICT: readonly string[] = ["Well", "Watch", "Tend"];

/** A verdict and the words saying why. */
export interface Health {
  readonly v: Verdict;
  readonly words: string;
}

/** How full a cell's context is, from 0 to 1, or 0 where it says nothing. */
export const contextShare = (s: Cell): number => {
  const c = s.usage?.context;
  return c?.limit ? c.used / c.limit : 0;
};

/** A count of tokens, in thousands or millions. */
export const compactCount = (n: number): string =>
  n >= MILLION
    ? `${NUMBER.format(Number((n / MILLION).toFixed(1)))}M`
    : `${NUMBER.format(Math.round(n / THOUSAND))}k`;

/** How full a cell's context is, in words, or nothing where it says nothing. */
export const contextWords = (s: Cell): string => {
  const c = s.usage?.context;
  return c?.limit
    ? `context ${Math.round(contextShare(s) * PERCENT)}% of ${compactCount(c.limit)}`
    : "";
};

/**
 * Tend where anything waits on the person or is stuck, owes an answer or is
 * nearly out of context; watch where a context is filling; else well.
 */
export const verdictOf = (cells: readonly Cell[]): Verdict => {
  let v: Verdict = 0;
  for (const s of cells) {
    const share = contextShare(s);
    if (
      s.standing === "asks" ||
      s.standing === "stuck" ||
      s.owes.length > 0 ||
      share >= CONTEXT_TEND
    ) {
      return 2;
    }
    if (share >= CONTEXT_WATCH) {
      v = 1;
    }
  }
  return v;
};

const QUIETER = ["moving", "quiet", "dormant"] as const;

/** How a set of cells stands, and why, with what it spent today. */
export const groupHealth = (cells: readonly Cell[]): Health => {
  const n = countsOf(cells);
  const owes = cells.reduce((t, s) => t + s.owes.length, 0);
  const full = cells.filter((s) => contextShare(s) >= CONTEXT_TEND).length;
  const filling = cells.filter((s) => {
    const share = contextShare(s);
    return share >= CONTEXT_WATCH && share < CONTEXT_TEND;
  }).length;
  const bits = [
    n.asks > 0 ? `${n.asks} ${n.asks === 1 ? "needs" : "need"} you` : "",
    n.stuck > 0 ? `${n.stuck} stuck` : "",
    owes > 0 ? `${plural(owes, "answer")} owed` : "",
    full > 0 ? `${full} nearly out of context` : "",
    filling > 0 ? `${filling} with context filling` : "",
    ...QUIETER.filter((s) => n[s] > 0).map((s) => `${n[s]} ${STANDING_WORD[s]}`),
  ].filter(Boolean);
  const spent = spentOf(cells);
  const today = spent ? `; ${money(spent)} today` : "";
  return { v: verdictOf(cells), words: `${bits.join(", ") || "no cells"}${today}` };
};

/** A machine as the map's reading holds it, of what its health reads. */
export interface MachineFacts extends Machine {
  readonly leaves_at?: string | null | undefined;
  readonly declared?: { readonly leaves_at?: string | null | undefined } | null | undefined;
  readonly basin?: unknown;
  /** How pressed it is, as a peer's node says it, so in no shape for sure. */
  readonly pressure?: unknown;
  readonly power?:
    | { readonly on_battery?: boolean | null | undefined; readonly charge?: number | null }
    | null
    | undefined;
  /** The resources it runs short of, as a peer's node says them. */
  readonly over?: unknown;
  readonly available?: number | null | undefined;
  readonly disk?: number | null | undefined;
  readonly disk_free?: number | null | undefined;
  readonly disk_band?: string | null | undefined;
}

/**
 * When a machine a provider lent leaves, as it declares: at its lease's end.
 * Bare metal never does, so the map tells them apart by what each machine
 * says of itself, never by its name or its provider's.
 */
export const leavesAt = (x: MachineFacts | null | undefined): string | null =>
  x?.leaves_at || x?.declared?.leaves_at || null;

/** Whether a machine was lent, and leaves at its lease's end. */
export const isSandbox = (x: MachineFacts | null | undefined): boolean => leavesAt(x) !== null;

/**
 * Whether a machine is a box borrowed from a pool of cloud machines, which
 * says which basin it is in, in what it declares; never by its name, which
 * the node makes up. A basin's machines are drawn as the basin (model.ts).
 */
export const isBasin = (x: MachineFacts | null | undefined): boolean => Boolean(x?.basin);

/** The form a machine is drawn in: a basin, lent or bare metal. */
export const machineForm = (x: MachineFacts): "basin" | "lent" | "shut" => {
  if (isBasin(x)) {
    return "basin";
  }
  return isSandbox(x) ? "lent" : "shut";
};

/** Bytes, in whole gibibytes, or nothing where there are none to say. */
export const gib = (bytes: number | null | undefined): string =>
  bytes === null || bytes === undefined ? "" : `${(bytes / GIB).toFixed(0)} GiB`;

/**
 * Whether a lease ending at `end` is one the person keeps rather than one
 * that runs out: a cloud machine made for him declares its end a year or
 * more away (9R6RA6's node-said.sh declares 9999-01-01), so it is never
 * said as so many thousand hours left.
 */
const kept = (end: string, at: string): boolean => Date.parse(end) - Date.parse(at) >= KEPT_MS;

/** How long a lease has left, or that it is kept. */
const leaseWords = (end: string, at: string): string =>
  kept(end, at) ? "kept until you let it go" : leftWords(end, at);

/** What a lent machine is, and how long it has left; nothing for bare metal. */
export const sandboxWords = (x: MachineFacts, at: string): string => {
  const end = leavesAt(x);
  return end === null ? "" : `a cloud machine, ${leaseWords(end, at)}`;
};

/** A machine's pressure, only where each share of it is a number. */
interface Pressure {
  readonly cpu: number;
  readonly memory: number;
  readonly io: number;
}

const SHARES = ["cpu", "memory", "io"] as const;

/** A machine's pressure where its node said all of it, as a number each. */
const pressureOf = (x: MachineFacts): Pressure | null => {
  const p = x.pressure;
  if (typeof p !== "object" || p === null) {
    return null;
  }
  const said: Record<string, unknown> = { ...p };
  const [cpu, memory, io] = SHARES.map((share) => said[share]);
  return typeof cpu === "number" && typeof memory === "number" && typeof io === "number"
    ? { cpu, memory, io }
    : null;
};

/** What a machine runs short of, as names, whatever else its node sent. */
const overOf = (x: MachineFacts): string[] =>
  Array.isArray(x.over) ? x.over.filter((name) => typeof name === "string") : [];

const leaseShort = (x: MachineFacts, at: string): boolean => {
  const end = leavesAt(x);
  return end !== null && Date.parse(end) - Date.parse(at) < LEASE_WATCH_MS;
};

/** The disk guard's bands past filling, each said after a machine's free disk. */
const FULL: Readonly<Record<string, string>> = {
  "nearly-full": ", nearly full",
  "critically-full": ", critically full",
};

/** Every band the disk guard says that a machine's free disk is said with. */
const BAND_WORDS: Readonly<Record<string, string>> = { filling: ", filling", ...FULL };

const machineVerdict = (x: MachineFacts, at: string): Verdict => {
  const p = pressureOf(x);
  const charge = x.power?.on_battery ? x.power.charge : null;
  const low = charge !== null && charge !== undefined && charge < CHARGE_TEND;
  const full = typeof x.disk_band === "string" && Object.hasOwn(FULL, x.disk_band);
  if (x.state !== "ready" || overOf(x).length > 0 || low || full) {
    return 2;
  }
  const pressed = p && (p.cpu >= PRESSURE_WATCH || p.memory >= PRESSURE_WATCH);
  // A filling disk is said but never watched, as the system page has it: the
  // disk guard calls a disk filling under a quarter free and takes back what
  // regenerates by itself, so only a nearly full one wants the reader.
  return pressed || leaseShort(x, at) ? 1 : 0;
};

/**
 * How a machine stands, and why: its state, its pressure, what it runs
 * short of, its memory, disk and battery, and a lent one's time left, after
 * whatever `lead` says first.
 */
export const machineHealth = (
  x: MachineFacts,
  at: string,
  lead: readonly string[] = [],
): Health => {
  const p = pressureOf(x);
  const over = overOf(x);
  const charge = x.power?.on_battery ? x.power.charge : null;
  const end = leavesAt(x);
  const band =
    typeof x.disk_band === "string" && Object.hasOwn(BAND_WORDS, x.disk_band)
      ? BAND_WORDS[x.disk_band]
      : "";
  const disk = x.disk ? `${gib(x.disk_free)} of ${gib(x.disk)} disk free${band}` : "";
  const bits = [
    ...lead,
    x.state === "ready" ? "" : (x.state ?? ""),
    p ? `cpu ${Math.round(p.cpu)}%, memory ${Math.round(p.memory)}%, io ${Math.round(p.io)}%` : "",
    over.length > 0 ? `${over.join(" and ")} short` : "",
    x.available === null || x.available === undefined ? "" : `${gib(x.available)} memory free`,
    disk,
    charge === null || charge === undefined ? "" : `on battery, ${charge}%`,
    end === null ? "" : leaseWords(end, at),
  ].filter(Boolean);
  return { v: machineVerdict(x, at), words: bits.join("; ") || "nothing said" };
};

/** A machine's share of the realm's work, as the balance service reads it. */
export interface Share {
  readonly load?: number | null | undefined;
  readonly over?: boolean | null | undefined;
  readonly room?: boolean | null | undefined;
  readonly busy?: number | null | undefined;
  readonly idle?: number | null | undefined;
  readonly busy_per_core?: number | null | undefined;
}

/** What the balance holds wrong over most of the last ten minutes. */
export interface Flag {
  readonly flag: string;
  readonly hot?: string | null | undefined;
  readonly cool?: string | null | undefined;
  readonly machine?: string | null | undefined;
  readonly spent?: string | null | undefined;
}

/** How loaded a machine is, as a percentage, or nothing where it doesn't say. */
export const loadWords = (load: unknown): string =>
  typeof load === "number" ? `${Math.round(load * PERCENT)}%` : "";

const STANDS_WORDS = {
  over: "over its limits",
  room: "with room for more",
  full: "with no room for more",
} as const;

const standsOf = (share: Share): string => {
  if (share.over) {
    return STANDS_WORDS.over;
  }
  return share.room ? STANDS_WORDS.room : STANDS_WORDS.full;
};

/** A machine's share of the realm's work, in the words `mycelium balance` says it in. */
export const shareWords = (share: Share): string => {
  const load = loadWords(share.load);
  const each =
    typeof share.busy_per_core === "number"
      ? `, ${Number(share.busy_per_core.toFixed(2))} busy a core`
      : "";
  const sessions =
    typeof share.busy === "number"
      ? `; ${plural(share.busy, "cell")} busy and ${NUMBER.format(share.idle || 0)} idle${each}`
      : "";
  return `${load ? `${load} loaded, ` : ""}${standsOf(share)}${sessions}`;
};

/** The flags the balance holds, those that say what they are. */
export const heldFlags = (balance: unknown): readonly Flag[] => {
  const held =
    typeof balance === "object" && balance !== null && "held" in balance ? balance.held : null;
  return Array.isArray(held) ? held.filter((f): f is Flag => typeof f?.flag === "string") : [];
};

/** The machines a flag names. */
export const flagNames = (f: Flag): readonly string[] =>
  [f.hot, f.cool, f.machine].filter((name): name is string => Boolean(name));

/**
 * What a flag says, as `mycelium balance` says it. In a machine's own
 * drawer, `self` is that machine, said as "it". A held flag is worth
 * watching, never a fault: placing is the realm's, not the machine's.
 */
export const flagWords = (f: Flag, self: string | null = null): string => {
  const its = (name: string | null | undefined): string => (name === self ? "its" : `${name}'s`);
  if (f.flag === "hot_while_cool") {
    if (f.hot === self) {
      return `it is over while ${f.cool} has room, so new work should go to ${f.cool}`;
    }
    if (f.cool === self) {
      return `${f.hot} is over while this has room, so new work should come here`;
    }
    return `${f.hot} is over while ${f.cool} has room, so new work should go to ${f.cool}`;
  }
  if (f.flag === "stranded") {
    return f.spent === "memory"
      ? `${its(f.machine)} memory is nearly all used while its processors idle`
      : `${its(f.machine)} processors are spent while half its memory or more is free`;
  }
  if (f.flag === "stale") {
    return `${its(f.machine)} facts are too old to place by`;
  }
  if (f.flag === "imbalanced") {
    return "the machines' loads are uneven, so new work leans to the least loaded";
  }
  return f.flag.replace(/_/gu, " ");
};
