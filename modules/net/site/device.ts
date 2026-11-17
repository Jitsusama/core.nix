// The panel beside the map for one device: what it is, what net names worth
// a look about it, its route to the internet, its own link, its speed and
// radios, and its facts, in net's own words wherever net has them.

import { cable } from "./cable.ts";
import { type Cell, type Child, facts, h, table } from "./elements.ts";
import { inMycelium } from "./machine.ts";
import { type Device, type Link, type Narrowest, type Radio, worthALook } from "./model.ts";
import { type Context, concernRows } from "./panel.ts";
import { busier, endAt, linkBetween, medium } from "./shape.ts";
import { el } from "./svg.ts";
import { ago, clock, dbm, lasted, linkSpeed, plural, rate, roleWords, v4 } from "./words.ts";

/** From this signal down, net names a link weak and its rate falls. */
const WEAK_DBM = -70;
/** An eero's two 5 GHz radios, named for the channels each takes. */
const FIVE_GHZ_PAIR = / (low|high)$/u;
const SPACE = /\s/u;

const has = (device: Device, role: string): boolean => device.roles.includes(role);

/** The narrowest room one way, and why it is what it is. */
function narrowestLine(context: Context, narrow: Narrowest | undefined, way: string): Child {
  if (narrow === undefined) {
    return null;
  }
  const { network, shape, now } = context;
  const name = (id: string): string => shape.byId.get(id)?.name ?? id;
  // Why the figure is what it is, from the link it's on: a radio's rate
  // less the time its busier radio is busy, or a cable's speed.
  const link = linkBetween(network, narrow.a, narrow.b);
  const busy = link?.medium === "wifi" ? busier(network, link) : undefined;
  let why: string;
  if (narrow.b === undefined) {
    const when = network.internet?.measuredAt;
    const since = when === undefined ? "" : `, ${ago(when, now)}`;
    why = `the internet itself, as eero last measured it${since}`;
  } else if (link?.speedMbps !== undefined && busy !== undefined) {
    const less = `less the ${busy}% of the time its radio is busy`;
    why = `between ${name(narrow.a)} and ${name(narrow.b)}: their ${linkSpeed(link.speedMbps)} link, ${less}`;
  } else {
    why = `between ${name(narrow.a)} and ${name(narrow.b)}`;
  }
  const one = narrow.unmeasured === 1;
  const links = one ? "One link" : `${narrow.unmeasured} links`;
  const blind =
    narrow.unmeasured > 0
      ? `. ${links} on the way ${one ? "has" : "have"} no speed given, so it could be narrower`
      : "";
  const most = `${way}: at most ${rate(narrow.roomMbps)}`;
  return h("p", {}, h("span", { class: "narrowest" }, most), `, ${why}${blind}.`);
}

/** How one hop reaches the next: the link's medium, speed and signal. */
function hopWords(link: Link): string {
  const speed = link.speedMbps
    ? linkSpeed(link.speedMbps)
    : `${link.source ?? "net"} gives no speed for it`;
  const signal = link.signalDbm === undefined ? undefined : `${dbm(link.signalDbm)} dBm`;
  return [medium(link), speed, signal].filter(Boolean).join(", ");
}

/** A device's route to the internet, hop by hop. */
function route(context: Context, device: Device): Child[] {
  const { network, shape } = context;
  const { path } = device;
  if (path === undefined) {
    return [];
  }
  const hops = [device.id, ...path.through];
  const list = h("ol", { class: "way" });
  hops.forEach((id, k) => {
    const at = shape.byId.get(id);
    const next = hops[k + 1];
    let onward: string;
    if (next === undefined) {
      const coax = at?.ports.some((port) => port.kind === "coax") === true ? "coax" : undefined;
      const how = at !== undefined && has(at, "modem") ? [network.internet?.provider, coax] : [];
      const said = how.filter(Boolean).join(", ");
      onward = ` to the internet${said ? `: ${said}` : ""}`;
    } else {
      const link = linkBetween(network, id, next);
      onward = ` to ${shape.byId.get(next)?.name ?? next}: ${link === undefined ? "" : hopWords(link)}`;
    }
    const step = h("span", { class: "step" }, String(k + 1));
    list.append(
      h("li", {}, step, h("span", {}, h("span", { class: "name" }, at?.name ?? id), onward)),
    );
  });
  return [
    h("h3", {}, "Its Route to the Internet"),
    list,
    narrowestLine(context, path.down, "Down"),
    narrowestLine(context, path.up, "Up"),
  ];
}

/** What a signal means, said beside a Wi-Fi link's facts. */
function signalHint(weak: boolean): HTMLElement {
  return h(
    "p",
    { class: "hint" },
    "The link rate is the speed the two radios agreed on, not what flows over it. ",
    "Signal is in dBm: the nearer 0, the stronger. At ",
    `${dbm(WEAK_DBM)} or weaker the rate falls and net names it worth a look; `,
    weak ? "this one is." : "this one is fine.",
  );
}

/** Its own link, for a device on an eero or a switch. */
function ownLink(context: Context, device: Device): Child[] {
  const { network, shape } = context;
  const up = shape.via(device);
  const link = linkBetween(network, device.id, up);
  if (
    up === undefined ||
    link === undefined ||
    has(device, "access-point") ||
    has(device, "modem")
  ) {
    return [];
  }
  const far = endAt(link, up);
  const near = endAt(link, device.id);
  const at = far.radio ?? (far.port === undefined ? undefined : `port ${far.port}`);
  const wifi = link.medium === "wifi";
  const channel = link.channel === undefined ? "" : `, channel ${link.channel}`;
  const out: Child[] = [
    h("h3", {}, "Its Link"),
    facts([
      ["to", `${shape.byId.get(up)?.name ?? up}${at === undefined ? "" : `, ${at}`}`],
      ["from", near.port === undefined ? null : `its port ${near.port}`],
      ["over", wifi ? `Wi-Fi${channel}` : medium(link)],
      [wifi ? "link rate" : "speed", link.speedMbps ? linkSpeed(link.speedMbps) : null, true],
      ["signal", link.signalDbm === undefined ? null : `${dbm(link.signalDbm)} dBm`, true],
    ]),
  ];
  if (link.signalDbm !== undefined) {
    // Whether this one is weak comes from net's concerns, so the page draws
    // the line where net does.
    const weak = network.concerns.some(
      (concern) => concern.kind === "weak" && concern.device === device.id,
    );
    out.push(signalHint(weak));
  }
  return out;
}

/**
 * Its speed as eero counts it, now and at its busiest in eero's hour; gold
 * where net names it filling the internet, so the panel and the drawing
 * draw the line where net does.
 */
function speedOf(context: Context, device: Device): Child[] {
  const { speed } = device;
  if (speed === undefined) {
    return [];
  }
  const fills = (way: string): boolean =>
    context.network.concerns.some(
      (concern) => concern.kind === "fills" && concern.device === device.id && concern.on === way,
    );
  const now = (n: number | undefined, way: string): Cell => {
    if (n === undefined) {
      return "";
    }
    return fills(way) ? { text: rate(n), look: true } : rate(n);
  };
  const most = (n: number | undefined, at: number | undefined): Cell => {
    if (n === undefined) {
      return "";
    }
    return at === undefined ? rate(n) : `${rate(n)} at ${clock(at)}`;
  };
  const rows = [
    ["now", now(speed.downMbps, "down"), now(speed.upMbps, "up")],
    [
      "busiest",
      most(speed.busiestDownMbps, speed.busiestDownAt),
      most(speed.busiestUpMbps, speed.busiestUpAt),
    ],
  ];
  return [
    h("h3", {}, "Its Speed"),
    table(["when", "down", "up"], rows, [1, 2]),
    h(
      "p",
      { class: "hint" },
      "What it carries each way, as eero counts it. ",
      "Busiest is the most in eero's last hour, from readings five minutes apart.",
    ),
  ];
}

/** One radio's row: its channel, how busy it is, and the devices on it. */
function radioRow(context: Context, device: Device, radio: Radio, busiest: boolean): Cell[] {
  const { network } = context;
  // Gold where net names the radio a concern, so the panel and the drawing
  // draw the line where net does.
  const look = network.concerns.some(
    (concern) =>
      concern.kind === "busy" && concern.device === device.id && concern.on === radio.name,
  );
  const busy = radio.busyPct === undefined ? "" : `${radio.busyPct}%`;
  const on = network.links.filter(
    (link) =>
      link.b.device === device.id &&
      link.b.radio === radio.name &&
      !link.a.device.startsWith("eero:"),
  ).length;
  const most = radio.busiestPct === undefined ? "" : `${radio.busiestPct}%`;
  return [
    radio.name,
    radio.channel ?? "",
    radio.widthMhz ?? "",
    look && busy !== "" ? { text: busy, look: true } : busy,
    ...(busiest ? [most] : []),
    on,
  ];
}

/** An eero's radios, and how many devices each carries. */
function radiosOf(context: Context, device: Device): Child[] {
  const radios = device.radios.filter(
    (radio) => has(device, "access-point") || radio.busyPct !== undefined,
  );
  if (radios.length === 0) {
    return [];
  }
  // The day's busiest shows only once it says more than now does.
  const busiest = radios.some(
    (radio) => radio.busiestPct !== undefined && radio.busiestPct > (radio.busyPct ?? 0),
  );
  const head = ["radio", "ch", "MHz", "busy", ...(busiest ? ["busiest"] : []), "on it"];
  const rows = radios.map((radio) => radioRow(context, device, radio, busiest));
  const out: Child[] = [
    h("h3", {}, "Radios"),
    table(head, rows, busiest ? [1, 2, 3, 4, 5] : [1, 2, 3, 4]),
  ];
  const [top] = radios
    .filter((radio) => radio.busiestAt !== undefined)
    .sort((one, other) => (other.busiestPct ?? 0) - (one.busiestPct ?? 0));
  if (busiest && top?.busiestAt !== undefined) {
    const peaked = `${top.name} peaked at ${clock(top.busiestAt)}`;
    const said = `Busiest is the most each was in the last day, from readings a minute apart; ${peaked}.`;
    out.push(h("p", { class: "hint" }, said));
  }
  if (radios.some((radio) => FIVE_GHZ_PAIR.test(radio.name))) {
    out.push(
      h(
        "p",
        { class: "hint" },
        "An eero has two 5 GHz radios: low takes channels 36 to 64, high 100 and up. ",
        "Busy is the share of the time the radio's channel is taken, by this eero or anything near.",
      ),
    );
  }
  return out;
}

/** A switch's or the modem's ports, as they stand. */
function ports(device: Device): Child[] {
  if (device.ports.length === 0) {
    return [];
  }
  const rows = device.ports.map((port) => {
    let speed = "";
    if (port.speedMbps) {
      speed = String(port.speedMbps);
    } else if (port.link === "up") {
      speed = "not given";
    }
    return [port.name, port.link ?? "", speed, port.duplex ?? ""];
  });
  return [h("h3", {}, "Ports"), table(["port", "link", "speed", "duplex"], rows, [2])];
}

/** What net read of it, and how to ask net for it at a terminal. */
function factsOf(context: Context, device: Device): Child[] {
  const four = v4(device);
  const six = device.addresses.length - four.length;
  const sixWords = six > 0 ? ` and ${plural(six, "IPv6 address", "IPv6 addresses")}` : "";
  const seen = device.source === "eero" && four.length > 1 ? "addresses eero saw" : "address";
  const read =
    device.readAt === undefined
      ? null
      : `${ago(device.readAt, context.now)}, from ${device.source ?? "net"}`;
  const name = SPACE.test(device.name) ? `"${device.name}"` : device.name;
  return [
    h("h3", {}, "Facts"),
    facts([
      [seen, four.join(", ") + sixWords, true],
      ["hardware", device.macs.join(", "), true],
      ["model", device.model],
      ["hardware version", device.hardware],
      ["firmware", device.firmware, true],
      ["serial", device.serial, true],
      ["up for", device.upS ? lasted(device.upS) : null],
      ["read", read],
    ]),
    h(
      "p",
      { class: "hint" },
      "At a terminal, ",
      h("code", {}, `net devices ${name}`),
      " shows everything net read of it.",
    ),
  ];
}

/** The back arrow out of a device, to how the network stands. */
function back(context: Context): HTMLElement {
  const arrow = el("svg", { viewBox: "0 0 12 12", "aria-hidden": "true" });
  el("path", { d: "M8 2L4 6l4 4" }, arrow);
  const leave = { click: () => context.letGo() };
  return h("button", { type: "button", class: "back", on: leave }, arrow, "How the network stands");
}

/** One device in full. */
function details(context: Context, device: Device): Child[] {
  // A client's maker is its network card's, looked up from its hardware
  // address, so it's said as that rather than as what the device is.
  const maker =
    has(device, "client") && device.vendor !== undefined && device.model === undefined
      ? `network card by ${device.vendor}`
      : [device.vendor, device.model].filter(Boolean).join(", ");
  const mine = worthALook(context.network, device.id);
  return [
    back(context),
    h("h2", {}, device.name),
    h("p", { class: "what-it-is" }, [roleWords(device), maker].filter(Boolean).join(" \u00b7 ")),
    mine.length > 0 ? concernRows(context, mine) : null,
    ...route(context, device),
    ...inMycelium(context.mycelium, device),
    ...ownLink(context, device),
    ...speedOf(context, device),
    ...radiosOf(context, device),
    ...ports(device),
    ...cable(device),
    ...factsOf(context, device),
  ];
}

export { details };
