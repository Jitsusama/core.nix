// What the system page says of each machine: the ties a machine record
// carries to the other three parts (modules/system/src/parts/ties.rs), read
// leniently, and the words each is said in, over all the machines and for
// each one. A machine is the bridge between the others (Joel, 2026-11-17),
// so the page leads with them. Nothing here touches the page.

import { type Fields, isObject, number, object, text, texts } from "../../local-sites/kit/records.ts";
import { capital, type Health, named, numbered, type System, type Thing, WORDS } from "./model.ts";

/** A gigabyte and a terabyte, as a node counts bytes. */
const GIGABYTE = 1e9;
const TERABYTE = 1e12;

/** The bars at or over which a radio's signal is said strong, and fair. */
const STRONG_BARS = 4;
const FAIR_BARS = 3;

/** A full charge. */
const FULL = 100;

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

/** How many machines the lede names, so thousands keep it a sentence or two. */
const LEDE_MACHINES = 3;

/** How a machine's health is said in the headline, worst first. */
const STANDS: readonly (readonly [Health, string])[] = [
  ["bad", "in trouble"],
  ["watch", "worth a look"],
  ["unread", "not read"],
];

const EMPTY: Fields = {};

/** The mycelium's cells on a machine. */
interface Cells {
  readonly busy: number;
  readonly idle: number;
  readonly ended: number;
}

/** Where a machine meets the network: one device the network names for it. */
interface Place {
  readonly device: string | undefined;
  /** Its band, or "wired". */
  readonly by: string | undefined;
  readonly point: string | undefined;
  readonly bars: number | undefined;
  readonly room: number | undefined;
}

/** The code a machine holds, counted. */
interface Code {
  readonly repositories: number;
  readonly checkouts: number;
  readonly trees: number;
  readonly behind: number;
  readonly changes: number;
}

/** A machine's own figures, as its node says them. */
interface Figures {
  readonly model: string | undefined;
  readonly system: string | undefined;
  readonly disk: number | undefined;
  readonly diskFree: number | undefined;
  readonly cpuBusy: number | undefined;
  readonly memory: number | undefined;
  readonly memoryAvailable: number | undefined;
  readonly charge: number | undefined;
  readonly onBattery: boolean | undefined;
  readonly updatedAt: string | undefined;
  readonly updateStep: string | undefined;
}

/** What Joel can do with a machine: the launcher's link and the commands. */
interface Acts {
  readonly link: string | undefined;
  readonly commands: readonly string[];
}

/** What ties a machine to the other parts; each tie none when it wasn't read. */
interface Ties {
  readonly cells: Cells | undefined;
  readonly network: readonly Place[] | undefined;
  readonly code: Code | undefined;
  readonly figures: Figures;
  readonly acts: Acts;
}

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

/** A count the record says, or none when it says none or a negative. */
function counted(record: Fields, field: string): number | undefined {
  const value = number(record, field);
  return value === undefined || value < 0 ? undefined : value;
}

/** A yes or no the record says, or none. */
function flag(record: Fields, field: string): boolean | undefined {
  const value = record[field];
  return typeof value === "boolean" ? value : undefined;
}

function cellsOf(said: Fields | undefined): Cells | undefined {
  if (said === undefined) {
    return undefined;
  }
  return {
    busy: counted(said, "busy") ?? 0,
    idle: counted(said, "idle") ?? 0,
    ended: counted(said, "ended") ?? 0,
  };
}

function placesOf(said: unknown): readonly Place[] | undefined {
  if (!Array.isArray(said)) {
    return undefined;
  }
  return said.filter(isObject).map((place) => ({
    device: text(place, "device"),
    by: text(place, "by"),
    point: text(place, "point"),
    bars: counted(place, "bars"),
    room: counted(place, "room_mbps"),
  }));
}

function codeOf(said: Fields | undefined): Code | undefined {
  if (said === undefined) {
    return undefined;
  }
  const of = (field: string): number => counted(said, field) ?? 0;
  return {
    repositories: of("repositories"),
    checkouts: of("checkouts"),
    trees: of("trees"),
    behind: of("behind"),
    changes: of("changes"),
  };
}

function figuresOf(said: Fields): Figures {
  return {
    model: text(said, "model"),
    system: text(said, "system"),
    disk: counted(said, "disk"),
    diskFree: counted(said, "disk_free"),
    cpuBusy: counted(said, "cpu_busy"),
    memory: counted(said, "memory"),
    memoryAvailable: counted(said, "memory_available"),
    charge: counted(said, "charge"),
    onBattery: flag(said, "on_battery"),
    updatedAt: text(said, "updated_at"),
    updateStep: text(said, "update_step"),
  };
}

/**
 * A machine's ties as its record says them, each none that the record
 * doesn't say, so a daemon from before ties draws its machines bare.
 */
function tiesOf(value: unknown): Ties {
  const said = isObject(value) ? value : EMPTY;
  const acts = object(said, "acts") ?? EMPTY;
  return {
    cells: cellsOf(object(said, "cells")),
    network: placesOf(said["network"]),
    code: codeOf(object(said, "code")),
    figures: figuresOf(object(said, "figures") ?? EMPTY),
    acts: { link: text(acts, "link"), commands: texts(acts, "commands") ?? [] },
  };
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
  }).filter((stand) => stand.count > 0);
  const worst = stands[0];
  if (worst === undefined) {
    return { machines, stand: "all ready", health: "good" };
  }
  const stand = stands
    .map((each) => `${howMany(each.count, length)} ${each.words}`)
    .join(" and ");
  return { machines, stand, health: worst.health };
}

/** `words` ended as a sentence, once. */
function stopped(words: string): string {
  return /[.?!]$/u.test(words) ? words : `${words}.`;
}

/**
 * The lede: what each machine worth a look has, the worst first and its
 * name leading, since a name keeps its case; then the worst of the other
 * parts' first look. None when nothing anywhere wants a look.
 */
function machineLede(system: System): string | undefined {
  const things = system.parts.find((part) => part.part === "machines")?.things ?? [];
  const machines = things
    .filter((thing) => thing.side.length > 0)
    .slice(0, LEDE_MACHINES)
    .map((thing) => stopped(`${thing.name}: ${thing.side.join("; ")}`));
  const others = system.parts.filter((part) => part.part !== "machines");
  const other = ["bad", "watch"]
    .map((health) => others.find((part) => part.health === health && part.side.length > 0))
    .find((part) => part !== undefined);
  const rest =
    other === undefined
      ? []
      : [stopped(`And ${WHERE[other.part] ?? `in ${named(other.part)}`}: ${other.side[0] ?? ""}`)];
  const said = [...machines, ...rest];
  return said.length === 0 ? undefined : said.join(" ");
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
  const rooms = places.map((place) => place.room).filter((room) => room !== undefined);
  const room = rooms.length === 0 ? undefined : `${Math.round(Math.max(...rooms))} Mb/s of room`;
  const how = radio?.bars === undefined ? undefined : signal(radio.bars);
  const aside =
    how === undefined
      ? room === undefined
        ? "How much room it has isn't measured."
        : `With ${room}.`
      : `${how}${room === undefined ? "" : `, with ${room}`}.`;
  return { said, aside, device: places[0]?.device };
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

export type { Acts, Cells, Code, Figures, MachineHeadline, Place, Said, Ties };
export {
  cellsSaid,
  codeSaid,
  itselfSaid,
  kindOf,
  machineHeadline,
  machineLede,
  networkSaid,
  standing,
  tiesOf,
};
