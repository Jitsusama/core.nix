// When Code asks the sites again where its cells are home. A cell's site may list it a moment
// after the node says it started, a record may fail to read while the sites' server restarts,
// and a cell may start while an ask is still out, so homes are asked for at once when a cell
// with none is first seen, and again, waiting twice as long each time up to a minute, while any
// cell has none or the last ask didn't read whole. A record that couldn't be read keeps the
// links it gave before, rather than taking them away.

import { type Asked, type Homes, NOWHERE } from "./homes.ts";

/** How an asker reaches out: asking the sites, telling what it learned, and waiting. */
interface Asking {
  readonly ask: () => Promise<Asked>;
  readonly told: (homes: Homes) => void;
  readonly after: (milliseconds: number, then: () => void) => void;
}

// How long an asker waits before asking again, first and at most.
const FIRST_WAIT_MS = 2000;
const LONGEST_WAIT_MS = 60_000;
// How long a read of the sites' files may take before it counts as failed, so one the server
// never answers can't hold every later ask back.
const READ_MS = 10_000;
// What the sites' server answers for a file it doesn't have.
const NOT_FOUND = 404;

/** How the page fetches a file of the sites', as the browser's fetch does. */
type Fetching = (address: string, init: RequestInit) => Promise<Response>;

/** `address` fetched fresh, given up on after a while. */
function fetched(fetching: Fetching, address: string): Promise<Response> {
  return fetching(address, { cache: "no-cache", signal: AbortSignal.timeout(READ_MS) });
}

/**
 * The rows the home lists, as it writes them beside its record, the declared ones as declared;
 * a home from before it listed them apart keeps every row in the record itself, so only a
 * home with no rows file falls back. One failing otherwise rejects, rather than reading the
 * record whose live rows it no longer keeps; so does a home its builder hasn't written yet,
 * which the server answers 503, asked again as this file's opening says.
 */
async function readRows(fetching: Fetching): Promise<unknown> {
  const rows = await fetched(fetching, "../rows.json");
  const read = rows.status === NOT_FOUND ? await fetched(fetching, "../sites.json") : rows;
  if (!read.ok) {
    throw new Error(`the home's rows answered ${read.status}`);
  }
  return read.json();
}

/**
 * A site's homes, as its homes.json says them: undefined for a site that keeps none, and a
 * rejection for one that couldn't be read.
 */
async function readSite(fetching: Fetching, site: string): Promise<unknown> {
  const folder = site.endsWith("/") ? site : `${site}/`;
  const said = await fetched(fetching, `../${folder}homes.json`);
  if (said.status === NOT_FOUND) {
    return undefined;
  }
  if (!said.ok) {
    throw new Error(`${folder}homes.json answered ${said.status}`);
  }
  return said.json();
}

/** Where `fresh` says a thing is home, else where `kept` did. */
function keeping(fresh: Homes, kept: Homes): Homes {
  return (name) => fresh(name) ?? kept(name);
}

/**
 * Asks at once where cells are home, then keeps asking as this file's opening says, telling
 * `asking.told` each time it learns more. Gives back what to tell it of the cells' names each
 * time the node says they moved.
 */
function askerOf(asking: Asking): (names: readonly string[]) => void {
  let homes: Homes = NOWHERE;
  let names: readonly string[] = [];
  // The names there were when homes were last asked for: only a cell with no home that isn't
  // among them asks again at once.
  let askedFor = new Set<string>();
  let out = false;
  let waiting = false;
  let wait = FIRST_WAIT_MS;
  const homeless = (name: string): boolean => homes(name) === undefined;
  const unasked = (): boolean => names.some((name) => homeless(name) && !askedFor.has(name));
  const later = (): void => {
    if (waiting) {
      return;
    }
    waiting = true;
    asking.after(wait, () => {
      waiting = false;
      if (!out) {
        ask();
      }
    });
    wait = Math.min(wait * 2, LONGEST_WAIT_MS);
  };
  const settle = (whole: boolean): void => {
    out = false;
    if (unasked()) {
      wait = FIRST_WAIT_MS;
      ask();
    } else if (!whole || names.some(homeless)) {
      later();
    } else {
      wait = FIRST_WAIT_MS;
    }
  };
  function ask(): void {
    out = true;
    askedFor = new Set(names);
    asking.ask().then(
      (asked) => {
        homes = asked.whole ? asked.homes : keeping(asked.homes, homes);
        asking.told(homes);
        settle(asked.whole);
      },
      () => settle(false),
    );
  }
  ask();
  return (now) => {
    names = now;
    if (!out && unasked()) {
      wait = FIRST_WAIT_MS;
      ask();
    }
  };
}

export { type Asking, askerOf, FIRST_WAIT_MS, type Fetching, LONGEST_WAIT_MS, readRows, readSite };
