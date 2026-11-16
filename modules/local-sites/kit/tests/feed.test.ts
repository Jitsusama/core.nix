// A watch followed over a stand-in socket and clock: what it asks, how it
// says it stands, what it holds, and how it comes back after the server goes.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { feedAddress, follow, type Hearing, type Standing, waitBefore } from "../feed.ts";
import type { Moved } from "../records.ts";
import { Hands } from "./hands.ts";
import { StandIn } from "./stand-in.ts";

/** A watch of `node`'s cells, followed over stand-ins. */
function followed() {
  const sockets: StandIn[] = [];
  const clock = new Hands();
  const stood: Standing[] = [];
  const moves: Moved[] = [];
  const problems: string[] = [];
  const hearing: Hearing = {
    moved: (_held, moved) => moves.push(moved),
    stands: (standing) => stood.push(standing),
    problem: (what) => problems.push(what),
  };
  const following = follow({ feed: "node", kind: "cell", fields: ["title"] }, hearing, {
    clock,
    page: "http://sites.localhost:8484/mycelium/",
    connect: (url) => {
      const socket = new StandIn(url);
      sockets.push(socket);
      return socket;
    },
  });
  const socket = (): StandIn => {
    const last = sockets.at(-1);
    assert.ok(last, "the watch opened a socket");
    return last;
  };
  return { following, sockets, socket, clock, stood, moves, problems };
}

it("a feed is reached on the page's own server", () => {
  assert.equal(
    feedAddress("http://sites.localhost:8484/mycelium/#cell=A", "node"),
    "ws://sites.localhost:8484/feed/node",
  );
  assert.equal(feedAddress("https://x.localhost/a/", "net"), "wss://x.localhost/feed/net");
});

it("it says hello, then asks its watch narrowed as the page asked", () => {
  const { socket } = followed();
  socket().onopen?.();
  assert.deepEqual(socket().sent, [
    { id: 1, ask: "hello" },
    { id: 2, ask: "watch", kind: "cell", fields: ["title"] },
  ]);
});

it("it says connecting, then live, and holds what it's told", () => {
  const { following, socket, stood, moves } = followed();
  socket().onopen?.();
  socket().tell({ id: 1, version: 1, domain: "local-sites" });
  socket().tell({ id: 2, state: "connecting" });
  socket().tell({ id: 2, state: "live" });
  socket().tell({ id: 2, records: [{ key: "cell:A", title: "A" }] });
  socket().tell({ id: 2, changed: [{ key: "cell:A", title: "Again" }] });
  assert.deepEqual(
    stood.map((each) => each.state),
    ["connecting", "live"],
  );
  assert.equal(moves.length, 2);
  assert.equal(following.held.get("cell:A")?.title, "Again");
});

it("a broken daemon is said with the server's reason and time", () => {
  const { socket, stood } = followed();
  socket().tell({ id: 2, state: "broken", reason: "nobody.sock isn't there", since: 42 });
  assert.deepEqual(stood.at(-1), { state: "broken", since: 42, reason: "nobody.sock isn't there" });
});

it("a broken daemon the server gives no words or time for is still said", () => {
  const { socket, clock, stood } = followed();
  socket().tell({ id: 2, state: "broken", reason: 7, since: "then" });
  assert.deepEqual(
    stood.at(-1),
    { state: "broken", since: clock.now(), reason: "its daemon didn't answer" },
    "a reason or time not in its shape gives way to the page's own",
  );
});

it("a refusal without a message is said in the page's words", () => {
  const { socket, stood } = followed();
  socket().tell({ id: 2, error: { code: "refused" } });
  assert.equal(stood.at(-1)?.reason, "the feed refused the watch", "the page's own words");
});

it("a watch narrowed by keys asks for those keys", () => {
  const sockets: StandIn[] = [];
  const ignored = (): void => undefined;
  follow(
    { feed: "node", kind: "cell", keys: ["cell:A"] },
    { moved: ignored, stands: ignored, problem: ignored },
    {
      clock: new Hands(),
      page: "http://sites.localhost:8484/mycelium/",
      connect: (url) => {
        const socket = new StandIn(url);
        sockets.push(socket);
        return socket;
      },
    },
  );
  sockets[0]?.onopen?.();
  assert.deepEqual(
    sockets[0]?.sent.at(-1),
    { id: 2, ask: "watch", kind: "cell", keys: ["cell:A"] },
    "the keys, and no fields",
  );
});

it("a refused watch is said broken with the refusal", () => {
  const { socket, stood } = followed();
  socket().tell({ id: 2, error: { code: "refused", message: "no kind called cell" } });
  assert.equal(stood.at(-1)?.state, "broken");
  assert.equal(stood.at(-1)?.reason, "no kind called cell");
});

it("what can't be read is told as a problem, never dropped", () => {
  const { socket, problems } = followed();
  socket().tell("not json");
  socket().tell("[1]");
  socket().tell({ id: 9 });
  socket().tell({ id: 2, records: [{ title: "keyless" }] });
  assert.equal(problems.length, 4);
});

it("a server gone is said stale, kept, and tried again sooner first", () => {
  const { following, sockets, socket, clock, stood } = followed();
  socket().tell({ id: 2, records: [{ key: "cell:A" }] });
  socket().onclose?.();
  assert.equal(stood.at(-1)?.state, "stale");
  assert.equal(following.held.size, 1, "the records stay as they stood");
  clock.pass(waitBefore(1));
  assert.equal(sockets.length, 2, "it tried again after the first wait");
  socket().onclose?.();
  clock.pass(waitBefore(1));
  assert.equal(sockets.length, 2, "the second wait is longer than the first");
  clock.pass(waitBefore(2));
  assert.equal(sockets.length, 3);
  assert.equal(stood.filter((each) => each.state === "stale").length, 1, "said stale once");
});

it("the whole told again after coming back stands for every record", () => {
  const { following, socket, clock } = followed();
  socket().tell({ id: 2, records: [{ key: "cell:A" }, { key: "cell:B" }] });
  socket().onclose?.();
  clock.pass(waitBefore(1));
  socket().tell({ id: 2, records: [{ key: "cell:B" }] });
  assert.deepEqual(
    following.held.all().map((record) => record.key),
    ["cell:B"],
  );
});

it("closed, it closes its socket and tries nothing again", () => {
  const { following, sockets, socket, clock } = followed();
  following.close();
  assert.ok(socket().closed);
  socket().onclose?.();
  clock.pass(waitBefore(10));
  assert.equal(sockets.length, 1);
});

it("the wait doubles from a second to at most half a minute", () => {
  assert.deepEqual([1, 2, 3, 6, 20].map(waitBefore), [1000, 2000, 4000, 30_000, 30_000]);
});
