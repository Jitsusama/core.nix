// What net's watches told the page beside their records: what each couldn't
// read, and whether each kind's records have come since its feed last stood
// other than live. The server says live before the records that bring it, so
// what a page held from before is current only once they've come; and the
// kit's feed says what it couldn't read of an answer before handing over the
// rest of it in the same turn, so those records mustn't clear what was lost
// from them. Only a whole answer makes a watch's records complete again
// (the kit's records.ts), so only one clears what it couldn't read.

import { type Hearing as Heeding, STRAY, type Standing } from "../../local-sites/kit/feed.ts";

/** How the lede hears the feeds: whether all are current, and how much went unread. */
interface Heard {
  readonly fresh: boolean;
  readonly unheard: number;
}

/** The watches whose unread answers may have been worth a look. */
const ASKING = ["concern", "source"];

class Hearing {
  readonly #kinds: readonly string[];
  // The last thing each watch couldn't read, said until a whole answer comes.
  readonly #problems = new Map<string, string>();
  // The watches whose unread lines may have held a record, until a whole answer comes.
  readonly #hiding = new Set<string>();
  readonly #raising = new Set<string>();
  readonly #stopped = new Set<string>();
  readonly #fresh = new Set<string>();

  constructor(kinds: readonly string[]) {
    this.#kinds = kinds;
  }

  /**
   * The watch of `kind` handed over records, all of them when `whole`. A
   * page joining is told them before the state, so they count as current
   * unless told while the watch stood stale or broken, when they're only
   * what it held from before.
   */
  moved(kind: string, whole: boolean): void {
    if (!this.#stopped.has(kind)) {
      this.#fresh.add(kind);
    }
    if (whole && !this.#raising.has(kind)) {
      this.#problems.delete(kind);
      this.#hiding.delete(kind);
    }
  }

  /** The watch of `kind` now stands as `state`. */
  stands(kind: string, state: string): void {
    if (state === "stale" || state === "broken") {
      this.#stopped.add(kind);
    } else {
      this.#stopped.delete(kind);
    }
    if (state !== "live") {
      this.#fresh.delete(kind);
    }
  }

  /** The watch of `kind` couldn't read `what`. */
  problem(kind: string, what: string): void {
    const stray = what.startsWith(STRAY);
    // A stray answer hid no record, so it never takes the place of a line
    // that may have, which the count beside it still counts.
    if (stray && this.#hiding.has(kind)) {
      return;
    }
    this.#problems.set(kind, `${kind}: ${what}`);
    if (!stray) {
      this.#hiding.add(kind);
    }
    this.#raising.add(kind);
    queueMicrotask(() => this.#raising.delete(kind));
  }

  /** What each watch last couldn't read. */
  problems(): string[] {
    return [...this.#problems.values()];
  }

  /** Whether every kind's records are current, and how many asking watches may hide a record. */
  now(): Heard {
    return {
      fresh: this.#kinds.every((kind) => this.#fresh.has(kind)),
      unheard: ASKING.filter((kind) => this.#hiding.has(kind)).length,
    };
  }
}

/** What the page does as one of net's watches is heard. */
interface Page {
  /** Notes that the watch's records have come. */
  readonly told: () => void;
  /** Notes how the watch stands now. */
  readonly stood: (stands: Standing) => void;
  /** Draws the page once in the next frame, its lede and panel with it. */
  readonly drawSoon: () => void;
  /** Says again how net's feeds stand, in the head and the panel. */
  readonly sayStands: () => void;
}

/**
 * What the watch of `kind` does as its feed goes: `hearing` is told
 * first, so the drawing or the saying that follows counts what it heard.
 * Records moved draw the page; a feed that never answers moves nothing,
 * so its standing is said as it comes; and what it couldn't read is said,
 * and counted by the lede and the panel alike, with the next drawing.
 */
function heeding(hearing: Hearing, kind: string, page: Page): Heeding {
  return {
    moved: (_held, change) => {
      page.told();
      hearing.moved(kind, change.whole);
      page.drawSoon();
    },
    stands: (stands) => {
      page.stood(stands);
      hearing.stands(kind, stands.state);
      page.sayStands();
    },
    problem: (what) => {
      hearing.problem(kind, what);
      page.drawSoon();
    },
  };
}

export type { Heard, Page };
export { Hearing, heeding };
