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

// An IPv4 address's bits and parts, and the values a part holds; and the
// /31 of a link between two, which keeps no broadcast address (RFC 3021).
const FOUR_BITS = 32;
const FOUR_PARTS = 4;
const PART_VALUES = 256;
const NO_BROADCAST = 31;
// An IPv6 address's bits and groups; each group's bits, values, digits in
// hex, and mask; and the top bits of a global unicast first one, 2000::/3.
const SIX_BITS = 128;
const SIX_GROUPS = 8;
const GROUP_BITS = 16;
const GROUP_VALUES = 65_536;
const GROUP_DIGITS = 4;
const HEX = 16;
const GROUP_MASK = 0xff_ff;
const GLOBAL_MASK = 0xe0_00;
const GLOBAL = 0x20_00;
// The digits of a hardware address's maker's OUI, and the bits of its first
// pair that say it's locally run and a group's.
const OUI_DIGITS = 6;
const LOCAL_BIT = 2;
const MULTICAST_BIT = 1;
// An IPv4 address's part; an IPv6 address's zone, as "%wlan0", the IPv4
// address it can end with, and a group; and an address with the prefix its
// network is written with, if any.
const PART = /^\d{1,3}$/;
const ZONE = /%.*$/;
const FOUR_TAIL = /:(\d+\.\d+\.\d+\.\d+)$/;
const GROUP = /^[0-9a-f]{1,4}$/;
const SLASHED = /^([^/\s]+)(?:\/(\d{1,3}))?$/;
// A hardware address with colons or dashes, the same one throughout, or
// with Cisco's dots.
const MAC_SPACED = /^[0-9a-f]{2}([:-])[0-9a-f]{2}(?:\1[0-9a-f]{2}){4}$/;
const MAC_DOTTED = /^([0-9a-f]{4})\.([0-9a-f]{4})\.([0-9a-f]{4})$/;
// What's written between a hardware address's digits, and its digits in
// twos and in fours.
const MAC_MARKS = /[:.-]/g;
const DIGIT_PAIRS = /../g;
const DIGIT_FOURS = /..../g;

// An IPv4 address as a number, or null.
function four(text) {
    const parts = text.split(".");
    if (parts.length !== FOUR_PARTS) {
        return null;
    }
    let value = 0;
    for (const part of parts) {
        if (!PART.test(part) || Number(part) >= PART_VALUES) {
            return null;
        }
        value = value * PART_VALUES + Number(part);
    }
    return value;
}

function fourText(value) {
    return Array.from({ length: FOUR_PARTS }, (_, i) =>
        Math.floor(value / PART_VALUES ** (FOUR_PARTS - 1 - i)) % PART_VALUES).join(".");
}

// An IPv6 address as eight 16-bit groups, or null. A zone, "%wlan0", is
// dropped, and a trailing IPv4 address is read as the last two groups.
function six(text) {
    let rest = text.toLowerCase().replace(ZONE, "");
    const tail = FOUR_TAIL.exec(rest);
    let last = [];
    if (tail) {
        const v4 = four(tail[1]);
        if (v4 === null) {
            return null;
        }
        last = [Math.floor(v4 / GROUP_VALUES), v4 % GROUP_VALUES];
        rest = `${rest.slice(0, rest.length - tail[1].length)}0:0`;
    }
    const halves = rest.split("::");
    if (halves.length > 2) {
        return null;
    }
    const groups = half => half === "" ? [] : half.split(":");
    const head = groups(halves[0]);
    const end = halves.length === 2 ? groups(halves[1]) : [];
    const missing = SIX_GROUPS - head.length - end.length;
    if (halves.length === 1 ? missing !== 0 : missing < 1) {
        return null;
    }
    const all = head.concat(new Array(halves.length === 2 ? missing : 0).fill("0"), end);
    const out = [];
    for (const group of all) {
        if (!GROUP.test(group)) {
            return null;
        }
        out.push(Number.parseInt(group, HEX));
    }
    if (tail) {
        // The IPv4 address's two groups are the last two.
        out.splice(SIX_GROUPS - 2, 2, ...last);
    }
    return out;
}

// The shortest form, as RFC 5952 writes it: the longest run of two or more
// zero groups becomes "::".
function sixText(groups) {
    let best = -1;
    let length = 1;
    for (let i = 0; i < SIX_GROUPS; i += 1) {
        let j = i;
        while (j < SIX_GROUPS && groups[j] === 0) {
            j += 1;
        }
        if (j - i > length) {
            best = i;
            length = j - i;
        }
        i = Math.max(i, j - 1);
    }
    const hex = groups.map(g => g.toString(HEX));
    if (best < 0) {
        return hex.join(":");
    }
    return `${hex.slice(0, best).join(":")}::${hex.slice(best + length).join(":")}`;
}

function sixFull(groups) {
    return groups.map(g => `000${g.toString(HEX)}`.slice(-GROUP_DIGITS)).join(":");
}

// The groups with every bit past the prefix set to `bit`.
function edge(groups, prefix, bit) {
    return groups.map((g, i) => {
        const kept = Math.max(0, Math.min(GROUP_BITS, prefix - GROUP_BITS * i));
        // biome-ignore lint/suspicious/noBitwiseOperators: a mask keeps a group's prefix bits.
        const mask = kept === 0 ? 0 : (GROUP_MASK << (GROUP_BITS - kept)) & GROUP_MASK;
        // biome-ignore lint/suspicious/noBitwiseOperators: the bits past the prefix are set.
        return bit ? (g & mask) | (~mask & GROUP_MASK) : g & mask;
    });
}

function within4(value, range) {
    const [base, prefix] = range.split("/");
    const size = 2 ** (FOUR_BITS - Number(prefix));
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
    if (hit) {
        return { kind: hit[1], rfc: hit[2], public: false };
    }
    // biome-ignore lint/suspicious/noBitwiseOperators: a first group's top bits are its range.
    const global = (groups[0] & GLOBAL_MASK) === GLOBAL;
    return global ? { kind: "public", rfc: "", public: true }
        : { kind: "reserved", rfc: "RFC 4291", public: false };
}

// An IPv4 address or network, read, as read() gives it, or null when the
// prefix is too long for one.
function read4(v4, prefix) {
    if (prefix !== null && prefix > FOUR_BITS) {
        return null;
    }
    const bits = prefix ?? FOUR_BITS;
    const size = 2 ** (FOUR_BITS - bits);
    const first = Math.floor(v4 / size) * size;
    const last = first + size - 1;
    return {
        version: 4,
        text: fourText(v4),
        prefix,
        network: `${fourText(first)}/${bits}`,
        aligned: first === v4,
        first: fourText(bits >= NO_BROADCAST ? first : first + 1),
        last: fourText(bits >= NO_BROADCAST ? last : last - 1),
        broadcast: fourText(last),
        mask: fourText(2 ** FOUR_BITS - size),
        size,
        usable: bits >= NO_BROADCAST ? size : size - 2,
        kind: kind4(first),
        whole: kind4(last)
    };
}

// An address or network, read: { version, address, prefix, network } with
// the forms worth showing, or null when it isn't one.
function read(text) {
    const slash = SLASHED.exec(text.trim());
    if (!slash) {
        return null;
    }
    const prefix = slash[2] === undefined ? null : Number(slash[2]);
    const v4 = four(slash[1]);
    if (v4 !== null) {
        return read4(v4, prefix);
    }
    const groups = six(slash[1]);
    if (groups === null || (prefix !== null && prefix > SIX_BITS)) {
        return null;
    }
    const bits = prefix ?? SIX_BITS;
    const low = edge(groups, bits, false);
    const high = edge(groups, bits, true);
    return {
        version: 6,
        text: sixText(groups),
        full: sixFull(groups),
        prefix,
        network: `${sixText(low)}/${bits}`,
        aligned: low.every((g, i) => g === groups[i]),
        first: sixText(low),
        last: sixText(high),
        // How many /64s, the size of one network, or addresses below that.
        bits: SIX_BITS - bits,
        kind: kind6(low),
        whole: kind6(high)
    };
}

// A hardware address, read: { oui, colon, dotted, local, multicast }, or
// null. Written with colons, dashes, Cisco's dots, or none at all.
function mac(text) {
    const typed = text.trim().toLowerCase();
    if (!(MAC_SPACED.test(typed) || MAC_DOTTED.test(typed))) {
        return null;
    }
    // The digits, without the colons, dashes or dots written between them.
    const hex = typed.replace(MAC_MARKS, "");
    const first = Number.parseInt(hex.slice(0, 2), HEX);
    return {
        oui: hex.slice(0, OUI_DIGITS).toUpperCase(),
        colon: hex.match(DIGIT_PAIRS).join(":"),
        dotted: hex.match(DIGIT_FOURS).join("."),
        // biome-ignore lint/suspicious/noBitwiseOperators: the first pair's bits are its flags.
        local: (first & LOCAL_BIT) !== 0,
        // biome-ignore lint/suspicious/noBitwiseOperators: the first pair's bits are its flags.
        multicast: (first & MULTICAST_BIT) !== 0
    };
}
