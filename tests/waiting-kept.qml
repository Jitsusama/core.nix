// For You weighs what waits on Joel again only when what the node says of
// it changes, through Quickshell itself, with WaitingOnYou.qml as the shell
// has it and the node stood in for. The node tells each kind again whole
// whenever a file of it is written, every record's `heard` moved and
// nothing else, and each time For You weighed again, with every binding
// below it: about 600 ms of the shell's main thread with the launcher shut
// (2026-11-10). So a write that says nothing new weighs nothing again, and
// a quest that ends still takes its item with it. The
// `for-you-weighs-again-only-for-news` check runs it, with stand-ins for
// the rest of the shell.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect

ShellRoot {
    id: root

    // How many times For You weighed what waits, after the first reading.
    property int weighed: 0
    property bool counting: false
    property string waiting: "the first reading"
    // The writes the node makes again, each saying nothing new: of the
    // quests, the verdicts and the items, in turn.
    property int written: 0
    readonly property var writes: ["quest", "quest", "verdict", "waiting", "quest"]

    Connections {
        target: WaitingOnYou
        function onWeighedChanged() {
            if (root.counting)
                root.weighed += 1;
        }
        function onHeardAllChanged() {
            if (WaitingOnYou.heardAll && root.waiting === "the first reading")
                Qt.callLater(root.step, root.first);
        }
    }

    // Each kind's watch, so the node can tell it again, by kind.
    property var watches: ({})

    function item(id, quest) {
        return { key: `waiting:${quest}:${id}`, kind: "waiting", heard: 1, id: id,
            title: `Look at ${id}`, quest: quest, ready: false, raisedAt: "2026-11-10T06:00:00Z" };
    }
    function quest(id, status) {
        return { key: `quest:${id}`, kind: "quest", heard: 1, id: id, title: id, status: status };
    }
    readonly property var records: ({
        waiting: [item("alpha", "QEST-20261101-AAAAAA"), item("beta", "QEST-20261102-BBBBBB")],
        verdict: [{ key: "verdict:QEST-20261101-AAAAAA:alpha", kind: "verdict", heard: 1,
            state: "waiting", at: "2026-11-10T06:00:00Z" }],
        quest: [quest("QEST-20261101-AAAAAA", "active"), quest("QEST-20261102-BBBBBB", "active")]
    })

    // A kind told again, each record heard anew and otherwise as it was,
    // and with `changing` laid over the record it keys.
    function tell(kind, heard, changing) {
        const changed = records[kind].map(one => Object.assign({}, one, { heard: heard },
            changing?.key === one.key ? changing : {}));
        const watch = watches[kind];
        watch.socket.write(`${JSON.stringify({ id: watch.id, changed: changed, gone: [] })}\n`);
        watch.socket.flush();
    }

    function kindOf(kind) {
        return kind === "quest" ? WaitingOnYou.questKind
            : kind === "verdict" ? WaitingOnYou.verdictKind : WaitingOnYou.waitingKind;
    }

    // A case's step, failing by what it found wrong.
    function step(done) {
        try {
            done();
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    function first() {
        Expect.same("both items wait, later", WaitingOnYou.later.map(one => one.id), ["alpha",
            "beta"]);
        Expect.same("no item holds when the node heard it", WaitingOnYou.later
            .concat(Object.values(WaitingOnYou.verdicts.items)).some(one => "heard" in one), false);
        waiting = "the writes that say nothing new";
        counting = true;
        again();
    }

    // Each write is told once the one before it was taken, by its kind's
    // revision moving.
    property int revisionWas: 0
    function again() {
        if (written === writes.length) {
            Expect.same("a write that says nothing new weighs nothing again", weighed, 0);
            waiting = "a quest ending";
            revisionWas = WaitingOnYou.questKind.revision;
            tell("quest", 100, { key: "quest:QEST-20261102-BBBBBB", status: "concluded" });
            return;
        }
        const kind = writes[written];
        revisionWas = kindOf(kind).revision;
        written += 1;
        tell(kind, 1 + written);
    }

    Timer {
        interval: 20
        repeat: true
        running: root.counting
        onTriggered: {
            if (root.waiting === "the writes that say nothing new"
                && root.kindOf(root.writes[root.written - 1]).revision !== root.revisionWas)
                root.step(root.again);
            else if (root.waiting === "a quest ending"
                && WaitingOnYou.questKind.revision !== root.revisionWas)
                root.step(root.ended);
        }
    }

    function ended() {
        waiting = "";
        Expect.same("an ending weighs again, once", weighed, 1);
        Expect.same("the ended quest's item goes", WaitingOnYou.later.map(one => one.id),
            ["alpha"]);
        console.info("every case holds");
        Qt.callLater(() => Qt.exit(0));
    }

    // A stand-in for the machine's node, speaking the daemon interface:
    // hello, and a watch of each kind For You reads, answered whole.
    SocketServer {
        active: true
        path: Sessions.nodePath
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    } else if (ask.ask === "watch" && ask.kind in root.records) {
                        root.watches[ask.kind] = { socket: asked, id: ask.id };
                        answer = { id: ask.id, records: root.records[ask.kind] };
                    } else {
                        Expect.fail(`an ask For You has no need of: ${line}`);
                        answer = { id: ask.id, error: { code: "unknown-ask", message: line } };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
    }
}
