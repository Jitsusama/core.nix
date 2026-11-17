// The system page's markup: how everything Joel runs stands. Each part
// stands on one horizon, and how it stands is its health in the system's
// own words, good, watch, bad or not read, each with its own mark so none
// is told by colour. The outlook draws the four stations large under the
// headline; each part below hangs its mark in the gutter, then what in it
// is worth a look and the way deeper. Every row the keys step through is a
// link marked data-row, in reading order, so the cursor is focus itself and
// Return follows it, and a row shows its own keys while it has the cursor.
// Every word read from a record is escaped before it reaches the page.

import {
  CLEAR,
  type Headline,
  type Health,
  looksIn,
  named,
  type Part,
  type System,
  type Thing,
  WORDS,
} from "./model.ts";

/**
 * Each health's mark on a 16 grid, its horizon at the foot: the sun half
 * risen with its rays, a cloud resting on the line, a storm cloud with its
 * bolt, and fog in broken lines.
 */
const MARKS: { readonly [health in Health]: string } = {
  good: '<path d="M1.5 13h13"/><path d="M4.2 13a3.8 3.8 0 0 1 7.6 0"/><path d="M8 6.4V4.6M4.2 8.1 3 6.9M11.8 8.1 13 6.9"/>',
  watch: '<path d="M4.6 13h6.9a2.7 2.7 0 0 0 .2-5.4 3.8 3.8 0 0 0-7.3.9A2.3 2.3 0 0 0 4.6 13z"/>',
  bad: '<path d="M4.6 9.6h6.9a2.6 2.6 0 0 0 .2-5.2 3.7 3.7 0 0 0-7.1.9 2.2 2.2 0 0 0 0 4.3z"/><path d="M8.8 10.2 7.2 12.6h2.1L7.8 15"/>',
  unread: '<path d="M2 6.5h12M3.5 9.5h9M2 12.5h12" stroke-dasharray="2.4 1.8"/>',
};

/** The most machines a part draws, worst first, so thousands stay short. */
const MACHINES_DRAWN = 3;

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** A row the keys step through: where it goes, its key, and what it shows. */
interface Row {
  readonly part: Part;
  readonly href: string;
  readonly key: string;
  readonly inner: string;
  /** The mark hung in its gutter, when it has one. */
  readonly health?: Health;
  /** The keys it answers while it has the cursor, beyond Return. */
  readonly acts?: string;
}

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** The mark `health` is drawn with, its word said only to a screen reader. */
function mark(health: Health, size = "small"): string {
  return `<svg class="sky ${health} ${size}" viewBox="0 0 16 16" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">${MARKS[health]}</g></svg>`;
}

/** How `health` is said to a screen reader, after a name. */
function spoken(health: Health): string {
  // In parentheses, since a reader takes the hidden span out of flow as a
  // block and puts a space before it: "Machines (watch)" reads right
  // where "Machines , watch" would not.
  return `<span class="sr"> (${WORDS[health].toLowerCase()})</span>`;
}

/** Where a thing's link goes: its home's address, or the mycelium site. */
function home(thing: Thing): string {
  return `../${thing.address ?? "mycelium/"}`;
}

/** The headline, its counts each a link to what it counts. */
function headlineOf(heading: Headline): string {
  const looks =
    heading.looks === undefined
      ? undefined
      : `<a href="#parts" data-to="look"><em>${escaped(heading.looks)}</em></a>`;
  const unread =
    heading.unread === undefined
      ? undefined
      : `<a href="#parts" data-to="unread">${escaped(heading.unread)}</a>`;
  const clauses = [looks, unread].filter((clause) => clause !== undefined);
  const rest = clauses.length === 0 ? `<em>${CLEAR}</em>` : clauses.join(" and ");
  return `<a href="#parts" data-to="part">${escaped(heading.parts)}</a>, ${rest}.`;
}

/**
 * What one part says under its name on the outlook, as markup: how many looks,
 * or its health's word.
 */
function forecast(part: Part): string {
  const looks = looksIn(part);
  if (looks > 0) {
    return `${looks} worth a look`;
  }
  return WORDS[part.health].toLowerCase();
}

/** The outlook: the four parts as stations on one horizon, each its mark. */
function outlookOf(system: System): string {
  return system.parts
    .map((part, at) => {
      const name = escaped(named(part.part));
      return `<li><a class="station ${part.health}" href="#part-${escaped(part.part)}" data-station="${at + 1}">${mark(part.health, "large")}<span class="place">${name}</span><span class="forecast">${forecast(part)}</span></a></li>`;
    })
    .join("");
}

/** The place words at the top: each part's name and how many looks it has. */
function placesOf(system: System): string {
  return system.parts
    .map((part) => {
      const looks = looksIn(part);
      const count = looks > 0 ? `<span class="n">${looks}</span>` : "";
      return `<a class="place" href="#part-${escaped(part.part)}" data-place="${escaped(part.part)}">${escaped(named(part.part))}${count}</a>`;
    })
    .join("");
}

/** The keys a row answers while it has the cursor, shown on the row then. */
function actsOf(acts: string | undefined): string {
  const more = acts === undefined ? "" : ` ${acts}`;
  return `<span class="acts" aria-hidden="true"><kbd>Return</kbd> opens${more}</span>`;
}

/** A row the keys step through, as a list item. */
function row(drawn: Row): string {
  return `<li>${link(drawn)}</li>`;
}

/** A row's link: its mark in the gutter, its words on the main line, its keys. */
function link(drawn: Row): string {
  const gutter = drawn.health === undefined ? "" : mark(drawn.health);
  const health = drawn.health === undefined ? "" : ` data-health="${drawn.health}"`;
  return `<a class="row" data-row="${escaped(drawn.key)}" data-part="${escaped(drawn.part.part)}"${health} href="${escaped(drawn.href)}"><span class="gut">${gutter}</span><span class="line">${drawn.inner}</span>${actsOf(drawn.acts)}</a>`;
}

/** A machine's row: its name, then what it says. */
function machineRow(part: Part, thing: Thing): string {
  const inner = `<span class="own" translate="no">${escaped(thing.name)}</span>${spoken(thing.health)} ${escaped(thing.says)}`;
  return row({
    part,
    href: home(thing),
    key: `machine:${thing.name}`,
    inner,
    health: thing.health,
  });
}

/** What in `part` is worth a look, as rows: its machines, or its side. */
function itemsOf(part: Part): string[] {
  const site = `../${part.deeper.site}`;
  if (part.things.length > 0) {
    const drawn = part.things.slice(0, MACHINES_DRAWN).map((thing) => machineRow(part, thing));
    const more = part.things.length - MACHINES_DRAWN;
    if (more > 0) {
      const words = `and ${more} more ${more === 1 ? "machine" : "machines"}`;
      drawn.push(row({ part, href: site, key: `${part.part}:more`, inner: escaped(words) }));
    }
    return drawn;
  }
  if (part.health === "unread") {
    return [];
  }
  return part.side.map((item, at) =>
    row({ part, href: site, key: `${part.part}:${at}`, inner: escaped(item) }),
  );
}

/** The ways deeper into `part`: its command, which y copies, and where it is in the launcher. */
function deeperOf(part: Part): string {
  const command = escaped(part.deeper.command);
  return `<p class="deeper"><code>${command}</code><button type="button" class="copy" data-command="${command}" aria-label="Copy ${command}">copy</button><span class="launcher"><span class="sr">In the launcher: </span>${escaped(part.deeper.launcher)}</span></p>`;
}

/** `part`, drawn: its heading, what it holds, what is worth a look and the ways deeper. */
function partOf(part: Part): string {
  const id = escaped(part.part);
  const holds =
    part.health === "unread"
      ? `<p class="holds unread">${escaped(part.says)}</p>`
      : `<p class="holds">${escaped(part.holds ?? "")}</p>`;
  const items = itemsOf(part);
  const list = items.length === 0 ? "" : `<ul class="items">${items.join("")}</ul>`;
  const heading = link({
    part,
    href: `../${part.deeper.site}`,
    key: `part:${part.part}`,
    inner: `${escaped(named(part.part))}${spoken(part.health)}`,
    health: part.health,
    acts: "and <kbd>y</kbd> copies its command",
  });
  return `<section class="part ${part.health}" id="part-${id}" aria-labelledby="name-${id}">
  <h2 class="name" id="name-${id}">${heading}</h2>
  ${holds}
  ${list}
  ${deeperOf(part)}
</section>`;
}

/** Every part of `system`, in its order. */
function partsOf(system: System): string {
  return system.parts.map((part) => partOf(part)).join("\n");
}

/**
 * The legend: each health's mark beside its word, said once, an item each
 * so a screen reader hears the words apart, the marks being hidden from it.
 */
function legend(): string {
  const healths: readonly Health[] = ["good", "watch", "bad", "unread"];
  return healths
    .map(
      (health) => `<li class="key">${mark(health)}<span class="word">${WORDS[health]}</span></li>`,
    )
    .join("");
}

export { escaped, headlineOf, home, legend, outlookOf, partsOf, placesOf };
