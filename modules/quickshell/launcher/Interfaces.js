// This machine's connections, made sense of: each interface as ip's JSON
// says it, by what it is (Wi-Fi, wired, a tunnel, a virtual bridge or
// another virtual network), named the same from sysfs for the network's
// speed, the gateways from the kernel's routing tables, the name servers
// from resolv.conf, and how a signal and a time lasted are told. A Wi-Fi
// card's own link is wifid's, served ready (Wireless.qml). Networks.qml
// and Traffic.qml read the files and run the programs; everything here is
// plain reading of what they said, so it's tested offscreen
// (tests/answers.qml).
.pragma library

const HEX = 16;
const PAIRS = /../g;
const FOURS = /..../g;
const SPACES = /\s+/;
const ZEROS = /^0+$/;
const MIRROR = /^ifb/;
const WIRELESS = /^wl/;
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

// The kinds of tunnel sysfs names only by their link-layer type, as
// /sys/class/net's `type` numbers it, by the kind ip would say.
const TUNNEL_TYPES = {
    "768": "ipip",
    "769": "ip6tnl",
    "776": "sit",
    "778": "gre",
    "823": "ip6gre",
};
// The loopback's link-layer type, and that of a link with none, as a VPN's.
const LOOPBACK_TYPE = "772";
const BARE_TYPE = "65534";
const DEVTYPE = /^DEVTYPE=(.*)$/m;

// What an interface is, as Joel would say it: "wifi", "wired", "tunnel",
// "bridge" or "virtual", or "" for the loopback and the shaper's mirrors,
// which carry nothing of their own. It's told { name, loopback, wireless,
// made, bare, card }: whether it's the loopback, whether the kernel calls
// it a WLAN device, the kind of link it was made as (ip's info_kind),
// whether it has no link-layer type at all, and whether it sits on a bus
// as a card does. The addresses learn these from ip (`kind`) and the
// network's speed from sysfs (`sysKind`), and both come here, so each names
// an interface as the other does.
function kindOf(link) {
    if (link.loopback || MIRROR.test(link.name)) {
        return "";
    }
    if (link.wireless) {
        return "wifi";
    }
    if (link.made === "bridge") {
        return "bridge";
    }
    if (TUNNELS.includes(link.made) || link.bare) {
        return "tunnel";
    }
    if (link.made === "" && link.card) {
        return WIRELESS.test(link.name) ? "wifi" : "wired";
    }
    return "virtual";
}

// What an interface is, from ip's JSON of it. `wireless` is whether the
// kernel calls it a WLAN device, which ip doesn't say.
function kind(link, wireless) {
    return kindOf({
        name: String(link.ifname ?? ""),
        loopback: link.link_type === "loopback",
        wireless,
        made: link.linkinfo?.info_kind ?? "",
        bare: link.link_type === "none",
        card: Boolean(link.parentbus),
    });
}

// What an interface is, from its folder in /sys/class/net, read without
// spawning anything: { uevent, type, tun, card }, what its `uevent` and
// `type` say, whether it has `tun_flags` (a tun or tap device) and whether
// it has a `device` (a card's). sysfs says the kind of link only for some:
// a bridge and some tunnels in its uevent, a tun by its flags, the rest of
// the tunnels by their type; anything else ip calls by a kind of its own
// is virtual here as there, since it's no card.
function sysKind(name, sys) {
    const said = DEVTYPE.exec(String(sys.uevent ?? ""));
    const devtype = said ? said[1].trim() : "";
    const type = String(sys.type ?? "").trim();
    let made = TUNNEL_TYPES[type] ?? "";
    if (devtype === "bridge" || TUNNELS.includes(devtype)) {
        made = devtype;
    } else if (sys.tun) {
        made = "tun";
    }
    return kindOf({
        name: String(name),
        loopback: type === LOOPBACK_TYPE,
        wireless: devtype === "wlan",
        made,
        bare: type === BARE_TYPE,
        card: Boolean(sys.card),
    });
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

// The kinds of error a cable counts each way, by the file in its
// statistics/ folder, in the order they're told: what came in broken, then
// what couldn't go out. Each total (rx_errors, tx_errors) counts them all,
// as the kernel's drivers keep them.
const ERRORS_IN = [
    ["damaged", "rx_crc_errors"],
    ["misaligned", "rx_frame_errors"],
    ["wrong-length", "rx_length_errors"],
    ["overrun", "rx_over_errors"],
    ["missed", "rx_missed_errors"],
    ["queue-in", "rx_fifo_errors"],
];
const ERRORS_OUT = [
    ["aborted", "tx_aborted_errors"],
    ["no-link", "tx_carrier_errors"],
    ["queue-out", "tx_fifo_errors"],
    ["heartbeat", "tx_heartbeat_errors"],
    ["late", "tx_window_errors"],
    ["collisions", "collisions"],
];
const WHOLE = /^\d+$/;
const MEGA = 1000;
const TENS = 10;

// A whole count a sysfs file says, or null where it says none or can't be
// read, as a link that's down can't.
function counted(text) {
    const said = String(text ?? "").trim();
    return WHOLE.test(said) ? Number(said) : null;
}

// A cable's link as its folder in /sys/class/net says it, `read` giving a
// file's text by its name there, or "": how it stands, from its carrier;
// its speed in Mb/s and its duplex, null where the driver doesn't know
// them; how many times its link was lost since the card started; and its
// errors, each way in all, the kinds it had in their order, each
// { id, count, way }, and what it set aside each way, which aren't errors.
function cable(read) {
    const carrier = String(read("carrier")).trim();
    const speed = counted(read("speed"));
    const duplex = String(read("duplex")).trim();
    const stat = (file) => counted(read(`statistics/${file}`));
    const kinds = ERRORS_IN.map(([id, file]) => ({ id, count: stat(file), way: "in" }))
        .concat(ERRORS_OUT.map(([id, file]) => ({ id, count: stat(file), way: "out" })))
        .filter((each) => each.count !== null && each.count > 0);
    let stands = "off";
    if (carrier === "1") {
        stands = "up";
    } else if (carrier === "0") {
        stands = "no link";
    }
    return {
        state: stands,
        speed: speed !== null && speed > 0 ? speed : null,
        duplex: duplex === "full" || duplex === "half" ? duplex : null,
        lost: counted(read("carrier_down_count")),
        errors: {
            in: stat("rx_errors"),
            out: stat("tx_errors"),
            kinds,
            droppedIn: stat("rx_dropped"),
            droppedOut: stat("tx_dropped"),
        },
    };
}

// A link's rate, given in Mb/s, in its own units: "100 Mb/s", "1 Gb/s",
// "2.5 Gb/s". `number` writes a figure to so many places, as the
// launcher's settings group and point it.
function linkRate(mbps, number) {
    if (mbps < MEGA) {
        return `${number(mbps, 0)} Mb/s`;
    }
    const gbps = mbps / MEGA;
    const whole = Math.round(gbps * TENS) % TENS === 0;
    return `${number(gbps, whole ? 0 : 1)} Gb/s`;
}

// What a hardware address's kernel says of where it came from
// (addr_assign_type): the card's own, made up at random, taken from another
// device, or set by software.
function assigned(text) {
    return ["own", "random", "taken", "set"][Number.parseInt(text, 10)] ?? "";
}

// How long something's lasted, in days, hours and minutes, the largest two,
// leaving out the smaller when it's none: "20 h", not "20 h 0 min".
function lasted(seconds) {
    const days = Math.floor(seconds / DAY);
    const hours = Math.floor((seconds % DAY) / HOUR);
    const minutes = Math.floor((seconds % HOUR) / MINUTE);
    if (days > 0) {
        return hours > 0 ? `${days} d ${hours} h` : `${days} d`;
    }
    if (hours > 0) {
        return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
    }
    return minutes > 0 ? `${minutes} min` : "under a minute";
}

// A signal in dBm as a graph's height, from 0 for the faintest heard to 1
// for the strongest.
function level(dbm) {
    return Math.max(0, Math.min(1, (dbm - FAINTEST) / (LOUDEST - FAINTEST)));
}
