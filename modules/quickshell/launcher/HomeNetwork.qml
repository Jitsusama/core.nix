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
// Each is a thing whose drawer holds a glance, at most six, of every
// figure the eero contract places there (modules/eero/contract/README.md),
// and stepping into one lists every figure as rows, each copied by Return. The internet holds what
// can be done to the home network too: Run a Speed Test and Optimize for
// Conferencing and Gaming, rows the daemon serves inside it whose acts it
// does itself, answering once eero has, so a refusal is said in eero's
// words (ReadyRows.qml's asked). A speed test fills the internet for most
// of a minute, so Return puts :speedtest on the command line and a second
// Return runs it; :set sqm and :set nosqm turn the switch, which is an
// option, as every switch is, by the switch's own act.
//
// The rows are the eero daemon's (modules/eero's rows.rs), served ready
// in the launcher's words (ReadyRows.qml), so nothing here words a row or
// orders one, and a row is made again only once the daemon says it
// changed. Every row's drawer is served beside it, ready too
// (modules/eero's drawers.rs), its times worded as it's drawn, and what's
// a step into it is every fact the drawer serves inside it, as rows.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "HomeNetwork.js" as Home
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

    // A drawer's figures as rows a step in, each copied by Return and noted
    // by the heading it's under, as the drawer has it.
    function parts(key, said) {
        let under = "";
        const rows = [];
        for (const one of said.facts) {
            if (one.heading !== undefined) {
                under = one.heading;
                continue;
            }
            const name = one.label;
            const graph = one.figure ?? null;
            rows.push(copyRow({
                key: `${key}:${name}`,
                name: name,
                side: one.value,
                copy: one.value,
                hue: "machine",
                glyph: "",
                note: under,
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
        // The place's own drawer and every row's, served beside the rows.
        beside: ["drawer"]
        watching: ({
                words: provider.settings.words ?? "",
                numbers: provider.settings.numbers ?? ""
            })
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    // A served row as the place draws it: a thing, its drawer served beside
    // it, and what's a step in it. Each says
    // no note on the cursor's row while its drawer is out, as the drawer
    // says every part of it, so the row's side reads as its neighbours'
    // do (VKCF3Z, 2026-11-08).
    function madeFrom(row, key) {
        const kind = row.kind ?? "";
        // What can be done to the home network, inside the internet, is
        // drawn as ready-rows draws any served act. The switch reads by its
        // words' state, as the command line does.
        if ((row.parent ?? "") !== "")
            return ready.thing(row, key, { wordState: row.held?.on === true ? "on" : "" });
        const said = () => provider.inside(row);
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
            drawer: () => ready.drawerOf(row)
        });
        if (made === null)
            return null;
        return entering(made, kind === "internet"
            ? () => ready.rows(key).concat(provider.parts(made.key,
                provider.withoutSwitch(said())))
            : () => provider.parts(made.key, said()));
    }

    // Every figure a row's drawer says, which its step in lists: the drawer
    // holds a glance of them, at most six, and serves them all as `inside`,
    // worded here as its facts are. The internet's 57 facts took up to 77 ms
    // making on Joel's machine (2026-11-12), past the launcher's 50.
    function inside(row) {
        const served = typeof row.drawer === "string" ? ready.held[row.drawer]?.drawer
            : row.drawer;
        const every = served?.inside;
        return (Array.isArray(every) ? ready.drawerOf({ drawer: { facts: every } })
            : ready.drawerOf(row)) ?? { facts: [] };
    }

    // The internet's figures less how conferencing and gaming stands, which
    // its switch says a few rows above, so the step says it once.
    function withoutSwitch(said) {
        const sqm = Words.name("eero:internet:sqm");
        return Object.assign({}, said, { facts: said.facts.filter(one => one.label !== sqm) });
    }

    // :set sqm and :set nosqm, by the switch's served act, which turns it
    // the other way; set as it stands, nothing is asked.
    setOptions: !present || (model?.internet?.sqm ?? null) === null ? ({}) : ({
        sqm: {
            kind: "switch",
            now: () => provider.model?.internet?.sqm ?? false,
            set: to => {
                const key = "row:eero:sqm";
                const one = ready.served(key)?.acts?.[0] ?? null;
                if (one !== null && one.ask?.on === (to === true))
                    ready.act(key, one);
                return "";
            }
        }
    })

    // Every row as the daemon serves them, in its order: the internet,
    // then each node with its devices under it, then the profiles. The
    // record itself is asked, not `present`, which follows it a binding
    // later, so a record let go draws no row whose drawer says nothing.
    readonly property var all: model === null || model === undefined ? [] : ready.rows()
    onAllChanged: changedTo(all)

    function everything() {
        return all;
    }

    // The place's own drawer, its row's in Network: how the home network
    // stands and its figures, as the eero daemon serves it beside the
    // rows (its place.rs), the speed test's day worded now. Until it's
    // served, none, so the launcher says the row's own name alone.
    function placePlate() {
        return ready.drawerOf({ drawer: "drawer:eero:place" });
    }

    function counted() {
        return present ? Home.holding(model) : "";
    }

    // A row is found by its name, then by the words the daemon serves it
    // with (a node's model and serial, a device's address), as Launcher's
    // `within` finds one: a served row's words hold only those, not its
    // name, so a node was found by its model and never by where it is.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        const matched = prepared => prepared && (prepared.mask & need) === need
            ? Fuzzy.match(prepared, words, false) : { score: -1 };
        for (const row of everything()) {
            const byName = matched(row.prepared);
            const m = byName.score >= 0 ? byName : matched(row.keywords);
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
