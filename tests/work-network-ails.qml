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
        degraded: "degraded for {lasted}",
        degradedOnAndOff: "degraded on and off for {lasted}",
        evidence: "WARP measures {rtt} and {loss} lost on the way to its edge",
        evidenceNone: "WARP gives no figures for it",
        hour: "{minutes} of the last hour, in {times} spells",
        hourOnce: "{minutes} of the last hour, in one spell",
        causeAgents: "Agents fill it: nothing for you to do.",
        causeUpload: "Your upload is {full} full: nothing for you to do.",
        causeBeyond: "It's past your home: nothing for you to do; it recovers by itself.",
        connecting: "connecting since {time}",
        notAnswering: "not answering",
        slow: "slow \u00b7 {waiting} waiting"
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
        Expect.same("WARP saying degraded for a moment says nothing until the daemon judges it held",
            ails({ status: "Connected", reason: "NetworkDegraded" }, began), []);
        const episode = (said, degraded) => Object.assign(state(said, null), { degraded });
        const held = { since_ms: began - 6 * 60000, now: true, times: 1,
            hour: { times: 1, minutes: 6 }, rtt_ms: 92, loss_percent: 5.4, cause: "beyond",
            upload: { full_pct: 40, shares: null } };
        const degraded = episode({ status: "Connected", reason: "NetworkDegraded" }, held);
        Expect.same("an episode the daemon judges held says for how long",
            WorkNetwork.ails(degraded, said, began, clock), ["degraded for 6 min"]);
        Expect.same("and says WARP's evidence, the hour, and that there's nothing to do",
            WorkNetwork.context(degraded, said),
            "WARP measures 92 ms and 5.4% lost on the way to its edge; 6 min of the last hour, "
            + "in one spell. It's past your home: nothing for you to do; it recovers by itself.");
        const flapping = episode({ status: "Connected", reason: "NetworkHealthy" },
            Object.assign({}, held, { since_ms: began - 75 * 60000, now: false, times: 9,
                hour: { times: 9, minutes: 40 },
                cause: "agents", upload: { full_pct: 95, shares: { bulk: 75 } } }));
        Expect.same("one that flips is said on and off, through its healthy gaps",
            WorkNetwork.ails(flapping, said, began, clock), ["degraded on and off for 1 h 15 min"]);
        Expect.same("spells past one are counted",
            WorkNetwork.context(flapping, said).includes("of the last hour, in 9 spells."), true);
        Expect.same("agents filling the upload are named",
            WorkNetwork.context(flapping, said).endsWith("Agents fill it: nothing for you to do."),
            true);
        const full = episode({ status: "Connected", reason: "NetworkDegraded" },
            Object.assign({}, held, { rtt_ms: null, cause: "upload",
                upload: { full_pct: 90, shares: null } }));
        Expect.same("a full upload says how full, and no figures says none",
            WorkNetwork.context(full, said), "WARP gives no figures for it; 6 min of the last "
            + "hour, in one spell. Your upload is 90% full: nothing for you to do.");
        Expect.same("an episode is timed, since its minutes are said",
            WorkNetwork.timed(degraded), true);
        Expect.same("no episode has no context", WorkNetwork.context(state(
            { status: "Connected", reason: "NetworkDegraded" }, null), said), "");
        const slow = Object.assign(state({ status: "Connected", reason: "NetworkHealthy" },
            null), { slow: { waiting_ms: 120 } });
        Expect.same("a way the daemon judges slow says what waits",
            WorkNetwork.ails(slow, said, now, clock), ["slow \u00b7 120 ms waiting"]);
        const slowAndDegraded = Object.assign(state({ status: "Connected",
            reason: "NetworkDegraded" }, null), { slow: { waiting_ms: 120 } });
        Expect.same("a moment's degraded leaves a slow way said",
            WorkNetwork.ails(slowAndDegraded, said, now, clock), ["slow \u00b7 120 ms waiting"]);
        Expect.same("a held episode says more than slow", WorkNetwork.ails(
            Object.assign({}, slowAndDegraded, { degraded: held }), said, began, clock),
            ["degraded for 6 min"]);
        Expect.same("a way not slow says nothing", WorkNetwork.ails(Object.assign(
            state({ status: "Connected", reason: "NetworkHealthy" }, null), { slow: null }),
            said, now, clock), []);
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
