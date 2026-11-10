// Slack's notice of a message Joel read in Slack is read in Notices once,
// after the change that told it, through SlackUnreads.qml itself, run in
// offscreen Quickshell beside a stand-in Daemon and Notices
// (tests/stand-ins/). The `slack-reads-a-notice-read-there-once` check
// lays them out as the shell has them and fails on any binding loop
// Quickshell logs: reading the notice inside readThere's own change
// rewrote the history readThere is found from, a loop Joel's shell logged
// on 2026-11-10.
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
        SlackSays.held = {
            "slack-account:slack": { kind: "slack-account",
                "slack-account": { standing: "ok", messages: "whole" } },
            "slack-message:D1:1000.000100": { kind: "slack-message",
                "slack-message": { channel: "D1", ts: "1000.000100", place: "Ada Lovelace",
                    who: "Ada Lovelace", first: "can you check the build",
                    text: "can you check the build", at: 1000000,
                    done: { how: "readInSlack", at: "2024-10-31T18:40:00Z" } } }
        };
        SlackSays.revision += 1;
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
            Qt.exit(0);
        }
    }

    function fail(why) {
        console.warn(`FAIL ${why}`);
        Qt.exit(1);
    }
}
