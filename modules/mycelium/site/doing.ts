// What things on the map are doing, as the drawer says it: a set of cells by
// its moving ones, the receptors each in full, one cell by how it stands,
// what a receptor fed a cell, and how much of the talk asked or found
// something. Times are said against the reading's own, as when.ts says them.

import { agoWords } from "./when.ts";
import { type Cell, plural } from "./words.ts";

/** How many moving cells a set's doing names before it counts the rest. */
export const DOINGS_SHOWN = 2;

// What a receptor feeds a cell, by its kind: GitHub's review asked of the
// person, or the calls receptor's summary of a call the cell's quest claims.
const FED_WORDS: Readonly<Record<string, string>> = {
  review: "a review asked of you",
  summary: "a call's summary",
};

/** What a receptor fed a cell, by its kind, or plainly something from outside. */
export const fedWords = (kind: string): string =>
  Object.hasOwn(FED_WORDS, kind) ? (FED_WORDS[kind] ?? "") : "something from outside";

/** When the latest of a set of cells' turns ended, if any says. */
export const lastTurnOf = (cells: readonly Cell[]): string | undefined =>
  cells
    .map((s) => s.usage?.last_turn)
    .filter((t): t is string => Boolean(t))
    .sort()
    .at(-1);

// A doing said with its end's full stop or spaces taken off, so a list of
// them reads as one sentence.
const TRAILING = /[.\s]+$/u;
const bare = (said: string): string => said.replace(TRAILING, "");

/** What a set of cells is doing: the moving ones, by what they say. */
export const groupDoing = (cells: readonly Cell[], at: string): string => {
  const moving = cells.filter((s) => s.standing === "moving");
  if (moving.length === 0) {
    const asks = cells.filter((s) => s.standing === "asks").length;
    if (asks > 0) {
      return `Nothing moving; ${plural(asks, "cell")} waiting on you.`;
    }
    const last = lastTurnOf(cells);
    return `Nothing moving${last ? `; the last turn ended ${agoWords(last, at)}` : ""}.`;
  }
  const shown = moving
    .slice(0, DOINGS_SHOWN)
    .map((s) => `${s.short}: ${bare(s.doing || "in a turn")}`);
  const more = moving.length - shown.length;
  const rest = more > 0 ? `; and ${more} more` : "";
  return `${plural(moving.length, "cell")} moving. ${shown.join("; ")}${rest}.`;
};

/**
 * What the receptors are doing, each said in full, since there are only a
 * few and each hears something different.
 */
export const sensesDoing = (cells: readonly Cell[]): string => {
  const reading = cells.filter((s) => s.standing === "moving").length;
  const lead = reading > 0 ? `${plural(reading, "receptor")} reading` : "Nothing reading";
  const each = cells.map((s) => `${s.short}: ${bare(s.doing || "listening")}`);
  return `${lead}. ${each.join("; ")}.`;
};

/** What one cell is doing, by how it stands. */
export const cellDoing = (s: Cell, at: string): string => {
  if (s.standing === "moving") {
    return s.doing || "In a turn now";
  }
  if (s.standing === "asks") {
    return s.wants ? `Asks you: ${s.wants}` : "Waiting on you";
  }
  if (s.standing === "stuck") {
    return "Stuck: paused, waiting to be resumed";
  }
  if (s.standing === "dormant") {
    return "Parked, its place kept";
  }
  const last = s.usage?.last_turn;
  const ended = last ? `; its last turn ended ${agoWords(last, at)}` : "";
  return `Quiet, waiting for its next word${ended}`;
};

/** A message on the mesh, of what the talk line reads. */
export interface Said {
  readonly intent?: string | null | undefined;
}

// What asks or finds something, against what is sent for context only.
const SIGNAL = new Set(["question", "finding", "hold", "pivot"]);

/** Whether a message, by its intent, asked or found something. */
export const asksOrFinds = (x: Said): boolean => SIGNAL.has(x.intent ?? "");

/**
 * Signal against noise: how much of the talk asked or found something,
 * against what was sent for context only. Where the talk is `among` the
 * latest `latest` messages, it says how many of those touch the thing.
 */
export const talkLineWords = (
  messages: readonly Said[],
  latest: number,
  among: boolean,
): string => {
  if (messages.length === 0) {
    return "";
  }
  const said = messages.filter(asksOrFinds).length;
  const rest = messages.length - said;
  const of = among
    ? `Of the latest ${latest} messages, ${plural(messages.length, "touches", "touch")} it`
    : `Of the latest ${plural(messages.length, "message")}`;
  const verb = rest === 1 ? "was" : "were";
  return `${of}: ${said} asked or found something, and ${rest} ${verb} for context only.`;
};
