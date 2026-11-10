// What in Slack wants Joel, as Now's Slack piece says it, run in Qt's own
// JavaScript engine, the one the shell runs it in: what Slack's daemon
// holds gathered in Slack's order whatever order it was heard in, a
// change moving only the parts whose keys it names, a count
// shown only while the daemon answers and Slack answered it, why there's
// none when not, said on Slack's program rather than in Now, a channel
// merely unread kept for Slack's Unread but never counted in Now, Recent,
// each conversation's words, and which of Slack's own desktop notices
// were of a message read in Slack. The `slack-says-only-whats-current`
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
        // What the daemon holds, by key, as Daemon.qml holds it, from what
        // it `said`: { unread, recent, messages, whole }. `messages` are
        // held each by its conversation and Slack's time, and `whole` says
        // the daemon holds every one.
        const held = (standing, said) => {
            const all = {
                "slack-account:slack": { key: "slack-account:slack", kind: "slack-account",
                    heard: 1, "slack-account": Object.assign({ standing: standing },
                        said?.whole ? { messages: "whole" } : {}) }
            };
            for (const one of said?.messages ?? []) {
                const key = `slack-message:${one.channel}:${one.ts}`;
                all[key] = { key: key, kind: "slack-message", heard: 1, "slack-message": one };
            }
            const add = (kind, one) => {
                all[`${kind}:${one?.id}`] = { key: `${kind}:${one?.id}`, kind: kind, heard: 1,
                    [kind]: one };
            };
            for (const one of said?.unread ?? [])
                add("slack-unread", one);
            for (const one of said?.recent ?? [])
                add("slack-recent", one);
            return all;
        };
        const told = (standing, unread, recent) => Unreads.gathered(
            held(standing, { unread: unread, recent: recent }), true);

        // Nothing gathered while the daemon isn't answering, whatever it
        // held, nor before it says how Slack stands.
        check("not answering", Unreads.gathered(held("ok", { unread: [ada] }), false), null);
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

        // Each message wanting Joel, newest first whatever order it was
        // heard in, done ones too; one missing who said it or when is no
        // message to list.
        const first = { channel: "D1", ts: "1730000001.000100", place: "Ada Lovelace",
            who: "Ada Lovelace", first: "lunch?", text: "lunch?", at: 1730000001000,
            link: "https://slack.example/D1", team: "T1" };
        const second = Object.assign({}, first, { ts: "1730000002.000100", first: "or tea",
            at: 1730000002000 });
        const mention = { channel: "C1", ts: "1730000003.000100", place: "#deploys",
            who: "Grace Hopper", first: "@joel ship it", at: 1730000003000,
            done: { how: "readInSlack", at: 1730000004000 } };
        const nameless = { channel: "C1", ts: "1730000005.000100", at: 1730000005000 };
        const whole = Unreads.gathered(held("ok", { unread: [ada, deploys, threads],
            messages: [second, nameless, first, mention], whole: true }), true);
        check("messages newest first", whole.messages.map(one => one.ts),
            [mention.ts, second.ts, first.ts]);
        check("whole", whole.whole, true);
        check("held whole", Unreads.messages(whole, "ok").length, 3);
        check("done", Unreads.messages(whole, "ok").map(Unreads.done), [true, false, false]);
        // Only while Slack answers and the daemon holds every one do the
        // messages stand for the conversations; else the conversations
        // stand, as they did before the daemon held messages.
        check("held unknown", Unreads.messages(whole, "unknown"), null);
        const some = Unreads.gathered(held("ok", { unread: [ada], messages: [first] }), true);
        check("some held", some.whole, false);
        check("some stand for none", Unreads.messages(some, "ok"), null);
        check("none held", Unreads.gathered(held("ok", { unread: [ada] }), true).messages, []);
        // Threads, whose replies the daemon never holds, stay beside them.
        check("unheld", Unreads.unheld(whole).map(one => one.id), ["threads"]);
        // What's counted is what's listed: each message still to read and
        // Threads' replies, never the conversations the messages are in.
        check("whole counts what it lists", Unreads.value(whole, "ok"), "5 to read");
        check("some counts conversations", Unreads.value(Unreads.gathered(
            held("ok", { unread: [ada, deploys, threads], messages: [first] }), true), "ok"),
            "6 to read");
        // Slack's own Unread lists conversations, so it counts them, even
        // while For You lists each message.
        check("Unread counts its conversations", Unreads.conversationsCount(whole), 6);
        // Two said in one millisecond read newest first by Slack's time.
        const later = Object.assign({}, first, { ts: "1730000001.000200" });
        check("a tie newest first", Unreads.gathered(held("ok", { messages: [first, later],
            whole: true }), true).messages.map(one => one.ts), [later.ts, first.ts]);

        // A change remakes only the parts whose keys it names (Daemon.qml's
        // heard), each kind known by its key alone, so a key gone is too:
        // a message heard lists the messages alone, a conversation gone
        // the conversations alone, and a key of no part it lists nothing.
        const holds = held("ok", { unread: [ada, deploys], recent: [read], messages: [first],
            whole: true });
        const byKey = Unreads.keeping();
        const parts = moved => Object.keys(moved).sort();
        check("every part at first", parts(Unreads.keep(byKey, holds, Object.keys(holds))),
            ["account", "conversations", "messages", "recent"]);
        check("kept as gathered", Unreads.toldBy(byKey.account, Unreads.listed(byKey), true),
            Unreads.gathered(holds, true));
        const heard = `slack-message:${second.channel}:${second.ts}`;
        holds[heard] = { key: heard, kind: "slack-message", heard: 2, "slack-message": second };
        check("a message heard moves the messages alone",
            parts(Unreads.keep(byKey, holds, [heard])), ["messages"]);
        check("and is listed", Unreads.messagesOf(byKey).map(one => one.ts),
            [second.ts, first.ts]);
        delete holds["slack-unread:C1"];
        check("a conversation gone moves the conversations alone",
            parts(Unreads.keep(byKey, holds, ["slack-unread:C1"])), ["conversations"]);
        check("and is let go", Unreads.conversationsOf(byKey).map(one => one.id), ["D1"]);
        check("a key of no part moves nothing",
            parts(Unreads.keep(byKey, holds, ["slack-search:C1", "slack-account:other"])), []);
        check("the account's key is the account", byKey.account.standing, "ok");
        check("nothing told while not answering",
            Unreads.toldBy(byKey.account, Unreads.listed(byKey), false), null);
        check("the threads among those wanting him",
            Unreads.threads(whole.conversations).map(one => one.id), ["threads"]);

        // Slack's own notices of what was read in Slack are read too, each
        // known by its conversation, sender, time and words, one to one
        // among every message held, read or not.
        const readAt = { how: "readInSlack", at: "2024-10-31T18:40:00Z" };
        const said = (channel, place, who, text, at, done) => ({ channel: channel,
            ts: `${at / 1000}.000100`, place: place, who: who, first: text, text: text,
            at: at, done: done === undefined ? readAt : done });
        const raised = (id, summary, body, at, more) => Object.assign({ id: id, app: "Slack",
            entry: "slack", summary: summary, body: body, at: at, read: false }, more ?? {});
        const dm = said("D1", "Ada Lovelace", "Ada Lovelace",
            "Thank you! :heart: I'm so excited!", 1000000);
        const deploy = said("C1", "#deploys", "Grace", "the deploy is out, cc @joel", 1000000);
        const lunch = said("G1", "ada, grace", "Grace", "lunch at noon, then we ship", 1000000);
        const long = `the release notes ${"say more ".repeat(40)}`;
        const cut = said("D1", "Ada Lovelace", "Ada Lovelace", long.trim(), 2000000);
        const review = said("D1", "Ada Lovelace", "Ada Lovelace",
            "Thanks for the review, I will merge it after lunch today", 5000000);
        const build = said("D1", "Ada Lovelace", "Ada Lovelace", "can you check the build",
            6000000);
        const unread = said("D1", "Ada Lovelace", "Ada Lovelace", "are you there", 3000000,
            null);
        const gone = said("C2", "#old", "Grace", "this channel is going away", 3000000,
            { how: "goneFromSlack", at: "2024-10-31T18:40:00Z" });
        const okOnce = said("D1", "Ada Lovelace", "Ada Lovelace", "ok", 4000000);
        const okTwice = said("D1", "Ada Lovelace", "Ada Lovelace", "ok", 4060000, null);
        const thanks = said("D2", "Bo", "Bo", "ok \u8c22\u8c22", 7000000);
        const holding = [dm, deploy, lunch, cut, review, build, unread, gone, okOnce, okTwice,
            thanks];
        const readIn = notices => Unreads.readInSlack(notices, holding);
        check("a direct message's notice is read, its emoji shown rather than named",
            readIn([raised("n1", "[ada] from Ada Lovelace",
                "Thank you! \u2764\ufe0f I\u2019m so excited!", 1000300)]), ["n1"]);
        check("a channel's, by the sender its body opens with",
            readIn([raised("n2", "[ada] in #deploys", "Grace: the deploy is out, cc @joel",
                1000400)]), ["n2"]);
        check("a group's, named by its title where the daemon names its members",
            readIn([raised("n3", "[ada] in Lunch Crew", "Grace: lunch at noon, then we ship",
                1000200)]), ["n3"]);
        check("one the daemon cut short at a space, near its 400 characters",
            readIn([raised("n4", "[ada] from Ada Lovelace", `${long}and then it ends`,
                2000100)]), ["n4"]);
        check("one Slack cut short, its last word split",
            readIn([raised("n20", "[ada] from Ada Lovelace",
                "Thanks for the review, I will merge it after lun\u2026", 5000200)]), ["n20"]);
        check("but never one where neither could be cut",
            [readIn([raised("n21", "[ada] from Ada Lovelace", "can you check it", 6000200)]),
                readIn([raised("n22", "[ada] from Ada Lovelace",
                    "can you check the build logs too please", 6000300)])], [[], []]);
        check("words in a script without case are words",
            [readIn([raised("n23", "[ada] from Bo", "ok \u8c22\u8c22", 7000100)]),
                readIn([raised("n24", "[ada] from Bo", "ok \u4e0d\u884c", 7000100)])],
            [["n23"], []]);
        check("one still unread in Slack, or gone from it, isn't",
            readIn([raised("n5", "[ada] from Ada Lovelace", "are you there", 3000100),
                raised("n6", "[ada] in #old", "Grace: this channel is going away", 3000100)]),
            []);
        check("nor one a twin still unread could be",
            readIn([raised("n7", "[ada] from Ada Lovelace", "ok", 4060300)]), []);
        check("nor one of a message another notice, read already, is of",
            Unreads.readInSlack([raised("n17", "[ada] from Ada Lovelace", "ok", 4000300,
                { read: true }), raised("n18", "[ada] from Ada Lovelace", "ok", 4050000)],
                [okOnce]), []);
        // Joel read Kai's first "ok" in Slack, which raised no notice; the
        // notice of a second, raised after, isn't of it.
        const seen = [said("D4", "Kai", "Kai", "ok", 8000000,
            { how: "readInSlack", at: new Date(8000500).toISOString() })];
        check("nor one raised after its message was read",
            [Unreads.readInSlack([raised("n25", "[ada] from Kai", "ok", 8000300)], seen),
                Unreads.readInSlack([raised("n26", "[ada] from Kai", "ok", 8030000)], seen)],
            [["n25"], []]);
        check("nor one in a script whose letters aren't known",
            Unreads.readInSlack([raised("n27", "[ada] from Lin",
                "ok \u179f\u17bd\u179f\u17d2\u178f\u17b8", 9000100)],
                [said("D5", "Lin", "Lin", "ok \u1781\u17d2\u1798\u17c2\u179a", 9000000)]), []);
        check("nor a group's of a direct message whose sender isn't named yet",
            Unreads.readInSlack([raised("n19", "[ada] in Lunch Crew",
                "Grace: lunch at noon, then we ship", 1000200)],
                [said("D3", "someone", "Grace", "lunch at noon, then we ship", 1000000)]), []);
        check("nor one of two notices one message could be",
            readIn([raised("n8", "[ada] in #deploys", "Grace: the deploy is out, cc @joel",
                1000400), raised("n9", "[ada] in #deploys", "Grace: the deploy is out, cc @joel",
                1000900)]), []);
        check("nor one from someone else, elsewhere, or long after",
            readIn([raised("n10", "[ada] in #deploys", "Bo: the deploy is out, cc @joel",
                1000400), raised("n11", "[ada] in #builds", "Grace: the deploy is out, cc @joel",
                1000400), raised("n12", "[ada] in #deploys",
                "Grace: the deploy is out, cc @joel", 1000000 + 6 * 60 * 1000)]), []);
        check("nor one whose words differ, as where an emoji stands for one",
            readIn([raised("n13", "[ada] from Ada Lovelace",
                "Thank you! love I\u2019m so excited!", 1000300)]), []);
        check("nor one read already, nor another program's",
            readIn([raised("n14", "[ada] from Ada Lovelace",
                "Thank you! \u2764\ufe0f I\u2019m so excited!", 1000300, { read: true }),
                raised("n15", "[ada] from Ada Lovelace", "Thank you! I'm so excited!", 1000300,
                    { app: "Build", entry: "" })]), []);
        check("and none while the daemon doesn't hold every message",
            Unreads.readInSlack([raised("n16", "[ada] in #deploys",
                "Grace: the deploy is out, cc @joel", 1000400)], null), []);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
