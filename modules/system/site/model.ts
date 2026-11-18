// What the system page draws, read from the system daemon's records: the
// summary, a part record for each part and a record for each thing in one
// (modules/system/src/record.rs). Nothing here touches the page, so every
// case is proven in node, and a reading the page couldn't make sense of is
// said rather than drawn as if it were whole.

import type { Held } from "../../local-sites/kit/records.ts";
import { object, text, texts } from "../../local-sites/kit/records.ts";
import { copyable, type Ties, tiesOf } from "./ties.ts";

/** How a part or thing stands, as the daemon words it. */
type Health = "good" | "watch" | "bad" | "unread";

/** One thing in a part: a machine, for now. */
interface Thing {
  readonly name: string;
  readonly health: Health;
  readonly says: string;
  /** What in it is worth a look, worded for a side, worst first. */
  readonly side: readonly string[];
  /** Its home's address from the sites' root, when its home publishes one. */
  readonly address: string | undefined;
  /** What ties it to the other parts, a machine's (ties.ts). */
  readonly ties: Ties;
}

/** The ways deeper into a part. */
interface Deeper {
  readonly command: string;
  readonly site: string;
  readonly launcher: string;
}

/** One part of the system, as the page draws it. */
interface Part {
  readonly part: string;
  readonly health: Health;
  readonly says: string;
  /** What in it is worth a look, an item each, the last maybe "and N more". */
  readonly side: readonly string[];
  /** What it holds, counted in a few words; none when it wasn't read. */
  readonly holds: string | undefined;
  readonly things: readonly Thing[];
  /** How many things it names whose records haven't arrived yet. */
  readonly untold: number;
  readonly deeper: Deeper;
}

/** The whole system as the page draws it. */
interface System {
  readonly machine: string;
  readonly readAt: string;
  readonly health: Health;
  readonly parts: readonly Part[];
  /** What the records held that couldn't be read, for the page to say. */
  readonly unread: readonly string[];
  /**
   * The parts the summary names whose records haven't arrived: the summary
   * and each part are told apart, so a page can be drawn before its parts.
   */
  readonly untold: readonly string[];
}

/** Each health by the word the daemon says it in (modules/system's Health::word). */
const HEALTHS: ReadonlyMap<string, Health> = new Map([
  ["good", "good"],
  ["watch", "watch"],
  ["bad", "bad"],
  ["not read", "unread"],
]);

const NAMES: { readonly [part: string]: string } = {
  machines: "Machines",
  mycelium: "Mycelium",
  network: "Network",
  code: "Code",
};

/**
 * How each health is said, in the system's own words as the `system`
 * command says them: in the legend and to a screen reader.
 */
const WORDS: { readonly [health in Health]: string } = {
  good: "Good",
  watch: "Watch",
  bad: "Bad",
  unread: "Not read",
};

/** The numbers a headline says in words, as a sentence would. */
const NUMBERS = [
  "no",
  "one",
  "two",
  "three",
  "four",
  "five",
  "six",
  "seven",
  "eight",
  "nine",
  "ten",
];

/** The health the daemon's word `value` says, or unread when it says none. */
function healthOf(value: string | undefined): Health {
  return (value === undefined ? undefined : HEALTHS.get(value)) ?? "unread";
}

/** The part's name as the page says it. */
function named(part: string): string {
  return Object.hasOwn(NAMES, part) ? (NAMES[part] ?? part) : part;
}

/** Figures as en-CA writes them, their thousands grouped. */
const FIGURES = new Intl.NumberFormat("en-CA");

/** `count` as a sentence says it: in words to ten, then in figures. */
function numbered(count: number): string {
  return NUMBERS[count] ?? FIGURES.format(count);
}

/** `said` with its first letter a capital, as a sentence starts. */
function capital(said: string): string {
  return `${said.charAt(0).toUpperCase()}${said.slice(1)}`;
}

/**
 * The thing `key` names, read from its record. A thing not told yet is left
 * out unsaid: its part and its record are two watches, so a new machine can
 * reach its part one push before its own record arrives.
 */
function thingOf(held: Held, key: string, unread: string[]): Thing | undefined {
  const record = held.get(key);
  if (record === undefined) {
    return undefined;
  }
  const name = text(record, "name");
  if (name === undefined) {
    unread.push(`${key} has no name`);
    return undefined;
  }
  return {
    name,
    health: healthOf(text(record, "health")),
    says: text(record, "says") ?? "",
    side: texts(record, "side") ?? [],
    address: text(record, "address"),
    ties: tiesOf(record["ties"]),
  };
}

/** The part `name`, read from its record and its things'. */
function partOf(parts: Held, things: Held, name: string, unread: string[]): Part | undefined {
  const record = parts.get(`part:${name}`);
  if (record === undefined) {
    unread.push(`part:${name} wasn't told`);
    return undefined;
  }
  const deeper = object(record, "deeper") ?? {};
  const says = (field: string): string => {
    const value = deeper[field];
    return typeof value === "string" ? value : "";
  };
  const keys = texts(record, "things") ?? [];
  return {
    part: name,
    health: healthOf(text(record, "health")),
    says: text(record, "says") ?? "",
    side: texts(record, "side") ?? [],
    holds: text(record, "holds"),
    things: keys.map((key) => thingOf(things, key, unread)).filter((thing) => thing !== undefined),
    untold: keys.filter((key) => things.get(key) === undefined).length,
    deeper: {
      command: copyable(says("command")) ? says("command") : "",
      site: says("site"),
      launcher: says("launcher"),
    },
  };
}

/** The system the three watches hold, or none until the summary is told. */
function systemOf(summaries: Held, parts: Held, things: Held): System | undefined {
  const summary = summaries.get("summary");
  if (summary === undefined) {
    return undefined;
  }
  const unread: string[] = [];
  const names = texts(summary, "parts");
  if (names === undefined) {
    unread.push("the summary named no parts");
  }
  return {
    untold: (names ?? []).filter((name) => parts.get(`part:${name}`) === undefined),
    machine: text(summary, "machine") ?? "",
    readAt: text(summary, "read_at") ?? "",
    health: healthOf(text(summary, "health")),
    parts: (names ?? [])
      .map((name) => partOf(parts, things, name, unread))
      .filter((part) => part !== undefined),
    unread,
  };
}

/**
 * Whether the page may draw: once every watch in `kinds` has told its records
 * whole, or can't, since it's broken or stale. Drawn sooner, a page paints
 * the summary's half and then jumps as each later kind lands.
 */
function allTold(
  kinds: readonly string[],
  told: ReadonlySet<string>,
  standings: ReadonlyMap<string, { readonly state: string }>,
): boolean {
  return kinds.every((kind) => {
    const state = standings.get(kind)?.state;
    return told.has(kind) || state === "broken" || state === "stale";
  });
}

export type { Deeper, Health, Part, System, Thing };
export { allTold, capital, named, numbered, systemOf, WORDS };
