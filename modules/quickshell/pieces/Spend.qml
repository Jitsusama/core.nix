// What AI use costs, as spend-watch (modules/spend-watch) says it: today's
// spend against usual in the overview, and in the margin while something
// wants a look, a cap near or reached, a burn, an override ending. The
// daemon works every figure out and writes each string ready to draw, so
// this reads its status file and formats nothing, and asks the daemon's
// socket only to do an action it offers. While the
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
    readonly property var said: status.said
    // The daemon writes its heartbeat at least every 30 seconds while it
    // runs.
    readonly property bool silent: status.silent
    // The chord that opens the collection, beside the action that does.
    readonly property var opening: Shortcuts.actions
        .find(action => action.opens === "spend") ?? { chord: [] }
    readonly property var shown: said === null ? null : Quiet.sharing ? said.quiet : said.open
    // Each action's mark, by the kind the daemon gives it: going to a
    // session wears its window's, as the agents' piece does, and marking a
    // finding expected all's fine, as the launcher's Spend does. The daemon
    // asks for anything it doesn't name.
    readonly property var marks: ({
        go: "windows",
        ask: "ask",
        "sign-in": "signIn",
        report: "report",
        mark: "fine"
    })

    present: said !== null
    active: said !== null && (said.active === true || silent)
    role: silent ? "warning" : said?.role ?? "text"
    wants: said !== null && !silent && said.wants === true
    value: silent ? "not watching" : shown?.value ?? ""
    title: said?.title ?? "AI Spend"
    hint: silent ? "Nothing is watching AI spend" : shown?.hint ?? value
    context: silent ? "the spend-watch unit says why" : shown?.context ?? ""
    // The Spend row it stands for draws its drawer, as the daemon names it:
    // the top finding's, else today's, the quiet one's while the screen is
    // shared. None while nothing watches.
    drawnBy: !silent && shown?.row ? { place: "spend", key: `spend:${shown.row}` } : null
    // What the watch offers is asked of its socket by the action's key,
    // which spawns nothing, and is offered only while the socket answers,
    // so there's no second way of acting (Y2YBTS, 2026-11-10).
    actions: (silent || SpendWatch.fromFile ? [] : (said?.actions ?? []).map(action => ({
        glyph: Theme.glyph[marks[action.kind] ?? "ask"],
        name: action.name,
        run: () => SpendWatch.act("run", action.key, () => {})
    }))).concat([{
        glyph: Theme.glyph.trend,
        name: "Show where it went",
        verb: "show",
        chord: opening.chord,
        run: shell => shell.launcher.into("spend")
    }]).concat(silent ? [{
        glyph: Theme.glyph.resume,
        name: "Start the watch",
        run: () => machine.ask({ ask: "act", act: "restart-unit", unit: "spend-watch.service" },
            () => {})
    }] : [])

    // The machine daemon, asked only to start the watch again, since the
    // watch isn't there to ask; nothing of it is watched.
    Daemon {
        id: machine

        socket: `${piece.runtime}/daemons/machine.sock`
        kind: "unit"
        live: false
    }

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

    StatusFile {
        id: status
        path: piece.settings.status ?? `${piece.runtime}/spend-watch/status.json`
        silence: piece.settings.silence ?? 120
        parse: text => piece.parsed(text)
    }
}
