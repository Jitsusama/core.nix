// What health the system page gives each of a machine's looks, read from the word the daemon
// sends with it, and where that ranks the look among the ones listed first.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { Held } from "../../../local-sites/kit/records.ts";
import { looksOf as listedOf } from "../looks.ts";
import { systemOf, thingsHeld } from "../model.ts";

const summary = {
  key: "summary",
  kind: "summary",
  machine: "kestrel",
  read_at: "2026-11-16T05:41:00Z",
  health: "watch",
  parts: ["machines"],
};
const machines = {
  key: "part:machines",
  kind: "part",
  part: "machines",
  health: "watch",
  side: ["code failing"],
  things: ["machine:kestrel", "machine:osprey"],
  deeper: { command: "mycelium machines", site: "mycelium/", launcher: "Agents" },
};
const kestrel = { key: "machine:kestrel", kind: "machine", name: "kestrel", health: "good" };
// As many looks as the page lists first, so none of osprey's is left out.
const LISTED = 8;

/** Osprey's one look with `health` as its word: as the page reads it, and as it lists it. */
function ospreyLooking(health: unknown) {
  const look = { side: "code failing", matters: "Builds there fail.", health };
  const osprey = {
    key: "machine:osprey",
    kind: "machine",
    name: "osprey",
    health: "watch",
    side: [look.side],
    looks: [look],
  };
  const heldOf = (records: readonly object[]) => {
    const held = new Held();
    held.took({ records });
    return held;
  };
  const machine = thingsHeld(heldOf([kestrel, osprey]), heldOf([]));
  const system = systemOf(heldOf([summary]), heldOf([machines]), machine);
  assert.ok(system, "the records whole drew no system");
  const own = system.parts[0]?.things.find((thing) => thing.name === "osprey")?.looks?.[0];
  const listed = listedOf(system, LISTED).looks.find((each) => each.whose === "osprey");
  return { own, listed };
}

it("a look's own health is read from each word the daemon says a health in", () => {
  for (const [word, health] of [
    ["good", "good"],
    ["watch", "watch"],
    ["bad", "bad"],
    ["not read", "unread"],
  ] as const) {
    const { own, listed } = ospreyLooking(word);
    assert.equal(own?.health, health, `a look said ${word} was read as ${own?.health}`);
    assert.equal(listed?.health, health, `a look said ${word} was listed as ${listed?.health}`);
  }
});

it("a look with any other health word has none, and is listed as its machine is", () => {
  // Osprey itself is watch, so a look with no health of its own is listed as watch.
  for (const [word, why] of [
    ["odd", "a word the page doesn't know"],
    ["unread", "the page's own word, which the daemon never says"],
    ["", "an empty word"],
    [7, "a number"],
    [null, "null"],
  ] as const) {
    const { own, listed } = ospreyLooking(word);
    assert.ok(own !== undefined, `a look whose health is ${why} wasn't read at all`);
    assert.ok(!Object.hasOwn(own, "health"), `${why} gave a look the health ${own.health}`);
    assert.equal(listed?.health, "watch", `${why} listed a look as ${listed?.health}, not watch`);
  }
});
