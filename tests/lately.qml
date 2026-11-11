// Lately's sources and what's done, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `lately-follows-each-source` check copies
// modules/quickshell/launcher/Lately.js and Order.js beside this file and
// runs `qml -platform offscreen lately.qml`. Each case is a promise
// Lately makes: what's still unread, from which source, and how what's
// done went, each as its source said it.
import QtQuick
import "Lately.js" as Lately

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

    function check(failures, what, got, want) {
        const said = JSON.stringify(got);
        if (said !== JSON.stringify(want))
            failures.push(`${what}: got ${said}, wanted ${JSON.stringify(want)}`);
    }

    function notice(summary, fields) {
        return Object.assign({ app: "Build", entry: "", summary: summary, read: false,
            at: 1000 }, fields ?? {});
    }

    function message(who, fields) {
        return Object.assign({ channel: "D1", ts: "1.0", who: who, first: "hi" }, fields ?? {});
    }

    readonly property var say: ({
        mail: { readInGmail: "read in Gmail" },
        slack: {},
        notice: { read: "read", readInSlack: "read in Slack", withdrawn: "taken back",
            earlier: "read once" }
    })

    function summaries(rows) {
        return rows.map(row => row.summary ?? row.who ?? row.subject);
    }

    function run() {
        const failures = [];

        const slackNotice = notice("from Ada", { app: "Slack", entry: "slack" });
        const asked = notice("Sign in to Slack", { app: "waiting" });
        const wants = notice("core.nix asks", { app: "pi" });
        const build = notice("Build finished");
        const seen = notice("Deploy log", { read: true, at: 2000 });
        const notices = [slackNotice, asked, wants, build, seen];

        const held = Lately.held(notices);
        check(failures, "neither a Needs You item's notice nor a session's want is the desktop's",
            summaries(held.desktop), ["Build finished"]);
        check(failures, "Slack's unread notices are Slack's, for its row while its piece can't",
            summaries(held.slack), ["from Ada"]);
        check(failures, "while Slack's daemon answers, even with nothing unread, they're none",
            summaries(Lately.held(notices, ["slack"]).slack), []);
        check(failures, "Slack's answering takes nothing from the desktop's",
            summaries(Lately.held(notices, ["slack"]).desktop), ["Build finished"]);

        const opened = 5000;
        const readSince = notice("Read just now", { read: true, readAt: 6000 });
        const readBefore = notice("Read before", { read: true, readAt: 4000 });
        const withdrawnSince = notice("Arrived at Home", { read: true, readAt: 6000,
            withdrawn: true });
        const still = [readSince, readBefore, withdrawnSince];
        check(failures, "one read since the launcher opened is unread till it closes",
            summaries(Lately.held(Lately.holdingStill(still, opened)).desktop), ["Read just now"]);
        check(failures, "one withdrawn since it opened goes at once, as Joel never read it",
            Lately.holdingStill(still, opened)[2].read, true);
        check(failures, "with the launcher closed, nothing read is held",
            summaries(Lately.held(Lately.holdingStill(still, Number.POSITIVE_INFINITY))
                .desktop), []);

        const unread = message("Ada");
        const read = message("Grace", { done: { how: "read in Slack", at: 5000 } });
        const mail = [{ id: "m1", subject: "Lunch", at: 3000 }];
        const alike = () => 1;
        check(failures, "a source with nothing is left out, those alike in the sources' order",
            Lately.bands({ mail: mail, slack: [], desktop: [build] }, alike)
                .map(row => [row.source, row.count]), [["mail", 1], ["desktop", 1]]);
        check(failures, "nothing unread is no rows",
            Lately.bands(Object.assign({ mail: [] }, Lately.held([seen, asked])), alike), []);
        const heavy = notice("Disk nearly full", { heavy: true });
        const weigh = thing => thing.heavy === true ? 9 : 1;
        check(failures, "rows stand by the heaviest each holds",
            Lately.bands({ mail: mail, slack: [unread], desktop: [build, heavy] }, weigh)
                .map(row => row.source), ["desktop", "mail", "slack"]);

        check(failures, "a row's members are named newest first, each once",
            Lately.members([message("Ada"), message("Bo"), message("Ada"), message("")],
                one => one.who), ["Ada", "Bo"]);

        const taken = notice("Call ended", { read: true, left: "withdrawn", leftAt: 4000 });
        const stale = notice("Arrived at Home", { read: true, withdrawn: true, readAt: 4500,
            left: "dismissed" });
        const finished = [{ id: "m2", subject: "Invoice",
            done: { how: "readInGmail", at: 6000 } }];
        const history = Lately.done({ notices: [slackNotice, seen, stale, taken, asked],
            messages: [unread, read], finished: finished }, say);
        check(failures, "what's done is newest first across sources, each how its source says",
            history.map(one => [one.source, one.title, one.how, one.at]), [
                ["mail", "Invoice", "read in Gmail", 6000],
                ["slack", "Grace", "read in Slack", 5000],
                ["desktop", "Arrived at Home", "taken back", 4500],
                ["desktop", "Call ended", "taken back", 4000],
                ["desktop", "Deploy log", "read once", 2000]
            ]);

        const readSlack = notice("from Lin", { app: "Slack", read: true, at: 7000 });
        check(failures, "Slack's read notices are its history only while its daemon is away",
            [Lately.done({ notices: [readSlack], messages: [] }, say).length,
                Lately.done({ notices: [readSlack], messages: null }, say)
                    .map(one => [one.source, one.title])],
            [0, [["slack", "from Lin"]]]);

        const readThere = notice("from Lin", { app: "Slack", read: true, readAt: 7500,
            readHow: "readInSlack" });
        check(failures, "one read in Slack says so, not that it was read here",
            Lately.done({ notices: [readThere], messages: null }, say)
                .map(one => [one.how, one.at]), [["read in Slack", 7500]]);
        check(failures, "and goes at once, as Joel didn't read it here",
            Lately.holdingStill([readThere], 7000).map(one => one.read), [true]);

        const readLate = notice("Backup done", { read: true, at: 1000, readAt: 8000 });
        check(failures, "a notice read is done when it was read, not when it came",
            Lately.done({ notices: [readLate], messages: null }, say).map(one => one.at), [8000]);

        check(failures, "an ISO time reads as one",
            Lately.done({ messages: [message("Kai", { done: { how: "gone",
                at: "1970-01-01T00:00:01.000Z" } })] }, say).map(one => [one.how, one.at]),
            [["gone", 1000]]);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
        } else {
            console.log(`all held`);
            Qt.exit(0);
        }
    }
}
