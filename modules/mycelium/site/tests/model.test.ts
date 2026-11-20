// What the map makes of the node's records, told as a watch tells them:
// every record whole, then only what changed, and the keys gone.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { expiryOf, LATEST_MESSAGES, nowOf } from "../model.ts";

const AT = Date.parse("2026-11-16T07:30:00Z");

/** The watches, each told its kind's records of `these` whole. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const heard = {
    network: new Held(),
    cell: new Held(),
    machine: new Held(),
    message: new Held(),
    builder: new Held(),
  };
  for (const [kind, held] of Object.entries(heard)) {
    held.took({ records: these.filter((record) => record.kind === kind) });
  }
  return heard;
}

const network = {
  key: "network:orchestration.kestrel",
  kind: "network",
  heard: AT,
  name: "orchestration.kestrel",
  shape: "orchestration",
  short: "kestrel",
  cells: ["OSPREY", "TERN01"],
  machines: ["skiff"],
  counts: { moving: 2 },
};

/** A session's cell, as the node keys and tells it. */
function session(name: string, title: string) {
  return {
    key: `cell:${name}`,
    kind: "cell",
    heard: AT,
    name,
    type: "session",
    title,
    machine: "skiff",
    standing: "moving",
    networks: ["orchestration.kestrel"],
  };
}

const machineCell = { key: "cell:skiff", kind: "cell", heard: AT, name: "skiff", type: "machine" };
const receptor = {
  key: "cell:slack@skiff",
  kind: "cell",
  heard: AT,
  name: "slack@skiff",
  type: "receptor",
  counts: { unread: 3 },
};

const skiff = {
  key: "machine:skiff",
  kind: "machine",
  heard: AT,
  name: "skiff",
  state: "up",
  model_name: "a laptop",
  os: "linux",
  system: "x86_64-linux",
  cpu: { cores: 12 },
  memory: 64,
  disk: 1000,
  disk_free: 400,
  disk_band: "fine",
  power: "ac",
  room: { memory_available: 30, pressure: 0.1, over: ["cpu"] },
  declared: { reach: ["cistern"], leaves_at: "2026-11-16T09:00:00Z" },
  measured_at: "2026-11-16T07:29:00Z",
};

/** A message, as the node keys it, its words already left at the feed. */
function message(id: number, at: string) {
  return {
    key: `message:${id}`,
    kind: "message",
    heard: Date.parse(at),
    at,
    from: "OSPREY",
    intent: "fyi",
    groups: ["peer.tern01"],
  };
}

it("the networks and cells are as the command prints them, machines' cells left out", () => {
  const now = nowOf(told([network, session("OSPREY", "the map"), machineCell, receptor]), AT);
  assert.equal(now.at, "2026-11-16T07:30:00.000Z");
  assert.deepEqual(now.networks, [
    {
      kind: "network",
      name: "orchestration.kestrel",
      shape: "orchestration",
      short: "kestrel",
      cells: ["OSPREY", "TERN01"],
      machines: ["skiff"],
      counts: { moving: 2 },
    },
  ]);
  assert.deepEqual(
    now.cells.map((cell) => [cell.kind, cell.name, cell.type, "key" in cell, "heard" in cell]),
    [
      ["cell", "OSPREY", "session", false, false],
      ["cell", "slack@skiff", "receptor", false, false],
    ],
  );
});

// A cell's name is its key, so two cells "sharing a name" share the name a
// person reads, their title.
it("two cells sharing a title are two cells, each its own", () => {
  const now = nowOf(told([session("OSPREY", "the map"), session("TERN01", "the map")]), AT);
  assert.deepEqual(
    now.cells.map((cell) => [cell.name, cell.title]),
    [
      ["OSPREY", "the map"],
      ["TERN01", "the map"],
    ],
  );
});

it("a machine's room and what it declares are flattened, as the map reads them", () => {
  const { machines } = nowOf(told([skiff]), AT);
  assert.deepEqual(machines, [
    {
      name: "skiff",
      state: "up",
      model_name: "a laptop",
      os: "linux",
      system: "x86_64-linux",
      cores: 12,
      memory: 64,
      disk: 1000,
      disk_free: 400,
      disk_band: "fine",
      power: "ac",
      available: 30,
      pressure: 0.1,
      over: ["cpu"],
      reach: ["cistern"],
      leaves_at: "2026-11-16T09:00:00Z",
      basin: null,
      measured_at: "2026-11-16T07:29:00Z",
      share: null,
    },
  ]);
});

it("a machine that measured no room and declares nothing reads as none", () => {
  const { key, kind, heard, name, state } = skiff;
  const [machine] = nowOf(told([{ key, kind, heard, name, state }]), AT).machines;
  assert.ok(machine);
  assert.deepEqual(
    [machine.cores, machine.available, machine.pressure, machine.over],
    [undefined, null, null, []],
  );
  assert.deepEqual([machine.reach, machine.leaves_at, machine.basin], [[], null, null]);
});

it("only the latest messages are heard, the newest last, as who sent them where", () => {
  const sent = Array.from({ length: LATEST_MESSAGES + 5 }, (_, at) =>
    message(at, `2026-11-16T07:${String(at).padStart(2, "0")}:00Z`),
  );
  const { messages } = nowOf(told([...sent].reverse()), AT);
  assert.equal(messages.length, LATEST_MESSAGES);
  assert.deepEqual(messages.at(0), {
    at: "2026-11-16T07:05:00Z",
    from: "OSPREY",
    intent: "fyi",
    groups: ["peer.tern01"],
  });
  assert.equal(messages.at(-1)?.at, "2026-11-16T07:24:00Z");
});

it("a message with no time to it is passed over", () => {
  const { at: _at, ...timeless } = message(1, "2026-11-16T07:01:00Z");
  const { messages } = nowOf(told([timeless, message(2, "2026-11-16T07:02:00Z")]), AT);
  assert.deepEqual(
    messages.map((each) => each.at),
    ["2026-11-16T07:02:00Z"],
  );
});

it("a cut title reads with three full stops, wherever it sits", () => {
  const cut = { ...session("OSPREY", "Serve the map\u2026"), trees: [{ branch: "map\u2026" }] };
  const [cell] = nowOf(told([cut]), AT).cells;
  assert.deepEqual([cell?.title, cell?.trees], ["Serve the map...", [{ branch: "map..." }]]);
});

/** The balance service's latest reading, as the node serves it, `ago` ms before AT. */
function reading(ago: number) {
  return {
    key: "balance:skiff",
    kind: "balance",
    heard: AT,
    measures: "balance/2",
    at: new Date(AT - ago).toISOString(),
    machines: [
      {
        machine: "skiff",
        ready: true,
        memory: 0.63,
        stall: 0.85,
        processor: 0.27,
        load: 0.85,
        over: false,
        room: false,
        busy: 30,
        idle: 42,
        busy_per_core: 1.88,
        age: 0,
      },
    ],
    imbalance: 0.06,
    flags: [],
    held: [{ flag: "stranded", machine: "skiff", spent: "processor" }],
  };
}

/** The four watches and the balance's, told `these` and the balance `latest`. */
function toldBalance(these: Parameters<typeof told>[0], latest: readonly object[]) {
  const balance = new Held();
  balance.took({ records: latest });
  return { ...told(these), balance };
}

it("a fresh balance reading gives each machine its share and the map what's held", () => {
  const now = nowOf(toldBalance([skiff], [reading(60_000)]), AT);
  assert.deepEqual(
    now.machines.map((machine) => machine.share),
    [{ busy: 30, idle: 42, busy_per_core: 1.88, load: 0.85, over: false, room: false }],
  );
  assert.deepEqual(now.balance, {
    measures: "balance/2",
    at: new Date(AT - 60_000).toISOString(),
    imbalance: 0.06,
    held: [{ flag: "stranded", machine: "skiff", spent: "processor" }],
  });
});

it("a balance reading past three minutes old, ahead of the clock, with no time, or none, is no share at all", () => {
  const { at: _at, ...timeless } = reading(0);
  for (const [why, heard] of [
    ["181 s old", toldBalance([skiff], [reading(181_000)])],
    ["181 s ahead", toldBalance([skiff], [reading(-181_000)])],
    ["no time", toldBalance([skiff], [timeless])],
    ["no reading", toldBalance([skiff], [])],
    ["no balance watch", told([skiff])],
  ] as const) {
    const now = nowOf(heard, AT);
    assert.deepEqual(
      [now.machines.map((machine) => machine.share), now.balance],
      [[null], null],
      why,
    );
  }
});

it("a balance reading 179 s old, or as far ahead, still gives its shares", () => {
  for (const ago of [179_000, -179_000]) {
    const now = nowOf(toldBalance([skiff], [reading(ago)]), AT);
    assert.notEqual(now.balance, null, `${ago} ms`);
  }
});

it("the map draws again when its balance reading's three minutes are up", () => {
  const now = nowOf(toldBalance([skiff], [reading(2 * 60_000)]), AT);
  assert.equal(expiryOf(now), AT + 60_000);
});

it("of a balance and a feeding, the map draws again for whichever goes first", () => {
  /** The watches with a balance read `read` ago and a feeding said `said` ago. */
  const both = (read: number, said: number) => {
    const fed = new Held();
    const at = new Date(AT - said).toISOString();
    const feeding = { key: "fed:github@kestrel~SKIFF1", kind: "fed", heard: AT - said, at };
    fed.took({ records: [{ ...feeding, receptor: "github@kestrel", cell: "SKIFF1" }] });
    return nowOf({ ...toldBalance([skiff], [reading(read)]), fed }, AT);
  };
  assert.equal(expiryOf(both(2 * 60_000, 5 * 60_000)), AT + 60_000, "the balance goes first");
  assert.equal(expiryOf(both(0, 9.5 * 60_000)), AT + 30_000, "the feeding goes first");
});

it("of the readings each machine's node serves, the newest is the one drawn", () => {
  const older = { ...reading(90_000), key: "balance:tern", imbalance: 0.3 };
  const now = nowOf(toldBalance([skiff], [reading(30_000), older]), AT);
  assert.equal(now.balance?.imbalance, 0.06);
});

it("a reading of one machine has no spread, and one that holds nothing holds none", () => {
  const { held: _held, ...alone } = { ...reading(0), imbalance: null };
  const now = nowOf(toldBalance([skiff], [alone]), AT);
  assert.deepEqual([now.balance?.imbalance, now.balance?.held], [null, []]);
});

it("a machine the balance doesn't name has no share, and the others keep theirs", () => {
  const tern = { ...skiff, key: "machine:tern", name: "tern" };
  const now = nowOf(toldBalance([skiff, tern], [reading(0)]), AT);
  assert.deepEqual(
    now.machines.map((machine) => [machine.name, machine.share === null]),
    [
      ["skiff", false],
      ["tern", true],
    ],
  );
});

/** What the node judges waits on the mesh, as it serves the `unanswered` kind. */
function open(id: string, since: string, fields: object) {
  return {
    key: `unanswered:${id}`,
    kind: "unanswered",
    heard: AT,
    id,
    from: "OSPREY",
    since,
    text: "what was said",
    ...fields,
  };
}

/** The four watches and the unanswered kind's, told `these` and what waits. */
function toldWaits(these: Parameters<typeof told>[0], waiting: readonly object[]) {
  const unanswered = new Held();
  unanswered.took({ records: waiting });
  return { ...told(these), unanswered };
}

it("what waits on the mesh is heard oldest first, as who waits on whom, never what was said", () => {
  const hold = open("2", "2026-11-16T07:20:00Z", {
    intent: "hold",
    groups: ["#machine"],
    until: "2026-11-16T09:20:00Z",
  });
  const question = open("1", "2026-11-16T07:10:00Z", {
    intent: "question",
    groups: ["@TERN01"],
    re: "0",
    waiting_on: ["TERN01"],
  });
  const { waits } = nowOf(toldWaits([], [hold, question]), AT);
  assert.deepEqual(waits, [
    {
      id: "1",
      from: "OSPREY",
      intent: "question",
      groups: ["@TERN01"],
      since: "2026-11-16T07:10:00Z",
      re: "0",
      waiting_on: ["TERN01"],
      until: null,
    },
    {
      id: "2",
      from: "OSPREY",
      intent: "hold",
      groups: ["#machine"],
      since: "2026-11-16T07:20:00Z",
      re: null,
      waiting_on: [],
      until: "2026-11-16T09:20:00Z",
    },
  ]);
});

it("a node that serves nothing of what waits is told apart from one with nothing waiting", () => {
  assert.deepEqual([nowOf(told([]), AT).waits, nowOf(toldWaits([], []), AT).waits], [null, []]);
});

it("a wait with no time to it is passed over", () => {
  const { since: _since, ...timeless } = open("1", "", { intent: "question" });
  assert.deepEqual(nowOf(toldWaits([], [timeless]), AT).waits, []);
});

/** A tie as the node serves the `tie` kind, heard at its last message. */
function tie(ends: string, heard: number, fields: object) {
  return { key: `tie:${ends}`, kind: "tie", heard, ...fields };
}

/** The four watches and the tie kind's, told `these` and the ties. */
function toldTies(these: Parameters<typeof told>[0], ties: readonly object[]) {
  const held = new Held();
  held.took({ records: ties });
  return { ...told(these), tie: held };
}

const HOUR_MS = 3_600_000;

it("a tie is heard as its two ends, how each talked, and its strength faded to now", () => {
  const lately = tie("OSPREY~TERN01", AT - 2 * HOUR_MS, {
    hour: [0, 0],
    today: [3, 1],
    intents: ["fyi", "question"],
    groups: ["peer.osprey"],
    strength: 4,
  });
  const { ties } = nowOf(toldTies([], [lately]), AT);
  assert.deepEqual(ties, [
    {
      ends: ["OSPREY", "TERN01"],
      at: new Date(AT - 2 * HOUR_MS).toISOString(),
      hour: [0, 0],
      today: [3, 1],
      intents: ["fyi", "question"],
      strength: 2,
    },
  ]);
});

it("a cell's own talk, and a tie with no time or no second end, are no tie", () => {
  const own = tie("OSPREY", AT, { hour: 1, today: 4, ties: 2 });
  const timeless = { key: "tie:OSPREY~TERN01", kind: "tie", strength: 1 };
  const lone = tie("OSPREY~", AT, { strength: 1 });
  assert.deepEqual(nowOf(toldTies([], [own, timeless, lone]), AT).ties, []);
});

it("the strongest tie comes first, and a node that serves none is told apart", () => {
  const weak = tie("KITE01~OSPREY", AT, { strength: 1 });
  const strong = tie("OSPREY~TERN01", AT, { strength: 5 });
  const { ties } = nowOf(toldTies([], [weak, strong]), AT);
  assert.deepEqual(
    ties?.map((one) => one.ends),
    [
      ["OSPREY", "TERN01"],
      ["KITE01", "OSPREY"],
    ],
  );
  assert.equal(nowOf(told([]), AT).ties, null);
});

it("a cell's change is drawn, and a cell gone is gone from the map", () => {
  const heard = told([session("OSPREY", "the map"), session("TERN01", "the feed")]);
  heard.cell.took({
    changed: [{ key: "cell:OSPREY", standing: "needs-you" }],
    gone: ["cell:TERN01"],
  });
  const now = nowOf(heard, AT);
  assert.deepEqual(
    now.cells.map((cell) => [cell.name, cell.standing]),
    [["OSPREY", "needs-you"]],
  );
});
