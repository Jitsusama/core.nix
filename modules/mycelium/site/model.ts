// What the map draws, made from the node's records as a watch holds them:
// the same `now` that now.json held while a timer wrote it, so the drawing
// (view.js) reads the live feed without a change to how it draws.
//
// The networks and cells are the very objects `mycelium networks -o json`
// prints, made by the node's own model (its `network` and `cell` kinds), so
// the map and the command never differ and nothing here re-derives them. A
// record adds its `key` and `heard`, and drops its `kind`, which is the
// interface's; both are put back as the command says them. The machines add
// what their cells leave out (memory, disk, power, room), a basin's machines
// folded into the one stand the map draws of it, and the messages only who
// sent, to whom and when, which is all the feed passes of them. The
// builders the machine sends its builds to are the node's own records too
// (its `builder` kind), apart from the cells, since nothing runs on them
// but builds.
// What waits on the mesh, each question still waiting on a peer and each
// hold still standing, is the node's own judgement (its `unanswered` kind),
// read likewise without its words. The ties between cells, each pair that
// talked in the last day, are the node's too (its `tie` kind): how many
// messages each end sent and of what kinds, never what they said, with a
// strength the map fades to now. A feeding (its `fed` kind) is who fed
// whom, when and the kind in a word, never the change itself; who started
// each cell is read off its record, else its forage (lineage.ts).

import {
  type Fields,
  type Held,
  isObject,
  number,
  object,
  text,
  texts,
} from "../../local-sites/kit/records.ts";
import { BALANCE_FRESH_MS, type Balance, balanceOf, latestReading, sharesOf } from "./balance.ts";
import { foragedOf, type Started, startedOf } from "./lineage.ts";
import type { Builder } from "./words.ts";

/** How many messages the map hears, the latest, as `mycelium messages` says. */
const LATEST_MESSAGES = 20;

/**
 * How long a tie's strength takes to halve, as the node grows it: two
 * hours, its own `HALF_LIFE`, so a tie said as it stood at its last
 * message reads as it stands now.
 */
const TIE_HALF_LIFE_MS = 2 * 3_600_000;

/**
 * How long a feeding is drawn after it was said: ten minutes, the node's own
 * `SHOWN_FOR`, so the map lets the line go when the node stops serving it,
 * and before, where the record lingers in a watch with nothing else moving.
 */
const FED_SHOWN_MS = 600_000;

/**
 * How far ahead of the page's clock a feeding may be said and still be drawn:
 * a minute, for clocks a little apart. A feeding is an event with its own ten
 * minutes, so one said further ahead would stay up past them.
 */
const FED_AHEAD_MS = 60_000;

/** What joins a tie's two ends in its key, which no cell's name holds. */
const TIE_SEPARATOR = "~";

/**
 * The node's records, a watch's for each kind the map reads. The balance is
 * the balance service's latest reading, which a node serves only where the
 * service runs, so a map without it draws as before; the builders are
 * watched the same way, so a node too old to serve them draws none. What waits on the
 * mesh, and who started each cell, is there only once its watch has
 * answered, since a node too old to serve it would otherwise read as one
 * with nothing waiting, or no cell started by another.
 */
interface Heard {
  readonly network: Held;
  readonly cell: Held;
  readonly machine: Held;
  readonly message: Held;
  readonly balance?: Held;
  readonly builder?: Held;
  readonly unanswered?: Held | undefined;
  readonly tie?: Held | undefined;
  readonly fed?: Held | undefined;
  readonly forage?: Held | undefined;
}

/**
 * A receptor feeding a cell: the receptor's address, the cell's, when it
 * was fed and of what kind, a word such as `review`.
 */
interface Fed {
  readonly receptor: string;
  readonly cell: string;
  readonly at: string;
  readonly kind: string | null;
}

/**
 * Two cells that talked in the last day: their names, the one that sorts
 * first first; when the last message between them was sent; how many each
 * sent in the last hour and the day, in the order of `ends`; the kinds
 * their messages were; and the tie's strength as it stands now.
 */
interface Tie {
  readonly ends: readonly [string, string];
  readonly at: string;
  readonly hour: unknown;
  readonly today: unknown;
  readonly intents: unknown;
  readonly strength: number;
}

/**
 * What waits on the mesh as the map draws it: who sent it, a question or a
 * hold, where it went and since when; for a question, whom it waits on, and
 * for a hold, until when it stands at most.
 */
interface Wait {
  readonly id: unknown;
  readonly from: unknown;
  readonly intent: unknown;
  readonly groups: unknown;
  readonly since: string;
  readonly re: unknown;
  readonly waiting_on: unknown;
  readonly until: unknown;
}

/** A message as the map draws it: who sent, of what intent, to which groups. */
interface Message {
  readonly at: string;
  readonly from: unknown;
  readonly intent: unknown;
  readonly groups: unknown;
}

/** What the map draws, as now.json held it. */
interface Now {
  readonly at: string;
  readonly networks: readonly Fields[];
  readonly cells: readonly Fields[];
  readonly machines: readonly Fields[];
  readonly builders: readonly Builder[];
  readonly messages: readonly Message[];
  readonly balance: Balance | null;
  readonly waits: readonly Wait[] | null;
  readonly ties: readonly Tie[] | null;
  readonly fed: readonly Fed[] | null;
  readonly started: readonly Started[] | null;
  /** Every cell the forage page lists, by its address. */
  readonly foraged: readonly string[] | null;
}

/**
 * `value` with every Unicode ellipsis written as three full stops: window
 * titles arrive cut with one, and the map writes a cut the same way
 * whoever made it.
 */
function plain(value: unknown): unknown {
  if (typeof value === "string") {
    return value.replace(/\u2026/gu, "...");
  }
  if (Array.isArray(value)) {
    return value.map(plain);
  }
  if (isObject(value)) {
    return Object.fromEntries(Object.entries(value).map(([field, each]) => [field, plain(each)]));
  }
  return value;
}

/** `record` as the command prints it: its own kind, and no key or heard. */
function asPrinted(record: Fields, kind: string): Fields {
  const { key: _key, heard: _heard, ...fields } = record;
  return { kind, ...fields };
}

/** The field `field` of `record` when it is an object, else an empty one. */
function within(record: Fields, field: string): Fields {
  const value = record[field];
  return isObject(value) ? value : {};
}

/**
 * A basin's name as the node reads one, a DNS label: lower-case letters,
 * digits and hyphens, at most 63, never a hyphen at either end, so a name
 * the node would refuse is never drawn either.
 */
const BASIN_NAME = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/u;

/** The basin a machine declares itself one of, or null where it names none. */
function basinOf(said: unknown): string | null {
  return typeof said === "string" && BASIN_NAME.test(said) ? said : null;
}

/** A machine record as the map draws it, its room and what it declares flattened. */
function machineOf(record: Fields): Fields {
  const room = within(record, "room");
  const declared = within(record, "declared");
  return {
    name: record["name"],
    state: record["state"],
    model_name: record["model_name"],
    os: record["os"],
    system: record["system"],
    cores: within(record, "cpu")["cores"],
    memory: record["memory"],
    disk: record["disk"],
    disk_free: record["disk_free"],
    disk_band: record["disk_band"],
    power: record["power"],
    available: room["memory_available"] ?? null,
    pressure: room["pressure"] ?? null,
    over: room["over"] ?? [],
    reach: declared["reach"] ?? [],
    leaves_at: declared["leaves_at"] ?? null,
    // The pool of cloud machines it was borrowed from, by the pool's own
    // short name; anything but a name reads as no basin at all.
    basin: basinOf(declared["basin"]),
    measured_at: record["measured_at"],
  };
}

/**
 * The word a basin's mark is named by after the basin's own name, so it
 * reads as what it is and is never a machine's name, which holds no space.
 */
const BASIN_WORD = "basin";

/** Orders names as the map lists them. */
const collate = new Intl.Collator("en-CA").compare;

/** `record`'s `field` where it is a number, else nothing. */
function count(record: Fields, field: string): number {
  const value = record[field];
  return typeof value === "number" ? value : 0;
}

/** The most each of `machines` is pressed, field by field, or null where none measured. */
/** The disk guard's bands that say a disk fills, the fullest last. */
const FILLING_BANDS = ["filling", "nearly-full", "critically-full"];

/** The fullest band of any machine's disk, or none while none fills. */
function fullest(machines: readonly Fields[]): string | null {
  const bands = machines.map((each) => FILLING_BANDS.indexOf(String(each["disk_band"])));
  return FILLING_BANDS[Math.max(-1, ...bands)] ?? null;
}

function mostPressed(machines: readonly Fields[]): Fields | null {
  const measured = machines.map((machine) => machine["pressure"]).filter(isObject);
  if (measured.length === 0) {
    return null;
  }
  const most = (field: string): number => Math.max(...measured.map((each) => count(each, field)));
  return { cpu: most("cpu"), memory: most("memory"), io: most("io") };
}

/**
 * A basin's machines as the one stand the map draws: named by the basin,
 * its machines listed in it by name, ready while any of them is, the sum of
 * their cores and memory, and the worst of how they are pressed, short and
 * how full their disks are, so a machine in trouble is never hidden by the
 * basin it is in.
 */
function basinStand(basin: string, machines: readonly Fields[]): Fields {
  const members = [...machines].sort((one, other) =>
    collate(String(one["name"]), String(other["name"])),
  );
  const sum = (field: string): number =>
    members.reduce((total, member) => total + count(member, field), 0);
  return {
    name: `${basin} ${BASIN_WORD}`,
    basin,
    members,
    state: members.some((member) => member["state"] === "ready") ? "ready" : members[0]?.["state"],
    cores: sum("cores"),
    memory: sum("memory"),
    pressure: mostPressed(members),
    over: [...new Set(members.flatMap((member) => member["over"]).filter(Boolean))],
    disk_band: fullest(members),
    leaves_at: null,
    share: null,
  };
}

/**
 * The machines as the map draws them. A basin lends machines by the hour,
 * under names it makes up, so each basin stands once, named by the basin,
 * whatever machines it has lent this minute. Each machine of no basin
 * stands as itself, in the order the node first told of it, and the basins
 * after them by name, so a basin giving a machine back moves nothing.
 */
function standsOf(machines: readonly Fields[]): Fields[] {
  const lone: Fields[] = [];
  const basins = new Map<string, Fields[]>();
  for (const machine of machines) {
    const basin = machine["basin"];
    if (typeof basin === "string") {
      basins.set(basin, [...(basins.get(basin) ?? []), machine]);
    } else {
      lone.push(machine);
    }
  }
  const stands = [...basins]
    .sort(([one], [other]) => collate(one, other))
    .map(([basin, members]) => basinStand(basin, members));
  return [...lone, ...stands];
}

/** The latest messages, the newest last, each only who sent it, how and where. */
function messagesOf(held: Held): Message[] {
  const told: Message[] = [];
  for (const record of held.all()) {
    const at = text(record, "at");
    if (at !== undefined) {
      told.push({
        at,
        from: record["from"],
        intent: record["intent"],
        groups: record["groups"],
      });
    }
  }
  told.sort((one, other) => one.at.localeCompare(other.at));
  return told.slice(-LATEST_MESSAGES);
}

/**
 * What waits on the mesh, the oldest first, without what was said; none
 * at all when the node serves nothing of it.
 */
function waitsOf(held: Held | undefined): Wait[] | null {
  if (held === undefined) {
    return null;
  }
  const waits: Wait[] = [];
  for (const record of held.all()) {
    const since = text(record, "since");
    if (since !== undefined) {
      waits.push({
        id: record["id"],
        from: record["from"],
        intent: record["intent"],
        groups: record["groups"],
        since,
        re: record["re"] ?? null,
        waiting_on: record["waiting_on"] ?? [],
        until: record["until"] ?? null,
      });
    }
  }
  return waits.sort((one, other) => one.since.localeCompare(other.since));
}

/**
 * The feedings still drawn at `at`, the latest first, each said in the last
 * ten minutes; none at all when the node serves nothing of them.
 */
function fedOf(held: Held | undefined, at: number): Fed[] | null {
  if (held === undefined) {
    return null;
  }
  const fed: { readonly when: number; readonly said: Fed }[] = [];
  for (const record of held.all()) {
    const receptor = text(record, "receptor");
    const cell = text(record, "cell");
    const said = text(record, "at");
    const when = said === undefined ? Number.NaN : Date.parse(said);
    // One said a little ahead of this clock is drawn rather than held back
    // until something else moves, but never one so far ahead that it would
    // stay up past its ten minutes, or that the clocks can't both be right.
    if (receptor && cell && said && at - when < FED_SHOWN_MS && when - at < FED_AHEAD_MS) {
      // The record's own `kind` is the interface's, so the node moves the
      // feeding's, its word for what was fed, under `source`.
      const kind = text(object(record, "source") ?? {}, "kind") ?? null;
      fed.push({ when, said: { receptor, cell, at: new Date(when).toISOString(), kind } });
    }
  }
  return fed.sort((one, other) => other.when - one.when).map((each) => each.said);
}

/**
 * When the first thing drawn of `now` goes out of date, if anything will: a
 * balance reading going stale, or a feeding's ten minutes ending. The page
 * draws again then, so either leaves on time with nothing else moving.
 */
function expiryOf(now: Now): number | undefined {
  const dues = [
    ...(now.balance === null ? [] : [Date.parse(now.balance.at) + BALANCE_FRESH_MS]),
    ...(now.fed ?? []).map((each) => Date.parse(each.at) + FED_SHOWN_MS),
  ];
  return dues.length > 0 ? Math.min(...dues) : undefined;
}

/** A tie's two ends from its key, when it is a pair's and not a cell's own. */
function endsOf(key: string): [string, string] | undefined {
  const ends = key.slice(key.indexOf(":") + 1).split(TIE_SEPARATOR);
  const [first, second] = ends;
  return ends.length === 2 && first && second ? [first, second] : undefined;
}

/**
 * The ties between cells, the strongest first, each faded from its last
 * message to `at`; none at all when the node serves nothing of them. A
 * cell's own talk, keyed by its name alone, is no tie.
 */
function tiesOf(held: Held | undefined, at: number): Tie[] | null {
  if (held === undefined) {
    return null;
  }
  const ties: Tie[] = [];
  for (const record of held.all()) {
    const ends = endsOf(record.key);
    const heard = record["heard"];
    const strength = record["strength"];
    if (ends !== undefined && typeof heard === "number" && typeof strength === "number") {
      const faded = strength * 0.5 ** (Math.max(at - heard, 0) / TIE_HALF_LIFE_MS);
      ties.push({
        ends,
        at: new Date(heard).toISOString(),
        hour: record["hour"] ?? [0, 0],
        today: record["today"] ?? [0, 0],
        intents: record["intents"] ?? [],
        strength: Math.round(faded * 1000) / 1000,
      });
    }
  }
  return ties.sort((one, other) => other.strength - one.strength);
}

/** Whether a cell is drawn as one: a session or a receptor, not a machine. */
function drawn(cell: Fields): boolean {
  return cell["type"] === "session" || cell["type"] === "receptor";
}

/**
 * A builder record as the map draws it: what is missing reads as nothing
 * said, and a state the node didn't give, or gave empty, as unknown.
 */
function builderOf(record: Fields): Builder {
  return {
    name: text(record, "name") ?? "",
    state: text(record, "state") || "unknown",
    systems: texts(record, "systems") ?? [],
    jobs: number(record, "jobs") ?? 0,
    lent: record["lent"] === true,
    said: text(record, "said") ?? null,
  };
}

/** The builders, by name, and none without one. */
function buildersOf(records: readonly Fields[]): Builder[] {
  return records
    .map(builderOf)
    .filter((builder) => builder.name !== "")
    .sort((one, other) => collate(one.name, other.name));
}

/** What the map draws from `heard`, read at `at` (milliseconds since the epoch). */
function nowOf(heard: Heard, at: number): Now {
  const reading = latestReading(heard.balance, at);
  const shares = sharesOf(reading);
  const now: Now = {
    at: new Date(at).toISOString(),
    networks: heard.network.all().map((record) => asPrinted(record, "network")),
    cells: heard.cell
      .all()
      .filter(drawn)
      .map((record) => asPrinted(record, "cell")),
    machines: standsOf(
      heard.machine.all().map((record) => ({
        ...machineOf(record),
        share: shares.get(text(record, "name") ?? "") ?? null,
      })),
    ),
    builders: buildersOf(heard.builder?.all() ?? []),
    messages: messagesOf(heard.message),
    balance: balanceOf(reading),
    waits: waitsOf(heard.unanswered),
    ties: tiesOf(heard.tie, at),
    fed: fedOf(heard.fed, at),
    started: startedOf(heard.cell, heard.forage),
    foraged: foragedOf(heard.forage),
  };
  // Every field is untyped JSON from the node, and plain keeps its shape.
  return plain(now) as Now;
}

export type { Balance, Fed, Heard, Message, Now, Started, Tie, Wait };
export { expiryOf, FED_AHEAD_MS, FED_SHOWN_MS, LATEST_MESSAGES, nowOf };
