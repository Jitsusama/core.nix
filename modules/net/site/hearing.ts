// What net's watches told the page beside their records: what each couldn't
// read, and whether each kind's records have come since its feed last stood
// other than live. The server says live before the records that bring it, so
// what a page held from before is current only once they've come; and the
// kit's feed says what it couldn't read of an answer before handing over the
// rest of it in the same turn, so those records mustn't clear what was lost
// from them. Only a whole answer makes a watch's records complete again
// (the kit's records.ts), so only one clears what it couldn't read.

/** How the lede hears the feeds: whether all are current, and how much went unread. */
interface Heard {
  readonly fresh: boolean;
  readonly unheard: number;
}

/** The watches whose unread answers may have been worth a look. */
const ASKING = ["concern", "source"];

/**
 * How the kit's feed begins what it says of an answer to another ask, which
 * was never this watch's and so can hide none of its records.
 */
const STRAY = "an answer to no ask of this page";

class Hearing {
  readonly #kinds: readonly string[];
  // The last thing each watch couldn't read, said until a whole answer comes.
  readonly #problems = new Map<string, string>();
  // The watches whose unread lines may have held a record, until a whole answer comes.
  readonly #hiding = new Set<string>();
  readonly #raising = new Set<string>();
  readonly #fresh = new Set<string>();

  constructor(kinds: readonly string[]) {
    this.#kinds = kinds;
  }

  /** The watch of `kind` handed over records, all of them when `whole`. */
  moved(kind: string, whole: boolean): void {
    this.#fresh.add(kind);
    if (whole && !this.#raising.has(kind)) {
      this.#problems.delete(kind);
      this.#hiding.delete(kind);
    }
  }

  /** The watch of `kind` now stands as `state`. */
  stands(kind: string, state: string): void {
    if (state !== "live") {
      this.#fresh.delete(kind);
    }
  }

  /** The watch of `kind` couldn't read `what`. */
  problem(kind: string, what: string): void {
    this.#problems.set(kind, `${kind}: ${what}`);
    if (!what.startsWith(STRAY)) {
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

export type { Heard };
export { Hearing };
