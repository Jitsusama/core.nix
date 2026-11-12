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
// turn the switch, which is an option, as every switch is.
//
// The rows are the eero daemon's (modules/eero's rows.rs), served ready
// in the launcher's words (ReadyRows.qml), so nothing here words a row or
// orders one, and a row is made again only once the daemon says it
// changed. Each row's drawer and what's a step into it are still worked
// out in HomeNetworkDrawers.js, held in Qt's own engine
// (tests/home-network.qml), from the record Eero.qml keeps, found by what
// the row holds, until the daemon serves them too.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "HomeNetwork.js" as Home
import "HomeNetworkDrawers.js" as Drawers
import "Order.js" as Order

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
            // An eyebrow naming a node or an internet provider keeps its case, as a name
            // from the world is never re-cased (Drawer.qml).
            eyebrowNames: said.names === true,
            title: said.title,
            facts: said.facts.map(one => one.heading !== undefined
                ? { heading: provider.named(scope, one.heading) }
                : {
                    label: one.label ?? provider.named(scope, one.id, one.fill),
                    value: one.value,
                    figure: one.figure ?? undefined,
                    // A sentence, as eero's events are, set as prose in
                    // Words rather than as a figure (VKCF3Z, 2026-11-08).
                    words: one.words ?? undefined
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

    // The place's rows as the eero daemon serves them, in the launcher's
    // words and number settings, each a file in the store, since the
    // daemon reads nothing else.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/eero/socket`
        place: "eero"
        watching: ({
                words: provider.settings.words ?? "",
                numbers: provider.settings.numbers ?? ""
            })
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    // A served row as the place draws it: a thing, its drawer and what's
    // a step in it worked out from the record as they're asked. Each says
    // no note on the cursor's row while its drawer is out, as the drawer
    // says every part of it, so the row's side reads as its neighbours'
    // do (VKCF3Z, 2026-11-08).
    function madeFrom(row, key) {
        const kind = row.kind ?? "";
        const held = row.held ?? {};
        const said = () => provider.drawerSaid(kind, held);
        const made = ready.thing(row, key, {
            thing: true,
            noteInDrawer: true,
            what: kind === "device" ? "eero device" : kind,
            // Drawn as a figure, as a copied value is, and copying what
            // it says at its side.
            figure: true,
            plain: true,
            copies: () => typeof made?.side === "string" ? made.side : "",
            // The first node and the first profile each start a group.
            gap: (kind === "node" || kind === "profile")
                && ready.keys.find(one => ready.held[one]?.row?.kind === kind) === key,
            drawer: () => provider.plate(kind, said())
        });
        if (made === null)
            return null;
        return entering(made, kind === "internet"
            ? () => provider.acts().concat(provider.parts(kind, made.key,
                provider.withoutSwitch(said())))
            : () => provider.parts(kind, made.key, said()));
    }

    // What a row's drawer says, of the record's internet, node, device or
    // profile the row holds; nothing while the record doesn't hold it.
    function drawerSaid(kind, held) {
        const nothing = { facts: [] };
        if (kind === "internet")
            return Drawers.internet(model, now(), settings);
        if (kind === "node") {
            const node = (model?.nodes ?? []).find(one => one.id === held.id);
            return node ? Drawers.node(model, node, now(), settings) : nothing;
        }
        if (kind === "device") {
            const device = (model?.devices ?? []).find(one => one.mac === held.mac);
            return device ? deviceDrawer(device) : nothing;
        }
        const profile = (model?.profiles ?? []).find(one => one.id === held.id);
        return profile ? Drawers.profile(model, profile, settings) : nothing;
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

    // A device's drawer, with its readings since the shell started.
    function deviceDrawer(device) {
        return Drawers.device(model, device, now(), {
            settings: settings,
            readings: Eero.history[device.mac]
        });
    }

    // Every row as the daemon serves them, in its order: the internet,
    // then each node with its devices under it, then the profiles. The
    // record itself is asked, not `present`, which follows it a binding
    // later, so a record let go draws no row whose drawer says nothing.
    readonly property var all: model === null || model === undefined ? [] : ready.rows()
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
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
        return { rows: kept.map(f => Object.assign({}, f.row, { depth: 0 })), grade: finest };
    }
}
