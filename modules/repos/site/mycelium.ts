// What Code reads of the mycelium: the cells at work, watched live from the
// machine's node over the sites' feed, so a cell joins its tree the moment
// it starts there, and where each cell is home, from the homes of the site
// whose row in the sites' record claims cells. A machine in no realm has no node, so its feed never answers and
// no cell is drawn, which is the truth there.

import { follow } from "../../local-sites/kit/feed.ts";
import { homeSiteOf, homesOf } from "./address.ts";
import { type Cell, cellsOf } from "./model.ts";

/** What the mycelium has said so far. */
interface Mycelium {
  readonly cells: readonly Cell[];
  /** The home folder, from where a cell works, so a path can say ~ as Joel does. */
  readonly home: string | undefined;
  /** Each cell's home, by its name, when the mycelium's site has homes. */
  readonly homes: ReadonlyMap<string, string>;
}

const HOME_FOLDER = /^\/home\/[^/]+/u;

/** The home folder the first of `cells` with one works under, else `known`. */
function homeOf(cells: readonly Cell[], known: string | undefined): string | undefined {
  const match = cells
    .map((each) => HOME_FOLDER.exec(each.cwd)?.[0])
    .find((each) => each !== undefined);
  return match ?? known;
}

/** Each cell's home, from the homes of the site sites.json says is home to cells. */
async function readHomes(): Promise<ReadonlyMap<string, string>> {
  try {
    const sites = await (await fetch("../sites.json", { cache: "no-cache" })).json();
    const site = homeSiteOf(sites, "cell");
    if (site === undefined) {
      return new Map();
    }
    const homes = await (await fetch(`../${site}homes.json`, { cache: "no-cache" })).json();
    return homesOf(homes, "cell");
  } catch {
    // A mycelium without homes leaves each cell's name unlinked, as it should.
    return new Map();
  }
}

/** Watches the mycelium, telling `heard` each time it has more. */
function watchMycelium(heard: (now: Mycelium) => void): void {
  let now: Mycelium = { cells: [], home: undefined, homes: new Map() };
  // Only what Code draws of a cell is asked for, so its usage and leads
  // moving every turn don't redraw the grove.
  follow(
    { feed: "node", kind: "cell", fields: ["name", "cwd", "title", "standing", "networks"] },
    {
      moved: (held) => {
        const cells = cellsOf({ cells: held.all() });
        now = { ...now, cells, home: homeOf(cells, now.home) };
        heard(now);
      },
      // A node away leaves the cells last heard, as the map does; the page's
      // own standing is the repos daemon's, which is what it's about.
      stands: () => undefined,
      problem: () => undefined,
    },
  );
  readHomes()
    .then((homes) => {
      now = { ...now, homes };
      heard(now);
    })
    .catch(() => undefined);
}

export { type Mycelium, watchMycelium };
