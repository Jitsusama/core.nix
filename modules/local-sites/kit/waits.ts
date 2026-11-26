// The asks on a page that most of the time has none: the to-do feed is
// watched with what the page already loads, and the ask rows (asks.ts) and
// what they're read and answered by are fetched only once a look waits.
// A visit with nothing waiting pays one socket and no file for them, so a
// page's first-visit budget holds whether or not it can answer an ask. The
// rows' styles (ask.css) come with them, linked from beside this script, so
// a page links none of its own.
//
// Whether a look waits is read here only from its state, which the rows
// read again as For You does, so a look that lapsed costs at most the files
// and draws no row. Files that fail to load, as while the server restarts,
// are asked for again once the feed is watched again, after waiting as a
// feed waits to try again (feed.ts).

import type { Asking, Following, Made } from "./asks.ts";
import { browserClock, follow, waitBefore } from "./feed.ts";
import { type Held, isObject } from "./records.ts";

/** How the rows' files are loaded: the browser's import of asks.ts. */
type Loading = () => Promise<Pick<typeof import("./asks.ts"), "followAsks">>;

const browserLoad: Loading = () => import("./asks.ts");


/** Whether any look `held` holds says it waits. */
function oneWaits(held: Held): boolean {
  return held.all().some((record) => {
    const look = record["look"];
    return isObject(look) && look["state"] === "waiting";
  });
}

/** Links the ask rows' styles into the page, from the kit this script came from. */
function styled(): void {
  if (typeof document === "undefined") {
    return;
  }
  const sheet = document.createElement("link");
  sheet.rel = "stylesheet";
  sheet.href = new URL("ask.css", import.meta.url).href;
  document.head.append(sheet);
}

/**
 * Draws the asks into `asking`'s rows once one waits, narrowed as the page
 * narrows them meanwhile, and nothing until then, loading the rows' files
 * with `load`.
 */
function asksOnceOneWaits<Item extends Made<Item>>(
  asking: Asking<Item>,
  load: Loading = browserLoad,
): Pick<Following, "narrow"> {
  let address = asking.address ?? null;
  let drawn: Following | null = null;
  let loading = false;
  let styledYet = false;
  let failed = 0;
  const clock = asking.reaching?.clock ?? browserClock;
  const watching = (): { readonly close: () => void } => {
    const watch = follow(
      { feed: "to-do", kind: "look" },
      {
        moved: (held) => {
          if (loading || !oneWaits(held)) {
            return;
          }
          loading = true;
          watch.close();
          if (!styledYet) {
            styledYet = true;
            styled();
          }
          load().then(
            ({ followAsks }) => {
              const { address: _first, ...rest } = asking;
              drawn = followAsks(address === null ? rest : { ...rest, address });
            },
            () => {
              loading = false;
              failed += 1;
              clock.later(waitBefore(failed), watching);
            },
          );
        },
        stands: () => undefined,
        // A record it can't read is one look it can't show; For You still holds it.
        problem: () => undefined,
      },
      asking.reaching ?? {},
    );
    return watch;
  };
  watching();
  return {
    narrow: (next) => {
      address = next;
      drawn?.narrow(next);
    },
  };
}

export { asksOnceOneWaits };
