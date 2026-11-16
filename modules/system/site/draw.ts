// The system page's markup: a part's row, the panel of the part the cursor
// is on, and the marks each health is drawn with. Every word read from a
// record is escaped before it reaches the page. The mark's ink says the
// same as the word beside it, so nothing is told by colour alone.

import { type Health, named, type Part, type Thing, WORDS } from "./model.ts";

const MARKS: { readonly [health in Health]: string } = {
  good: '<circle cx="8" cy="8" r="6.2"/><path d="M5.2 8.3l2 2 3.8-4.3"/>',
  watch:
    '<circle cx="8" cy="8" r="6.2"/><path d="M8 4.6v4.2"/><circle cx="8" cy="11.2" r="0.5" fill="currentColor"/>',
  bad: '<path d="M8 1.8l6.6 11.8H1.4z"/><path d="M8 6.2v3.4"/><circle cx="8" cy="11.6" r="0.5" fill="currentColor"/>',
  unread: '<circle cx="8" cy="8" r="6.2" stroke-dasharray="2.2 2.2"/>',
};

/** Each site a part goes deeper into, by its title. */
const SITES: { readonly [site: string]: string } = {
  "mycelium/": "Your mycelium, now",
  "network/": "Your home network, now",
  "code/": "Code",
};

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

/** The mark `health` is drawn with. */
function mark(health: Health): string {
  return `<svg class="state ${health}" viewBox="0 0 16 16" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">${MARKS[health]}</g></svg>`;
}

/** How `health` is said, marked for its look. */
function word(health: Health): string {
  return `<span class="word ${health}">${WORDS[health]}</span>`;
}

/** The title of the site `part` goes deeper into. */
function siteName(part: Part): string {
  return SITES[part.deeper.site] ?? part.deeper.site;
}

/** Where a thing's link goes: its home's address, or the mycelium site. */
function home(thing: Thing): string {
  return `../${thing.address ?? "mycelium/"}`;
}

/** `part`'s row in the list of parts. */
function partRow(part: Part): string {
  return `<div role="option" id="part-${escaped(part.part)}" data-part="${escaped(part.part)}" aria-selected="false">
  ${mark(part.health)}
  <span class="name">${escaped(named(part.part))}</span>
  ${word(part.health)}
  <span class="says">${escaped(part.says)}</span>
</div>`;
}

/** The row of `thing`, the cursor's when `current`. */
function thingRow(thing: Thing, current: boolean): string {
  return `    <li><a class="row" href="${escaped(home(thing))}" data-name="${escaped(thing.name)}" tabindex="${current ? 0 : -1}" aria-current="${current}">
      ${mark(thing.health)}
      <code class="own">${escaped(thing.name)}</code>
      ${word(thing.health)}
      <span class="says">${escaped(thing.says)}</span>
    </a></li>`;
}

/** The things of `part`, the cursor on `inner`'s, or the first's. */
function thingsOf(part: Part, inner: string | null): string {
  if (part.things.length === 0) {
    return "";
  }
  const title = part.things.length === 1 ? "Its Machine" : `Its ${part.things.length} Machines`;
  const rows = part.things.map((thing, index) =>
    thingRow(thing, thing.name === inner || (inner === null && index === 0)),
  );
  return `<section aria-labelledby="things-title" class="things">
  <h4 id="things-title">${title}</h4>
  <ul class="tl" id="things">
${rows.join("\n")}
  </ul>
</section>`;
}

/** The panel of `part`, the part the cursor is on. */
function panelOf(part: Part, inner: string | null): string {
  const command = escaped(part.deeper.command);
  return `
<div class="title">${mark(part.health)}<h3>${escaped(named(part.part))}</h3>${word(part.health)}</div>
<p class="lead">${escaped(part.says)}</p>
${thingsOf(part, inner)}
<section aria-labelledby="deeper-title" class="deeper">
  <h4 id="deeper-title">Deeper</h4>
  <dl class="facts">
    <div><dt>On its site</dt><dd><a href="../${escaped(part.deeper.site)}">${escaped(siteName(part))}</a></dd></div>
    <div><dt>In a terminal</dt><dd><code>${command}</code> <button type="button" class="copy" data-copy="${command}">Copy <kbd>y</kbd></button></dd></div>
    <div><dt>In the launcher</dt><dd>${escaped(part.deeper.launcher)}</dd></div>
  </dl>
</section>
<p class="act">Press <kbd>Return</kbd> to open ${escaped(inner ?? siteName(part))}.</p>`;
}

export { escaped, home, panelOf, partRow, siteName };
