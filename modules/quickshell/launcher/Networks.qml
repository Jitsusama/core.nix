// Addresses and networks, answered in the field: "ip" says this machine's
// addresses, by interface, and its public ones; an IPv4 or IPv6 address or
// network says its size, range, mask and the kind of range it's in; and a
// public one says who holds it, asked of the regional registries through
// rdap.org. A private, shared, local or documentation address is never
// asked about, and with `lookups` set false nothing leaves the machine.
// The local addresses are read from the kernel's files under /proc, so
// nothing is spawned.
import Quickshell
import Quickshell.Io
import QtQuick
import "Addresses.js" as Addresses
import "Numbers.js" as Numbers

Provider {
    id: provider

    readonly property bool lookups: settings.lookups !== false
    // How long what a registry or ipify said is kept, in milliseconds.
    readonly property int day: 24 * 60 * 60 * 1000
    readonly property int minutes: 5 * 60 * 1000

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    function grouped(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    // How many of something there are in 2^bits, written whole while it can
    // be, and as a power of two past that.
    function counted(bits) {
        return bits <= 52 ? grouped(Math.pow(2, bits)) : `2^${bits}`;
    }

    function answer(typed) {
        const text = typed.trim();
        if (/^(?:my\s+)?(?:ip|ips|ip\s+address(?:es)?|addresses)$/i.test(text))
            return mine();
        const read = Addresses.read(text);
        return read ? about(read) : null;
    }

    // An interface's glyph: Wi-Fi, a cable, or a tunnel.
    function wire(name) {
        return glyph(/^wl/.test(name) ? 0xf05a9 : /^(en|eth)/.test(name) ? 0xf0200 : 0xf0582);
    }

    // This machine's addresses, the one on the way out first, then its
    // public ones.
    function mine() {
        const routes = [];
        let out = "";
        for (const line of read(route).split("\n").slice(1)) {
            const f = line.trim().split(/\s+/);
            if (f.length < 8)
                continue;
            const flip = hex => parseInt(hex.match(/../g).reverse().join(""), 16);
            if (f[1] === "00000000")
                out = f[0];
            else
                routes.push({ name: f[0], at: flip(f[1]), mask: flip(f[7]) });
        }
        const rows = [];
        const lines = read(trie).split("\n");
        const seen = {};
        for (let i = 1; i < lines.length; i++) {
            if (!/\/32 host LOCAL/.test(lines[i]))
                continue;
            const address = lines[i - 1].replace(/^[\s|+-]*/, "").trim();
            const value = Addresses.read(address);
            if (!value || seen[address] || value.kind.kind === "loopback")
                continue;
            seen[address] = true;
            const number = address.split(".").reduce((all, part) => all * 256 + Number(part), 0);
            const on = routes.find(r => ((number & r.mask) >>> 0) === r.at);
            const bits = on ? on.mask.toString(2).replace(/0/g, "").length : 32;
            const name = on ? on.name : "";
            rows.push(copyRow({
                name: bits === 32 ? address : `${address}/${bits}`,
                copy: address,
                side: name,
                glyph: wire(name),
                first: name === out
            }));
        }
        for (const line of read(six).split("\n")) {
            const f = line.trim().split(/\s+/);
            // Only addresses that reach past the link: global and unique local.
            if (f.length < 6 || f[3] !== "00")
                continue;
            const value = Addresses.read(f[0].match(/..../g).join(":"));
            if (!value)
                continue;
            rows.push(copyRow({
                name: `${value.text}/${parseInt(f[2], 16)}`,
                copy: value.text,
                side: f[5],
                glyph: wire(f[5]),
                first: false
            }));
        }
        rows.sort((a, b) => (b.first ? 1 : 0) - (a.first ? 1 : 0));
        const headline = rows.length > 0 ? rows[0].copy : "no address";
        if (lookups) {
            for (const [url, label] of [["https://api.ipify.org?format=json", "public"],
                    ["https://api6.ipify.org?format=json", "public IPv6"]]) {
                const said = ask(url, minutes);
                if (said.value)
                    rows.push(copyRow({ name: said.value.ip, side: label, glyph: glyph(0xf01e7) }));
                else if (said.failed)
                    rows.push(copyRow({
                        name: label === "public" ? "no public route" : "no public IPv6 route",
                        side: "",
                        glyph: glyph(0xf01e7),
                        copy: ""
                    }));
                else
                    rows.push(copyRow({
                        name: "asking ipify",
                        side: label,
                        glyph: glyph(0xf01e7)
                    }));
            }
        }
        return {
            headline: headline,
            rows: rows,
            note: lookups ? "public addresses from ipify" : ""
        };
    }

    // An address or network: its size and range, and who holds it.
    function about(at) {
        const rows = [];
        const net = glyph(0xf0c90);
        let headline = "";
        let copy = at.text;
        const network = at.prefix !== null && at.prefix < (at.version === 4 ? 32 : 128);
        if (at.version === 4 && network) {
            headline = `${grouped(at.usable)} ${at.usable === 1 ? "host" : "hosts"}`;
            copy = at.network;
            const usable = `${at.first} \u2013 ${at.last}`;
            rows.push(copyRow({ name: usable, side: "usable", glyph: net }));
            rows.push(copyRow({ name: at.mask, side: "mask", glyph: net }));
            if (at.prefix <= 30)
                rows.push(copyRow({ name: at.broadcast, side: "broadcast", glyph: net }));
            if (!at.aligned)
                rows.push(copyRow({ name: at.network, side: "network", glyph: net }));
        } else if (at.version === 6 && network) {
            headline = at.bits === 64 ? "one /64"
                : at.bits > 64 ? `${counted(at.bits - 64)} /64s`
                : `${counted(at.bits)} addresses`;
            copy = at.network;
            const range = `${at.first} \u2013 ${at.last}`;
            rows.push(copyRow({ name: range, copy: at.network, glyph: net }));
            rows.push(copyRow({ name: at.full, side: "full", glyph: net }));
            if (!at.aligned)
                rows.push(copyRow({ name: at.network, side: "network", glyph: net }));
        } else if (at.version === 6) {
            rows.push(copyRow({ name: at.full, side: "full", glyph: net }));
        }
        const kind = at.kind;
        // A network part public and part not is said as what it starts in.
        if (!kind.public) {
            if (headline === "")
                headline = kind.kind;
            rows.push(copyRow({
                name: kind.kind,
                side: kind.rfc,
                copy: kind.rfc,
                glyph: glyph(0xf0ccc)
            }));
            return { headline: headline, copy: copy, rows: rows, note: "" };
        }
        if (!lookups)
            return { headline: headline || "public", copy: copy, rows: rows, note: "" };
        const held = holder(at, network ? at.network : at.text);
        // A lone address's headline is its holder, so its row isn't repeated.
        if (headline === "")
            return { headline: held.name, copy: copy, rows: held.rows.slice(1).concat(rows),
                note: held.note };
        return { headline: headline, copy: copy, rows: held.rows.concat(rows), note: held.note };
    }

    // Who holds an address or network, as the registry for it says.
    function holder(at, asked) {
        const said = ask(`https://rdap.org/ip/${asked}`, day);
        if (!said.value)
            return {
                name: said.failed ? "public" : "\u2026",
                rows: [],
                note: said.failed ? "rdap.org didn't answer" : "asking rdap.org"
            };
        const value = said.value;
        const fn = entity => {
            const card = entity.vcardArray ? entity.vcardArray[1] : [];
            const field = card.find(f => f[0] === "fn");
            return field ? field[3] : "";
        };
        const people = [];
        const walk = list => {
            for (const entity of list ?? []) {
                people.push(entity);
                walk(entity.entities);
            }
        };
        walk(value.entities);
        const registrant = people.find(e => (e.roles ?? []).includes("registrant") && fn(e) !== "");
        const name = registrant ? fn(registrant)
            : (people.map(fn).find(n => n !== "") || value.name);
        const rows = [copyRow({ name: name, side: "holder", glyph: glyph(0xf01d7) })];
        for (const block of value.cidr0_cidrs ?? [])
            rows.push(copyRow({
                name: `${block.v4prefix ?? block.v6prefix}/${block.length}`,
                side: value.name ?? "",
                glyph: glyph(0xf0c90)
            }));
        if (at.version === 4 && value.startAddress && value.endAddress) {
            const start = Addresses.read(value.startAddress);
            const end = Addresses.read(value.endAddress);
            const size = start && end ? Addresses.four(value.endAddress)
                - Addresses.four(value.startAddress) + 1 : 0;
            rows.push(copyRow({
                name: `${value.startAddress} \u2013 ${value.endAddress}`,
                side: size > 0 ? `${grouped(size)} addresses` : "",
                glyph: glyph(0xf0c90)
            }));
        }
        const server = (value.port43 ?? "").toLowerCase();
        const registries = { arin: "ARIN", ripe: "RIPE NCC", apnic: "APNIC", lacnic: "LACNIC",
            afrinic: "AFRINIC" };
        const registry = Object.keys(registries).find(key => server.includes(key));
        return {
            name: name,
            rows: rows,
            note: `${registry ? registries[registry] : "the registry"}, through rdap.org`
        };
    }

    FileView {
        id: route
        path: "/proc/net/route"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: trie
        path: "/proc/net/fib_trie"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: six
        path: "/proc/net/if_inet6"
        blockLoading: true
        printErrors: false
    }
}
