// When Code asks the sites again where its cells are home, and how it reads the rows and each
// site's record.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { askerOf, type Fetching, readRows, readSite } from "../asking.ts";
import type { Asked, Homes } from "../homes.ts";

/** An asker whose every ask, telling and wait the test holds, to answer or run as it likes. */
function held() {
  const asks: ((answer: Asked | Error) => void)[] = [];
  const told: Homes[] = [];
  const waits: { milliseconds: number; then: () => void }[] = [];
  const seen = askerOf({
    ask: () =>
      new Promise((resolve, reject) => {
        asks.push((said) => (said instanceof Error ? reject(said) : resolve(said)));
      }),
    told: (homes) => told.push(homes),
    after: (milliseconds, then) => waits.push({ milliseconds, then }),
  });
  /** Answers the newest ask with `said`, once every promise it moves has settled. */
  const answer = async (said: Asked | Error): Promise<void> => {
    asks.at(-1)?.(said);
    await new Promise((settled) => setImmediate(settled));
  };
  return { answer, asks, seen, told, waits };
}

/** Homes as `addresses` says them, whole unless said otherwise. */
function homing(addresses: Record<string, string>, whole = true): Asked {
  return { homes: (name) => addresses[name], whole };
}

it("asks at once, tells where cells are home, and asks no more while each has one", async () => {
  const { answer, asks, seen, told, waits } = held();
  assert.equal(asks.length, 1);
  await answer(homing({ A: "m/#c=A" }));
  assert.equal(told.at(-1)?.("A"), "m/#c=A");
  seen(["A"]);
  assert.equal(asks.length, 1);
  assert.deepEqual(waits, []);
});

it("asks at once for a cell first seen with no home, not one asked about", async () => {
  const { answer, asks, seen } = held();
  await answer(homing({}));
  seen(["B"]);
  assert.equal(asks.length, 2);
  await answer(homing({}));
  seen(["B"]);
  assert.equal(asks.length, 2);
});

it("asks about a cell that started while an ask was out as soon as it ends", async () => {
  const { answer, asks, seen } = held();
  seen(["B"]);
  assert.equal(asks.length, 1);
  await answer(homing({}));
  assert.equal(asks.length, 2);
});

it("asks again while a cell has no home, twice as long each time up to a minute", async () => {
  const { answer, asks, seen, told, waits } = held();
  await answer(homing({}));
  seen(["B"]);
  for (let each = 0; each < 7; each += 1) {
    await answer(homing({}));
    waits.at(-1)?.then();
  }
  const waited = waits.map((each) => each.milliseconds);
  assert.deepEqual(waited, [2000, 4000, 8000, 16_000, 32_000, 60_000, 60_000]);
  await answer(homing({ B: "m/#c=B" }));
  assert.equal(told.at(-1)?.("B"), "m/#c=B");
  assert.equal(waits.length, 7, "it waited to ask again once every cell had a home");
  assert.equal(asks.length, 9);
});

it("keeps the links a record gave when it couldn't be read, and asks again", async () => {
  const { answer, seen, told, waits } = held();
  await answer(homing({ A: "m/#c=A" }));
  seen(["A", "B"]);
  await answer(homing({ B: "m/#c=B" }, false));
  assert.equal(told.at(-1)?.("A"), "m/#c=A");
  assert.equal(told.at(-1)?.("B"), "m/#c=B");
  assert.equal(waits.length, 1);
});

it("keeps every link when an ask fails, and asks again once it has waited", async () => {
  const { answer, asks, seen, told, waits } = held();
  await answer(homing({ A: "m/#c=A" }));
  seen(["A", "B"]);
  await answer(new Error("the sites' server is restarting"));
  assert.equal(told.length, 1);
  assert.equal(waits.length, 1);
  waits[0]?.then();
  assert.equal(asks.length, 3);
  await answer(homing({ A: "m/#c=A", B: "m/#c=B" }));
  assert.equal(told.at(-1)?.("B"), "m/#c=B");
  assert.equal(waits.length, 1);
});

/** A fetch answering each address from `answers` by its status and body, keeping each asked. */
function serving(answers: Record<string, [number, unknown]>): Fetching & { asked: string[] } {
  const asked: string[] = [];
  const fetching = (address: string): Promise<Response> => {
    asked.push(address);
    const [status, body] = answers[address] ?? [404, undefined];
    return Promise.resolve(new Response(JSON.stringify(body ?? null), { status }));
  };
  return Object.assign(fetching, { asked });
}

it("reads the rows the home lists, and the record only when there's no rows file", async () => {
  const listed = { sites: [{ path: "rows/" }] };
  const kept = { sites: [{ path: "record/" }] };
  const both = serving({ "../rows.json": [200, listed], "../sites.json": [200, kept] });
  assert.deepEqual(await readRows(both), listed);
  assert.deepEqual(await readRows(serving({ "../sites.json": [200, kept] })), kept);
  const failing = serving({ "../rows.json": [500, undefined], "../sites.json": [200, kept] });
  await assert.rejects(readRows(failing));
  assert.deepEqual(failing.asked, ["../rows.json"]);
});

it("reads a site's record, nothing where it keeps none, failing where unread", async () => {
  const record = { things: [] };
  assert.deepEqual(await readSite(serving({ "../m/homes.json": [200, record] }), "m"), record);
  assert.equal(await readSite(serving({}), "m/"), undefined);
  await assert.rejects(readSite(serving({ "../m/homes.json": [503, undefined] }), "m/"));
});
