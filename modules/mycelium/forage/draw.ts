// The forage page's markup: a cell's row, set in under the cell that
// started it, and the panel of the cell the cursor is on, with its purpose,
// its documents and how far each has come. Every word read from a record is
// escaped before it reaches the page. How far a plan has come is said in
// words beside its bar, so nothing is told by the bar alone.

import { type Document, type Forage, keeps, steps } from "./model.ts";

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** Each stage of a document as the page says it. */
const STAGES: { readonly [stage: string]: string } = {
  think: "Thinking",
  draft: "Drafting",
  build: "Building",
  concluded: "Done",
  retired: "Put down",
};

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

/** Where the map opens on `forage`'s cell. */
function onMap(forage: Forage): string {
  return `../mycelium/#cell=${encodeURIComponent(forage.cell)}`;
}

/** The purpose as the page says it, or that none is written. */
function purposeOf(forage: Forage): string {
  return forage.purpose === "" ? "No purpose written yet." : forage.purpose;
}

/** How far `ticked` of `all` steps have come, said, or nothing with no steps. */
function come(ticked: number, all: number): string {
  if (all === 0) {
    return "";
  }
  return `${ticked} of ${all} ${all === 1 ? "step" : "steps"} done`;
}

/** A bar of `ticked` of `all`, its words beside it saying the same. */
function bar(ticked: number, all: number): string {
  if (all === 0) {
    return "";
  }
  const done = Math.round((ticked / all) * 100);
  return `<span class="bar" aria-hidden="true"><span style="inline-size:${done}%"></span></span>`;
}

/** `forage`'s row in the list of cells, set in by how deep it was started. */
function forageRow(forage: Forage): string {
  const { ticked, all } = steps(forage);
  const quiet = forage.purpose === "" ? " quiet" : "";
  return `<div role="option" id="cell-${escaped(forage.cell)}" data-cell="${escaped(forage.cell)}" aria-selected="false" style="--depth:${forage.depth}">
  <code class="name">${escaped(forage.cell)}</code>
  <span class="come">${escaped(come(ticked, all))}</span>
  <span class="purpose${quiet}">${escaped(purposeOf(forage))}</span>
  <span class="keeps">${escaped(keeps(forage))}</span>
</div>`;
}

/** The row of `document` in the panel. */
function documentRow(document: Document): string {
  const all = document.ticked + document.open;
  return `    <li class="doc">
      <span class="kind">${escaped(document.kind)}</span>
      <span class="named">${escaped(document.title)}</span>
      <span class="stage">${escaped(staged(document.stage))}</span>
      <span class="steps">${bar(document.ticked, all)}<span>${escaped(come(document.ticked, all))}</span></span>
    </li>`;
}

/** The documents `forage` keeps, or that it keeps none yet. */
function documentsOf(forage: Forage): string {
  if (forage.documents.length === 0) {
    return `<section aria-labelledby="docs-title" class="docs">
  <h4 id="docs-title">What It Keeps</h4>
  <p>No plans, research, briefs or reports yet.</p>
</section>`;
  }
  return `<section aria-labelledby="docs-title" class="docs">
  <h4 id="docs-title">What It Keeps</h4>
  <ul class="tl">
${forage.documents.map(documentRow).join("\n")}
  </ul>
</section>`;
}

/** Who started `forage`, linked to that cell's row when it is drawn here. */
function startedBy(forage: Forage, here: ReadonlySet<string>): string {
  const by = forage.startedBy;
  if (by === undefined) {
    return "Nothing; it started on its own";
  }
  const name = `<code>${escaped(by)}</code>`;
  return here.has(by)
    ? `<a href="#cell=${encodeURIComponent(by)}" data-cell="${escaped(by)}">${name}</a>`
    : `<span>${name}, on another machine</span>`;
}

/** The panel of `forage`, the cell the cursor is on; `here` holds every cell drawn. */
function panelOf(forage: Forage, here: ReadonlySet<string>): string {
  const said = escaped(command(forage));
  const quiet = forage.purpose === "" ? " quiet" : "";
  return `
<div class="title"><h3><code>${escaped(forage.cell)}</code></h3></div>
<p class="lead${quiet}">${escaped(purposeOf(forage))}</p>
${documentsOf(forage)}
<section aria-labelledby="ties-title" class="ties">
  <h4 id="ties-title">Where It Sits</h4>
  <dl class="facts">
    <div><dt>Its quest</dt><dd><code>${escaped(forage.quest)}</code>${forage.status === "" ? "" : ` ${escaped(forage.status)}`}</dd></div>
    <div><dt>Started by</dt><dd>${startedBy(forage, here)}</dd></div>
    <div><dt>Under the quest</dt><dd>${forage.parent === undefined ? "None" : `<code>${escaped(forage.parent)}</code>`}</dd></div>
    <div><dt>Runs on</dt><dd>${escaped(forage.machine)}</dd></div>
    <div><dt>In a terminal</dt><dd><code>${said}</code> <button type="button" class="copy" data-copy="${said}">Copy <kbd>y</kbd></button></dd></div>
  </dl>
</section>
<p class="act">Press <kbd>Return</kbd> to open <code>${escaped(forage.cell)}</code> on the map.</p>`;
}

export { command, escaped, forageRow, onMap, panelOf, purposeOf };
