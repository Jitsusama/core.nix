// The panel beside the map, when no device is open: how the whole network
// stands, said in net's own words wherever net has them, and the devices a
// search finds; and how net's feeds stand, which the head says.

import type { Standing } from "../../local-sites/kit/feed.ts";
import { type Child, facts, glyph, h } from "./elements.ts";
import type { Heard } from "./hearing.ts";
import type { Concern, Device, Network, Source } from "./model.ts";
import type { Mycelium } from "./mycelium.ts";
import type { Shape } from "./shape.ts";
import { ago, gigabytes, listed, plural, rate, roleWords, v4 } from "./words.ts";

/** What the panel draws from, and what its rows do. */
interface Context {
  readonly network: Network;
  readonly shape: Shape;
  /** Now, in seconds since the epoch. */
  readonly now: number;
  readonly mycelium: Mycelium;
  readonly open: (id: string) => void;
  readonly letGo: () => void;
  /** How many of net's concern and source watches sent a line the page couldn't read. */
  readonly unheard?: number;
}

/** How many devices a line names before it counts the rest. */
const NAMED = 3;
const PERCENT = 100;
/** A sentence's own stop, which the panel's adds again after its words. */
const STOP = /\.$/u;

const has = (device: Device, role: string): boolean => device.roles.includes(role);

/** What each source is called in a sentence. */
const CALLED: { readonly [source: string]: string } = {
  modem: "the modem",
  switch: "the switch",
  sony: "Sony's receivers",
  hp: "HP's printers",
  airplay: "the Apple TVs",
  samsung: "the Samsung TVs",
};

/** What a source knows, said where it can't be shown. */
const KNOWS: { readonly [source: string]: string } = { switch: "its ports" };

/** A row of the panel's list: its mark, its name, and a fact under it. */
function row(look: boolean, name: Child, fact?: Child): HTMLElement {
  const what = h(
    "span",
    { class: "what" },
    h("span", { class: "name" }, name),
    fact === undefined ? null : h("span", { class: "fact" }, fact),
  );
  return h("li", {}, h("div", { class: "row", role: "presentation" }, glyph(look), what));
}

/** Each of `list` as a row that opens its device. */
function concernRows(context: Context, list: readonly Concern[]): HTMLElement {
  const rows = list.map((concern) => {
    const open = { click: () => context.open(concern.device) };
    const name = h("span", { class: "what" }, h("span", { class: "name" }, concern.says));
    const button = h("button", { type: "button", class: "row", on: open }, glyph(true), name);
    return h("li", {}, button);
  });
  return h("ul", { class: "rows" }, rows);
}

/**
 * The internet as the limit, with the devices it isn't the limit for
 * named, since a count alone leaves the reader asking which.
 */
function ceilingLine(network: Network, concern: Concern): HTMLElement {
  const way = concern.on === "up" || concern.on === "down" ? concern.on : undefined;
  const nearer = network.devices
    .filter((device) => !(has(device, "router") || has(device, "access-point")))
    .filter((device) => way !== undefined && device.path?.[way]?.b !== undefined)
    .map((device) => device.name);
  const named =
    nearer.length > NAMED
      ? `${nearer.slice(0, NAMED).join(", ")} and ${nearer.length - NAMED} more`
      : nearer.join(" and ");
  const narrower = ` For ${named}, a link of their own is narrower; open one to see it.`;
  return h("p", {}, `${concern.says.replace(STOP, "")}.`, nearer.length > 0 ? narrower : "");
}

/** A source that didn't answer, as a row of its own under Worth a Look. */
function quietRow(network: Network, source: Source): HTMLElement {
  const device = network.devices.find((each) => has(each, source.name));
  const called = device === undefined ? source.name : `${source.name}, ${device.name},`;
  const knows = KNOWS[source.name] ?? "what it knows";
  const reason = source.reason ?? "";
  const why = reason === "" ? "" : `${reason.charAt(0).toUpperCase()}${reason.slice(1)}. `;
  const name =
    source.state === "signed-out"
      ? `net isn't signed in to the ${called} so ${knows} can't be shown`
      : `The ${called} didn't answer, so ${knows} can't be shown`;
  return row(true, name, `${why}It's asked again each minute.`);
}

/** Where net reads from: one line while all answer, and each when any doesn't. */
function readFrom(context: Context): HTMLElement[] {
  const { network, now } = context;
  const times = network.sources.map((source) => source.readAt ?? network.readAt ?? now);
  const oldest = Math.min(...times);
  if (network.sources.every((source) => source.state === "answering")) {
    const names = listed(network.sources.map((source) => CALLED[source.name] ?? source.name));
    const line = `${names}, all answering; the oldest read ${ago(oldest, now)}.`;
    return [h("h3", {}, "Read From"), h("p", {}, line)];
  }
  const rows = network.sources.map((source) => {
    const answering = source.state === "answering";
    const read = source.readAt === undefined ? "lately" : ago(source.readAt, now);
    const stands = answering ? "answering" : source.state.replace("-", " ");
    const fact = answering
      ? `${source.what ?? "it"}, read ${read}`
      : "not read now; why is under Worth a Look";
    return row(!answering, `${source.name}, ${stands}`, fact);
  });
  return [h("h3", {}, "Read From"), h("ul", { class: "rows" }, rows)];
}

/** One way of what the line carried: its bytes, its average and its busiest hour. */
function carriedWay(name: string, each: readonly number[], line?: number): [string, string] {
  const average = each.reduce((sum, n) => sum + n, 0) / each.length;
  const share = line ? `, ${Math.round((average / line) * PERCENT)}% of its speed` : "";
  return [
    name,
    `averaging ${rate(average)}${share}; ${rate(Math.max(...each))} in its busiest hour`,
  ];
}

/**
 * What the line carried over the last day beside its speed, so a line full
 * all day reads apart from one busy for an hour. Only averages of net's own
 * hours; what counts as too full stays net's to say.
 */
function carried(network: Network): Child[] {
  const { internet } = network;
  const day = internet?.carried;
  if (internet === undefined || day === undefined || day.hours.length === 0) {
    return [];
  }
  const [downName, down] = carriedWay(
    "down",
    day.hours.map((hour) => hour.downMbps),
    internet.downMbps,
  );
  const [upName, up] = carriedWay(
    "up",
    day.hours.map((hour) => hour.upMbps),
    internet.upMbps,
  );
  return [
    h("p", {}, `What the line carried over the last ${plural(day.hours.length, "hour")}:`),
    facts([
      [downName, `${gigabytes(day.downBytes)}, ${down}`],
      [upName, `${gigabytes(day.upBytes)}, ${up}`],
    ]),
  ];
}

/** The internet: its provider, its speeds, and what limits it. */
function theInternet(context: Context): Child[] {
  const { network, now } = context;
  const { internet } = network;
  if (internet === undefined) {
    return [];
  }
  const speed = (n?: number): string => (n === undefined ? "not measured" : rate(n));
  const tested =
    internet.measuredAt === undefined
      ? null
      : `${ago(internet.measuredAt, now)}, by ${internet.source ?? "eero"}`;
  const ceilings = network.concerns.filter((concern) => concern.kind === "ceiling");
  return [
    h("h3", {}, "The Internet"),
    facts([
      ["provider", internet.provider],
      ["down", speed(internet.downMbps), true],
      ["up", speed(internet.upMbps), true],
      ["last speed test", tested],
    ]),
    ...ceilings.map((concern) => ceilingLine(network, concern)),
    ...carried(network),
  ];
}

/** What's worth a look: the concerns, then the sources that didn't answer. */
function worthOf(network: Network): { worth: readonly Concern[]; quiet: readonly Source[] } {
  return {
    // The internet being the limit is how a home line is, not a fault, so
    // it's said under The Internet rather than among what's worth a look.
    worth: network.concerns.filter((concern) => concern.kind !== "ceiling"),
    // A source that didn't answer is worth a look too, one row a source, its
    // reason folded in, so a skim reads one problem rather than two.
    quiet: network.sources.filter((source) => source.state !== "answering"),
  };
}

/** The concerns and sources read that the page couldn't make out, each maybe worth a look. */
function unreadOf(network: Network): readonly string[] {
  return network.unread.filter(
    (record) => record.startsWith("concern ") || record.startsWith("source "),
  );
}

/** What the panel says of `count` things net told that couldn't be read. */
function unreadSaid(count: number): string {
  return `${plural(count, "thing")} net told couldn't be read, and may be worth a look.`;
}

/**
 * How many things net told that the page couldn't read, each maybe worth a
 * look: the records in `network` it couldn't make out, and the watches that
 * sent a line it couldn't read at all, as a line that isn't JSON or a record
 * with no key, which never reaches the network. The lede and the panel both
 * count this.
 */
function unreadCount(network: Network, unheard: number): number {
  return unreadOf(network).length + unheard;
}

/**
 * The words Worth a Look says beneath its rows, or alone when it has none:
 * how much couldn't be read, or that nothing needs a look.
 */
function worthHint(context: Context): string | undefined {
  const { network } = context;
  const unread = unreadCount(network, context.unheard ?? 0);
  if (unread > 0) {
    return unreadSaid(unread);
  }
  const { worth, quiet } = worthOf(network);
  return worth.length === 0 && quiet.length === 0 ? "Nothing needs a look." : undefined;
}

/** How many devices use the network: every one read, less the eeros and the modem. */
function users(context: Context): number {
  const { network, shape } = context;
  return network.devices.length - shape.points.length - (shape.modem === undefined ? 0 : 1);
}

/** Every feed's records heard since it went live, and every answer read. */
const ALL_HEARD: Heard = { fresh: true, unheard: 0 };

/** What net last told of the house, and what of it is worth a look, unless it's gone. */
function asksOf(network: Network, told: boolean, unheard: number): string {
  if (!told) {
    // A feed that stopped keeps its last records, each source still answering
    // as it last said, so what they ask would read as an all-clear.
    return "as net last told them";
  }
  const { worth, quiet } = worthOf(network);
  // A concern or a source the page couldn't read may be one worth a look,
  // so it's counted as one rather than read as an all-clear, as the panel
  // counts it.
  const looks = worth.length + quiet.length + unreadCount(network, unheard);
  return looks === 0 ? "and nothing needs a look" : `and ${plural(looks, "thing")} worth a look`;
}

/**
 * What stands now, in one sentence: the head's lede. While net's feeds are
 * live, it says what is worth a look; otherwise it says the counts are as net
 * last told them.
 */
function standsNow(context: Context, told = true, unheard = 0): string {
  const { network, shape } = context;
  return `${plural(users(context), "device")} on ${plural(shape.points.length, "eero")} across ${plural(shape.floors.length, "floor")}, ${asksOf(network, told, unheard)}.`;
}

/**
 * The lede while the server says it can't reach net and nothing is drawn,
 * whether net never answered or answered once and stopped, since a page
 * opened after can't tell the two apart.
 */
const UNANSWERED = "Net isn't answering, so no device is drawn.";

/** The lede while nothing is drawn and the page can't reach the sites' server. */
const UNREACHED = "The sites' server can't be reached, so no device is drawn.";

/** The lede while net answers and its records, all read, name no device. */
const NAMELESS = "Net is answering, but has named no device yet.";

/** The lede while net is being reached, as the page first says it. */
const REACHING = "Reaching net for every device on it.";

/** The worst standing first, the one the page says. */
const WORST_FIRST = ["broken", "stale", "connecting", "live"];

/** Each of net's watches' standing, as its feed last said it. */
class Standings {
  readonly #each = new Map<string, Standing>();

  /** Takes how the watch of `kind` stands now. */
  set(kind: string, stands: Standing): void {
    this.#each.set(kind, stands);
  }

  /** How the worst of net's feeds stands, or undefined before any has said. */
  worst(): Standing | undefined {
    return [...this.#each.values()].sort(
      (one, other) => WORST_FIRST.indexOf(one.state) - WORST_FIRST.indexOf(other.state),
    )[0];
  }
}

/**
 * What the lede says, from the worst of net's feeds and the network read so
 * far, so words said of an earlier standing never outlast it. While the map
 * draws a device it counts them, saying what's worth a look only while the
 * feeds are live and each has told its records since: a page reconnecting
 * to a server that restarted is told connecting and no records, then live,
 * and only then the records that make what it holds current again. A page
 * opened while net is down is given the records the
 * server still holds, and draws them, so the lede counts them too rather
 * than saying none is drawn. With none drawn, the counts are never said: a
 * page joining a watch the server holds as broken is told every kind empty,
 * net serves every kind empty until its first reading is done, and "0
 * devices, and nothing needs a look" would read as an all-clear. Until
 * every kind is read, there is no network yet, so even live feeds are still
 * being reached.
 */
function ledeOf(
  worst: string | undefined,
  context: Context | undefined,
  heard: Heard = ALL_HEARD,
): string {
  // A feed is stale while the sites' server can't be reached, broken while
  // the server is reached and says net isn't (the kit's feed.ts), and
  // connecting while net's records may be from before.
  const told = worst === "live" && heard.fresh;
  if (context !== undefined && context.network.devices.length > 0) {
    return standsNow(context, told, heard.unheard);
  }
  const stands: Record<string, string> = {
    broken: UNANSWERED,
    // Empty records held from before a reconnect name nothing net says now.
    live: context === undefined || !heard.fresh ? REACHING : NAMELESS,
    stale: UNREACHED,
  };
  return stands[worst ?? ""] ?? REACHING;
}

/** Says what stands now in the head's lede, writing only when it changes. */
function sayStandsNow(
  lede: HTMLElement,
  worst: string | undefined,
  context: Context | undefined,
  heard: Heard = ALL_HEARD,
): void {
  const words = ledeOf(worst, context, heard);
  if (lede.textContent !== words) {
    lede.textContent = words;
  }
}

/**
 * What's worth a look: net's concerns, any source that didn't answer, and
 * how much net told that couldn't be read, counted as the lede counts it.
 */
function worthALook(context: Context): Child[] {
  const { network } = context;
  const { worth, quiet } = worthOf(network);
  const hint = worthHint(context);
  const said = hint === undefined ? [] : [h("p", { class: "hint" }, hint)];
  if (worth.length === 0 && quiet.length === 0) {
    return said;
  }
  const rows = worth.length > 0 ? concernRows(context, worth) : h("ul", { class: "rows" });
  rows.prepend(...quiet.map((source) => quietRow(network, source)));
  return [rows, ...said];
}

/** How the whole network stands. */
function overview(context: Context): Child[] {
  const { network, shape } = context;
  const joins =
    shape.modem === undefined
      ? "no modem was read."
      : `the modem joins them to ${network.internet?.provider ?? "the internet"}.`;
  const command = (words: string): HTMLElement => h("code", {}, words);
  return [
    h("h2", {}, "How It Stands"),
    h(
      "p",
      { class: "what-it-is" },
      `${plural(users(context), "device")} use `,
      `${plural(shape.points.length, "eero")}, the Wi-Fi points, on `,
      `${plural(shape.floors.length, "floor")}; ${joins}`,
    ),
    h("h3", {}, "Worth a Look"),
    ...worthALook(context),
    ...theInternet(context),
    ...readFrom(context),
    h(
      "p",
      { class: "hint" },
      "At a terminal, the ",
      command("net"),
      " command says the same, and ",
      command("net devices NAME"),
      " shows one device in full.",
    ),
  ];
}

/** The devices `query` finds, by name, maker, model or address. */
function found(context: Context, query: string): Child[] {
  const q = query.trim().toLowerCase();
  const hits = context.network.devices.filter((device) =>
    [device.name, device.vendor, device.model, ...device.addresses, ...device.macs].some(
      (value) => value?.toLowerCase().includes(q) === true,
    ),
  );
  const rows = hits.map((device) => {
    const fact = [roleWords(device), v4(device)[0]].filter(Boolean).join(" \u00b7 ");
    const what = h(
      "span",
      { class: "what" },
      h("span", { class: "name" }, device.name),
      h("span", { class: "fact" }, fact),
    );
    const given = { type: "button", class: "row", "data-id": device.id };
    const open = { click: () => context.open(device.id) };
    return h("li", {}, h("button", { ...given, on: open }, glyph(false), what));
  });
  return [
    h("h2", {}, hits.length > 0 ? `${plural(hits.length, "device")} found` : "None found"),
    h("ul", { class: "rows", id: "found" }, rows),
  ];
}

export type { Context };
export {
  concernRows,
  found,
  ledeOf,
  NAMELESS,
  overview,
  REACHING,
  row,
  sayStandsNow,
  Standings,
  standsNow,
  UNANSWERED,
  UNREACHED,
  unreadSaid,
  worthHint,
};
