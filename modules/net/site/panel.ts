// The panel beside the map, when no device is open: how the whole network
// stands, said in net's own words wherever net has them, and the devices a
// search finds.

import { type Child, facts, glyph, h } from "./elements.ts";
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

/** What's worth a look: net's concerns, and any source that didn't answer. */
function worthALook(context: Context): HTMLElement {
  const { network } = context;
  // The internet being the limit is how a home line is, not a fault, so
  // it's said under The Internet rather than among what's worth a look.
  const worth = network.concerns.filter((concern) => concern.kind !== "ceiling");
  // A source that didn't answer is worth a look too, one row a source, its
  // reason folded in, so a skim reads one problem rather than two.
  const quiet = network.sources.filter((source) => source.state !== "answering");
  if (worth.length === 0 && quiet.length === 0) {
    return h("p", { class: "hint" }, "Nothing needs a look.");
  }
  const rows = worth.length > 0 ? concernRows(context, worth) : h("ul", { class: "rows" });
  rows.prepend(...quiet.map((source) => quietRow(network, source)));
  return rows;
}

/** How the whole network stands. */
function overview(context: Context): Child[] {
  const { network, shape } = context;
  const boxes = shape.points.length + (shape.modem === undefined ? 0 : 1);
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
      `${plural(network.devices.length - boxes, "device")} use `,
      `${plural(shape.points.length, "eero")}, the Wi-Fi points, on `,
      `${plural(shape.floors.length, "floor")}; ${joins}`,
    ),
    h("h3", {}, "Worth a Look"),
    worthALook(context),
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
export { concernRows, found, overview, row };
