// What the system page reads of a machine's two words on what is worth a look, its looks list
// with their whys and its side, as the daemon sends them: each readable, absent, unreadable or
// torn from the other, and what the page says it couldn't read.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { systemOf } from "../model.ts";

const machines = {
  key: "part:machines",
  kind: "part",
  part: "machines",
  health: "watch",
  side: [],
  things: ["machine:kestrel", "machine:osprey"],
  deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
};
const summary = {
  key: "summary",
  kind: "summary",
  machine: "kestrel",
  read_at: "2026-11-16T05:41:00Z",
  health: "watch",
  parts: ["machines"],
};
const kestrel = { key: "machine:kestrel", kind: "machine", name: "kestrel", health: "good" };
const named = { key: "machine:osprey", kind: "machine", name: "osprey", health: "bad" };
const reach = { side: "out of reach", matters: "What can't be read from here.", health: "bad" };
const side = ["out of reach", "code failing"];
const osprey = { ...named, side, looks: [reach, { side: "code failing" }] };
const looks = [reach, { side: "code failing", matters: "" }];

/** The system the page reads with osprey's record as `record`, and every other record fine. */
function ospreyAs(record: Record<string, unknown>) {
  const told = (these: readonly object[]) => {
    const held = new Held();
    held.took({ records: these });
    return held;
  };
  return systemOf(told([summary]), told([machines]), told([kestrel, record]));
}

/**
 * What the page reads osprey as, beside what it says it couldn't read. Osprey is found by its
 * name, so a case dropping it from the part finds nothing rather than reading another thing.
 */
function readAs(record: Record<string, unknown>) {
  const system = ospreyAs(record);
  const found = system?.parts[0]?.things.find((thing) => thing.name === "osprey");
  return { osprey: found, unread: system?.unread };
}

it("a machine's looks are read with their whys, and none from a record listing none", () => {
  const noSide = ["machine:osprey's side, so the page goes by its looks alone"];
  const untold = (what: string) => [
    `machine:osprey's ${what}, so whether anything there is worth a look can't be told`,
  ];
  const lostSide = untold("side");
  const lostBoth = untold("looks and side");
  const lostLooks = untold("looks");
  for (const [record, listed, said, why, unread] of [
    [osprey, looks, side, "a list whose look without a why is kept with none", []],
    [{ ...osprey, looks: undefined }, undefined, side, "an old daemon's record, listing none", []],
    [{ ...osprey, health: "good", looks: [], side: [] }, [], [], "a fine machine's", []],
    [{ ...osprey, side: null }, looks, side, "a null side beside a list", []],
    [{ ...osprey, side: "out of reach" }, looks, side, "an unreadable side beside a list", noSide],
    [{ ...osprey, looks: [], side: [7] }, [], [], "an unreadable side beside none", lostSide],
    [{ ...osprey, looks: undefined, side: 7 }, undefined, [], "a lone bad side", lostSide],
    [{ ...osprey, looks: 1, side: 7 }, undefined, [], "both unreadable", lostBoth],
    [{ ...osprey, looks: 1, side: undefined }, undefined, [], "lone bad looks", lostLooks],
  ] as const) {
    const read = readAs(record);
    assert.deepEqual(read.osprey?.looks, listed, `${why} is read from its list`);
    assert.deepEqual(read.osprey?.side, said, `${why} is said as ${read.osprey?.side}`);
    assert.deepEqual(read.unread, unread, `${why} says only what it can't read`);
  }
});

it("an unreadable side is said from the looks listed beside it", () => {
  const read = readAs({ ...osprey, side: "out of reach" });
  assert.deepEqual(read.osprey?.side, side, "the side wasn't said from the looks beside it");
});

it("a side absent or null is said from the looks listed beside it, and none is unread", () => {
  const { side: _side, ...unsided } = osprey;
  for (const [record, why] of [
    [{ ...osprey, side: null }, "a null side"],
    [unsided, "an absent side"],
  ] as const) {
    const read = readAs(record);
    assert.deepEqual(read.osprey?.side, side, `${why} wasn't said from the looks beside it`);
    assert.deepEqual(read.unread, [], `${why} was said as not read`);
  }
});

it("a side absent beside many looks is cut to three, as the daemon cuts one", () => {
  const five = ["out of reach", "code failing", "disk filling", "on battery", "running hot"];
  const { side: _side, ...unsided } = osprey;
  const read = readAs({ ...unsided, looks: five.map((one) => ({ side: one })) });
  const cut = ["out of reach", "code failing", "and 3 more worth a look"];
  const why = "an absent side from five looks wasn't cut as the daemon's";
  assert.deepEqual(read.osprey?.side, cut, why);
  assert.deepEqual(read.unread, [], "an absent side beside five looks was said as not read");
});

it("a side said from many looks is cut to three, as the daemon cuts one", () => {
  const five = ["out of reach", "code failing", "disk filling", "on battery", "running hot"];
  const read = readAs({ ...osprey, side: 7, looks: five.map((one) => ({ side: one })) });
  const cut = ["out of reach", "code failing", "and 3 more worth a look"];
  assert.deepEqual(read.osprey?.side, cut, "a side from five looks wasn't cut as the daemon's");
});

it("a side said from exactly three looks keeps all three, with nothing said of more", () => {
  const three = ["out of reach", "code failing", "disk filling"];
  const read = readAs({ ...osprey, side: 7, looks: three.map((one) => ({ side: one })) });
  assert.deepEqual(read.osprey?.side, three, "a side from three looks wasn't kept whole");
});

it("a side said from one look past three is cut, the cut said as the daemon says it", () => {
  const four = ["out of reach", "code failing", "disk filling", "on battery"];
  const read = readAs({ ...osprey, side: 7, looks: four.map((one) => ({ side: one })) });
  const cut = ["out of reach", "code failing", "and 2 more worth a look"];
  assert.deepEqual(read.osprey?.side, cut, "a side from four looks wasn't cut as the daemon's");
});

it("a looks list the page can't read, or torn from its side, leaves it to the side", () => {
  const notRead = ["machine:osprey's looks, so the page goes by its side alone, with no whys"];
  for (const [record, why] of [
    [{ ...osprey, looks: [...osprey.looks, { matters: "A why." }] }, "a look with no side"],
    [{ ...osprey, looks: [...osprey.looks, { side: "" }] }, "a look with an empty side"],
    [{ ...osprey, looks: [...osprey.looks, "a bare string"] }, "a look that's a bare string"],
    [{ ...osprey, looks: "a list" }, "a looks field that isn't a list"],
    [{ ...osprey, looks: [], side: ["on battery at 9%"] }, "an empty list beside a side's look"],
    [{ ...osprey, side: [] }, "a list beside a side naming none"],
  ] as const) {
    const read = readAs(record);
    assert.ok(read.osprey, `${why} left osprey off the page`);
    assert.equal(read.osprey.looks, undefined, `${why} leaves it to the side`);
    assert.deepEqual(read.unread, notRead, `${why} is said as not read`);
  }
});
