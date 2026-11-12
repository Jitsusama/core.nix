// What Now's Work Network piece says ails the link to work, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `now-says-what-ails-the-work-network` check copies pieces/WorkNetwork.js
// and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen work-network-ails.qml` over `warp:state`
// records shaped as the warp daemon tells them (modules/warp/src/serve.rs),
// in words.json's on:work-network words.
import QtQuick
import "pieces/WorkNetwork.js" as WorkNetwork
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    readonly property var said: ({
        down: "down",
        downSince: "down since {time}",
        degraded: "degraded",
        connecting: "connecting since {time}",
        notAnswering: "not answering"
    })

    // 21:50, as a clock reads it, and the time the cases are read at.
    readonly property real began: 1794451800000
    readonly property real now: began + 60000

    function clock(at) {
        return at === began ? "21:50" : `${at}`;
    }

    // A state as the daemon tells one: said, and when it began, if seen.
    function state(said, since) {
        return { key: "warp:state", kind: "warp-state", read_at_ms: began, since_ms: since,
            said: said };
    }

    function ails(said, since, at) {
        return WorkNetwork.ails(state(said, since), this.said, at ?? now, clock);
    }

    function run() {
        Expect.same("nothing told says nothing", WorkNetwork.ails(null, said, now, clock), []);
        Expect.same("a healthy link says nothing",
            ails({ status: "Connected", reason: "NetworkHealthy" }, null), []);
        Expect.same("a link Joel took down is his choice, not an ail",
            ails({ status: "Disconnected", reason: "ManualDisconnection" }, began), []);
        Expect.same("as a status asked says it", ails({ status: "Disconnected", reason: "Manual" },
            null), []);
        Expect.same("a link that dropped says since when",
            ails({ status: "Disconnected", reason: "NoNetwork" }, began), ["down since 21:50"]);
        Expect.same("one first heard down says no since, which may be hours before",
            ails({ status: "Disconnected", reason: "NoNetwork" }, null), ["down"]);
        Expect.same("a degraded link says so",
            ails({ status: "Connected", reason: "NetworkDegraded" }, began), ["degraded"]);
        Expect.same("connecting for a moment is how a link comes up",
            ails({ status: "Connecting", reason: "" }, began, began + 10000), []);
        Expect.same("connecting for long says since when",
            ails({ status: "Connecting", reason: "" }, began), ["connecting since 21:50"]);
        Expect.same("connecting since first heard is timed from then",
            ails({ status: "Connecting", reason: "" }, null), ["connecting since 21:50"]);
        Expect.same("WARP's service down says it isn't answering",
            ails({ error: "FailedToConnectToDaemon" }, began), ["not answering"]);
        Expect.same("any other error says nothing certain of the link",
            ails({ error: "NothingWritten" }, began), []);
        Expect.same("nor does a warp-cli gone missing", ails({ error: "missing" }, began), []);
        Expect.same("a link connecting is timed, since how long says whether it ails",
            WorkNetwork.timed(state({ status: "Connecting", reason: "" }, null)), true);
        Expect.same("a link that's up needs no clock",
            WorkNetwork.timed(state({ status: "Connected", reason: "NetworkHealthy" }, null)),
            false);
    }
}
