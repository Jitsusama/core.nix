// How the forage page reads the needs one cell serves for another, and
// joins each pair of cells they tie once on the floor.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { joined } from "../joins.ts";
import { foragesOf } from "../model.ts";
import { forage, told } from "./records.ts";

it("reads the needs a cell serves and relies on, passing over one naming no other cell", () => {
  const { forages, unread } = foragesOf(
    told([
      forage("KESTRL", {
        serves: [
          { cell: "OSPREY", need: "a hover review", since: "2026-11-18T13:05:00Z" },
          { need: "lost" },
          { cell: "", need: "nobody" },
          { cell: "KESTRL", need: "itself" },
        ],
        relies: [{ cell: "TERNAA" }],
      }),
      forage("OSPREY", { serves: { cell: "KESTRL" }, relies: null }),
    ]),
  );
  const [kestrel] = forages;
  assert.deepEqual(kestrel?.serves, [
    { cell: "OSPREY", need: "a hover review", since: "2026-11-18T13:05:00Z" },
  ]);
  assert.deepEqual(kestrel?.relies, [{ cell: "TERNAA", need: "", since: undefined }]);
  assert.deepEqual(forages[1]?.serves, []);
  assert.deepEqual(unread, [
    "3 of KESTRL's serves named no other cell",
    "OSPREY's serves listed no needs",
  ]);
});

it("a forage the node sends no joins for has none, and splits from nothing", () => {
  const [kestrel] = foragesOf(told([forage("KESTRL")])).forages;
  assert.equal(kestrel?.splitFrom, undefined);
  assert.deepEqual(kestrel?.splitInto, []);
  assert.deepEqual(kestrel?.serves, []);
  assert.deepEqual(kestrel?.relies, []);
});

it("joins each pair a need ties once, however both ends keep it, and only cells here", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", { serves: [{ cell: "OSPREY", need: "a review" }, { cell: "FARAWAY" }] }),
      forage("OSPREY", { relies: [{ cell: "KESTRL", need: "a review" }, { cell: "OSPREY" }] }),
      forage("TERNAA", { relies: [{ cell: "KESTRL", need: "a plan" }] }),
    ]),
  );
  assert.deepEqual(joined(forages), [
    { from: "KESTRL", to: "OSPREY" },
    { from: "TERNAA", to: "KESTRL" },
  ]);
});

it("joins a pair once even when its two ends word the need differently, or keep two", () => {
  const { forages } = foragesOf(
    told([
      forage("KESTRL", {
        serves: [
          { cell: "OSPREY", need: "a review" },
          { cell: "OSPREY", need: "a second look" },
        ],
      }),
      forage("OSPREY", {
        relies: [{ cell: "KESTRL", need: "review my hover" }],
        // A need the other way round is still the same pair on the floor.
        serves: [{ cell: "KESTRL" }],
      }),
    ]),
  );
  assert.deepEqual(joined(forages), [{ from: "KESTRL", to: "OSPREY" }]);
});
