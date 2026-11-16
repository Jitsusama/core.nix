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

/** The node's records, a watch's for each kind the map reads. */
interface Heard {
  readonly network: Held;
  readonly cell: Held;
  readonly machine: Held;
  readonly message: Held;
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

/** Whether a cell is drawn as one: a session or a receptor, not a machine. */
function drawn(cell: Fields): boolean {
  return cell["type"] === "session" || cell["type"] === "receptor";
}

/** What the map draws from `heard`, read at `at` (milliseconds since the epoch). */
function nowOf(heard: Heard, at: number): Now {
  const now: Now = {
    at: new Date(at).toISOString(),
    networks: heard.network.all().map((record) => asPrinted(record, "network")),
    cells: heard.cell
      .all()
      .filter(drawn)
      .map((record) => asPrinted(record, "cell")),
    machines: heard.machine.all().map(machineOf),
    messages: messagesOf(heard.message),
  };
  // Every field is untyped JSON from the node, and plain keeps its shape.
  return plain(now) as Now;
}

export type { Heard, Message, Now };
export { LATEST_MESSAGES, nowOf };
