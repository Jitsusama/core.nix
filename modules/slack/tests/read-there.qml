// Slack's notice of a message Joel read in Slack is read in Notices once,
// after the change that told it, through SlackUnreads.qml itself, run in
// offscreen Quickshell beside a stand-in Daemon and Notices
// (tests/stand-ins/). The `slack-reads-a-notice-read-there-once` check
// lays them out as the shell has them and fails on any binding loop
// Quickshell logs: reading the notice inside readThere's own change
// rewrote the history readThere is found from, a loop Joel's shell logged
// on 2026-11-10. A change remakes only the parts whose keys it names: a
// message heard leaves the conversations and Recent the same lists, and a
// conversation gone leaves the messages.
import qs
import Quickshell
import QtQuick

ShellRoot {
    // A test that throws ends as a failure, rather than leaving qs running.
    Component.onCompleted: {
        try {
            start();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function start() {
        Notices.history = [{ id: "n1", app: "Slack", entry: "slack",
            summary: "[ada] from Ada Lovelace", body: "can you check the build", at: 1000300,
            read: false }];
        // Read before the daemon says so, there's nothing to read.
        if (SlackUnreads.readThere.length !== 0) {
            fail(`read with nothing read in Slack: ${JSON.stringify(SlackUnreads.readThere)}`);
            return;
        }
        SlackSays.tell({
            "slack-account:slack": { kind: "slack-account",
                "slack-account": { standing: "ok", messages: "whole" } },
            "slack-message:D1:1000.000100": { kind: "slack-message",
                "slack-message": { channel: "D1", ts: "1000.000100", place: "Ada Lovelace",
                    who: "Ada Lovelace", first: "can you check the build",
                    text: "can you check the build", at: 1000000,
                    done: { how: "readInSlack", at: "2024-10-31T18:40:00Z" } } },
            "slack-unread:D1": { kind: "slack-unread", "slack-unread": { id: "D1",
                name: "Ada Lovelace", kind: "dm", count: 1, link: "slack://channel?id=D1",
                latest: "1000.000100" } },
            "slack-recent:C8": { kind: "slack-recent", "slack-recent": { id: "C8",
                name: "#launcher", kind: "channel", link: "slack://channel?id=C8",
                read: "900.000100" } }
        });
        Qt.callLater(settled);
    }

    function settled() {
        const notice = Notices.history[0];
        if (notice.read !== true || notice.readHow !== "readInSlack") {
            fail(`the notice wasn't read in Slack: ${JSON.stringify(notice)}`);
        } else if (JSON.stringify(Notices.asked) !== JSON.stringify([["n1"]])) {
            fail(`read other than once: ${JSON.stringify(Notices.asked)}`);
        } else if (SlackUnreads.readThere.length !== 0) {
            fail(`still to read once read: ${JSON.stringify(SlackUnreads.readThere)}`);
        } else {
            console.log("read in Slack once");
            onlyThoseHeard();
        }
    }

    // Each part is the same list until a change names one of its keys.
    function onlyThoseHeard() {
        const unread = SlackUnreads.unread;
        const recent = SlackUnreads.recent;
        const messages = SlackUnreads.messages;
        SlackSays.tell({ "slack-message:D1:1001.000100": { kind: "slack-message",
            "slack-message": { channel: "D1", ts: "1001.000100", place: "Ada Lovelace",
                who: "Ada Lovelace", first: "and the tests", text: "and the tests",
                at: 1001000 } } });
        if (SlackUnreads.messages === messages || SlackUnreads.messages.length !== 2) {
            fail(`the message heard wasn't listed: ${JSON.stringify(SlackUnreads.messages)}`);
            return;
        }
        if (SlackUnreads.unread !== unread || SlackUnreads.recent !== recent) {
            fail("a message heard remade the conversations or Recent");
            return;
        }
        const listed = SlackUnreads.messages;
        SlackSays.tell({}, ["slack-unread:D1"]);
        if (SlackUnreads.unread.length !== 0) {
            fail(`the conversation gone is kept: ${JSON.stringify(SlackUnreads.unread)}`);
        } else if (SlackUnreads.messages !== listed || SlackUnreads.recent !== recent) {
            fail("a conversation gone remade the messages or Recent");
        } else {
            console.log("remade only what was heard");
            Qt.exit(0);
        }
    }

    function fail(why) {
        console.warn(`FAIL ${why}`);
        Qt.exit(1);
    }
}
