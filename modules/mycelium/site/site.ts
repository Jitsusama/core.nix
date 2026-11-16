// Your mycelium, now, heard live: watches the node's networks, cells,
// machines and messages over the sites' server, makes of them what the map
// draws (model.ts), and hands that to the drawing (view.js) as a
// `mycelium:now` event, so the map moves the moment the realm does. The line
// over the map says whether what it shows is live and since when.
//
// A reading is handed over at most every FLOOR_MS, however fast the node
// moves, since a whole map redraws for each, and none while the tab is
// hidden: idle means zero, and the latest is handed over as it shows again.
// Nothing is drawn until every kind has answered once, so the first map is
// never one with its machines or its messages missing.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { said } from "../../local-sites/kit/status.ts";
import { type Heard, nowOf } from "./model.ts";

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

/** The worst of the four watches' states, the one the map says. */
function standing(): Standing | undefined {
  const order = ["broken", "stale", "connecting", "live"];
  return [...standings.values()].sort(
    (one, other) => order.indexOf(one.state) - order.indexOf(other.state),
  )[0];
}

/** Says how the map stands, and what it couldn't read. */
function sayHow(): void {
  const now = standing();
  const problems = [...unread.values()];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  asof.textContent = `${now === undefined ? "Reaching the node." : said(now)}${trouble}`;
  asof.dataset.state = now?.state ?? "connecting";
  // The drawing's own look for a map that isn't now.
  asof.classList.toggle("stale", now?.state === "stale" || now?.state === "broken");
}

/** Hands the drawing the map as it stands now. */
function handOver(): void {
  waiting = undefined;
  owed = false;
  handed = Date.now();
  document.dispatchEvent(new CustomEvent(NOW_EVENT, { detail: nowOf(heard, handed) }));
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

// What each watch holds, filled as its records arrive: the drawing reads these
// and never a copy, since a copy would stay as it was first told.
const heard: Heard = {
  network: watch("network"),
  cell: watch("cell"),
  machine: watch("machine"),
  message: watch("message"),
};

document.addEventListener("visibilitychange", () => {
  if (!document.hidden && owed) {
    owe();
  }
});
sayHow();
