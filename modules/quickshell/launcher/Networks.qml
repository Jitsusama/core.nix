// Addresses and networks, answered in the field: "address" says this machine's
// connections, each named by what it is (Wi-Fi, Wired, a VPN, a Virtual
// bridge) and saying its address, the one the traffic leaves by first,
// then its public addresses. A step into a connection holds its parts:
// its Address, Gateway, DNS (on the way out, as the machine asks names
// there), Hardware address, MTU, State and the Device name the kernel
// knows it by, which finds it too. An Ethernet card's holds its link as
// well, after the names: the Speed and Duplex it agreed with the other
// end, how often its Link was Lost, and its Errors, each kind a step in,
// read again each second while they're shown. A connection networkd leased
// its address for holds its Lease (the server, how long it's given for,
// when it was got, renews, rebinds and ends, and what else the server
// offered), and a cable that hears its other end over LLDP holds its
// Neighbour (its system, port, chassis, what it acts as and its VLAN).
// "ethernet" asks the cards on a cable alone, Network's Ethernet row,
// whatever their address. "gateway" and "dns" ask those parts alone. An
// IPv4 or IPv6 address or network, written with its prefix or
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
// What each connection is comes from the machine daemon, which dumps the
// links and their addresses from the kernel in ip's own names
// (modules/machine/src/links.rs) and says again only what the kernel says
// changed, watched while Network is wanted, so nothing is spawned and
// nothing polls; the routes and name servers are read from the kernel's
// files and resolv.conf when asked, so nothing blocks. A cable's link is
// read from its folder in /sys/class/net, and read again each second
// while it's shown. The leases and neighbours are asked of networkd over
// its varlink socket as the launcher opens and when what it said is five
// seconds old, so nothing is spawned for them either.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Addresses.js" as Addresses
import "Interfaces.js" as Interfaces
import "Networkd.js" as Networkd
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Subnets.js" as Subnets

Provider {
    id: provider

    examples: ["address", "10.0.0.0/22", "8.8.8.8", "2001:db8::/48", "10.0.0.0/22 in /24s"]

    readonly property bool lookups: settings.lookups !== false
    // How long what a registry or ipify said is kept, in milliseconds.
    readonly property int day: 24 * 60 * 60 * 1000
    readonly property int minutes: 5 * 60 * 1000
    // How long what networkd said stands before it's asked again.
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
    // The words that ask the cards on a cable alone.
    readonly property var cableAsked: Words.answers("ethernet")
    questions: own.concat(Object.keys(partsAsked), cableAsked)

    function answer(typed) {
        const text = typed.trim();
        const question = asks(text);
        if (partsAsked[question] !== undefined)
            return alone(partsAsked[question]);
        if (cableAsked.includes(question))
            return cabled();
        if (question !== "")
            return mine();
        const said = Subnets.answer(text, settings);
        return said ? about(said) : null;
    }

    // What the daemon last said of every connection, made sense of
    // (Interfaces.js), each with where its hardware address came from;
    // null until it first answers. `waiting` is whether an answer was given
    // without it, and so is asked again once it comes.
    property var links: null
    property bool waiting: false
    property real lookedAt: 0

    function look() {
        lookedAt = Date.now();
        if (!networkd.connected)
            networkd.connected = true;
    }

    // The links and their addresses, watched while Network is wanted, and
    // kept through a gap, so an opening answers from what was last said
    // while its watch asks again.
    Daemon {
        id: linked

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["link"]
        live: provider.wanted
        answersWhole: true
    }

    Connections {
        target: linked

        function onRevisionChanged() {
            provider.heard();
        }
    }

    // What networkd last said over varlink of each connection's DHCP lease
    // and each cable's neighbours, by the connection's name (Networkd.js);
    // empty until it answers, and where it isn't running.
    property var leases: ({})
    property var neighbours: ({})
    // The calls to networkd not yet answered, in order: one is sent at a
    // time, the next once the last is answered.
    property var calling: []

    function call(method) {
        return JSON.stringify({ method: `io.systemd.Network.${method}`, parameters: {} })
            + "\u0000";
    }

    function heardNetworkd(data) {
        const method = calling.shift();
        const reply = parsed(data);
        const said = reply.error ? null : reply.parameters ?? null;
        const before = JSON.stringify([leases, neighbours]);
        if (method === "Describe")
            leases = Networkd.leases(said);
        else if (method === "GetLLDPNeighbors")
            neighbours = Networkd.neighbours(said);
        if (calling.length > 0) {
            networkd.write(call(calling[0]));
            networkd.flush();
        } else {
            networkd.connected = false;
        }
        if (JSON.stringify([leases, neighbours]) !== before)
            answered();
    }

    Socket {
        id: networkd
        path: "/run/systemd/netif/io.systemd.Network"
        onConnectionStateChanged: {
            if (connected) {
                provider.calling = ["Describe", "GetLLDPNeighbors"];
                write(provider.call(provider.calling[0]));
                flush();
            }
        }
        parser: SplitParser {
            splitMarker: "\u0000"
            onRead: data => provider.heardNetworkd(data)
        }
    }

    function heard() {
        const listed = Object.values(linked.held).filter(record => record.kind === "link")
            .sort((a, b) => a.ifindex - b.ifindex);
        const wireless = listed.filter(link => link.wireless).map(link => String(link.ifname));
        const sources = {};
        for (const link of listed)
            sources[link.ifname] = String(link.addr_assign_type ?? "");
        const now = Interfaces.interfaces(listed, wireless).map(link => Object.assign(link, {
            assigned: link.mac === "" ? "" : Interfaces.assigned(sources[link.name] ?? "")
        }));
        if (JSON.stringify(now) === JSON.stringify(links))
            return;
        links = now;
        // A link that changed while it's shown is said at once, as the
        // kernel says it.
        if (waiting || watching) {
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

    // The connections as they stand, asking networkd again when what it
    // said is old, but for a cable's link read again while it's shown:
    // { links, routes, routes6, out }, or null before the daemon first
    // answers.
    function standing() {
        if (!ticking && Date.now() - lookedAt > stands)
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

    // What isn't known yet, while the daemon answers.
    readonly property var unknown: ({ headline: "\u2026", rows: [], asking: true, note: "" })

    // The order kinds of connection are listed in, after the way out.
    readonly property var kindOrder: ["wifi", "wired", "tunnel", "bridge", "virtual"]

    // The connections worth listing: each with an address, and a card
    // that's up without one, the way out first, then by kind and name.
    function listed(now) {
        const rank = link => link.name === now.out ? -1 : kindOrder.indexOf(link.kind);
        return Order.sorted(now.links.filter(link => link.four.length + link.six.length > 0
            || ((link.kind === "wifi" || link.kind === "wired") && link.state === "up")),
            Order.live({ stands: rank, name: link => link.name }));
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

    // The rows each connection is kept under, so a cable's link and a
    // lease's times are read again each second while one is shown, and
    // whether one is: stepped into, one of its parts, or the cards on a
    // cable asked alone.
    readonly property var connections: (links ?? []).map(link => `networks:${link.name}`)
    readonly property bool watching: launcher !== null && launcher.open
        && ((launcher.held ?? []).some(step => cableHeld(step.row))
            || ((launcher.held ?? []).length === 0 && cableHeld(launcher.answer)))

    // Whether a row is a connection, one of its parts, or the cards on a
    // cable asked alone.
    function cableHeld(row) {
        const key = row?.key ?? "";
        return (row?.asker === provider && cableAsked.includes(row?.chip ?? ""))
            || connections.some(each => key === each || key.startsWith(`${each}:`));
    }
    // Whether the answer is asked again for the link alone, which asks
    // networkd nothing.
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
        "no link": "nothing at the other end"
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
        const lease = leases[name] ?? null;
        if (lease !== null)
            rows.push(leaseRow(name, lease));
        (neighbours[name] ?? []).forEach((one, at) => rows.push(neighbourRow(name, one, at)));
        if (link.mac !== "")
            rows.push(part(key("hardware"), "hardware", link.mac,
                link.own !== "" ? `its own is ${link.own}` : assignedSaid[link.assigned] ?? ""));
        if (link.mtu > 0)
            rows.push(part(key("mtu"), "mtu", `${grouped(link.mtu)} bytes`,
                "the largest packet it sends whole", String(link.mtu)));
        // A cable's carrier is read as its link is, so it's said as it is
        // now rather than as the daemon last said.
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

    // How long the machine's been up, in seconds, as networkd counts a
    // lease's times.
    function uptime() {
        return Number(read(up).trim().split(/\s+/)[0]) || 0;
    }

    // A moment, in seconds since boot, as how long from now: "in 3 h",
    // "20 min ago".
    function fromNow(at, now) {
        return at >= now ? `in ${Interfaces.lasted(at - now)}`
            : `${Interfaces.lasted(now - at)} ago`;
    }

    // A connection's lease, its time left at its right and its server
    // under its name, the rest a step in.
    function leaseRow(name, lease) {
        const now = uptime();
        let side = lease.server ?? "held";
        if (lease.ends !== null)
            side = lease.ends > now ? `ends in ${Interfaces.lasted(lease.ends - now)}` : "ended";
        return holderRow({
            key: `networks:${name}:lease`,
            name: named("lease"),
            side: side,
            note: lease.server !== null ? `from ${lease.server}` : "from DHCP",
            glyph: Theme.glyph.wired
        }, () => provider.leaseParts(name));
    }

    // A lease a step in, as networkd last said it, its times from now: the
    // server, how long it's given for, when it was got, renews, rebinds
    // and ends, then what else the server offered. Its router and subnet
    // mask are the connection's Gateway and its address's prefix, so
    // they aren't said twice.
    function leaseParts(name) {
        const lease = leases[name] ?? null;
        if (lease === null)
            return [];
        const now = uptime();
        const key = id => `networks:${name}:lease:${id}`;
        const said = (id, side, note, copy) => part(key(id), `lease-${id}`, side, note, copy);
        const rows = [];
        if (lease.server !== null)
            rows.push(said("server", lease.server, "the DHCP server that gave the address"));
        if (lease.lasts !== null)
            rows.push(said("lasts", Interfaces.lasted(lease.lasts), "how long each lease lasts",
                String(lease.lasts)));
        if (lease.got !== null)
            rows.push(said("got", fromNow(lease.got, now), "given or last renewed", ""));
        if (lease.renews !== null)
            rows.push(said("renews", fromNow(lease.renews, now),
                "when it asks its server to keep it", ""));
        if (lease.rebinds !== null)
            rows.push(said("rebinds", fromNow(lease.rebinds, now),
                "when it asks any server, if its own hasn't answered", ""));
        if (lease.ends !== null)
            rows.push(said("ends", fromNow(lease.ends, now), "when the address goes, unless kept",
                ""));
        if (lease.nameServers.length > 0)
            rows.push(said("name-servers", lease.nameServers.join(", "),
                "what it offered to answer names"));
        if (lease.timeServers.length > 0)
            rows.push(said("time-servers", lease.timeServers.join(", "),
                "what it offered to keep time by"));
        if (lease.domain !== null)
            rows.push(said("domain", lease.domain, "the domain it says this network is"));
        if (lease.hostname !== null)
            rows.push(said("hostname", lease.hostname, "the name it gave this machine"));
        return rows;
    }

    // What's at a cable's other end, by what it calls itself and the port
    // it hears this card on, the rest a step in.
    function neighbourRow(name, one, at) {
        const port = one.portName ?? one.port;
        return holderRow({
            key: `networks:${name}:neighbour:${at}`,
            name: named("neighbour"),
            side: [one.system ?? one.chassis, port].filter(said => said).join(" \u00b7 "),
            note: "what the cable is plugged into",
            glyph: Theme.glyph.wired
        }, () => provider.neighbourParts(name, at));
    }

    // A neighbour a step in, each thing it says over LLDP (Networkd.js).
    function neighbourParts(name, at) {
        const one = (neighbours[name] ?? [])[at] ?? null;
        return one === null ? [] : Networkd.neighbourParts(one).map(said => part(
            `networks:${name}:neighbour:${at}:${said.id}`, `neighbour-${said.id}`, said.side,
            said.note));
    }

    // The cards on a cable, asked alone: one's parts, or several each
    // under its name, whatever their address, since a cable with no link
    // is what's worth seeing. Nothing on a machine with no Ethernet card,
    // so Network has no Ethernet row there.
    function cabled() {
        const now = standing();
        if (now === null)
            return unknown;
        // In ip's order, as the addresses number them, so a card has one
        // name in both.
        const cards = now.links.filter(link => link.kind === "wired");
        const said = link => {
            const cable = Interfaces.cable(file => sys(link.name, file));
            return cable.speed !== null ? Interfaces.linkRate(cable.speed,
                (value, places) => Numbers.format(value, places, settings.group, settings.point))
                : stateSaid[cable.state];
        };
        if (cards.length === 0)
            return { headline: "no Ethernet card", copy: "", rows: [], note: "" };
        if (cards.length === 1)
            return {
                headline: said(cards[0]),
                copy: "",
                glyph: Theme.glyph.wired,
                rows: partsOf(cards[0].name),
                note: ""
            };
        return {
            headline: `${grouped(cards.length)} cards`,
            copy: "",
            glyph: Theme.glyph.wired,
            rows: cards.map(link => thingRow({
                thing: true,
                key: `networks:${link.name}`,
                name: linkName(link, cards),
                side: said(link),
                glyph: Theme.glyph.wired,
                keywords: link.name,
                note: addressOf(link)
            }, () => provider.partsOf(link.name))),
            note: ""
        };
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
        id: up
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}
