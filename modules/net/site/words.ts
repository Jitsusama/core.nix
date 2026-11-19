// How the network page says figures, times and devices in words. Nothing
// here touches the page, so each is proven in node.

import type { Device } from "./model.ts";

/** The minus sign, rather than a hyphen, before a figure below zero. */
const MINUS = "\u2212";

const KILO = 1e3;
/** A cable's speed comes in whole hundreds of megabits. */
const HUNDRED = 100;
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

/** How many significant digits a measured rate keeps, as the design asks. */
const DIGITS = 3;
/** The smallest unit a rate given in Mb/s is said in, whatever its size. */
const BITS: readonly [number, string] = [1 / MEGA, "b/s"];
/** Each unit a rate given in Mb/s may be said in, the largest first. */
const RATES: readonly (readonly [number, string])[] = [
  [KILO, "Gb/s"],
  [1, "Mb/s"],
  [1 / KILO, "kb/s"],
  BITS,
];

/**
 * A measured rate given in Mb/s, in three figures, in the largest unit it
 * is at least one of once rounded: 370 Mb/s, 1.20 Gb/s, 400 kb/s, and
 * 0 b/s for none, as under a bit a second is. net's figures.rs says them
 * the same way, held to the same cases (tests/rates.json).
 */
function rate(mbps: number): string {
  const rounded = (value: number): number => Number(value.toPrecision(DIGITS));
  const [bit] = BITS;
  if (!(Number.isFinite(mbps) && rounded(mbps / bit) >= 1)) {
    return "0 b/s";
  }
  const [scale, unit] = RATES.find(([s]) => rounded(mbps / s) >= 1) ?? BITS;
  const near = rounded(mbps / scale);
  // toFixed rather than toPrecision, which writes 1000 as "1.00e+3".
  const power = Number(near.toExponential().split("e")[1]);
  return `${near.toFixed(Math.max(0, DIGITS - 1 - power))} ${unit}`;
}

/**
 * A link's speed given in Mb/s, which names the link rather than measures
 * it, so it keeps the digits it's named by. A cable's come in hundreds of
 * megabits and read 1 Gb/s or 2.5 Gb/s; Wi-Fi's read as they're named,
 * 866, 1201 or 72.2 Mb/s.
 */
function linkSpeed(mbps: number): string {
  const near = Number(mbps.toFixed(2));
  const trim = (n: number): string => n.toFixed(2).replace(TRAILING_ZEROS, "$<kept>");
  return near >= KILO && near % HUNDRED === 0 ? `${trim(near / KILO)} Gb/s` : `${trim(near)} Mb/s`;
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

/** The characters a shell reads as themselves, wherever they sit. */
const PLAIN = /^[A-Za-z0-9_@%+=:,./-]+$/u;

/**
 * A word as a shell reads it back whole and runs nothing of, as the
 * terminal kit's `shell_word` writes it: as it is where it's plain and
 * doesn't open with `=`, which zsh turns into a command's path, and
 * otherwise in single quotes, each quote of its own closed, escaped and
 * opened again. A device's name is anyone's to set, so a hint naming it
 * as a command would otherwise run what the name holds when pasted.
 */
function shellWord(word: string): string {
  if (PLAIN.test(word) && !word.startsWith("=")) {
    return word;
  }
  return `'${word.replaceAll("'", "'\\''")}'`;
}

/**
 * What the terminal kit keeps to shape a word, though alone it draws
 * nothing, as its `unseen` leaves them: the soft hyphen, Hangul's vowel
 * filler, Mongolian's selectors and vowel separator, the joiners, the
 * variation selectors and their supplement, and the tags that spell a
 * flag. An emoji as a phone types it, a heart with its selector or a
 * family joined, and a script's letters are drawn wrong without them.
 * Hangul's leading filler, U+115F, isn't among them: the kit reads it as
 * a blank, as it reads braille's.
 */
const SHAPING =
  String.raw`\u00ad\u1160\u180b-\u180f\u200c\u200d\ufe00-\ufe0f` +
  String.raw`\u{e0020}-\u{e007f}\u{e0100}-\u{e01ef}`;

/**
 * What a terminal could be handed besides the words it shows: a control
 * character, such as an escape that ends a terminal's bracketed paste and
 * has the rest run as typed; a mark of format or direction, or one that
 * draws nothing, which read one way and paste another; a line or
 * paragraph break; and any blank but a plain space, braille's and
 * Hangul's blanks among them, which a page shows as one space wherever it
 * stands. What shapes a word stays, as it does at a terminal. Hangul's
 * blank is default-ignorable; braille's is a letter to Unicode, so it's
 * named.
 */
const UNSEEN = new RegExp(
  String.raw`[[\p{Cc}\p{Cf}\p{Co}\p{Cs}\p{Zl}\p{Zp}\p{Default_Ignorable_Code_Point}\u2800]` +
    String.raw`--[${SHAPING}]]|[^\S ]`,
  "v",
);

/** A word without what shapes it, as it reads to a person. */
const UNSHAPED = new RegExp(`[${SHAPING}]`, "gv");

/**
 * A character that draws nothing standing where a word starts, as the
 * kit's `typed` refuses it: alone it shows nothing, and after a space it
 * draws as a second one. Hangul's medial vowels and final consonants,
 * U+1161 to U+11FF and U+D7B0 to U+D7FF, are letters to Unicode but take
 * no cell of their own, as the kit measures them.
 */
const OPENS_BARE = new RegExp(
  String.raw`(?:^| )[\p{Mn}\p{Me}${SHAPING}\u1161-\u11ff\ud7b0-\ud7ff]`,
  "v",
);

/**
 * Whether a word pastes into a terminal just as the page shows it:
 * something to read, with nothing unseen, no run of spaces, since a page
 * shows a run as one, and nothing that draws nothing where a word starts.
 */
function pastesAsRead(word: string): boolean {
  return word !== "" && !UNSEEN.test(word) && !word.includes("  ") && !OPENS_BARE.test(word);
}

/** ASCII letters in lower case, as net matches a name in any case. */
function folded(word: string): string {
  return word.replace(/[A-Z]/gu, (letter) => letter.toLowerCase());
}

/**
 * A word as it reads to a person: without what shapes it, in net's case,
 * then composed as Unicode composes it. Shaping goes first so a joiner
 * between a letter and its mark can't keep them apart, and case before
 * composing so two words net folds to one always read as one.
 */
function reads(word: string): string {
  return folded(word.replace(UNSHAPED, "")).normalize("NFC");
}

/**
 * The words that show a device at a terminal, each typed as it reads and
 * finding it alone, best first. net takes a word as an id, then as an
 * address, then as a name in any case, refuses a name two devices share,
 * and reads a word opening with `-` as a flag. So a name is offered when
 * nothing else answers to it; then the first address, as net's own try
 * lines choose it, when no other device answers there; then net's id,
 * unless it's one net gives to several. A name that reads as another's
 * name, id or address, or as an id net gives several, which it reads as
 * the first device holding it, isn't offered, since the command would read
 * as naming another.
 */
function findingWords(device: Device, devices: readonly Device[]): string[] {
  const others = devices.filter((other) => other.id !== device.id);
  const { name } = device;
  const read = reads(name);
  const taken = [
    ...SHARED_IDS,
    ...others.flatMap((other) => [other.id, ...other.addresses, other.name]),
  ];
  const nameFinds = taken.every((word) => reads(word) !== read);
  const address = device.addresses[0];
  const addressFinds =
    address !== undefined &&
    address !== device.id &&
    others.every((other) => other.id !== address && !other.addresses.includes(address));
  return [
    ...(nameFinds ? [name] : []),
    ...(addressFinds ? [address] : []),
    ...(SHARED_IDS.has(device.id) ? [] : [device.id]),
  ].filter((word) => !word.startsWith("-") && pastesAsRead(word));
}

/**
 * The ids net gives every device of a kind it reads without one of its
 * own: an eero client with no hardware address, and an eero with no id
 * (modules/net/src/sources/eero.rs). The page keeps one device to an id,
 * so it can't see the others that share it, and net finds the first.
 */
const SHARED_IDS: ReadonlySet<string> = new Set(["mac:?", "eero:unknown"]);

/**
 * The command that shows a device at a terminal, as net's own try lines
 * write it: by the first word that finds it alone and pastes as it reads,
 * quoted as a shell word, or none when no word does. Changing a name to
 * show it would have the command name a device it didn't.
 */
function deviceCommand(device: Device, devices: readonly Device[]): string | undefined {
  const [word] = findingWords(device, devices);
  return word === undefined ? undefined : `net devices ${shellWord(word)}`;
}

export {
  ago,
  clock,
  dbm,
  deviceCommand,
  gigabytes,
  lasted,
  listed,
  linkSpeed,
  MINUS,
  plural,
  possessive,
  rank,
  rate,
  roleWords,
  shellWord,
  slug,
  v4,
};
