// The words a page says how its feed stands in, one line above what it
// draws, so a reading that has stopped moving never passes for now. Each
// state is said in words, never by a colour alone; the page's element takes
// the state as data-state too, for its look.

import type { Standing } from "./feed.ts";

/** `at` (milliseconds since the epoch) as the hour and minute, locally. */
function clockTime(at: number): string {
  const when = new Date(at);
  const hours = String(when.getHours()).padStart(2, "0");
  const minutes = String(when.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

/** How `standing` is said, in a sentence. */
function said(standing: Standing): string {
  const since = clockTime(standing.since);
  const then = "This is how it stood then.";
  switch (standing.state) {
    case "connecting":
      return "Reaching its daemon.";
    case "live":
      return `Live since ${since}.`;
    case "stale":
      return `Not live since ${since}: the sites' server can't be reached. ${then} Trying again.`;
    case "broken":
      return `Not live since ${since}: ${standing.reason ?? "its daemon didn't answer"}. ${then}`;
    default:
      return standing.state satisfies never;
  }
}

/** Says `standing` in `element`, and marks it with the state. */
function show(element: HTMLElement, standing: Standing): void {
  element.textContent = said(standing);
  element.dataset.state = standing.state;
}

export { clockTime, said, show };
