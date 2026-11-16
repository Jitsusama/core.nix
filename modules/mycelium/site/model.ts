// What the map draws, made from the node's records as a watch holds them:
// the same `now` that now.json held while a timer wrote it, so the drawing
// (view.js) reads the live feed without a change to how it draws.
//
// The networks and cells are the very objects `mycelium networks -o json`
// prints, made by the node's own model (its `network` and `cell` kinds), so
// the map and the command never differ and nothing here re-derives them. A
// record adds its `key` and `heard`, and drops its `kind`, which is the
// interface's; both are put back as the command says them. The machines add
// what their cells leave out (memory, disk, power, room), and the messages
// only who sent, to whom and when, which is all the feed passes of them.

import { type Fields, type Held, isObject, text } from "../../local-sites/kit/records.ts";

/** How many messages the map hears, the latest, as `mycelium messages` says. */
const LATEST_MESSAGES = 20;

/**
 * How old the balance's latest reading may be and still be drawn. The
 * service reads each minute, so three minutes is two missed: past it, the
 * service has stopped and its last reading would only look current.
 */
const BALANCE_FRESH_MS = 180_000;

/**
 * The node's records, a watch's for each kind the map reads. The balance is
 * the balance service's latest reading, which a node serves only where the
 * service runs, so a map without it draws as before.
 */
interface Heard {
  readonly network: Held;
  readonly cell: Held;
  readonly machine: Held;
  readonly message: Held;
  readonly balance?: Held;
}

/** The realm's balance as the map says it, from the service's latest reading. */
interface Balance {
  readonly measures: unknown;
  readonly at: string;
  readonly imbalance: unknown;
  readonly held: unknown;
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
  readonly messages: readonly Message[];
  readonly balance: Balance | null;
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

/** A machine record as the map draws it, its room and declared reach flattened. */
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
    measured_at: record["measured_at"],
  };
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
 * The newest balance reading held, when it is fresh at `at`: each machine's node
 * serves its own, so the newest wins, and one dated further ahead than a reading
 * may age is a wrong clock, never fresh.
 */
function latestReading(held: Held | undefined, at: number): Fields | undefined {
  let latest: Fields | undefined;
  let latestAt = Number.NEGATIVE_INFINITY;
  for (const record of held?.all() ?? []) {
    const when = Date.parse(text(record, "at") ?? "");
    if (when > latestAt) {
      latest = record;
      latestAt = when;
    }
  }
  return Math.abs(at - latestAt) <= BALANCE_FRESH_MS ? latest : undefined;
}

/** Each machine's share of the realm's work in `reading`, by its name. */
function sharesOf(reading: Fields | undefined): Map<unknown, Fields> {
  const shares = new Map<unknown, Fields>();
  const machines = reading?.["machines"];
  for (const load of Array.isArray(machines) ? machines : []) {
    if (isObject(load)) {
      shares.set(load["machine"], {
        busy: load["busy"],
        idle: load["idle"],
        busy_per_core: load["busy_per_core"],
        load: load["load"],
        over: load["over"],
        room: load["room"],
      });
    }
  }
  return shares;
}

/** The realm's balance from `reading`: its definition, time, spread and held flags. */
function balanceOf(reading: Fields | undefined): Balance | null {
  const at = reading === undefined ? undefined : text(reading, "at");
  if (reading === undefined || at === undefined) {
    return null;
  }
  return {
    measures: reading["measures"],
    at,
    imbalance: reading["imbalance"] ?? null,
    held: reading["held"] ?? [],
  };
}

/** Whether a cell is drawn as one: a session or a receptor, not a machine. */
function drawn(cell: Fields): boolean {
  return cell["type"] === "session" || cell["type"] === "receptor";
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
    machines: heard.machine.all().map((record) => ({
      ...machineOf(record),
      share: shares.get(text(record, "name") ?? "") ?? null,
    })),
    messages: messagesOf(heard.message),
    balance: balanceOf(reading),
  };
  // Every field is untyped JSON from the node, and plain keeps its shape.
  return plain(now) as Now;
}

export type { Balance, Heard, Message, Now };
export { BALANCE_FRESH_MS, LATEST_MESSAGES, nowOf };
