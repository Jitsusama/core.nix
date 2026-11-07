// The Home Network, a place inside Network: the home network as the eero
// daemon keeps it (Eero.qml). Its head is the internet, saying how it
// stands, who carries it, the last speed test and when, whether it's
// optimized for conferencing and gaming, its outages, the nodes by state
// and the devices eero counts; then each node, the gateway node first and
// each after the node it hangs from, saying its backhaul, as a bar of its
// bars over Wi-Fi, its model, devices and firmware; and under each node
// its devices, the strongest signal first, each saying its signal, as a
// bar too, its band and channel, its rate each way and its speed now.
//
// Each is a thing with a drawer of every figure the eero contract places
// there (modules/eero/contract/README.md), and stepping into one lists
// those figures as rows, each copied by Return. The internet holds what
// can be done to the home network too: Run a Speed Test and Optimize for
// Conferencing and Gaming, which `eero speedtest` and `eero sqm` do,
// started through niri as every program the shell starts is; :set sqm
// and :set nosqm turn the second. What each says and in what order is
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

    function bar(level) {
        return level === null ? undefined : { shape: "bar", values: [level], warn: false };
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
            Drawers.internet(provider.model, provider.now(), provider.settings))));
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
            command: Words.ending("verb:eero:speedtest").command,
            hue: "action",
            stays: true,
            note: "eero measures the internet from the gateway node",
            run: () => Niri.spawn(["eero", "speedtest"])
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

    function optimize(on) {
        Niri.spawn(["eero", "sqm", on ? "on" : "off"]);
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
            graph: bar(said.level),
            glyph: Words.glyph(node.gateway ? "eero:gateway" : "eero:node"),
            gap: first,
            drawer: () => provider.plate("node", Drawers.node(node, provider.now(),
                provider.settings))
        }, () => provider.parts("node", `eero:node:${node.id}`, Drawers.node(node,
            provider.now(), provider.settings)));
    }

    function deviceRow(device) {
        const said = Home.deviceRow(device, settings);
        return thingRow({
            thing: true,
            what: "eero device",
            key: `eero:device:${device.mac}`,
            name: said.name,
            side: said.side,
            note: said.note,
            keywords: [device.mac, device.hostname ?? "", device.address ?? "",
                Home.worded(device.kind)].join(" "),
            graph: bar(said.level),
            glyph: Theme.glyph[said.mark] ?? "",
            depth: 1,
            drawer: () => provider.plate("device", Drawers.device(provider.model, device,
                provider.now(), provider.settings))
        }, () => provider.parts("device", `eero:device:${device.mac}`,
            Drawers.device(provider.model, device, provider.now(), provider.settings)));
    }

    // Every row, made again as the record changes: the internet, then
    // each node with its devices under it.
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
