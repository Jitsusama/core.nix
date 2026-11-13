// Drives looks Joel clears in For You (modules/quickshell/launcher/Now.qml)
// through Quickshell itself, with the to-do daemon (modules/to-do) stood in
// on its socket: it streams two looks, refuses to settle the first and
// settles the second. Each shows cleared at once, in the act's own frame.
// The one refused is put back as it read, rather than shown done until a
// later reading brings it back with nothing said (Y2YBTS, 2026-11-10); the
// one settled stays cleared once the stream tells it so, with nothing of
// Joel's act left laid over it.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // When the looks came: an hour before the case runs, since a fixed date
    // grows three days old and goes quiet (Looks.js's QUIET_AFTER).
    readonly property string lookedAt: new Date(Date.now() - 60 * 60 * 1000).toISOString()

    function look(id, title, state) {
        return { key: `look:${id}`, kind: "look", look: { v: 1, id: id, kind: "page",
            at: root.lookedAt, target: "https://example.com", title: title,
            why: "Because.", state: state, session: "" } };
    }

    // The settle-look asks the stand-in was sent, by look.
    property var settles: []

    // The to-do daemon as the shell hears it: a hello, a watch of its
    // looks answered whole, and settle-look, refused for "a" and done for
    // any other, which it then tells its watch as changed.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/to-do.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answers = [{ id: ask.id, ok: true }];
                    if (ask.ask === "hello") {
                        answers = [{ id: ask.id, version: 1, domain: "to-do",
                            state: "running" }];
                    } else if (ask.ask === "watch") {
                        answers = [{ id: ask.id, records: [root.look("a", "Read A", "waiting"),
                            root.look("b", "Read B", "waiting")] }];
                        root.watch = ask.id;
                    } else if (ask.ask === "settle-look") {
                        root.settles = root.settles.concat([ask.look]);
                        answers = ask.look === "a"
                            ? [{ id: ask.id, error: { code: "refused",
                                message: "the looks folder is read-only" } }]
                            : [{ id: ask.id, done: true }, { id: root.watch,
                                changed: [root.look(ask.look, "Read B", ask.state)] }];
                    }
                    for (const answer of answers)
                        asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }
    property string watch: "all"

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

    // Clears the look of `id` through its row's own act.
    function clear(id) {
        const row = now.waiting.verbs().find(each => each.key
            === "on:waitingOnYou:kind:look")?.verbs()?.find(each => each.key === `on:look:${id}`);
        const clearing = (row?.verbs?.() ?? []).find(each => each.key
            === `verb:on:look:clear:${id}`);
        Expect.expect(`look ${id}'s row offers to clear it`, clearing !== undefined,
            JSON.stringify((row?.verbs?.() ?? []).map(each => each.key)));
        clearing.run();
    }

    property int step: 0
    readonly property var steps: [
        {
            what: "the looks come from the daemon's stream, both waiting on Joel",
            holds: () => root.waitingFiles() === "[\"a.json\",\"b.json\"]"
        },
        {
            what: "cleared, the first shows so in the act's own frame",
            run: () => {
                root.clear("a");
                Expect.expect("cleared, it shows so in the act's own frame",
                    root.waitingFiles() === "[\"b.json\"]", root.waitingFiles());
            },
            holds: () => root.settles.includes("a")
        },
        {
            what: "refused, it's put back as it read",
            holds: () => root.waitingFiles() === "[\"a.json\",\"b.json\"]"
                && now.looks.waiting[0].title === "Read A"
                && Object.keys(now.looksSettling).length === 0
        },
        {
            what: "the second, cleared, shows so in the act's own frame",
            run: () => {
                root.clear("b");
                Expect.expect("the second shows cleared in the act's own frame",
                    root.waitingFiles() === "[\"a.json\"]", root.waitingFiles());
            },
            holds: () => root.settles.includes("b")
        },
        {
            what: "settled and told so, it stays cleared with nothing of the act laid over it",
            holds: () => root.waitingFiles() === "[\"a.json\"]"
                && Object.keys(now.looksSettling).length === 0
                && JSON.parse(now.lookTexts["b.json"] ?? "{}").state === "cleared"
        }
    ]

    // Each step polled for ten seconds before it's said not to hold.
    Timer {
        id: polling

        interval: 50
        repeat: true
        property int left: 200

        onTriggered: {
            const here = root.steps[root.step];
            if (root.tried(here.holds) === true) {
                root.step += 1;
                if (root.step === root.steps.length) {
                    stop();
                    Qt.exit(0);
                    return;
                }
                left = 200;
                root.tried(() => root.steps[root.step].run?.());
            } else if (--left === 0) {
                stop();
                root.tried(() => Expect.expect(here.what, false, `${root.waitingFiles()}, settling`
                    + ` ${JSON.stringify(Object.keys(now.looksSettling))}, asked`
                    + ` ${JSON.stringify(root.settles)}`));
            }
        }
    }

    Component.onCompleted: polling.start()

    // What the cases gave, or false, failing the check, when one threw.
    function tried(cases) {
        try {
            return cases();
        } catch (error) {
            Expect.fail(error.message ?? error);
            polling.stop();
            return false;
        }
    }
}
