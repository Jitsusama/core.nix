// The Home Network, a place inside Network: the home network as the eero
// daemon keeps it (Eero.qml). Its head is the internet, saying how it
// stands, who carries it, the last speed test and when, whether it's
// optimized for conferencing and gaming, its outages, the nodes by state
// and the devices eero counts; then each node, the gateway node first and
// each after the node it hangs from, saying its devices, its backhaul
// with its bars over Wi-Fi, its model and firmware; and under each node
// its devices, the strongest signal first, each saying its signal, its
// band and channel, its rate each way, its speed now and its address. No
// row draws a graph, whose column would cut a device's name short: the
// signal is a figure at the side, and its stroke and history are the
// drawer's.
//
// After them come the profiles, each saying how it stands and how many of
// its devices are on now.
//
// Each is a thing with a drawer of every figure the eero contract places
// there (modules/eero/contract/README.md), and stepping into one lists
// those figures as rows, each copied by Return. The internet holds what
// can be done to the home network too: Run a Speed Test and Optimize for
// Conferencing and Gaming, which `eero speedtest` and `eero sqm` do,
// started through niri as every program the shell starts is. A speed test
// fills the internet for most of a minute, so Return puts :speedtest on
// the command line and a second Return runs it; :set sqm and :set nosqm
// turn the switch, which is an option, as every switch is. What each says and in what order is
// worked out in HomeNetwork.js and HomeNetworkDrawers.js, held in Qt's
// own engine (tests/home-network.qml); this only makes rows of it.
pragma ComponentBehavior: Bound
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "HomeNetwork.js" as Home
import "HomeNetworkDrawers.js" as Drawers

Provider {
    id: provider

    what: "node"

    readonly property var model: Eero.model
    readonly property bool present: Eero.present

    collection: !present ? null : Words.collection("eero")

    // While the place, or anything in it, is open, the daemon is held to
    // read eero every 30 seconds (Eero.qml), and never otherwise.
    readonly property bool looking: launcher !== null && launcher.open
        && (launcher.held ?? []).some(step => step.row.from === provider)
    onLookingChanged: Eero.watching = looking

    // A part's name, as the vocabulary says it within one of the place's
    // things: "eero:node:backhaul" is Backhaul.
    function named(scope, id, fill) {
        return Words.name(`eero:${scope}:${id}`, "", fill);
    }

    // The time the drawers say "today" against, and ages from.
    function now() {
        return Date.now();
    }

    // A drawer from what HomeNetworkDrawers.js says, each part named by the
    // vocabulary, a stroke drawn as the drawer draws one.
    function plate(scope, said) {
        return {
            eyebrow: said.eyebrow,
            // An eyebrow naming a node or an ISP keeps its case, as a name
            // from the world is never re-cased (Drawer.qml).
            eyebrowNames: said.names === true,
            title: said.title,
            facts: said.facts.map(one => one.heading !== undefined
                ? { heading: provider.named(scope, one.heading) }
                : {
                    label: one.label ?? provider.named(scope, one.id, one.fill),
                    value: one.value,
                    figure: one.figure ?? undefined
                })
        };
    }

    // The same figures as rows a step in, each copied by Return, a heading
    // naming the rows under it, as the drawer's does.
    function parts(scope, key, said) {
        let under = "";
        const rows = [];
        for (const one of said.facts) {
            if (one.heading !== undefined) {
                under = provider.named(scope, one.heading);
                continue;
            }
            const name = one.label ?? provider.named(scope, one.id, one.fill);
            const graph = one.figure ?? null;
            rows.push(copyRow({
                key: `${key}:${one.id ?? ""}:${name}`,
                name: name,
                side: one.value,
                copy: one.value,
                hue: "machine",
                glyph: "",
                note: one.label !== undefined || one.fill !== undefined ? under : "",
                graph: graph === null ? undefined : {
                    shape: graph.shape === "stroke" ? "bar" : graph.shape,
                    values: graph.values,
                    warn: graph.warn
                }
            }));
        }
        return rows;
    }

    // The internet, the place's head.
    function internetRow() {
        const said = Home.internetRow(model, now(), settings);
        return thingRow({
            thing: true,
            what: "internet",
            key: "eero:internet",
            name: Words.name("eero:internet"),
            side: said.side,
            note: said.note,
            glyph: Words.glyph("eero:internet"),
            drawer: () => provider.plate("internet",
                Drawers.internet(provider.model, provider.now(), provider.settings))
        }, () => provider.acts().concat(provider.parts("internet", "eero:internet",
            provider.withoutSwitch(Drawers.internet(provider.model, provider.now(),
                provider.settings)))));
    }

    // The internet's figures less how conferencing and gaming stands, which
    // its switch says a few rows above, so the step says it once.
    function withoutSwitch(said) {
        return Object.assign({}, said, { facts: said.facts.filter(one => one.id !== "sqm") });
    }

    // What can be done to the home network, a step into the internet: run
    // a speed test, and optimize for conferencing and gaming or stop,
    // each by the eero program, started through niri.
    function acts() {
        const sqm = model?.internet?.sqm ?? null;
        const rows = [thing({
            key: "verb:eero:speedtest",
            name: Words.name("verb:eero:speedtest"),
            verb: Words.verb("verb:eero:speedtest"),
            glyph: Words.glyph("verb:eero:speedtest"),
            hue: "action",
            stays: true,
            note: "eero measures the internet from the gateway node",
            run: () => provider.speedTest()
        })];
        if (sqm !== null)
            rows.push(sqmRow(sqm));
        return rows;
    }

    // Optimize for Conferencing and Gaming, a switch saying how it stands.
    function sqmRow(on) {
        const state = on ? "on" : "";
        return thing({
            key: "eero:sqm",
            name: Words.name("eero:sqm", state),
            verb: Words.verb("eero:sqm", state),
            glyph: Words.glyph("eero:sqm", state),
            wordState: state,
            side: on ? "on" : "off",
            hue: "machine",
            stays: true,
            note: "what the internet carries, queued so a call stays quick",
            run: () => provider.optimize(!on)
        });
    }

    // eero's speed test takes it most of a minute, and the daemon says the
    // new one once it reads it.
    function speedTest() {
        Niri.spawn(["eero", "speedtest"]);
        Eero.readNow();
    }

    // The switch reads as it will at once, and goes back if eero hasn't
    // agreed by the daemon's next two reads.
    function optimize(on) {
        becomes("eero:sqm", sqmRow(on), Words.name("eero:sqm"), on ? "on" : "off",
            { within: 75000 });
        Niri.spawn(["eero", "sqm", on ? "on" : "off"]);
        Eero.readNow();
    }

    // :set sqm and :set nosqm, as the row turns it.
    setOptions: !present || (model?.internet?.sqm ?? null) === null ? ({}) : ({
        sqm: {
            kind: "switch",
            now: () => provider.model?.internet?.sqm ?? false,
            set: to => {
                provider.optimize(to === true);
                return "";
            }
        }
    })

    function nodeRow(node, first) {
        const said = Home.nodeRow(node);
        return thingRow({
            thing: true,
            what: "node",
            key: `eero:node:${node.id}`,
            name: said.name,
            side: said.side,
            note: said.note,
            keywords: [node.model ?? "", node.serial ?? ""].join(" "),
            glyph: Words.glyph(node.gateway ? "eero:gateway" : "eero:node"),
            gap: first,
            drawer: () => provider.plate("node", Drawers.node(node, provider.now(),
                provider.settings))
        }, () => provider.parts("node", `eero:node:${node.id}`, Drawers.node(node,
            provider.now(), provider.settings)));
    }

    function deviceRow(device) {
        const said = Home.deviceRow(device, settings, model.profiles);
        return thingRow({
            thing: true,
            what: "eero device",
            key: `eero:device:${device.mac}`,
            name: said.name,
            side: said.side,
            note: said.note,
            keywords: [device.mac, device.hostname ?? "", device.address ?? "",
                Home.worded(device.kind)].join(" "),
            glyph: Theme.glyph[said.mark] ?? "",
            depth: 1,
            drawer: () => provider.plate("device", provider.deviceDrawer(device))
        }, () => provider.parts("device", `eero:device:${device.mac}`,
            provider.deviceDrawer(device)));
    }

    // A device's drawer, with its readings since the shell started.
    function deviceDrawer(device) {
        return Drawers.device(model, device, now(), {
            settings: settings,
            readings: Eero.history[device.mac]
        });
    }

    function profileRow(profile, first) {
        const said = Home.profileRow(model, profile);
        return thingRow({
            thing: true,
            what: "profile",
            key: `eero:profile:${profile.id}`,
            name: said.name,
            side: said.side,
            note: said.note,
            glyph: Words.glyph("eero:profile"),
            gap: first,
            drawer: () => provider.plate("profile", Drawers.profile(provider.model, profile))
        }, () => provider.parts("profile", `eero:profile:${profile.id}`,
            Drawers.profile(provider.model, profile)));
    }

    // Every row, made again as the record changes: the internet, then
    // each node with its devices under it, then the profiles.
    readonly property var all: {
        if (!present)
            return [];
        const rows = [internetRow()];
        Home.grouped(model).forEach((group, at) => {
            if (group.node !== null)
                rows.push(nodeRow(group.node, at === 0));
            for (const device of group.devices)
                rows.push(deviceRow(device));
        });
        (model.profiles ?? []).forEach((profile, at) => rows.push(profileRow(profile, at === 0)));
        return rows;
    }
    onAllChanged: changedTo(all)

    function everything() {
        return all;
    }

    function counted() {
        return present ? Home.holding(model) : "";
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of everything()) {
            const prepared = row.keywords ?? row.prepared;
            if ((prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => Object.assign({}, f.row, { depth: 0 })), grade: finest };
    }
}
