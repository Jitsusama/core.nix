// The realm's balance as the map says it, from the balance service's
// readings (the node's `balance` kind): each machine's node serves its own,
// so the map reads the newest that is still fresh, and each machine's share
// of the work beside it.

import { type Fields, type Held, isObject, text } from "../../local-sites/kit/records.ts";

/**
 * How old the balance's latest reading may be and still be drawn. The
 * service reads each minute, so three minutes is two missed: past it, the
 * service has stopped and its last reading would only look current.
 */
const BALANCE_FRESH_MS = 180_000;

/** The realm's balance as the map says it, from the service's latest reading. */
interface Balance {
  readonly measures: unknown;
  readonly at: string;
  readonly imbalance: unknown;
  readonly held: unknown;
}

/**
 * The newest balance reading held, when it is fresh at `at`: each machine's node
 * serves its own, so the newest wins, and one dated further ahead than a reading
 * may age is a wrong clock, never fresh.
 */
function latestReading(held: Held | undefined, at: number): Fields | undefined {
  let latest: Fields | undefined;
  let latestAt = Number.NEGATIVE_INFINITY;
  for (const record of held?.all() ?? []) {
    const when = Date.parse(text(record, "at") ?? "");
    if (when > latestAt) {
      latest = record;
      latestAt = when;
    }
  }
  return Math.abs(at - latestAt) <= BALANCE_FRESH_MS ? latest : undefined;
}

/** Each machine's share of the realm's work in `reading`, by its name. */
function sharesOf(reading: Fields | undefined): Map<unknown, Fields> {
  const shares = new Map<unknown, Fields>();
  const machines = reading?.["machines"];
  for (const load of Array.isArray(machines) ? machines : []) {
    if (isObject(load)) {
      shares.set(load["machine"], {
        busy: load["busy"],
        idle: load["idle"],
        busy_per_core: load["busy_per_core"],
        load: load["load"],
        over: load["over"],
        room: load["room"],
      });
    }
  }
  return shares;
}

/** The realm's balance from `reading`: its definition, time, spread and held flags. */
function balanceOf(reading: Fields | undefined): Balance | null {
  const at = reading === undefined ? undefined : text(reading, "at");
  if (reading === undefined || at === undefined) {
    return null;
  }
  return {
    measures: reading["measures"],
    at,
    imbalance: reading["imbalance"] ?? null,
    held: reading["held"] ?? [],
  };
}

export type { Balance };
export { BALANCE_FRESH_MS, balanceOf, latestReading, sharesOf };
