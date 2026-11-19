// Waits in a QML test for what a daemon it runs has yet to do: `until`
// checks `holds` every 50 ms, ten seconds at most, then goes on to `then`,
// or fails saying `what`; `then` failing fails too, and either exits 1.
// A case may set `saw` to say what it found as a wait ran out, since
// `what` alone names the wait but not which part of it never held.
// A test copies it beside itself and holds one, as spend-served.qml and
// asks-served.qml in modules/quickshell/tests/ do.
import QtQuick

Timer {
    id: waits

    property string what: ""
    property var holds: null
    property var then: null
    property var saw: null
    property int left: 0

    // What the case saw, said after the wait's name; a report that throws
    // says so rather than hiding which wait ran out.
    function told() {
        if (!saw)
            return "";
        try {
            return `: ${saw()}`;
        } catch (error) {
            return `: (its report threw ${error.message})`;
        }
    }

    function until(what, holds, then) {
        waits.what = what;
        waits.holds = holds;
        waits.then = then;
        waits.left = 200;
        waits.start();
    }

    interval: 50
    repeat: true
    onTriggered: {
        try {
            if (holds()) {
                stop();
                then();
            } else if (--left === 0) {
                stop();
                throw new Error(`${what} never held${waits.told()}`);
            }
        } catch (error) {
            stop();
            // Where it threw, since a case's message alone rarely says.
            console.warn(`FAIL ${error.message}\n${error.stack ?? ""}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}
