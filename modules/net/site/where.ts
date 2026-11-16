// What the page says aloud as the keys land on a device, and how an address
// names one: by the name the page shows, since the address bar is on
// Joel's screen too, and by net's id only where two devices share a name.

import { type Network, worthALook } from "./model.ts";
import { endAt, linkBetween, type Shape } from "./shape.ts";
import { dbm, plural, possessive, roleWords, slug } from "./words.ts";

/** The forms a link once named a device by, all still landing. */
const ASKED = /^(?<form>device|host)=/u;

/** A device in a sentence: what it is, what it's on, and what's worth a look. */
function whereWords(network: Network, shape: Shape, id: string): string | undefined {
  const device = shape.byId.get(id);
  if (device === undefined) {
    return undefined;
  }
  const upId = shape.via(device);
  const up = upId === undefined ? undefined : shape.byId.get(upId);
  const link = up === undefined ? undefined : linkBetween(network, device.id, up.id);
  let words = `${device.name}, ${roleWords(device)}`;
  if (device.roles.includes("access-point")) {
    const on = network.links.filter(
      (each) => each.b.device === device.id && !each.a.device.startsWith("eero:"),
    );
    words += `, ${plural(on.length, "device")} on it`;
  } else if (link !== undefined && up !== undefined) {
    const end = endAt(link, up.id);
    const at = end.radio ?? (end.port === undefined ? "network" : `port ${end.port}`);
    words += `, on ${possessive(up.name)} ${at}`;
    if (link.signalDbm !== undefined) {
      words += ` at ${dbm(link.signalDbm)} dBm`;
    }
  }
  const worth = worthALook(network, id).length;
  if (worth > 0) {
    words += `; ${plural(worth, "thing")} worth a look`;
  }
  return words;
}

/** The devices a slug of a name picks out. */
const named = (network: Network, name: string): readonly string[] =>
  network.devices.filter((device) => slug(device.name) === name).map((device) => device.id);

/** What an address names `id` by. */
function fragment(network: Network, shape: Shape, id: string): string {
  const device = shape.byId.get(id);
  const name = device === undefined ? "" : slug(device.name);
  return name !== "" && named(network, name).length === 1 ? name : id;
}

/**
 * The device an address's fragment names. The sites' shared form is
 * #device=<name>; a bare name or id, and #host=<name> from mycelium, still
 * land, since an address that was once a home never breaks. One that
 * doesn't decode, typed or cut short, names nothing, so the page opens as
 * it would with none rather than stopping.
 */
function linkedTo(network: Network, shape: Shape, hash: string): string | null {
  let decoded: string;
  try {
    decoded = decodeURIComponent(hash.slice(1));
  } catch {
    return null;
  }
  const asked = decoded.replace(ASKED, "");
  if (asked === "") {
    return null;
  }
  if (shape.byId.has(asked)) {
    return asked;
  }
  const ids = named(network, slug(asked));
  return ids.length === 1 ? (ids[0] ?? null) : null;
}

export { fragment, linkedTo, whereWords };
