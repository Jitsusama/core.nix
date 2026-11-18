// How the network page lays the house out, and says its figures.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { study } from "../shape.ts";
import { lasted, linkSpeed, possessive, rank, rate, shellWord, slug } from "../words.ts";
import { told } from "./house.ts";

function shaped() {
  const { watched, kinds } = told();
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  return { network, shape: study(network) };
}

it("the floors go top first, each eero's devices on its own", () => {
  const { shape } = shaped();
  assert.deepEqual(
    shape.floors.map((floor) => [floor.point.name, floor.devices.map((device) => device.name)]),
    [
      ["Upstairs", ["Kitchen Printer"]],
      ["Basement", []],
    ],
  );
});

it("the router is the gateway, and the modem sits on no floor of its own", () => {
  const { shape } = shaped();
  assert.equal(shape.gateway?.id, "eero:1");
  assert.equal(shape.modem?.id, "modem");
  assert.ok(shape.floors.every((floor) => !floor.devices.includes(shape.modem ?? floor.point)));
});

it("a device hangs from the first hop of its way out, and routes cross where net counted", () => {
  const { shape } = shaped();
  const printer = shape.byId.get("mac:00:00:5e:00:53:10");
  assert.equal(printer === undefined ? undefined : shape.via(printer), "eero:2");
  assert.equal(shape.crossing("modem", "eero:1"), 3);
  assert.equal(shape.routed, 3);
});

it("a floor's place comes from the words in its eero's name", () => {
  assert.deepEqual(["Attic", "Kitchen", "Den", "Basement"].map(rank), [0, 1, 2, 3]);
});

it("a measured rate reads as net says it, in three figures", () => {
  // net's figures_tests.rs reads the same cases, so the panel and net's
  // concerns can't round one rate two ways.
  const cases: { mbps: number; says: string }[] = JSON.parse(
    readFileSync(new URL("rates.json", import.meta.url), "utf8"),
  );
  assert.ok(cases.length > 0, "the shared cases are there");
  assert.deepEqual(
    cases.map((c) => rate(c.mbps)),
    cases.map((c) => c.says),
  );
});

it("a rate of no number is none", () => {
  assert.deepEqual([Number.NaN, Number.POSITIVE_INFINITY].map(rate), ["0 b/s", "0 b/s"]);
});

it("a link's speed is a name, said with only the digits it has", () => {
  assert.deepEqual([1000, 2500, 10_000, 866, 1201, 2402, 72, 72.2, 100, 999.999].map(linkSpeed), [
    "1 Gb/s",
    "2.5 Gb/s",
    "10 Gb/s",
    "866 Mb/s",
    "1201 Mb/s",
    "2402 Mb/s",
    "72 Mb/s",
    "72.2 Mb/s",
    "100 Mb/s",
    "1 Gb/s",
  ]);
});

it("a span says the largest unit or two", () => {
  assert.deepEqual([42, 600, 3 * 3600 + 120, 30 * 3600, 50 * 3600].map(lasted), [
    "42 seconds",
    "10 minutes",
    "3 hours, 2 minutes",
    "30 hours",
    "2 days, 2 hours",
  ]);
});

it("a name becomes an address's fragment, and owns things as English writes it", () => {
  assert.equal(slug("Joel's Café TV"), "joel-s-cafe-tv");
  assert.equal(possessive("Upstairs"), "Upstairs'");
  assert.equal(possessive("Basement"), "Basement's");
});

it("a name in a command is quoted so pasting it runs nothing else", () => {
  for (const [name, typed] of [
    ["Study", "Study"],
    ["Main Floor", "'Main Floor'"],
    ["lamp;reboot", "'lamp;reboot'"],
    ["$(reboot)", "'$(reboot)'"],
    ["`reboot`", "'`reboot`'"],
    ["Joel's | sh", "'Joel'\\''s | sh'"],
    ["=ls", "'=ls'"],
    ["", "''"],
  ]) {
    assert.equal(shellWord(name), typed, `${name} wasn't one word that runs nothing`);
  }
});
