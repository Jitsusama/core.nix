// What the page says of each machine and its ties to the other parts, read
// from a machine record's ties as the daemon writes them
// (modules/system/src/parts/ties.rs): the headline and lede over all of
// them, and each tie's words, every one of them without a browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  cellsSaid,
  codeSaid,
  itselfSaid,
  kindOf,
  machineHeadline,
  machineLede,
  networkSaid,
} from "../machine.ts";
import { tiesOf } from "../ties.ts";
import type { Part, System, Thing } from "../model.ts";

const deeper = { command: "mycelium machines", site: "mycelium/", launcher: "Machines" };

const ties = {
  cells: { busy: 45, idle: 27, ended: 50 },
  network: [
    {
      device: "mac:a8",
      by: "2.4 GHz",
      point: "Hall",
      bars: 5,
      signal_dbm: -36,
      room_mbps: 183.7,
    },
  ],
  code: { repositories: 18, checkouts: 20, trees: 2174, behind: 8, changes: 127 },
  figures: {
    model: "Example 14",
    system: "linux",
    disk: 1_023_117_623_296,
    disk_free: 218_676_555_776,
    cores: 16,
    cpu_busy: 0.39,
    memory: 66_898_141_184,
    memory_available: 25_340_307_046,
    charge: 79,
    on_battery: false,
    updated_at: "2026-11-18T02:19:50Z",
    update_step: "done",
  },
  acts: {
    link: "mycelium:machine/kestrel?from=system",
    commands: ["mycelium sprout --on kestrel", "mycelium update kestrel"],
  },
};

function machine(name: string, health: Thing["health"], side: string[] = []): Thing {
  return { name, health, says: "", side, address: undefined, ties: tiesOf(ties) };
}

function system(things: Thing[], others: Part[] = []): System {
  const machines: Part = {
    part: "machines",
    health: "watch",
    says: "",
    side: [],
    holds: undefined,
    things,
    deeper,
  };
  return {
    machine: "kestrel",
    readAt: "2026-11-18T02:20:00Z",
    health: "watch",
    parts: [machines, ...others],
    unread: [],
    untold: [],
  };
}

function part(name: string, health: Part["health"], side: string[]): Part {
  return { part: name, health, says: "", side, holds: undefined, things: [], deeper };
}

it("the headline counts the machines, then how they stand", () => {
  const two = [
    machine("kestrel", "watch", ["filling disk"]),
    machine("osprey", "watch", ["filling disk"]),
  ];
  assert.deepEqual(machineHeadline(system(two)), {
    machines: "Two machines",
    stand: "both worth a look",
    health: "watch",
  });
  const mixed = [machine("kestrel", "bad", ["out of reach"]), machine("osprey", "watch")];
  assert.equal(machineHeadline(system(mixed)).stand, "one in trouble and one worth a look");
  assert.equal(machineHeadline(system(mixed)).health, "bad");
  const three = [machine("a", "watch"), machine("b", "watch"), machine("c", "watch")];
  assert.equal(machineHeadline(system(three)).stand, "all worth a look");
  const ready = [machine("kestrel", "good"), machine("osprey", "good")];
  assert.deepEqual(machineHeadline(system(ready)), {
    machines: "Two machines",
    stand: "all ready",
    health: "good",
  });
  assert.equal(machineHeadline(system([machine("kestrel", "good")])).machines, "One machine");
  assert.equal(machineHeadline(system([])).stand, "none known");
});

it("the lede says what each machine worth a look has, then the worst of the rest", () => {
  const two = [
    machine("kestrel", "watch", ["filling disk, 135 GB free", "code wants a look"]),
    machine("osprey", "watch", ["filling disk, 178 GB free"]),
    machine("heron", "good"),
  ];
  assert.equal(
    machineLede(system(two)),
    "kestrel: filling disk, 135 GB free; code wants a look. osprey: filling disk, 178 GB free.",
  );
  const network = part("network", "watch", ["the attic's radio is busy 53% of the time"]);
  assert.equal(
    machineLede(system([machine("heron", "good")], [network])),
    "And on the network: the attic's radio is busy 53% of the time.",
  );
  assert.equal(machineLede(system([machine("heron", "good")])), undefined);
});

it("the lede ends each sentence once, keeping an owner's own mark", () => {
  const stopped = [machine("kestrel", "watch", ["out of reach, 20 hours ago."])];
  assert.equal(machineLede(system(stopped)), "kestrel: out of reach, 20 hours ago.");
  const asked = part("network", "bad", ["is the modem on?"]);
  assert.equal(
    machineLede(system([machine("heron", "good")], [asked])),
    "And on the network: is the modem on?",
  );
});

it("the lede says why a part wasn't read when nothing else wants a look", () => {
  const unread = { ...part("code", "unread", []), says: "repos timed out." };
  assert.equal(
    machineLede(system([machine("heron", "good")], [unread])),
    "And your code couldn't be read: repos timed out.",
  );
  const network = part("network", "watch", ["the attic's radio is busy"]);
  assert.equal(
    machineLede(system([machine("heron", "good")], [unread, network])),
    "And on the network: the attic's radio is busy.",
    "a look comes before what wasn't read",
  );
});

it("a machine's kind is its model and its system, and says which is this one", () => {
  const here = machine("kestrel", "good");
  assert.equal(kindOf(here, "kestrel"), "Example 14, Linux, this machine");
  assert.equal(kindOf(here, "osprey"), "Example 14, Linux");
  const mac = { ...here, ties: tiesOf({ figures: { system: "macos" } }) };
  assert.equal(kindOf(mac, "osprey"), "macOS");
  assert.equal(kindOf({ ...here, ties: tiesOf(undefined) }, "osprey"), undefined);
});

it("its cells are counted busy and idle, with those ended aside", () => {
  assert.deepEqual(cellsSaid(tiesOf(ties)), {
    said: "45 busy and 27 idle",
    aside: "And 50 that have ended.",
  });
  assert.deepEqual(cellsSaid(tiesOf({ cells: { busy: 0, idle: 1, ended: 0 } })), {
    said: "None busy and 1 idle",
    aside: "None has ended.",
  });
  assert.equal(
    cellsSaid(tiesOf({ cells: { busy: 4, idle: 0, ended: 0 } })).said,
    "4 busy and none idle",
  );
  assert.deepEqual(cellsSaid(tiesOf({ cells: { busy: 0, idle: 0, ended: 1 } })), {
    said: "None busy or idle",
    aside: "And 1 that has ended.",
  });
  assert.equal(cellsSaid(tiesOf({ cells: null })).said, "Not read");
});

it("its network says where it meets it, by band or by wire, and how well", () => {
  assert.deepEqual(networkSaid(tiesOf(ties)), {
    said: "Wi-Fi to the Hall point, on 2.4 GHz",
    aside: "A strong signal, with 184 Mb/s of room.",
    device: "mac:a8",
  });
  const both = tiesOf({
    network: [
      { device: "mac:d0", by: "wired", point: "Den", bars: null, room_mbps: 369.85 },
      { device: "mac:66", by: "5 GHz", point: "Hall", bars: 3, room_mbps: 120 },
    ],
  });
  assert.deepEqual(
    networkSaid(both),
    {
      said: "Wired to the Den point, and Wi-Fi to the Hall point, on 5 GHz",
      aside: "370 Mb/s of room; a fair signal, with 120 Mb/s of room.",
      device: "mac:d0",
    },
    "each place's figures stay with it, in the order the places are said",
  );
  const unmeasured = tiesOf({
    network: [
      { device: "mac:66", by: null, point: "Hall", bars: null, room_mbps: null },
      { device: "mac:d0", by: "wired", point: "Den", bars: null, room_mbps: 941 },
    ],
  });
  assert.equal(
    networkSaid(unmeasured).aside,
    "Not measured; 941 Mb/s of room.",
    "a place not measured keeps its turn, so the next one's figure stays its own",
  );
  assert.deepEqual(networkSaid(tiesOf({ network: [] })), {
    said: "Not seen on the home network",
    aside: "No device on it carries this machine's name.",
    device: undefined,
  });
  assert.equal(networkSaid(tiesOf({ network: null })).said, "Not read");
});

it("its code is counted on this machine and said not read from another", () => {
  assert.deepEqual(codeSaid(tiesOf(ties), "kestrel", true), {
    said: "20 checkouts of 18 repositories",
    aside: "2,174 trees; 8 checkouts behind their upstreams; 127 open changes.",
    read: true,
  });
  const few = tiesOf({
    code: { repositories: 1, checkouts: 1, trees: 0, behind: 1, changes: 0 },
  });
  assert.deepEqual(codeSaid(few, "kestrel", true), {
    said: "1 checkout of 1 repository",
    aside: "No tree; 1 checkout behind its upstream; no open change.",
    read: true,
  });
  assert.deepEqual(codeSaid(tiesOf({ code: null }), "osprey", false), {
    said: "Not read from here",
    aside: "Its code is read on osprey itself.",
    read: false,
  });
  assert.deepEqual(
    codeSaid(tiesOf({ code: null }), "kestrel", true),
    { said: "Not read", aside: "Repos couldn't be had.", read: false },
    "on the machine it's read on, an unread code is a read that failed",
  );
});

it("itself says its disk, then its processor, memory, power and update", () => {
  assert.deepEqual(
    itselfSaid(tiesOf(ties), (at) => `at ${at.slice(11, 16)}`),
    {
      said: "219 GB of 1 TB free",
      aside:
        "Processor 39% busy, 25 of 67 GB of memory free, 79% charged and plugged in. Updated at 02:19.",
    },
  );
  const mac = tiesOf({
    figures: {
      disk: 994_662_584_320,
      disk_free: 184_893_198_336,
      charge: 100,
      on_battery: false,
      updated_at: "2026-11-18T02:38:03Z",
      update_step: "unchanged",
    },
  });
  assert.deepEqual(
    itselfSaid(mac, (at) => `at ${at.slice(11, 16)}`),
    {
      said: "185 GB of 995 GB free",
      aside: "Fully charged and plugged in. Unchanged at its update at 02:38.",
    },
  );
  const away = tiesOf({ figures: { charge: 12, on_battery: true } });
  assert.deepEqual(
    itselfSaid(away, (at) => at),
    {
      said: "Its disk not said",
      aside: "12% charged, on battery.",
    },
  );
  assert.deepEqual(
    itselfSaid(tiesOf({ figures: { charge: 40 } }), (at) => at),
    { said: "Its disk not said", aside: "40% charged." },
    "a charge whose node doesn't say its plug says no plug",
  );
});
