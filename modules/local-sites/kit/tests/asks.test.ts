// The asks a page draws from the to-do feed: only looks waiting on him, the
// newest first, a cell's own by its address; every word a session wrote set
// as text, never as markup; answers offered only while the feed is live and
// until one is pressed; and what the server said of a press, said in the
// row in plain words, with nothing posted for a look that no longer asks.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import type { Posted } from "../answer.ts";
import { carry, ready } from "../answer.ts";
import { asked } from "../ask.ts";
import { drawAsk, followAsks, waiting } from "../asks.ts";
import { asksOnceOneWaits } from "../waits.ts";
import { Held } from "../records.ts";
import { Hands } from "./hands.ts";
import { StandIn } from "./stand-in.ts";

/**
 * An element as a row is built from it, which keeps whatever text it's given
 * as text and throws on every way of writing markup into it, so a row drawn
 * whole was drawn from text alone.
 */
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

  set outerHTML(_markup: string) {
    throw new Error("a row was given markup");
  }

  insertAdjacentHTML(_where: string, _markup: string): void {
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

// What a row says when an answer can't be carried, and once one is.
const STALE = /reload it to answer/u;
const TAKES_NONE = /take no answers: answer in For You/u;
const UNREACHED = /can't be reached/u;
const AGENTS = /one of your agents/u;
const FOR_YOU = /answer in For You/u;
const UNTOLD = /couldn't tell this page is yours/u;
const NOTHING_SENT = /nothing was sent/u;

/** Waits out every task already queued, however many turns it takes. */
const flush = (): Promise<void> => new Promise((then) => setImmediate(then));
const COME_BACK = /come back once this page is live/u;
const ANSWERED = /Answered Merge It/u;

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

/** `looks` as the feed's records, keyed as the to-do daemon keys them. */
function recordsOf(looks: object[]): object[] {
  return looks.map((one) => ({
    key: `look:${(one as { id: string }).id}`,
    kind: "look",
    look: one,
  }));
}

/** The feed's records holding `looks`. */
function holding(...looks: object[]): Held {
  const held = new Held();
  held.took({ records: recordsOf(looks) });
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
  assert.match(row.words(), /<img src=x onerror=alert\(1\)> Its checks passed\./u);
  assert.equal(
    row.all("a")[0]?.getAttribute("href"),
    "https://github.com/Jitsusama/core.nix/pull/1",
  );
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
  assert.deepEqual(carried, {
    carried: true,
    words: "Answered Merge It: sent to the session that asked.",
  });
  assert.deepEqual(JSON.parse(posted[0] ?? ""), { look: "l1", value: "yes" });
  const stale = await carry(look("l1", "abc"), choice, NOW, answering(403, '{"refused":"cookie"}'));
  assert.match(stale.words, STALE);
  const none = await carry(
    look("l1", "abc"),
    choice,
    NOW,
    answering(404, "this server takes no answers\n"),
  );
  assert.match(none.words, TAKES_NONE);
  const down = await carry(look("l1", "abc"), choice, NOW, () => Promise.reject(new Error("down")));
  assert.match(down.words, UNREACHED);
  const agents = await carry(look("l1", "abc"), choice, NOW, answering(403, '{"refused":"agent"}'));
  assert.match(agents.words, AGENTS);
  const form = await carry(look("l1", "abc"), choice, NOW, answering(415, '{"refused":"type"}'));
  assert.match(form.words, FOR_YOU);
});

/** A server that takes the page's answers. */
const takes = (): Promise<Posted> => Promise.resolve({ status: 204, text: "" });

it("a page asks once whether its answers are taken, and is told why not", async () => {
  assert.equal(await ready(takes), null);
  const none = await ready(() =>
    Promise.resolve({ status: 404, text: "this server takes no answers\n" }),
  );
  assert.match(String(none), TAKES_NONE);
  const agents = await ready(() => Promise.resolve({ status: 403, text: '{"refused":"agent"}' }));
  assert.match(String(agents), AGENTS);
  const untold = await ready(() => Promise.resolve({ status: 403, text: '{"refused":"untold"}' }));
  assert.match(String(untold), UNTOLD);
  const down = await ready(() => Promise.reject(new Error("down")));
  assert.match(String(down), UNREACHED);
  const elsewhere = await ready(() => Promise.resolve({ status: 403, text: '{"refused":"site"}' }));
  for (const said of [none, agents, untold, down, elsewhere]) {
    assert.doesNotMatch(String(said), NOTHING_SENT, "a row nobody pressed said nothing was sent");
  }
});

it("answers wait on the server taking them, and say why where it doesn't", async () => {
  for (const [said, shown] of [
    [{ status: 204, text: "" }, 2],
    [{ status: 403, text: '{"refused":"agent"}' }, 0],
  ] as const) {
    let answer: (posted: Posted) => void = () => undefined;
    const sockets: StandIn[] = [];
    const holder = new Element("div");
    const waits = () =>
      new Promise<Posted>((then) => {
        answer = then;
      });
    followAsks({ holder, make, reaching: { ...reaching(sockets), ready: waits } });
    telling(sockets[0], look("l1", "abc"));
    assert.equal(
      holder.all("button").length,
      0,
      "answers showed before the server said it takes them",
    );
    answer(said);
    await flush();
    assert.equal(holder.all("button").length, shown, `${said.status}`);
    assert.equal(
      AGENTS.test(holder.words()),
      shown === 0,
      `${said.status}: whether the row named the agents`,
    );
  }
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
      ready: takes,
    },
  });
  // The server says it takes the page's answers before the feed goes live.
  await flush();
  const feed = socket as unknown as StandIn;
  feed.onopen?.();
  feed.tell({ id: 2, records: [{ key: "look:l1", kind: "look", look: look("l1", "abc") }] });
  assert.equal(holder.all("button").length, 0, "answers showed before the feed was live");
  assert.match(holder.words(), COME_BACK);
  feed.tell({ id: 2, state: "live" });
  const [yes] = holder.all("button");
  assert.ok(yes !== undefined, "no answers showed while live");
  yes.clicks[0]?.();
  assert.equal(holder.all("button").length, 0, "answers stayed once one was pressed");
  await flush();
  assert.deepEqual(
    posted.map((body) => JSON.parse(body)),
    [{ look: "l1", value: "yes" }],
  );
  assert.match(holder.words(), ANSWERED);
  assert.deepEqual(counts.slice(-1), [1]);
});

/** The sockets a page opened, each a stand-in, in the order it opened them. */
function reaching(sockets: StandIn[]): {
  page: string;
  clock: Hands;
  connect: (url: string) => StandIn;
  ready: () => Promise<Posted>;
} {
  const hands = new Hands();
  hands.at = NOW;
  return {
    page: "http://sites.localhost:8484/",
    clock: hands,
    ready: takes,
    connect: (url) => {
      const socket = new StandIn(url);
      sockets.push(socket);
      return socket;
    },
  };
}

/** `socket` opened, told `looks` whole, then live. */
function telling(socket: StandIn | undefined, ...looks: object[]): void {
  socket?.onopen?.();
  socket?.tell({ id: 2, records: recordsOf(looks) });
  socket?.tell({ id: 2, state: "live" });
}

it("a page narrowed to a cell shows only its asks, and every one widened again", () => {
  const sockets: StandIn[] = [];
  const holder = new Element("div");
  const asks = followAsks({ holder, make, address: "abc", reaching: reaching(sockets) });
  telling(sockets[0], look("mine", "abc"), look("theirs", "def"));
  assert.equal(holder.all("article").length, 1, "a cell was shown another's asks");
  asks.narrow(null);
  assert.equal(holder.all("article").length, 2, "widened, the page didn't show every ask");
  asks.narrow("def");
  assert.deepEqual(holder.all("h3").length, 1);
});

it("a page asks again whether its answers are taken once its feed comes back", async () => {
  const sockets: StandIn[] = [];
  const holder = new Element("div");
  const near = reaching(sockets);
  let asked = 0;
  const asking = (): Promise<Posted> => {
    asked += 1;
    return takes();
  };
  followAsks({ holder, make, reaching: { ...near, ready: asking } });
  telling(sockets[0], look("l1", "abc"));
  await flush();
  assert.equal(asked, 1);
  sockets[0]?.onclose?.();
  near.clock.pass(60_000);
  telling(sockets[1], look("l1", "abc"));
  assert.equal(asked, 2, "a server that may have restarted wasn't asked again");
  assert.equal(holder.all("button").length, 0, "answers showed before it said again");
  await flush();
  assert.equal(holder.all("button").length, 2, "answers didn't come back once it said");
});

/** Loads the rows' files as the browser does, counting each load. */
function counted(): { loads: number; load: () => Promise<typeof import("../asks.ts")> } {
  const loading = {
    loads: 0,
    load: () => {
      loading.loads += 1;
      return import("../asks.ts");
    },
  };
  return loading;
}

it("rows whose files failed to load are loaded again once the feed is watched again", async () => {
  const sockets: StandIn[] = [];
  const holder = new Element("div");
  const near = reaching(sockets);
  const loading = counted();
  let fails = true;
  const load = (): Promise<typeof import("../asks.ts")> => {
    if (fails) {
      fails = false;
      loading.loads += 1;
      return Promise.reject(new Error("the server restarted"));
    }
    return loading.load();
  };
  asksOnceOneWaits({ holder, make, reaching: near }, load);
  telling(sockets[0], look("l1", "abc"));
  await flush();
  assert.equal(loading.loads, 1);
  assert.equal(sockets.length, 1, "the feed was watched again at once");
  near.clock.pass(60_000);
  telling(sockets[1], look("l1", "abc"));
  assert.equal(loading.loads, 2, "rows whose files failed to load were never loaded again");
});

it("the rows load only once a look waits, narrowed as the page narrowed them meanwhile", async () => {
  const sockets: StandIn[] = [];
  const holder = new Element("div");
  const loading = counted();
  const asks = asksOnceOneWaits({ holder, make, reaching: reaching(sockets) }, loading.load);
  telling(sockets[0], { ...look("done", "abc"), state: "resolved" });
  await flush();
  assert.equal(loading.loads, 0, "the rows loaded with nothing waiting");
  assert.equal(sockets.length, 1, "a second watch opened with nothing waiting");
  assert.equal(sockets[0]?.closed, false, "the watch closed with nothing waiting");
  asks.narrow("def");
  sockets[0]?.tell({ id: 2, changed: [{ key: "look:l2", kind: "look", look: look("l2", "abc") }] });
  assert.equal(sockets[0]?.closed, true, "the watch for a waiting look stayed open");
  assert.equal(loading.loads, 1);
  for (let turns = 0; turns < 200 && sockets.length < 2; turns += 1) {
    await new Promise((then) => setTimeout(then, 5));
  }
  assert.equal(sockets.length, 2, "the rows' files never loaded, so they never watched");
  telling(sockets[1], look("l2", "abc"), look("l3", "def"));
  assert.deepEqual(
    holder.all("h3").length,
    1,
    "the rows forgot the cell the page narrowed to before they loaded",
  );
});
