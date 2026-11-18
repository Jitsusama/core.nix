// What the system page says of each machine: the words its ties to the
// other three parts (ties.ts) are said in, over all the machines and for
// each one. A machine is the bridge between the others (Joel, 2026-11-17),
// so the page leads with them. Nothing here touches the page.

import {
  capital,
  type Health,
  named,
  numbered,
  type Part,
  type System,
  type Thing,
  WORDS,
} from "./model.ts";
import type { Figures, Place, Ties } from "./ties.ts";

/** A gigabyte and a terabyte, as a node counts bytes. */
const GIGABYTE = 1e9;
const TERABYTE = 1e12;

/** The bars at or over which a radio's signal is said strong, and fair. */
const STRONG_BARS = 4;
const FAIR_BARS = 3;

/** A full charge. */
const FULL = 100;

/** How a sentence ends. */
const ENDED = /[.?!]$/u;

/** How each system a node names is said. */
const SYSTEMS: { readonly [system: string]: string } = {
  linux: "Linux",
  macos: "macOS",
};

/** How the lede places what another part says, after "And". */
const WHERE: { readonly [part: string]: string } = {
  mycelium: "in the mycelium",
  network: "on the network",
  code: "in your code",
};

/** How the lede names another part as what couldn't be read. */
const WHAT: { readonly [part: string]: string } = {
  mycelium: "the mycelium",
  network: "the network",
  code: "your code",
};

/** How many machines the lede names, so thousands keep it a sentence or two. */
const LEDE_MACHINES = 3;

/** How a machine's health is said in the headline, worst first. */
const STANDS: readonly (readonly [Health, string])[] = [
  ["bad", "in trouble"],
  ["watch", "worth a look"],
  ["unread", "not read"],
];

/** A tie said: its main words and what goes beside them. */
interface Said {
  readonly said: string;
  readonly aside: string;
}

/** The headline over the machines: how many, then how they stand. */
interface MachineHeadline {
  readonly machines: string;
  readonly stand: string;
  /** The worst of them, which the way they stand is drawn in. */
  readonly health: Health;
}

/** `count` of `one`, its plural `many` past one, as a sentence says it. */
function many(count: number, one: string, more = `${one}s`): string {
  return `${count.toLocaleString("en-CA")} ${count === 1 ? one : more}`;
}

/** How many of `all` `count` is, as the headline says it: both, all, or the count. */
function howMany(count: number, all: number): string {
  if (count === all && all === 2) {
    return "both";
  }
  return count === all && all > 2 ? "all" : numbered(count);
}

/** The headline over the machines: how many there are, then how they stand. */
function machineHeadline(system: System): MachineHeadline {
  const things = system.parts.find((part) => part.part === "machines")?.things ?? [];
  const { length } = things;
  const machines = capital(`${numbered(length)} ${length === 1 ? "machine" : "machines"}`);
  if (length === 0) {
    return { machines, stand: "none known", health: "unread" };
  }
  const stands = STANDS.map(([health, words]) => {
    const count = things.filter((thing) => thing.health === health).length;
    return { health, count, words };
  }).filter((each) => each.count > 0);
  const [worst] = stands;
  if (worst === undefined) {
    return { machines, stand: "all ready", health: "good" };
  }
  const stand = stands.map((each) => `${howMany(each.count, length)} ${each.words}`).join(" and ");
  return { machines, stand, health: worst.health };
}

/** `words` ended as a sentence, once. */
function stopped(words: string): string {
  return ENDED.test(words) ? words : `${words}.`;
}

/**
 * The lede: what each machine worth a look has, the worst first and its
 * name leading, since a name keeps its case; then the worst of the other
 * parts' first look, or else why one wasn't read. None when nothing
 * anywhere wants a look.
 */
function machineLede(system: System): string | undefined {
  const things = system.parts.find((part) => part.part === "machines")?.things ?? [];
  const machines = things
    .filter((thing) => thing.side.length > 0)
    .slice(0, LEDE_MACHINES)
    .map((thing) => stopped(`${thing.name}: ${thing.side.join("; ")}`));
  const rest = restOf(system.parts.filter((part) => part.part !== "machines"));
  const said = [...machines, ...(rest === undefined ? [] : [rest])];
  return said.length === 0 ? undefined : said.join(" ");
}

/** The worst of the other parts' first look, or else why one wasn't read. */
function restOf(others: readonly Part[]): string | undefined {
  const other = ["bad", "watch"]
    .map((health) => others.find((part) => part.health === health && part.side.length > 0))
    .find((part) => part !== undefined);
  if (other !== undefined) {
    return stopped(`And ${WHERE[other.part] ?? `in ${named(other.part)}`}: ${other.side[0] ?? ""}`);
  }
  const unread = others.find((part) => part.health === "unread");
  return unread === undefined
    ? undefined
    : stopped(`And ${WHAT[unread.part] ?? named(unread.part)} couldn't be read: ${unread.says}`);
}

/** What kind of machine it is: its model and its system, and whether it's this one. */
function kindOf(thing: Thing, here: string): string | undefined {
  const { model, system } = thing.ties.figures;
  const words = [model, system === undefined ? undefined : (SYSTEMS[system] ?? system)];
  const said = words.filter((word) => word !== undefined);
  if (said.length === 0) {
    return undefined;
  }
  return [...said, ...(thing.name === here ? ["this machine"] : [])].join(", ");
}

/** How a machine stands, in a line: its health's word, then what is worth a look. */
function standing(thing: Thing, items = thing.side.length): string {
  const side = thing.side.slice(0, items);
  if (side.length === 0) {
    return thing.health === "good" ? `${WORDS.good}: ready` : WORDS[thing.health];
  }
  return `${WORDS[thing.health]}: ${side.join("; ")}`;
}

const NOT_READ = "Not read";

/** A machine's cells, busy and idle, with those that have ended aside. */
function cellsSaid(ties: Ties): Said {
  const { cells } = ties;
  if (cells === undefined) {
    return { said: NOT_READ, aside: "The mycelium's health couldn't be had." };
  }
  const busy = cells.busy === 0 ? "None" : cells.busy.toLocaleString("en-CA");
  return {
    said: `${busy} busy and ${cells.idle.toLocaleString("en-CA")} idle`,
    aside: cells.ended === 0 ? "None has ended." : `And ${cells.ended} that have ended.`,
  };
}

/**
 * Where `place` meets the network, as a clause, its first word a capital
 * when it `leads`; Wi-Fi is a name, so it keeps its own either way.
 */
function meets(place: Place, leads: boolean): string {
  const point = place.point === undefined ? "an unnamed point" : `the ${place.point} point`;
  if (place.by === "wired") {
    return `${leads ? "Wired" : "wired"} to ${point}`;
  }
  if (place.by === undefined) {
    return `${leads ? "On" : "on"} ${point}`;
  }
  return `Wi-Fi to ${point}, on ${place.by}`;
}

/** A radio's signal, said from its bars. */
function signal(bars: number): string {
  if (bars >= STRONG_BARS) {
    return "A strong signal";
  }
  return bars >= FAIR_BARS ? "A fair signal" : "A weak signal";
}

/**
 * Where a machine meets the network, a clause for each place, wired first
 * as the network lists them, and how well: its radio's signal, and the
 * most room any place has. Its device is the first place's, to go to.
 */
function networkSaid(ties: Ties): Said & { readonly device: string | undefined } {
  const places = ties.network;
  if (places === undefined) {
    return { said: NOT_READ, aside: "The network's answer couldn't be had.", device: undefined };
  }
  if (places.length === 0) {
    return {
      said: "Not seen on the home network",
      aside: "No device on it carries this machine's name.",
      device: undefined,
    };
  }
  const said = places.map((place, at) => meets(place, at === 0)).join(", and ");
  const radio = places.find((place) => place.by !== "wired" && place.bars !== undefined);
  const rooms = places.map((place) => place.room).filter((mbps) => mbps !== undefined);
  const room = rooms.length === 0 ? undefined : `${Math.round(Math.max(...rooms))} Mb/s of room`;
  const how = radio?.bars === undefined ? undefined : signal(radio.bars);
  const aside = asideOf(how, room);
  return { said, aside, device: places[0]?.device };
}

/** The aside under a machine's network line: its signal, then its room, as known. */
function asideOf(how: string | undefined, room: string | undefined): string {
  if (how !== undefined) {
    return `${how}${room === undefined ? "" : `, with ${room}`}.`;
  }
  return room === undefined ? "How much room it has isn't measured." : `With ${room}.`;
}

/** A machine's code, counted where it was read, and said not read from another. */
function codeSaid(ties: Ties, name: string): Said & { readonly read: boolean } {
  const { code } = ties;
  if (code === undefined) {
    return {
      said: "Not read from here",
      aside: `Its code is read on ${name} itself.`,
      read: false,
    };
  }
  const aside = [
    many(code.trees, "tree"),
    `${many(code.behind, "checkout")} behind their upstream`,
    `${many(code.changes, "open change")}`,
  ].join("; ");
  return {
    said: `${many(code.repositories, "repository", "repositories")} in ${many(code.checkouts, "checkout")}`,
    aside: `${aside}.`,
    read: true,
  };
}

/** A size in bytes, in gigabytes, or terabytes from a thousand of them. */
function size(bytes: number): string {
  if (bytes >= TERABYTE) {
    const terabytes = Math.round((bytes / TERABYTE) * 10) / 10;
    return `${terabytes.toLocaleString("en-CA")} TB`;
  }
  return `${Math.round(bytes / GIGABYTE).toLocaleString("en-CA")} GB`;
}

/** A machine's power, as a clause. */
function power(charge: number, onBattery: boolean | undefined): string {
  if (onBattery === true) {
    return `${charge}% charged, on battery`;
  }
  return charge >= FULL ? "fully charged and plugged in" : `${charge}% charged and plugged in`;
}

/** A machine's last update, as a sentence, when its node says one. */
function updated(figures: Figures, when: (at: string) => string): string | undefined {
  if (figures.updatedAt === undefined) {
    return undefined;
  }
  const at = when(figures.updatedAt);
  switch (figures.updateStep) {
    case "done":
      return `Updated ${at}.`;
    case "unchanged":
      return `Unchanged at its update ${at}.`;
    case undefined:
      return `Last updated ${at}.`;
    default:
      return `Its update ${at} says ${figures.updateStep}.`;
  }
}

/**
 * A machine itself: its disk's room, then its processor, memory and power,
 * then its last update, each said only when its node says it. `when` says
 * a time, as "at 02:19".
 */
function itselfSaid(ties: Ties, when: (at: string) => string): Said {
  const { figures } = ties;
  const said =
    figures.diskFree === undefined
      ? "Its disk not said"
      : `${size(figures.diskFree)}${figures.disk === undefined ? "" : ` of ${size(figures.disk)}`} free`;
  const clauses: string[] = [];
  if (figures.cpuBusy !== undefined) {
    clauses.push(`processor ${Math.round(figures.cpuBusy * FULL)}% busy`);
  }
  if (figures.memoryAvailable !== undefined && figures.memory !== undefined) {
    const free = Math.round(figures.memoryAvailable / GIGABYTE);
    clauses.push(`${free} of ${size(figures.memory)} of memory free`);
  }
  if (figures.charge !== undefined) {
    clauses.push(power(figures.charge, figures.onBattery));
  }
  const sentences = [
    clauses.length === 0 ? undefined : stopped(capital(clauses.join(", "))),
    updated(figures, when),
  ].filter((sentence) => sentence !== undefined);
  return { said, aside: sentences.join(" ") };
}

export type { MachineHeadline, Said };
export {
  cellsSaid,
  codeSaid,
  itselfSaid,
  kindOf,
  machineHeadline,
  machineLede,
  networkSaid,
  standing,
};
