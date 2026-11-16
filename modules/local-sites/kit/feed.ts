// A page's watch of one kind on one feed: the WebSocket the local-sites
// server bridges to a daemon's socket at /feed/<name>, spoken in the daemon
// interface (docs/daemons.md), held as the records it has told.
//
// The page is told how the watch stands as well as what it holds:
// connecting until the server says the daemon answers, live while it does,
// broken with the server's reason while it doesn't, and stale while the
// server itself can't be reached, the records kept as they last stood and
// the socket tried again, sooner at first and then at most every half
// minute. Each state says since when, so a page never passes an old
// reading off as now.
//
// Nothing read off the socket is trusted: a line that isn't JSON, an answer
// that isn't an object, or a record without a key is told to the page as a
// problem, which it says, rather than dropped or thrown where no one sees.

import { type Fields, Held, isObject, type Moved } from "./records.ts";

/** How a watch stands: what a page says above what it draws. */
type State = "connecting" | "live" | "stale" | "broken";

/** A state, since when (milliseconds since the epoch), and why, if broken. */
interface Standing {
  readonly state: State;
  readonly since: number;
  readonly reason?: string;
}

/** What a page watches: a feed, one of its kinds, narrowed if it likes. */
interface Watching {
  readonly feed: string;
  readonly kind: string;
  readonly keys?: readonly string[];
  readonly fields?: readonly string[];
  /** Fields the feed declares deep, whose changes are mended one level in. */
  readonly deep?: readonly string[];
}

/** What a page is told as the watch goes. */
interface Hearing {
  /** The records moved; `held` holds every record as it now stands. */
  readonly moved: (held: Held, moved: Moved) => void;
  /** The watch's state changed. */
  readonly stands: (standing: Standing) => void;
  /** Something read off the feed couldn't be understood. */
  readonly problem: (what: string) => void;
}

/** The part of a WebSocket a watch uses, so a test can stand in for one. */
interface Socket {
  onopen: (() => void) | null;
  onmessage: ((event: { readonly data: unknown }) => void) | null;
  onclose: (() => void) | null;
  send: (line: string) => void;
  close: () => void;
}

/** The clock a watch tells time and waits by, so a test can stand in. */
interface Clock {
  readonly now: () => number;
  readonly later: (milliseconds: number, then: () => void) => () => void;
}

/** How a watch reaches its feed. Every part has the browser's default. */
interface Reaching {
  readonly connect?: (url: string) => Socket;
  readonly clock?: Clock;
  /** The page's own address, whose host the feed is asked at. */
  readonly page?: string;
}

/** A watch under way: the records it holds, and a way to end it. */
interface Following {
  readonly held: Held;
  readonly close: () => void;
}

const HELLO = 1;
const WATCH = 2;
const FIRST_WAIT = 1000;
const LONGEST_WAIT = 30_000;

/** How long to wait before the `attempt`th try again, doubling to a cap. */
function waitBefore(attempt: number): number {
  return Math.min(LONGEST_WAIT, FIRST_WAIT * 2 ** Math.max(0, attempt - 1));
}

/** The address of `feed` on the server that served `page`. */
function feedAddress(page: string, feed: string): string {
  const url = new URL(`/feed/${encodeURIComponent(feed)}`, page);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.href;
}

const browserClock: Clock = {
  now: () => Date.now(),
  later: (milliseconds, then) => {
    const timer = setTimeout(then, milliseconds);
    return () => clearTimeout(timer);
  },
};

/** The browser's WebSocket to `url`, as the part of one a watch uses. */
function browserSocket(url: string): Socket {
  const socket = new WebSocket(url);
  const used: Socket = {
    onopen: null,
    onmessage: null,
    onclose: null,
    send: (line) => socket.send(line),
    close: () => socket.close(),
  };
  socket.addEventListener("open", () => used.onopen?.());
  socket.addEventListener("message", (event) => used.onmessage?.({ data: event.data }));
  socket.addEventListener("close", () => used.onclose?.());
  return used;
}

/** The watch's ask, with only what the page narrowed it by. */
function asked(watching: Watching): string {
  const ask: { id: number; ask: string; kind: string; keys?: unknown; fields?: unknown } = {
    id: WATCH,
    ask: "watch",
    kind: watching.kind,
  };
  if (watching.keys !== undefined) {
    ask.keys = watching.keys;
  }
  if (watching.fields !== undefined) {
    ask.fields = watching.fields;
  }
  return JSON.stringify(ask);
}

/** What an error answer says, as a reason. */
function refusal(error: unknown): string {
  if (isObject(error) && typeof error["message"] === "string") {
    return error["message"];
  }
  return "the feed refused the watch";
}

/** A line read off the feed as an answer, or why it isn't one. */
function answerIn(data: unknown): Fields | string {
  if (typeof data !== "string") {
    return "the feed sent something that wasn't text";
  }
  let line: unknown;
  try {
    line = JSON.parse(data);
  } catch (error) {
    return `the feed sent a line that isn't JSON (${String(error)})`;
  }
  return isObject(line) ? line : `the feed sent a line that isn't an answer: ${data}`;
}

/** One watch's life: its socket, how it stands, and its tries again. */
class Follower {
  readonly held: Held;
  readonly #watching: Watching;
  readonly #hearing: Hearing;
  readonly #clock: Clock;
  readonly #connect: (url: string) => Socket;
  readonly #url: string;
  #socket: Socket | null = null;
  #attempt = 0;
  #cancel: (() => void) | null = null;
  #standing: Standing | null = null;

  constructor(watching: Watching, hearing: Hearing, reaching: Reaching) {
    this.#watching = watching;
    this.#hearing = hearing;
    this.#clock = reaching.clock ?? browserClock;
    this.#connect = reaching.connect ?? browserSocket;
    this.#url = feedAddress(reaching.page ?? location.href, watching.feed);
    this.held = new Held(watching.deep);
    this.#stand({ state: "connecting", since: this.#clock.now() });
    this.#open();
  }

  /** Ends the watch: its socket closed, and nothing tried again. */
  close(): void {
    this.#cancel?.();
    this.#socket?.close();
    this.#socket = null;
  }

  #open(): void {
    this.#cancel = null;
    const socket = this.#connect(this.#url);
    this.#socket = socket;
    socket.onopen = () => {
      socket.send(JSON.stringify({ id: HELLO, ask: "hello" }));
      socket.send(asked(this.#watching));
    };
    socket.onmessage = (event) => this.#read(event.data);
    socket.onclose = () => this.#lost(socket);
  }

  /** The server went: the records kept as they stood, and tried again. */
  #lost(socket: Socket): void {
    // A socket closed by the page, or one already given up for another,
    // is no longer the watch's, and nothing is tried again for it.
    if (this.#socket !== socket) {
      return;
    }
    this.#socket = null;
    this.#attempt += 1;
    const since = this.#standing?.state === "stale" ? this.#standing.since : this.#clock.now();
    this.#stand({ state: "stale", since, reason: "the sites' server can't be reached" });
    this.#cancel = this.#clock.later(waitBefore(this.#attempt), () => this.#open());
  }

  #stand(next: Standing): void {
    if (this.#standing?.state === next.state && this.#standing.reason === next.reason) {
      return;
    }
    this.#standing = next;
    this.#hearing.stands(next);
  }

  #read(data: unknown): void {
    const answer = answerIn(data);
    if (typeof answer === "string") {
      this.#hearing.problem(answer);
    } else if (answer["id"] === WATCH) {
      this.#heard(answer);
    } else if (answer["id"] !== HELLO) {
      this.#hearing.problem(`an answer to no ask of this page: ${JSON.stringify(answer)}`);
    } else if (answer["error"] !== undefined) {
      this.#refused(answer["error"]);
    }
  }

  #refused(error: unknown): void {
    this.#stand({ state: "broken", since: this.#clock.now(), reason: refusal(error) });
  }

  /** An answer to the watch: a refusal, a state the server says, or records. */
  #heard(answer: Fields): void {
    const state = answer["state"];
    if (answer["error"] !== undefined) {
      this.#refused(answer["error"]);
    } else if (state === "connecting" || state === "live") {
      this.#stand({ state, since: this.#clock.now() });
    } else if (state === "broken") {
      const told = answer["reason"];
      const reason = typeof told === "string" ? told : "its daemon didn't answer";
      const at = answer["since"];
      const since = typeof at === "number" ? at : this.#clock.now();
      this.#stand({ state, since, reason });
    } else {
      const moved = this.held.took(answer);
      if (moved.skipped > 0) {
        this.#hearing.problem(`${moved.skipped} of what the feed told had no key`);
      }
      this.#attempt = 0;
      this.#hearing.moved(this.held, moved);
    }
  }
}

/** Follows `watching`, telling `hearing` each change, until closed. */
function follow(watching: Watching, hearing: Hearing, reaching: Reaching = {}): Following {
  const follower = new Follower(watching, hearing, reaching);
  return { held: follower.held, close: () => follower.close() };
}

export type { Clock, Following, Hearing, Reaching, Socket, Standing, State, Watching };
export { feedAddress, follow, waitBefore };
