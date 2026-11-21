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
  const part = system.parts.find((each) => each.part === "machines");
  const things = part?.things ?? [];
  // A part not told, or naming machines whose records haven't come, isn't none.
  const awaited = part === undefined || (things.length === 0 && part.untold > 0);
  if (awaited || part.health === "unread") {
    return { machines: "Your machines", stand: "not read", health: "unread" };
  }
  // Those named whose records haven't come are counted, as not read.
  const all = things.length + part.untold;
  const machines = capital(`${numbered(all)} ${all === 1 ? "machine" : "machines"}`);
  if (all === 0) {
    return { machines, stand: "none known", health: "unread" };
  }
  const stands = STANDS.map(([health, words]) => {
    const untold = health === "unread" ? part.untold : 0;
    const count = things.filter((thing) => thing.health === health).length + untold;
    return { health, count, words };
  }).filter((each) => each.count > 0);
  const [worst] = stands;
  if (worst === undefined) {
    return { machines, stand: "all ready", health: "good" };
  }
  const stand = stands.map((each) => `${howMany(each.count, all)} ${each.words}`).join(" and ");
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
  const part = system.parts.find((each) => each.part === "machines");
  if (part?.health === "unread") {
    return stopped(`The machines couldn't be read: ${part.says}`);
  }
  const things = part?.things ?? [];
  const machines = things
    .filter((thing) => thing.side.length > 0)
    .slice(0, LEDE_MACHINES)
    .map((thing) => stopped(`${thing.name}: ${thing.side.join("; ")}`));
  const rest = restOf(system.parts.filter((other) => other.part !== "machines"));
  const said = [...machines, ...(rest === undefined ? [] : [rest])];
  return said.length === 0 ? undefined : said.join(" ");
}

/** The worst of the other parts' first look, or else why one wasn't read. */
function restOf(others: readonly Part[]): string | undefined {
  const other = ["bad", "watch"]
    .map((health) => others.find((part) => part.health === health && part.side.length > 0))
    .find((part) => part !== undefined);
  if (other !== undefined) {
    const where = Object.hasOwn(WHERE, other.part) ? WHERE[other.part] : `in ${named(other.part)}`;
    return stopped(`And ${where}: ${other.side[0] ?? ""}`);
  }
  const unread = others.find((part) => part.health === "unread");
  return unread === undefined
    ? undefined
    : stopped(
        `And ${Object.hasOwn(WHAT, unread.part) ? WHAT[unread.part] : named(unread.part)} couldn't be read: ${unread.says}`,
      );
}

/** How a node's system is said, its own word for one the page doesn't know. */
function systemSaid(system: string | undefined): string | undefined {
  if (system === undefined) {
    return undefined;
  }
  return Object.hasOwn(SYSTEMS, system) ? SYSTEMS[system] : system;
}

/** What kind of machine it is: its model and its system, and whether it's this one. */
function kindOf(thing: Thing, here: string): string | undefined {
  const { model, system } = thing.ties.figures;
  const words = [model, systemSaid(system)];
  const said = words.filter((word) => word !== undefined);
  const itself = thing.name === here;
  if (said.length === 0) {
    return itself ? "This machine" : undefined;
  }
  return [...said, ...(itself ? ["this machine"] : [])].join(", ");
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

/** `said`, with `note` beside it when the tie was read from an owner's kept answer. */
function keptSaid<T extends Said>(said: T, note: string | undefined): T {
  if (note === undefined || note === "") {
    return said;
  }
  const told = `${note.charAt(0).toUpperCase()}${note.slice(1)}.`;
  return { ...said, aside: said.aside === "" ? told : `${said.aside} ${told}` };
}

/** A machine's cells, busy and idle, with those that have ended aside. */
function cellsSaid(ties: Ties): Said {
  return keptSaid(cellsNow(ties), ties.kept.cells);
}

function cellsNow(ties: Ties): Said {
  const { cells } = ties;
  if (cells === undefined) {
    return { said: NOT_READ, aside: "The mycelium's health couldn't be had." };
  }
  const counted = (count: number): string => (count === 0 ? "none" : count.toLocaleString("en-CA"));
  const said =
    cells.busy === 0 && cells.idle === 0
      ? "None busy or idle"
      : capital(`${counted(cells.busy)} busy and ${counted(cells.idle)} idle`);
  return {
    said,
    aside:
      cells.ended === 0
        ? "None has ended."
        : `And ${cells.ended.toLocaleString("en-CA")} that ${cells.ended === 1 ? "has" : "have"} ended.`,
  };
}

/**
 * Where `place` meets the network, as a clause, its first word a capital
 * when it `leads`; Wi-Fi is a name, so it keeps its own either way.
 */
function meets(place: Place, leads: boolean): string {
  const point = pointSaid(place);
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
    return "a strong signal";
  }
  return bars >= FAIR_BARS ? "a fair signal" : "a weak signal";
}

/**
 * How well one place reaches, as far as it's measured: a radio's signal,
 * then its room, as a clause.
 */
function reach(place: Place): string | undefined {
  const how = place.by === "wired" || place.bars === undefined ? undefined : signal(place.bars);
  const room = place.room === undefined ? undefined : `${Math.round(place.room)} Mb/s of room`;
  const said = how === undefined ? room : `${how}${room === undefined ? "" : `, with ${room}`}`;
  if (said === undefined) {
    return undefined;
  }
  return said;
}

/** A place's point, by its name, or unnamed where its path names none. */
function pointSaid(place: Place): string {
  return place.point === undefined ? "an unnamed point" : `the ${place.point} point`;
}

/**
 * Where one place meets the network, leading its figures as a sentence
 * of their own: its point as the said line names it, then its band or wire.
 */
function placeLeads(place: Place): string {
  const point = pointSaid(place);
  return place.by === undefined ? `At ${point}` : `At ${point}, ${place.by}`;
}

/** Where one place meets the network, in a few words, as the bridge labels it. */
function placeWords(place: Place): string {
  const point = place.point ?? "a point";
  return place.by === undefined ? point : `${point}, ${place.by}`;
}

/**
 * Where a machine meets the network, a clause for each place in the order
 * its record lists them, and how well each reaches, in the same order,
 * so no place's figure is said as another's. Two places or more each lead
 * their figures with their name, since two alike read as one said twice.
 * Its device is the first place's, to go to.
 */
function networkSaid(ties: Ties): Said & { readonly device: string | undefined } {
  return keptSaid(networkNow(ties), ties.kept.network);
}

function networkNow(ties: Ties): Said & { readonly device: string | undefined } {
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
  // A place not measured keeps its turn among those that are, so each
  // figure stays beside the place it was said after.
  const reaches = places.map(reach);
  const byPlace = places.map((place, at) => {
    const reached = reaches[at] ?? "not measured";
    return places.length === 1 ? reached : `${placeLeads(place)}: ${reached}`;
  });
  const aside = reaches.every((reached) => reached === undefined)
    ? "How much room it has isn't measured."
    : stopped(capital(byPlace.join(". ")));
  return { said, aside, device: places[0]?.device };
}

/** `count` of a thing as `many` says it, and no `one` for none. */
function noneOr(count: number, one: string, more = `${one}s`): string {
  return count === 0 ? `no ${one}` : many(count, one, more);
}

/**
 * A machine's code, counted where it was read, and said not read from
 * another; on the machine it's read on, `here`, no code is a read that
 * failed.
 */
function codeSaid(ties: Ties, name: string, here: boolean): Said & { readonly read: boolean } {
  return keptSaid(codeNow(ties, name, here), ties.kept.code);
}

function codeNow(ties: Ties, name: string, here: boolean): Said & { readonly read: boolean } {
  const { code } = ties;
  if (code === undefined) {
    return here
      ? { said: NOT_READ, aside: "Repos couldn't be had.", read: false }
      : { said: "Not read from here", aside: `Its code is read on ${name} itself.`, read: false };
  }
  const behind =
    code.behind <= 1
      ? `${noneOr(code.behind, "checkout")} behind its upstream`
      : `${many(code.behind, "checkout")} behind their upstreams`;
  const aside = [noneOr(code.trees, "tree"), behind, noneOr(code.changes, "open change")].join(
    "; ",
  );
  return {
    said: `${noneOr(code.checkouts, "checkout")} of ${noneOr(code.repositories, "repository", "repositories")}`,
    aside: stopped(capital(aside)),
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
  const charged = charge >= FULL ? "fully charged" : `${charge}% charged`;
  return onBattery === false ? `${charged} and plugged in` : charged;
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

/** The months as a date says them, short. */
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** A day's milliseconds. */
const DAY = 86_400_000;

/** The start of the local day `time` falls in. */
function dayOf(time: number): number {
  const day = new Date(time);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/**
 * A time as the ties say it, seen from `now`, locally: "at 02:19" today,
 * "yesterday at 23:05", or "on 12 Nov at 09:00" on any other day, so an
 * update from days ago never reads as this morning's. A day of another
 * year says its year.
 */
function whenSaid(at: string, now: number): string {
  const time = Date.parse(at);
  if (Number.isNaN(time)) {
    return "at an unknown time";
  }
  const when = new Date(time);
  const clock = `${String(when.getHours()).padStart(2, "0")}:${String(when.getMinutes()).padStart(2, "0")}`;
  const today = dayOf(now);
  // Half a day past a day, so a day of 23 or 25 hours still lands on the next.
  const tomorrow = dayOf(today + DAY + DAY / 2);
  if (time >= today && time < tomorrow) {
    return `at ${clock}`;
  }
  if (time < today && time >= dayOf(today - DAY / 2)) {
    return `yesterday at ${clock}`;
  }
  const year = when.getFullYear() === new Date(now).getFullYear() ? "" : ` ${when.getFullYear()}`;
  return `on ${when.getDate()} ${MONTHS[when.getMonth()] ?? ""}${year} at ${clock}`;
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
  placeWords,
  standing,
  whenSaid,
};
