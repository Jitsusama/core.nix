// Your mycelium, now, heard live: watches the node's networks, cells,
// machines and messages over the sites' server, makes of them what the map
// draws (model.ts), and hands that to the drawing (view.js) as a
// `mycelium:now` event, so the map moves the moment the realm does. The
// standing line under the head's sentence says whether what it shows is live,
// from whose node, and since when.
//
// A reading is handed over at most every FLOOR_MS, however fast the node
// moves, since a whole map redraws for each, and none while the tab is
// hidden: idle means zero, and the latest is handed over as it shows again.
// Nothing is drawn until the networks, cells and machines have answered
// once, so the first map is never one with its machines missing. Messages
// are waited for only where the node serves no ties, which the map draws
// its talk from: a node's first answer of them is every message it keeps,
// once 12,179 of them and 14 s late, for a map that lists the latest few,
// so it draws without them and says it is still reading them. The balance, each
// machine's share of the realm's work, and what waits on the mesh are
// watched apart: a node serves the one only where the balance service runs
// and the other only once it is new enough, so the map neither waits for
// them nor says them as trouble, and draws without them until they come.
// The ties between cells are watched apart for the same reason: a node too
// old to serve them draws its talk from the latest messages, as before.
// What each receptor fed is watched apart too, since only a node new enough
// serves it, and a map without it draws no feeding at all. So are the
// forages, which say who started each of this machine's cells where the
// cell's own record doesn't, as an older node's doesn't.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { clockTime, said } from "../../local-sites/kit/status.ts";
import { expiryOf, type Heard, nowOf } from "./model.ts";
import { asked, sayReach } from "./reach.ts";

/** The least time between two readings handed to the drawing. */
const FLOOR_MS = 2000;

/** The event the drawing hears each reading by. */
const NOW_EVENT = "mycelium:now";

const KINDS = ["network", "cell", "machine", "message"] as const;
type Kind = (typeof KINDS)[number];

/** The kinds the map is never drawn without. */
const DRAWN_WITH: readonly Kind[] = ["network", "cell", "machine"];

/** What the standing line adds while the map is drawn and its messages aren't read. */
const READING_TALK = " Reading what passed between cells.";

/** What it adds once they have taken longer than a busy node takes to answer. */
const WAITING_TALK = " Still waiting on what passed between cells.";

/**
 * How long messages may take to first answer once the map is drawn before
 * the line says it is waiting on them rather than reading them: a node
 * took 14 s on 26 November, its machine at its busiest.
 */
const TALK_PATIENCE_MS = 10_000;

const found = document.querySelector("#asof");
if (!(found instanceof HTMLParagraphElement)) {
  throw new Error("the page has no #asof, where the map says how it stands");
}
const asof = found;

const answered = new Set<Kind>();
const standings = new Map<Kind, Standing>();
// The last thing each watch couldn't read, said until it reads another.
const unread = new Map<Kind, string>();

let handed = Number.NEGATIVE_INFINITY;
let waiting: number | undefined;
let owed = false;
let expires: number | undefined;
// When the map was first drawn with its messages yet to answer.
let readingSince: number | undefined;

/**
 * The worst of the watches' states, the one the map says. Messages still
 * connecting for their first answer once the map is drawn are said apart,
 * so a map drawn live isn't said as one still reaching its node; once they
 * have answered, connecting again is said as any watch's is.
 */
function standing(): Standing | undefined {
  const order = ["broken", "stale", "connecting", "live"];
  const counted = [...standings].filter(
    ([kind, now]) => !(kind === "message" && now.state === "connecting" && readingTalk()),
  );
  return counted
    .map(([, now]) => now)
    .sort((one, other) => order.indexOf(one.state) - order.indexOf(other.state))[0];
}

/**
 * Whose node the map is read from: the machine's, by name once the page
 * is told it (as `data-home` on the page), and "this machine's" until
 * then, so the line never reads with a gap in it.
 */
function nodeOf(): string {
  const home = document.documentElement.dataset["home"];
  return home ? `${home}'s node` : "this machine's node";
}

/** How the map stands, in a sentence: whose node it is live from, and since when. */
function sayStanding(now: Standing | undefined): string {
  if (now === undefined || now.state === "connecting") {
    return `Reaching ${nodeOf()}.`;
  }
  if (now.state === "live") {
    return `Live from ${nodeOf()} since ${clockTime(now.since)}.`;
  }
  return said(now);
}

/** Says how the map stands, and what it couldn't read. */
function sayHow(): void {
  const now = standing();
  const problems = [...unread.values()];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  asof.textContent = `${sayStanding(now)}${talkWords()}${trouble}`;
  asof.dataset["state"] = now?.state ?? "connecting";
  // The drawing's own look for a map that isn't now.
  asof.classList.toggle("stale", now?.state === "stale" || now?.state === "broken");
}

/**
 * What the line says of messages the map is drawn without: that it is
 * reading them, and past its patience that it is still waiting on them,
 * said again then so the words move without a reading to move them. A
 * message watch that broke is said by the standing instead.
 */
function talkWords(): string {
  if (!readingTalk() || standings.get("message")?.state === "broken") {
    return "";
  }
  if (readingSince === undefined) {
    readingSince = Date.now();
    globalThis.setTimeout(sayHow, TALK_PATIENCE_MS + 1);
  }
  return Date.now() - readingSince > TALK_PATIENCE_MS ? WAITING_TALK : READING_TALK;
}

/** Hands the drawing the map as it stands now. */
function handOver(): void {
  waiting = undefined;
  owed = false;
  handed = Date.now();
  const now = nowOf(heard, handed);
  document.dispatchEvent(new CustomEvent(NOW_EVENT, { detail: now }));
  // A drawn balance goes stale, and a feeding's line has had its time, with
  // nothing else moving, as when the service stops or the node falls quiet,
  // so draw again the moment the first of them does.
  globalThis.clearTimeout(expires);
  const due = expiryOf(now);
  expires = due === undefined ? undefined : globalThis.setTimeout(owe, due - handed + 1);
}

/**
 * Whether the map can be drawn: its networks, cells and machines heard, and
 * its talk, from the ties where the node serves them, else the messages.
 */
function drawable(): boolean {
  return (
    DRAWN_WITH.every((kind) => answered.has(kind)) && (answered.has("message") || ties.answered())
  );
}

/** Whether the map is drawn and its messages have yet to first answer. */
function readingTalk(): boolean {
  return drawable() && !answered.has("message");
}

/** Hands a reading over once the floor allows, and while the tab shows. */
function owe(): void {
  owed = true;
  if (document.hidden || !drawable() || waiting !== undefined) {
    return;
  }
  const wait = handed + FLOOR_MS - Date.now();
  if (wait <= 0) {
    handOver();
  } else {
    waiting = globalThis.setTimeout(handOver, wait);
  }
}

/** A kind watched apart from those the map waits on, and whether it has answered. */
interface Apart {
  readonly held: Held;
  readonly answered: () => boolean;
}

/**
 * Watches a kind the map draws without: a reading moves the map as any
 * does, but its standing and its trouble are the map's only once it is
 * served everywhere, and until then the map just draws none of it.
 */
function watchApart(kind: string): Apart {
  let answeredOnce = false;
  const { held } = follow(
    { feed: "node", kind },
    {
      moved: () => {
        // The ties first answering can be what lets the map draw before
        // its messages, which the standing line says. Only then, so a
        // line the drawing wrote, as when a reading couldn't draw, stays.
        if (!answeredOnce) {
          answeredOnce = true;
          sayHow();
        }
        owe();
      },
      stands: () => undefined,
      problem: () => undefined,
    },
  );
  return { held, answered: () => answeredOnce };
}

/** Watches one kind, answering with the records its watch holds. */
function watch(kind: Kind): Held {
  return follow(
    { feed: "node", kind },
    {
      moved: () => {
        // What the standing line says moves with what has answered, as
        // much as with how each watch stands, so it is said again when a
        // kind first answers or was unread, and otherwise left as it is.
        const news = !answered.has(kind) || unread.has(kind);
        answered.add(kind);
        unread.delete(kind);
        if (news) {
          sayHow();
        }
        owe();
      },
      stands: (now) => {
        standings.set(kind, now);
        sayHow();
      },
      problem: (what) => {
        unread.set(kind, `${kind}: ${what}`);
        sayHow();
      },
    },
  ).held;
}

const waits = watchApart("unanswered");
const ties = watchApart("tie");
const feedings = watchApart("fed");
const forages = watchApart("forage");

// What each watch holds, filled as its records arrive: the drawing reads these
// and never a copy, since a copy would stay as it was first told. What waits
// is there only once its watch has answered, so a node that doesn't serve it
// reads as one that says nothing of it rather than one with nothing waiting.
const messages = watch("message");

const heard: Heard = {
  network: watch("network"),
  cell: watch("cell"),
  machine: watch("machine"),
  // None until told, so a map drawn before its messages says it is still
  // reading them rather than that nothing passed.
  get message() {
    return answered.has("message") ? messages : undefined;
  },
  balance: watchApart("balance").held,
  builder: watchApart("builder").held,
  get unanswered() {
    return waits.answered() ? waits.held : undefined;
  },
  get tie() {
    return ties.answered() ? ties.held : undefined;
  },
  get fed() {
    return feedings.answered() ? feedings.held : undefined;
  },
  get forage() {
    return forages.answered() ? forages.held : undefined;
  },
};

document.addEventListener("visibilitychange", () => {
  if (!document.hidden && owed) {
    owe();
  }
});
sayHow();
// Opened to be checked, the page says whether it reaches every cell.
if (asked(location.search)) {
  sayReach(document.documentElement, document);
}
