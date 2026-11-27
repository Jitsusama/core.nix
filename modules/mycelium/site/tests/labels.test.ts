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
// The stem two long ids clip to, alike, before each takes its number.
const STEM_W = "(SKIFF-OF-THE-NORTH-SHORE-AND-THE-W...";
// Any run of the spaces, tabs and line breaks a page draws as one; a no-break space it keeps.
const SPACES = /[ \t\n\r\f]+/gu;
// The space a page drops at either end, once each run is one.
const EDGE_SPACES = /^ | $/gu;
// A text starting or ending on a space.
const AT_AN_END = /^ | $/u;

/** A text as a page draws it, its spaces run together and none at its ends. */
const drawn = (text: string) => text.replace(SPACES, " ").replace(EDGE_SPACES, "");

/** A clip as the map's: the text whole within the room, else cut short with dots. */
const clip = (text: string, within: number) =>
  text.length <= within ? text : `${text.slice(0, within - 3)}...`;

/** Labels the map would draw for `networks`, said to be apart and within its room. */
const labelledApart = (networks: readonly Named[]): ReadonlyMap<string, string> => {
  const labels = labelsApart(networks, toldApart(networks), clip, ROOM);
  const each = [...labels.values()];
  assert.equal(
    new Set(each.map(drawn)).size,
    each.length,
    `two labels read alike: ${each.join(" | ")}`,
  );
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

it("a name keeps a place beside its id while eight of its characters fit, and no fewer", () => {
  const name = "Rig the Skiff for the Whole of the Coming Season";
  const pair = (ids: readonly string[]) =>
    labelledApart(
      ids.map((id, at) => ({ key: `orchestration.${id}`, name, carries: `Mend gauges ${at}` })),
    );
  // Each id is 29 characters, its hint 31, which leaves the name 8.
  const fits = pair(["skiff-of-the-north-shore-ab-1", "skiff-of-the-north-shore-ab-2"]);
  assert.equal(
    fits.get("orchestration.skiff-of-the-north-shore-ab-1"),
    "Rig t... (SKIFF-OF-THE-NORTH-SHORE-AB-1)",
    "a name with eight characters of room lost its place beside its id",
  );
  // One character more leaves it 7, so the id is the label alone.
  const over = pair(["skiff-of-the-north-shore-abc-1", "skiff-of-the-north-shore-abc-2"]);
  assert.equal(
    over.get("orchestration.skiff-of-the-north-shore-abc-1"),
    "(SKIFF-OF-THE-NORTH-SHORE-ABC-1)",
    "a name with seven characters of room kept a place beside its id",
  );
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

it("two sets of ids that clip to one stem share its numbers, so none read alike", () => {
  const name = "Rig the Skiff for the Season";
  const stem = "orchestration.skiff-of-the-north-shore-and-the-w";
  const keys = [`${stem}ab-bay`, `${stem}ab-cove`, `${stem}cd-bay`, `${stem}cd-cove`];
  const labels = labelledApart(keys.map((key) => ({ key, name, carries: key })));
  assert.deepEqual(
    keys.map((key) => labels.get(key)),
    [1, 2, 3, 4].map((place) => `(SKIFF-OF-THE-NORTH-SHORE-AND-THE-W... ${place}`),
  );
});

it("a number a network's own title stands on is passed over, so none read alike", () => {
  const name = "Rig the Skiff for the Season";
  const shore = "orchestration.skiff-of-the-north-shore-and-the-whole-bay";
  const bay = "orchestration.skiff-of-the-north-shore-and-the-whole-cove";
  const first = `${STEM_W} 1`;
  const labels = labelledApart([
    { key: shore, name, carries: "Mend the north gauges" },
    { key: bay, name, carries: "Mend the south gauges" },
    { key: "orchestration.heron1", name: first, carries: first },
  ]);
  assert.deepEqual(
    [labels.get(shore), labels.get(bay), labels.get("orchestration.heron1")],
    [`${STEM_W} 2`, `${STEM_W} 3`, first],
    "the title keeps its words, and the numbers step past it",
  );
});

it("ids alike past nine leave their stem room for a number of two digits", () => {
  const name = "Rig the Skiff for the Season";
  const stem = "orchestration.skiff-of-the-north-shore-and-the-whole-bay-";
  const places = Array.from({ length: 12 }, (_, at) => at + 1);
  const keys = places.map((place) => `${stem}${String(place).padStart(2, "0")}`);
  const labels = labelledApart(keys.map((key) => ({ key, name, carries: key })));
  assert.deepEqual(
    keys.map((key) => labels.get(key)),
    places.map((place) => `(SKIFF-OF-THE-NORTH-SHORE-AND-THE-... ${place}`),
  );
});

it("a network keeps its number however the node lists the networks alike with it", () => {
  const name = "Rig the Skiff for the Season";
  const shore = "orchestration.skiff-of-the-north-shore-and-the-whole-bay";
  const bay = "orchestration.skiff-of-the-north-shore-and-the-whole-cove";
  const listed = [
    { key: shore, name, carries: "Mend the north gauges" },
    { key: bay, name, carries: "Mend the south gauges" },
  ];
  const forward = labelledApart(listed);
  const backward = labelledApart(listed.toReversed());
  assert.deepEqual(
    [backward.get(shore), backward.get(bay)],
    [forward.get(shore), forward.get(bay)],
  );
});

it("a title padded with spaces the map draws as one is told apart as drawn", () => {
  const name = "Rig the Skiff for the Season";
  const labels = labelledApart([
    { key: "orchestration.skiff1", name, carries: "Mend the north gauges" },
    {
      key: "orchestration.heron1",
      name: " Rig the  Skiff for\tthe Season ",
      carries: " Rig the  Skiff for\tthe Season ",
    },
  ]);
  assert.equal(
    labels.get("orchestration.heron1"),
    name,
    "a padded title wasn't read as the words it draws, so it wasn't told apart as drawn",
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

it("padded titles hinted beside their ids are clipped as drawn, then take their ids", () => {
  const labels = labelledApart([
    {
      key: "orchestration.skiff1",
      name: " Rig the  Skiff for\tthe Whole of the Coming Season ",
      carries: "Mend the north gauges",
    },
    {
      key: "orchestration.skiff2",
      name: "Rig the Skiff for the\nWhole of  the Coming Season",
      carries: "Mend the south gauges",
    },
  ]);
  assert.deepEqual(
    [labels.get("orchestration.skiff1"), labels.get("orchestration.skiff2")],
    ["Rig the Skiff for the Whole ... (SKIFF1)", "Rig the Skiff for the Whole ... (SKIFF2)"],
    "a padded title beside its id wasn't clipped as the words it draws",
  );
});

it("a title that draws as nothing is its tell alone, with no space at either end", () => {
  const blank = (id: string, carries: string) => ({
    key: `orchestration.${id}`,
    name: " \t ",
    carries,
  });
  const networks = [
    blank("heron1", "Mend the north gauges along the whole shore first"),
    blank("heron2", "Mend the north gauges along the whole shore second"),
    blank("heron3", ""),
    blank("heron4", " \n "),
  ];
  const told = toldApart(networks);
  const labels = labelledApart(networks);
  for (const said of [...told.values(), ...labels.values()]) {
    assert.doesNotMatch(said, AT_AN_END, `"${said}" starts or ends on a space`);
  }
  assert.deepEqual(
    networks.map((n) => told.get(n.key)),
    [
      "Mend the north gauges along the whole shore first",
      "Mend the north gauges along the whole shore second",
      "(HERON3)",
      "(HERON4)",
    ],
    "a title that draws as nothing wasn't told by its tell alone",
  );
  assert.deepEqual(
    networks.map((n) => labels.get(n.key)),
    ["(HERON1)", "(HERON2)", "(HERON3)", "(HERON4)"],
    "a title that draws as nothing wasn't labelled by its id alone",
  );
});

it("a lone title that draws as nothing is told by what it carries, else by its id", () => {
  const skiff = { key: "orchestration.skiff1", name: "Rig the Skiff for the Season", carries: "" };
  const lone = (carries: string) => [{ key: "orchestration.heron1", name: " \t ", carries }, skiff];
  const read = (networks: readonly Named[]) => [
    toldApart(networks).get("orchestration.heron1"),
    labelledApart(networks).get("orchestration.heron1"),
    toldApart(networks).get(skiff.key),
  ];
  assert.deepEqual(
    [read(lone("Mend the north gauges")), read(lone(" \n "))],
    [
      ["Mend the north gauges", "Mend the north gauges", skiff.name],
      ["(HERON1)", "(HERON1)", skiff.name],
    ],
    "a lone title that draws as nothing was told as nothing, or its neighbour told more",
  );
});

it("what a network carries is read as drawn, so its padded name tells nothing more", () => {
  const name = "Rig the Skiff for the Season";
  const told = toldApart([
    { key: "orchestration.skiff1", name, carries: " Rig the  Skiff for\tthe Season " },
    { key: "orchestration.skiff2", name, carries: "Mend the  south\tgauges " },
  ]);
  assert.deepEqual(
    [told.get("orchestration.skiff1"), told.get("orchestration.skiff2")],
    [name, `${name}: Mend the south gauges`],
    "what a network carries wasn't read as the words it draws",
  );
});

it("an id with spaces or a line break in it is hinted as drawn", () => {
  const name = "Rig the Skiff for the Whole of the Coming Season";
  const networks = [
    { key: "orchestration.a  b", name, carries: name },
    { key: "orchestration.c\nd", name, carries: name },
  ];
  const told = toldApart(networks);
  const labels = labelledApart(networks);
  assert.deepEqual(
    networks.map((n) => told.get(n.key)),
    [`${name} (A B)`, `${name} (C D)`],
    "an id's hint in a told name wasn't drawn as the map draws it",
  );
  assert.deepEqual(
    networks.map((n) => labels.get(n.key)),
    ["Rig the Skiff for the Whole of ... (A B)", "Rig the Skiff for the Whole of ... (C D)"],
    "an id's hint in a label wasn't drawn as the map draws it",
  );
});

it("a no-break space, which a page draws as it is, is kept apart from a space", () => {
  const name = "Rig the Skiff for the Season";
  const kept = ["Rig the\u00a0Skiff for the Season", "\u00a0Rig the Skiff for the Season"];
  const networks = [name, ...kept].map((title, at) => ({
    key: `orchestration.skiff${at}`,
    name: title,
    carries: title,
  }));
  const labels = labelledApart(networks);
  assert.deepEqual(
    networks.map((n) => labels.get(n.key)),
    [name, ...kept],
    "a no-break space was run into a space, or dropped at an end, as a page never does",
  );
});
