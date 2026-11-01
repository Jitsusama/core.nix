// What in Slack wants Joel, as Now's Slack piece says it, run in Qt's own
// JavaScript engine, the one the shell runs it in: a count shown only
// while Slack answered and the file is fresh, why there's none when it
// isn't, said on Slack's program rather than in Now, a channel merely
// unread kept for Slack's Unread but never counted in Now, Recent, and
// each conversation's words. The `slack-says-only-whats-current`
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
        const random = { id: "C9", name: "#random", kind: "unread", count: 0,
            link: "slack://channel?team=T1&id=C9" };
        const read = { id: "C8", name: "#launcher", kind: "channel", count: 0,
            link: "slack://channel?team=T1&id=C8" };
        const file = (state, conversations, recent) => JSON.stringify({
            checked: checked, state: state, conversations: conversations,
            recent: recent });

        // What isn't slack-unreads' file is no file at all.
        check("no file", Unreads.read(""), null);
        check("not JSON", Unreads.read("{"), null);
        check("no check time", Unreads.read("{\"state\": \"ok\"}"), null);
        // A conversation with nothing to read, or no link to open it by,
        // is no conversation to list.
        const kept = Unreads.read(file("ok", [ada, Object.assign({}, deploys, { count: 0 }),
            { id: "C2", name: "#nolink", kind: "mention", count: 1 }, null]));
        check("what's kept", kept.conversations.map(one => one.id), ["D1"]);
        check("no recent is none", kept.recent, []);
        // A channel merely unread has no count, and is kept all the same.
        const all = Unreads.read(file("ok", [ada, deploys, threads, random], [read, null]));
        check("merely unread kept", all.conversations.map(one => one.id),
            ["D1", "C1", "threads", "C9"]);
        check("recent kept", all.recent.map(one => one.id), ["C8"]);
        // Now says only what wants Joel, so a channel merely unread is
        // neither counted nor named there, nor lists Slack alone.
        check("wants", all.conversations.map(Unreads.wants), [true, true, true, false]);
        check("merely unread uncounted", Unreads.value(all, "ok"), "6 to read");
        check("merely unread unnamed in Now", Unreads.context(all, "ok"),
            "Ada Lovelace, #deploys and 1 more");
        check("named in Unread", Unreads.context(all, "ok", true),
            "Ada Lovelace, #deploys and 2 more");
        check("merely unread wants nothing",
            Unreads.shown(Unreads.read(file("ok", [random], [])), "ok"), false);

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

        // Listed only with something to read: never while Slack isn't
        // saying, which doesn't want Joel, nor with nothing, nor with no
        // file (VKCF3Z, b8c8920e).
        check("listed fresh", Unreads.shown(ok, "ok"), true);
        check("not listed quiet", Unreads.shown(quiet, "ok"), false);
        check("not listed unknown", Unreads.shown(ok, "unknown"), false);
        check("not listed signed out", Unreads.shown(out, "signedOut"), false);
        check("not listed without a file", Unreads.shown(null, "none"), false);

        // An old count is never said as though it were Slack's.
        check("its count", Unreads.value(ok, "ok"), "6 to read");
        check("no stale count", Unreads.value(ok, "unknown"), "");
        check("its hint", Unreads.hint(ok, "ok"), "6 Slack messages want you");
        check("one's hint", Unreads.hint(Unreads.read(file("ok", [deploys])), "ok"),
            "A Slack message wants you");
        check("no stale hint", Unreads.hint(ok, "unknown"), "");

        // Who and where, the first two named.
        check("one named", Unreads.context(Unreads.read(file("ok", [ada])), "ok"),
            "Ada Lovelace");
        check("two named", Unreads.context(Unreads.read(file("ok", [ada, deploys])), "ok"),
            "Ada Lovelace and #deploys");
        check("more", Unreads.context(ok, "ok"), "Ada Lovelace, #deploys and 1 more");
        check("no stale names", Unreads.context(ok, "unknown"), "");

        // Why there's no count, as Slack's program row says it; nothing
        // while there is one, or no file.
        check("stale says since", Unreads.programNote(ok, "unknown"),
            "can't be checked since 05:07");
        check("unreachable says so", Unreads.programNote(away, "unknown"),
            "Slack can't be reached");
        check("signed out says so", Unreads.programNote(out, "signedOut"), "signed out");
        check("a count says nothing", Unreads.programNote(ok, "ok"), "");
        check("no file says nothing", Unreads.programNote(null, "none"), "");

        // Each conversation's own words.
        check("beside", Unreads.beside(ada), "2 to read");
        check("a direct message", Unreads.note(ada), "a direct message");
        check("a mention", Unreads.note(deploys), "mentions you");
        check("threads", Unreads.note(threads), "replies in threads");
        check("nothing beside one uncounted", Unreads.beside(random), "");
        check("a channel merely unread", Unreads.note(random), "a channel");
        check("a channel read", Unreads.note(read), "a channel");

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
