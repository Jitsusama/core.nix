// Drives the orchestrators' talk readers (modules/quickshell/launcher/
// Orchestrators.qml) through Quickshell itself, over a steer folder of 120
// inboxes, two of them holding talks, with 120 sessions running and pi's
// sessions stood in. Only the two inboxes holding talks are read, and each
// talk is heard; the sessions reordered as focus moves 30 times, and the
// launcher opened, make no reader again, so the shell's threads stay where
// they were (each reader is a thread and a file watcher of Qt's own, and a
// reader per session ran the shell out of files on 2026-10-29). A talk
// made while the launcher was shut, in an inbox that held none, is read
// once it opens. It exits 0 once every case holds, and otherwise 1,
// saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string here: Quickshell.env("PWD")
    property string waiting: "the talks heard"
    property int threadsAtFirst: 0
    property int moved: 0

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // The shell's threads now, as the kernel counts them.
    function threads() {
        status.reload();
        status.waitForJob();
        const line = status.text().split("\n").find(each => each.startsWith("Threads:"));
        return Number(String(line ?? "").replace(/\D/g, ""));
    }

    FileView {
        id: status
        path: "/proc/self/status"
        blockLoading: true
    }

    QtObject {
        id: launcher

        property bool open: false
        function enter(row) {
        }
    }

    Launcher.Orchestrators {
        id: orchestrators

        launcher: launcher
        settings: ({ name: "orchestrators" })
        onTalksChanged: Qt.callLater(root.check)
        onTalkingChanged: Qt.callLater(root.check)
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function heard(address) {
        return orchestrators.talksOf(address).map(talk => talk.name).join(", ");
    }

    function check() {
        try {
            if (root.waiting === "the talks heard") {
                if (heard("A003") === "" || heard("A117") === "")
                    return;
                expect("only inboxes holding talks are read", orchestrators.talking.join(" ")
                    === "A003 A117", orchestrators.talking.join(" "));
                expect("a talk is heard", heard("A003") === "wrap up the release",
                    heard("A003"));
                root.threadsAtFirst = threads();
                root.waiting = "focus moved";
                focusing.start();
            } else if (root.waiting === "the talk made since" && heard("A050") !== "") {
                expect("a talk made since is heard", heard("A050") === "look at the disk",
                    heard("A050"));
                expect("the inbox holding it is read", orchestrators.talking.join(" ")
                    === "A003 A050 A117", orchestrators.talking.join(" "));
                console.log("PASS the talks are read from the inboxes holding them alone");
                Qt.callLater(() => Qt.exit(0));
            }
        } catch (error) {
            root.fail(error.message);
        }
    }

    // Focus moving reorders the sessions running, as Sessions.qml does.
    Timer {
        id: focusing
        interval: 10
        repeat: true
        onTriggered: {
            Sessions.running = Sessions.running.slice(1).concat(Sessions.running.slice(0, 1));
            root.moved += 1;
            if (root.moved < 30)
                return;
            stop();
            launcher.open = true;
            launcher.open = false;
            settle.start();
        }
    }

    // Long enough for any reader made again to have started its thread.
    Timer {
        id: settle
        interval: 500
        onTriggered: {
            try {
                const now = root.threads();
                root.expect("focus moving makes no reader again", now <= root.threadsAtFirst + 2,
                    `${root.threadsAtFirst} threads became ${now}`);
                root.expect("still only inboxes holding talks", orchestrators.talking.join(" ")
                    === "A003 A117", orchestrators.talking.join(" "));
                root.waiting = "the talk made since";
                making.running = true;
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    Process {
        id: making
        command: ["sh", "-c",
            'mkdir -p "$1/A050/replies" && printf "%s\\n" "$2" > "$1/A050/replies/t-3.jsonl"',
            "make", Sessions.inboxes,
            '{"stage":"delivered","at":"2026-10-29T10:20:00Z","text":"look at the disk"}']
        onExited: launcher.open = true
    }
}
