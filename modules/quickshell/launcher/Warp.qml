// WARP, the VPN, answered in the field by its words ("warp" or "vpn", as
// words.json lists them) and listed in Network after Speed. The headline
// says how its link stands, "connected · 127 ms", in the warning colour
// while WARP finds the network degraded or its service doesn't answer;
// a step in are its State and why, the network's Health as WARP judges it,
// the tunnel's Latency and Loss as WARP estimates them, its Protocol, the
// Colo it ends in, and what it has Sent and Received (Warp.js works each
// out). A machine whose system installs no warp-cli has no WARP here at
// all (home.nix enables this only where it does).
//
// What it shows comes from `warp --json` (modules/warp), which asks
// warp-cli and keeps nothing, so it's run as the answer is wanted and
// again every two seconds while it's shown, never between: warp-cli's
// answers take about 80 ms together, and nothing is spawned while the
// launcher is closed or shows something else. Whatever warp printed last
// stands until the next, so the figures move in place rather than
// flickering back to waiting.
import qs
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers
import "Warp.js" as Warp

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

    // Whether anyone can see what it says, so warp is run only then.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider || launcher.placed.includes("warp")
            || launcher.held.some(step => step.row.asker === provider))

    function number(value, places) {
        return Numbers.format(value, places, settings.group, settings.point);
    }

    function answer(typed) {
        if (warp === "" || asks(typed) === "")
            return null;
        if (launcher !== null && (launcher.open || launcher.cataloguing)
            && (said === null || Date.now() - saidAt > fresh))
            look();
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
        const failed = Warp.failed(said);
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

    function told() {
        const linked = Warp.told(said, number);
        if (linked === null)
            return null;
        return {
            headline: linked.headline,
            copy: linked.headline,
            warns: linked.warns,
            glyph: Words.glyph("warp"),
            rows: linked.parts.map(part => copyRow({
                key: `warp:${part.id}`,
                name: Words.name(`warp:${part.id}`),
                side: part.side,
                copy: part.copy,
                note: part.note,
                warns: part.warns,
                hue: "machine",
                glyph: ""
            })),
            note: "",
            live: provider
        };
    }

    function look() {
        if (!asker.running)
            asker.running = true;
    }

    // What warp printed, kept until the next, unless the launcher closed
    // meanwhile, so the next opening asks afresh.
    function heard(text) {
        if (launcher === null || !(launcher.open || launcher.cataloguing))
            return;
        const printed = parsed(text);
        said = printed.status !== undefined || printed.error !== undefined
            ? printed : { error: "unreadable", said: "warp printed nothing it could read" };
        saidAt = Date.now();
        answered();
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.said = null;
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
}
