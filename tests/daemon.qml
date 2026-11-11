// Drives the shell's client of a daemon (modules/quickshell/Daemon.qml)
// against the real people daemon (modules/people), with two sources stood
// in for, one asked (daemon-source.sh) and one saying everyone
// unasked (daemon-roster.sh), through Quickshell itself. With no
// daemon running the client says it isn't answering and gives a get none
// at once; once the daemon starts the client connects by itself and says
// hello, which names the source; a watch narrowed to those listed holds
// them alone; one not live holds nothing until it's refreshed, and then
// holds its kind, watched no longer; one paced by a frame watches only
// once its frame is drawn, and one begun again before then is answered
// once it's drawn; a search's answer is given to it and never held; one no
// source knows any more is let go; the keys it watches ahead come to be
// held, asked ahead; a get is answered by id, asked as a step-in; the
// daemon stopping is said, keeping what was held and giving a get none,
// unless the client is one that lets what it held go in a gap; and once
// it starts again, the client connects again by itself, asks ahead
// again, and the one that let go holds again. A client of a daemon
// holding two kinds, as Google's does, stood in for
// (daemon-kinds.py), holds records of both over one connection,
// and names both in the watch ahead too, and its watch ahead emptied is
// unwatched, so the daemon hears the keys let go. A client of a daemon that says
// only what differs, as mycelium's node does, lays a change over the
// record it holds, letting go a field said as null. A client of a kind
// with no records knows it holds none once it's answered, and one whose
// answer is the whole of its kind, as the node's is, lets go what went in
// a gap. What each message changed and let go is heard apart, a key let
// go never said as changed, and a whole answer says only the keys whose
// records differ from what was held and those it dropped, so a rewatch
// answered with the same records hears nothing. A handler of a line reads
// when that line was heard, and one it puts off reads none. A client
// holding its wire watches neither its socket nor its folder. A socket that refuses is tried again twice as late
// each time, a minute at most, counted again from its next answer. Each
// step waits on what the client says, at most ten seconds.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
// Its stand-ins are in modules/quickshell/tests/stand-ins.
import Quickshell
import Quickshell.Io
import QtQuick
import "Hearing.js" as Hearing

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string ada: "person:ada@example.com"
    readonly property string grace: "person:grace@example.com"

    Daemon {
        id: people

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        onHeard: (changed, gone) => {
            root.peopleChanged = root.peopleChanged.concat(changed);
            root.peopleGone = root.peopleGone.concat(gone);
        }
    }
    property var peopleChanged: []
    property var peopleGone: []

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
        onHeard: (changed, gone) => {
            if (gone.includes("waiting:a"))
                root.wholeHeard = { changed: changed, gone: gone };
        }
    }
    property bool heldBoth: false
    property var wholeHeard: null

    // Watched again once it holds its one record, as each opening watches
    // again, and answered with the same record: the keys it hears after,
    // counted from the rewatch, or -1 before it.
    Daemon {
        id: rewatched

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "note"
        onHeard: (changed, gone) => {
            if (root.rewatchHeard >= 0)
                root.rewatchHeard += changed.length + gone.length;
        }
    }
    property int rewatchHeard: -1

    // What a handler of its records reads as the time its line was heard,
    // and what one it puts off reads, so the launcher times a change from
    // its line and never one told later (Hearing.js).
    Daemon {
        id: marked

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "mark"
        onHeard: (changed, gone) => {
            root.markedAt = Hearing.heardAt();
            root.markedThen = Date.now();
            Qt.callLater(() => root.markedLater = Hearing.heardAt());
        }
    }
    property real markedAt: -1
    property real markedThen: -1
    property real markedLater: -1

    // Not live, so watching only once it's refreshed, until answered.
    Daemon {
        id: refreshed

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        live: false
    }

    // What a paced watch waits on, as the launcher's next frame: drawn
    // only when a step says.
    QtObject {
        id: pacer

        property var queued: []

        function afterFrame(then) {
            queued = queued.concat([then]);
        }

        function drawn() {
            const due = queued;
            queued = [];
            for (const then of due)
                then();
        }
    }

    // Watching only once its frame is drawn.
    Daemon {
        id: paced

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        live: false
        paced: pacer
    }

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
        id: peopleRunning

        command: [Quickshell.env("PEOPLE"), `${root.runtime}/daemons/people.sock`,
            Quickshell.env("SOURCES")]
    }

    // The shell's inotify watches on the daemons' folder and its two
    // sockets, read from its own /proc as the shell's parent of the count,
    // since a watch held is a wake for every file written there.
    property int watches: -1
    Process {
        id: watchesCounted

        command: ["sh", "-c", `cd ${root.runtime}/daemons
            set -- $(stat -c %i . people.sock kinds.sock)
            grep -h '^inotify' /proc/$PPID/fdinfo/* \\
                | grep -c -w $(printf -- '-e ino:%x ' "$@")`]
        stdout: StdioCollector {
            onStreamFinished: root.watches = Number(text.trim())
        }
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
    // The stages an ask answered in steps was given, in order.
    property var stepsHeard: []

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
                peopleRunning.running = true;
            },
            holds: () => people.state === "answering" && "slack" in people.sources
        },
        {
            what: "a watch narrowed to those listed holds them alone",
            holds: () => root.eli in listed.held && !(root.hal in listed.held)
                && root.hal in people.held && root.ivy in people.held
                && root.hal in forgetting.held
                && Object.keys(refreshed.held).length === 0
        },
        {
            what: "one not live, refreshed, holds its kind and is watched no longer",
            run: () => refreshed.refresh(),
            holds: () => root.hal in refreshed.held && !refreshed.watched
        },
        {
            // A quarter second is time enough for the daemon to answer a
            // watch sent, so holding nothing then says none was sent.
            what: "one paced watches only once its frame is drawn",
            run: () => {
                paced.live = true;
            },
            holds: () => root.left <= 195 && pacer.queued.length === 1
                && Object.keys(paced.held).length === 0 && !paced.answeredThisWatch
        },
        {
            what: "a paced watch begun again before its frame is answered once it's drawn",
            run: () => {
                paced.live = false;
                paced.live = true;
                pacer.drawn();
            },
            holds: () => root.hal in paced.held && paced.answeredThisWatch
                && pacer.queued.length === 0
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
            what: "one no source knows any more is let go, heard as gone",
            run: () => {
                root.peopleChanged = [];
            },
            holds: () => !(root.ivy in people.held) && root.peopleGone.includes(root.ivy)
                && !root.peopleChanged.includes(root.ivy)
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
                peopleRunning.running = false;
            },
            holds: () => people.state === "not answering" && root.heldWay(root.ada) === "ahead"
                && forgetting.state === "not answering"
                && Object.keys(forgetting.held).length === 0
        },
        {
            what: "the daemon starting again is connected to, and asked ahead again",
            run: () => {
                peopleRunning.running = true;
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
            what: "its watch ahead emptied is unwatched before the next",
            run: () => {
                kinds.ahead = [];
                kinds.ahead = ["mail:three"];
            },
            holds: () => kinds.held["mail:three"]?.unwatched === 1
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
            what: "an ask answered in steps is given each, until one says it's done",
            run: () => {
                root.stepsHeard = [];
                kinds.ask({ ask: "steps" }, answer => {
                    root.stepsHeard = root.stepsHeard.concat([answer?.stage ?? "none"]);
                });
            },
            holds: () => root.stepsHeard.join(" ") === "joining gettingAddress online"
                && Object.keys(kinds.awaited).length === 0
        },
        {
            what: "a whole answer after a gap lets go what went in it, heard as gone",
            holds: () => root.heldBoth && whole.state === "answering"
                && "waiting:b" in whole.held && !("waiting:a" in whole.held)
                && JSON.stringify(root.wholeHeard)
                === '{"changed":[],"gone":["waiting:a"]}'
        },
        {
            what: "a rewatch answered with the same records hears nothing",
            holds: () => {
                if (!("note:one" in rewatched.held))
                    return false;
                if (root.rewatchHeard < 0) {
                    root.rewatchHeard = 0;
                    rewatched.live = false;
                    rewatched.live = true;
                    return false;
                }
                return rewatched.answeredThisWatch && root.rewatchHeard === 0;
            }
        },
        {
            what: "a handler of a line reads when it was heard, and one put off reads none",
            holds: () => root.markedAt > 0 && root.markedAt <= root.markedThen
                && root.markedThen - root.markedAt < 1000 && root.markedLater === 0
                && Hearing.heardAt() === 0
        },
        {
            what: "a client holding its wire watches neither its socket nor its folder",
            holds: () => {
                if (!watchesCounted.running)
                    watchesCounted.running = true;
                return root.watches === 0;
            }
        },
        {
            what: "a socket that refuses is tried twice as late each time, a minute at most",
            holds: () => typeof people.waitAfter === "function"
                && [0, 1, 2, 5, 9].map(n => people.waitAfter(n)).join(" ")
                    === "2000 4000 8000 60000 60000"
                && people.refusals === 0
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
                    peopleRunning.running = false;
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
