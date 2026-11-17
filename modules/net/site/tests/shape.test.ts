// How the network page lays the house out, and says its figures.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { study } from "../shape.ts";
import { lasted, linkSpeed, possessive, rank, rate, slug } from "../words.ts";
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

it("a measured rate keeps three figures, in the unit that holds it under a thousand", () => {
  assert.deepEqual([369.86, 11.93, 9.9, 0.4, 0.05, 1200, 999.96, 0.000_098_2].map(rate), [
    "370 Mb/s",
    "11.9 Mb/s",
    "9.90 Mb/s",
    "400 kb/s",
    "50.0 kb/s",
    "1.20 Gb/s",
    "1.00 Gb/s",
    "98.2 b/s",
  ]);
});

it("a rate of nothing, or of no number, is none", () => {
  assert.deepEqual([0, -3, Number.NaN].map(rate), ["0 b/s", "0 b/s", "0 b/s"]);
});

it("a link's speed is a name, said with only the digits it has", () => {
  assert.deepEqual([1000, 2500, 866, 72, 72.2, 100].map(linkSpeed), [
    "1 Gb/s",
    "2.5 Gb/s",
    "866 Mb/s",
    "72 Mb/s",
    "72.2 Mb/s",
    "100 Mb/s",
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
