// When things happened, said against the reading's own time rather than the
// clock on the wall, so a reading says the same of itself however late it is
// drawn: a clock time, its day where that isn't the reading's, how long ago,
// and how long a lease has left. Times are drawn in the reader's own zone.

const MINUTE_MS = 60_000;
const HOUR_MS = 3_600_000;
const DAY_MS = 86_400_000;
// Words that open with a digit are a clock or a date, which take a verb's word.
const OPENS_WITH_DIGIT = /^\d/u;

// One formatter for each clock, made once: toLocaleString makes a new one
// on every call, which at 10k cells was most of what a tree row cost.
const HOURS = { hour: "2-digit", minute: "2-digit", hour12: false } as const;
const CLOCK = [
  new Intl.DateTimeFormat("en-CA", HOURS),
  new Intl.DateTimeFormat("en-CA", { ...HOURS, second: "2-digit" }),
] as const;
const DAY = new Intl.DateTimeFormat("en-GB", { day: "numeric", month: "short" });

/** A time as the clock reads it, to the second unless `seconds` is false. */
export const clock = (iso: string, seconds = true): string =>
  CLOCK[seconds ? 1 : 0].format(new Date(iso));

const midnight = (t: string): number => new Date(t).setHours(0, 0, 0, 0);

/**
 * A time to the minute, carrying its day when that isn't the reading's, or
 * yesterday's 06:49 reads as this morning's.
 */
export const dayClock = (t: string, at: string): string => {
  const days = Math.round((midnight(t) - midnight(at)) / DAY_MS);
  const hm = clock(t, false);
  if (days === 0) {
    return hm;
  }
  if (days === -1) {
    return `yesterday at ${hm}`;
  }
  if (days === 1) {
    return `tomorrow at ${hm}`;
  }
  return `${DAY.format(new Date(t))} at ${hm}`;
};

/** A time to follow a verb: "at 06:49", "on 3 Nov at 06:49" or "yesterday at 06:49". */
export const atWords = (t: string, at: string): string => {
  const said = dayClock(t, at);
  if (!OPENS_WITH_DIGIT.test(said)) {
    return said;
  }
  return said.includes(" ") ? `on ${said}` : `at ${said}`;
};

/** How long ago a time was, by the minute within the hour and by the clock past it. */
export const agoWords = (iso: string, at: string): string => {
  const ms = Date.parse(at) - Date.parse(iso);
  if (!(ms >= 0) || ms >= HOUR_MS) {
    return atWords(iso, at);
  }
  return ms < MINUTE_MS ? "under a minute ago" : `${Math.round(ms / MINUTE_MS)} min ago`;
};

/** How long a wait has run, as "under a minute" or so many minutes. */
export const minutesWord = (ms: number): string =>
  ms < MINUTE_MS ? "under a minute" : `${Math.round(ms / MINUTE_MS)} min`;

/** How long is left until a time, and when it is, or that it has passed. */
export const leftWords = (iso: string, at: string): string => {
  const ms = Date.parse(iso) - Date.parse(at);
  if (!(ms > 0)) {
    return "its time is up";
  }
  const h = Math.floor(ms / HOUR_MS);
  const min = Math.floor((ms % HOUR_MS) / MINUTE_MS);
  const hours = h > 0 ? `${h} h` : "";
  const minutes = min > 0 || h === 0 ? `${min} min` : "";
  return `${[hours, minutes].filter(Boolean).join(" ")} left, until ${dayClock(iso, at)}`;
};
