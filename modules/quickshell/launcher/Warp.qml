// WARP, the VPN, answered in the field by its words ("warp" or "vpn", as
// words.json lists them) and listed in Network after Speed. The headline
// says how its link stands, "connected · 127 ms", in the warning colour
// while WARP finds the network degraded or its service doesn't answer;
// a step in are its switch, Disconnect WARP or Connect WARP as :disconnect
// and :connect name it, then its State and why, the network's Health as
// WARP judges it, its Settings (the mode, always on and the split tunnel
// first, each saying who set it), the tunnel's Round Trip and Loss as
// WARP estimates them, its Protocol, the Data Centre it ends in, its
// Endpoint and Last Handshake, its Traffic each way and its TLS, and what
// its DNS proxy answered (Warp.js works each out, WarpSeen.js the sparks
// and WarpSwitch.js the switch, in the words words.json gives warp). A
// machine
// whose system installs no warp-cli has no WARP here at all (home.nix
// enables this only where it does).
//
// What it shows comes from `warp --json` (modules/warp), which asks
// warp-cli and keeps nothing, so it's run as the answer is wanted and
// again every two seconds while it's shown, never between: warp-cli's
// answers take about 80 ms together, and nothing is spawned while the
// launcher is closed or shows something else, but for a terminal's ask
// (`launcher show`). Whatever warp printed last stands until the next, so
// the figures move in place rather than flickering back to waiting. What
// was read while it's shown is kept for the tunnel's sparks, whose
// drawers say once whose figures they are, and forgotten as the launcher
// opens or closes. The switch is warp's too, `warp
// connect` and `warp disconnect`, which leave one the organisation holds
// as it is and say so.
import qs
import Quickshell.Io
import QtQuick
import "Warp.js" as Warp
import "WarpSeen.js" as WarpSeen
import "WarpSwitch.js" as WarpSwitch

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("warp")

    readonly property string warp: settings.warp ?? ""
    readonly property string warpCli: settings.warpCli ?? ""
    // How old what warp printed may be and still be what's shown.
    readonly property int fresh: 2000

    // What warp printed last, as JSON: the record, or { error, said }; null
    // before it first answers this opening.
    property var said: null
    property real saidAt: 0
    // What it has read of the tunnel since it was shown (WarpSeen.saw), for
    // the sparks, or null before a read; forgotten as the launcher closes.
    property var seen: null
    // What the last switch said, for its row's note, until the launcher
    // closes.
    property string switchSaid: ""
    // Whether warp was last asked for a terminal (`launcher show`) or the
    // catalogue while the launcher is closed. Their ask is answered at
    // once and can't wait on warp, so what it prints is kept for the next
    // ask, and the reads after spark as they do while shown.
    property bool askedOutside: false
    // How old what an ask from outside kept may be and still be said to
    // the next, rather than asked afresh.
    readonly property int keptOutside: 60000

    // Whether anyone can see what it says, so warp is run only then.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider || launcher.placed.includes("warp")
            || launcher.held.some(step => step.row.asker === provider))

    // warp's words, as words.json gives them.
    readonly property var words: Words.of("warp").said ?? {}

    function answer(typed) {
        if (warp === "" || asks(typed) === "")
            return null;
        const outside = launcher !== null && !launcher.open && launcher.cataloguing;
        if (outside && said !== null && Date.now() - saidAt > keptOutside) {
            said = null;
            seen = null;
        }
        if (launcher !== null && (launcher.open || launcher.cataloguing)
            && (said === null || Date.now() - saidAt > fresh)) {
            askedOutside = outside;
            look();
        }
        if (said === null)
            return waitingAnswer();
        return said.error !== undefined ? failedAnswer() : told();
    }

    // While warp first answers: held in its place, its figures to come.
    function waitingAnswer() {
        return {
            headline: "\u2026",
            parted: true,
            rows: [],
            asking: true,
            glyph: Words.glyph("warp"),
            note: "",
            live: provider
        };
    }

    // warp couldn't read WARP: its service isn't answering, or said what
    // warp can't read, which the row says in its place; or there's no
    // warp-cli, so no WARP at all.
    function failedAnswer() {
        const failed = Warp.failed(said, words);
        if (failed === null)
            return null;
        return {
            headline: failed.headline,
            warns: failed.warns,
            whole: true,
            rows: [],
            glyph: Words.glyph("warp"),
            note: failed.note,
            live: provider
        };
    }

    // How the link stands: its switch first, as :connect and :disconnect
    // name it, then every part.
    function told() {
        const linked = Warp.told(said, settings, words);
        if (linked === null)
            return null;
        const parts = WarpSeen.drawn(linked.parts, seen, settings, words);
        return {
            headline: linked.headline,
            copy: linked.headline,
            warns: linked.warns,
            glyph: Words.glyph("warp"),
            rows: [switchRow()].concat(parts.map(partRow)),
            note: "",
            live: provider
        };
    }

    // A part's drawer, where it has one: its eyebrow saying whose figures
    // they are, once, its name, and each fact named by the vocabulary.
    function plate(part) {
        return {
            eyebrow: part.drawer.eyebrow,
            title: Words.name(`warp:${part.id}`),
            facts: part.drawer.facts.map(fact => ({
                label: Words.name(`warp:${fact.id}`),
                value: fact.value,
                figure: fact.figure ?? undefined
            }))
        };
    }

    // Disconnect WARP while it's up or on its way, Connect WARP while it's
    // down, as the vocabulary names each, so :disconnect and :connect name
    // them; drawn as `upAs` says it will be, where it's given. Its note
    // says who holds a switch the organisation holds, when WARP always on
    // comes back, or what the last switch said.
    function switchRow(upAs) {
        const up = upAs ?? WarpSwitch.up(said);
        const wordState = up ? "" : "off";
        const note = switchSaid !== "" ? switchSaid : WarpSwitch.switchNote(said, up, words);
        return thing(Object.assign({
            key: "warp:power",
            what: "",
            verb: Words.verb("warp:power", wordState),
            name: Words.name("warp:power", wordState),
            glyph: Words.glyph("warp:power", wordState),
            // Its words' state, so the : line names the command for it.
            wordState: wordState,
            hue: "action",
            note: note,
            stays: true,
            state: () => provider.switchSaid !== "" ? provider.switchSaid : note,
            run: () => provider.turn(!up),
            undo: () => provider.turn(up)
        }, up ? Words.ending("warp:power") : {}));
    }

    // Has warp switch WARP, its row drawn as it will be until warp reads
    // it so, and put back with why if warp leaves it. warp itself refuses
    // a switch the organisation holds, asking warp-cli nothing.
    function turn(up) {
        if (switcher.running)
            return;
        switchSaid = "";
        becomes("warp:power", switchRow(up), Words.name("warp"), up ? "connect" : "disconnect");
        switcher.command = [warp, up ? "connect" : "disconnect", "--json"];
        switcher.running = true;
    }

    // What warp said of a switch (modules/warp/src/switch.rs): done, its
    // note says when WARP comes back where it will; held or refused, the
    // row goes back and says why.
    function switched(text) {
        const done = WarpSwitch.switched(parsed(text), words);
        if (done.refused !== "") {
            refused("warp:power", done.refused);
            return;
        }
        switchSaid = done.note;
        look();
    }

    // A part as its row: a figure Return copies, or one holding parts of
    // its own, as the Settings, TLS and DNS do, which wears WARP's glyph
    // dimmed and steps in to them. An entry of a list, a split tunnel's
    // host or a DoH server's address, is named by itself.
    function partRow(part) {
        const entry = part.name !== undefined;
        const fields = {
            key: entry ? `warp:${part.id}:${part.name}` : `warp:${part.id}`,
            name: entry ? part.name : Words.name(`warp:${part.id}`),
            side: part.side,
            copy: part.copy,
            note: part.note,
            warns: part.warns,
            // A figure with a history sparks it, and its drawer says the
            // rest (WarpSeen.drawn).
            graph: part.graph,
            drawer: part.drawer ? () => provider.plate(part) : undefined,
            hue: "machine",
            glyph: ""
        };
        if (part.parts === undefined)
            return copyRow(fields);
        return holderRow(Object.assign(fields, { glyph: Words.glyph("warp") }),
            () => part.parts.map(partRow));
    }

    function look() {
        if (!asker.running)
            asker.running = true;
    }

    // What warp printed, kept until the next, unless the launcher closed
    // meanwhile, so the next opening asks afresh, or it was asked from
    // outside, so the next ask says it; and what the sparks keep of it.
    function heard(text) {
        const kept = askedOutside;
        askedOutside = false;
        if (launcher === null || !(launcher.open || launcher.cataloguing || kept))
            return;
        const printed = parsed(text);
        said = printed.status !== undefined || printed.error !== undefined
            ? printed : { error: "unreadable", said: words.printedNothing ?? "" };
        saidAt = Date.now();
        if (said.status !== undefined)
            seen = WarpSeen.saw(seen, said, saidAt);
        answered();
    }

    Connections {
        target: provider.launcher

        // Opened or closed, it asks afresh, so what a terminal was told
        // is never shown as though just asked.
        function onOpenChanged() {
            provider.said = null;
            provider.seen = null;
            provider.switchSaid = "";
        }
    }

    Timer {
        interval: provider.fresh
        repeat: true
        running: provider.shown
        onTriggered: provider.look()
    }

    Process {
        id: asker
        command: [provider.warp, "--json"]
        // The warp-cli the system runs, rather than whatever the shell's
        // PATH finds first.
        environment: provider.warpCli === "" ? ({}) : ({ WARP_CLI: provider.warpCli })
        stdout: StdioCollector {
            id: printed
            onStreamFinished: provider.heard(printed.text)
        }
    }

    // warp connect or warp disconnect, which say what they did as a
    // record with --json, done, held or refused.
    Process {
        id: switcher
        environment: provider.warpCli === "" ? ({}) : ({ WARP_CLI: provider.warpCli })
        stdout: StdioCollector {
            id: switchPrinted
            onStreamFinished: provider.switched(switchPrinted.text)
        }
    }
}
