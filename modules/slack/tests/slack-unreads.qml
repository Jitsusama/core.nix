// What in Slack wants Joel, as Now's Slack piece says it, run in Qt's own
// JavaScript engine, the one the shell runs it in: what Slack's daemon
// holds gathered in Slack's order whatever order it was heard in, a count
// shown only while the daemon answers and Slack answered it, why there's
// none when not, said on Slack's program rather than in Now, a channel
// merely unread kept for Slack's Unread but never counted in Now, Recent,
// and each conversation's words. The `slack-says-only-whats-current`
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
        const ada = { id: "D1", name: "Ada Lovelace", kind: "dm", count: 2,
            link: "slack://channel?team=T1&id=D1", latest: "1730000002.000100" };
        const grace = { id: "G1", name: "Grace and Ada", kind: "dm", count: 1,
            link: "slack://channel?team=T1&id=G1", latest: "1730000001.000100" };
        const deploys = { id: "C1", name: "#deploys", kind: "mention", count: 1,
            link: "slack://channel?team=T1&id=C1", latest: "1730000009.000100" };
        const threads = { id: "threads", name: "Threads", kind: "thread", count: 3,
            link: "slack://open?team=T1", latest: "" };
        const random = { id: "C9", name: "#random", kind: "unread", count: 0,
            link: "slack://channel?team=T1&id=C9", latest: "1730000003.000100" };
        const read = { id: "C8", name: "#launcher", kind: "channel",
            link: "slack://channel?team=T1&id=C8", read: "1730000005.000100" };
        const older = { id: "D1", name: "Ada Lovelace", kind: "dm",
            link: "slack://channel?team=T1&id=D1", read: "1730000004.000100" };
        // What the daemon holds, by key, as Daemon.qml holds it.
        const held = (standing, unread, recent) => {
            const all = {
                "slack-account:slack": { key: "slack-account:slack", kind: "slack-account",
                    heard: 1, "slack-account": { standing: standing } }
            };
            const add = (kind, one) => {
                all[`${kind}:${one?.id}`] = { key: `${kind}:${one?.id}`, kind: kind, heard: 1,
                    [kind]: one };
            };
            for (const one of unread ?? [])
                add("slack-unread", one);
            for (const one of recent ?? [])
                add("slack-recent", one);
            return all;
        };
        const told = (standing, unread, recent) => Unreads.gathered(
            held(standing, unread, recent), true);

        // Nothing gathered while the daemon isn't answering, whatever it
        // held, nor before it says how Slack stands.
        check("not answering", Unreads.gathered(held("ok", [ada]), false), null);
        check("nothing held", Unreads.gathered({}, true), null);
        check("no account yet", Unreads.gathered({ "slack-unread:D1": {
            key: "slack-unread:D1", kind: "slack-unread", "slack-unread": ada } }, true), null);
        // A conversation with nothing to read, or no link to open it by,
        // is no conversation to list.
        const kept = told("ok", [ada, Object.assign({}, deploys, { count: 0 }),
            { id: "C2", name: "#nolink", kind: "mention", count: 1 }]);
        check("what's kept", kept.conversations.map(one => one.id), ["D1"]);
        check("no recent is none", kept.recent, []);
        // Slack's order, whatever order it was heard in: direct messages,
        // mentions, threads, then channels merely unread, each newest
        // first; Recent newest read first.
        const all = told("ok", [random, threads, grace, deploys, ada], [older, read]);
        check("in Slack's order", all.conversations.map(one => one.id),
            ["D1", "G1", "C1", "threads", "C9"]);
        check("recent newest first", all.recent.map(one => one.id), ["C8", "D1"]);
        // Now says only what wants Joel, so a channel merely unread is
        // neither counted nor named there, nor lists Slack alone.
        check("wants", all.conversations.map(Unreads.wants), [true, true, true, true, false]);
        check("merely unread uncounted", Unreads.value(all, "ok"), "7 to read");
        check("merely unread unnamed in Now", Unreads.context(all, "ok"),
            "Ada Lovelace, Grace and Ada and 2 more");
        check("named in Unread", Unreads.context(all, "ok", true),
            "Ada Lovelace, Grace and Ada and 3 more");
        check("merely unread wants nothing", Unreads.shown(told("ok", [random]), "ok"), false);

        const ok = told("ok", [ada, deploys, threads]);
        const quiet = told("ok", []);
        const out = told("signedOut");
        const away = told("unreachable");
        const none = told("none");

        // Current only while the daemon answers and Slack answered it.
        check("not answering stands", Unreads.standing(null), "none");
        check("no sign-in stands", Unreads.standing(none), "none");
        check("answered", Unreads.standing(ok), "ok");
        check("unreachable", Unreads.standing(away), "unknown");
        check("signed out", Unreads.standing(out), "signedOut");

        // Listed only with something to read: never while Slack isn't
        // saying, which doesn't want Joel, nor with nothing, nor with the
        // daemon not answering (VKCF3Z, b8c8920e).
        check("listed", Unreads.shown(ok, "ok"), true);
        check("not listed quiet", Unreads.shown(quiet, "ok"), false);
        check("not listed unknown", Unreads.shown(away, "unknown"), false);
        check("not listed signed out", Unreads.shown(out, "signedOut"), false);
        check("not listed not answering", Unreads.shown(null, "none"), false);

        // An old count is never said as though it were Slack's.
        check("its count", Unreads.value(ok, "ok"), "6 to read");
        check("no count unknown", Unreads.value(away, "unknown"), "");
        check("its hint", Unreads.hint(ok, "ok"), "6 Slack messages want you");
        check("one's hint", Unreads.hint(told("ok", [deploys]), "ok"),
            "A Slack message wants you");
        check("no hint unknown", Unreads.hint(away, "unknown"), "");

        // Who and where, the first two named.
        check("one named", Unreads.context(told("ok", [ada]), "ok"), "Ada Lovelace");
        check("two named", Unreads.context(told("ok", [deploys, ada]), "ok"),
            "Ada Lovelace and #deploys");
        check("more", Unreads.context(ok, "ok"), "Ada Lovelace, #deploys and 1 more");
        check("no names unknown", Unreads.context(away, "unknown"), "");

        // Why there's no count, as Slack's program row says it; nothing
        // while there is one, nor with nothing to ask.
        check("unreachable says so", Unreads.programNote(Unreads.standing(away)),
            "Slack can't be reached");
        check("signed out says so", Unreads.programNote(Unreads.standing(out)), "signed out");
        check("a count says nothing", Unreads.programNote(Unreads.standing(ok)), "");
        check("not answering says nothing", Unreads.programNote(Unreads.standing(null)), "");
        check("no sign-in says nothing", Unreads.programNote(Unreads.standing(none)), "");

        // Each conversation's own words.
        check("beside", Unreads.beside(ada), "2 to read");
        check("a direct message", Unreads.note(ada), "a direct message");
        check("a mention", Unreads.note(deploys), "mentions you");
        check("threads", Unreads.note(threads), "replies in threads");
        check("nothing beside one uncounted", Unreads.beside(random), "");
        check("nothing beside one read", Unreads.beside(read), "");
        check("a channel merely unread", Unreads.note(random), "a channel");
        check("a channel read", Unreads.note(read), "a channel");

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
