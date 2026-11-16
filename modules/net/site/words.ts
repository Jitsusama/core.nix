// How the network page says figures, times and devices in words. Nothing
// here touches the page, so each is proven in node.

import type { Device } from "./model.ts";

/** The minus sign, rather than a hyphen, before a figure below zero. */
const MINUS = "\u2212";

const MEGA = 1e6;
const GIGA = 1e9;
/** From a hundred gigabytes on, a tenth of one says nothing more. */
const WHOLE_GIGABYTES = 100 * GIGA;
const MILLISECONDS = 1000;
/** Under two days, hours say it; from two on, days do. */
const HOURS_SAID = 48;
/** Under six hours, the minutes past the hour are said too. */
const MINUTES_SAID = 6;
const HOURS_A_DAY = 24;
const SIXTY = 60;

/** A trailing zero, or a point with only zeros after it. */
const TRAILING_ZEROS = /\.0+$|(?<kept>\.\d*?)0+$/u;
const IPV4 = /^\d+\.\d+\.\d+\.\d+$/u;
const TOP_FLOOR = /attic|upstairs|upper|second|top|loft/u;
const LOW_FLOOR = /basement|cellar|lower|garage/u;
const MAIN_FLOOR = /main|ground|first|living|kitchen/u;

/** `n` of `one`, made plural past one. */
function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** A signal in dBm, its minus a minus sign. */
function dbm(n: number): string {
  return `${n < 0 ? MINUS : ""}${Math.abs(n)}`;
}

/** Bytes in decimal units, as a provider bills them. */
function gigabytes(bytes: number): string {
  if (bytes >= WHOLE_GIGABYTES) {
    return `${Math.round(bytes / GIGA)} GB`;
  }
  return bytes >= GIGA ? `${(bytes / GIGA).toFixed(1)} GB` : `${Math.round(bytes / MEGA)} MB`;
}

/** A rate to a tenth, as net and eero give them, without a trailing zero. */
function mbps(n: number): string {
  return (n >= 10 ? n.toFixed(1) : n.toFixed(2)).replace(TRAILING_ZEROS, "$<kept>");
}

/** How long `s` seconds is, in the largest unit or two that say it. */
function lasted(s: number): string {
  if (s < SIXTY) {
    return plural(Math.round(s), "second");
  }
  const m = Math.floor(s / SIXTY);
  if (m < SIXTY) {
    return plural(m, "minute");
  }
  const h = Math.floor(m / SIXTY);
  if (h < HOURS_SAID) {
    const past = m % SIXTY && h < MINUTES_SAID ? `, ${plural(m % SIXTY, "minute")}` : "";
    return plural(h, "hour") + past;
  }
  const d = Math.floor(h / HOURS_A_DAY);
  return plural(d, "day") + (h % HOURS_A_DAY ? `, ${plural(h % HOURS_A_DAY, "hour")}` : "");
}

/** How long before `now` the moment `at` was, both in seconds. */
function ago(at: number, now: number): string {
  return `${lasted(Math.max(0, now - at))} ago`;
}

/** `words` joined as a list is said, the last with "and". */
function listed(words: readonly string[]): string {
  return words.length < 2
    ? words.join("")
    : `${words.slice(0, -1).join(", ")} and ${words.at(-1) ?? ""}`;
}

/** The hour and minute of `at`, in seconds since the epoch, locally. */
function clock(at: number): string {
  return new Date(at * MILLISECONDS).toLocaleTimeString("en-CA", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

const ROLES: { readonly [role: string]: string } = {
  modem: "modem",
  router: "router",
  "access-point": "access point",
  switch: "switch",
  client: "client",
  receiver: "receiver",
};

/** What a device is, by its roles. */
function roleWords(device: Device): string {
  return device.roles.map((role) => ROLES[role] ?? role).join(", ");
}

/** A device's IPv4 addresses. */
function v4(device: Device): string[] {
  return device.addresses.filter((address) => IPV4.test(address));
}

/** `name`'s, as English writes it after a name ending in s. */
function possessive(name: string): string {
  return name.endsWith("s") ? `${name}'` : `${name}'s`;
}

/** A name as an address's fragment: plain letters and digits, dashed. */
function slug(name: string): string {
  return name
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-|-$/gu, "");
}

/** The floors a name can say, top first, and mid-house for none. */
const FLOORS = { top: 0, main: 1, mid: 2, low: 3 } as const;

/**
 * A floor's place from the words its eero's name holds, top floor first;
 * a name with none of them sits mid-house.
 */
function rank(name: string): number {
  const n = name.toLowerCase();
  if (TOP_FLOOR.test(n)) {
    return FLOORS.top;
  }
  if (LOW_FLOOR.test(n)) {
    return FLOORS.low;
  }
  if (MAIN_FLOOR.test(n)) {
    return FLOORS.main;
  }
  return FLOORS.mid;
}

export {
  ago,
  clock,
  dbm,
  gigabytes,
  lasted,
  listed,
  MINUS,
  mbps,
  plural,
  possessive,
  rank,
  roleWords,
  slug,
  v4,
};
