// What the page says of each machine and its ties to the other parts, read
// from a machine record's ties as the daemon writes them
// (modules/system/src/parts/ties.rs): the headline over all of them, and
// each tie's words, every one of them without a browser.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  cellsSaid,
  codeSaid,
  itselfSaid,
  kindOf,
  machineHeadline,
  networkSaid,
  whenSaid,
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
  return { name, health, says: "", side, looks: undefined, address: undefined, ties: tiesOf(ties) };
}

function system(things: Thing[], others: Part[] = []): System {
  const machines: Part = {
    part: "machines",
    health: "watch",
    says: "",
    side: [],
    holds: undefined,
    things,
    named: things.map((thing) => thing.name),
    untold: 0,
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

it("the headline counts the machines, then how they stand", () => {
  const two = [
    machine("kestrel", "watch", ["on battery at 12%"]),
    machine("osprey", "watch", ["on battery at 9%"]),
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

it("a machine's kind is its model and its system, and says which is this one", () => {
  const here = machine("kestrel", "good");
  assert.equal(kindOf(here, "kestrel"), "Example 14, Linux, this machine");
  assert.equal(kindOf(here, "osprey"), "Example 14, Linux");
  const mac = { ...here, ties: tiesOf({ figures: { system: "macos" } }) };
  assert.equal(kindOf(mac, "osprey"), "macOS");
  assert.equal(kindOf({ ...here, ties: tiesOf(undefined) }, "osprey"), undefined);
  assert.equal(kindOf({ ...here, ties: tiesOf(undefined) }, "kestrel"), "This machine");
});

it("its cells are counted running, busy and idle, with the parked or ended aside", () => {
  assert.deepEqual(cellsSaid(tiesOf(ties)), {
    said: "72 running: 45 busy and 27 idle",
    aside: "And 50 parked or ended.",
  });
  assert.deepEqual(cellsSaid(tiesOf({ cells: { busy: 0, idle: 1, ended: 0 } })), {
    said: "1 running: none busy and 1 idle",
    aside: "None parked or ended.",
  });
  assert.equal(
    cellsSaid(tiesOf({ cells: { busy: 4, idle: 0, ended: 0 } })).said,
    "4 running: 4 busy and none idle",
  );
  assert.deepEqual(cellsSaid(tiesOf({ cells: { busy: 0, idle: 0, ended: 1 } })), {
    said: "None running",
    aside: "And 1 parked or ended.",
  });
  assert.equal(cellsSaid(tiesOf({ cells: null })).said, "Not read");
  const far = (busy: number, unreachable: number): string =>
    cellsSaid(tiesOf({ cells: { busy, idle: 0, ended: 0, unreachable } })).said;
  assert.equal(far(0, 12), "12 out of reach", "an unreached machine's cells");
  assert.equal(
    far(2, 1),
    "3 running: 2 busy, none idle and 1 out of reach",
    "some reached and some not",
  );
});

it("its parked cells are told from its ended ones where the node's health does", () => {
  const aside = (parked: number, ended: number): string =>
    cellsSaid(tiesOf({ cells: { busy: 9, idle: 31, parked, ended } })).aside;
  assert.equal(aside(22, 7), "And 22 parked; 7 have ended.", "both counted");
  assert.equal(aside(22, 1), "And 22 parked; 1 has ended.", "one ended");
  assert.equal(aside(1, 0), "And 1 parked; none has ended.", "none ended");
  assert.equal(aside(0, 7), "None parked; 7 have ended.", "none parked");
  assert.equal(aside(0, 0), "None parked or ended.", "neither");
});

it("a tie read from an owner's last good answer says when and why", () => {
  const note = "as read at 11:54; just now mycelium health gave no answer in 20 s";
  const told = "As read at 11:54; just now mycelium health gave no answer in 20 s.";
  const kept = tiesOf({ ...ties, kept: { cells: note } });
  assert.deepEqual(
    cellsSaid(kept),
    {
      said: "72 running: 45 busy and 27 idle",
      aside: `And 50 parked or ended. ${told}`,
    },
    "cells read from a kept health say when it was read",
  );

  assert.deepEqual(
    networkSaid(kept),
    networkSaid(tiesOf(ties)),
    "a tie read from its own answer says nothing of another's",
  );
});

it("each tie says its own owner's note, stopped once", () => {
  const net = "as read at 11:54; just now net --json ended with 1: no route.";
  const repos = "as read at 11:55; just now repos --json gave no answer in 20 s";
  const kept = tiesOf({ ...ties, kept: { network: net, code: repos } });
  const netTold = "As read at 11:54; just now net --json ended with 1: no route.";
  const reposTold = "As read at 11:55; just now repos --json gave no answer in 20 s.";
  assert.equal(
    networkSaid(kept).aside,
    `A strong signal, with 184 Mb/s of room. ${netTold}`,
    "the network read from a kept net says so, its owner's stop kept once",
  );
  assert.equal(
    codeSaid(kept, "kestrel", true).aside,
    `2,174 trees; 8 checkouts behind their upstreams; 127 open changes. ${reposTold}`,
    "the code read from a kept repos says so",
  );
  assert.deepEqual(
    cellsSaid(kept),
    cellsSaid(tiesOf(ties)),
    "cells read from their own answer say nothing of the others",
  );
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
      aside:
        "At the Den point, wired: 370 Mb/s of room. " +
        "At the Hall point, 5 GHz: a fair signal, with 120 Mb/s of room.",
      device: "mac:d0",
    },
    "each place's figures stay with it, named, in the order the places are said",
  );
  const unnamed = tiesOf({
    network: [
      { device: "mac:66", by: "5 GHz", point: "Hall", bars: 4, room_mbps: 370 },
      { device: "mac:d0", by: "wired", point: null, bars: null, room_mbps: 941 },
    ],
  });
  assert.equal(
    networkSaid(unnamed).aside,
    "At the Hall point, 5 GHz: a strong signal, with 370 Mb/s of room. " +
      "At an unnamed point, wired: 941 Mb/s of room.",
    "a place with no point opens its own sentence, named as the said line names it",
  );
  const unmeasured = tiesOf({
    network: [
      { device: "mac:66", by: null, point: "Hall", bars: null, room_mbps: null },
      { device: "mac:d0", by: "wired", point: "Den", bars: null, room_mbps: 941 },
    ],
  });
  assert.equal(
    networkSaid(unmeasured).aside,
    "At the Hall point: not measured. At the Den point, wired: 941 Mb/s of room.",
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
  const refused = tiesOf({
    figures: {
      updated_at: "2026-11-18T02:40:03Z",
      update_step: "refused",
      update_why: "the Mac is over its room",
    },
  });
  assert.deepEqual(
    itselfSaid(refused, (at) => `at ${at.slice(11, 16)}`),
    {
      said: "Its disk not said",
      aside: "Its update at 02:40 says refused: the Mac is over its room.",
    },
    "an update put off says why",
  );
  const asideOf = (figures: Record<string, unknown>): string =>
    itselfSaid(
      tiesOf({ figures: { updated_at: "2026-11-18T02:40:03Z", ...figures } }),
      (at) => `at ${at.slice(11, 16)}`,
    ).aside;
  assert.equal(
    asideOf({}),
    "Its last update was at 02:40.",
    "an update whose step isn't known, or couldn't be read, isn't said to have gone ahead",
  );
  assert.equal(
    asideOf({ update_step: "done", update_why: "the boot check failed" }),
    "Updated at 02:40, but the boot check failed.",
    "a done update that didn't all go ahead says why",
  );
  assert.equal(
    asideOf({ update_step: "building" }),
    "Its update at 02:40 says building.",
    "an update with no why says how it stands",
  );
  assert.equal(
    asideOf({ update_step: "building", update_why: "left over" }),
    "Its update at 02:40 says building.",
    "a why beside a step that carries none is left unsaid, as the system command leaves it",
  );
  assert.equal(
    asideOf({ update_step: "refused", update_why: "" }),
    "Its update at 02:40 says refused.",
    "an empty why is none",
  );
  assert.equal(
    asideOf({ update_step: "failed", update_why: "attribute 'x' missing" }),
    "Its update at 02:40 says failed: attribute 'x' missing.",
    "a failed update says its error",
  );
  assert.equal(
    asideOf({ update_step: "refused", update_why: "make /run/given and\u2026" }),
    "Its update at 02:40 says refused: make /run/given and\u2026",
    "a why cut short keeps its mark and gains no stop",
  );
  assert.equal(
    asideOf({ update_step: "refused", update_why: "is the modem on?" }),
    "Its update at 02:40 says refused: is the modem on?",
    "a why ending in its own mark ends once",
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

it("a machines part that wasn't read says so over the machines", () => {
  const read = system([]);
  const unread: System = {
    ...read,
    parts: read.parts.map((each) =>
      each.part === "machines"
        ? { ...each, health: "unread", says: "the mycelium couldn't be reached" }
        : each,
    ),
  };
  assert.deepEqual(machineHeadline(unread), {
    machines: "Your machines",
    stand: "not read",
    health: "unread",
  });
});

it("machines not told yet are said not read, never none known", () => {
  const notRead = { machines: "Your machines", stand: "not read", health: "unread" };
  const read = system([]);
  const noPart: System = { ...read, parts: [], untold: ["machines"] };
  assert.deepEqual(machineHeadline(noPart), notRead);
  const awaited: System = {
    ...read,
    parts: read.parts.map((each) => ({ ...each, untold: 2 })),
  };
  assert.deepEqual(machineHeadline(awaited), notRead);
});

it("counts machines whose records haven't come, and says them not read", () => {
  const read = system([machine("kestrel", "good")]);
  const partial: System = {
    ...read,
    parts: read.parts.map((each) => ({ ...each, untold: 1 })),
  };
  assert.deepEqual(machineHeadline(partial), {
    machines: "Two machines",
    stand: "one not read",
    health: "unread",
  });
});

it("offers only commands that hold no shell's punctuation", () => {
  const commands = [
    "mycelium sprout --on kestrel",
    "mycelium sprout; rm -rf ~",
    "echo $(id)",
    "mycelium update kestrel | sh",
    "mycelium update kestrel\nrm",
  ];
  const read = tiesOf({ ...ties, acts: { ...ties.acts, commands } });
  assert.deepEqual(read.acts.commands, ["mycelium sprout --on kestrel"]);
});

it("counts thousands of machines with their digits grouped", () => {
  const many = Array.from({ length: 1200 }, (_, at) => machine(`m${at}`, "good"));
  assert.equal(machineHeadline(system(many)).machines, "1,200 machines");
});

it("says a time on its clock today, yesterday by name, and earlier by its date", () => {
  const now = new Date(2026, 10, 18, 13, 40).getTime();
  const at = (day: number, hour: number, minute: number) =>
    new Date(2026, 10, day, hour, minute).toISOString();
  assert.equal(whenSaid(at(18, 2, 19), now), "at 02:19");
  assert.equal(whenSaid(at(17, 23, 5), now), "yesterday at 23:05");
  assert.equal(whenSaid(at(12, 9, 0), now), "on 12 Nov at 09:00");
  assert.equal(whenSaid("soon", now), "at an unknown time");
  // A clock ahead of this one says its day, and another year says its year.
  assert.equal(whenSaid(at(19, 1, 0), now), "on 19 Nov at 01:00");
  assert.equal(
    whenSaid(new Date(2025, 10, 12, 9, 0).toISOString(), now),
    "on 12 Nov 2025 at 09:00",
  );
});

it("says today and yesterday true across days of 25 and 23 hours", () => {
  const zone = process.env["TZ"];
  process.env["TZ"] = "America/Toronto";
  try {
    // 1 Nov 2026 runs 25 hours there, and 8 Mar 2026 runs 23.
    const late = "2026-11-02T04:30:00Z";
    assert.equal(whenSaid(late, Date.parse("2026-11-01T14:00:00Z")), "at 23:30");
    assert.equal(whenSaid(late, Date.parse("2026-11-02T17:00:00Z")), "yesterday at 23:30");
    assert.equal(
      whenSaid("2026-03-08T04:30:00Z", Date.parse("2026-03-09T16:00:00Z")),
      "on 7 Mar at 23:30",
    );
  } finally {
    if (zone === undefined) {
      delete process.env["TZ"];
    } else {
      process.env["TZ"] = zone;
    }
  }
});
