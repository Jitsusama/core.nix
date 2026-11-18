// The forage page's markup: the forest floor cut through, a cell's row with
// its marks in the gutter, and the panel of the cell the cursor is on. On
// the floor each cell is a compartment along its hypha, whole when its
// purpose is written and hollow when it isn't, so a cell without purpose
// is told by its shape and never by colour alone; a cell started by
// another, or split from it, branches below it; a dotted arc joins a cell
// to one serving a need of its; and a cap rises over a cell whose plans have
// landed steps, its gills lit as they land. What a cell's quest waits on
// its person for leads its panel and is counted in its row. Every word
// read from a record is escaped before it reaches the page, and how far a
// plan has come is said in words beside its marks.

import { clockTime } from "../../local-sites/kit/status.ts";
import {
  type Document,
  type Forage,
  grewFrom,
  type Join,
  joined,
  keeps,
  steps,
  type Wait,
} from "./model.ts";

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** Each month as a need's since says it, when the need wasn't taken today. */
const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
] as const;

/** Each stage of a document as the page says it. */
const STAGES: { readonly [stage: string]: string } = {
  think: "Thinking",
  draft: "Drafting",
  build: "Building",
  concluded: "Done",
  retired: "Put down",
};

/** The floor's measures, in its own units: a cell's room, the ground and each branch's drop. */
const FLOOR = {
  room: 30,
  margin: 24,
  ground: 52,
  hypha: 78,
  drop: 20,
  gills: 5,
  /** The most cells drawn on the floor; the list holds every one. */
  most: 48,
  /** How much larger than its own units the floor is drawn, where the page has room. */
  scale: 1.5,
} as const;

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** How `stage` is said. */
function staged(stage: string): string {
  return STAGES[stage] ?? stage;
}

/** The command that says `forage` in a terminal. */
function command(forage: Forage): string {
  return `mycelium forage ${forage.cell}`;
}

/** Where the `mycelium` program's repository is at home, on the Code site. */
const MYCELIUM_HOME = "../code/#repo=Jitsusama%2Fmycelium";

/** How many of the cells a cell started its panel names before counting the rest. */
const MOST_STARTED = 6;

/** Where the map opens on `forage`'s cell. */
function onMap(forage: Forage): string {
  return `../mycelium/#cell=${encodeURIComponent(forage.cell)}`;
}

/** The purpose as the page says it, or that none is written. */
function purposeOf(forage: Forage): string {
  return forage.purpose === "" ? "No purpose written yet." : forage.purpose;
}

/**
 * Marks words the quest `quest` wrote, shown as written, so a page check
 * reads them as its words rather than the page's: a purpose can quote a
 * path, an id or a repository it never links. A record naming no quest
 * still holds a quest's words, so it says so.
 */
function saidBy(quest: string): string {
  return ` data-said-by="${escaped(quest === "" ? "its quest" : quest)}"`;
}

/** Where `forage`'s purpose is its quest's own words, the mark saying so. */
function purposeSaid(forage: Forage): string {
  return forage.purpose === "" ? "" : saidBy(forage.quest);
}

/** How far `ticked` of `all` steps have come, said, or nothing with no steps. */
function come(ticked: number, all: number): string {
  if (all === 0) {
    return "";
  }
  return `${ticked} of ${all} ${all === 1 ? "step" : "steps"} done`;
}

/** How many of a cap's gills are lit for `ticked` of `all` steps. */
function lit(ticked: number, all: number): number {
  return all === 0 ? 0 : Math.round((ticked / all) * FLOOR.gills);
}

/** A compartment's mark for the gutter: whole or hollow, as on the floor. */
function compartment(forage: Forage): string {
  const kind = forage.purpose === "" ? "hollow" : "whole";
  return `<svg class="cm" viewBox="0 0 20 10" aria-hidden="true"><rect class="${kind}" x="1" y="1" width="18" height="8" rx="4"/></svg>`;
}

/** `forage`'s row in the list of cells, set in by how deep it was started. */
function forageRow(forage: Forage): string {
  const { ticked, all } = steps(forage);
  const quiet = forage.purpose === "" ? " quiet" : "";
  const count = all === 0 ? "" : `${ticked}/${all}`;
  return `<div role="option" id="cell-${escaped(forage.cell)}" data-cell="${escaped(forage.cell)}" aria-selected="false" style="--depth:${forage.depth}">
  <span class="mk">${compartment(forage)}</span>
  <code class="name">${escaped(forage.cell)}</code>
  <span class="come" aria-label="${escaped(come(ticked, all))}">${escaped(count)}</span>
  <span class="line">
    <span class="purpose${quiet}"${purposeSaid(forage)}>${escaped(purposeOf(forage))}</span>
    <span class="keeps">${escaped(keeps(forage))}</span>${waitsCounted(forage)}
  </span>
</div>`;
}

/** How many things `forage`'s quest waits on its person for, for its row; nothing when none. */
function waitsCounted(forage: Forage): string {
  const count = forage.waits.length;
  if (count === 0) {
    return "";
  }
  const things = count === 1 ? "one thing" : `${count} things`;
  return `\n    <span class="waits">Waits on you for ${things}</span>`;
}

/** About how long a wait takes, in whole minutes, or nothing when it doesn't say so. */
function takes(minutes: number | undefined): string | undefined {
  const whole = minutes === undefined ? 0 : Math.round(minutes);
  if (whole < 1) {
    return undefined;
  }
  return whole === 1 ? "About a minute" : `About ${whole} minutes`;
}

/**
 * The row of `wait`, of quest `quest`, in the panel: what is asked and why
 * in the quest's own words, then how long it takes, when it's for and
 * whether it can be done now.
 */
function waitRow(wait: Wait, quest: string): string {
  const said = [
    takes(wait.minutes),
    wait.when === undefined ? undefined : `<span${saidBy(quest)}>${escaped(wait.when)}</span>`,
    wait.ready ? "Ready now" : "Not ready yet",
  ].filter((each) => each !== undefined);
  const why =
    wait.why === "" ? "" : `\n      <span class="why"${saidBy(quest)}>${escaped(wait.why)}</span>`;
  return `    <li class="doc">
      <span class="named"${saidBy(quest)}>${escaped(wait.title)}</span>${why}
      <span class="said">${said.join(" · ")}</span>
    </li>`;
}

/** What `forage`'s quest waits on its person for, or nothing when it waits on nothing. */
function waitsOf(forage: Forage): string {
  if (forage.waits.length === 0) {
    return "";
  }
  return `<section aria-labelledby="waits-title" class="waits">
  <h4 id="waits-title">What It Waits On You For</h4>
  <ul class="tl">
${forage.waits.map((wait) => waitRow(wait, forage.quest)).join("\n")}
  </ul>
</section>
`;
}

/** Where each cell drawn sits on the floor, by its place in the list and its depth. */
function placed(
  forages: readonly Forage[],
): readonly { readonly forage: Forage; readonly x: number; readonly y: number }[] {
  return forages.slice(0, FLOOR.most).map((forage, at) => ({
    forage,
    x: FLOOR.margin + at * FLOOR.room + FLOOR.room / 2,
    y: FLOOR.hypha + Math.min(forage.depth, 3) * FLOOR.drop,
  }));
}

/** The cap over a cell whose plans have landed steps, its gills lit as they land. */
function cap(x: number, y: number, ticked: number, all: number): string {
  const top = FLOOR.ground - 34;
  const brim = FLOOR.ground - 14;
  const on = lit(ticked, all);
  const gills = Array.from({ length: FLOOR.gills }, (_, at) => {
    const gx = x - 8 + at * 4;
    return `<path class="${at < on ? "gill" : "gill dim"}" d="M${gx} ${brim}v-4"/>`;
  }).join("");
  return `<path class="stalk" d="M${x} ${y - 6}V${brim}"/><path class="cap" d="M${x - 11} ${brim}C${x - 11} ${top + 6} ${x - 6} ${top} ${x} ${top}S${x + 11} ${top + 6} ${x + 11} ${brim}Z"/>${gills}`;
}

/**
 * The floor cut through: the ground, the hyphae under it and each cell a
 * compartment along them, `cursor`'s ringed. Each top cell starts a hypha
 * of its own on the main line; a cell started by another branches down
 * from it. `substrate` names the machine the cells stand on.
 */
function floorOf(forages: readonly Forage[], cursor: string | null, substrate: string): string {
  const cells = placed(forages);
  const width = Math.max(FLOOR.margin * 2 + cells.length * FLOOR.room, 320);
  const deepest = cells.reduce((most: number, each) => Math.max(most, each.y), FLOOR.hypha);
  const height = deepest + 40;
  const at = new Map(cells.map((each) => [each.forage.cell, each]));
  const here = new Set(forages.map((each) => each.cell));
  const main = `<path class="hypha" d="M${FLOOR.margin / 2} ${FLOOR.hypha}H${width - FLOOR.margin / 2}"/>`;
  const branches = cells
    .map(({ forage, x, y }) => {
      const by = grewFrom(forage, here);
      const from = by === undefined ? undefined : at.get(by);
      if (from === undefined || forage.depth === 0) {
        return "";
      }
      return `<path class="hypha" d="M${from.x} ${from.y}C${from.x} ${y} ${x - FLOOR.room / 2} ${y} ${x} ${y}"/>`;
    })
    .join("");
  const joins = joined(cells.map((each) => each.forage))
    .map(({ from, to }) => {
      const one = at.get(from);
      const other = at.get(to);
      if (one === undefined || other === undefined) {
        return "";
      }
      // It bows below both cells and is dotted, so it never reads as a branch.
      const low = Math.max(one.y, other.y) + FLOOR.drop;
      return `<path class="join" d="M${one.x} ${one.y + 6}Q${(one.x + other.x) / 2} ${low} ${other.x} ${other.y + 6}"/>`;
    })
    .join("");
  const caps = cells
    .map(({ forage, x, y }) => {
      const { ticked, all } = steps(forage);
      return ticked > 0 ? cap(x, y, ticked, all) : "";
    })
    .join("");
  const compartments = cells
    .map(({ forage, x, y }) => {
      const kind = forage.purpose === "" ? "hollow" : "whole";
      const on = forage.cell === cursor ? " on" : "";
      return `<rect class="${kind}${on}" data-cell="${escaped(forage.cell)}" x="${x - 11}" y="${y - 6}" width="22" height="12" rx="6"/>`;
    })
    .join("");
  const more =
    forages.length > cells.length
      ? `<text class="note" x="${width - FLOOR.margin}" y="${height - 12}" text-anchor="end">and ${forages.length - cells.length} more in the list</text>`
      : "";
  return `<svg viewBox="0 0 ${width} ${height}" style="--w:${width * FLOOR.scale}px" aria-hidden="true">
<path class="ground" d="M0 ${FLOOR.ground}H${width}"/>
${main}${branches}${joins}${caps}${compartments}
<text class="substrate" x="${FLOOR.margin / 2}" y="${height - 12}">${escaped(substrate)}</text>${more}
</svg>`;
}

/**
 * The row of `document`, kept by quest `quest`, in the panel, its gills
 * saying how far it has come.
 */
function documentRow(document: Document, quest: string): string {
  const all = document.ticked + document.open;
  const on = lit(document.ticked, all);
  const gills =
    all === 0
      ? ""
      : `<svg class="gills" viewBox="0 0 24 8" aria-hidden="true">${Array.from(
          { length: FLOOR.gills },
          (_, at) => `<path class="${at < on ? "gill" : "gill dim"}" d="M${2 + at * 5} 8V1"/>`,
        ).join("")}</svg>`;
  return `    <li class="doc">
      <span class="named"${document.title === document.id ? "" : saidBy(quest)}>${escaped(document.title)}</span>
      <span class="said">${escaped(document.kind)} · ${escaped(staged(document.stage))}</span>
      <span class="steps">${gills}<span>${escaped(come(document.ticked, all))}</span></span>
    </li>`;
}

/** The documents `forage` keeps, or that it keeps none yet. */
function documentsOf(forage: Forage): string {
  if (forage.documents.length === 0) {
    return `<section aria-labelledby="docs-title" class="docs">
  <h4 id="docs-title">What It Keeps</h4>
  <p class="none">No plans, research, briefs or reports yet.</p>
</section>`;
  }
  return `<section aria-labelledby="docs-title" class="docs">
  <h4 id="docs-title">What It Keeps</h4>
  <ul class="tl">
${forage.documents.map((document) => documentRow(document, forage.quest)).join("\n")}
  </ul>
</section>`;
}

/** The cell that started `forage`, linked to that cell's row when it is drawn here. */
function startedBy(forage: Forage, here: ReadonlySet<string>): string {
  const by = forage.startedBy;
  if (by === undefined) {
    return "Nothing; it started on its own";
  }
  return cellOrAway(by, here);
}

/**
 * The cells drawn here that `forage` started, each linked to its row, as
 * "Started by" links back. Past `MOST_STARTED` the rest are counted rather
 * than named, since a wall of links would bury the facts below; one more than
 * that is named, since "and 1 more" says less than its name. The panel sets it
 * in one span, so it reads as a sentence rather than a row of the fact's
 * flexed items, and ties each comma to the address before it, so no line
 * starts with one.
 */
function cellsStarted(
  started: readonly string[],
  name: (cell: string) => string = cellLink,
): string {
  if (started.length === 0) {
    return "None here yet";
  }
  const count = started.length === 1 ? "One cell" : `${started.length} cells`;
  const named = started.length > MOST_STARTED + 1 ? MOST_STARTED : started.length;
  const links = started.slice(0, named).map(name);
  const rest = started.length - links.length;
  const last = rest === 0 ? links.pop() : `${rest} more`;
  const before = links
    .map((link, at) => (at < links.length - 1 ? `<span class="tie">${link},</span>` : link))
    .join(" ");
  return before === "" ? `${count}: ${last}` : `${count}: ${before} and ${last}`;
}

/**
 * `cell` linked to its row when it is drawn here, else said not to stand
 * here, since it may have ended or stand on another machine.
 */
function cellOrAway(cell: string, here: ReadonlySet<string>): string {
  return here.has(cell)
    ? cellLink(cell)
    : `<span><code>${escaped(cell)}</code>, not standing here</span>`;
}

/**
 * When `since` was, by the clock when it was today and by the day and
 * clock before, so a need held for days never reads as this morning's;
 * nothing when it isn't a time. `now` is when the page draws.
 */
function sinceOf(since: string | undefined, now: number): string {
  const at = since === undefined ? Number.NaN : Date.parse(since);
  if (Number.isNaN(at)) {
    return "";
  }
  const when = new Date(at);
  const today = new Date(now).toDateString() === when.toDateString();
  const day = today ? "" : `${when.getDate()} ${MONTHS[when.getMonth()]}, `;
  return `, since <time datetime="${when.toISOString()}">${day}${clockTime(at)}</time>`;
}

/**
 * The needs `joins` holds, one line each: the cell at the other end, what
 * the need was about in the asking cell's own words, and since when.
 * `asker` names who said each need's words: the cell at the other end of
 * one this cell serves, or this cell's own quest for one it relies on.
 */
function needsOf(
  joins: readonly Join[],
  here: ReadonlySet<string>,
  asker: (join: Join) => string,
): string {
  // As with the cells a cell started, a long list names six and counts the rest.
  const named = joins.length > MOST_STARTED + 1 ? MOST_STARTED : joins.length;
  const now = Date.now();
  const lines = joins.slice(0, named).map((join) => {
    const need =
      join.need === "" ? "" : `, for <span${saidBy(asker(join))}>${escaped(join.need)}</span>`;
    const since = sinceOf(join.since, now);
    return `<span class="need">${cellOrAway(join.cell, here)}${need}${since}</span>`;
  });
  const rest = joins.length - named;
  const more = rest === 0 ? "" : `<span class="need">and ${rest} more</span>`;
  return lines.join("") + more;
}

/**
 * The facts of how `forage` joins other forages, beside who started it:
 * the cell it split from, the cells it split into, and the needs it serves
 * and relies on. Each is said only when it holds, since most cells split
 * from none and a need fades once it's done.
 */
function joinsOf(forage: Forage, here: ReadonlySet<string>): string {
  const facts: string[] = [];
  if (forage.splitFrom !== undefined) {
    facts.push(`<div><dt>Split from</dt><dd>${cellOrAway(forage.splitFrom, here)}</dd></div>`);
  }
  if (forage.splitInto.length > 0) {
    const into = cellsStarted(forage.splitInto, (cell) => cellOrAway(cell, here));
    facts.push(`<div><dt>Split into</dt><dd><span>${into}</span></dd></div>`);
  }
  if (forage.serves.length > 0) {
    const said = needsOf(forage.serves, here, (join) => join.cell);
    facts.push(`<div><dt>Serves</dt><dd class="needs">${said}</dd></div>`);
  }
  if (forage.relies.length > 0) {
    const said = needsOf(forage.relies, here, () => forage.quest);
    facts.push(`<div><dt>Relies on</dt><dd class="needs">${said}</dd></div>`);
  }
  return facts.map((fact) => `\n    ${fact}`).join("");
}

/** `cell`'s address, linked to its row, where a press moves the cursor to it. */
function cellLink(cell: string): string {
  const address = escaped(cell);
  return `<a href="#cell=${encodeURIComponent(cell)}" data-cell="${address}"><code>${address}</code></a>`;
}

/**
 * The panel of `forage`, the cell the cursor is on; `here` holds every cell
 * drawn, and `started` the cells it started among them.
 */
function panelOf(forage: Forage, here: ReadonlySet<string>, started: readonly string[]): string {
  const said = escaped(command(forage));
  const quiet = forage.purpose === "" ? " quiet" : "";
  return `
<p class="eyebrow">the cell open</p>
<h3><code>${escaped(forage.cell)}</code></h3>
<p class="lead${quiet}"${purposeSaid(forage)}>${escaped(purposeOf(forage))}</p>
${waitsOf(forage)}${documentsOf(forage)}
<section aria-labelledby="ties-title" class="ties">
  <h4 id="ties-title">Where It Grows</h4>
  <dl class="facts">
    <div><dt>Its quest</dt><dd><code>${escaped(forage.quest)}</code>${forage.status === "" ? "" : ` ${escaped(forage.status)}`}</dd></div>
    <div><dt>Started by</dt><dd>${startedBy(forage, here)}</dd></div>
    <div><dt>Started</dt><dd><span>${cellsStarted(started)}</span></dd></div>${joinsOf(forage, here)}
    <div><dt>Under the quest</dt><dd>${forage.parent === undefined ? "None" : `<code>${escaped(forage.parent)}</code>`}</dd></div>
    <div><dt>Stands on</dt><dd>${escaped(forage.machine)}</dd></div>
    <div><dt>In a terminal</dt><dd><code><a href="${MYCELIUM_HOME}">mycelium</a> forage ${escaped(forage.cell)}</code> <button type="button" class="copy" data-copy="${said}">Copy <kbd>y</kbd></button></dd></div>
  </dl>
</section>
<p class="act">Press <kbd>Return</kbd> to open <code>${escaped(forage.cell)}</code> on the map.</p>`;
}

export { command, escaped, floorOf, forageRow, onMap, panelOf, purposeOf };
