// What in Slack wants Joel, as Now's Slack piece says it, run in Qt's own
// JavaScript engine, the one the shell runs it in: a count shown only
// while Slack answered and the file is fresh, why there's none when it
// isn't, and each conversation's words. The `slack-says-only-whats-current`
// check copies SlackUnreads.js beside this file and runs
// `qml -platform offscreen slack-unreads.qml`.
import QtQuick
import "SlackUnreads.js" as Unreads

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

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        // 05:07 local time, so the stale note's time reads the same in any
        // zone the check runs in.
        const checked = new Date(2026, 9, 31, 5, 7).getTime();
        const minute = 60 * 1000;
        const ada = { id: "D1", name: "Ada Lovelace", kind: "dm", count: 2,
            link: "slack://channel?team=T1&id=D1" };
        const deploys = { id: "C1", name: "#deploys", kind: "mention", count: 1,
            link: "slack://channel?team=T1&id=C1" };
        const threads = { id: "threads", name: "Threads", kind: "thread", count: 3,
            link: "slack://open?team=T1" };
        const file = (state, conversations) => JSON.stringify({
            checked: checked, state: state, conversations: conversations });

        // What isn't slack-unreads' file is no file at all.
        check("no file", Unreads.read(""), null);
        check("not JSON", Unreads.read("{"), null);
        check("no check time", Unreads.read("{\"state\": \"ok\"}"), null);
        // A conversation with nothing to read, or no link to open it by,
        // is no conversation to list.
        const kept = Unreads.read(file("ok", [ada, Object.assign({}, deploys, { count: 0 }),
            { id: "C2", name: "#nolink", kind: "mention", count: 1 }, null]));
        check("what's kept", kept.conversations.map(one => one.id), ["D1"]);

        const ok = Unreads.read(file("ok", [ada, deploys, threads]));
        const quiet = Unreads.read(file("ok", []));
        const out = Unreads.read(file("signedOut", []));
        const away = Unreads.read(file("unreachable", []));

        // Current only while Slack answered and the file is under ten
        // minutes old.
        check("no file stands", Unreads.standing(null, checked), "none");
        check("fresh", Unreads.standing(ok, checked + 9 * minute), "ok");
        check("stale", Unreads.standing(ok, checked + 11 * minute), "unknown");
        check("unreachable", Unreads.standing(away, checked), "unknown");
        check("signed out", Unreads.standing(out, checked), "signedOut");
        check("goes stale at", Unreads.staleAt(ok), checked + 10 * minute);
        check("an unreachable one never goes stale", Unreads.staleAt(away), null);
        check("no file never goes stale", Unreads.staleAt(null), null);

        // Listed with something to read, or when Slack isn't saying; not
        // with nothing, nor with no file.
        check("listed fresh", Unreads.shown(ok, "ok"), true);
        check("not listed quiet", Unreads.shown(quiet, "ok"), false);
        check("listed unknown", Unreads.shown(ok, "unknown"), true);
        check("listed signed out", Unreads.shown(out, "signedOut"), true);
        check("not listed without a file", Unreads.shown(null, "none"), false);

        // An old count is never said as though it were Slack's.
        check("its count", Unreads.value(ok, "ok"), "6 to read");
        check("no stale count", Unreads.value(ok, "unknown"), "can't check");
        check("no count signed out", Unreads.value(out, "signedOut"), "signed out");
        check("its hint", Unreads.hint(ok, "ok"), "6 Slack messages want you");
        check("one's hint", Unreads.hint(Unreads.read(file("ok", [deploys])), "ok"),
            "A Slack message wants you");
        check("a stale hint", Unreads.hint(ok, "unknown"), "Slack: can't check");

        // Who and where, the first two named; or why there's no count.
        check("one named", Unreads.context(Unreads.read(file("ok", [ada])), "ok"),
            "Ada Lovelace");
        check("two named", Unreads.context(Unreads.read(file("ok", [ada, deploys])), "ok"),
            "Ada Lovelace and #deploys");
        check("more", Unreads.context(ok, "ok"), "Ada Lovelace, #deploys and 1 more");
        check("stale says since", Unreads.context(ok, "unknown"), "not checked since 05:07");
        check("unreachable says so", Unreads.context(away, "unknown"),
            "Slack can't be reached");
        check("signed out says so", Unreads.context(out, "signedOut"),
            "pi's sign-in to Slack was refused");

        // Each conversation's own words.
        check("beside", Unreads.beside(ada), "2 to read");
        check("a direct message", Unreads.note(ada), "a direct message");
        check("a mention", Unreads.note(deploys), "mentions you");
        check("threads", Unreads.note(threads), "replies in threads");

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
