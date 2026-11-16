// The network's shape as the map lays it out: who is on what, floor by
// floor, each floor named for its eero. Nothing here touches the page, so
// each case is proven in node.

import type { Device, End, Link, Network } from "./model.ts";
import { rank } from "./words.ts";

/** One floor: its eero, the switches beside it and the devices on it. */
interface Floor {
  readonly point: Device;
  readonly devices: Device[];
  readonly switches: Device[];
}

/** The network laid out by floor, with the ways to look things up. */
interface Shape {
  readonly byId: ReadonlyMap<string, Device>;
  /** The device `device` reaches the internet through first. */
  readonly via: (device: Device) => string | undefined;
  readonly points: readonly Device[];
  readonly gateway: Device | undefined;
  readonly modem: Device | undefined;
  readonly floors: readonly Floor[];
  /** How many devices' routes cross the link between `a` and `b`. */
  readonly crossing: (a: string, b: string) => number;
  /** How many devices have a route to the internet at all. */
  readonly routed: number;
}

/** The link between `a` and `b`, in either order. */
function linkBetween(network: Network, a: string, b: string | undefined): Link | undefined {
  return b === undefined
    ? undefined
    : network.links.find(
        (link) =>
          (link.a.device === a && link.b.device === b) ||
          (link.a.device === b && link.b.device === a),
      );
}

/** The end of `link` at `id`, so either order reads the same. */
function endAt(link: Link, id: string): End {
  return link.a.device === id ? link.a : link.b;
}

/** A link's medium in words: "cable" would read as the modem's coax. */
function medium(link: Link): string {
  if (link.medium === "wifi") {
    return "Wi-Fi";
  }
  return link.medium === "coax" ? "coax" : "Ethernet";
}

/** The busier of a Wi-Fi link's two radios, as net works out its room. */
function busier(network: Network, link: Link): number | undefined {
  const at = (end: End): number | undefined =>
    network.devices
      .find((device) => device.id === end.device)
      ?.radios.find((radio) => radio.name === end.radio)?.busyPct;
  const both = [at(link.a), at(link.b)].filter((value) => value !== undefined);
  return both.length > 0 ? Math.max(...both) : undefined;
}

const has = (device: Device, role: string): boolean => device.roles.includes(role);

/** How far up a device's way its eero is looked for, past any loop. */
const FARTHEST_HOPS = 8;

/** How `network` lays out: its floors, its gateway and its modem. */
function study(network: Network): Shape {
  const byId = new Map(network.devices.map((device) => [device.id, device]));
  const via = (device: Device): string | undefined => device.path?.through[0];
  const points = network.devices.filter((device) => has(device, "access-point"));
  const gateway =
    network.devices.find((device) => has(device, "router")) ??
    points.find((point) => via(point) === "modem") ??
    points[0];
  // Up a device's way to the internet to the first eero it reaches; past
  // the modem, or lost, it counts as the gateway's.
  const home = (device: Device): string | undefined => {
    let at: Device | undefined = device;
    for (let hop = 0; at !== undefined && hop < FARTHEST_HOPS; hop += 1) {
      if (has(at, "access-point")) {
        return at.id;
      }
      if (has(at, "modem")) {
        return gateway?.id;
      }
      const next = via(at);
      at = next === undefined ? undefined : byId.get(next);
    }
    return gateway?.id;
  };
  const floors: Floor[] = points
    .map((point) => ({ point, devices: [], switches: [] }))
    .sort(
      (one, other) =>
        rank(one.point.name) - rank(other.point.name) ||
        one.point.name.localeCompare(other.point.name),
    );
  const floorOf = new Map(floors.map((floor) => [floor.point.id, floor]));
  const modem = network.devices.find((device) => has(device, "modem"));
  for (const device of network.devices) {
    const at = home(device);
    const floor = at === undefined ? undefined : floorOf.get(at);
    if (!has(device, "access-point") && device !== modem && floor !== undefined) {
      (has(device, "switch") ? floor.switches : floor.devices).push(device);
    }
  }
  for (const floor of floors) {
    floor.devices.sort((one, other) =>
      one.name.localeCompare(other.name, "en", { sensitivity: "base" }),
    );
  }
  return {
    byId,
    via,
    points,
    gateway,
    modem,
    floors,
    // net counts the routes crossing each link, not the traffic on it:
    // eero gives no figure for what each link carries.
    crossing: (a, b) => linkBetween(network, a, b)?.crossings ?? 0,
    routed: network.devices.filter((device) => device.path !== undefined).length,
  };
}

export type { Floor, Shape };
export { busier, endAt, linkBetween, medium, study };
