// What Code reads of the mycelium: the cells at work, watched live from the
// machine's node over the sites' feed, so a cell joins its tree the moment
// it starts there, and where each cell is home, by the row among the sites'
// that holds cells. A machine in no realm has no node, so its feed never
// answers and no cell is drawn, which is the truth there.

import { follow } from "../../local-sites/kit/feed.ts";
import { askerOf, readRows, readSite } from "./asking.ts";
import { type Homes, homesFrom, NOWHERE } from "./homes.ts";
import { type Cell, cellsOf } from "./model.ts";

/** What the mycelium has said so far. */
interface Mycelium {
  readonly cells: readonly Cell[];
  /** The home folder, from where a cell works, so a path can say ~ as Joel does. */
  readonly home: string | undefined;
  readonly homes: Homes;
}

/** What the mycelium has said before it has said anything. */
const UNHEARD: Mycelium = { cells: [], home: undefined, homes: NOWHERE };

const HOME_FOLDER = /^\/home\/[^/]+/u;

/** The home folder the first of `cells` with one works under, else `known`. */
function homeOf(cells: readonly Cell[], known: string | undefined): string | undefined {
  const match = cells
    .map((each) => HOME_FOLDER.exec(each.cwd)?.[0])
    .find((each) => each !== undefined);
  return match ?? known;
}

/** Watches the mycelium, telling `heard` each time it has more. */
function watchMycelium(heard: (now: Mycelium) => void): void {
  let now = UNHEARD;
  const fetching = (address: string, init: RequestInit): Promise<Response> => fetch(address, init);
  const seen = askerOf({
    ask: async () =>
      homesFrom(await readRows(fetching), "cell", (site) => readSite(fetching, site)),
    told: (homes) => {
      now = { ...now, homes };
      heard(now);
    },
    after: (milliseconds, then) => {
      setTimeout(then, milliseconds);
    },
  });
  // Only what Code draws of a cell is asked for, so its usage and leads
  // moving every turn don't redraw the grove.
  follow(
    { feed: "node", kind: "cell", fields: ["name", "cwd", "title", "standing", "networks"] },
    {
      moved: (held) => {
        const cells = cellsOf({ cells: held.all() });
        now = { ...now, cells, home: homeOf(cells, now.home) };
        heard(now);
        seen(cells.map((each) => each.name));
      },
      // A node away leaves the cells last heard, as the map does; the page's
      // own standing is the repos daemon's, which is what it's about.
      stands: () => undefined,
      problem: () => undefined,
    },
  );
}

export { type Mycelium, UNHEARD, watchMycelium };
