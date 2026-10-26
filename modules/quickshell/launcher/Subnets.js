// Subnet maths for the launcher's answers, built on Addresses.js's reading
// of an address, in plain JavaScript so a check can run it in Qt's own
// engine, as the shell does.
//
// What's asked is read whole or not at all: a network by its prefix,
// "10.0.0.0/22", or by its mask, "10.0.0.0 255.255.252.0"; an address in
// one, "10.0.0.5/22", which says the network holding it; "is 10.1.2.3 in
// 10.0.0.0/16"; a prefix alone, "/22"; and a network split into smaller
// ones, "10.0.0.0/22 in /24s". A part is { id, side, copy }, named by its id
// in the vocabulary (words.json's networks:*), and a split's subnet is
// { name, side, copy }, named by itself. An answer about an address or a
// network carries Addresses.js's reading of it as `at`, since the kind of
// range it's in, and who holds it, are the provider's to add.
.pragma library
.import "Addresses.js" as Addresses
.import "Numbers.js" as Numbers

// The two versions of IP an address is read as.
const IPV4 = 4;
const IPV6 = 6;
// An IPv4 address's bits, and the /31 of a link, which keeps no broadcast
// address (RFC 3021); an IPv6 address's bits, and one network's prefix.
const FOUR_BITS = 32;
const NO_BROADCAST = 31;
const SIX_BITS = 128;
const ONE_NETWORK = 64;
// An IPv6 address's groups, each group's bits and the values it holds.
const SIX_GROUPS = 8;
const GROUP_BITS = 16;
const GROUP_VALUES = 65_536;
// The most bits a count can have and still be written whole exactly.
const EXACT_BITS = 52;
// How many of a split's subnets are listed, so a split into thousands
// stays a list worth reading.
const LISTED = 256;

// The word that leads the ip command, as the commands list it: "ip
// 10.0.0.0/22".
const COMMAND = /^ip\s+(?=\S)/i;
// An address with its mask written out, after a space or a slash.
const MASKED = /^([^\s/]+)(?:\s+|\/)(\d+\.\d+\.\d+\.\d+)$/;
// Whether a network holds an address, asked: "is 10.1.2.3 in 10.0.0.0/16".
const INSIDE = /^is\s+(\S+)\s+(?:in|inside|within)\s+(.+?)\s*\??$/i;
// A prefix alone: "/22".
const PREFIX = /^\/(\d{1,3})$/;
// A network split by a longer prefix: "10.0.0.0/22 in /24s".
const SPLIT = /^(?:split\s+)?(.+?)\s+(?:in|into|as|to)\s+\/(\d{1,3})s?$/i;

function grouped(value, settings) {
    return Numbers.format(value, 0, settings.group, settings.point);
}

// How many of something there are in 2^bits, written whole while it can
// be, and as a power of two past that.
function counted(bits, settings) {
    return bits <= EXACT_BITS ? grouped(2 ** bits, settings) : `2^${bits}`;
}

function part(id, side, copy) {
    return { id, side, copy: copy ?? side };
}

// A range as it's read, and as it's pasted where a range is taken.
function range(first, last) {
    return part("range", `${first} \u2013 ${last}`, `${first}-${last}`);
}

// The prefix a mask written out stands for, or null when its bits don't
// run unbroken from the top.
function prefixOf(mask) {
    const value = Addresses.four(mask);
    for (let bits = 0; value !== null && bits <= FOUR_BITS; bits += 1) {
        if (2 ** FOUR_BITS - 2 ** (FOUR_BITS - bits) === value) {
            return bits;
        }
    }
    return null;
}

// An address or network as Addresses.js reads one, its mask written out
// or its prefix, or null.
function network(text) {
    const masked = MASKED.exec(text);
    if (!masked) {
        return Addresses.read(text);
    }
    const bits = prefixOf(masked[2]);
    return bits === null ? null : Addresses.read(`${masked[1]}/${bits}`);
}

// A network's lowest address, without its prefix.
function base(at) {
    return at.network.split("/")[0];
}

// The parts of an IPv4 network, in the order a network is told: its range,
// mask and wildcard, the network and its broadcast, its first and last
// hosts, and how many it holds. `hosts` is whether the headline leaves
// the count of hosts unsaid.
function fourParts(at, hosts, settings) {
    const rows = [range(base(at), at.broadcast), part("mask", at.mask),
        part("wildcard", Addresses.fourText(at.size - 1)), part("network", at.network)];
    if (at.prefix < NO_BROADCAST) {
        rows.push(part("broadcast", at.broadcast));
    }
    rows.push(part("first", at.first), part("last", at.last));
    if (hosts) {
        rows.push(part("hosts", grouped(at.usable, settings), String(at.usable)));
    }
    rows.push(part("size", `${grouped(at.size, settings)} addresses`, String(at.size)));
    return rows;
}

// How many /64s a network of `bits` holds, or the addresses in one smaller.
function networks(bits, settings) {
    if (bits === SIX_BITS - ONE_NETWORK) {
        return "one /64";
    }
    if (bits > SIX_BITS - ONE_NETWORK) {
        return `${counted(bits - (SIX_BITS - ONE_NETWORK), settings)} /64s`;
    }
    return `${counted(bits, settings)} addresses`;
}

// The parts of an IPv6 network: its range, the network when it isn't what
// was typed, how many /64s it holds when the headline doesn't say, its
// size in addresses and the address in full.
function sixParts(at, settings) {
    const rows = [range(at.first, at.last)];
    if (!at.aligned) {
        rows.push(part("network", at.network));
        if (at.bits >= SIX_BITS - ONE_NETWORK) {
            rows.push(part("subnets", networks(at.bits, settings)));
        }
    }
    if (!at.aligned || at.bits >= SIX_BITS - ONE_NETWORK) {
        rows.push(part("size", `${counted(at.bits, settings)} addresses`));
    }
    rows.push(part("full", at.full));
    return rows;
}

// An address or a network, told. A network says how many hosts it holds,
// or, typed by an address inside it, which network that is; a lone address
// says nothing of its own, leaving its kind and holder to the provider.
function about(text, settings) {
    const at = network(text);
    if (!at) {
        return null;
    }
    const most = at.version === IPV4 ? FOUR_BITS : SIX_BITS;
    const lone = at.prefix === null || at.prefix === most;
    if (lone) {
        const rows = at.version === IPV6 ? [part("full", at.full)] : [];
        return { at, network: false, headline: "", copy: at.text, rows, note: "" };
    }
    const holder = `in ${at.network}`;
    if (at.version === IPV4) {
        const hosts = `${grouped(at.usable, settings)} ${at.usable === 1 ? "host" : "hosts"}`;
        return { at, network: true, headline: at.aligned ? hosts : holder, copy: at.network,
            rows: fourParts(at, !at.aligned, settings), note: "" };
    }
    return { at, network: true, headline: at.aligned ? networks(at.bits, settings) : holder,
        copy: at.network, rows: sixParts(at, settings), note: "" };
}

// Which of two IPv6 addresses, as groups, comes first: below zero for the
// first, above for the second.
function order6(one, other) {
    const at = one.findIndex((group, i) => group !== other[i]);
    return at < 0 ? 0 : one[at] - other[at];
}

// The lowest and highest addresses an address or network spans, as
// numbers for IPv4 and groups for IPv6.
function span(at) {
    if (at.version === IPV4) {
        const low = Addresses.four(base(at));
        return { low, high: low + at.size - 1 };
    }
    const bits = at.prefix ?? SIX_BITS;
    const low = Addresses.six(base(at));
    return { low, high: Addresses.edge(low, bits, true) };
}

// Whether one address or network lies wholly in another.
function holds(outer, inner) {
    if (outer.version !== inner.version) {
        return false;
    }
    const out = span(outer);
    const within = span(inner);
    if (outer.version === IPV4) {
        return within.low >= out.low && within.high <= out.high;
    }
    return order6(within.low, out.low) >= 0 && order6(within.high, out.high) <= 0;
}

// "is 10.1.2.3 in 10.0.0.0/16": yes or no, with the network and its range.
function inside(text) {
    const asked = INSIDE.exec(text);
    const inner = asked ? network(asked[1]) : null;
    const outer = asked ? network(asked[2]) : null;
    if (!(inner && outer)) {
        return null;
    }
    const said = holds(outer, inner) ? "yes" : "no";
    const last = outer.version === IPV4 ? outer.broadcast : outer.last;
    return { headline: said, copy: said, note: "",
        rows: [part("network", outer.network), range(base(outer), last)] };
}

// "/22": its mask, with its wildcard and how many it holds; past an IPv4
// prefix's length, how many /64s or addresses an IPv6 one holds.
function prefix(text, settings) {
    const asked = PREFIX.exec(text);
    const bits = asked ? Number(asked[1]) : null;
    if (bits === null || bits > SIX_BITS) {
        return null;
    }
    if (bits > FOUR_BITS) {
        const held = networks(SIX_BITS - bits, settings);
        return { headline: held, copy: held, note: "",
            rows: [part("size", `${counted(SIX_BITS - bits, settings)} addresses`)] };
    }
    const at = Addresses.read(`0.0.0.0/${bits}`);
    return { headline: at.mask, copy: at.mask, note: "", rows: [
        part("wildcard", Addresses.fourText(at.size - 1)),
        part("hosts", grouped(at.usable, settings), String(at.usable)),
        part("size", `${grouped(at.size, settings)} addresses`, String(at.size))
    ] };
}

// The groups with `count` added at the bit `shift` up from the last.
function added(groups, count, shift) {
    const out = groups.slice();
    let at = SIX_GROUPS - 1 - Math.floor(shift / GROUP_BITS);
    let carry = count * 2 ** (shift % GROUP_BITS);
    while (carry > 0 && at >= 0) {
        const sum = out[at] + carry;
        out[at] = sum % GROUP_VALUES;
        carry = Math.floor(sum / GROUP_VALUES);
        at -= 1;
    }
    return out;
}

// The `index`th subnet of `bits` in a network: { name, side, copy }.
function subnet(at, bits, index) {
    if (at.version === IPV4) {
        const size = 2 ** (FOUR_BITS - bits);
        const low = Addresses.four(base(at)) + index * size;
        const first = Addresses.fourText(low);
        const last = Addresses.fourText(low + size - 1);
        return { name: `${first}/${bits}`, side: `${first} \u2013 ${last}`,
            copy: `${first}/${bits}` };
    }
    const low = added(Addresses.six(base(at)), index, SIX_BITS - bits);
    const first = Addresses.sixText(low);
    const last = Addresses.sixText(Addresses.edge(low, bits, true));
    return { name: `${first}/${bits}`, side: `${first} \u2013 ${last}`, copy: `${first}/${bits}` };
}

// "10.0.0.0/22 in /24s": how many there are, and the first of them as rows,
// Return copying those listed, a line each.
function split(text, settings) {
    const asked = SPLIT.exec(text);
    const at = asked ? network(asked[1]) : null;
    const bits = asked ? Number(asked[2]) : null;
    const most = at && at.version === IPV4 ? FOUR_BITS : SIX_BITS;
    if (!at || at.prefix === null || bits < at.prefix || bits > most) {
        return null;
    }
    const more = bits - at.prefix;
    const count = 2 ** more;
    const listed = Math.min(count, LISTED);
    const rows = Array.from({ length: listed }, (_, i) => subnet(at, bits, i));
    return {
        headline: count === 1 ? `one /${bits}` : `${counted(more, settings)} /${bits}s`,
        copy: rows.map(row => row.copy).join("\n"),
        rows,
        note: listed < count ? `the first ${grouped(listed, settings)}` : ""
    };
}

// What's typed, answered as subnet maths: { headline, copy, rows, note },
// with `at` and `network` when it's an address or a network, or null when
// it's none of these. `options` gives the marks numbers are grouped by.
function answer(typed, options) {
    const settings = options ?? {};
    const text = typed.trim().replace(COMMAND, "");
    return inside(text) ?? split(text, settings) ?? prefix(text, settings)
        ?? about(text, settings);
}
