// Waits in a QML test for what a daemon it runs has yet to do: `until`
// checks `holds` every 50 ms, ten seconds at most, then goes on to `then`,
// or fails saying `what`; `then` failing fails too, and either exits 1.
// A test copies it beside itself and holds one, as tests/spend-served.qml
// and tests/asks-served.qml do.
import QtQuick

Timer {
    id: waits

    property string what: ""
    property var holds: null
    property var then: null
    property int left: 0

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
                throw new Error(`${what} never held`);
            }
        } catch (error) {
            stop();
            // Where it threw, since a case's message alone rarely says.
            console.warn(`FAIL ${error.message}\n${error.stack ?? ""}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}
