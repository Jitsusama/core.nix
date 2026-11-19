// How things stand, as the drawer's health rows say it: a verdict for cells
// and machines, the words saying why, a machine's form, and the balance's
// shares and flags. The words are written out whole, so an edit to any of
// them shows here. Times are in the reader's own zone, with no offset.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  contextWords,
  type Flag,
  flagNames,
  flagWords,
  groupHealth,
  heldFlags,
  loadWords,
  type MachineFacts,
  machineForm,
  machineHealth,
  sandboxWords,
  shareWords,
  type Verdict,
  verdictOf,
} from "../health.ts";
import type { Cell } from "../words.ts";

const AT = "2026-11-19T10:00:00";
const GIB = 2 ** 30;

/** A cell on kestrel, quiet with nothing waiting, but for what `more` says. */
const cell = (address: string, more: Partial<Cell> = {}): Cell => ({
  address,
  short: address,
  standing: "quiet",
  owes: [],
  holds: [],
  machine: "kestrel",
  ...more,
});

/** A cell whose context holds `used` tokens of 200,000. */
const filled = (address: string, used: number): Cell =>
  cell(address, { usage: { context: { used, limit: 200_000 } } });

/** A ready machine saying nothing more, but for what `more` says. */
const machine = (more: Partial<MachineFacts> = {}): MachineFacts => ({
  name: "kestrel",
  state: "ready",
  ...more,
});

it("tends what waits, is stuck, owes or is nearly full, and watches a filling context", () => {
  const said: Verdict[] = [
    verdictOf([]),
    verdictOf([cell("A")]),
    verdictOf([cell("A", { standing: "asks" })]),
    verdictOf([cell("A", { standing: "stuck" })]),
    verdictOf([cell("A", { owes: [{}] })]),
    verdictOf([filled("A", 180_000)]),
    verdictOf([filled("A", 150_000)]),
    verdictOf([filled("A", 149_000)]),
    verdictOf([filled("A", 150_000), filled("B", 180_000)]),
  ];
  assert.deepEqual(said, [0, 0, 2, 2, 2, 2, 1, 0, 2], "each set should stand as its worst cell");
});

it("says how full a context is, by the thousand and the million", () => {
  const said = [
    contextWords(filled("A", 150_000)),
    contextWords(cell("A", { usage: { context: { used: 500_000, limit: 1_000_000 } } })),
    contextWords(cell("A", { usage: { context: { used: 300_000, limit: 1_500_000 } } })),
    contextWords(cell("A", { usage: { context: { used: 10 } } })),
    contextWords(cell("A")),
  ];
  assert.deepEqual(
    said,
    ["context 75% of 200k", "context 50% of 1M", "context 20% of 1.5M", "", ""],
    "a context with no limit should say nothing",
  );
});

it("says why a set of cells stands as it does, worst first, with what it spent", () => {
  const cells = [
    cell("A", { standing: "asks" }),
    cell("B", { standing: "asks", owes: [{}, {}] }),
    cell("C", { standing: "stuck" }),
    filled("D", 190_000),
    cell("E", { standing: "moving", usage: { context: { used: 160_000, limit: 200_000 } } }),
    cell("F", { standing: "dormant", usage: { cost_today: 1.5 } }),
  ];
  assert.deepEqual(
    groupHealth(cells),
    {
      v: 2,
      words:
        "2 need you, 1 stuck, 2 questions waiting, 1 nearly out of context, " +
        "1 with context filling, 1 moving, 1 quiet, 1 dormant; $1.50 today",
    },
    "the words should name each way the cells stand, in order",
  );
  assert.deepEqual(
    [groupHealth([cell("A", { standing: "asks" })]), groupHealth([])],
    [
      { v: 2, words: "1 needs you" },
      { v: 0, words: "no cells" },
    ],
    "one cell needing the person should take needs, and none should say so",
  );
});

it("says a ready machine with nothing to tell is well", () => {
  assert.deepEqual(
    machineHealth(machine(), AT),
    { v: 0, words: "nothing said" },
    "a machine saying nothing should be well",
  );
});

it("tends a machine that isn't ready, runs short or is nearly out of charge", () => {
  const said = [
    machineHealth(machine({ state: "lost" }), AT),
    machineHealth(machine({ over: ["memory", "disk"] }), AT),
    machineHealth(machine({ power: { on_battery: true, charge: 15 } }), AT),
    machineHealth(machine({ power: { on_battery: true, charge: 60 } }), AT),
    machineHealth(machine({ power: { on_battery: false, charge: 15 } }), AT),
    machineHealth(machine({ power: { on_battery: true, charge: 0 } }), AT),
    machineHealth(machine({ power: { on_battery: true, charge: 19 } }), AT),
    machineHealth(machine({ power: { on_battery: true, charge: 20 } }), AT),
  ];
  assert.deepEqual(
    said,
    [
      { v: 2, words: "lost" },
      { v: 2, words: "memory and disk short" },
      { v: 2, words: "on battery, 15%" },
      { v: 0, words: "on battery, 60%" },
      { v: 0, words: "nothing said" },
      { v: 2, words: "on battery, 0%" },
      { v: 2, words: "on battery, 19%" },
      { v: 0, words: "on battery, 20%" },
    ],
    "a machine on the mains should say nothing of its charge, and under 20% on battery tend",
  );
});

it("watches a machine under pressure, filling its disk or near its lease's end", () => {
  const said = [
    machineHealth(machine({ pressure: { cpu: 90.4, memory: 10, io: 0.2 } }), AT),
    machineHealth(machine({ pressure: { cpu: 10, memory: 85, io: 3 } }), AT),
    machineHealth(machine({ disk: 100 * GIB, disk_free: 50 * GIB, disk_band: "filling" }), AT),
    machineHealth(machine({ leaves_at: "2026-11-19T10:20:00" }), AT),
    machineHealth(machine({ declared: { leaves_at: "2026-11-19T12:00:00" } }), AT),
    machineHealth(machine({ leaves_at: "2026-11-19T10:29:00" }), AT),
    machineHealth(machine({ leaves_at: "2026-11-19T10:30:00" }), AT),
    machineHealth(machine({ leaves_at: "2026-11-19T09:00:00" }), AT),
  ];
  assert.deepEqual(
    said,
    [
      { v: 1, words: "cpu 90%, memory 10%, io 0%" },
      { v: 1, words: "cpu 10%, memory 85%, io 3%" },
      { v: 1, words: "50 GiB of 100 GiB disk free, filling" },
      { v: 1, words: "20 min left, until 10:20" },
      { v: 0, words: "2 h left, until 12:00" },
      { v: 1, words: "29 min left, until 10:29" },
      { v: 0, words: "30 min left, until 10:30" },
      { v: 1, words: "its time is up" },
    ],
    "a lease with half an hour or more left should be well, and one past watched",
  );
});

it("says what leads first, then each fact in its order", () => {
  const x = machine({
    state: "draining",
    pressure: { cpu: 20, memory: 30, io: 1 },
    over: ["memory"],
    available: 8 * GIB,
    disk: 200 * GIB,
    disk_free: 120 * GIB,
    power: { on_battery: true, charge: 50 },
    leaves_at: "2026-11-19T12:00:00",
  });
  assert.deepEqual(
    machineHealth(x, AT, ["3 cells"]),
    {
      v: 2,
      words:
        "3 cells; draining; cpu 20%, memory 30%, io 1%; memory short; 8 GiB memory free; " +
        "120 GiB of 200 GiB disk free; on battery, 50%; 2 h left, until 12:00",
    },
    "the lead should come first and each fact after it",
  );
});

it("says a machine's nothing left as none, not as nothing said", () => {
  const x = machine({ available: 0, disk: 100 * GIB, disk_free: 0 });
  assert.deepEqual(
    machineHealth(x, AT),
    { v: 0, words: "0 GiB memory free; 0 GiB of 100 GiB disk free" },
    "no memory or disk free should still be said",
  );
});

it("takes only the facts a peer's node said in their shape", () => {
  const said = [
    machineHealth(machine({ pressure: { cpu: 99 } }), AT),
    machineHealth(machine({ pressure: "high" }), AT),
    machineHealth(machine({ over: "disk" }), AT),
    machineHealth(machine({ over: ["disk", 3] }), AT),
  ];
  assert.deepEqual(
    said,
    [
      { v: 0, words: "nothing said" },
      { v: 0, words: "nothing said" },
      { v: 0, words: "nothing said" },
      { v: 2, words: "disk short" },
    ],
    "a pressure missing a share, or what it runs short of not as names, should say nothing",
  );
});

it("tells a basin's box, a lent machine and bare metal apart by what each declares", () => {
  const said = [
    machineForm(machine({ basin: "east" })),
    machineForm(machine({ basin: "east", leaves_at: AT })),
    machineForm(machine({ leaves_at: AT })),
    machineForm(machine({ declared: { leaves_at: AT } })),
    machineForm(machine()),
    machineForm(machine({ leaves_at: "", declared: { leaves_at: AT } })),
    machineForm(machine({ leaves_at: "" })),
  ];
  assert.deepEqual(
    said,
    ["basin", "basin", "lent", "lent", "shut", "lent", "shut"],
    "a basin should win over a lease",
  );
  assert.equal(
    sandboxWords(machine({ leaves_at: "2026-11-19T11:00:00" }), AT),
    "a cloud sandbox, 1 h left, until 11:00",
    "a lent machine should say how long it has left",
  );
  assert.equal(sandboxWords(machine(), AT), "", "bare metal should be no sandbox");
});

it("says a machine's share as the balance reads it", () => {
  const said = [
    shareWords({ load: 0.62, room: true, busy: 3, idle: 1, busy_per_core: 0.375 }),
    shareWords({ load: 1.2, over: true, room: true }),
    shareWords({ busy: 1 }),
    loadWords("high"),
  ];
  assert.deepEqual(
    said,
    [
      "62% loaded, with room for more; 3 cells busy and 1 idle, 0.38 busy a core",
      "120% loaded, over its limits",
      "with no room for more; 1 cell busy and 0 idle",
      "",
    ],
    "a share should say its load, how it stands and its cells",
  );
});

it("keeps only the balance's flags that say what they are", () => {
  const held = heldFlags({ held: [{ flag: "stale", machine: "tern" }, null, { flag: 3 }, "hot"] });
  assert.deepEqual(held, [{ flag: "stale", machine: "tern" }], "a flag should name itself");
  assert.deepEqual(
    [heldFlags(null), heldFlags({ held: "stale" }), heldFlags("held")],
    [[], [], []],
    "a balance holding no list should hold no flags",
  );
  assert.deepEqual(
    flagNames({ flag: "hot_while_cool", hot: "tern", cool: "skiff" }),
    ["tern", "skiff"],
    "a flag should name the machines it says",
  );
});

it("says each flag as `mycelium balance` says it, and a machine's own as it", () => {
  const hot: Flag = { flag: "hot_while_cool", hot: "tern", cool: "skiff" };
  const said = [
    flagWords(hot),
    flagWords(hot, "tern"),
    flagWords(hot, "skiff"),
    flagWords({ flag: "stranded", machine: "tern", spent: "memory" }),
    flagWords({ flag: "stranded", machine: "tern", spent: "cpu" }, "tern"),
    flagWords({ flag: "stale", machine: "tern" }),
    flagWords({ flag: "imbalanced" }),
    flagWords({ flag: "too_many_moves" }),
  ];
  assert.deepEqual(
    said,
    [
      "tern is over while skiff has room, so new work should go to skiff",
      "it is over while skiff has room, so new work should go to skiff",
      "tern is over while this has room, so new work should come here",
      "tern's memory is nearly all used while its processors idle",
      "its processors are spent while half its memory or more is free",
      "tern's facts are too old to place by",
      "the machines' loads are uneven, so new work leans to the least loaded",
      "too many moves",
    ],
    "a flag the page doesn't know should be said by its own name",
  );
});
