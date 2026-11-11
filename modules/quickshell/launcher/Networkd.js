// What systemd-networkd says over its varlink socket, made sense of: each
// connection's DHCP lease from Describe, and each cable's neighbours from
// GetLLDPNeighbors. Networks.qml asks and hears; everything here is plain
// reading of what it heard, so it's tested offscreen (tests/answers.qml).
.pragma library

const HEX = 16;
const PAIRS = /../g;
// The DHCP options a lease is read for, by their tags (RFC 2132). The
// subnet mask (1) and router (3) are left out, as a connection already says
// them as its address's prefix and its Gateway.
const NAME_SERVERS = 6;
const DOMAIN = 15;
const TIME_SERVERS = 42;
const LEASE_TIME = 51;
const SERVER = 54;
const USEC = 1_000_000;
const BYTE = 256;
const IPV4_BYTES = 4;

// An option's bytes, from the hex networkd says them in.
function octets(hex) {
    return (String(hex ?? "").match(PAIRS) ?? []).map((pair) => Number.parseInt(pair, HEX));
}

// The IPv4 addresses an option holds, four bytes each.
function quads(hex) {
    const said = octets(hex);
    const out = [];
    for (let at = 0; at + IPV4_BYTES <= said.length; at += IPV4_BYTES) {
        out.push(said.slice(at, at + IPV4_BYTES).join("."));
    }
    return out;
}

// A time networkd says in microseconds, in seconds, or null.
function seconds(usec) {
    return typeof usec === "number" ? usec / USEC : null;
}

// What a lease's server offered, from its options by tag: the server,
// how long it's given for, in seconds, and the name servers, time servers
// and domain, each null or empty where it offered none.
function offered(given) {
    const options = {};
    for (const option of given?.Message?.options ?? []) {
        options[option.tag] = String(option.data ?? "");
    }
    const domain = octets(options[DOMAIN]).filter((octet) => octet > 0);
    return {
        server: quads(options[SERVER])[0] ?? null,
        lasts:
            options[LEASE_TIME] === undefined
                ? null
                : octets(options[LEASE_TIME]).reduce((sum, octet) => sum * BYTE + octet, 0),
        nameServers: quads(options[NAME_SERVERS]),
        timeServers: quads(options[TIME_SERVERS]),
        domain: domain.length > 0 ? String.fromCharCode(...domain) : null,
    };
}

// One connection's lease: the server that gave it, how long it's given
// for; when it was got, when it's renewed with that server, rebound with
// any and when it ends, each in seconds since boot, as /proc/uptime counts;
// and the name servers, time servers, domain and machine name the server
// offered. What networkd doesn't say is null, as an older one says only
// the times.
function lease(said) {
    const offer = offered(said);
    const got = seconds(said.LeaseTimestampUSec);
    return {
        server: offer.server,
        lasts: offer.lasts,
        got,
        renews: seconds(said.Timeout1USec),
        rebinds: seconds(said.Timeout2USec),
        ends: got !== null && offer.lasts !== null ? got + offer.lasts : null,
        nameServers: offer.nameServers,
        timeServers: offer.timeServers,
        domain: offer.domain,
        hostname: said.Hostname ?? null,
    };
}

// The DHCP lease on each connection networkd holds one for, by its name,
// from Describe's reply.
function leases(described) {
    const out = {};
    for (const link of described?.Interfaces ?? []) {
        const said = link?.DHCPv4Client?.Lease ?? null;
        if (said !== null) {
            out[link.Name] = lease(said);
        }
    }
    return out;
}

// What a neighbour says it can do, by its bit in LLDP's capabilities
// (IEEE 802.1AB), lowest first.
const ACTS = [
    "other",
    "repeater",
    "bridge",
    "access point",
    "router",
    "telephone",
    "cable modem",
    "station",
    "customer VLAN bridge",
    "service VLAN bridge",
    "two-port MAC relay",
];

// What's at the other end of each cable, by the connection's name, from
// GetLLDPNeighbors' reply: each neighbour's system name and description,
// the port it's heard on and that port's name, its chassis, what it's
// acting as now, its VLAN and its maker's profile's address (RFC 8520), each
// null where it doesn't say.
function neighbours(reply) {
    const out = {};
    for (const link of reply?.Neighbors ?? []) {
        out[link.InterfaceName] = (link.Neighbors ?? []).map((one) => ({
            system: one.SystemName ?? null,
            description: one.SystemDescription ?? null,
            port: one.PortID ?? null,
            portName: one.PortDescription ?? null,
            chassis: one.ChassisID ?? null,
            acts: ACTS.filter(
                (_, bit) => Math.floor(Number(one.EnabledCapabilities ?? 0) / 2 ** bit) % 2 === 1,
            ),
            vlan: one.VlanID ?? null,
            profile: one.MUDURL ?? null,
        }));
    }
    return out;
}

// What a neighbour says a step in, in the order it's said: what it's read
// from, the part's id and its note.
const NEIGHBOUR_PARTS = [
    ["system", "system", "what it calls itself"],
    ["description", "description", "what it says it is"],
    ["port", "port", "the port it hears this card on"],
    ["portName", "port-name", "what it calls that port"],
    ["chassis", "chassis", "what tells it apart from others"],
    ["acts", "acts", "what it's doing now"],
    ["vlan", "vlan", "the VLAN this port is on"],
    ["profile", "profile", "where it says what it should reach"],
];

// A neighbour's parts a step in, each { id, side, note }: a list is said
// joined, a number as it's written, and what it doesn't say is left out.
function neighbourParts(one) {
    const out = [];
    for (const [field, id, note] of NEIGHBOUR_PARTS) {
        const side = [one[field] ?? []].flat().join(", ");
        if (side !== "") out.push({ id, side, note });
    }
    return out;
}
