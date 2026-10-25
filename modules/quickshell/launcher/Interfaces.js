// This machine's connections, made sense of: each interface as ip's JSON
// says it, by what it is (Wi-Fi, wired, a tunnel, a virtual bridge or
// another virtual network), the gateways from the kernel's routing tables,
// the name servers from resolv.conf, and a Wi-Fi card's link as iw reads
// it. Networks.qml and Wireless.qml read the files and run the programs;
// everything here is plain reading of what they said, so it's tested
// offscreen (tests/answers.qml).
.pragma library

const HEX = 16;
const PAIRS = /../g;
const FOURS = /..../g;
const SPACES = /\s+/;
const ZEROS = /^0+$/;
const MIRROR = /^ifb/;
const WIRELESS = /^wl/;
const FIELD = /^\s*([^:]+):\s*(.*)$/;
const SECTION = /^@@ (\w+)$/;
const NUMBER = /-?\d+(?:\.\d+)?/;
const CHAINS = /\[([^\]]*)\]/;
const RATE = /([\d.]+) MBit\/s/;
const WIDTH = /(\d+)MHz/;
const TX_POWER = /txpower ([\d.]+) dBm/;
const CHANNEL = /channel (\d+) \((\d+) MHz\)/;
const ROUTE_FIELDS = 8;
const ROUTE_GATEWAY = 2;
const ROUTE_METRIC = 6;
const SIX_ROUTE_FIELDS = 10;
const SIX_NEXT_HOP = 4;
const SIX_METRIC = 5;
const SIX_DEVICE = 9;
const DEFAULT_FOUR = "00000000";
const DEFAULT_SIX = "00000000000000000000000000000000";
const MINUTE = 60;
const HOUR = 3600;
const DAY = 86_400;
// The weakest and strongest signals a card is likely to hear, in dBm, as
// the ends of a signal's graph.
const FAINTEST = -90;
const LOUDEST = -30;
// Wi-Fi 4's rates name each stream count by eight indexes.
const HT_PER_STREAM = 8;
const WIFI_4 = 4;
const WIFI_5 = 5;
const WIFI_6 = 6;
const WIFI_7 = 7;
// How iw marks a rate of each newer Wi-Fi, the newest first.
const GENERATIONS = [
    { tag: "EHT", generation: WIFI_7 },
    { tag: "HE", generation: WIFI_6 },
    { tag: "VHT", generation: WIFI_5 },
];

// The kinds of link the kernel makes for a tunnel.
const TUNNELS = [
    "tun",
    "wireguard",
    "ipip",
    "gre",
    "gretap",
    "ip6gre",
    "sit",
    "ip6tnl",
    "vti",
    "vxlan",
    "geneve",
];

// What an interface is, as Joel would say it: "wifi", "wired", "tunnel",
// "bridge" or "virtual", or "" for the loopback and the shaper's mirrors,
// which carry nothing of their own. `wireless` is whether the kernel calls
// it a WLAN device, which ip doesn't say.
function kind(link, wireless) {
    const name = String(link.ifname ?? "");
    if (link.link_type === "loopback" || MIRROR.test(name)) {
        return "";
    }
    const made = link.linkinfo?.info_kind ?? "";
    if (wireless) {
        return "wifi";
    }
    if (made === "bridge") {
        return "bridge";
    }
    if (TUNNELS.includes(made) || link.link_type === "none") {
        return "tunnel";
    }
    if (made === "" && link.parentbus) {
        return WIRELESS.test(name) ? "wifi" : "wired";
    }
    return "virtual";
}

// Whether it's turned on and has something at the other end.
function state(link) {
    const flags = link.flags ?? [];
    if (!flags.includes("UP")) {
        return "off";
    }
    return flags.includes("NO-CARRIER") ? "no link" : "up";
}

// Its addresses that reach past its own link, IPv4 and IPv6 apart.
function reaching(link, family) {
    return (link.addr_info ?? [])
        .filter((each) => each.family === family && each.scope === "global")
        .map((each) => ({ address: String(each.local), prefix: Number(each.prefixlen) }));
}

// Every interface ip lists, leaving out what isn't a connection, each as
// { name, kind, state, mtu, mac, own, four, six }: `own` is the hardware
// address the card was made with when it uses another, and `wireless` the
// names the kernel calls WLAN devices.
function interfaces(listed, wireless) {
    if (!Array.isArray(listed)) {
        return [];
    }
    return listed
        .map((link) => ({
            name: String(link.ifname ?? ""),
            kind: kind(link, wireless.includes(link.ifname)),
            state: state(link),
            mtu: Number(link.mtu ?? 0),
            mac: link.link_type === "ether" ? String(link.address ?? "") : "",
            own: String(link.permaddr ?? ""),
            four: reaching(link, "inet"),
            six: reaching(link, "inet6"),
        }))
        .filter((link) => link.kind !== "" && link.name !== "");
}

// An IPv4 address as /proc/net/route writes it, in hex, its bytes reversed.
function flipped(hex) {
    return hex
        .match(PAIRS)
        .reverse()
        .map((pair) => Number.parseInt(pair, HEX))
        .join(".");
}

// Each interface's IPv4 default gateway, by name, from /proc/net/route,
// with its metric, the lowest the way out.
function gateways(text) {
    const found = {};
    for (const line of String(text).split("\n").slice(1)) {
        const fields = line.trim().split(SPACES);
        if (fields.length >= ROUTE_FIELDS && fields[1] === DEFAULT_FOUR) {
            found[fields[0]] = {
                address: flipped(fields[ROUTE_GATEWAY]),
                metric: Number(fields[ROUTE_METRIC]),
            };
        }
    }
    return found;
}

// Each interface's IPv6 default gateway, by name, from /proc/net/ipv6_route,
// in groups of four digits for Addresses.js to shorten.
function gateways6(text) {
    const found = {};
    for (const line of String(text).split("\n")) {
        const fields = line.trim().split(SPACES);
        const hop = fields[SIX_NEXT_HOP] ?? "";
        if (
            fields.length >= SIX_ROUTE_FIELDS &&
            fields[0] === DEFAULT_SIX &&
            fields[1] === "00" &&
            !ZEROS.test(hop) &&
            found[fields[SIX_DEVICE]] === undefined
        ) {
            found[fields[SIX_DEVICE]] = {
                address: hop.match(FOURS).join(":"),
                metric: Number.parseInt(fields[SIX_METRIC], HEX),
            };
        }
    }
    return found;
}

// The interface the machine's traffic leaves by: the one whose default
// route costs least.
function wayOut(routes) {
    const names = Object.keys(routes).sort((a, b) => routes[a].metric - routes[b].metric);
    return names[0] ?? "";
}

// The name servers a resolv.conf names, in its order.
function servers(text) {
    return String(text)
        .split("\n")
        .map((line) => line.trim().split(SPACES))
        .filter((fields) => fields[0] === "nameserver" && fields[1])
        .map((fields) => fields[1]);
}

// Whether those are only systemd-resolved's own stub, which says nothing of
// where names are asked, so resolved's own file is read instead.
function stubbed(said) {
    return said.length > 0 && said.every((server) => server.startsWith("127.0.0.5"));
}

// What a hardware address's kernel says of where it came from
// (addr_assign_type): the card's own, made up at random, taken from another
// device, or set by software.
function assigned(text) {
    return ["own", "random", "taken", "set"][Number.parseInt(text, 10)] ?? "";
}

// A number at the start of what iw says, or null.
function figure(text) {
    const found = NUMBER.exec(String(text ?? ""));
    return found ? Number(found[0]) : null;
}

// A rate as iw says it, "1200.9 MBit/s 80MHz HE-MCS 11 HE-NSS 2", as
// { mbps, width, generation, mcs, streams }: HE is Wi-Fi 6, EHT 7, VHT 5 and
// HT 4, whose index says the streams too; anything older has no generation.
function rate(text) {
    const said = String(text ?? "");
    const field = (key) => {
        const found = new RegExp(`\\b${key} (\\d+)`).exec(said);
        return found ? Number(found[1]) : null;
    };
    const mbps = RATE.exec(said);
    const width = WIDTH.exec(said);
    const known = GENERATIONS.find((each) => field(`${each.tag}-MCS`) !== null);
    const coded = { generation: null, mcs: null, streams: null };
    if (known) {
        coded.generation = known.generation;
        coded.mcs = field(`${known.tag}-MCS`);
        coded.streams = field(`${known.tag}-NSS`);
    } else if (field("MCS") !== null) {
        coded.generation = WIFI_4;
        coded.mcs = field("MCS") % HT_PER_STREAM;
        coded.streams = Math.floor(field("MCS") / HT_PER_STREAM) + 1;
    }
    return Object.assign(
        {
            mbps: mbps ? Number(mbps[1]) : null,
            width: width ? Number(width[1]) : null,
        },
        coded,
    );
}

// What iw said of each part it was asked, by the section the script heads
// it with ("@@ link", "@@ station", "@@ info"), each a map of its fields.
function sections(text) {
    const said = { link: {}, station: {}, info: {}, infoLines: [] };
    let at = "";
    for (const line of String(text).split("\n")) {
        const heading = SECTION.exec(line.trim());
        if (heading) {
            at = String(heading[1]);
        } else if (at !== "" && said[at] !== undefined) {
            const found = FIELD.exec(line);
            if (found && said[at][found[1].trim()] === undefined) {
                said[at][found[1].trim()] = found[2].trim();
            }
            if (at === "info") {
                said.infoLines.push(line.trim());
            }
        }
    }
    return said;
}

// A Wi-Fi card's link as iw reads it: its signal now, at each antenna, on
// average and of the access point's beacons, in dBm; what it transmits at;
// how many frames it sent again, couldn't send and dropped, and beacons it
// missed; how long it's been on, in seconds; whether its management frames
// are protected; and its rates each way. Null when it's on no network.
function radio(text) {
    const said = sections(text);
    const { station } = said;
    if (Object.keys(station).length === 0) {
        return null;
    }
    const chains = CHAINS.exec(station.signal ?? "");
    const info = said.infoLines.join("\n");
    const power = TX_POWER.exec(info);
    const channel = CHANNEL.exec(info);
    return {
        signal: figure(station.signal),
        antennas: chains ? chains[1].split(",").map((each) => Number(each.trim())) : [],
        average: figure(station["signal avg"]),
        beacons: figure(station["beacon signal avg"]),
        power: power ? Number(power[1]) : null,
        retries: figure(station["tx retries"]),
        failed: figure(station["tx failed"]),
        missed: figure(station["beacon loss"]),
        dropped: figure(station["rx drop misc"]),
        connected: figure(station["connected time"]),
        protected: station.MFP === undefined ? null : station.MFP === "yes",
        channel: channel ? Number(channel[1]) : null,
        frequency: figure(said.link.freq),
        receive: rate(station["rx bitrate"]),
        send: rate(station["tx bitrate"]),
    };
}

// How long something's lasted, in days, hours and minutes, the largest two.
function lasted(seconds) {
    const days = Math.floor(seconds / DAY);
    const hours = Math.floor((seconds % DAY) / HOUR);
    const minutes = Math.floor((seconds % HOUR) / MINUTE);
    if (days > 0) {
        return `${days} d ${hours} h`;
    }
    if (hours > 0) {
        return `${hours} h ${minutes} min`;
    }
    return minutes > 0 ? `${minutes} min` : "under a minute";
}

// A signal in dBm as a graph's height, from 0 for the faintest heard to 1
// for the strongest.
function level(dbm) {
    return Math.max(0, Math.min(1, (dbm - FAINTEST) / (LOUDEST - FAINTEST)));
}
