// What Code reads from the mycelium's site beside it: the cells at work, from
// its map, and where each cell is home, from its homes. The map is a file
// its timer rewrites every 20 seconds, so it's read as often, and only while
// the page is seen: idle is nothing.

import { isObject, text } from "../../local-sites/kit/records.ts";
import { type Cell, cellsOf } from "./model.ts";

/** What the mycelium has said so far. */
interface Mycelium {
  readonly cells: readonly Cell[];
  /** The home folder, from where a cell works, so a path can say ~ as Joel does. */
  readonly home: string | undefined;
  /** Each cell's home, by its name, when the mycelium's site has homes. */
  readonly homes: ReadonlyMap<string, string>;
}

const CELLS_EVERY = 20_000;
const HOME_FOLDER = /^\/home\/[^/]+/u;

/** The home folder the first of `cells` with one works under, else `known`. */
function homeOf(cells: readonly Cell[], known: string | undefined): string | undefined {
  const match = cells
    .map((each) => HOME_FOLDER.exec(each.cwd)?.[0])
    .find((each) => each !== undefined);
  return match ?? known;
}

/** The cells at work, from the mycelium's map; none when it can't be read. */
async function readCells(): Promise<readonly Cell[]> {
  try {
    const answer = await fetch("../mycelium/now.json", { cache: "no-cache" });
    return answer.ok ? cellsOf(await answer.json()) : [];
  } catch {
    return [];
  }
}

/** The mycelium's homes for cells, when its row in sites.json says it has them. */
async function readHomes(): Promise<ReadonlyMap<string, string>> {
  try {
    const sites = await (await fetch("../sites.json", { cache: "no-cache" })).json();
    const row =
      isObject(sites) && Array.isArray(sites["sites"])
        ? sites["sites"].find(
            (each: unknown) => isObject(each) && text(each, "name") === "mycelium",
          )
        : undefined;
    if (!(isObject(row) && Array.isArray(row["homes"]))) {
      return new Map();
    }
    const homes = await (await fetch("../mycelium/homes.json", { cache: "no-cache" })).json();
    const things = isObject(homes) && Array.isArray(homes["things"]) ? homes["things"] : [];
    return new Map(
      things.flatMap((each: unknown) => {
        const key = isObject(each) ? text(each, "key") : undefined;
        const address = isObject(each) ? text(each, "address") : undefined;
        return key === undefined || address === undefined ? [] : [[key, address] as const];
      }),
    );
  } catch {
    // A mycelium without homes leaves each cell's name unlinked, as it should.
    return new Map();
  }
}

/** Reads the mycelium while the page is seen, telling `heard` each time it has more. */
function watchMycelium(heard: (now: Mycelium) => void): void {
  let now: Mycelium = { cells: [], home: undefined, homes: new Map() };
  const cellsRead = async (): Promise<void> => {
    const cells = await readCells();
    now = { ...now, cells, home: homeOf(cells, now.home) };
    heard(now);
  };
  let every: number | undefined;
  const whileSeen = (): void => {
    if (document.visibilityState === "visible") {
      cellsRead().catch(() => undefined);
      every ??= globalThis.setInterval(() => {
        cellsRead().catch(() => undefined);
      }, CELLS_EVERY);
    } else if (every !== undefined) {
      globalThis.clearInterval(every);
      every = undefined;
    }
  };
  document.addEventListener("visibilitychange", whileSeen);
  whileSeen();
  readHomes()
    .then((homes) => {
      now = { ...now, homes };
      heard(now);
    })
    .catch(() => undefined);
}

export { type Mycelium, watchMycelium };
