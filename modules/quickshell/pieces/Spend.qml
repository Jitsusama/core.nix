// What AI use costs, as spend-watch (modules/spend-watch) says it: today's
// spend against usual in the overview, and in the margin while something
// wants a look, a cap near or reached, a burn, an override ending. The
// daemon works every figure out and writes each string ready to draw, so
// this reads its status file, asks nothing and formats nothing. While the
// screen is shared it draws the quiet strings, which hold no amounts. It's
// true as well when the daemon has stopped writing, since then nothing
// watches the spend; with no file at all there's no piece.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Piece {
    id: piece

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property var said: file.said
    // The daemon writes its heartbeat at least every 30 seconds while it runs.
    readonly property bool silent: said !== null
        && clock.now - (said.heartbeat ?? 0) > (settings.silence ?? 120)
    // The chord that opens the collection, beside the action that does.
    readonly property var opening: Shortcuts.actions
        .find(action => action.opens === "spend") ?? { chord: [] }
    readonly property var shown: said === null ? null : Quiet.sharing ? said.quiet : said.open
    // Each action's glyph, by the kind the daemon gives it.
    readonly property var glyphs: ({
        go: 0xf03cb,
        ask: 0xf0369,
        "sign-in": 0xf0342,
        report: 0xf0215,
        mark: 0xf012c
    })

    present: said !== null
    active: said !== null && (said.active === true || silent)
    role: silent ? "warning" : said?.role ?? "text"
    wants: said !== null && !silent && said.wants === true
    value: silent ? "not watching" : shown?.value ?? ""
    title: said?.title ?? "AI spend"
    badge: silent ? "" : shown?.badge ?? ""
    hint: silent ? "Nothing is watching what AI use costs" : shown?.hint ?? value
    context: silent ? "the spend-watch unit says why" : shown?.context ?? ""
    actions: (silent ? [] : (said?.actions ?? []).map(action => ({
        glyph: String.fromCodePoint(glyphs[action.kind] ?? 0xf0054),
        name: action.name,
        run: () => Niri.spawn(action.run)
    }))).concat([{
        glyph: String.fromCodePoint(0xf012a),
        name: "Show where it went",
        chord: opening.chord,
        run: shell => shell.launcher.into("spend")
    }]).concat(silent ? [{
        glyph: String.fromCodePoint(0xf040a),
        name: "Start the watch",
        run: () => Niri.spawn(["systemctl", "--user", "restart", "spend-watch.service"])
    }] : [])

    // Says the status the daemon wrote, or nothing when it wrote none that
    // reads or one of a version this doesn't know.
    function parsed(text) {
        try {
            const status = JSON.parse(text);
            return typeof status === "object" && status !== null && status.v === 1
                && typeof status.open === "object" && typeof status.quiet === "object"
                ? status : null;
        } catch (error) {
            return null;
        }
    }

    // The daemon writes its file anew, which a watch can miss, and may start
    // after the shell, so the file is read again every few seconds too.
    Timer {
        id: clock
        property real now: Date.now() / 1000
        interval: piece.settings.every ?? 5000
        running: true
        repeat: true
        onTriggered: {
            now = Date.now() / 1000;
            file.reload();
        }
    }

    FileView {
        id: file
        property var said: null
        path: piece.settings.status ?? `${piece.runtime}/spend-watch/status.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: said = piece.parsed(text())
        onLoadFailed: said = null
    }
}
