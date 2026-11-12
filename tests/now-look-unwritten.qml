// Drives a look Joel clears in For You (modules/quickshell/launcher/Now.qml)
// whose file can't be written, through Quickshell itself: the check makes
// the looks folder a file, so the write fails off the shell's thread. The
// look shows cleared at once, in the act's own frame, and is put back as
// it read once the write fails, rather than shown done until the next
// read of its file brings it back with nothing said (Y2YBTS, 2026-11-10).
// Shut, the launcher keeps the looks it read: its listing of files, given
// no folder, lists the shell's working one, which once dropped them all,
// so each opening parsed every look again.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // When the look came: an hour before the case runs, since a fixed date
    // grows three days old and goes quiet (Looks.js's QUIET_AFTER).
    readonly property string lookedAt: new Date(Date.now() - 60 * 60 * 1000).toISOString()

    // The launcher as Now reads it, shut, with no other source.
    QtObject {
        id: theLauncher

        property bool open: false
        property var held: []
        readonly property var sources: []

        function wants(source) {
            return true;
        }
        function resolved(id) {
            return null;
        }
    }

    Launcher.Now {
        id: now

        launcher: theLauncher
        settings: ({ name: "on" })
    }

    function waitingFiles() {
        return JSON.stringify(now.looks.waiting.map(one => one.file));
    }

    Component.onCompleted: Qt.callLater(root.clear)

    function clear() {
        if (tried(() => {
            now.lookTexts = { "a.json": JSON.stringify({ v: 1, id: "a", kind: "page",
                at: root.lookedAt, target: "https://example.com", title: "Read A",
                why: "Because.", state: "waiting", session: "" }) };
            Expect.expect("the look waits on Joel", waitingFiles() === "[\"a.json\"]", waitingFiles());
            const row = now.waiting.verbs().find(each => each.key
                === "on:waitingOnYou:kind:look")?.verbs()?.[0];
            const clearing = (row?.verbs?.() ?? []).find(each => each.key
                === "verb:on:look:clear:a");
            Expect.expect("its row offers to clear it", clearing !== undefined,
                JSON.stringify((row?.verbs?.() ?? []).map(each => each.key)));
            clearing.run();
            Expect.expect("cleared, it shows so in the act's own frame", waitingFiles() === "[]",
                waitingFiles());
        }))
            putBack.start();
    }

    // The write fails off the shell's thread, so its failure is waited on.
    Timer {
        id: putBack

        interval: 50
        repeat: true
        property int tries: 0

        onTriggered: {
            tries += 1;
            if (now.looks.waiting.length === 1 && Object.keys(now.lookWriters).length === 0) {
                stop();
                root.tried(() => Expect.expect("put back, it reads as it did",
                    now.looks.waiting[0].title === "Read A", root.waitingFiles()));
                Qt.exit(0);
            } else if (tries > 200) {
                stop();
                root.tried(() => Expect.expect("a write that failed put the look back", false,
                    `${root.waitingFiles()} with ${Object.keys(now.lookWriters)} writing`));
            }
        }
    }

    // Whether the cases held, exiting 1 saying which didn't.
    function tried(cases) {
        try {
            cases();
            return true;
        } catch (error) {
            Expect.fail(error.message ?? error);
            return false;
        }
    }
}
