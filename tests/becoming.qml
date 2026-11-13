// How the launcher draws what an act's rows will be before the daemon it
// asked has said so, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `an-act-shows-before-its-daemon-answers` check copies
// modules/quickshell/launcher/Becoming.js beside this file and runs
// `qml -platform offscreen becoming.qml`. Each case is a promise behind
// Joel's "it should show right away": the row changes on the key's frame,
// holds until its source agrees, and nothing else moves.
import QtQuick
import "Becoming.js" as Becoming

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function reads(one, row) {
        return `${row.key}:${row.side}`;
    }

    function read(rows) {
        return rows.map(row => reads(null, row) + (row.becoming ? "*" : "")).join(" ");
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    function run() {
        const failures = [];
        const lantern = { key: "lantern", side: "connected", prepared: "lantern's" };
        const office = { key: "office", side: "saved" };
        const rows = [lantern, office];
        const left = { lantern: { key: "lantern", row: { key: "lantern", side: "saved" }, at: 0 } };

        const laid = Becoming.laid(rows, left, reads);
        check(failures, "a held row is drawn as it will be, marked so", read(laid.rows),
            "lantern:saved* office:saved");
        check(failures, "and keeps what the launcher made of it", laid.rows[0].prepared,
            "lantern's");
        check(failures, "a row nobody holds is the row itself", laid.rows[1], office);
        check(failures, "nothing's agreed while its source still says otherwise",
            laid.agreed.length, 0);

        const now = Becoming.laid([{ key: "lantern", side: "saved" }, office], left, reads);
        check(failures, "a row reading as it was to become is shown as it is", read(now.rows),
            "lantern:saved office:saved");
        check(failures, "and its hold is agreed", now.agreed.join(" "), "lantern");

        const stopping = { lantern: { key: "lantern", row: null, at: 0 } };
        check(failures, "a row held to go is left out",
            read(Becoming.laid(rows, stopping, reads).rows), "office:saved");

        const one = left.lantern;
        check(failures, "its source's row agrees as it reads the same",
            Becoming.agrees(one, { key: "lantern", side: "saved" }, reads), true);
        check(failures, "and not while it reads as it did", Becoming.agrees(one, lantern, reads),
            false);
        check(failures, "a row made no more never agrees to one to be shown",
            Becoming.agrees(one, null, reads), false);
        check(failures, "a row to go agrees once it's gone",
            Becoming.agrees(stopping.lantern, null, reads), true);
        check(failures, "and not while it's there", Becoming.agrees(stopping.lantern, lantern,
            reads), false);

        const holds = {
            a: { key: "a", row: null, at: 1000 },
            b: { key: "b", row: null, at: 3000 }
        };
        check(failures, "a hold comes due once its time is up",
            Becoming.late(holds, 6000, 5000).join(" "), "a");
        check(failures, "and none before", Becoming.late(holds, 5999, 5000).length, 0);
        check(failures, "the next due is the soonest", Becoming.nextDue(holds, 2000, 5000), 4000);
        check(failures, "one past due is due now", Becoming.nextDue(holds, 9000, 5000), 0);
        check(failures, "none held is none due", Becoming.nextDue({}, 0, 5000), -1);
        check(failures, "a hold let go is gone from them",
            Object.keys(Becoming.without(holds, ["a"])).join(" "), "b");
        check(failures, "and the rest are left as they were", Object.keys(holds).length, 2);

        // An act whose outcome its source can't foretell says how it knows it's
        // done, as a session woken comes back listening or at work, and waits
        // as long as it says, as a pi takes to start.
        const waking = { key: "lantern", row: { key: "lantern", side: "listening" }, at: 0,
            done: row => row !== null && row.side !== "parked", within: 30000 };
        const parked = { key: "lantern", side: "parked" };
        check(failures, "one held till it's done is drawn as it will be till then",
            read(Becoming.laid([parked], { lantern: waking }, reads).rows), "lantern:listening*");
        const working = Becoming.laid([{ key: "lantern", side: "working" }],
            { lantern: waking }, reads);
        check(failures, "and is agreed however it comes back done, as it is",
            `${read(working.rows)} ${working.agreed.join(" ")}`, "lantern:working lantern");
        check(failures, "its source agrees by what it says is done",
            `${Becoming.agrees(waking, parked, reads)} `
                + `${Becoming.agrees(waking, { key: "lantern", side: "working" }, reads)}`,
            "false true");
        const slow = { a: Object.assign({}, holds.a, { within: 30000 }), b: holds.b };
        check(failures, "a hold that waits longer comes due when it says",
            `${Becoming.late(slow, 8000, 5000).join(" ")}|${Becoming.late(slow, 31000, 5000)}`,
            "b|a,b");
        check(failures, "and the next due counts it", Becoming.nextDue(slow, 2000, 5000), 6000);

        const told = Becoming.told({ mark: "x", what: "Home-5G not disconnected",
            why: "not connected" }, { what: "alert", why: "muted" });
        check(failures, "the foot reads what wasn't done, then why", JSON.stringify(told),
            JSON.stringify([["not\u00a0connected", "", "muted"],
                ["x", "Home-5G\u00a0not\u00a0disconnected", "alert", "alert"]]));

        const did = { off: "turned off", run: "run" };
        check(failures, "an act is said by its own word where there's one",
            Becoming.did(did, ["off", "turn off"], "done"), "turned off");
        check(failures, "else by the foot's verb", Becoming.did(did, ["speedtest", "run"], "done"),
            "run");
        check(failures, "a protocol's word is never said",
            Becoming.did(did, ["sqm", "optimize for conferencing and gaming"], "done"), "done");
        check(failures, "one act alone", Becoming.did(did, "off", "done"), "turned off");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
