pragma Singleton
pragma ComponentBehavior: Bound
// Joel's timers as the shell holds them: read from the timers daemon on
// $XDG_RUNTIME_DIR/daemons/timers.sock through Daemon.qml, one connection
// watching every timer, and changed by asking it. The daemon keeps them and
// rings each as a notice itself (timers/src/main.rs), so nothing here has
// to be running for a timer to ring; the shell only shows them and asks.
// Each ask's answer is held at once, so a row says the new state the
// moment the daemon has it, before the watch tells of it too. The one
// stopwatch is kept there too, a record of its own kind, so it counts on
// through a restart of the shell.
import Quickshell
import QtQuick
import "launcher/Timing.js" as Timing

Singleton {
    id: root

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/timers.sock`
        kind: ["timer", "stopwatch"]
        // The daemon's watch is every timer it keeps, so one held that it
        // no longer says went while the shell wasn't hearing.
        answersWhole: true
    }

    // Every timer, rung first, then by what each has left, made again only
    // as the daemon says one changed.
    readonly property var timers: daemon.revision >= 0
        ? Timing.ordered(Object.values(daemon.held).filter(record => record.kind === "timer"),
            Date.now()) : []
    // The stopwatch as the daemon says it, or null while it isn't there.
    readonly property var stopwatch: daemon.revision >= 0 ? daemon.held.stopwatch ?? null : null
    // Whether the launcher is open, where the time left is drawn, so Now's
    // row is told it only then (shell.qml binds it, as it does Jobs').
    property bool showing: false
    // Whether the daemon is there to ask.
    readonly property bool answering: daemon.wire !== null
    // Said once a change, with the keys it touched, as Daemon.qml says it.
    signal heard(var keys)

    Connections {
        target: daemon

        function onHeard(keys) {
            root.heard(keys);
        }
    }

    // A timer by its key, or null.
    function timer(key) {
        return daemon.held[key] ?? null;
    }

    // Asks the daemon, `then` given the timer as it now stands, null when
    // it's gone, or the daemon's refusal as { error } or null when it
    // isn't there to ask.
    function asking(said, then) {
        daemon.ask(said, answer => {
            if (!then)
                return;
            if (answer === null || answer.error)
                then(answer === null ? null : { error: answer.error.message ?? "" });
            else
                then(answer.records?.[0] ?? null);
        });
    }

    // Sets one going for `span` milliseconds, called `name`.
    function start(span, name, then) {
        asking({ ask: "start", span: span, name: name }, then);
    }

    function pause(key, then) {
        asking({ ask: "pause", key: key }, then);
    }

    function resume(key, then) {
        asking({ ask: "resume", key: key }, then);
    }

    // Sets it going afresh for its whole span, however it stands.
    function again(key, then) {
        asking({ ask: "again", key: key }, then);
    }

    // Gives it `span` more to count, or a rung one that long again.
    function add(key, span, then) {
        asking({ ask: "add", key: key, span: span }, then);
    }

    // Takes it away, its notice with it.
    function end(key, then) {
        asking({ ask: "end", key: key }, then);
    }

    // The stopwatch set counting on from what it has, stopped holding its
    // count, its count now marked as a lap, and set back to nothing.
    function run(then) {
        asking({ ask: "run" }, then);
    }

    function stop(then) {
        asking({ ask: "stop" }, then);
    }

    function lap(then) {
        asking({ ask: "lap" }, then);
    }

    function reset(then) {
        asking({ ask: "reset" }, then);
    }
}
