// How a cell is named the one way every site names it: read out of a link
// or its address, and written as the link mycelium-link reads.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { addressIn, cellLink } from "../cell.ts";

it("a cell's address is read out of its whole link, whatever follows its name", () => {
  for (const link of [
    "mycelium:cell/SKIFF1",
    "mycelium:cell/SKIFF1@kestrel",
    "mycelium:cell/SKIFF1@kestrel#notice:42",
    "mycelium:cell/SKIFF1@kestrel/go",
    "MYCELIUM://cell/SKIFF1?from=map",
  ]) {
    assert.equal(addressIn(link), "SKIFF1", link);
  }
});

it("an address alone, or with its machine and anchor, names the same cell", () => {
  for (const named of ["SKIFF1", "SKIFF1@kestrel", "SKIFF1@kestrel#notice:42"]) {
    assert.equal(addressIn(named), "SKIFF1", named);
  }
});

it("a link to anything else, or a misshaped address or machine, names no cell", () => {
  for (const named of [
    "mycelium:network/orchestration.skiff1",
    "mycelium:machine/kestrel",
    "mycelium:cell/",
    "mycelium:cell/skiff1",
    "calls@kestrel",
    "SKIFF1@",
    "SKIFF1@-kestrel",
    "SKIFF1@kes trel",
    "ABCDEFGHIJKLM",
    "",
  ]) {
    assert.equal(addressIn(named), null, named);
  }
});

it("a cell's link says its machine where it has one a link may carry", () => {
  assert.equal(cellLink("SKIFF1", "kestrel"), "mycelium:cell/SKIFF1@kestrel");
  assert.equal(cellLink("SKIFF1", "osprey.local"), "mycelium:cell/SKIFF1@osprey.local");
  assert.equal(cellLink("SKIFF1"), "mycelium:cell/SKIFF1");
  assert.equal(cellLink("SKIFF1", null), "mycelium:cell/SKIFF1");
  assert.equal(cellLink("SKIFF1", ""), "mycelium:cell/SKIFF1");
  assert.equal(cellLink("SKIFF1", "a machine"), "mycelium:cell/SKIFF1");
});

it("an address the launcher couldn't reach has no link", () => {
  for (const address of ["calls", "skiff1", "", "ABCDEFGHIJKLM"]) {
    assert.equal(cellLink(address, "kestrel"), null, address);
  }
});

it("each link written reads back as the address it was written for", () => {
  for (const machine of ["kestrel", null, "a machine"]) {
    const link = cellLink("TERN02", machine);
    assert.ok(link !== null, String(machine));
    assert.equal(addressIn(link), "TERN02", link);
  }
});
