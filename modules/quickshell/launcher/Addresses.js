// IPv4 and IPv6 addresses and networks, and hardware (MAC) addresses, for
// the launcher's answers, in plain JavaScript so a check can run it in Qt's
// own engine, as the shell does.
//
// An address is read whole or not at all: "10.0.0.0/22", "8.8.8.8",
// "2001:db8::/48", "fe80::1%wlan0", "3c:22:fb:12:34:56". Each says what
// sort of range it's in, by IANA's special-purpose registries, so the
// launcher knows a private address is never worth asking the world about.
.pragma library

// IANA's special-purpose ranges, most specific first where they nest.
const special4 = [
    ["0.0.0.0/8", "this network", "RFC 791"],
    ["10.0.0.0/8", "private", "RFC 1918"],
    ["100.64.0.0/10", "shared, behind carrier NAT", "RFC 6598"],
    ["127.0.0.0/8", "loopback", "RFC 1122"],
    ["169.254.0.0/16", "link-local", "RFC 3927"],
    ["172.16.0.0/12", "private", "RFC 1918"],
    ["192.0.0.0/24", "IETF protocol assignments", "RFC 6890"],
    ["192.0.2.0/24", "documentation only", "RFC 5737"],
    ["192.88.99.0/24", "6to4 relay, retired", "RFC 7526"],
    ["192.168.0.0/16", "private", "RFC 1918"],
    ["198.18.0.0/15", "benchmarking", "RFC 2544"],
    ["198.51.100.0/24", "documentation only", "RFC 5737"],
    ["203.0.113.0/24", "documentation only", "RFC 5737"],
    ["224.0.0.0/4", "multicast", "RFC 5771"],
    ["255.255.255.255/32", "broadcast", "RFC 919"],
    ["240.0.0.0/4", "reserved", "RFC 1112"]
];
const special6 = [
    ["::/128", "unspecified", "RFC 4291"],
    ["::1/128", "loopback", "RFC 4291"],
    ["::ffff:0:0/96", "an IPv4 address, mapped", "RFC 4291"],
    ["64:ff9b::/96", "IPv4 translation", "RFC 6052"],
    ["64:ff9b:1::/48", "local IPv4 translation", "RFC 8215"],
    ["100::/64", "discard only", "RFC 6666"],
    ["2001::/32", "Teredo", "RFC 4380"],
    ["2001:db8::/32", "documentation only", "RFC 3849"],
    ["2002::/16", "6to4", "RFC 3056"],
    ["3fff::/20", "documentation only", "RFC 9637"],
    ["fc00::/7", "unique local", "RFC 4193"],
    ["fe80::/10", "link-local", "RFC 4291"],
    ["ff00::/8", "multicast", "RFC 4291"]
];

// An IPv4 address as a number, or null.
function four(text) {
    const parts = text.split(".");
    if (parts.length !== 4)
        return null;
    let value = 0;
    for (const part of parts) {
        if (!/^\d{1,3}$/.test(part) || Number(part) > 255)
            return null;
        value = value * 256 + Number(part);
    }
    return value;
}

function fourText(value) {
    return [24, 16, 8, 0].map(shift => Math.floor(value / Math.pow(2, shift)) % 256).join(".");
}

// An IPv6 address as eight 16-bit groups, or null. A zone, "%wlan0", is
// dropped, and a trailing IPv4 address is read as the last two groups.
function six(text) {
    let rest = text.toLowerCase().replace(/%.*$/, "");
    const tail = /:(\d+\.\d+\.\d+\.\d+)$/.exec(rest);
    let last = [];
    if (tail) {
        const v4 = four(tail[1]);
        if (v4 === null)
            return null;
        last = [Math.floor(v4 / 65536), v4 % 65536];
        rest = rest.slice(0, rest.length - tail[1].length) + "0:0";
    }
    const halves = rest.split("::");
    if (halves.length > 2)
        return null;
    const groups = half => half === "" ? [] : half.split(":");
    const head = groups(halves[0]);
    const end = halves.length === 2 ? groups(halves[1]) : [];
    const missing = 8 - head.length - end.length;
    if (halves.length === 1 ? missing !== 0 : missing < 1)
        return null;
    const all = head.concat(Array(halves.length === 2 ? missing : 0).fill("0"), end);
    const out = [];
    for (const group of all) {
        if (!/^[0-9a-f]{1,4}$/.test(group))
            return null;
        out.push(parseInt(group, 16));
    }
    if (tail) {
        out[6] = last[0];
        out[7] = last[1];
    }
    return out;
}

// The shortest form, as RFC 5952 writes it: the longest run of two or more
// zero groups becomes "::".
function sixText(groups) {
    let best = -1;
    let length = 1;
    for (let i = 0; i < 8; i++) {
        let j = i;
        while (j < 8 && groups[j] === 0)
            j++;
        if (j - i > length) {
            best = i;
            length = j - i;
        }
        i = Math.max(i, j - 1);
    }
    const hex = groups.map(g => g.toString(16));
    if (best < 0)
        return hex.join(":");
    return hex.slice(0, best).join(":") + "::" + hex.slice(best + length).join(":");
}

function sixFull(groups) {
    return groups.map(g => ("000" + g.toString(16)).slice(-4)).join(":");
}

// The groups with every bit past the prefix set to `bit`.
function edge(groups, prefix, bit) {
    return groups.map((g, i) => {
        const kept = Math.max(0, Math.min(16, prefix - 16 * i));
        const mask = kept === 0 ? 0 : (0xffff << (16 - kept)) & 0xffff;
        return bit ? (g & mask) | (~mask & 0xffff) : g & mask;
    });
}

function within4(value, range) {
    const [base, prefix] = range.split("/");
    const size = Math.pow(2, 32 - Number(prefix));
    return Math.floor(value / size) === Math.floor(four(base) / size);
}

function within6(groups, range) {
    const [base, prefix] = range.split("/");
    const a = edge(groups, Number(prefix), false);
    const b = edge(six(base), Number(prefix), false);
    return a.every((g, i) => g === b[i]);
}

// What sort of range an address is in: { kind, rfc }, or the public
// internet's, which has no RFC to name.
function kind4(value) {
    const hit = special4.find(range => within4(value, range[0]));
    return hit ? { kind: hit[1], rfc: hit[2], public: false }
        : { kind: "public", rfc: "", public: true };
}

function kind6(groups) {
    const hit = special6.find(range => within6(groups, range[0]));
    if (hit)
        return { kind: hit[1], rfc: hit[2], public: false };
    const global = (groups[0] & 0xe000) === 0x2000;
    return global ? { kind: "public", rfc: "", public: true }
        : { kind: "reserved", rfc: "RFC 4291", public: false };
}

// An address or network, read: { version, address, prefix, network } with
// the forms worth showing, or null when it isn't one.
function read(text) {
    const typed = text.trim();
    const slash = /^([^/\s]+)(?:\/(\d{1,3}))?$/.exec(typed);
    if (!slash)
        return null;
    const prefix = slash[2] === undefined ? null : Number(slash[2]);
    const v4 = four(slash[1]);
    if (v4 !== null) {
        if (prefix !== null && prefix > 32)
            return null;
        const bits = prefix ?? 32;
        const size = Math.pow(2, 32 - bits);
        const first = Math.floor(v4 / size) * size;
        const last = first + size - 1;
        const usable = bits >= 31 ? size : size - 2;
        return {
            version: 4,
            text: fourText(v4),
            prefix: prefix,
            network: `${fourText(first)}/${bits}`,
            aligned: first === v4,
            first: fourText(bits >= 31 ? first : first + 1),
            last: fourText(bits >= 31 ? last : last - 1),
            broadcast: fourText(last),
            mask: fourText(Math.pow(2, 32) - size),
            size: size,
            usable: usable,
            kind: kind4(first),
            whole: kind4(last)
        };
    }
    const groups = six(slash[1]);
    if (groups === null)
        return null;
    if (prefix !== null && prefix > 128)
        return null;
    const bits = prefix ?? 128;
    const low = edge(groups, bits, false);
    const high = edge(groups, bits, true);
    return {
        version: 6,
        text: sixText(groups),
        full: sixFull(groups),
        prefix: prefix,
        network: `${sixText(low)}/${bits}`,
        aligned: low.every((g, i) => g === groups[i]),
        first: sixText(low),
        last: sixText(high),
        // How many /64s, the size of one network, or addresses below that.
        bits: 128 - bits,
        kind: kind6(low),
        whole: kind6(high)
    };
}

// A hardware address, read: { oui, colon, dotted, local, multicast }, or
// null. Written with colons, dashes, Cisco's dots, or none at all.
function mac(text) {
    const typed = text.trim().toLowerCase();
    const pair = "([0-9a-f]{2})";
    const forms = [
        new RegExp(`^${pair}([:-])${pair}` + `\\2${pair}`.repeat(4) + "$"),
        /^([0-9a-f]{4})\.([0-9a-f]{4})\.([0-9a-f]{4})$/
    ];
    let hex = null;
    const spaced = forms[0].exec(typed);
    if (spaced)
        hex = [1, 3, 4, 5, 6, 7].map(i => spaced[i]).join("");
    const dotted = forms[1].exec(typed);
    if (dotted)
        hex = dotted.slice(1).join("");
    if (hex === null)
        return null;
    const first = parseInt(hex.slice(0, 2), 16);
    return {
        oui: hex.slice(0, 6).toUpperCase(),
        colon: hex.match(/../g).join(":"),
        dotted: hex.match(/..../g).join("."),
        local: (first & 2) !== 0,
        multicast: (first & 1) !== 0
    };
}
