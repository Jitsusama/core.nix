// Drives the shell's client of a daemon (modules/quickshell/Daemon.qml)
// against the real people daemon (modules/people), with two sources stood
// in for, one asked (tests/daemon-source.sh) and one saying everyone
// unasked (tests/daemon-roster.sh), through Quickshell itself. With no
// daemon running the client says it isn't answering and gives a get none
// at once; once the daemon starts the client connects by itself and says
// hello, which names the source; a watch narrowed to those listed holds
// them alone; a search's answer is given to it and never held; one no
// source knows any more is let go; the keys
// it watches ahead come to be
// held, asked ahead; a get is answered by id, asked as a step-in; the
// daemon stopping is said, keeping what was held and giving a get none;
// and once it starts again, the client connects again by itself and asks
// ahead again. Each step waits on what the client says, at most ten
// seconds. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string ada: "person:ada@example.com"
    readonly property string grace: "person:grace@example.com"

    Daemon {
        id: people

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
    }

    // Watching only those a source lists with nothing typed.
    Daemon {
        id: listed

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        watching: ({ listed: true })
    }

    readonly property string eli: "person:eli@example.com"
    readonly property string hal: "person:hal@example.com"
    readonly property string ivy: "person:ivy@example.com"

    Process {
        id: running

        command: [Quickshell.env("PEOPLE"), `${root.runtime}/daemons/people.sock`,
            Quickshell.env("SOURCES")]
    }

    function fail(what) {
        console.warn(`FAIL ${what}: state ${people.state}, held ${JSON.stringify(people.held)}`
            + `, listed ${Object.keys(listed.held).join(" ")}`
            + `, answered ${JSON.stringify(root.answered)}`);
        Qt.exit(1);
    }

    function heldWay(key) {
        return people.held[key]?.sources?.slack?.way ?? "";
    }

    // The steps in order, each with what it does and what it waits for.
    property int step: 0
    property int left: 0
    property var answered: undefined
    property real heardBefore: 0

    readonly property var steps: [
        {
            what: "no daemon is said as not answering",
            holds: () => people.state === "not answering"
        },
        {
            what: "a get with no daemon is given none at once",
            run: () => people.get([root.ada], 1000, answer => {
                root.answered = answer;
            }),
            holds: () => root.answered === null
        },
        {
            what: "a daemon starting is connected to and says hello, naming its source",
            run: () => {
                running.running = true;
            },
            holds: () => people.state === "answering" && "slack" in people.sources
        },
        {
            what: "a watch narrowed to those listed holds them alone",
            holds: () => root.eli in listed.held && !(root.hal in listed.held)
                && root.hal in people.held && root.ivy in people.held
        },
        {
            what: "a search's answer is given to it and never held",
            run: () => {
                root.answered = undefined;
                listed.find({ ask: "search", kind: "person", words: "hal", limit: 5 },
                    answer => {
                        root.answered = answer;
                    });
            },
            holds: () => (root.answered?.records ?? []).some(record => record.key === root.hal)
                && !(root.hal in listed.held)
        },
        {
            // The roster lets Ivy go once it's asked anything, as a get asks.
            what: "one no source knows any more is let go",
            run: () => people.get([root.ivy], 1000, null),
            holds: () => !(root.ivy in people.held)
        },
        {
            what: "a key watched ahead comes to be held, asked ahead",
            run: () => {
                people.ahead = [root.ada];
            },
            holds: () => root.heldWay(root.ada) === "ahead"
        },
        {
            what: "a get is answered by its id, asked as a step-in",
            run: () => {
                root.answered = undefined;
                people.get([root.grace], 5000, answer => {
                    root.answered = answer;
                });
            },
            holds: () => root.answered?.records?.[0]?.key === root.grace
                && root.heldWay(root.grace) === "step"
        },
        {
            what: "the daemon stopping is said, and what was held stays",
            run: () => {
                root.heardBefore = people.held[root.ada].heard;
                running.running = false;
            },
            holds: () => people.state === "not answering" && root.heldWay(root.ada) === "ahead"
        },
        {
            what: "the daemon starting again is connected to, and asked ahead again",
            run: () => {
                running.running = true;
            },
            holds: () => people.state === "answering"
                && people.held[root.ada].heard > root.heardBefore
        }
    ]

    function begin() {
        const now = steps[step];
        left = 200;
        if (now.run)
            now.run();
        waiting.start();
    }

    Timer {
        id: waiting

        interval: 50
        repeat: true
        onTriggered: {
            const now = root.steps[root.step];
            if (now.holds()) {
                stop();
                root.step += 1;
                if (root.step === root.steps.length) {
                    running.running = false;
                    Qt.callLater(() => Qt.exit(0));
                } else {
                    root.begin();
                }
            } else {
                root.left -= 1;
                if (root.left === 0) {
                    stop();
                    root.fail(now.what);
                }
            }
        }
    }

    Component.onCompleted: begin()
}
