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

/** Where `tell` writes: the element's words and its data, as a test can stand in for. */
interface Shown {
  textContent: string | null;
  readonly dataset: { [name: string]: string | undefined };
}

/**
 * Writes `words` in `element`, and marks it with `state`, writing the words
 * only when they differ from what's there: the element is a live region,
 * and a screen reader says it again each time its words are replaced, even
 * by the same words, so a feed that tells the same state every few seconds
 * would be heard every few seconds. A page that builds its own standing
 * words writes them through this.
 */
function tell(element: Shown, words: string, state: Standing["state"]): void {
  if (element.textContent !== words) {
    element.textContent = words;
  }
  element.dataset["state"] = state;
}

/**
 * Says `standing` in `element`, then `after`, the page's own words on what
 * it couldn't read, through `tell`.
 */
function show(element: Shown, standing: Standing, after = ""): void {
  tell(element, `${said(standing)}${after}`, standing.state);
}

export type { Shown };
export { clockTime, said, show, tell };
