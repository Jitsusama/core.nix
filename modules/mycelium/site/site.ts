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
// Nothing is drawn until every kind has answered once, so the first map is
// never one with its machines or its messages missing. The balance, each
// machine's share of the realm's work, and what waits on the mesh are
// watched apart: a node serves the one only where the balance service runs
// and the other only once it is new enough, so the map neither waits for
// them nor says them as trouble, and draws without them until they come.
// The ties between cells are watched apart for the same reason: a node too
// old to serve them draws its talk from the latest messages, as before.
// What each receptor fed is watched apart too, since only a node new enough
// serves it, and a map without it draws no feeding at all.

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

/** The worst of the four watches' states, the one the map says. */
function standing(): Standing | undefined {
  const order = ["broken", "stale", "connecting", "live"];
  return [...standings.values()].sort(
    (one, other) => order.indexOf(one.state) - order.indexOf(other.state),
  )[0];
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
  asof.textContent = `${sayStanding(now)}${trouble}`;
  asof.dataset["state"] = now?.state ?? "connecting";
  // The drawing's own look for a map that isn't now.
  asof.classList.toggle("stale", now?.state === "stale" || now?.state === "broken");
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

/** Hands a reading over once the floor allows, and while the tab shows. */
function owe(): void {
  owed = true;
  if (document.hidden || answered.size < KINDS.length || waiting !== undefined) {
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
        answeredOnce = true;
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
        answered.add(kind);
        unread.delete(kind);
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

// What each watch holds, filled as its records arrive: the drawing reads these
// and never a copy, since a copy would stay as it was first told. What waits
// is there only once its watch has answered, so a node that doesn't serve it
// reads as one that says nothing of it rather than one with nothing waiting.
const heard: Heard = {
  network: watch("network"),
  cell: watch("cell"),
  machine: watch("machine"),
  message: watch("message"),
  balance: watchApart("balance").held,
  get unanswered() {
    return waits.answered() ? waits.held : undefined;
  },
  get tie() {
    return ties.answered() ? ties.held : undefined;
  },
  get fed() {
    return feedings.answered() ? feedings.held : undefined;
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
