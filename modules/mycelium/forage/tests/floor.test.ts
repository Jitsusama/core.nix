// Which cells the forage page's floor draws when the list holds more than
// it can: those around the cell it's drawn around, with the cells that grew
// from that one, and each cell drawn with every cell it grew from, so no
// branch hangs from a starter left off and a booked start far down a long
// list still draws solid; and when a step draws the floor again.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { floorAfter, floorFor, floorOf, floorStepped, stepsOff, UNDRAWN } from "../draw.ts";
import type { Forage } from "../model.ts";

/** A forage for `cell` with `more` over its fields, its documents none. */
function forage(cell: string, more: Partial<Forage> = {}): Forage {
  return {
    cell,
    machine: "kestrel",
    quest: `QEST-20261116-${cell}`,
    purpose: `${cell} learns to hover.`,
    status: "active",
    documents: [],
    waits: [],
    startedBy: undefined,
    booked: false,
    parent: undefined,
    splitFrom: undefined,
    splitInto: [],
    serves: [],
    relies: [],
    depth: 0,
    ...more,
  };
}

/** The cell at `at` in the lists below. */
const named = (at: number): string => `C${String(at).padStart(2, "0")}`;

/** The cells from `first` to `last`, both drawn. */
const span = (first: number, last: number): string[] =>
  Array.from({ length: last - first + 1 }, (_, at) => named(first + at));

/** Sixty top cells, C00 to C59, with a starter at C10 and the cell it booked at C55. */
const LONG: readonly Forage[] = Array.from({ length: 60 }, (_, at) =>
  at === 55 ? forage(named(at), { startedBy: "C10", booked: true, depth: 1 }) : forage(named(at)),
);

/** `LONG` with `cell` started by `by`. */
const startedOn = (list: readonly Forage[], cell: string, by: string): readonly Forage[] =>
  list.map((each) => (each.cell === cell ? { ...each, startedBy: by, depth: 1 } : each));

/** The cells `floor` draws a compartment for, in the order it draws them. */
const compartments = (floor: string): string[] =>
  [...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)"/gu)].map((match) => match[1] ?? "");

/**
 * What `floor` draws along its main line, left to right: each compartment's
 * cell, and an ellipsis for each gap where it skips cells of its list.
 */
const along = (floor: string): string[] =>
  [
    ...[...floor.matchAll(/<rect class="[^"]*" data-cell="([^"]+)" x="([^"]+)"/gu)].map(
      (match) => ({ mark: match[1] ?? "", x: Number(match[2]) + 11 }),
    ),
    ...[...floor.matchAll(/<text class="gap" x="([^"]+)"/gu)].map((match) => ({
      mark: "\u2026",
      x: Number(match[1]),
    })),
  ]
    .sort((one, other) => one.x - other.x)
    .map((each) => each.mark);

/** The main line, the one hypha naming no cell. */
const MAIN = /<path class="hypha" d="([^"]+)"/u;

/** The runs `floor`'s main line is drawn in, broken at each gap. */
const runs = (floor: string): number => (floor.match(MAIN)?.[1] ?? "").split("M").length - 1;

/** A ringed compartment, naming its cell. */
const RINGED = /<rect class="[^"]* on" data-cell="([^"]+)"/u;

/** A floor's width and height in its own units. */
const VIEWED = /viewBox="0 0 (\d+) (\d+)"/u;
// The floor's smallest words in its own units, and the least the kit draws text at.
const WORD_PX = 13;
const LEAST_PX = 12;

/** The cell whose compartment `floor` rings. */
const ringed = (floor: string): string | undefined => floor.match(RINGED)?.[1];

/** The floor of `list` drawn around `around`, ringing `cursor`. */
const floorAt = (
  list: readonly Forage[],
  cursor: string | null,
  around: string | null = cursor,
): string => floorOf(floorFor(list, around), cursor, "kestrel");

it("draws every cell of a list that fits, whatever the cursor", () => {
  assert.deepEqual(compartments(floorAt(LONG.slice(0, 5), "C04")), span(0, 4));
  assert.deepEqual(compartments(floorAt(LONG.slice(0, 48), "C47")), span(0, 47));
});

// The walk draws a list that fits whole either way, so this case, not the
// one before, is what pins drawn()'s 48.
it("draws 48 of a list one cell too long", () => {
  assert.deepEqual(compartments(floorAt(LONG.slice(0, 49), "C00")), span(0, 47));
});

it("marks no gap on a list that fits, drawing its main line whole and as wide as before", () => {
  const floor = floorAt(LONG.slice(0, 48), "C47");
  assert.deepEqual(along(floor), span(0, 47));
  assert.equal(runs(floor), 1);
  // Its margins and 48 rooms of 30.
  assert.ok(floor.startsWith('<svg viewBox="0 0 1488 '), floor);
});

it("marks each gap where the floor skips cells of its list, before, between and after", () => {
  const cut = "\u2026";
  // C00 to C09 come before C10, and C11 and C12 between it and C13.
  const around = floorAt(LONG, "C55");
  assert.deepEqual(along(around), [cut, "C10", cut, ...span(13, 59)]);
  // The line breaks for a gap of 24 centred on each mark, at 36 and 90, and
  // runs on to the middle of each margin, a dash before the first saying
  // the list goes on before it.
  assert.equal(around.match(MAIN)?.[1], "M12 78H24M48 78H78M102 78H1524");
  assert.ok(around.includes("and 12 more in the list"), around);
  // The list's first 48 leave its last 12 after them.
  const first = floorAt(LONG, null);
  assert.deepEqual(along(first), [...span(0, 47), cut]);
  // Its 48 rooms end at 1464, the gap's mark at 1476, and a dash after it.
  assert.equal(first.match(MAIN)?.[1], "M12 78H1464M1488 78H1500");
  // Its 48 rooms and a gap's 24 between the margins.
  assert.ok(first.startsWith('<svg viewBox="0 0 1512 '), first);
});

it("draws the list's first 48 with no cursor, and counts the rest", () => {
  const floor = floorAt(LONG, null);
  assert.deepEqual(compartments(floor), span(0, 47));
  assert.ok(floor.includes("and 12 more in the list"), floor);
  // C55's starter is drawn, but C55 isn't, so no branch is.
  assert.ok(!floor.includes('data-cell="C55"'), floor);
});

it("draws a cursor past the 48th with its starter, so its booked branch is solid", () => {
  const floor = floorAt(LONG, "C55");
  // Its starter, then the 47 nearest it, kept in the list's order.
  assert.deepEqual(compartments(floor), ["C10", ...span(13, 59)]);
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
  assert.equal(ringed(floor), "C55");
});

it("draws the cells a cursor started, however far down the list", () => {
  const floor = floorAt(LONG, "C10");
  assert.deepEqual(compartments(floor), [...span(0, 46), "C55"]);
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
  // One it only claims to have started comes after the booked one, ahead
  // of the nearest cells.
  const claimed = floorAt(startedOn(LONG, "C58", "C10"), "C10");
  assert.deepEqual(compartments(claimed), [...span(0, 45), "C55", "C58"]);
});

it("draws a cell near the cursor only with every cell it grew from", () => {
  const list = startedOn(LONG, "C30", "C02");
  const floor = floorAt(list, "C40");
  const drawn = compartments(floor);
  assert.equal(drawn.length, 48);
  assert.ok(drawn.includes("C30") && drawn.includes("C02"), drawn.join(" "));
  assert.ok(floor.includes('<path class="hypha claimed" data-cell="C30"'), floor);
  // C55 is drawn, so its starter is too.
  assert.ok(drawn.includes("C55") && drawn.includes("C10"), drawn.join(" "));
  assert.ok(floor.includes('<path class="hypha" data-cell="C55"'), floor);
});

it("follows a ring of starts once round, drawing each cell in it", () => {
  const two = startedOn(LONG, "C10", "C55");
  const three = startedOn(startedOn(LONG, "C10", "C30"), "C30", "C55");
  for (const [ring, cells] of [
    [two, ["C10", "C55"]],
    [three, ["C10", "C30", "C55"]],
  ] as const) {
    const drawn = compartments(floorAt(ring, "C55"));
    assert.equal(drawn.length, 48, drawn.join(" "));
    assert.equal(new Set(drawn).size, 48, drawn.join(" "));
    assert.ok(
      cells.every((cell) => drawn.includes(cell)),
      drawn.join(" "),
    );
  }
  // The ring at the cursor fits however often it's followed round, so only a
  // ring beside it shows the walk stops once round: followed round more than
  // once, it would ask for more room than is left, and go undrawn.
  const beside = startedOn(startedOn(LONG, "C51", "C52"), "C52", "C51");
  const drawn = compartments(floorAt(beside, "C50"));
  assert.ok(drawn.includes("C51") && drawn.includes("C52"), drawn.join(" "));
});

/** Ninety cells, C00 to C89, grown from each other in chains of every depth. */
const GROWN: readonly Forage[] = Array.from({ length: 90 }, (_, at) => {
  if (at > 0 && at % 4 === 0) {
    return forage(named(at), { startedBy: named(Math.floor(at * 0.6)), booked: true, depth: 1 });
  }
  return at % 4 === 1 && at > 1
    ? forage(named(at), { startedBy: named(at - 1), booked: true, depth: 2 })
    : forage(named(at));
});

it("draws 48 cells around any cursor, with those it started, each with every cell it grew from", () => {
  const starter = new Map(GROWN.map((each) => [each.cell, each.startedBy]));
  for (const { cell } of GROWN) {
    const drawn = compartments(floorAt(GROWN, cell));
    assert.equal(drawn.length, 48, `${cell}: ${drawn.join(" ")}`);
    assert.ok(drawn.includes(cell), `${cell}: ${drawn.join(" ")}`);
    for (const each of GROWN.filter((one) => one.startedBy === cell)) {
      assert.ok(drawn.includes(each.cell), `${cell}: leaves off ${each.cell}`);
    }
    for (const each of drawn) {
      const by = starter.get(each);
      assert.ok(by === undefined || drawn.includes(by), `${cell}: ${each} without ${by}`);
    }
  }
});

it("draws around the cell the page keeps it at, ringing the cursor there", () => {
  const floor = floorAt(LONG, "C30", "C55");
  assert.deepEqual(compartments(floor), ["C10", ...span(13, 59)]);
  assert.equal(ringed(floor), "C30");
});

/** Sixty cells, C00 to C59, each started by the one before it. */
const CHAIN: readonly Forage[] = Array.from({ length: 60 }, (_, at) =>
  at === 0
    ? forage(named(at))
    : forage(named(at), { startedBy: named(at - 1), depth: Math.min(at, 3) }),
);

it("cuts a line deeper than the floor at its top, keeping the 48 nearest the cursor", () => {
  const floor = floorAt(CHAIN, "C59");
  assert.deepEqual(compartments(floor), span(12, 59));
  // C12's starter isn't drawn, so no branch leads to it; C13's is.
  assert.ok(!floor.includes('data-cell="C12" d='), floor);
  assert.ok(floor.includes('data-cell="C13" d='), floor);
  // C51 only claims C50 as its starter, so C50's line keeps the whole floor
  // and C51 is left off, and a step to C50 keeps the floor around it.
  assert.deepEqual(compartments(floorAt(CHAIN, "C50")), span(3, 50));
  assert.equal(stepsOff(floorFor(CHAIN, "C50"), "C50"), false);
});

/** Forty cells each started by the one before, and forty more the last of them booked. */
const BROOD: readonly Forage[] = Array.from({ length: 80 }, (_, at) => {
  if (at === 0) {
    return forage(named(at));
  }
  const by = named(Math.min(at - 1, 39));
  return forage(named(at), { startedBy: by, booked: true, depth: Math.min(at, 3) });
});

it("cuts a line shallower than the floor to leave its cell's booked brood up to half", () => {
  // C39's line of 40 would fit whole, but the 40 cells booked as started by
  // C39 take 24 places, so it's cut to the other 24.
  assert.deepEqual(compartments(floorAt(BROOD, "C39")), span(16, 63));
});

it("keeps a line whole beside a brood that only claims its cell", () => {
  const claimed = BROOD.map((each, at) => (at >= 40 ? { ...each, booked: false } : each));
  // None of C39's children is booked, so its line of 40 keeps the whole
  // floor, and 8 claimed cells fill the rest.
  assert.deepEqual(compartments(floorAt(claimed, "C39")), span(0, 47));
});

it("cuts a line only for the booked part of a mixed brood", () => {
  const mixed = BROOD.map((each, at) => (at >= 50 ? { ...each, booked: false } : each));
  // C40 to C49 are booked, so C39's line of 40 is cut to 38 beside them,
  // and the 30 claimed cells after them get no room.
  assert.deepEqual(compartments(floorAt(mixed, "C39")), span(2, 49));
});

/** A top cell, C00, then a ring of 59 claimed starts: C01 started by C59, each other by the one before. */
const RING: readonly Forage[] = Array.from({ length: 60 }, (_, at) =>
  at === 0
    ? forage(named(at))
    : forage(named(at), { startedBy: named(at === 1 ? 59 : at - 1), depth: 1 }),
);

it("fills the floor from a ring longer than the room it leaves, cut at its top", () => {
  // No whole line of the ring fits beside C00, so C01's is cut to the 47 places left.
  assert.deepEqual(compartments(floorAt(RING, "C00")), ["C00", "C01", ...span(14, 59)]);
});

it("stops a line at a cell already drawn, so a long line leaves room for the cells it started", () => {
  // C40's line is 41 cells; the cell it started, C41, needs only itself.
  assert.deepEqual(compartments(floorAt(CHAIN, "C40")), span(0, 47));
});

it("draws the starts the node's book holds before those only claimed", () => {
  // Fifty cells claim C00 started them before the one its book holds, C51.
  const crowded = LONG.map((each, at) => {
    if (at === 51) {
      return { ...each, startedBy: "C00", booked: true, depth: 1 };
    }
    return at > 0 && at <= 50 ? { ...each, startedBy: "C00", depth: 1 } : each;
  });
  const floor = floorAt(crowded, "C00");
  assert.ok(floor.includes('<path class="hypha" data-cell="C51"'), floor);
});

it("draws a cell split from another with it, as one it started", () => {
  const split = LONG.map((each) =>
    each.cell === "C57" ? { ...each, splitFrom: "C03", depth: 1 } : each,
  );
  const floor = floorAt(split, "C57");
  assert.ok(compartments(floor).includes("C03"), floor);
  assert.ok(floor.includes('data-cell="C57" d='), floor);
  // C03 is drawn, but the cell split from it isn't.
  assert.equal(stepsOff(floorFor(split, null), "C03"), true);
});

it("draws the floor again only for a step to a cell it leaves off, or one that grew a cell it does", () => {
  const first = floorFor(LONG, null);
  assert.equal(stepsOff(first, "C55"), true);
  assert.equal(stepsOff(first, "C47"), false);
  assert.equal(stepsOff(first, null), false);
  // C10 is drawn, but the cell it booked, C55, isn't.
  assert.equal(stepsOff(first, "C10"), true);
  // Kept around C55, the floor draws C10 and C13 to C59.
  const kept = floorFor(LONG, "C55");
  assert.equal(stepsOff(kept, "C10"), false);
  assert.equal(stepsOff(kept, "C30"), false);
  assert.equal(stepsOff(kept, "C12"), true);
  // With no cursor, the floor stays.
  assert.equal(stepsOff(kept, null), false);
  // A floor drawing the whole list never redraws.
  assert.equal(stepsOff(floorFor(LONG.slice(0, 5), null), "C04"), false);
});

it("keeps the floor drawn on a step it holds, and draws another for a step it leaves off", () => {
  const kept = floorFor(LONG, "C55");
  assert.equal(floorStepped(kept, "C30"), kept);
  const off = floorStepped(kept, "C12");
  assert.equal(off.around, "C12");
  // It draws the new floor over the list the old one was worked out for.
  assert.equal(off.of, LONG);
  // With no cells, there's no floor to step off.
  assert.equal(floorStepped(UNDRAWN, "C12"), UNDRAWN);
  const emptied = floorAfter([], kept, null);
  assert.equal(floorStepped(emptied, "C12"), emptied);
});

it("keeps the floor for a cursor it holds or none, and moves it for one off it", () => {
  const kept = floorFor(LONG, "C55");
  // A find that matches nothing keeps the cursor, on C30 here, and the floor with it.
  const typo = floorAfter(LONG, kept, "C30");
  assert.equal(typo.around, "C55");
  assert.deepEqual(typo.drawn, kept.drawn);
  // Once C30 ends, the page holds no cursor, and the floor stays where it was.
  const ended = LONG.filter((each) => each.cell !== "C30");
  const none = floorAfter(ended, kept, null);
  assert.equal(none.around, "C55");
  // A find that hides the cursor moves it to its first match; C00 is off the floor.
  assert.equal(floorAfter(LONG, kept, "C00").around, "C00");
  // A cursor off the floor moves it from a floor kept with no cursor too.
  assert.equal(floorAfter(ended, none, "C00").around, "C00");
  // With no cursor, a floor whose own cell ends goes back to the list's first cells.
  const gone = LONG.filter((each) => each.cell !== "C55");
  const back = floorAfter(gone, kept, null);
  assert.equal(back.around, null);
  assert.deepEqual(
    back.drawn.map((each) => each.cell),
    gone.slice(0, 48).map((each) => each.cell),
  );
  // With no cursor, its cell coming back doesn't pull the floor back to it.
  assert.deepEqual(
    floorAfter(LONG, back, null).drawn.map((each) => each.cell),
    LONG.slice(0, 48).map((each) => each.cell),
  );
});

it("keeps the floor still as the page steps and records arrive, until a step leaves it", () => {
  // The page opens on C55, steps to C30 on the floor, hears a record, then steps off to C12.
  let shown = floorAfter(LONG, UNDRAWN, "C55");
  assert.equal(shown.around, "C55");
  const opened = shown;
  shown = floorStepped(shown, "C30");
  assert.equal(shown, opened);
  const recorded = startedOn(LONG, "C59", "C58");
  shown = floorAfter(recorded, shown, "C30");
  assert.equal(shown.around, "C55");
  assert.deepEqual(
    shown.drawn.map((each) => each.cell),
    opened.drawn.map((each) => each.cell),
  );
  shown = floorStepped(shown, "C12");
  assert.equal(shown.around, "C12");
  assert.equal(shown.of, recorded);
});

it("draws the floor around the cursor once a list that fit grows past it", () => {
  // The page opens on forty cells at C00, steps to C39, then records bring twenty more.
  let shown = floorAfter(LONG.slice(0, 40), UNDRAWN, "C00");
  shown = floorStepped(shown, "C39");
  assert.equal(shown.around, "C00");
  shown = floorAfter(LONG, shown, "C39");
  assert.equal(shown.around, "C39");
  assert.equal(shown.of, LONG);
  // Forty-eight cells still fit, so a floor over them keeps no cell either.
  shown = floorStepped(floorAfter(LONG.slice(0, 48), UNDRAWN, "C00"), "C40");
  assert.equal(floorAfter(LONG.slice(0, 49), shown, "C40").around, "C40");
  // Forty-nine don't, so a floor over them keeps its cell.
  const kept = floorFor(LONG.slice(0, 49), "C00");
  assert.equal(floorAfter(LONG.slice(0, 49), kept, "C20").around, "C00");
  // Records end twelve cells of a floor kept around C55, then bring them back.
  const shrunk = floorAfter(
    LONG.filter((_, at) => at < 36 || at >= 48),
    floorFor(LONG, "C55"),
    "C30",
  );
  assert.equal(shrunk.around, "C55");
  assert.equal(floorAfter(LONG, shrunk, "C30").around, "C30");
});

it("keeps the floor on its cell when a record adds one near it, shifting the cells beside it", () => {
  // A record brings C40X in after C40, inside the floor kept around C55.
  const inserted = LONG.flatMap((each) => (each.cell === "C40" ? [each, forage("C40X")] : [each]));
  const shown = floorAfter(inserted, floorFor(LONG, "C55"), "C30");
  assert.equal(shown.around, "C55");
  const drawn = shown.drawn.map((each) => each.cell);
  assert.deepEqual(drawn, ["C10", ...span(14, 40), "C40X", ...span(41, 59)]);
});

it("keeps the floor where it is while it holds the cursor, else draws it around the cursor", () => {
  assert.equal(floorAfter(LONG, floorFor(LONG, "C55"), "C30").around, "C55");
  assert.equal(floorAfter(LONG, UNDRAWN, "C02").around, "C02");
  assert.equal(floorAfter(LONG, floorFor(LONG, "C99"), "C02").around, "C02");
  // A record puts thirty cells between the kept cell and the cursor.
  const newcomers = Array.from({ length: 30 }, (_, at) =>
    forage(`N${String(at).padStart(2, "0")}`),
  );
  const pushed = [...LONG.slice(0, 31), ...newcomers, ...LONG.slice(31)];
  assert.equal(floorAfter(pushed, floorFor(LONG, "C55"), "C30").around, "C30");
  // A record grows a cell from the cursor that the kept floor leaves off.
  const grown = startedOn(LONG, "C05", "C20");
  assert.equal(floorAfter(grown, floorFor(LONG, "C55"), "C20").around, "C20");
});

it("draws the widest, deepest floor in the room the page holds before the node speaks", () => {
  const startedAt = (depth: number) =>
    LONG.map((each, at) => (at === 1 ? { ...each, startedBy: named(0), depth } : each));
  const drawnAt = (depth: number) =>
    floorOf(floorFor(startedAt(depth), named(0)), named(0), "kestrel");
  const floor = drawnAt(9);
  // A floor draws no deeper than its steps, and is as tall however deep it goes.
  assert.equal(floor, drawnAt(3), "a floor drew deeper than its steps");
  assert.equal(
    floor.match(VIEWED)?.[2],
    floorOf(floorFor(LONG, named(0)), named(0), "kestrel").match(VIEWED)?.[2],
    "a flat floor and a deep one were drawn at different heights",
  );
  const [, width = "", height = ""] = floor.match(VIEWED) ?? [];
  const sheet = readFileSync(new URL("../site.css", import.meta.url), "utf8");
  // The room is this floor's shape at most half again as large, never under 6rem, so a shift
  // can't come of it and a floor of few cells on a phone still reads.
  const room = `clamp(6rem, 100cqi * ${height} / ${width}, ${(Number(height) * 3) / 2}px)`;
  assert.ok(sheet.includes(room), `site.css holds no room of ${room}`);
  // The words go where this floor would draw its smallest under the kit's floor for text.
  const words = `@container (max-width: ${Math.ceil((Number(width) * LEAST_PX) / WORD_PX)}px)`;
  assert.ok(sheet.includes(words), `site.css holds no ${words}`);
});

it("draws the cursor after a redraw around any kept cell, keeping it there while it can", () => {
  for (const { cell: kept } of GROWN.filter((_, at) => at % 9 === 0)) {
    const there = floorFor(GROWN, kept);
    for (const { cell: cursor } of GROWN) {
      const floor = floorAfter(GROWN, there, cursor);
      const drawn = compartments(floorOf(floor, cursor, "kestrel"));
      assert.ok(drawn.includes(cursor), `${kept}, ${cursor}: ${drawn.join(" ")}`);
      // Kept around another cell, it draws every cell the cursor started.
      const started = GROWN.filter((each) => each.startedBy === cursor);
      assert.ok(
        floor.around === cursor || started.every((each) => drawn.includes(each.cell)),
        `${kept}, ${cursor}: ${drawn.join(" ")}`,
      );
      // Kept there, it holds still.
      assert.ok(!there.holds.has(cursor) || floor.around === kept, `${kept}, ${cursor}`);
    }
  }
});
