// Your system, now: the four parts of the system as the system daemon holds
// them, each with how it stands and what it says, and beside them the part
// the cursor is on, with its machines and the ways deeper. It watches the
// daemon's summary, parts and things over the sites' server, so a part
// redraws the moment its owner's answer moves, and the line under the list
// says whether what it shows is live and since when.

import { follow, type Standing } from "../../local-sites/kit/feed.ts";
import { hasOwnKeys, listen, toggle } from "../../local-sites/kit/keys.ts";
import { Held } from "../../local-sites/kit/records.ts";
import { clockTime, said } from "../../local-sites/kit/status.ts";
import { themeSwitch } from "../../local-sites/kit/theme.ts";
import { home, panelOf, partRow, siteName } from "./draw.ts";
import { judged, type Part, type System, systemOf, WORDS } from "./model.ts";

/** The element `selector` finds, which the page's markup always holds. */
function element<T extends Element>(selector: string, kind: new () => T): T {
  const found = document.querySelector(selector);
  if (!(found instanceof kind)) {
    throw new Error(`the page has no ${selector}, which the system page draws into`);
  }
  return found;
}

const list = element("#parts", HTMLDivElement);
const verdict = element("#verdict", HTMLParagraphElement);
const asof = element("#asof", HTMLParagraphElement);
const panel = element("#panel-body", HTMLDivElement);
const keys = element("#keys", HTMLDialogElement);

const KINDS = ["summary", "part", "machine"] as const;
const held = {
  summary: new Held(),
  part: new Held(),
  machine: new Held(),
};
const standings = new Map<string, Standing>();
// The last thing each watch couldn't read, said until it reads another.
const heard = new Map<string, string>();

let system: System | undefined;
let cursor: string | null = null;
// The machine the cursor is on inside the part, once Joel has gone in.
let inner: string | null = null;

/** The worst of the three watches' states, the one the page says. */
function standing(): Standing | undefined {
  const order = ["broken", "stale", "connecting", "live"];
  return [...standings.values()].sort(
    (one, other) => order.indexOf(one.state) - order.indexOf(other.state),
  )[0];
}

/** Says when the system was read, and whether that is live. */
function sayWhen(): void {
  const now = standing();
  const read = system === undefined ? "" : readAt(system);
  const problems = [...(system?.unread ?? []), ...heard.values()];
  const trouble = problems.length > 0 ? ` Couldn't read: ${problems.join("; ")}.` : "";
  asof.textContent = `${read}${now === undefined ? "" : said(now)}${trouble}`;
  asof.dataset.state = now?.state ?? "connecting";
}

/** When and where `read` was read, said as a sentence's start. */
function readAt(read: System): string {
  const at = Date.parse(read.readAt);
  const when = Number.isNaN(at) ? "at an unknown time" : `at ${clockTime(at)}`;
  return `Read on ${read.machine} ${when}. `;
}

const part = (): Part | undefined => system?.parts.find((each) => each.part === cursor);

function select(): void {
  for (const row of list.querySelectorAll<HTMLElement>("[role=option]")) {
    const on = row.dataset.part === cursor;
    row.setAttribute("aria-selected", String(on));
    if (on) {
      list.setAttribute("aria-activedescendant", row.id);
      row.scrollIntoView({ block: "nearest" });
    }
  }
}

function drawPanel(): void {
  const open = part();
  if (open === undefined) {
    panel.innerHTML = "";
    return;
  }
  if (inner !== null && !open.things.some((thing) => thing.name === inner)) {
    inner = null;
  }
  panel.innerHTML = panelOf(open, inner);
}

function draw(): void {
  system = systemOf(held.summary, held.part, held.machine);
  sayWhen();
  if (system === undefined) {
    return;
  }
  verdict.textContent = judged(system);
  document.title =
    system.health === "good" ? "Your system, now" : `Your system, now: ${WORDS[system.health]}`;
  const { parts } = system;
  if (!parts.some((each) => each.part === cursor)) {
    cursor = parts[0]?.part ?? null;
  }
  list.innerHTML = parts.map(partRow).join("");
  select();
  drawPanel();
}

for (const kind of KINDS) {
  follow(
    { feed: "system", kind },
    {
      moved: draw,
      stands: (now) => {
        standings.set(kind, now);
        sayWhen();
      },
      problem: (what) => {
        heard.set(kind, `${kind}: ${what}`);
        sayWhen();
      },
    },
  );
}

/** Inside a part, the cursor is the machine whose link holds focus. */
function focusInner(): void {
  if (inner !== null) {
    panel.querySelector<HTMLElement>(`#things a[data-name="${CSS.escape(inner)}"]`)?.focus();
  }
}

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
  const open = part();
  if (system === undefined || open === undefined) {
    return;
  }
  if (inner === null) {
    const { parts } = system;
    const at = parts.findIndex((each) => each.part === cursor);
    cursor = parts[stepped(at, by, parts.length)]?.part ?? cursor;
    select();
    drawPanel();
    return;
  }
  const at = open.things.findIndex((thing) => thing.name === inner);
  inner = open.things[stepped(at, by, open.things.length)]?.name ?? inner;
  drawPanel();
  focusInner();
}

function goIn(): void {
  const first = part()?.things[0];
  if (first === undefined || inner !== null) {
    return;
  }
  inner = first.name;
  drawPanel();
  focusInner();
}

function goOut(): void {
  if (inner === null) {
    return;
  }
  inner = null;
  drawPanel();
  list.focus();
}

function goDeeper(): void {
  const at = part();
  if (at === undefined) {
    return;
  }
  const thing = at.things.find((each) => each.name === inner);
  location.href = thing === undefined ? `../${at.deeper.site}` : home(thing);
}

/** Copies the part's command, and says whether it could. */
function copy(): void {
  const at = part();
  if (at === undefined) {
    return;
  }
  const { command } = at.deeper;
  navigator.clipboard.writeText(command).then(
    () => {
      asof.textContent = `Copied ${command}.`;
    },
    (error: unknown) => {
      asof.textContent = `Couldn't copy (${String(error)}); the command is ${command}.`;
    },
  );
}

/** Whether focus is on one of the part's machines, which is the cursor. */
function onThing(): boolean {
  return document.activeElement?.closest("#things") !== null;
}

listen({
  help: keys,
  // On a machine, Return follows its link; anywhere else in the list it
  // goes the part's way deeper.
  actions: {
    Enter: () => (onThing() ? false : goDeeper()),
    j: () => move(1),
    ArrowDown: () => move(1),
    k: () => move(-1),
    ArrowUp: () => move(-1),
    g: () => move(Number.NEGATIVE_INFINITY),
    Home: () => move(Number.NEGATIVE_INFINITY),
    G: () => move(Number.POSITIVE_INFINITY),
    End: () => move(Number.POSITIVE_INFINITY),
    l: goIn,
    ArrowRight: goIn,
    "-": goOut,
    h: goOut,
    ArrowLeft: goOut,
    y: copy,
  },
  // A machine's link is the cursor, not a control of its own.
  answersAt: (target) => target.closest("#things") !== null || !hasOwnKeys(target),
  afterwards: () => {
    if (inner === null && document.activeElement !== list) {
      list.focus();
    }
  },
});

list.addEventListener("click", (event) => {
  const row = event.target instanceof Element ? event.target.closest("[role=option]") : null;
  if (!(row instanceof HTMLElement)) {
    return;
  }
  cursor = row.dataset.part ?? cursor;
  inner = null;
  select();
  drawPanel();
  list.focus();
});

panel.addEventListener("click", (event) => {
  if (event.target instanceof Element && event.target.closest(".copy") !== null) {
    copy();
  }
});

// Tabbing onto a machine goes into the part at that machine.
panel.addEventListener("focusin", (event) => {
  const link = event.target instanceof Element ? event.target.closest("#things a") : null;
  if (!(link instanceof HTMLAnchorElement) || link.dataset.name === inner) {
    return;
  }
  inner = link.dataset.name ?? null;
  for (const each of panel.querySelectorAll<HTMLAnchorElement>("#things a")) {
    const on = each === link;
    each.tabIndex = on ? 0 : -1;
    each.setAttribute("aria-current", String(on));
  }
  const act = panel.querySelector(".act");
  const at = part();
  if (act !== null && at !== undefined) {
    act.textContent = `Press Return to open ${inner ?? siteName(at)}.`;
  }
});

themeSwitch();

for (const key of document.querySelectorAll<HTMLElement>(".head .keys kbd[data-key]")) {
  key.addEventListener("click", () => {
    if (key.dataset.key === "?") {
      toggle(keys);
    } else if (key.dataset.key === "Enter") {
      goDeeper();
    } else {
      list.focus();
    }
  });
}
