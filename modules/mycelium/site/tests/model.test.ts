// What the map makes of the node's records, told as a watch tells them:
// every record whole, then only what changed, and the keys gone.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { LATEST_MESSAGES, nowOf } from "../model.ts";

const AT = Date.parse("2026-11-16T07:30:00Z");

/** The four watches, each told its kind's records of `these` whole. */
function told(these: readonly { readonly key: string; readonly kind: string }[]) {
  const heard = {
    network: new Held(),
    cell: new Held(),
    machine: new Held(),
    message: new Held(),
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

it("a machine's room and declared reach are flattened, as the map reads them", () => {
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
      measured_at: "2026-11-16T07:29:00Z",
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
  assert.deepEqual([machine.reach, machine.leaves_at], [[], null]);
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
