// What device an address lands the network page on, as others link to it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { study } from "../shape.ts";
import { fragment, linkedTo } from "../where.ts";
import { records, told } from "./house.ts";

function shaped(these = records) {
  const { watched, kinds } = told(these);
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  return { network, shape: study(network) };
}

it("a device's On the Map row lands on it, its mac's colons encoded as eero writes them", () => {
  const { network, shape } = shaped();
  // modules/eero's rows.rs writes each colon of `device=mac:<mac>` as %3A.
  assert.equal(
    linkedTo(network, shape, "#device=mac%3A00%3A00%3A5e%3A00%3A53%3A10"),
    "mac:00:00:5e:00:53:10",
  );
});

/** Where each device's own address, as the page writes it, lands, by the device. */
function landings(these = records): { ids: string[]; landed: (string | null)[] } {
  const { network, shape } = shaped(these);
  const ids = network.devices.map((device) => device.id);
  const landed = ids.map((id) =>
    linkedTo(network, shape, `#device=${encodeURIComponent(fragment(network, shape, id))}`),
  );
  return { ids, landed };
}

it("every device's own address, by its name, lands back on it", () => {
  const { ids, landed } = landings();
  assert.deepEqual(landed, ids);
});

it("two devices of one name are each addressed by net's id, and land on it", () => {
  const printer = records.find((each) => each.name === "Kitchen Printer");
  assert.ok(printer, "the house has its printer");
  const twin = {
    ...printer,
    key: "device:mac:00:00:5e:00:53:11",
    id: "mac:00:00:5e:00:53:11",
    macs: ["00:00:5e:00:53:11"],
  };
  const { network, shape } = shaped([...records, twin]);
  assert.equal(fragment(network, shape, twin.id), twin.id, "a shared name picked one");
  const { ids, landed } = landings([...records, twin]);
  assert.deepEqual(landed, ids);
});

it("an address naming no device opens on none", () => {
  const { network, shape } = shaped();
  assert.equal(linkedTo(network, shape, "#device=mac%3A00%3A00%3A5e%3A00%3A53%3Aff"), null);
  assert.equal(linkedTo(network, shape, ""), null);
});

it("an address that doesn't decode opens on none, rather than stopping the page", () => {
  const { network, shape } = shaped();
  assert.equal(linkedTo(network, shape, "#device=%"), null);
  assert.equal(linkedTo(network, shape, "#device=mac%3A00%E0%A4%A"), null);
});
