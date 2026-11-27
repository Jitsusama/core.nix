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
// once it's drawn; one whose pace is lifted, as the launcher opens, is
// asked again once its frame is drawn; a search's answer is given to it and never held; one no
// source knows any more is let go; the keys it watches ahead come to be
// held, asked ahead; a get is answered by id, asked as a step-in; one
// the shell lets go with an ask in flight goes, its ask given none, and
// says so as let go, read from the log; the daemon stopping is said,
// keeping what was held and giving a get none,
// unless the client is one that lets what it held go in a gap; and once
// it starts again, the client connects again by itself, asks ahead
// again, and the one that let go holds again. A client of a daemon
// holding two kinds, as Google's does, stood in for
// (daemon-kinds.py), holds records of both over one connection,
// and names both in the watch ahead too, and its watch ahead emptied is
// unwatched, so the daemon hears the keys let go. A client of a daemon that says
// only what differs, as mycelium's node does, lays a change over the
// record it holds, letting go a field said as null, and one it asked deep,
// as hello offers, mended with only what moved inside it, while one asked
// nothing deep is replaced whole. A client of a kind
// with no records knows it holds none once it's answered, and one whose
// answer is the whole of its kind, as the node's is, lets go what went in
// a gap, and one whose answer is whole only of some records lets go only
// those. What each message changed and let go is heard apart, a key let
// go never said as changed, and a whole answer says only the keys whose
// records differ from what was held and those it dropped, so a rewatch
// answered with the same records hears nothing, and lets go nothing
// watched ahead, which it never names. A handler of a told change
// reads when its line was heard, and one of a watch's whole answer, or one
// it puts off, reads none. Each daemon's lines and their length are
// counted by its name, for the launcher's census, and begun again once
// said. A going says the asks it left unanswered, said as let go where the
// shell let the connection go itself. A client holding its wire watches
// neither its socket nor its folder. A socket that refuses is tried again
// twice as late each time, a minute at most, counted again from its next
// answer. Each step waits on what the client says, at most ten seconds.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
// Its stand-ins are in modules/quickshell/tests/stand-ins. Long because
// its cases run in turn against one real people daemon and its stand-ins,
// each starting from the state the last left, so split apart each file
// would start the daemon and walk the client back to that state again.
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
        // Told by the people daemon's watch ahead, which tells Ada as she
        // was before it stopped, so her record held doesn't change: what
        // it tells is what says it was asked. Started again, it knows her
        // only once the source it asks ahead answers, so she comes as a
        // change of the watch rather than in its first answer.
        onTold: answer => {
            if (answer.id === "ahead" && (answer.records ?? answer.changed ?? [])
                .some(record => record?.key === root.ada))
                root.aheadTold += 1;
        }
    }
    property var peopleChanged: []
    property var peopleGone: []
    property int aheadTold: 0

    // Watching only those a source lists with nothing typed.
    Daemon {
        id: listed

        socket: `${root.runtime}/daemons/people.sock`
        kind: "person"
        watching: ({ listed: true })
    }

    // Let go by the shell with an ask in flight, as OnSelection lets its
    // connection go once nothing needs it, through a link of its own to the
    // people daemon's socket, so its going is said under its own name
    // (checks/shell.nix reads the log for it).
    Daemon {
        id: lettingGo

        socket: `${root.runtime}/daemons/letting-go.sock`
        kind: "person"
    }
    property bool letGo: false
    // Whether the daemon's starting step has connected the let-go link again.
    property bool relinked: false
    // What its ask in flight was given as it went: undefined until then.
    property var letGoGiven: undefined

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

    // Watching sessions, told what differs, as mycelium's node tells it,
    // their presence asked deep, as hello offers it.
    Daemon {
        id: differing

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "session"
        changesDiffer: true
        watching: ({ deep: ["presence"] })
    }

    // Watching sessions told what differs, asking nothing deep.
    Daemon {
        id: flat

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

    // Watching two kinds of record, its answer the whole of one, as the passwords child's is of
    // its asks' rows and not of its entries, which it lists only once a manager is read.
    Daemon {
        id: wholeFor

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "lapsing"
        answersWholeFor: key => key === "lapsing:ask"
        onRevisionChanged: {
            if ("lapsing:ask" in held && "lapsing:entry" in held)
                root.heldLapsing = true;
        }
    }
    property bool heldLapsing: false

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

    // Answered whole and watching a key ahead that its kind's whole answer never names, as a
    // place's ready rows watch a drawer served apart: watched again, as each opening watches.
    Daemon {
        id: wholeAhead

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "drawn"
        answersWhole: true
        ahead: ["drawn:apart"]
    }
    property bool wholeAheadRewatched: false

    // What a handler of its records reads as the time its line was heard,
    // for the watch's whole answer and the change told after it, and what
    // one it puts off reads, so the launcher times a change from its line,
    // never what an opening's watch catches up on, and never one told
    // later (Hearing.js).
    Daemon {
        id: marked

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "mark"
        onHeard: (changed, gone) => {
            if (marked.held["mark:one"]?.heard === 1) {
                root.wholeMarkedAt = Hearing.heardAt();
                return;
            }
            root.markedAt = Hearing.heardAt();
            root.markedBy = Hearing.heardBy();
            root.marks = root.marks.concat([root.markedAt]);
            root.markedThen = Date.now();
            Qt.callLater(() => root.markedLater = Hearing.heardAt());
            Qt.callLater(Hearing.putOff(root.carriedLater));
            Hearing.putOff(root.carriedAsk);
            if (root.askedFrom < 0)
                carrying.start();
        }
    }
    property real wholeMarkedAt: -1
    property real markedAt: -1
    property string markedBy: ""
    property real markedThen: -1
    property real markedLater: -1
    property var marks: []
    // What the census would say of the lines heard, taken once.
    property var heardSaid: null
    // The lines the people and kinds daemons' connections had heard then.
    property var heardLines: []

    // A tell put off carrying its line reads it, by Qt.callLater or a
    // Timer; and an ask sent from one is answered as from that line, not
    // from the answer's own, as the launcher times a change an answer
    // tells (Hearing.js, Daemon.qml's sent).
    readonly property var carriedLater: Hearing.carried(() => {
        root.markedCarried = Hearing.heardAt();
        root.carriedBy = Hearing.heardBy();
    })
    property real markedCarried: -1
    property string carriedBy: ""
    readonly property var carriedAsk: Hearing.carried(() => {
        root.askedFrom = Hearing.heardAt();
        marked.find({ ask: "get", kind: "mark", keys: ["mark:asked"] }, () => {
            root.askAnswered = Hearing.heardAt();
            root.askAnsweredBy = Hearing.heardBy();
            root.askAnsweredThen = Date.now();
        });
    })
    property real askedFrom: -1
    property string askAnsweredBy: ""
    property real askAnswered: -1
    property real askAnsweredThen: -1
    // Long enough that the answer's own line is read in a later
    // millisecond than the line that asked, so the two can't be confused.
    Timer {
        id: carrying

        interval: 20
        onTriggered: root.carriedAsk.tell()
    }

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

    // Paced as a shut launcher's watch is, until the case opens it.
    property bool repacedShut: true
    Daemon {
        id: repaced

        socket: `${root.runtime}/daemons/kinds.sock`
        kind: "meeting"
        watching: root.repacedShut ? ({ every: 10000 }) : ({})
        paced: pacer
    }
    // The pace its record says its watch asked for, read as each step
    // polls, since what's held changes in place.
    function repacedSaid() {
        return repaced.held["meeting:one"]?.every;
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
        // Its refusals and whether its daemon runs say which of a restart's
        // tries a failing step was waiting on.
        console.warn(`FAIL ${what}: state ${people.state}, refused ${people.refusals} times,`
            + ` its daemon ${peopleRunning.running ? "running" : "stopped"}`
            + `, held ${JSON.stringify(people.held)}`
            + `, listed ${Object.keys(listed.held).join(" ")}`
            + `, kinds ${JSON.stringify(kinds.held)}`
            + `, lapsing ${wholeFor.state} ${root.heldLapsing} ${JSON.stringify(wholeFor.held)}`
            + `, answered ${JSON.stringify(root.answered)}`
            + `, letting go ${lettingGo.state} ${root.letGo} ${JSON.stringify(root.letGoGiven)}`
            + ` awaiting ${Object.keys(lettingGo.awaited).length}`
            + `, relinked ${root.relinked}, backed off ${lettingGo.refusals} times`
            + `, its last try ${root.failure(lettingGo.failedWith)}`
            + `, the census heard ${root.heardSaid} of ${root.heardLines.join(" and ")}`);
        Qt.exit(1);
    }

    // A link's last failure in words, by QLocalSocket's LocalSocketError, so a FAIL line read
    // from any step says it without Qt's enum beside it.
    function failure(code) {
        if (code === null)
            return "none failed since it last connected or was relinked, if it tried at all";
        const named = { "-1": "unknown", 0: "refused", 1: "closed by the daemon",
            2: "found nothing there", 3: "not allowed" }[code];
        return `failed with ${code}${named === undefined ? "" : `, ${named}`}`;
    }

    function heldWay(key) {
        return people.held[key]?.sources?.slack?.way ?? "";
    }

    // The steps in order, each with what it does and what it waits for.
    property int step: 0
    property int left: 0
    property var answered: undefined
    property int aheadToldBefore: 0
    // The stages an ask answered in steps was given, in order.
    property var stepsHeard: []

    readonly property var steps: [
        {
            what: "no daemon is said as not answering",
            // The let-go link's socket is a symlink to the daemon's, not there yet, so its
            // first try says it found nothing, as its FAIL line's words would.
            holds: () => people.state === "not answering" && !none.answered
                && lettingGo.failedWith === lettingGo.serverNotFound
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
            // The let-go link shares this daemon's socket through a symlink,
            // whose target appearing its file watch may not see, and its own
            // tries end 14 s in, so a daemon slower than that to start on a
            // busy builder could leave it unconnected. Once the daemon
            // answers, the link is connected again here, once, and waited for
            // as long as a restart's from then, rather than what's left of this
            // step or inside the let-go step's ten seconds. Its tries count
            // afresh, as the file watch's do, so a refused relink is tried again
            // 2 s on rather than 16 s, and one finding no socket is tried again
            // at all, 2, 6 and 14 s on, each inside the 20 s it's given. What
            // its tries failed with counts afresh too, so a FAIL line names
            // only the relink's, and it's said gone once the relink connects.
            polls: 400,
            holds: () => {
                const answering = people.state === "answering" && "slack" in people.sources;
                if (answering && lettingGo.wire === null && !root.relinked) {
                    root.relinked = true;
                    root.left = 400;
                    lettingGo.refusals = 0;
                    lettingGo.failedWith = null;
                    lettingGo.connectAgain();
                }
                return answering && lettingGo.wire !== null && lettingGo.failedWith === null;
            }
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
            // The socket's own handler may say its going before
            // onConnectsChanged does: a going read from who called it was
            // said as one gone wrong (#6513's F1). Which runs first isn't
            // pinned here, since connectAgain sets the loader's `active`,
            // which drops its binding. This step only lets it go: what its
            // going said, and at what level, checks/shell.nix reads from the
            // log. It comes before the daemon stops, while the link holds
            // the connection the daemon's start waited for: after the restart,
            // when it connects again rests on its watch of a symlinked
            // socket or on its tries, which no step waits on. It asks for
            // Grace, whom the step before held as a step-in, since a get is
            // a step-in told to the daemon at once, and one for Ada would
            // turn the way she's held, which the next step reads.
            what: "a connection the shell lets go with an ask in flight goes, its ask given none",
            holds: () => {
                if (!root.letGo && lettingGo.wire !== null) {
                    lettingGo.get([root.grace], 5000, answer => {
                        root.letGoGiven = answer;
                    });
                    lettingGo.connects = false;
                    root.letGo = true;
                }
                return root.letGo && lettingGo.wire === null
                    && Object.keys(lettingGo.awaited).length === 0 && root.letGoGiven === null;
            }
        },
        {
            what: "the daemon stopping is said, and what was held stays unless let go",
            run: () => {
                root.aheadToldBefore = root.aheadTold;
                peopleRunning.running = false;
            },
            // Held only once the daemon has exited, not just closed its
            // socket: a start asked of a Process still reaping its last is
            // nothing, so on a loaded builder the next step started no
            // daemon and its clients found only the old socket refusing
            // (Y2YBTS, 2026-11-12).
            holds: () => people.state === "not answering" && root.heldWay(root.ada) === "ahead"
                && forgetting.state === "not answering"
                && Object.keys(forgetting.held).length === 0 && !peopleRunning.running
        },
        {
            what: "the daemon starting again is connected to, and asked ahead again",
            // A daemon run by hand, as here, is tried again 2, 4 and 8 s
            // after it went, about 14 s in all (Daemon.qml), so the step
            // waits 20 s rather than ten: on a loaded builder its start
            // missed the 6 s try and came at 14 (2026-11-12, twice).
            polls: 400,
            run: () => {
                peopleRunning.running = true;
            },
            holds: () => people.state === "answering"
                && root.aheadTold > root.aheadToldBefore
                && root.hal in forgetting.held
        },
        {
            what: "a client watching two kinds holds records of both",
            // Its stand-in starts only now, and a try refused while it binds
            // waits up to 16 s before the next (Daemon.qml's waitAfter), past
            // the usual 10 s, as it did on a busy builder (2026-11-13).
            polls: 500,
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
            what: "a field asked deep is mended with only what moved inside it",
            holds: () => JSON.stringify(differing.held["session:a"]?.presence)
                === '{"at":2,"cwd":"~/src/skiff"}'
        },
        {
            what: "a field not asked deep is replaced whole, though hello offers it",
            holds: () => JSON.stringify(flat.held["session:a"]?.presence) === '{"at":2}'
        },
        {
            what: "a kind with no records is known to hold none once answered",
            holds: () => none.answered && Object.keys(none.held).length === 0
        },
        {
            what: "a watch paced as a shut launcher's is asks for its pace",
            holds: () => root.repacedSaid() === 10000
        },
        {
            // Asked at once, its answer would land before the opening's
            // frame, which draws from what's held.
            what: "its pace lifted, as the launcher opens, it's asked again after the frame",
            run: () => {
                root.repacedShut = false;
            },
            holds: () => root.left <= 195 && pacer.queued.length === 1
                && root.repacedSaid() === 10000
        },
        {
            // Agents' top level stayed paced, told at most every ten
            // seconds with the launcher open, so a parked orchestrator
            // woken read as closed for seconds (2026-11-13).
            what: "and once that frame is drawn it's told unpaced",
            run: () => pacer.drawn(),
            holds: () => root.repacedSaid() === null && pacer.queued.length === 0
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
            what: "a whole answer of some of its records lets go only those that went",
            holds: () => root.heldLapsing && wholeFor.answeredThisWatch
                && "lapsing:entry" in wholeFor.held && !("lapsing:ask" in wholeFor.held)
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
            what: "a whole answer lets go nothing watched ahead",
            holds: () => {
                if (!("drawn:apart" in wholeAhead.held))
                    return false;
                if (!root.wholeAheadRewatched) {
                    root.wholeAheadRewatched = true;
                    wholeAhead.live = false;
                    wholeAhead.live = true;
                    return false;
                }
                return wholeAhead.answeredThisWatch && "drawn:one" in wholeAhead.held;
            }
        },
        {
            what: "a handler of a change reads when its line was heard, one of a watch's"
                + " whole answer and one put off read none",
            holds: () => root.wholeMarkedAt === 0 && root.markedAt > 0 && root.markedAt <= root.markedThen
                && root.markedThen - root.markedAt < 1000 && root.markedLater === 0
                && Hearing.heardAt() === 0
        },
        {
            what: "one put off carrying its line reads it, and so does an ask's answer",
            holds: () => root.marks.includes(root.markedCarried)
                && root.marks.includes(root.askedFrom)
                && root.askAnswered === root.askedFrom
                && root.askAnsweredThen - root.askedFrom >= carrying.interval
                && Hearing.heardAt() === 0
        },
        {
            // Now is made from many daemons, and its changes said nothing of
            // whose lines set them going (2026-11-11, 20:10).
            what: "a line heard, one put off from it and an ask's answer each name its daemon",
            holds: () => root.markedBy === "kinds" && root.carriedBy === "kinds"
                && root.askAnsweredBy === "kinds" && Hearing.heardBy() === ""
        },
        {
            // The census says whose lines held the most, since parsing them
            // makes most of what Joel's shell makes (Hearing.js).
            what: "each daemon's lines and their length are counted by its name, the most"
                + " first, and the count begun again once said",
            holds: () => {
                // Taken once, since saying it begins the count again, with
                // the lines each daemon's connections heard at that moment,
                // so a line counted twice, or one connection's left out,
                // fails.
                const heardBy = daemons => daemons.reduce((sum, one) => sum + one.linesHeard, 0);
                if (root.heardSaid === null) {
                    root.heardLines = [heardBy([people, listed, forgetting, refreshed, paced]),
                        heardBy([kinds, differing, flat, whole, wholeFor, rewatched, wholeAhead,
                            marked, repaced, none])];
                    root.heardSaid = Hearing.heardMost(99).said;
                }
                const peopleSaid = /(?:^|, )people ([0-9]+) lines?, ([0-9]+) characters/
                    .exec(root.heardSaid);
                const kindsSaid = /(?:^|, )kinds ([0-9]+) lines?, ([0-9]+) characters/
                    .exec(root.heardSaid);
                // Each line is a JSON message, more than ten characters.
                const counts = [peopleSaid, kindsSaid].every((found, at) => found !== null
                    && Number(found[1]) === root.heardLines[at] && Number(found[1]) >= 2
                    && Number(found[2]) > 10 * Number(found[1]));
                // Begun at a known time, so each window's minutes are exact,
                // and the second is counted from the first's end.
                const begun = Date.now();
                Hearing.heardMost(0, begun);
                Hearing.counted("b", 3000);
                Hearing.counted("b", 1000);
                Hearing.counted("a", 5000);
                Hearing.counted("c", 10);
                const most = Hearing.heardMost(2, begun + 2 * 60000);
                const after = Hearing.heardMost(9, begun + 5 * 60000);
                const run = "/run/user/1000";
                const names = [Hearing.nameOf(`${run}/daemons/machine.sock`, [], run),
                    Hearing.nameOf(`${run}/eero/socket`, [], run),
                    Hearing.nameOf(`${run}/mycelium/personal.sock`, [], run),
                    Hearing.nameOf(`${run}/clipboard-history.sock`, [], run),
                    Hearing.nameOf("/run/wifid/rows", [], run),
                    Hearing.nameOf("", ["/nix/store/x/bin/passwords-child"], run),
                    Hearing.nameOf("/elsewhere/x/y.sock", [], ""),
                    Hearing.nameOf("", [], run)].join(" ");
                return counts
                    && most.said === "a 1 line, 5000 characters, b 2 lines, 4000 characters"
                    && most.minutes === 2 && most.lines === 4 && most.characters === 9010
                    && after.said === "" && after.minutes === 3 && after.lines === 0
                    && names === "machine eero mycelium/personal clipboard-history wifid/rows"
                        + " passwords-child x/y a child";
            }
        },
        {
            // A connection the shell let go itself, as OnSelection's does
            // once nothing needs it, was said as one gone wrong (#6287's
            // F31), so a journal read for a daemon's faults found ones
            // nobody had.
            what: "a going says its asks unanswered, as let go where the shell let"
                + " the connection go, and nothing where none was",
            holds: () => {
                const ids = Array.from({ length: 9 }, (_, at) => String(at + 1));
                return Hearing.goneSaid("people", ["7"], false)
                        === "people: the connection went with 1 ask unanswered: 7"
                    && Hearing.goneSaid("people", ids, false)
                        === "people: the connection went with 9 asks unanswered:"
                            + " 1, 2, 3, 4, 5, 6, 7, 8"
                    && Hearing.goneSaid("people", ["7", "8"], true)
                        === "people: let its connection go with 2 asks unanswered: 7, 8"
                    && Hearing.goneSaid("people", [], false) === ""
                    && Hearing.goneSaid("people", [], true) === "";
            }
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
        // Ten seconds a step, or as many polls as a step says it may need.
        left = now.polls ?? 200;
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

    // The whole run ends here, short of the check's 150 s timeout, so a run slowed past it
    // fails with its FAIL line, naming the step it was on, rather than killed with none. This
    // counts from the file loading, the timeout from the shell starting, so the 40 s between
    // are the shell's own start as well as the FAIL line's.
    Timer {
        running: true
        interval: 110000
        onTriggered: {
            waiting.stop();
            root.fail(`${root.steps[root.step]?.what ?? "its end"}, still waiting at 110 s`);
        }
    }

    Component.onCompleted: begin()
}
