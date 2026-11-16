// The system page's markup: the headline and its lede, and each part as a
// heading over what in it is worth a look and the ways deeper. Every row
// the keys step through is a link marked data-row, in reading order, so the
// cursor is focus itself and Return follows it. Every word read from a
// record is escaped before it reaches the page. A health is told by its
// mark's shape, drawn in the gutter, and said in words to a screen reader
// and once in the legend, so nothing is told by colour alone.

import {
  type Headline,
  type Health,
  named,
  type Part,
  type System,
  type Thing,
  WORDS,
} from "./model.ts";

const MARKS: { readonly [health in Health]: string } = {
  good: '<circle cx="8" cy="8" r="6.2"/><path d="M5.2 8.3l2 2 3.8-4.3"/>',
  watch:
    '<circle cx="8" cy="8" r="6.2"/><path d="M8 4.6v4.2"/><circle cx="8" cy="11.2" r="0.5" fill="currentColor"/>',
  bad: '<path d="M8 1.8l6.6 11.8H1.4z"/><path d="M8 6.2v3.4"/><circle cx="8" cy="11.6" r="0.5" fill="currentColor"/>',
  unread: '<circle cx="8" cy="8" r="6.2" stroke-dasharray="2.2 2.2"/>',
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

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** The mark `health` is drawn with, its word said only to a screen reader. */
function mark(health: Health): string {
  return `<svg class="state ${health}" viewBox="0 0 16 16" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">${MARKS[health]}</g></svg>`;
}

/** How `health` is said to a screen reader, after a name. */
function spoken(health: Health): string {
  return `<span class="sr">, ${WORDS[health].toLowerCase()}</span>`;
}

/** Where a thing's link goes: its home's address, or the mycelium site. */
function home(thing: Thing): string {
  return `../${thing.address ?? "mycelium/"}`;
}

/** The headline, its counts each a link to what it counts. */
function headlineOf(said: Headline): string {
  const looks =
    said.looks === undefined ? undefined : `<a href="#parts" data-to="look"><em>${escaped(said.looks)}</em></a>`;
  const unread =
    said.unread === undefined ? undefined : `<a href="#parts" data-to="unread">${escaped(said.unread)}</a>`;
  const clauses = [looks, unread].filter((clause) => clause !== undefined);
  const rest = clauses.length === 0 ? "all good" : clauses.join(" and ");
  return `<a href="#parts" data-to="part">${escaped(said.parts)}</a>, ${rest}.`;
}

/** A row the keys step through, in the part `part`. */
function row(part: Part, href: string, key: string, inner: string, health?: Health): string {
  const gutter = health === undefined ? "" : mark(health);
  return `<li><a class="row" data-row="${escaped(key)}" data-part="${escaped(part.part)}" href="${escaped(href)}"><span class="gut">${gutter}</span><span class="line">${inner}</span></a></li>`;
}

/** A machine's row: its name in the mono a name is set in, then what it says. */
function machineRow(part: Part, thing: Thing): string {
  const inner = `<code class="own">${escaped(thing.name)}</code>${spoken(thing.health)} ${escaped(thing.says)}`;
  return row(part, home(thing), `machine:${thing.name}`, inner, thing.health);
}

/** What in `part` is worth a look, as rows: its machines, or its side. */
function itemsOf(part: Part): string[] {
  const site = `../${part.deeper.site}`;
  if (part.things.length > 0) {
    const drawn = part.things.slice(0, MACHINES_DRAWN).map((thing) => machineRow(part, thing));
    const more = part.things.length - MACHINES_DRAWN;
    if (more > 0) {
      const words = `and ${more} more ${more === 1 ? "machine" : "machines"}`;
      drawn.push(row(part, site, `${part.part}:more`, escaped(words)));
    }
    return drawn;
  }
  if (part.health === "unread") {
    return [];
  }
  return part.side.map((item, at) => row(part, site, `${part.part}:${at}`, escaped(item)));
}

/** The ways deeper into `part`: its command, which y copies, and where it is in the launcher. */
function deeperOf(part: Part): string {
  const command = escaped(part.deeper.command);
  return `<p class="deeper"><code>${command}</code><button type="button" class="copy" data-command="${command}" aria-label="Copy ${command}">copy <kbd>y</kbd></button><span class="launcher"><span class="sr">In the launcher: </span>${escaped(part.deeper.launcher)}</span></p>`;
}

/** `part`, drawn: its heading, what it holds, what is worth a look and the ways deeper. */
function partOf(part: Part): string {
  const name = escaped(named(part.part));
  const holds =
    part.health === "unread"
      ? `<p class="holds unread">${escaped(part.says)}</p>`
      : `<p class="holds">${escaped(part.holds ?? "")}</p>`;
  const items = itemsOf(part);
  const list = items.length === 0 ? "" : `<ul class="items">${items.join("")}</ul>`;
  return `<section class="part ${part.health}" id="part-${escaped(part.part)}" aria-labelledby="name-${escaped(part.part)}">
  <h2 class="name" id="name-${escaped(part.part)}"><a class="row" data-row="part:${escaped(part.part)}" data-part="${escaped(part.part)}" data-health="${part.health}" href="../${escaped(part.deeper.site)}"><span class="gut">${mark(part.health)}</span><span class="line">${name}${spoken(part.health)}</span></a></h2>
  ${holds}
  ${list}
  ${deeperOf(part)}
</section>`;
}

/** Every part of `system`, in its order. */
function partsOf(system: System): string {
  return system.parts.map((part) => partOf(part)).join("\n");
}

/** The legend: each mark beside its word, said once for the whole page. */
function legend(): string {
  const healths: readonly Health[] = ["good", "watch", "bad", "unread"];
  return healths
    .map((health) => `<span class="key">${mark(health)}${WORDS[health]}</span>`)
    .join("");
}

export { escaped, headlineOf, home, legend, partsOf };
