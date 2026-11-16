// What the sessions murmur: every murmur this machine's cells said into the
// quests' webs over the last day, drawn first as a flock at dusk, each bird
// with the cell that said it and a hairline to each quest it is tied to, and
// settled ones come down to roost on the wire; then listed, newest first,
// with its words, who said it,
// when, its ties, and how it was settled. It watches the mycelium node's
// murmur records over the sites' server, so a murmur arrives, ties or
// settles the moment it is said, and the standing line says whether what it
// shows is live and since when. Nothing on it changes anything; a terminal's
// `mycelium web` says, ties and settles.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { type Actions, listen, toggle } from "../../local-sites/kit/keys.ts";
import type { Held } from "../../local-sites/kit/records.ts";
import { said } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { escaped, murmurRow, skyOf } from "./draw.ts";
import { headline, lede, type Murmur, murmursOf } from "./model.ts";

/** The element `selector` finds, which the page's markup always holds. */
function found<T extends Element>(selector: string, kind: new () => T): T {
  const one = document.querySelector(selector);
  if (!(one instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the murmurs page draws into`);
  }
  return one;
}

/** How often the ages on the rows are said again, in milliseconds. */
const AGES_EVERY = 30_000;

const list = found("#murmurs", HTMLDivElement);
const title = found("#headline", HTMLHeadingElement);
const ledeLine = found("#lede", HTMLParagraphElement);
const sky = found("#sky", HTMLDivElement);
const asof = found("#asof", HTMLSpanElement);
const keys = found("#keys", HTMLDialogElement);
const listTitle = found("#murmurs-title", HTMLHeadingElement);

let murmurs: readonly Murmur[] = [];
let unread: readonly string[] = [];
let standing: Standing | undefined;
// The last thing the watch couldn't read, said until it reads another.
let heard: string | undefined;
// Whether the node has given any records yet, so a refusal before the first
// never leaves the page saying it's still asking.
let given = false;
let cursor: string | null = null;
// Whether the list holds only the murmurs still in the air, as the
// headline's second count asks.
let airOnly = false;

/** The murmurs the list shows, newest first. */
function shown(): readonly Murmur[] {
  return murmurs.filter((each) => !airOnly || each.settled === null);
}

/** Now, in seconds since 1970, as the records count time. */
function now(): number {
  return Math.floor(Date.now() / 1000);
}

/** Says whether what's drawn is live, and what couldn't be read. */
function sayWhen(): void {
  const problems = [...unread, ...(heard === undefined ? [] : [heard])];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  asof.textContent = `${standing === undefined ? "Reaching the mycelium node." : said(standing)}${trouble}`;
  asof.setAttribute("data-state", standing?.state ?? "connecting");
  if (!given && standing?.state === "broken") {
    title.textContent = "Nothing to hear yet.";
    ledeLine.textContent =
      "The mycelium node didn't say what its sessions murmur; the line below says why.";
  }
}

/**
 * Marks the cursor's row and its bird. Only a step of the cursor brings its
 * row into view, so a murmur arriving or an age moving on never scrolls the
 * page under the reader.
 */
function select(bringIntoView = false): void {
  for (const row of list.querySelectorAll<HTMLElement>('[aria-selected="true"]')) {
    if (row.getAttribute("data-key") !== cursor) {
      row.setAttribute("aria-selected", "false");
    }
  }
  const row = cursor === null ? null : document.getElementById(`murmur-${cursor}`);
  if (row instanceof HTMLElement && list.contains(row)) {
    row.setAttribute("aria-selected", "true");
    list.setAttribute("aria-activedescendant", row.id);
    if (bringIntoView) {
      row.scrollIntoView({ block: "nearest" });
    }
  } else {
    list.removeAttribute("aria-activedescendant");
  }
  for (const mark of sky.querySelectorAll<SVGElement>("[data-key]")) {
    mark.classList.toggle("on", mark.getAttribute("data-key") === cursor);
  }
}

function draw(): void {
  const rows = shown();
  if (!rows.some((each) => each.key === cursor)) {
    cursor = rows[0]?.key ?? null;
  }
  const at = now();
  list.innerHTML = rows.map((each) => murmurRow(each, at)).join("");
  // Once the node has spoken the sky is its murmurs, so when the last one
  // goes the flock goes with it and the wire is left bare.
  sky.innerHTML = skyOf(murmurs, cursor);
  listTitle.innerHTML = airOnly ? "The murmurs <em>still in the air</em>" : "The <em>murmurs</em>";
  select();
}

/** The headline, its counts leading to what they count, and the lede under it. */
function sayWhole(): void {
  const whole = headline(murmurs);
  const lead = escaped(whole.lead);
  const turn = escaped(whole.turn);
  if (whole.said === 0) {
    title.innerHTML = `${lead} <em>${turn}</em>`;
  } else if (whole.open === 0 || whole.open === whole.said) {
    title.innerHTML = `<a href="#all" data-show="all">${lead}</a> <em>${turn}</em>`;
  } else {
    title.innerHTML = `<a href="#all" data-show="all">${lead}</a> <em><a href="#air" data-show="air">${turn}</a></em>`;
  }
  ledeLine.textContent = lede(murmurs);
}

function heardAll(held: Held): void {
  given = true;
  ({ murmurs, unread } = murmursOf(held));
  sayWhole();
  sayWhen();
  draw();
}

follow(
  { feed: "node", kind: "murmur" },
  {
    moved: heardAll,
    stands: (state) => {
      standing = state;
      sayWhen();
    },
    problem: (what) => {
      heard = what;
      sayWhen();
    },
  },
);

/** The index `by` steps to from `at` among `count`, kept inside them. */
function stepped(at: number, by: number, count: number): number {
  if (by === Number.POSITIVE_INFINITY) {
    return count - 1;
  }
  if (by === Number.NEGATIVE_INFINITY) {
    return 0;
  }
  return Math.max(0, Math.min(count - 1, at + by));
}

function move(by: number): void {
  const rows = shown();
  const at = rows.findIndex((each) => each.key === cursor);
  cursor = rows[stepped(at, by, rows.length)]?.key ?? cursor;
  select(true);
}

/** What each key does in the list. */
const ACTIONS: Actions = {
  j: () => move(1),
  ArrowDown: () => move(1),
  k: () => move(-1),
  ArrowUp: () => move(-1),
  g: () => move(Number.NEGATIVE_INFINITY),
  Home: () => move(Number.NEGATIVE_INFINITY),
  G: () => move(Number.POSITIVE_INFINITY),
  End: () => move(Number.POSITIVE_INFINITY),
};

// The keys are the page's wherever focus is, but on another control, whose
// own keys come first; after one, the list holds the keys again.
listen({
  help: keys,
  actions: ACTIONS,
  afterwards: () => {
    if (document.activeElement !== list) {
      list.focus();
    }
  },
});

list.addEventListener("click", (event) => {
  const row = event.target instanceof Element ? event.target.closest("[role=option]") : null;
  if (!(row instanceof HTMLElement)) {
    return;
  }
  cursor = row.getAttribute("data-key") ?? cursor;
  select();
  list.focus();
});

title.addEventListener("click", (event) => {
  const asked = event.target instanceof Element ? event.target.closest("a[data-show]") : null;
  if (!(asked instanceof HTMLAnchorElement)) {
    return;
  }
  event.preventDefault();
  airOnly = asked.getAttribute("data-show") === "air";
  draw();
  list.focus();
});

sky.addEventListener("click", (event) => {
  const mark = event.target instanceof Element ? event.target.closest("[data-key]") : null;
  if (!(mark instanceof SVGElement)) {
    return;
  }
  cursor = mark.getAttribute("data-key") ?? cursor;
  airOnly = airOnly && murmurs.some((each) => each.key === cursor && each.settled === null);
  draw();
  list.focus();
});

// A murmur's age moves on while nothing else does, so it is said again now and then.
setInterval(() => {
  if (given) {
    draw();
  }
}, AGES_EVERY);

themeSwitch();

for (const key of document.querySelectorAll<HTMLElement>(".hero .keys kbd[data-key]")) {
  key.addEventListener("click", () => {
    if (key.getAttribute("data-key") === "?") {
      toggle(keys);
    } else {
      list.focus();
    }
  });
}
