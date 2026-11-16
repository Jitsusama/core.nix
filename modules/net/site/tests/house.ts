// A made-up house told as net's daemon tells it (modules/net/src/serve.rs):
// each record is net's own object with its key and kind, a concern's own
// kind under `concern` and its place worst first under `rank`.

import { Held } from "../../../local-sites/kit/records.ts";
import type { Watched } from "../model.ts";

interface Told {
  readonly key: string;
  readonly kind: string;
  readonly [field: string]: unknown;
}

const records: readonly Told[] = [
  {
    key: "device:modem",
    kind: "device",
    id: "modem",
    name: "Modem",
    roles: ["modem"],
    addresses: ["192.168.100.1"],
    macs: [],
    source: "modem",
    read_at: 1_700_000_100,
    ports: [{ name: "cable", kind: "coax", link: "up" }],
    modem: {
      docsis: "3.1",
      downstream: [
        { channel: 16, kind: "sc-qam", power_dbmv: 2.1, snr_db: 38.6, uncorrectable: 878 },
        { channel: 3, kind: "sc-qam", power_dbmv: 1.5, snr_db: 40.2, uncorrectable: 12 },
      ],
      upstream: [{ channel: 1, kind: "atdma", power_dbmv: 44 }],
    },
  },
  {
    key: "device:eero:1",
    kind: "device",
    id: "eero:1",
    name: "Basement",
    roles: ["router", "access-point"],
    addresses: ["192.168.4.1"],
    macs: ["00:00:5e:00:53:01"],
    source: "eero",
    read_at: 1_700_000_000,
    path: { through: ["modem"], down: { a: "modem", room_mbps: 370, unmeasured: 0 } },
    radios: [{ name: "5 GHz low", channel: 36, width_mhz: 80, busy_pct: 12, busiest_pct: 40 }],
  },
  {
    key: "device:eero:2",
    kind: "device",
    id: "eero:2",
    name: "Upstairs",
    roles: ["access-point"],
    addresses: ["192.168.4.2"],
    macs: [],
    source: "eero",
    path: { through: ["eero:1", "modem"] },
    radios: [{ name: "2.4 GHz", channel: 6, busy_pct: 55 }],
  },
  {
    key: "device:mac:00:00:5e:00:53:10",
    kind: "device",
    id: "mac:00:00:5e:00:53:10",
    name: "Kitchen Printer",
    roles: ["client", "printer"],
    addresses: ["192.168.4.30", "fe80::1"],
    macs: ["00:00:5e:00:53:10"],
    source: "eero",
    path: { through: ["eero:2", "eero:1", "modem"] },
    speed: { down_mbps: 0.4, up_mbps: 0.1, busiest_down_mbps: 3.2, busiest_down_at: 1_700_000_050 },
  },
  // A device net couldn't name is told, and the page says it couldn't read it.
  { key: "device:mac:00:00:5e:00:53:99", kind: "device", id: "mac:00:00:5e:00:53:99" },
  {
    key: "link:eero:1|modem",
    kind: "link",
    a: { device: "eero:1", port: "wan" },
    b: { device: "modem", port: "lan" },
    medium: "ethernet",
    speed_mbps: 1000,
    source: "eero",
    crossings: 3,
  },
  {
    key: "link:eero:2|eero:1",
    kind: "link",
    a: { device: "eero:2" },
    b: { device: "eero:1", radio: "5 GHz low" },
    medium: "wifi",
    speed_mbps: 866,
    channel: 36,
    signal_dbm: -61,
    source: "eero",
  },
  {
    key: "link:mac:00:00:5e:00:53:10|eero:2",
    kind: "link",
    a: { device: "mac:00:00:5e:00:53:10" },
    b: { device: "eero:2", radio: "2.4 GHz" },
    medium: "wifi",
    speed_mbps: 72,
    signal_dbm: -74,
    source: "eero",
  },
  {
    key: "source:modem",
    kind: "source",
    name: "modem",
    what: "the modem's own status pages",
    state: "answering",
    read_at: 1_700_000_200,
  },
  {
    key: "source:eero",
    kind: "source",
    name: "eero",
    what: "eero's own account of the network",
    state: "answering",
    read_at: 1_700_000_000,
  },
  {
    key: "internet",
    kind: "internet",
    provider: "Example ISP",
    down_mbps: 369.86,
    up_mbps: 11.93,
    measured_at: 1_699_990_000,
    source: "eero",
  },
  // Told in key order, as a watch hears them, which isn't net's worst first.
  {
    key: "concern:lost:modem:downstream channel 16",
    kind: "concern",
    concern: "lost",
    device: "modem",
    on: "downstream channel 16",
    says: "The modem's downstream channel 16 has lost 878 pieces of data.",
    rank: 0,
  },
  {
    key: "concern:busy:eero:2:2.4 GHz",
    kind: "concern",
    concern: "busy",
    device: "eero:2",
    on: "2.4 GHz",
    says: "Upstairs's 2.4 GHz radio is busy 55% of the time.",
    rank: 2,
  },
  {
    key: "concern:weak:mac:00:00:5e:00:53:10",
    kind: "concern",
    concern: "weak",
    device: "mac:00:00:5e:00:53:10",
    says: "Kitchen Printer is heard at -74 dBm.",
    rank: 1,
  },
];

/** Each watch told `these` whole, each to its kind's, and the kinds told. */
function told(these: readonly Told[] = records): {
  readonly watched: Watched;
  readonly kinds: ReadonlySet<string>;
} {
  const heldOf = (kind: string): Held => {
    const held = new Held();
    held.took({ records: these.filter((record) => record.kind === kind) });
    return held;
  };
  const watched: Watched = {
    device: heldOf("device"),
    link: heldOf("link"),
    source: heldOf("source"),
    internet: heldOf("internet"),
    concern: heldOf("concern"),
  };
  return { watched, kinds: new Set(these.map((record) => record.kind)) };
}

export type { Told };
export { records, told };
