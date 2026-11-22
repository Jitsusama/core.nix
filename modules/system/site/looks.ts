// What is worth a look, across every machine and every part, as the list
// the system page opens with: the page reads top to bottom, what wants
// Joel first and then each machine (Joel, 2026-11-17). Each look is one
// of the daemon's own, a machine's or a part's side, so the page judges
// nothing the `system` command doesn't; each says whose it is and leads
// to where it can be seen whole, and a machine's says why it matters, as
// the daemon does, with its one act, the launcher's row for that machine.
// Nothing here touches the page.

import { worstOf } from "./machine.ts";
import {
  capital,
  type Health,
  named,
  numbered,
  type Part,
  type System,
  type Thing,
  type ThingLook,
  thingsOf,
} from "./model.ts";

/** The most looks listed, so a bad night keeps the list a glance. */
const LOOKS_LISTED = 8;

/** How each health ranks in the list, the worst first. */
// Worst first as the daemon ranks them: a part not read at all before a
// look, since nothing it holds can be told.
const RANK: { readonly [health in Health]: number } = { unread: 0, bad: 1, watch: 2, good: 3 };

/** What a machine's side says of it when the node can't reach it, as the daemon says it. */
const OUT_OF_REACH = "out of reach";

/** The most machines out of reach the one look for them names, the rest counted. */
const AWAY_LISTED = 3;

/** The item a side ends with when it had more than it lists, as the daemon says it. */
const MORE = /^and \d+ more worth a look$/u;
// The note a part carries when read from its owner's last answer, as
// modules/system/src/kept.rs words it.
const KEPT = /^as read at /u;

/** One thing worth a look: how bad, whose it is, what it is and where it leads. */
interface Look {
  readonly health: Health;
  /** The machine, builder or part it is in, as the page names it. */
  readonly whose: string;
  /** Whether `whose` is a thing's name, which no translator may change. */
  readonly machine: boolean;
  readonly words: string;
  /**
   * Where it is seen whole: a machine's section here, a builder on the map,
   * or a part's own site.
   */
  readonly href: string;
  /** The row's key, the same across redraws while the look stands. */
  readonly row: string;
  /** Why it matters, as the daemon says it; empty where it says none. */
  readonly matters: string;
  /**
   * Its one act, the launcher's row for its machine, where Joel updates it or
   * starts a session there; none for a part's look, whose own site is where
   * it leads, or for a machine whose record gives no launcher link.
   */
  readonly act: string | undefined;
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
 * to the map. The part's own note, said when the machines were read from
 * their owner's last answer, is the page's note's to say (keptOf).
 */
function machineLooks(part: Part, drawn: number): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, "unread")];
  }
  return thingLooks(part, drawn);
}

/**
 * What the machines part says in `system` that no machine does: the note it
 * carries when it was read from its owner's last answer, as a sentence. Its
 * side also names each machine worth a look, "kestrel on battery at 9%", then
 * how many more; those are left to the machines' own looks, which clear
 * first when a machine's record changes before the part's, and wait with the
 * list's settling for a machine named before its record. The note is no
 * look: the daemon leaves a read missed once its kept answer is too old to
 * stand on (modules/system/src/kept.rs), so one it still stands on is how
 * the page was read, said where the page says what it couldn't read.
 */
function keptOf(system: System): string {
  const part = system.parts.find((each) => each.part === "machines");
  if (part === undefined || part.health === "unread") {
    return "";
  }
  // Matched by kept.rs's own words rather than by what isn't a machine's,
  // so a machine named in the side the page doesn't know yet never reads as
  // the note.
  const own = part.side.filter((words) => KEPT.test(words));
  return own.length === 0 ? "" : `Machines ${own.join("; ")}.`;
}

/**
 * Each thing's side, a look a word, leading to its section or the map. A
 * machine drawn whole is told by its place among those in reach, `drawn` of
 * them drawn. Those out of reach are one look between them, in the place
 * after those in reach, so many gone at once read as one thing to know and
 * leave room for the few that need Joel; anything else their sides say
 * stays a look of its own. A builder, which has no section here, leads to
 * itself on the map, after the machines in reach.
 */
function thingLooks(part: Part, drawn: number): readonly Look[] {
  const machines = thingsOf(part, "machine");
  const reached = machines.filter((thing) => thing.ties.reached);
  const away = machines.filter((thing) => !thing.ties.reached);
  const hrefOf = (thing: Thing): string => {
    if (thing.kind !== "machine") {
      return `../mycelium/#${thing.kind}=${encodeURIComponent(thing.name)}`;
    }
    const at = reached.indexOf(thing);
    if (at < 0) {
      return "#away";
    }
    return at < drawn
      ? `#machine-${at + 1}`
      : `../mycelium/#machine=${encodeURIComponent(thing.name)}`;
  };
  const own = (thing: Thing): readonly Look[] => {
    const looks = lookedOf(thing);
    return keyed(looks.map((look) => look.side))
      .map((keyedLook, at) => ({ ...keyedLook, matters: looks[at]?.matters ?? "" }))
      .filter(({ words }) => thing.ties.reached || words !== OUT_OF_REACH)
      .map(({ words, key, matters }) => ({
        health: thing.health,
        whose: thing.name,
        machine: true,
        words: capital(words),
        href: hrefOf(thing),
        row: `look:${thing.kind}:${encodeURIComponent(thing.name)}:${key}`,
        matters,
        act: thing.ties.acts.link,
      }));
  };
  // Those in reach first, so within a health the few that need Joel lead.
  return [
    ...reached.flatMap(own),
    ...part.things.filter((thing) => thing.kind !== "machine").flatMap(own),
    ...(away.length === 0 ? [] : [awayLook(away)]),
    ...away.flatMap(own),
  ];
}

/**
 * A machine's looks, each with its why: every one the daemon judged, where
 * its record lists them, or its side's, cut to a few and saying no why, from
 * a daemon too old to list them.
 */
function lookedOf(thing: Thing): readonly ThingLook[] {
  // An empty list beside a side that names looks is a record caught between
  // its two fields, so the side says them rather than the machine saying none.
  const told = thing.looks?.length === 0 && thing.side.length > 0 ? undefined : thing.looks;
  return told ?? thing.side.map((side) => ({ side, matters: "" }));
}

/**
 * The one look for every machine out of reach: how many, and the first few
 * by name. One alone says the daemon's why and opens in the launcher, as
 * any machine's look does; several lead to where each is listed with its
 * own, since the daemon's why speaks of one machine and no row is theirs.
 */
function awayLook(away: readonly Thing[]): Look {
  const [only] = away;
  if (only !== undefined && away.length === 1) {
    return {
      health: only.health,
      whose: only.name,
      machine: true,
      words: capital(OUT_OF_REACH),
      href: "#away",
      row: "look:away",
      matters: lookedOf(only).find((look) => look.side === OUT_OF_REACH)?.matters ?? "",
      act: only.ties.acts.link,
    };
  }
  const names = away.slice(0, AWAY_LISTED).map((thing) => thing.name);
  const past = away.length - names.length;
  const listed =
    past > 0
      ? `${names.join(", ")} and ${past.toLocaleString("en-CA")} more`
      : `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
  return {
    health: worstOf(away),
    whose: `${capital(numbered(away.length))} machines`,
    machine: false,
    words: `${capital(OUT_OF_REACH)}: ${listed}`,
    href: "#away",
    row: "look:away",
    matters: "",
    act: undefined,
  };
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
    matters: "",
    act: undefined,
  };
}

/**
 * Another part's looks: each of its side's, or why it couldn't be read. The
 * note a part carries when it was read from its owner's last answer is
 * none, at any health: it no more wants a look than the machines' does, and
 * its own section across the machines says it. A good part has no other.
 * Each look standing on that answer says when it was read, as up to ten
 * minutes old it may no longer hold, keyed by its own words so the row
 * stays the same row as the clock moves.
 */
function partLooks(part: Part): readonly Look[] {
  if (part.health === "unread") {
    return [lookOf(part, "unread", `Couldn't be read: ${part.says}`, "unread")];
  }
  if (part.health === "good") {
    return [];
  }
  const kept = part.side.find((words) => KEPT.test(words));
  const asRead = kept === undefined ? "" : `, ${kept.split(";")[0]}`;
  const own = part.side.filter((words) => !KEPT.test(words));
  // How many more the daemon counts is no look read at a time of its own.
  const when = (words: string): string => (MORE.test(words) ? words : `${words}${asRead}`);
  return keyed(own).map(({ words, key }) => lookOf(part, part.health, when(words), key));
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
export { keptOf, LOOKS_LISTED, looksOf };
