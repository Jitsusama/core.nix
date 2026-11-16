// What the network page draws, read from net's records (modules/net's
// src/serve.rs): a record a device, a link, a source and a concern, and
// the internet's one. They are rebuilt here into one Network, in the shape
// net's own record has, its fields named as the page's code names things,
// so the drawing reads one thing. Nothing here touches the page, so every
// case is proven in node, and a record the page couldn't read is said
// rather than drawn as if it were whole.

import type { Held, Record } from "../../local-sites/kit/records.ts";
import { isObject, number, object, text, texts } from "../../local-sites/kit/records.ts";

/** A JSON object, read as nothing more. */
interface Fields {
  readonly [field: string]: unknown;
}

/** The narrowest room on a device's way to the internet, one way. */
interface Narrowest {
  /** The device nearer the device, at the narrow link's one end. */
  readonly a: string;
  /** The far end, or none when the internet itself is the narrowest. */
  readonly b: string | undefined;
  readonly roomMbps: number;
  /** How many links on the way give no speed, so it could be narrower. */
  readonly unmeasured: number;
}

/** A device's way to the internet: each hop after it, and the narrowest. */
interface Path {
  readonly through: readonly string[];
  readonly down: Narrowest | undefined;
  readonly up: Narrowest | undefined;
}

/** One of an eero's radios, as eero reads it. */
interface Radio {
  readonly name: string;
  readonly channel: number | undefined;
  readonly widthMhz: number | undefined;
  readonly busyPct: number | undefined;
  /** The most it was busy in the last day, and when. */
  readonly busiestPct: number | undefined;
  readonly busiestAt: number | undefined;
}

/** One port of a switch, or of the modem. */
interface Port {
  readonly name: string;
  readonly kind: string | undefined;
  readonly link: string | undefined;
  readonly speedMbps: number | undefined;
  readonly duplex: string | undefined;
}

/** What a device carries each way, now and at its busiest in eero's hour. */
interface Speed {
  readonly downMbps: number | undefined;
  readonly upMbps: number | undefined;
  readonly busiestDownMbps: number | undefined;
  readonly busiestDownAt: number | undefined;
  readonly busiestUpMbps: number | undefined;
  readonly busiestUpAt: number | undefined;
}

/** One of the modem's channels, as its status pages give it. */
interface Channel {
  readonly channel: number;
  readonly kind: string;
  readonly active: boolean;
  readonly powerDbmv: number | undefined;
  readonly snrDb: number | undefined;
  readonly uncorrectable: number | undefined;
  readonly lostLately: number | undefined;
}

/** The modem's own facts, beside its device's. */
interface Modem {
  readonly docsis: string | undefined;
  readonly networkAccess: string | undefined;
  readonly address: string | undefined;
  readonly downstream: readonly Channel[];
  readonly upstream: readonly Channel[];
  /** How long "lately" spans, once net keeps a day of readings. */
  readonly latelyS: number | undefined;
}

/** One device on the network, by net's id for it. */
interface Device {
  readonly id: string;
  readonly name: string;
  readonly roles: readonly string[];
  readonly addresses: readonly string[];
  readonly macs: readonly string[];
  readonly source: string | undefined;
  readonly vendor: string | undefined;
  readonly model: string | undefined;
  readonly hardware: string | undefined;
  readonly firmware: string | undefined;
  readonly serial: string | undefined;
  readonly upS: number | undefined;
  readonly readAt: number | undefined;
  readonly path: Path | undefined;
  readonly radios: readonly Radio[];
  readonly ports: readonly Port[];
  readonly speed: Speed | undefined;
  readonly modem: Modem | undefined;
}

/** One end of a link: its device, and the radio or port it uses there. */
interface End {
  readonly device: string;
  readonly radio: string | undefined;
  readonly port: string | undefined;
}

/** A link between two devices. */
interface Link {
  readonly a: End;
  readonly b: End;
  readonly medium: string;
  readonly source: string | undefined;
  readonly channel: number | undefined;
  readonly speedMbps: number | undefined;
  readonly signalDbm: number | undefined;
  /** How many devices' routes to the internet cross it, as net counts them. */
  readonly crossings: number;
}

/** One place net reads, and how it stands. */
interface Source {
  readonly name: string;
  readonly what: string | undefined;
  readonly state: string;
  readonly readAt: number | undefined;
  readonly reason: string | undefined;
}

/** One hour of what the internet carried. */
interface Hour {
  readonly downMbps: number;
  readonly upMbps: number;
}

/** What the internet carried over the hours net keeps. */
interface Carried {
  readonly hours: readonly Hour[];
  readonly downBytes: number;
  readonly upBytes: number;
}

/** The internet as eero last measured it, and what it carried lately. */
interface Internet {
  readonly provider: string | undefined;
  readonly downMbps: number | undefined;
  readonly upMbps: number | undefined;
  readonly measuredAt: number | undefined;
  readonly source: string | undefined;
  readonly carried: Carried | undefined;
}

/** Something net names worth a look, in its own words. */
interface Concern {
  readonly kind: string;
  readonly device: string;
  readonly on: string | undefined;
  readonly other: string | undefined;
  readonly says: string;
  /** Its place in net's order, worst first from 0. */
  readonly rank: number;
}

/** The whole network as the page draws it. */
interface Network {
  readonly devices: readonly Device[];
  readonly links: readonly Link[];
  readonly sources: readonly Source[];
  readonly internet: Internet | undefined;
  readonly concerns: readonly Concern[];
  /** The newest time any source was read, in seconds since the epoch. */
  readonly readAt: number | undefined;
  /** What the records held that couldn't be read, for the page to say. */
  readonly unread: readonly string[];
}

/** The watches the page holds, one a kind net serves it. */
interface Watched {
  readonly device: Held;
  readonly link: Held;
  readonly source: Held;
  readonly internet: Held;
  readonly concern: Held;
}

const KINDS = ["device", "link", "source", "internet", "concern"] as const;

/** The objects in `record`'s field `field`, leaving out anything else. */
function objects(record: Fields, field: string): readonly Fields[] {
  const value = record[field];
  return Array.isArray(value) ? value.filter(isObject) : [];
}

/** Each of `values` read by `reader`, leaving out those it couldn't. */
function read<T>(values: readonly Fields[], reader: (value: Fields) => T | undefined): T[] {
  return values.flatMap((value) => {
    const got = reader(value);
    return got === undefined ? [] : [got];
  });
}

function narrowest(value: Fields | undefined): Narrowest | undefined {
  const a = value === undefined ? undefined : text(value, "a");
  const room = value === undefined ? undefined : number(value, "room_mbps");
  if (value === undefined || a === undefined || room === undefined) {
    return undefined;
  }
  return { a, b: text(value, "b"), roomMbps: room, unmeasured: number(value, "unmeasured") ?? 0 };
}

function pathOf(value: Fields | undefined): Path | undefined {
  return value === undefined
    ? undefined
    : {
        through: texts(value, "through") ?? [],
        down: narrowest(object(value, "down")),
        up: narrowest(object(value, "up")),
      };
}

function radioOf(value: Fields): Radio | undefined {
  const name = text(value, "name");
  return name === undefined
    ? undefined
    : {
        name,
        channel: number(value, "channel"),
        widthMhz: number(value, "width_mhz"),
        busyPct: number(value, "busy_pct"),
        busiestPct: number(value, "busiest_pct"),
        busiestAt: number(value, "busiest_at"),
      };
}

function portOf(value: Fields): Port | undefined {
  const name = text(value, "name");
  return name === undefined
    ? undefined
    : {
        name,
        kind: text(value, "kind"),
        link: text(value, "link"),
        speedMbps: number(value, "speed_mbps"),
        duplex: text(value, "duplex"),
      };
}

function speedOf(value: Fields | undefined): Speed | undefined {
  return value === undefined
    ? undefined
    : {
        downMbps: number(value, "down_mbps"),
        upMbps: number(value, "up_mbps"),
        busiestDownMbps: number(value, "busiest_down_mbps"),
        busiestDownAt: number(value, "busiest_down_at"),
        busiestUpMbps: number(value, "busiest_up_mbps"),
        busiestUpAt: number(value, "busiest_up_at"),
      };
}

function channelOf(value: Fields): Channel | undefined {
  const channel = number(value, "channel");
  const kind = text(value, "kind");
  return channel === undefined || kind === undefined
    ? undefined
    : {
        channel,
        kind,
        active: value.active !== false,
        powerDbmv: number(value, "power_dbmv"),
        snrDb: number(value, "snr_db"),
        uncorrectable: number(value, "uncorrectable"),
        lostLately: number(value, "lost_lately"),
      };
}

function modemOf(value: Fields | undefined): Modem | undefined {
  return value === undefined
    ? undefined
    : {
        docsis: text(value, "docsis"),
        networkAccess: text(value, "network_access"),
        address: text(value, "address"),
        downstream: read(objects(value, "downstream"), channelOf),
        upstream: read(objects(value, "upstream"), channelOf),
        latelyS: number(value, "lately_s"),
      };
}

function deviceOf(record: Record): Device | undefined {
  const id = text(record, "id");
  const name = text(record, "name");
  if (id === undefined || name === undefined) {
    return undefined;
  }
  return {
    id,
    name,
    roles: texts(record, "roles") ?? [],
    addresses: texts(record, "addresses") ?? [],
    macs: texts(record, "macs") ?? [],
    source: text(record, "source"),
    vendor: text(record, "vendor"),
    model: text(record, "model"),
    hardware: text(record, "hardware"),
    firmware: text(record, "firmware"),
    serial: text(record, "serial"),
    upS: number(record, "up_s"),
    readAt: number(record, "read_at"),
    path: pathOf(object(record, "path")),
    radios: read(objects(record, "radios"), radioOf),
    ports: read(objects(record, "ports"), portOf),
    speed: speedOf(object(record, "speed")),
    modem: modemOf(object(record, "modem")),
  };
}

function endOf(value: Fields | undefined): End | undefined {
  const device = value === undefined ? undefined : text(value, "device");
  return value === undefined || device === undefined
    ? undefined
    : { device, radio: text(value, "radio"), port: text(value, "port") };
}

function linkOf(record: Record): Link | undefined {
  const a = endOf(object(record, "a"));
  const b = endOf(object(record, "b"));
  const medium = text(record, "medium");
  if (a === undefined || b === undefined || medium === undefined) {
    return undefined;
  }
  return {
    a,
    b,
    medium,
    source: text(record, "source"),
    channel: number(record, "channel"),
    speedMbps: number(record, "speed_mbps"),
    signalDbm: number(record, "signal_dbm"),
    crossings: number(record, "crossings") ?? 0,
  };
}

function sourceOf(record: Record): Source | undefined {
  const name = text(record, "name");
  const state = text(record, "state");
  return name === undefined || state === undefined
    ? undefined
    : {
        name,
        state,
        what: text(record, "what"),
        readAt: number(record, "read_at"),
        reason: text(record, "reason"),
      };
}

function hourOf(value: Fields): Hour | undefined {
  const down = number(value, "down_mbps");
  const up = number(value, "up_mbps");
  return down === undefined || up === undefined ? undefined : { downMbps: down, upMbps: up };
}

function carriedOf(value: Fields | undefined): Carried | undefined {
  const down = value === undefined ? undefined : number(value, "down_bytes");
  const up = value === undefined ? undefined : number(value, "up_bytes");
  return value === undefined || down === undefined || up === undefined
    ? undefined
    : { hours: read(objects(value, "hours"), hourOf), downBytes: down, upBytes: up };
}

function internetOf(record: Record): Internet {
  return {
    provider: text(record, "provider"),
    downMbps: number(record, "down_mbps"),
    upMbps: number(record, "up_mbps"),
    measuredAt: number(record, "measured_at"),
    source: text(record, "source"),
    carried: carriedOf(object(record, "carried")),
  };
}

/** A concern, its own kind told as `concern` since `kind` is the record's. */
function concernOf(record: Record): Concern | undefined {
  const kind = text(record, "concern");
  const device = text(record, "device");
  const says = text(record, "says");
  return kind === undefined || device === undefined || says === undefined
    ? undefined
    : {
        kind,
        device,
        says,
        on: text(record, "on"),
        other: text(record, "other"),
        // One with no place goes after every one net placed.
        rank: number(record, "rank") ?? Number.MAX_SAFE_INTEGER,
      };
}

/**
 * The network the watches hold, or none until each kind has been told
 * once, so a page never draws half a network as if it were all of it.
 */
function networkOf(watched: Watched, told: ReadonlySet<string>): Network | undefined {
  if (!KINDS.every((kind) => told.has(kind))) {
    return undefined;
  }
  const unread: string[] = [];
  const each = <T>(held: Held, reader: (record: Record) => T | undefined, kind: string): T[] =>
    held.all().flatMap((record) => {
      const got = reader(record);
      if (got === undefined) {
        unread.push(`${kind} ${record.key}`);
        return [];
      }
      return [got];
    });
  const devices = each(watched.device, deviceOf, "device");
  const sources = each(watched.source, sourceOf, "source");
  const [internet] = watched.internet.all();
  const times = [...devices, ...sources].flatMap((one) =>
    one.readAt === undefined ? [] : [one.readAt],
  );
  return {
    devices,
    links: each(watched.link, linkOf, "link"),
    sources: [...sources].sort((one, other) => one.name.localeCompare(other.name)),
    internet: internet === undefined ? undefined : internetOf(internet),
    // A watch hears records by key, so net's worst-first order comes back
    // from each concern's rank.
    concerns: each(watched.concern, concernOf, "concern").sort(
      (one, other) => one.rank - other.rank,
    ),
    readAt: times.length > 0 ? Math.max(...times) : undefined,
    unread,
  };
}

/**
 * What net names worth a look about the device `id`, as either end. The
 * internet's own ceiling isn't one: the page says it under The Internet, so
 * a device's count and its panel's list agree.
 */
function worthALook(network: Network, id: string): readonly Concern[] {
  return network.concerns.filter(
    (concern) => concern.kind !== "ceiling" && (concern.device === id || concern.other === id),
  );
}

export type {
  Carried,
  Channel,
  Concern,
  Device,
  End,
  Hour,
  Internet,
  Link,
  Modem,
  Narrowest,
  Network,
  Path,
  Port,
  Radio,
  Source,
  Speed,
  Watched,
};
export { KINDS, networkOf, worthALook };
