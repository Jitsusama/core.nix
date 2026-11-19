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
// nothing polls. The routes are loaded from the kernel's files in the
// background as the launcher opens, as the links change and when they're
// old, and an answer says them as last loaded, so a step into Network
// waits on no file for them. The name servers, from resolv.conf, and the
// time since boot load with them, and a cable's link loads from its
// folder in /sys/class/net a file at a time, again each second while it's
// shown, each said as last loaded too. The leases and neighbours are asked of networkd over
// its varlink socket as the launcher opens and when what it said is five
// seconds old, so nothing is spawned for them either.
//
// Long because each connection's parts, its drawer and its answer read the
// same links, routes and leases as they stand, and split apart they would
// each hold their own copy of what the daemon last said.
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
    // Where the kernel's routing tables are, its files route and
    // ipv6_route, so a case can stand in its own.
    property string kernel: "/proc/net"
    // Where each card's folder is, so a case can stand in its own.
    property string sysfs: "/sys/class/net"
    // The name servers' files and the time since boot's, for a case too.
    property string resolvFile: "/etc/resolv.conf"
    property string resolvedFile: "/run/systemd/resolve/resolv.conf"
    property string uptimeFile: "/proc/uptime"
    // networkd's varlink socket, so a case can give its own leases.
    property string networkdSocket: "/run/systemd/netif/io.systemd.Network"

    function grouped(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    // A part's name, as the vocabulary says it: "Gateway" for
    // networks:gateway.
    function named(id) {
        return Words.name(`networks:${id}`);
    }

    // A drawer's eyebrow, where what it says is and what it is, as every
    // drawer's says it (VKCF3Z, 2026-11-12): "on this machine \u00b7
    // connection".
    function eyebrow(key) {
        return String((Words.of("networks").said ?? {})[key] ?? "");
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
        reroute();
        reloadFacts();
        recable();
        if (!networkd.connected)
            networkd.connected = true;
    }

    // The routes loaded again in the background, said as they were until
    // the load lands (routed); one asked while another is under way is
    // that one. Each read waited on took a median 6 to 14 ms and up to 133
    // on Joel's laptop under load (2026-11-17), and Address and Ethernet
    // each read both files as a step into Network made its rows: four of a
    // step that has 50 ms for all of it.
    function reroute() {
        route.reload();
        sixRoute.reload();
    }

    // The name servers and the time since boot loaded again in the
    // background, as the routes are; with Ethernet's answer reading a
    // cable's twenty files twice, a docked step into Network made over
    // forty reads where it waited on each. Until they and every cable
    // have landed, a terminal's show asks again, so it prints what they
    // say now rather than what was loaded as the launcher last opened.
    // A fact already loading isn't asked again, so its load keeps the
    // tick it was asked on (factsSince, by the view itself, so a path
    // changed as it loads can't lose its tick), and one given up is passed
    // over while it's held off.
    property var factsAsked: []
    property var factsSince: new Map()
    property int factsTicks: 0
    function reloadFacts() {
        const fresh = [resolv, resolved, up].filter(view => String(view.path) !== ""
            && !factsAsked.includes(view) && !holding(factsStuck[String(view.path)]));
        if (fresh.length === 0)
            return;
        const since = new Map(factsSince);
        for (const view of fresh)
            since.set(view, factsTicks);
        factsSince = since;
        factsAsked = factsAsked.concat(fresh);
        stillReading = true;
        for (const view of fresh)
            view.reload();
    }
    // A fact still loading eight of the watch's ticks, about two seconds,
    // after it was asked is said as last loaded, warned of, and held off
    // as a card is (holdAfter), so a terminal isn't kept asking on a file
    // that never lands, nor a thread taken on it each tick. Each is given
    // up on its own two seconds, so a fact asked just before another is
    // given up isn't given up with it; they're counted in ticks rather
    // than on the wall clock, which a resume or a clock step jumps.
    property var factsStuck: Object.create(null)
    Timer {
        id: factsWatch

        interval: 250
        repeat: true
        running: provider.factsAsked.length > 0
        onTriggered: provider.factsTicked()
    }
    function factsTicked() {
        factsTicks += 1;
        factsGivenUp();
    }
    function factsGivenUp() {
        const late = factsAsked.filter(view =>
            factsTicks - factsSince.get(view) >= 8);
        if (late.length === 0)
            return;
        const stuck = Object.assign(Object.create(null), factsStuck);
        for (const view of late) {
            const path = String(view.path);
            stuck[path] = holdAfter(factsStuck[path]);
            console.warn(`Networks: ${path} took two seconds to load, so it's said as last`
                + ` loaded, and asked again in ${stuck[path].wait / 60000} min`);
        }
        factsStuck = stuck;
        factsAsked = factsAsked.filter(view => !late.includes(view));
        settled();
    }
    // A fact landing after it's given up keeps its hold (holdAfter); one
    // landing in time lets it go.
    function factLanded(view) {
        const late = !factsAsked.includes(view);
        factsAsked = factsAsked.filter(each => each !== view);
        const path = String(view.path);
        if (!late && factsStuck[path] !== undefined) {
            const stuck = Object.assign(Object.create(null), factsStuck);
            delete stuck[path];
            factsStuck = stuck;
        }
        settled();
    }
    // Read once nothing loading for an answer is still under way.
    function settled() {
        if (factsAsked.length === 0 && cableLoading === null && cablesAsked.length === 0)
            stillReading = false;
    }

    // Each interface's default gateway and its cost, IPv4's and IPv6's, as
    // the routes said when last loaded, and empty before the first load
    // lands, so an answer reads them, never a file.
    property var routes: Object.create(null)
    property var routes6: Object.create(null)

    // A load landed: when the gateways or their costs changed, what
    // changed is said again where it's shown, and the drawers made again,
    // as a link that changes is (heard). The tables are compared as Address
    // reads them, not as text, since each route's use counts change in
    // them with every connection made.
    function routed() {
        const now = Interfaces.gateways(route.text());
        const now6 = Interfaces.gateways6(sixRoute.text());
        if (JSON.stringify([now, now6]) === JSON.stringify([routes, routes6]))
            return;
        routes = now;
        routes6 = now6;
        glanceSoon();
        if (provider.launcher?.open)
            answered();
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
    property var leases: Object.create(null)
    property var neighbours: Object.create(null)

    // Every listed connection's facts at a glance (glance), by its name, for
    // its drawer and the way out's, made in a turn of their own rather than
    // inside the launcher's binding that asks: making one asks for its
    // cable (cableOf), which writes what the binding read (stillReading,
    // cablesAsked), as reading files there once did, which made the
    // drawer again and again, 297,092 loops in one VM run (opens-and-sets,
    // 2026-11-12). Made ahead of the ask, and made again in place as the
    // connections, their leases or their neighbours change and as the
    // launcher opens, a drawer shows whole or not yet, never growing under
    // Joel's eye (VKCF3Z, 2026-11-12, after Address's came part-made). Shut,
    // changes are gathered into one make a second at most, as a neighbour
    // table may churn and the closed launcher wakes under five times a
    // second (VKCF3Z); open, a quarter second after, past the opening's
    // first frame.
    property var glances: Object.create(null)
    function keptGlance(name, said) {
        const kept = glances[name];
        return kept === undefined ? null : kept.filter(fact => fact.value !== said);
    }
    function makeGlances() {
        const now = standing();
        if (now === null)
            return;
        const made = Object.create(null);
        for (const link of listed(now)) {
            // A cable whose files haven't landed keeps what it was, or
            // none, rather than being made without its link, which would
            // then grow under Joel's eye; but for a card given up, made
            // as the daemon says it, since its files may not land for a
            // while. The name servers and the uptime need no such wait:
            // they load as the shell starts, a file each, long before the
            // machine daemon's first word on the links and the second's
            // gathering after it.
            if (link.kind === "wired" && cables[link.name] === undefined
                && !stuck(link.name)) {
                if (glances[link.name] !== undefined)
                    made[link.name] = glances[link.name];
                recable();
                continue;
            }
            made[link.name] = glance(link.name, []);
        }
        if (JSON.stringify(made) !== JSON.stringify(glances))
            glances = made;
    }
    function glanceSoon() {
        if (!glancing.running)
            glancing.start();
    }
    Timer {
        id: glancing

        interval: provider.launcher?.open ? 250 : 1000
        onTriggered: provider.makeGlances()
    }
    // A link that changes most often changes the routes with it.
    onLinksChanged: {
        reroute();
        forgetCables();
        recable();
        glanceSoon();
    }
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
        if (JSON.stringify([leases, neighbours]) !== before) {
            glanceSoon();
            answered();
        }
    }

    Socket {
        id: networkd
        path: provider.networkdSocket
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

    // Each cable's link as its card's files said when last loaded
    // (Interfaces.cable), by the card's name; one not loaded yet isn't
    // here, and one gone from the links, or whose link the daemon says has
    // changed since, is forgotten. Kept with no prototype, so a card
    // named as one of an object's own members is a card like any other.
    property var cables: Object.create(null)
    // The daemon's word on each cable's link as its files last began to
    // load.
    property var cableStates: Object.create(null)
    // The files a cable's link is read from, in its card's folder, as
    // Interfaces.cable reads them.
    readonly property var cableFiles: {
        const files = [];
        Interfaces.cable(file => {
            files.push(file);
            return "";
        });
        return files;
    }
    // The cards whose files are still to be loaded, in turn, and the one
    // under way, { name, at, texts, state, stale, inFlight }, or null with
    // none: `state` is the daemon's word on its link as it began, `stale`
    // whether that word has changed since, or the card gone, so what it
    // loads isn't kept, and `inFlight` whether its file is asked and not
    // yet landed, so it's asked once.
    property var cablesAsked: []
    property var cableLoading: null
    // Each card whose files last took too long, by its name, held off as
    // holdAfter says.
    property var cablesStuck: Object.create(null)
    // Each card given up with its file still reading, by its name, with
    // the view that reads it: a read that hangs can't be called off, and
    // asked again through another view it would take another thread, so
    // the card isn't asked again until it lands, whatever its hold says.
    property var cablesHung: Object.create(null)
    function stuck(name) {
        return holding(cablesStuck[name]) || cablesHung[name] !== undefined;
    }
    // The view each file of the card under way is asked through.
    property var cableProbe: probeMade.createObject(provider)

    // A hold on a file given up, { at, wait }: when, and how long until
    // it's asked again, a minute, then twice as long each time it's given
    // up again, to an hour, since a read that hangs holds one of the
    // threads every view in the shell loads on for as long as it does.
    // It's kept until the file, asked again, lands in time, whatever its
    // link does, so a card that flaps or comes and goes on a hung driver
    // can't take another thread each time, nor a slow file landing late be
    // asked again at once; a clock stepped back ends it rather than
    // holding it off longer.
    function holdAfter(before) {
        return {
            at: Date.now(),
            wait: Math.min(before === undefined ? 60000 : before.wait * 2, 3600000)
        };
    }
    function holding(given) {
        if (given === undefined)
            return false;
        const since = Date.now() - given.at;
        return since >= 0 && since < given.wait;
    }

    // A cable's link as last loaded, or null before its files first land,
    // which are asked for then, and a terminal's show asks again.
    function cableOf(name) {
        const cable = cables[name];
        if (cable !== undefined)
            return cable;
        if (!stuck(name))
            recable();
        return null;
    }

    // Loads every card's files again in the background, a file at a time
    // through one view, the cables said as they were until they land. A
    // card already waiting or under way isn't asked twice, and one given
    // up is passed over when its turn comes (nextCable).
    function recable() {
        const cards = (links ?? []).filter(link => link.kind === "wired")
            .map(link => link.name)
            .filter(name => !cablesAsked.includes(name)
                && (cableLoading === null || cableLoading.stale || cableLoading.name !== name));
        if (cards.length === 0)
            return;
        // Reading until they land, whatever else is asked with them.
        stillReading = true;
        cablesAsked = cablesAsked.concat(cards);
        if (cableLoading === null)
            nextCable();
    }

    function nextCable() {
        cablesAsked = cablesAsked.filter(name => !stuck(name));
        if (cablesAsked.length === 0) {
            cableLoading = null;
            cableWatch.stop();
            settled();
            return;
        }
        const name = cablesAsked[0];
        const state = (links ?? []).find(link => link.name === name)?.state ?? "";
        cableLoading = { name: name, at: 0, texts: {}, state: state, stale: false,
            inFlight: false };
        cablesAsked = cablesAsked.slice(1);
        Qt.callLater(cableFile);
    }

    function cableFile() {
        if (cableLoading === null || cableLoading.inFlight)
            return;
        cableLoading.inFlight = true;
        const path = `${sysfs}/${cableLoading.name}/${cableFiles[cableLoading.at]}`;
        cableWatch.restart();
        if (cableProbe.path === path)
            cableProbe.reload();
        else
            cableProbe.path = path;
    }

    // A file of the card under way landed, saying `text`, or "" where it
    // can't be read: the next is asked a turn on, since a path set inside
    // the view's own failure loads nothing, and once the last has landed
    // the cable is kept, said again where it's shown if it changed.
    function cableLanded(text) {
        const loading = cableLoading;
        if (loading === null)
            return;
        // Each file's two seconds start as it's asked, not as the one
        // before it landed.
        cableWatch.stop();
        loading.inFlight = false;
        loading.texts[cableFiles[loading.at]] = text;
        loading.at += 1;
        if (loading.at < cableFiles.length) {
            Qt.callLater(cableFile);
            return;
        }
        if (cablesStuck[loading.name] !== undefined) {
            const still = Object.assign(Object.create(null), cablesStuck);
            delete still[loading.name];
            cablesStuck = still;
        }
        // The next card is asked though keeping this one fails, so the
        // watchdog never gives up a card whose files all landed.
        try {
            if (!loading.stale)
                keepCable(loading.name, loading.state,
                    Interfaces.cable(file => loading.texts[file] ?? ""));
        } finally {
            nextCable();
        }
    }

    // A card's cable kept as its files said, with the daemon's word on
    // its link as they began to load, said again where it's shown if it
    // changed.
    function keepCable(name, state, now) {
        const states = Object.assign(Object.create(null), cableStates);
        states[name] = state;
        cableStates = states;
        if (JSON.stringify(now) === JSON.stringify(cables[name]))
            return;
        const all = Object.assign(Object.create(null), cables);
        all[name] = now;
        cables = all;
        glanceSoon();
        if (provider.launcher?.open)
            answered();
    }

    // The cables of cards no longer among the links, or whose link the
    // daemon says has changed since their files began to load, forgotten,
    // so its newer word is said until they're loaded again; and a card
    // gone, or changed, under way or waiting, isn't kept once loaded. One
    // given up stays held off through either (holdAfter).
    function forgetCables() {
        const cards = Object.create(null);
        for (const link of links ?? [])
            if (link.kind === "wired")
                cards[link.name] = link.state;
        const kept = Object.create(null);
        const states = Object.create(null);
        for (const name of Object.keys(cables))
            if (cards[name] !== undefined && cards[name] === cableStates[name]) {
                kept[name] = cables[name];
                states[name] = cableStates[name];
            }
        if (Object.keys(kept).length !== Object.keys(cables).length) {
            cables = kept;
            cableStates = states;
        }
        cablesAsked = cablesAsked.filter(name => cards[name] !== undefined);
        if (cableLoading !== null && cards[cableLoading.name] !== cableLoading.state)
            cableLoading.stale = true;
    }

    // A file that takes two seconds, as one a busy driver holds up does,
    // gives up its card's load, its cable said as last loaded, or as the
    // daemon says its link before any has landed, rather than as a file
    // that said nothing, and warned of, and its drawer made as the daemon
    // says it. A view pointed elsewhere drops its late read, but the read
    // still holds its thread, so the view still reading is kept by its card
    // (cablesHung), another made for the next, and the card isn't asked
    // again until that view lands. It's held off (holdAfter) as well, not
    // asked again each second.
    Timer {
        id: cableWatch

        interval: 2000
        onTriggered: provider.cableGivenUp()
    }
    property alias cableWatching: cableWatch.running
    function cableGivenUp() {
        const loading = cableLoading;
        if (loading === null)
            return;
        const still = Object.assign(Object.create(null), cablesStuck);
        still[loading.name] = holdAfter(cablesStuck[loading.name]);
        cablesStuck = still;
        console.warn(`Networks: ${loading.name}'s ${cableFiles[loading.at]} took two seconds`
            + " to load, so its cable is said as last loaded, and asked again once that"
            + ` read lands and ${still[loading.name].wait / 60000} min have passed`);
        if (loading.inFlight) {
            const hung = Object.assign(Object.create(null), cablesHung);
            hung[loading.name] = cableProbe;
            cablesHung = hung;
            cableProbe = probeMade.createObject(provider);
        }
        nextCable();
        glanceSoon();
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open) {
                provider.look();
                glancing.restart();
            }
        }
    }

    // The connections as they stand, asking networkd again and loading
    // the routes again when what they said is old, but for a cable's link
    // read again while it's shown: { links, routes, routes6, out }, or
    // null before the daemon first answers. The routes are as last loaded,
    // never waited on (reroute).
    function standing() {
        if (!ticking && Date.now() - lookedAt > stands)
            look();
        if (links === null) {
            waiting = true;
            return null;
        }
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
        const rows = shown.map(link => {
            const name = linkName(link, shown);
            const side = addressOf(link) || stateSaid[link.state];
            return thingRow({
                thing: true,
                key: `networks:${link.name}`,
                name: name,
                side: side,
                glyph: linkGlyph(link.kind),
                keywords: link.name,
                // What its kind was called, so "wired" still finds Ethernet.
                once: Words.of(`networks:${link.kind}`).once ?? [],
                note: link.name === now.out ? "where the machine's traffic leaves" : "",
                drawer: () => provider.linkDrawer(link.name, name, side),
                // Not yet made, so a terminal's show asks again (drawerOf).
                drawerUnread: () => provider.glances[link.name] === undefined
            }, () => provider.partsOf(link.name));
        });
        const out = shown.find(link => link.name === now.out) ?? shown[0] ?? null;
        const first = out ? out.four[0] ?? out.six[0] ?? null : null;
        const headline = first ? shortened(first.address) : "no address";
        const publics = lookups ? [publicRow("https://api.ipify.org?format=json", "public"),
            publicRow("https://api6.ipify.org?format=json", "public6")] : [];
        rows.push(...publics);
        return {
            headline: headline,
            copy: first ? shortened(first.address) : "",
            rows: rows,
            note: lookups ? "public addresses from ipify" : "",
            // A thing with more to say than its address: the way out's
            // gateway and name servers, and the addresses the internet
            // sees it by.
            thing: true,
            drawer: () => provider.addressDrawer(out ? out.name : "",
                out ? linkName(out, shown) : headline, headline, publics)
        };
    }

    // The facts of a connection's parts that a glance wants, by the part's
    // id, in the order they're said: the address in full with its
    // prefix, the gateway and name servers, the cable's speed, the lease,
    // the packet size and state, then the hardware address, which
    // identifies it, last.
    readonly property var glanced: ["address", "ipv6", "gateway", "dns", "speed", "duplex",
        "lease", "neighbour", "mtu", "state", "hardware"]

    // A connection's facts, each part said once, one already said by
    // `said` (its row's side or the headline) left out, as a drawer never
    // repeats its row.
    function glance(name, said) {
        const parts = partsOf(name);
        const facts = [];
        for (const id of glanced) {
            const found = parts.find(row => row.name === named(id)) ?? null;
            if (found === null || found.side === "" || said.includes(found.side))
                continue;
            facts.push({ label: found.name, value: found.side, identifies: id === "hardware" });
        }
        return facts;
    }

    // A connection's drawer: what it is, then its parts at a glance, the
    // whole list a step in.
    function linkDrawer(name, title, side) {
        const facts = keptGlance(name, side);
        return facts === null ? null
            : { eyebrow: eyebrow("connection"), title: title, facts: facts };
    }

    // The address answer's drawer, the way out's: named by the connection
    // the traffic leaves by, its parts at a glance, the address the row
    // says left out, then the public addresses as ipify last saw them,
    // or that it found no route there, as an IPv6 address may.
    function addressDrawer(name, title, headline, publics) {
        const seen = publics.filter(row => row.side !== "\u2026" && row.side !== "asking\u2026")
            .map(row => ({ label: row.name, value: row.side }));
        const facts = name === "" ? [] : keptGlance(name, headline);
        if (facts === null)
            return null;
        return { eyebrow: eyebrow("wayOut"), title: title, facts: facts.concat(seen) };
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
            provider.reroute();
            provider.reloadFacts();
            provider.recable();
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
            return Object.assign(part(key, id, said.value.ip), {
                // Who holds it, as the registry says, and the block it's
                // in, asked of rdap.org once it's looked at.
                thing: true,
                drawer: () => provider.publicDrawer(id, said.value.ip)
            });
        return part(key, id, said.failed ? "no route" : "asking\u2026", "", "");
    }

    // A public address's drawer: who holds it and the block it's in, as
    // the registry says, or that it's still asking.
    function publicDrawer(id, ip) {
        const at = Addresses.read(ip);
        const held = at ? holderOf(at, at.text) : null;
        const facts = held ? held.rows.map(row => ({ label: row.name, value: row.side })) : [];
        return {
            eyebrow: eyebrow("public"),
            title: named(id),
            body: held ? held.note : "",
            facts: facts
        };
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
            const asked = nameServers();
            if (asked.length > 0)
                rows.push(part(key("dns"), "dns", asked.join(", "),
                    "what this machine asks names of"));
        }
        const cable = link.kind === "wired" ? cableOf(name) : null;
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
        // A cable's carrier, loaded since the daemon last said its link
        // changed, is the newer word, so it's said over the daemon's.
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
        const errors = cableOf(name)?.errors ?? null;
        if (errors === null)
            return [];
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
    // lease's times: as last loaded and counted on since, or null before
    // it first lands.
    property real upLoaded: 0
    property real upAt: 0
    function uptime() {
        return upAt === 0 ? null : upLoaded + Math.max(0, Date.now() - upAt) / 1000;
    }
    // Said again where a lease is shown the first time it lands, and when
    // a load moves it from what was counted on by more than a minute, as
    // a clock stepped does.
    function upLanded() {
        const counted = uptime();
        upLoaded = Number(up.text().trim().split(/\s+/)[0]) || 0;
        upAt = Date.now();
        const moved = counted === null || Math.abs(counted - upLoaded) > 60;
        if (moved && Object.keys(leases).length > 0) {
            glanceSoon();
            if (provider.launcher?.open)
                answered();
        }
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
        if (lease.ends !== null && now !== null)
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
        if (lease.got !== null && now !== null)
            rows.push(said("got", fromNow(lease.got, now), "given or last renewed", ""));
        if (lease.renews !== null && now !== null)
            rows.push(said("renews", fromNow(lease.renews, now),
                "when it asks its server to keep it", ""));
        if (lease.rebinds !== null && now !== null)
            rows.push(said("rebinds", fromNow(lease.rebinds, now),
                "when it asks any server, if its own hasn't answered", ""));
        if (lease.ends !== null && now !== null)
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
    // is what's worth seeing. With no Ethernet card, as once a USB adapter
    // is pulled out, it holds its place in Network saying it's not plugged
    // in, never gone, so a terminal's show finds the row Network's head
    // names, as Heat's does (VKCF3Z, 2026-11-12); a card with no carrier
    // says there's nothing at the other end.
    function cabled() {
        const now = standing();
        if (now === null)
            return unknown;
        // In ip's order, as the addresses number them, so a card has one
        // name in both.
        const cards = now.links.filter(link => link.kind === "wired");
        const said = link => {
            const cable = cableOf(link.name);
            return cable !== null && cable.speed !== null ? Interfaces.linkRate(cable.speed,
                (value, places) => Numbers.format(value, places, settings.group, settings.point))
                : stateSaid[cable?.state ?? link.state];
        };
        if (cards.length === 0)
            return {
                headline: "",
                copy: "",
                glyph: Theme.glyph.wired,
                rows: [],
                note: String((Words.of("ethernet").said ?? {}).unplugged ?? ""),
                parted: true
            };
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
    // systemd-resolved's own file where resolv.conf names only its stub,
    // as last loaded.
    property var servers: []
    function nameServers() {
        return servers;
    }
    // Either file landed: the servers are worked out again, and said again
    // where they're shown if they changed.
    function resolvLanded() {
        const said = Interfaces.servers(resolv.text());
        const now = Interfaces.stubbed(said) ? Interfaces.servers(resolved.text()) : said;
        if (JSON.stringify(now) === JSON.stringify(servers))
            return;
        servers = now;
        glanceSoon();
        if (provider.launcher?.open)
            answered();
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

    // Loaded in the background as the provider is made and after
    // (reroute), never waited on: one blocking until its first load finds
    // the file waits again after each reload while none has, as
    // ipv6_route would on a machine with IPv6 off.
    FileView {
        id: route
        path: `${provider.kernel}/route`
        printErrors: false
        onLoaded: provider.routed()
    }
    FileView {
        id: sixRoute
        path: `${provider.kernel}/ipv6_route`
        printErrors: false
        onLoaded: provider.routed()
    }
    // Loaded in the background as the routes are (reloadFacts), never
    // waited on.
    FileView {
        id: resolv
        path: provider.resolvFile
        printErrors: false
        onLoaded: {
            provider.resolvLanded();
            provider.factLanded(resolv);
        }
        onLoadFailed: {
            provider.resolvLanded();
            provider.factLanded(resolv);
        }
    }
    FileView {
        id: resolved
        path: provider.resolvedFile
        printErrors: false
        onLoaded: {
            provider.resolvLanded();
            provider.factLanded(resolved);
        }
        onLoadFailed: {
            provider.resolvLanded();
            provider.factLanded(resolved);
        }
    }
    FileView {
        id: up
        path: provider.uptimeFile
        printErrors: false
        onLoaded: {
            provider.upLanded();
            provider.factLanded(up);
        }
        onLoadFailed: provider.factLanded(up)
    }
    // A cable's file landed through `view`: the card under way's, or one
    // given up landing late, which lets its view go and keeps its hold.
    function probeLanded(view, text) {
        if (view === cableProbe) {
            cableLanded(text);
            return;
        }
        const hung = Object.assign(Object.create(null), cablesHung);
        for (const name of Object.keys(hung))
            if (hung[name] === view) {
                delete hung[name];
                console.info(`Networks: ${name}'s read given up landed, so it's asked again`
                    + " once its wait is over");
            }
        cablesHung = hung;
        view.destroy();
    }
    // Each cable's files in turn (recable).
    Component {
        id: probeMade

        FileView {
            id: probe

            printErrors: false
            onLoaded: provider.probeLanded(probe, text())
            onLoadFailed: provider.probeLanded(probe, "")
        }
    }
}
