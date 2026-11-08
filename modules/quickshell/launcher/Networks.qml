// Addresses and networks, answered in the field: "address" says this machine's
// connections, each named by what it is (Wi-Fi, Wired, a VPN, a Virtual
// bridge) and saying its address, the one the traffic leaves by first,
// then its public addresses. A step into a connection holds its parts:
// its Address, Gateway, DNS (on the way out, as the machine asks names
// there), Hardware address, MTU, State and the Device name the kernel
// knows it by, which finds it too. An Ethernet card's holds its link as
// well, after the names: the Speed and Duplex it agreed with the other
// end, how often its Link was Lost, and its Errors, each kind a step in,
// read again each second while they're shown. "gateway" and "dns" ask those parts
// alone. An IPv4 or IPv6 address or network, written with its prefix or
// its mask, says its size, range, mask and the rest of a subnet's parts
// (Subnets.js works them out), or which network holds the address typed,
// and the kind of range it's in; "is 10.1.2.3 in 10.0.0.0/16" says yes or
// no, "/22" a prefix's mask and size, and "10.0.0.0/22 in /24s" the
// subnets it splits into. A public one says who holds it, asked
// of the regional registries through rdap.org. A private, shared, local or
// documentation address is never asked about, and with `lookups` set false
// nothing leaves the machine. Every row is a part named first, its value
// after, and the words are the vocabulary's (words.json, networks:*).
//
// What each connection is comes from one `ip -json -details address`,
// spawned as the launcher opens and when it's asked after five seconds,
// never for a key; the routes and name servers are read from the kernel's
// files and resolv.conf when asked, so nothing blocks. A cable's link is
// read from its folder in /sys/class/net, and read again each second
// spawns nothing: ip is asked only when the answer is.
import qs
import Quickshell.Io
import QtQuick
import "Addresses.js" as Addresses
import "Interfaces.js" as Interfaces
import "Numbers.js" as Numbers
import "Subnets.js" as Subnets

Provider {
    id: provider

    examples: ["address", "10.0.0.0/22", "8.8.8.8", "2001:db8::/48", "10.0.0.0/22 in /24s"]

    readonly property bool lookups: settings.lookups !== false
    // How long what a registry or ipify said is kept, in milliseconds.
    readonly property int day: 24 * 60 * 60 * 1000
    readonly property int minutes: 5 * 60 * 1000
    // How long what ip said stands before it's asked again.
    readonly property int stands: 5000

    function grouped(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    // A part's name, as the vocabulary says it: "Gateway" for
    // networks:gateway.
    function named(id) {
        return Words.name(`networks:${id}`);
    }

    // The word that asks this machine's own addresses, Network's one word
    // for them (Words.qml), its start asking too.
    readonly property var own: Words.answers("networks")
    // The parts asked for alone, by their words, as the part's id.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["gateway", "dns"])
            for (const question of Words.answers(`networks:${id}`))
                out[question] = id;
        return out;
    }
    questions: own.concat(Object.keys(partsAsked))

    function answer(typed) {
        const text = typed.trim();
        const question = asks(text);
        if (partsAsked[question] !== undefined)
            return alone(partsAsked[question]);
        if (question !== "")
            return mine();
        const said = Subnets.answer(text, settings);
        return said ? about(said) : null;
    }

    // What ip last said of every connection, made sense of
    // (Interfaces.js), each with where its hardware address came from;
    // null until it first answers. `waiting` is whether an answer was given
    // without it, and so is asked again once it comes.
    property var links: null
    property real linksAt: 0
    property bool waiting: false

    function look() {
        if (!lister.running)
            lister.running = true;
    }

    function heard(text) {
        const said = parsed(text);
        const listed = Array.isArray(said) ? said : [];
        const wireless = listed.map(link => String(link.ifname ?? ""))
            .filter(name => name !== "" && sys(name, "uevent").includes("DEVTYPE=wlan"));
        links = Interfaces.interfaces(listed, wireless).map(link => Object.assign(link, {
            assigned: link.mac === "" ? "" : Interfaces.assigned(sys(link.name, "addr_assign_type"))
        }));
        linksAt = Date.now();
        if (waiting) {
            waiting = false;
            answered();
        }
    }

    // What a file in an interface's sysfs folder says, or "".
    function sys(name, file) {
        probe.path = `/sys/class/net/${name}/${file}`;
        return read(probe).trim();
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.look();
        }
    }

    // The connections as they stand, asking ip again when what it said is
    // old, but for a cable's link read again while it's shown: { links,
    // routes, routes6, out }, or null before ip first answers.
    function standing() {
        if (links === null || (!ticking && Date.now() - linksAt > stands))
            look();
        if (links === null) {
            waiting = true;
            return null;
        }
        const routes = Interfaces.gateways(read(route));
        const routes6 = Interfaces.gateways6(read(sixRoute));
        return {
            links: links,
            routes: routes,
            routes6: routes6,
            out: Interfaces.wayOut(routes) || Interfaces.wayOut(routes6)
        };
    }

    // What isn't known yet, while ip answers.
    readonly property var unknown: ({ headline: "\u2026", rows: [], asking: true, note: "" })

    // The order kinds of connection are listed in, after the way out.
    readonly property var kindOrder: ["wifi", "wired", "tunnel", "bridge", "virtual"]

    // The connections worth listing: each with an address, and a card
    // that's up without one, the way out first, then by kind and name.
    function listed(now) {
        const rank = link => link.name === now.out ? -1 : kindOrder.indexOf(link.kind);
        return now.links.filter(link => link.four.length + link.six.length > 0
            || ((link.kind === "wifi" || link.kind === "wired") && link.state === "up"))
            .sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
    }

    // A connection's name, by what it is, numbered when there are several
    // of a kind: "VPN 2".
    function linkName(link, all) {
        const alike = all.filter(each => each.kind === link.kind);
        const base = named(link.kind);
        return alike.length > 1 ? `${base} ${alike.indexOf(link) + 1}` : base;
    }

    function linkGlyph(kind) {
        return kind === "wifi" ? Theme.glyph.wireless
            : kind === "wired" ? Theme.glyph.wired : Theme.glyph.tunnel;
    }

    function addressOf(link) {
        const first = link.four[0] ?? null;
        if (first)
            return `${first.address}/${first.prefix}`;
        const six = link.six[0] ?? null;
        return six ? `${shortened(six.address)}/${six.prefix}` : "";
    }

    function shortened(address) {
        const value = Addresses.read(address);
        return value ? value.text : address;
    }

    // A part saying a value, Return copying it, wearing no glyph as a
    // figure doesn't.
    function part(key, id, side, note, copy) {
        return copyRow({
            key: key,
            name: named(id),
            side: side,
            copy: copy ?? side,
            hue: "machine",
            glyph: "",
            note: note ?? ""
        });
    }

    // This machine's connections, the one on the way out first, then its
    // public addresses.
    function mine() {
        const now = standing();
        if (now === null)
            return unknown;
        const shown = listed(now);
        const rows = shown.map(link => thingRow({
            thing: true,
            key: `networks:${link.name}`,
            name: linkName(link, shown),
            side: addressOf(link) || stateSaid[link.state],
            glyph: linkGlyph(link.kind),
            keywords: link.name,
            // What its kind was called, so "wired" still finds Ethernet.
            once: Words.of(`networks:${link.kind}`).once ?? [],
            note: link.name === now.out ? "where the machine's traffic leaves" : ""
        }, () => provider.partsOf(link.name)));
        const out = shown.find(link => link.name === now.out) ?? shown[0] ?? null;
        const first = out ? out.four[0] ?? out.six[0] ?? null : null;
        const headline = first ? shortened(first.address) : "no address";
        if (lookups)
            rows.push(publicRow("https://api.ipify.org?format=json", "public"),
                publicRow("https://api6.ipify.org?format=json", "public6"));
        return {
            headline: headline,
            copy: first ? shortened(first.address) : "",
            rows: rows,
            note: lookups ? "public addresses from ipify" : ""
        };
    }

    // The rows the cards on a cable are kept under, so their link is read
    // again each second while one is shown, and whether one is: stepped
    // into, or into one of its parts.
    readonly property var cables: (links ?? []).filter(link => link.kind === "wired")
        .map(link => `networks:${link.name}`)
    readonly property bool watching: launcher !== null && launcher.open
        && (launcher.held ?? []).some(step => cables.includes(step.row?.key ?? ""))
    // Whether the answer is asked again for the link alone, which asks ip
    // nothing.
    property bool ticking: false

    Timer {
        interval: 1000
        repeat: true
        running: provider.watching
        onTriggered: {
            provider.ticking = true;
            try {
                provider.answered();
            } finally {
                provider.ticking = false;
            }
        }
    }

    function publicRow(url, id) {
        const said = ask(url, minutes);
        const key = `networks:${id}`;
        if (said.value)
            return part(key, id, said.value.ip);
        return part(key, id, said.failed ? "no route" : "asking\u2026", "", "");
    }

    // How a connection stands, in words.
    readonly property var stateSaid: ({
        up: "up",
        off: "turned off",
        "no link": "up, nothing at the other end"
    })

    // Where a hardware address came from, in words.
    readonly property var assignedSaid: ({
        own: "its own, the one it was made with",
        random: "made up at random, so networks can't follow it",
        taken: "taken from another device",
        set: "set by software"
    })

    // A connection's parts, as it stands now: its addresses, the gateway,
    // the name servers where the traffic leaves, its hardware address, MTU
    // and state. The name the kernel knows it by is never shown, only kept
    // among the words that find the connection.
    function partsOf(name) {
        const now = standing();
        const link = now ? now.links.find(each => each.name === name) ?? null : null;
        if (link === null)
            return [];
        const key = id => `networks:${name}:${id}`;
        const rows = [];
        for (const each of link.four)
            rows.push(part(key(`address:${each.address}`), "address",
                `${each.address}/${each.prefix}`, "", each.address));
        for (const each of link.six)
            rows.push(part(key(`address:${each.address}`), "ipv6",
                `${shortened(each.address)}/${each.prefix}`, "", shortened(each.address)));
        const gateway = now.routes[name] ?? null;
        const gateway6 = now.routes6[name] ?? null;
        if (gateway || gateway6) {
            const six = gateway6 ? shortened(gateway6.address) : "";
            rows.push(part(key("gateway"), "gateway", gateway ? gateway.address : six,
                gateway && six !== "" ? `IPv6 by ${six}` : "where traffic for elsewhere goes"));
        }
        if (name === now.out) {
            const servers = nameServers();
            if (servers.length > 0)
                rows.push(part(key("dns"), "dns", servers.join(", "),
                    "what this machine asks names of"));
        }
        const cable = link.kind === "wired" ? Interfaces.cable(file => sys(name, file)) : null;
        if (cable !== null)
            rows.push(...cableParts(name, cable));
        if (link.mac !== "")
            rows.push(part(key("hardware"), "hardware", link.mac,
                link.own !== "" ? `its own is ${link.own}` : assignedSaid[link.assigned] ?? ""));
        if (link.mtu > 0)
            rows.push(part(key("mtu"), "mtu", `${grouped(link.mtu)} bytes`,
                "the largest packet it sends whole", String(link.mtu)));
        // A cable's carrier is read as its link is, so it's said as it is
        // now rather than as ip last said.
        rows.push(part(key("state"), "state", stateSaid[cable?.state ?? link.state], "", ""));
        return rows;
    }

    // How many, in words: "none", or the count grouped.
    function howMany(value) {
        return value === 0 ? "none" : grouped(value);
    }

    // What a cable's duplex means.
    readonly property var duplexSaid: ({
        full: "sends and hears at once",
        half: "sends and hears in turn, as when the two ends didn't agree on full"
    })

    // A cable's link, as its card reads it now: the Speed and Duplex it
    // agreed with the other end, each left out where the driver can't say,
    // as while nothing's at the other end; how often its Link was Lost;
    // and its Errors, each way, every kind a step in.
    function cableParts(name, cable) {
        const key = id => `networks:${name}:${id}`;
        const rows = [];
        if (cable.speed !== null)
            rows.push(part(key("speed"), "speed", Interfaces.linkRate(cable.speed,
                (value, places) => Numbers.format(value, places, settings.group, settings.point)),
                "what the two ends agreed on"));
        if (cable.duplex !== null)
            rows.push(part(key("duplex"), "duplex", cable.duplex, duplexSaid[cable.duplex]));
        if (cable.lost !== null)
            rows.push(part(key("lost"), "lost", cable.lost === 0 ? "never"
                : cable.lost === 1 ? "once" : `${grouped(cable.lost)} times`,
                "since the card started", String(cable.lost)));
        const errors = cable.errors;
        if (errors.in !== null && errors.out !== null)
            rows.push(holderRow({
                key: key("errors"),
                name: named("errors"),
                side: errors.in + errors.out === 0 ? "none"
                    : `${grouped(errors.in)} in \u00b7 ${grouped(errors.out)} out`,
                note: "since the card started",
                glyph: Theme.glyph.wired
            }, () => provider.errorParts(name)));
        return rows;
    }

    // What each kind of error a cable counts means.
    readonly property var errorSaid: ({
        "errors-in": "frames that came in broken, of every kind",
        damaged: "failed their checksum, most often a poor cable or port",
        misaligned: "didn't end on a whole byte",
        "wrong-length": "shorter or longer than they said",
        overrun: "came faster than the card could take them",
        missed: "the card had no room to keep",
        "queue-in": "overflowed the card's own queue",
        "errors-out": "frames that couldn't be sent, of every kind",
        aborted: "given up on, most often after too many collisions",
        "no-link": "sent as the link dropped",
        "queue-out": "the card's own queue ran dry mid-frame",
        heartbeat: "the other end's heartbeat went missing",
        late: "collided late, a sign the two ends disagree on duplex",
        collisions: "met another sender on the wire, as only half duplex can",
        "dropped-in": "set aside, not broken: for a protocol nothing here speaks, or no room",
        "dropped-out": "set aside before sending, not broken"
    })

    // A cable's errors a step in, as its card reads them now: what came in
    // broken, then each kind it had, what couldn't go out, then each kind
    // of that, and what was set aside each way, which isn't an error.
    function errorParts(name) {
        const errors = Interfaces.cable(file => sys(name, file)).errors;
        const key = id => `networks:${name}:errors:${id}`;
        const said = (id, value) => part(key(id), id, howMany(value), errorSaid[id] ?? "",
            String(value));
        const kinds = way => errors.kinds.filter(kind => kind.way === way)
            .map(kind => said(kind.id, kind.count));
        const rows = [];
        if (errors.in !== null)
            rows.push(said("errors-in", errors.in), ...kinds("in"));
        if (errors.out !== null)
            rows.push(said("errors-out", errors.out), ...kinds("out"));
        if (errors.droppedIn !== null)
            rows.push(said("dropped-in", errors.droppedIn));
        if (errors.droppedOut !== null)
            rows.push(said("dropped-out", errors.droppedOut));
        return rows;
    }

    // The name servers this machine asks, from resolv.conf, or from
    // systemd-resolved's own file where resolv.conf names only its stub.
    function nameServers() {
        const said = Interfaces.servers(read(resolv));
        return Interfaces.stubbed(said) ? Interfaces.servers(read(resolved)) : said;
    }

    // One part of the connection on the way out, asked for alone: the
    // gateway or the name servers.
    function alone(id) {
        const now = standing();
        if (now === null)
            return unknown;
        const shown = listed(now);
        const out = shown.find(link => link.name === now.out) ?? null;
        const found = out ? partsOf(out.name).find(row => row.name === named(id)) ?? null : null;
        if (!found)
            return { headline: id === "dns" ? "no name servers" : "no gateway", copy: "",
                rows: [], note: "" };
        return {
            headline: found.side,
            copy: found.side,
            rows: [],
            note: [linkName(out, shown), found.note].filter(said => said !== "")
                .join(" \u00b7 ")
        };
    }

    // An address or network as Subnets.js tells it, with the kind of range
    // it's in and, for a public one, who holds it; or what Subnets.js
    // worked out of no address in particular, as it said it.
    function about(said) {
        const rows = said.rows.map((row, i) => row.id
            ? part(`networks:about:${row.id}`, row.id, row.side, "", row.copy)
            : copyRow({ key: `networks:about:${i}:${row.name}`, name: row.name,
                side: row.side, copy: row.copy, hue: "machine", glyph: "" }));
        const at = said.at ?? null;
        if (at === null)
            return { headline: said.headline, copy: said.copy, rows: rows, note: said.note };
        let headline = said.headline;
        const copy = said.copy;
        const kind = at.kind;
        // A network part public and part not is said as what it starts in.
        if (!kind.public) {
            if (headline === "")
                headline = kind.kind;
            rows.push(part("networks:about:kind", "kind", kind.kind, kind.rfc, kind.kind));
            return { headline: headline, copy: copy, rows: rows, note: "" };
        }
        if (!lookups)
            return { headline: headline || "public", copy: copy, rows: rows, note: "" };
        const held = holderOf(at, said.network ? at.network : at.text);
        // A lone address's headline is its holder, so its row isn't repeated.
        if (headline === "")
            return { headline: held.name, copy: copy, rows: held.rows.slice(1).concat(rows),
                note: held.note };
        return { headline: headline, copy: copy, rows: held.rows.concat(rows), note: held.note };
    }

    // Who holds an address or network, as the registry for it says.
    function holderOf(at, asked) {
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
        const key = id => `networks:about:${id}`;
        const rows = [part(key("holder"), "holder", name)];
        for (const block of value.cidr0_cidrs ?? []) {
            const prefix = `${block.v4prefix ?? block.v6prefix}/${block.length}`;
            rows.push(part(key(`block:${prefix}`), "block", prefix, value.name ?? ""));
        }
        if (at.version === 4 && value.startAddress && value.endAddress) {
            const start = Addresses.read(value.startAddress);
            const end = Addresses.read(value.endAddress);
            const size = start && end ? Addresses.four(value.endAddress)
                - Addresses.four(value.startAddress) + 1 : 0;
            rows.push(part(key("registered"), "registered",
                `${value.startAddress} \u2013 ${value.endAddress}`,
                size > 0 ? `${grouped(size)} addresses` : ""));
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

    Process {
        id: lister
        command: [provider.settings.ip ?? "ip", "-json", "-details", "address", "show"]
        stdout: StdioCollector {
            id: listing
            onStreamFinished: provider.heard(listing.text)
        }
    }

    FileView {
        id: route
        path: "/proc/net/route"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: sixRoute
        path: "/proc/net/ipv6_route"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: resolv
        path: "/etc/resolv.conf"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: resolved
        path: "/run/systemd/resolve/resolv.conf"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}
