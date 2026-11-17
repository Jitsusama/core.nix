// What a checkout's git store weighs, as the repos daemon reads it (src/weight.rs): its
// loose objects and packs, about as git's own upkeep reckons them, and whether that upkeep
// is due. Nothing here touches the page, so each reading is tried in Node
// (tests/weight.test.ts).

import { type Fields, number, object } from "../../local-sites/kit/records.ts";

/** A checkout's git store, which its trees share. */
interface Weight {
  /** The room it takes on the disk, loose and packed, in bytes. */
  readonly bytes: number;
  /** How many objects git wrote a file each for, not yet gathered into a pack. */
  readonly loose: number;
  /** Whether that count is reckoned from one folder of 256, so only about. */
  readonly estimated: boolean;
  readonly packs: number;
  /** Whether git's own upkeep, at its thresholds, would gather it now. */
  readonly due: boolean;
}

/** The weight a checkout's record carries, where the daemon could weigh its store. */
function weightOf(record: Fields): Weight | undefined {
  const weight = object(record, "weight");
  if (weight === undefined) {
    return undefined;
  }
  return {
    bytes: (number(weight, "loose_bytes") ?? 0) + (number(weight, "packed_bytes") ?? 0),
    loose: number(weight, "loose") ?? 0,
    estimated: weight["estimated"] === true,
    packs: number(weight, "packs") ?? 0,
    due: weight["due"] === true,
  };
}

// Disks are sold in powers of a thousand, so a size reads as the disk's own does.
const UNITS = ["bytes", "kB", "MB", "GB", "TB"] as const;
const STEP = 1000;
// Under this, a size keeps one decimal, so 2.4 GB isn't said as 2 GB.
const WHOLE = 10;

/** A size in `unit` as it's said: whole bytes, else one decimal under ten once rounded. */
function sizeSaid(size: number, unit: number): string {
  const tenths = size.toFixed(1);
  return unit === 0 || Number(tenths) >= WHOLE ? Math.round(size).toString() : tenths;
}

/** A count of bytes as a person reads it: in the largest unit that leaves it one or more, as
 * it's said, so 999,999 bytes are 1.0 MB rather than 1000 kB. */
function bytes(count: number): string {
  let size = count;
  let unit = 0;
  while (Number(sizeSaid(size, unit)) >= STEP && unit < UNITS.length - 1) {
    size /= STEP;
    unit += 1;
  }
  return `${sizeSaid(size, unit)} ${UNITS[unit] ?? ""}`;
}

/** What a repository's checkouts weigh together, each weighed one counted, and whether any is
 * due upkeep; undefined when none could be weighed. */
function weightOfAll(weights: readonly (Weight | undefined)[]): Weight | undefined {
  const weighed = weights.filter((each): each is Weight => each !== undefined);
  if (weighed.length === 0) {
    return undefined;
  }
  return {
    bytes: weighed.reduce((sum, each) => sum + each.bytes, 0),
    loose: weighed.reduce((sum, each) => sum + each.loose, 0),
    estimated: weighed.some((each) => each.estimated),
    packs: weighed.reduce((sum, each) => sum + each.packs, 0),
    due: weighed.some((each) => each.due),
  };
}

/** A weight in a row's few words: its size, and that upkeep is due when it is. */
function weighedBriefly(weight: Weight | undefined): string {
  if (weight === undefined) {
    return "";
  }
  const size = said(weight);
  return weight.due ? `${size}, upkeep due` : size;
}

/** A weight's size, about where the daemon reckoned its loose objects from one folder. */
function said(weight: Weight): string {
  return weight.estimated ? `about ${bytes(weight.bytes)}` : bytes(weight.bytes);
}

/** A weight as the drawer says it: its size, what it's made of, and why upkeep is due; about,
 * once for both its size and its count, where the daemon reckoned its loose objects. */
function weighedFully(weight: Weight | undefined): string {
  if (weight === undefined) {
    return "";
  }
  const size = said(weight);
  const loose = `${weight.loose.toLocaleString("en-CA")} loose`;
  const packs = `${weight.packs} ${weight.packs === 1 ? "pack" : "packs"}`;
  const made = `${size} in git: ${loose}, ${packs}`;
  return weight.due ? `${made}; git's upkeep is due` : made;
}

export type { Weight };
export { bytes, weighedBriefly, weighedFully, weightOf, weightOfAll };
