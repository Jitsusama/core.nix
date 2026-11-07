// Drives the shell's client of a daemon (modules/quickshell/Daemon.qml)
// against the real people daemon (modules/people), with two sources stood
// in for, one asked (tests/daemon-source.sh) and one saying everyone
// unasked (tests/daemon-roster.sh), through Quickshell itself. With no
// daemon running the client says it isn't answering and gives a get none
// at once; once the daemon starts the client connects by itself and says
// hello, which names the source; a watch narrowed to those listed holds
// them alone; a search's answer is given to it and never held; one no
// source knows any more is let go; the keys it watches ahead come to be
// held, asked ahead; a get is answered by id, asked as a step-in; the
// daemon stopping is said, keeping what was held and giving a get none,
// unless the client is one that lets what it held go in a gap; and once
// it starts again, the client connects again by itself, asks ahead
// again, and the one that let go holds again. A client of a daemon
// holding two kinds, as Google's does, stood in for
// (tests/daemon-kinds.py), holds records of both over one connection,
// and names both in the watch ahead too. A client of a daemon that says
// only what differs, as mycelium's node does, lays a change over the
// record it holds, letting go a field said as null. A client of a kind
// with no records knows it holds none once it's answered, and one whose
// answer is the whole of its kind, as the node's is, lets go what went in
// a gap. Each step waits on what the client says, at most ten seconds.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
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

    // Letting what it held go while the daemon isn't answering.
    Daemon {
        id: forgetting

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        keptThroughGap: false
    }

    // Watching two kinds over one connection.
    Daemon {
        id: kinds

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: ["meeting", "mail"]
    }

    Process {
        id: kindsRunning

        command: [Quickshell.env("PYTHON"), Quickshell.env("KINDS"),
            `${root.runtime}/daemons/kinds.sock`]
    }

    // Watching sessions, told what differs, as mycelium's node tells it.
    Daemon {
        id: differing

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "session"
        changesDiffer: true
    }

    // Watching a kind the node answers whole, which loses one in a gap.
    Daemon {
        id: whole

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "waiting"
        changesDiffer: true
        answersWhole: true
        onRevisionChanged: {
            if ("waiting:a" in held && "waiting:b" in held)
                root.heldBoth = true;
        }
    }
    property bool heldBoth: false

    // Watching a kind with no records.
    Daemon {
        id: none

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "verdict"
    }

    // The kinds a record says its watch asked for, as one line.
    function askedOf(key) {
        return JSON.stringify(kinds.held[key]?.asked ?? null);
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
            + `, kinds ${JSON.stringify(kinds.held)}`
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
            holds: () => people.state === "not answering" && !none.answered
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
                && root.hal in forgetting.held
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
            what: "a key watched ahead comes to be held, asked ahead",
            run: () => {
                people.ahead = [root.ada];
            },
            holds: () => root.heldWay(root.ada) === "ahead"
        },
        {
            // The roster lets Ivy go once it's asked anything, as the ask
            // ahead for Ada asked it, and Slack's stand-in was never
            // asked of her.
            what: "one no source knows any more is let go",
            holds: () => !(root.ivy in people.held)
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
            what: "the daemon stopping is said, and what was held stays unless let go",
            run: () => {
                root.heardBefore = people.held[root.ada].heard;
                running.running = false;
            },
            holds: () => people.state === "not answering" && root.heldWay(root.ada) === "ahead"
                && forgetting.state === "not answering"
                && Object.keys(forgetting.held).length === 0
        },
        {
            what: "the daemon starting again is connected to, and asked ahead again",
            run: () => {
                running.running = true;
            },
            holds: () => people.state === "answering"
                && people.held[root.ada].heard > root.heardBefore
                && root.hal in forgetting.held
        },
        {
            what: "a client watching two kinds holds records of both",
            run: () => {
                kindsRunning.running = true;
            },
            holds: () => root.askedOf("meeting:one") === '["meeting","mail"]'
                && root.askedOf("mail:one") === '["meeting","mail"]'
        },
        {
            what: "its watch ahead names both kinds too",
            run: () => {
                kinds.ahead = ["mail:two"];
            },
            holds: () => root.askedOf("mail:two") === '["meeting","mail"]'
        },
        {
            what: "a change saying only what differs is laid over what's held",
            holds: () => differing.held["session:a"]?.state === "busy"
                && differing.held["session:a"]?.heard === 1
                && !("step" in differing.held["session:a"])
        },
        {
            what: "a kind with no records is known to hold none once answered",
            holds: () => none.answered && Object.keys(none.held).length === 0
        },
        {
            what: "a whole answer after a gap lets go what went in it",
            holds: () => root.heldBoth && whole.state === "answering"
                && "waiting:b" in whole.held && !("waiting:a" in whole.held)
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
                    kindsRunning.running = false;
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
