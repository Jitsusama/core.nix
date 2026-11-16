// What device an address lands the network page on, as others link to it.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { networkOf } from "../model.ts";
import { study } from "../shape.ts";
import { fragment, linkedTo } from "../where.ts";
import { told } from "./house.ts";

function shaped() {
  const { watched, kinds } = told();
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

it("every device's own address lands back on it, named or not", () => {
  const { network, shape } = shaped();
  for (const device of network.devices) {
    const address = `#device=${encodeURIComponent(fragment(network, shape, device.id))}`;
    assert.equal(linkedTo(network, shape, address), device.id, `${address} lands elsewhere`);
  }
});

it("an address naming no device opens on none", () => {
  const { network, shape } = shaped();
  assert.equal(linkedTo(network, shape, "#device=mac%3A00%3A00%3A5e%3A00%3A53%3Aff"), null);
  assert.equal(linkedTo(network, shape, ""), null);
});
