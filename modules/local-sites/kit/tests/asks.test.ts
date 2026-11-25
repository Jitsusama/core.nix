// The asks a page draws from the to-do feed: only looks waiting on him, the
// newest first, a cell's own by its address; every word a session wrote set
// as text, never as markup; answers offered only while the feed is live and
// until one is pressed; and what the server said of a press, said in the
// row in plain words, with nothing posted for a look that no longer asks.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import type { Posted } from "../answer.ts";
import { carry } from "../answer.ts";
import { asked } from "../ask.ts";
import { drawAsk, followAsks, waiting } from "../asks.ts";
import { Held } from "../records.ts";
import { Hands } from "./hands.ts";
import { StandIn } from "./stand-in.ts";

/** An element as a row is built from it, refusing markup as a test of text. */
class Element {
  readonly tag: string;
  className = "";
  textContent: string | null = null;
  readonly children: Element[] = [];
  readonly attributes = new Map<string, string>();
  readonly clicks: (() => void)[] = [];

  constructor(tag: string) {
    this.tag = tag;
  }

  get childNodes(): readonly Element[] {
    return this.children;
  }

  set innerHTML(_markup: string) {
    throw new Error("a row was given markup");
  }

  getAttribute(name: string): string | null {
    return this.attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.attributes.set(name, value);
  }

  append(...nodes: Element[]): void {
    this.children.push(...nodes);
  }

  addEventListener(_type: "click", listener: () => void): void {
    this.clicks.push(listener);
  }

  insertBefore(node: Element, before: Element | null): Element {
    this.removeChild(node);
    const at = before === null ? -1 : this.children.indexOf(before);
    this.children.splice(at < 0 ? this.children.length : at, 0, node);
    return node;
  }

  removeChild(node: object): void {
    const at = this.children.indexOf(node as Element);
    if (at >= 0) {
      this.children.splice(at, 1);
    }
  }

  /** Every word in it, as a browser's textContent reads them. */
  words(): string {
    return (this.textContent ?? "") + this.children.map((child) => child.words()).join("");
  }

  /** Every element in it of `kind`, a class or a tag. */
  all(kind: string): readonly Element[] {
    const found = this.className === kind || this.tag === kind ? [this] : [];
    return [...found, ...this.children.flatMap((child) => child.all(kind))];
  }
}

const make = (tag: string): Element => new Element(tag);

/** When the tests read each look. */
const NOW = Date.parse("2026-11-24T23:00:00Z");
const HOUR = 3_600_000;

/** A look waiting on him, left by the cell at `address` `ago` before now. */
function look(id: string, address: string, ago = HOUR): object {
  return {
    v: 1,
    id,
    at: new Date(NOW - ago).toISOString(),
    kind: "pr",
    target: "https://github.com/Jitsusama/core.nix/pull/1",
    title: "<b>Merge</b> it?",
    why: "<img src=x onerror=alert(1)> Its checks passed.",
    ask: "Say whether it merges.",
    choices: [
      { value: "yes", label: "Merge It" },
      { value: "no", label: "Leave It" },
    ],
    state: "waiting",
    address,
  };
}

/** The feed's records holding `looks`, keyed as the to-do daemon keys them. */
function holding(...looks: object[]): Held {
  const held = new Held();
  const records = looks.map((one) => ({ key: `look:${(one as { id: string }).id}`, kind: "look", look: one }));
  held.took({ records });
  return held;
}

it("only looks waiting on him show, the newest first, and a cell's own by its address", () => {
  const ended = { ...look("ended", "abc"), state: "resolved" };
  const held = holding(look("old", "abc", 2 * HOUR), look("new", "def"), ended, { v: 2 });
  assert.deepEqual(
    waiting(held, NOW).map((row) => row.ask.id),
    ["new", "old"],
  );
  assert.deepEqual(
    waiting(held, NOW, "ABC").map((row) => row.ask.id),
    ["old"],
    "a cell was shown another's asks",
  );
});

it("every word a session wrote is set as text, so markup in a title shows as written", () => {
  const ask = asked(look("l1", "abc"), NOW);
  assert.ok(ask !== null);
  const view = { offering: true, told: "", press: () => undefined };
  const row = drawAsk(make, { key: "look:l1", look: null, ask }, view, NOW);
  const [title] = row.all("h3");
  assert.equal(title?.words(), "<b>Merge</b> it?");
  assert.equal(title?.all("b").length, 0, "a title's markup became an element");
  assert.equal(row.all("img").length, 0, "a why's markup became an element");
  assert.equal(row.all("a")[0]?.getAttribute("href"), "https://github.com/Jitsusama/core.nix/pull/1");
  assert.deepEqual(
    row.all("button").map((button) => button.words()),
    ["Merge It", "Leave It"],
  );
  assert.equal(row.all("ask-told")[0]?.getAttribute("aria-live"), "polite");
});

it("nothing is posted for a value the look doesn't offer or a look that has ended", async () => {
  const posted: string[] = [];
  const send = (body: string): Promise<Posted> => {
    posted.push(body);
    return Promise.resolve({ status: 200, text: "{}" });
  };
  const offered = { value: "maybe", label: "Maybe", follows: "" };
  const told = await carry(look("l1", "abc"), offered, NOW, send);
  assert.equal(told.carried, false);
  const ended = { ...look("l1", "abc"), state: "cleared" };
  await carry(ended, { value: "yes", label: "Merge It", follows: "" }, NOW, send);
  assert.deepEqual(posted, [], "an answer no longer asked was posted");
});

it("a press posts the look and the value alone, and a refusal is said with what to do", async () => {
  const choice = { value: "yes", label: "Merge It", follows: "" };
  const posted: string[] = [];
  const answering = (status: number, text: string) => (body: string): Promise<Posted> => {
    posted.push(body);
    return Promise.resolve({ status, text });
  };
  const carried = await carry(look("l1", "abc"), choice, NOW, answering(200, "{}"));
  assert.deepEqual(carried, { carried: true, words: "Answered Merge It: sent to the session that asked." });
  assert.deepEqual(JSON.parse(posted[0] ?? ""), { look: "l1", value: "yes" });
  const stale = await carry(look("l1", "abc"), choice, NOW, answering(403, '{"refused":"cookie"}'));
  assert.match(stale.words, /reload it to answer/u);
  const none = await carry(look("l1", "abc"), choice, NOW, answering(404, "this server takes no answers\n"));
  assert.match(none.words, /take no answers: answer in For You/u);
  const down = await carry(look("l1", "abc"), choice, NOW, () => Promise.reject(new Error("down")));
  assert.match(down.words, /can't be reached/u);
});

it("answers show only while the feed is live and go once one is pressed", async () => {
  const hands = new Hands();
  hands.at = NOW;
  let socket: StandIn | null = null;
  const posted: string[] = [];
  const holder = new Element("div");
  const counts: number[] = [];
  followAsks({
    holder,
    make,
    counted: (count) => counts.push(count),
    reaching: {
      page: "http://sites.localhost:8484/",
      clock: hands,
      connect: (url) => {
        socket = new StandIn(url);
        return socket;
      },
      send: (body) => {
        posted.push(body);
        return Promise.resolve({ status: 200, text: "{}" });
      },
    },
  });
  const feed = socket as unknown as StandIn;
  feed.onopen?.();
  feed.tell({ id: 2, records: [{ key: "look:l1", kind: "look", look: look("l1", "abc") }] });
  assert.equal(holder.all("button").length, 0, "answers showed before the feed was live");
  assert.match(holder.words(), /come back once this page is live/u);
  feed.tell({ id: 2, state: "live" });
  const [yes] = holder.all("button");
  assert.ok(yes !== undefined, "no answers showed while live");
  yes.clicks[0]?.();
  assert.equal(holder.all("button").length, 0, "answers stayed once one was pressed");
  await Promise.resolve();
  await Promise.resolve();
  assert.deepEqual(posted.map((body) => JSON.parse(body)), [{ look: "l1", value: "yes" }]);
  assert.match(holder.words(), /Answered Merge It/u);
  assert.deepEqual(counts.slice(-1), [1]);
});
