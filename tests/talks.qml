pragma ComponentBehavior: Bound
// Drives the talks Agents shows (modules/quickshell/launcher/
// Orchestrators.qml) through Quickshell itself, over a stand-in for the
// realm's node that serves each talk as one record of its lines, as
// mycelium's does (its PROTOCOL.md, The Daemon Interface). Each talk is
// heard by its session's address, in either case, newest first; a line
// its session writes, told as only what differs, moves it on to its
// reply; one made since is heard; and one the node lets go goes. The
// quest index written while the launcher is shut is read only as it
// opens, though the orchestrators' file beside it is read at once, and
// that opening orders the closed ones by what it reads. It exits
// 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the talks heard"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: theLauncher

        property bool open: false
        property bool cataloguing: false
        function enter(row) {
        }
    }

    Launcher.Orchestrators {
        id: orchestrators

        launcher: theLauncher
        settings: ({ name: "orchestrators" })
        onTalksChanged: Qt.callLater(root.check)
        onListedChanged: Qt.callLater(root.check)
        onStartFoldersChanged: Qt.callLater(root.check)
    }

    // The harness's files beside the shell, written as the harness does.
    FileView {
        id: questsFile
        path: `${Sessions.state}/quest-workflow/quests.json`
        printErrors: false
    }
    FileView {
        id: orchestratorsFile
        path: `${Sessions.state}/quest-workflow/orchestrators.json`
        printErrors: false
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

    // A talk as the node serves it: its address, its id and its lines.
    function talk(address, id, lines) {
        return { key: `talk:${address}/${id}`, kind: "talk", heard: Date.now(),
            address: address, id: id, lines: lines };
    }
    function delivered(at, words) {
        return { stage: "delivered", at: `2026-10-29T10:${at}:00Z`, text: words };
    }

    function check() {
        try {
            if (root.waiting === "the talks heard") {
                if (heard("A003") === "" || heard("A117") === "")
                    return;
                expect("each talk is heard by its session's address, newest first",
                    heard("a003") === "check the lights, wrap up the release", heard("a003"));
                expect("an inbox's talks are its own", heard("A117") === "look at the disk",
                    heard("A117"));
                expect("a talk delivered says so", orchestrators.talksOf("A117")[0].side
                    === orchestrators.stageSaid({ stage: "delivered", after: "" }),
                    orchestrators.talksOf("A117")[0].side);
                root.waiting = "the reply heard";
                tell({ changed: [{ key: "talk:A117/t-3", lines: [delivered("20", "look at the disk"),
                    { stage: "replied", text: "It has room." }] }] });
            } else if (root.waiting === "the reply heard") {
                const told = orchestrators.talksOf("A117")[0];
                if (told?.reads !== "It has room.")
                    return;
                expect("a line its session writes moves it on", told.side
                    === orchestrators.stageSaid({ stage: "replied", after: "" }), told.side);
                root.waiting = "the talk made since";
                tell({ changed: [talk("A050", "t-4", [delivered("30", "keep the lamp lit")])] });
            } else if (root.waiting === "the talk made since") {
                if (heard("A050") === "")
                    return;
                expect("a talk made since is heard", heard("A050") === "keep the lamp lit",
                    heard("A050"));
                root.waiting = "the talk let go";
                tell({ gone: ["talk:A003/t-1"] });
            } else if (root.waiting === "the talk let go") {
                if (heard("A003") !== "check the lights")
                    return;
                root.waiting = "the orchestrators read while shut";
                questsFile.setText(JSON.stringify({ quests: [
                    { id: "QEST-A", start: "/lamp", resume: { at: "2026-11-01T09:00:00Z" } },
                    { id: "QEST-B", resume: { at: "2026-11-10T09:00:00Z" } }] }));
                orchestratorsFile.setText(JSON.stringify({ orchestrators: [
                    { address: "A003", quest: "QEST-A", title: "A lit" },
                    { address: "B001", quest: "QEST-B", title: "B lit" }] }));
            } else if (root.waiting === "the orchestrators read while shut") {
                if (orchestrators.listed[0]?.title !== "A lit")
                    return;
                expect("the quest index written while shut is left unread",
                    orchestrators.startFolders["QEST-A"] === undefined,
                    JSON.stringify(orchestrators.startFolders));
                root.waiting = "the quest index read as it opens";
                theLauncher.open = true;
            } else if (root.waiting === "the quest index read as it opens") {
                if (orchestrators.startFolders["QEST-A"] !== "/lamp")
                    return;
                // Its first opening read the index only as it opened, so
                // the closed ones were ordered before what it says of them
                // came (vm-desktop-looks, 2026-11-11: Beacon then Lamplight).
                expect("the opening that reads the index orders the closed by it",
                    JSON.stringify(orchestrators.closedOrder) === '["B001","A003"]',
                    JSON.stringify(orchestrators.closedOrder));
                console.log("PASS the talks are heard from the node");
                Qt.callLater(() => Qt.exit(0));
            }
        } catch (error) {
            root.fail(error.message);
        }
    }

    // The node's end of the shell's watch, so the node can tell it more.
    property var answering: null
    property var watchId: null

    // Tells the shell's watch a change, as the node does.
    function tell(change) {
        answering.write(`${JSON.stringify(Object.assign({ id: root.watchId }, change))}\n`);
        answering.flush();
    }

    // A stand-in for the machine's node, speaking the daemon interface:
    // hello, and a watch of talks answered with each whole, then each
    // change saying only what differs.
    SocketServer {
        active: true
        path: Sessions.nodePath
        handler: Socket {
            id: asked
            Component.onCompleted: root.answering = asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    } else if (ask.ask === "watch" && ask.kind === "talk") {
                        root.watchId = ask.id;
                        answer = { id: ask.id, records: [
                            root.talk("A003", "t-1", [root.delivered("00", "wrap up the release")]),
                            root.talk("A003", "t-2", [root.delivered("10", "check the lights")]),
                            root.talk("A117", "t-3", [root.delivered("20", "look at the disk")])
                        ] };
                    } else if (ask.ask === "watch"
                        && (ask.kind === "session" || ask.kind === "run")) {
                        // pi's source watches its sessions and runs too,
                        // which this node holds none of.
                        answer = { id: ask.id, records: [] };
                    } else {
                        root.fail(`an ask the talks have no need of: ${line}`);
                        answer = { id: ask.id, error: { code: "unknown-ask", message: line } };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }
}
