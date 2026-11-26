// Each network's label on the map, where a told name is clipped to the map's
// room: none read alike there, and none runs past the room, whatever the
// names, the ids or a title written to read as another's say.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { labelsApart, type Named, toldApart } from "../words.ts";

// The map's room for a name, in characters (view.js's NAME_ROOM).
const ROOM = 40;
// A label clipped beside a hint ends on its id.
const ENDS_ON_HERON = /\(HERON1\)$/u;

/** A clip as the map's: the text whole within the room, else cut short with dots. */
const clip = (text: string, within: number) =>
  text.length <= within ? text : `${text.slice(0, within - 3)}...`;

/** Labels the map would draw for `networks`, said to be apart and within its room. */
const labelledApart = (networks: readonly Named[]): ReadonlyMap<string, string> => {
  const labels = labelsApart(networks, toldApart(networks), clip, ROOM);
  const each = [...labels.values()];
  assert.equal(new Set(each).size, each.length, `two labels read alike: ${each.join(" | ")}`);
  for (const label of each) {
    assert.ok(label.length <= ROOM, `${label} is ${label.length} long, past the room of ${ROOM}`);
  }
  return labels;
};

it("a told name the map clips alike to another's keeps its id there instead", () => {
  const slice = (key: string, carries: string) => ({
    key,
    name: "Rig the Skiff for the Season",
    carries,
  });
  const labels = labelledApart([
    slice("orchestration.skiff1", "Mend the north gauges"),
    slice("orchestration.skiff2", "Mend the south gauges"),
    { key: "orchestration.heron1", name: "Chart the Harbour", carries: "Chart the Harbour" },
  ]);
  assert.deepEqual(
    Object.fromEntries(labels),
    {
      "orchestration.skiff1": "Rig the Skiff for the Season (SKIFF1)",
      "orchestration.skiff2": "Rig the Skiff for the Season (SKIFF2)",
      "orchestration.heron1": "Chart the Harbour",
    },
    "what told the two apart was past the clip, so their ids tell them apart on the map",
  );
});

it("a long name clipped beside its id leaves the id room within the map's", () => {
  const name = "Rig the Skiff for the Whole of the Coming Season";
  const labels = labelledApart([
    { key: "orchestration.skiff1", name, carries: "Mend the north gauges" },
    { key: "orchestration.skiff2", name, carries: "Mend the south gauges" },
  ]);
  assert.equal(labels.get("orchestration.skiff1"), "Rig the Skiff for the Whole ... (SKIFF1)");
});

it("an id too long to leave its name room is the label alone, clipped to the room", () => {
  const name = "Rig the Skiff for the Season";
  const long = "orchestration.skiff-of-the-north-shore-and-the-bay";
  const labels = labelledApart([
    { key: long, name, carries: "Mend the north gauges along the whole shore" },
    { key: "orchestration.skiff2", name, carries: "Mend the south gauges along the bay" },
  ]);
  assert.equal(labels.get(long), "(SKIFF-OF-THE-NORTH-SHORE-AND-THE-BAY)");
});

it("two ids too long for the room that clip alike are numbered apart", () => {
  const name = "Rig the Skiff for the Season";
  const shore = "orchestration.skiff-of-the-north-shore-and-the-whole-bay";
  const bay = "orchestration.skiff-of-the-north-shore-and-the-whole-cove";
  const labels = labelledApart([
    { key: shore, name, carries: "Mend the north gauges" },
    { key: bay, name, carries: "Mend the south gauges" },
  ]);
  assert.deepEqual(
    [labels.get(shore), labels.get(bay)],
    ["(SKIFF-OF-THE-NORTH-SHORE-AND-THE-W... 1", "(SKIFF-OF-THE-NORTH-SHORE-AND-THE-W... 2"],
  );
});

it("a title written as another's id-hinted label takes its own id in turn", () => {
  const name = "Rig the Skiff for the Season";
  const labels = labelledApart([
    { key: "orchestration.skiff1", name, carries: "Mend the north gauges" },
    { key: "orchestration.skiff2", name, carries: "Mend the south gauges" },
    {
      key: "orchestration.heron1",
      name: "Rig the Skiff for the Season (SKIFF1)",
      carries: "Rig the Skiff for the Season (SKIFF1)",
    },
  ]);
  assert.match(labels.get("orchestration.heron1") ?? "", ENDS_ON_HERON);
});
