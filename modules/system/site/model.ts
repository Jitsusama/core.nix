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

/** One part of the system, as the page draws its row and its panel. */
interface Part {
  readonly part: string;
  readonly health: Health;
  readonly says: string;
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

/** How each health is said beside its mark. */
const WORDS: { readonly [health in Health]: string } = {
  good: "Good",
  watch: "Worth a look",
  bad: "Bad",
  unread: "Not read",
};

/** The health the daemon's word `value` says, or unread when it says none. */
function healthOf(value: string | undefined): Health {
  return (value === undefined ? undefined : HEALTHS.get(value)) ?? "unread";
}

/** The part's name as the page says it. */
function named(part: string): string {
  return NAMES[part] ?? part;
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
    machine: text(summary, "machine") ?? "",
    readAt: text(summary, "read_at") ?? "",
    health: healthOf(text(summary, "health")),
    parts: (names ?? [])
      .map((name) => partOf(parts, things, name, unread))
      .filter((part) => part !== undefined),
    unread,
  };
}

/** The whole in one sentence, worst first: bad, then worth a look, then unread. */
function judged(system: System): string {
  const by = (health: Health): string[] =>
    system.parts.filter((part) => part.health === health).map((part) => named(part.part));
  const said: string[] = [];
  const bad = by("bad");
  const watch = by("watch");
  const unread = by("unread");
  const verb = (many: readonly string[]): string => (many.length === 1 ? "is" : "are");
  if (bad.length > 0) {
    said.push(`${and(bad)} ${verb(bad)} bad`);
  }
  if (watch.length > 0) {
    said.push(`${and(watch)} ${verb(watch)} worth a look`);
  }
  if (unread.length > 0) {
    said.push(`${and(unread)} couldn't be read`);
  }
  if (said.length === 0) {
    return "Every part is good.";
  }
  const sentence = and(said);
  return `${sentence.charAt(0).toUpperCase()}${sentence.slice(1)}.`;
}

export type { Deeper, Health, Part, System, Thing };
export { judged, named, systemOf, WORDS };
