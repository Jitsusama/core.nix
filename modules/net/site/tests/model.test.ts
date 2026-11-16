// What the network page makes of net's records, told as net's daemon tells
// them, one watch a kind.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf, worthALook } from "../model.ts";
import { records, told } from "./house.ts";

/** The house's network, which every case here expects to be drawn. */
function house() {
  const { watched, kinds } = told();
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  return network;
}

it("nothing is drawn until every kind has been told once", () => {
  const { watched, kinds } = told(records.filter((record) => record.kind !== "internet"));
  assert.equal(networkOf(watched, kinds), undefined);
});

it("an empty kind, once told, still lets the network be drawn", () => {
  const { watched } = told(records.filter((record) => record.kind !== "concern"));
  const kinds = new Set(["device", "link", "source", "internet", "concern"]);
  assert.deepEqual(networkOf(watched, kinds)?.concerns, []);
});

it("concerns come worst first by their rank, not in the order a watch hears them", () => {
  assert.deepEqual(
    house().concerns.map((concern) => concern.kind),
    ["lost", "weak", "busy"],
  );
});

it("a concern net gave no rank goes after every one it placed", () => {
  const unranked = records.map((record) =>
    record.key.startsWith("concern:lost") ? { ...record, rank: undefined } : record,
  );
  const { watched, kinds } = told(unranked);
  assert.equal(networkOf(watched, kinds)?.concerns.at(-1)?.kind, "lost");
});

it("what's worth a look about a device leaves out the internet's own ceiling", () => {
  const ceiling = {
    key: "concern:ceiling:modem:down",
    kind: "concern",
    concern: "ceiling",
    device: "modem",
    on: "down",
    says: "The internet is the narrowest point down.",
    rank: 3,
  };
  const { watched, kinds } = told([...records, ceiling]);
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  assert.deepEqual(
    worthALook(network, "modem").map((concern) => concern.kind),
    ["lost"],
  );
});

it("what's worth a look about a device counts the concerns naming it as the other end", () => {
  const weakToUpstairs = records.map((record) =>
    record.key.startsWith("concern:weak") ? { ...record, other: "eero:2" } : record,
  );
  const { watched, kinds } = told(weakToUpstairs);
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  assert.deepEqual(
    worthALook(network, "eero:2").map((concern) => concern.kind),
    ["weak", "busy"],
  );
});

it("a concern's own kind is read from its concern field, not the record's kind", () => {
  const [worst] = house().concerns;
  assert.deepEqual(
    { kind: worst?.kind, device: worst?.device, on: worst?.on },
    { kind: "lost", device: "modem", on: "downstream channel 16" },
  );
});

it("a record that can't be read is left out and named, the rest still drawn", () => {
  const network = house();
  assert.deepEqual(network.unread, ["device device:mac:00:00:5e:00:53:99"]);
  assert.equal(network.devices.length, 4);
});

it("net's names are read into the page's own", () => {
  const network = house();
  const printer = network.devices.find((device) => device.name === "Kitchen Printer");
  assert.deepEqual(printer?.speed, {
    downMbps: 0.4,
    upMbps: 0.1,
    busiestDownMbps: 3.2,
    busiestDownAt: 1_700_000_050,
    busiestUpMbps: undefined,
    busiestUpAt: undefined,
  });
  const basement = network.devices.find((device) => device.id === "eero:1");
  assert.deepEqual(basement?.radios[0], {
    name: "5 GHz low",
    channel: 36,
    widthMhz: 80,
    busyPct: 12,
    busiestPct: 40,
    busiestAt: undefined,
  });
  assert.deepEqual(basement?.path?.down, {
    a: "modem",
    b: undefined,
    roomMbps: 370,
    unmeasured: 0,
  });
});

it("a link's crossings are what net counted, and none when it says nothing", () => {
  const network = house();
  assert.deepEqual(
    network.links.map((link) => [link.a.device, link.crossings, link.signalDbm]),
    [
      ["eero:1", 3, undefined],
      ["eero:2", 0, -61],
      ["mac:00:00:5e:00:53:10", 0, -74],
    ],
  );
});

it("the modem's channels keep what each lost", () => {
  const modem = house().devices.find((device) => device.id === "modem")?.modem;
  assert.deepEqual(
    modem?.downstream.map((channel) => [channel.channel, channel.uncorrectable]),
    [
      [16, 878],
      [3, 12],
    ],
  );
  assert.equal(modem?.upstream[0]?.snrDb, undefined);
});

it("the sources come by name, and the network was read when its newest part was", () => {
  const network = house();
  assert.deepEqual(
    network.sources.map((source) => source.name),
    ["eero", "modem"],
  );
  assert.equal(network.readAt, 1_700_000_200);
});

it("the internet is the one record of its kind", () => {
  assert.deepEqual(house().internet, {
    provider: "Example ISP",
    downMbps: 369.86,
    upMbps: 11.93,
    measuredAt: 1_699_990_000,
    source: "eero",
    carried: undefined,
  });
});
