// What the system page draws, read from the system daemon's records: the
// summary, a part record for each part and a record for each thing in one
// (modules/system/src/record.rs). Nothing here touches the page, so every
// case is proven in node, and a reading the page couldn't make sense of is
// said rather than drawn as if it were whole.

import type { Held } from "../../local-sites/kit/records.ts";
import { object, text, texts } from "../../local-sites/kit/records.ts";

/** How a part or thing stands, as the daemon words it. */
type Health = "good" | "watch" | "bad" | "unread";

/** One thing in a part: a machine, for now. */
interface Thing {
  readonly name: string;
  readonly health: Health;
  readonly says: string;
  /** Its home's address from the sites' root, when its home publishes one. */
  readonly address: string | undefined;
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

/** How each health is said, once in the legend and to a screen reader. */
const WORDS: { readonly [health in Health]: string } = {
  good: "Good",
  watch: "Worth a look",
  bad: "Bad",
  unread: "Not read",
};

/**
 * Each health as the page's sky: the world it is drawn in, where a part is
 * a station on a horizon and how it stands is its weather. The word in
 * WORDS says what the sky means; this one names the sky.
 */
const SKIES: { readonly [health in Health]: string } = {
  good: "Clear",
  watch: "Cloud",
  bad: "Storm",
  unread: "Fog",
};

/** What a headline says when nothing anywhere is worth a look. */
const CLEAR = "all clear";

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

/** How the daemon closes a side that had more than it says (parts.rs's side_of). */
const MORE = /^and (\d+) more worth a look$/u;

/** Healths from worst to best, as a lede picks the worst. */
const WORST: readonly Health[] = ["bad", "watch"];

/** The health the daemon's word `value` says, or unread when it says none. */
function healthOf(value: string | undefined): Health {
  return (value === undefined ? undefined : HEALTHS.get(value)) ?? "unread";
}

/** The part's name as the page says it. */
function named(part: string): string {
  return NAMES[part] ?? part;
}

/** `count` as a sentence says it: in words to ten, then in figures. */
function numbered(count: number): string {
  return NUMBERS[count] ?? String(count);
}

/** `said` with its first letter a capital, as a sentence starts. */
function capital(said: string): string {
  return `${said.charAt(0).toUpperCase()}${said.slice(1)}`;
}

/**
 * How many things in `part` are worth a look: its side's items, with the
 * last counted for as many as it says when it says "and N more". A part
 * not read has none, since what it says is why it wasn't.
 */
function looksIn(part: Part): number {
  if (part.health === "unread") {
    return 0;
  }
  return part.side.reduce((sum, item) => sum + Number(MORE.exec(item)?.[1] ?? 1), 0);
}

/** The whole's headline, in its three clauses, the last two only when they count. */
interface Headline {
  /** How many parts: "Four parts". */
  readonly parts: string;
  /** How many things are worth a look, when any: "six worth a look". */
  readonly looks: string | undefined;
  /** How many parts couldn't be read, when any: "one not read". */
  readonly unread: string | undefined;
}

/** The whole counted as its headline says it. */
function headline(system: System): Headline {
  const looks = system.parts.reduce((sum, part) => sum + looksIn(part), 0);
  const unread = system.parts.filter((part) => part.health === "unread").length;
  const { length } = system.parts;
  return {
    parts: capital(`${numbered(length)} ${length === 1 ? "part" : "parts"}`),
    looks: looks === 0 ? undefined : `${numbered(looks)} worth a look`,
    unread: unread === 0 ? undefined : `${numbered(unread)} not read`,
  };
}

/** The headline as one plain sentence, for the title and the text twin. */
function sentence(said: Headline): string {
  const clauses = [said.looks, said.unread].filter((clause) => clause !== undefined);
  return `${said.parts}, ${clauses.length === 0 ? CLEAR : and(clauses)}.`;
}

/**
 * The one thing the whole most wants a look at, as a sentence: the first
 * item of the worst part, led by the part's name, since an item may start
 * with a name a capital would change, a machine's; or why the first part
 * not read wasn't. None when every part is good.
 */
function lede(system: System): string | undefined {
  for (const health of WORST) {
    const worst = system.parts.find((part) => part.health === health && part.side.length > 0);
    if (worst !== undefined) {
      return stopped(`${named(worst.part)}: ${worst.side[0] ?? ""}`);
    }
  }
  const unread = system.parts.find((part) => part.health === "unread");
  return unread === undefined
    ? undefined
    : stopped(`${named(unread.part)} couldn't be read: ${unread.says}`);
}

/** `words` ended as a sentence, once: an owner's words may end their own. */
function stopped(words: string): string {
  return /[.?!]$/u.test(words) ? words : `${words}.`;
}

/** `words` joined as a sentence lists them: a, b and c. */
function and(words: readonly string[]): string {
  if (words.length < 2) {
    return words.join("");
  }
  return `${words.slice(0, -1).join(", ")} and ${words.at(-1) ?? ""}`;
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
    address: text(record, "address"),
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
  return {
    part: name,
    health: healthOf(text(record, "health")),
    says: text(record, "says") ?? "",
    side: texts(record, "side") ?? [],
    holds: text(record, "holds"),
    things: (texts(record, "things") ?? [])
      .map((key) => thingOf(things, key, unread))
      .filter((thing) => thing !== undefined),
    deeper: { command: says("command"), site: says("site"), launcher: says("launcher") },
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

export type { Deeper, Headline, Health, Part, System, Thing };
export { CLEAR, headline, lede, looksIn, named, SKIES, sentence, systemOf, WORDS };
