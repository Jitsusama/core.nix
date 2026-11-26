pragma ComponentBehavior: Bound
// Drives the launcher's quests (modules/quickshell/launcher/Quests.qml)
// through Quickshell itself, its rows made by the quests daemon
// (modules/quests) from a stand-in for the realm's node serving the
// harness's two lists, the open quests and the ended ones, and the
// sessions, as mycelium's does (its PROTOCOL.md, The Daemon Interface),
// with niri, pi's sessions and the launcher stood in. With nothing typed it
// lists the open quests, the one driving first, each saying its kind, as
// the harness says it past the node's own, and priority, the open ones
// the only ones its watch serves; typing finds an ended one too, asked of
// the daemon, which sends its row with its answer, after the open ones
// that match as well, and by its id, its summary and acts as an open one's;
// Return opens pi on a quest in the folder its record says, autoloaded,
// unless a session has it loaded, when it goes to that session's window;
// the right arrow steps into its facts, its parent's title among them,
// and y takes its id without making them; Start a Session there steps
// into the machines the realm's node places on, where it has more than
// this one, each that can't take a session dimmed with why, each other
// saying how it stands as Machines says it, from the node's machine
// records, and Wherever It Fits last; a row told while the launcher is
// open in another place is made ahead, after a frame, and drawn as it is
// now on the step back, which makes no row, though none is made ahead as
// an opening fills in at home, holds another place's step or wakes its
// sources, under a viewer's command line, or shut with a step held; rows
// owed past a turn's worth are made a turn's worth a frame, felt as the
// place's ahead, one that throws let go for its step back and the rest
// made still, and none as a close runs a batch waiting on its frame,
// though the rows' drawn hasn't yet followed it; and a quest made since is heard,
// shut making no row, the daemon telling only the rows that moved, so a
// list written again with only when each was heard moved tells none and
// every other row is kept. It exits 0 once every case holds, and
// otherwise 1, saying which didn't. Long because each case follows the
// last on one daemon, its stand-in node and the launcher's held steps, so
// a row made ahead is told by the same records the list was first made
// from, which a split would have to serve again in each half.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Thrown.js" as Thrown

ShellRoot {
    id: root

    readonly property string here: Quickshell.env("PWD")
    property string waiting: "the first reading"
    // The rows before the store is written again, and how many were made.
    property var made: []
    property int counted: 0
    // The keys the daemon told moved or gone since a case began.
    property var told: []
    // A case waiting on the daemon's answer to what it typed, and what it
    // does with the rows found.
    property var pending: null

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // How many rows were made since a case counted them, and which.
    function rowsSince() {
        return `${theQuests.tally.rows - root.counted} rows: ${theQuests.tally.lately.join(", ")}`;
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: theLauncher

        property bool open: false
        // Whether it wants the quests' rows (Wanted.js), as it doesn't
        // while it's shut, which a case turns.
        property bool wanting: true
        function wants(source) {
            return wanting;
        }
        // The steps held, one in another place once a case steps there,
        // and whether the command line is over a viewer's.
        property var held: []
        property bool viewing: false
        // Whether it's opening or waking an opening's sources, as a chord's
        // and a kept opening's place is as it holds its step.
        property bool opening: false
        property bool waking: false
        // What waits for its next frame, run a frame's time later while
        // open or opening, and at once while shut, as no frame is coming,
        // as Launcher.qml's afterFrame does. A case holds the frames to
        // draw each itself.
        property var framed: []
        property bool framesHeld: false
        function afterFrame(then) {
            if (!open && !opening) {
                then();
                return;
            }
            framed = framed.concat([then]);
            if (!framesHeld)
                frameDrawn.restart();
        }
        function drawFrame() {
            const due = framed;
            framed = [];
            Thrown.runEach(due, said => console.warn(`what waited on a frame threw: ${said}`));
        }
        // Shut, or an opening ended unopened, what waits on a frame runs
        // in that turn, as Launcher.qml's afterFramesDue does from its close
        // and its unopened, before what follows open need have.
        onOpenChanged: {
            if (!open)
                drawFrame();
        }
        onOpeningChanged: {
            if (!opening && !open)
                drawFrame();
        }
        // What work it felt, by name, as Launcher.qml's feltSpan hears it.
        readonly property bool spansHeard: true
        property var felt: []
        function feltSpan(name, from) {
            felt = felt.concat([name]);
        }
        // The orchestrators the harness lists, as the launcher's sources
        // hold them, which a case fills.
        property var sources: []
    }

    Timer {
        id: frameDrawn

        interval: 16
        onTriggered: theLauncher.drawFrame()
    }

    // A moment for what's told to settle, then the next step, and the step
    // waiting on the quest's row of the session told next.
    Timer {
        id: settle

        property var then: null
        interval: 200
        onTriggered: then()
    }
    function settleThen(then) {
        settle.then = then;
        settle.restart();
    }
    property var toldWaits: null
    // The row whose telling it waits on.
    property string toldFor: "row:quests:QEST-20261003-PROMPT"

    Launcher.Quests {
        id: theQuests

        launcher: theLauncher
        settings: ({ name: "quests" })
        onAllChanged: {
            root.read();
            if (root.waiting === "the quest made since" && theQuests.all.length > 3)
                Qt.callLater(root.since);
        }
    }

    // Each change the daemon tells, after the quests took it, its keys
    // kept for the case that waits on it.
    Connections {
        target: theQuests.served

        function onHeard(changed, gone) {
            for (let at = 0; at < changed.length; at += 1)
                root.told.push(changed[at]);
            for (let at = 0; at < gone.length; at += 1)
                root.told.push(gone[at]);
            root.read();
            const rows = () => theQuests.looksThrough();
            if (root.waiting === "its session heard"
                && rows().some(row => row.side.endsWith("WCJQSG")))
                Qt.callLater(root.sessionHeard);
            if (root.waiting === "told away"
                && root.told.includes("row:quests:QEST-20261003-PROMPT") && !settled.running)
                settled.start();
            if (root.toldWaits !== null && root.told.includes(root.toldFor)) {
                const then = root.toldWaits;
                root.toldWaits = null;
                root.settleThen(then);
            }
            if (root.waiting === "heard shut" && theQuests.counted() === "4 open, 1 driving")
                Qt.callLater(root.heardShut);
            if (root.waiting === "only those heard"
                && !rows().some(row => row.key === "quest:QEST-20261020-NEWONE")
                && theQuests.counted() === "3 open, 1 driving")
                Qt.callLater(root.onlyThoseHeard);
        }

        // The answer a case waits on, taken as Quests takes it.
        function onSearched(words) {
            const pending = root.pending;
            if (pending === null || pending.words !== words)
                return;
            root.pending = null;
            Qt.callLater(() => {
                try {
                    pending.then(theQuests.search(words).rows);
                } catch (error) {
                    root.fail(error.message);
                }
            });
        }
    }

    // What `words` finds, once the daemon has answered it.
    function whenFound(words, then) {
        root.pending = { words: words, then: then };
        theQuests.search(words);
    }

    // What `words` finds once it finds anything, asked again a moment later
    // while the daemon finds nothing, as it does until it has heard what a
    // case told the node.
    function onceFound(words, then) {
        whenFound(words, rows => {
            if (rows.length > 0) {
                then(rows);
                return;
            }
            theQuests.served.ended();
            again.words = words;
            again.then = then;
            again.start();
        });
    }
    Timer {
        id: again

        property string words: ""
        property var then: null
        interval: 25
        onTriggered: root.onceFound(words, then)
    }

    // The first cases wait for the quests' rows and their count, and the
    // machines, in whichever order they come.
    readonly property bool machinesHeard: theQuests.machineKind?.answered ?? false
    onMachinesHeardChanged: read()
    function read() {
        if (root.waiting === "the first reading" && theQuests.all.length === 3 && machinesHeard
            && theQuests.counted() === "3 open, 1 driving")
            Qt.callLater(root.first);
    }

    // A quest as the harness lists it, and as the node serves it: under
    // the node's kind, with the quest's own again as quest.kind.
    function quest(id, title, served, fields) {
        return Object.assign({
            key: `${served}:${id}`, kind: served, heard: 1, id: id, title: title,
            "quest.kind": "quest", summary: `What ${title} is about.`, start: root.here
        }, fields);
    }
    readonly property var open: [
        quest("QEST-20261001-LAPTOP", "Bootstrap the laptop", "quest", { status: "active",
            priority: "driving", rank: 1, updated: "2026-10-22", start: `${root.here}/newer` }),
        quest("QEST-20261003-PROMPT", "Retune the prompt", "quest", {
            "quest.kind": "subquest", parent: "QEST-20261001-LAPTOP", status: "active",
            priority: "queued", rank: 2, updated: "2026-10-21T09:00:00.000Z",
            start: `${root.here}/session` }),
        quest("QEST-20261004-STICK", "Wait on the stick", "quest", { status: "blocked",
            priority: "bench", rank: 1, updated: "2026-10-04" })
    ]
    readonly property var ended: [
        quest("QEST-20250105-BANNER", "Tune the banner", "ended-quest", { status: "concluded",
            updated: "2025-01-05" })
    ]

    // The realm's machines as the node lists them: this one, one out of
    // reach and one ready on its battery.
    readonly property var machines: [
        { key: "machine:alpha", name: "alpha", here: true, state: "ready" },
        { key: "machine:beta", name: "beta", state: "unreachable",
            measured_at: "2026-11-08T05:20:00Z" },
        { key: "machine:gamma", name: "gamma", state: "ready",
            power: { on_battery: true, charge: 40 } }
    ]

    // The node's watch of each kind, by kind, so the node can tell it more,
    // and what it was to be told of a kind it hasn't watched yet, told once
    // it does: the quests daemon watches each kind in its own time, so a
    // case reached before its session watch would otherwise tell nobody.
    property var watches: ({})
    property var untold: ({})

    function tell(kind, change) {
        const watch = root.watches[kind];
        if (!watch) {
            root.untold[kind] = (root.untold[kind] ?? []).concat([change]);
            return;
        }
        watch.socket.write(`${JSON.stringify(Object.assign({ id: watch.id }, change))}\n`);
        watch.socket.flush();
    }

    // What was held for a kind until its watch, told after its answer.
    function toldLate(kind) {
        const held = root.untold[kind] ?? [];
        delete root.untold[kind];
        for (const change of held)
            tell(kind, change);
    }

    // A stand-in for the machine's node, speaking the daemon interface:
    // hello, and a watch of either list or the sessions, which the quests
    // daemon makes the rows from, or of the machines, which the shell
    // reads, each answered whole.
    SocketServer {
        active: true
        path: Sessions.nodePath
        handler: Socket {
            id: asked
            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    const lists = { quest: root.open, "ended-quest": root.ended, session: [] };
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "node", state: "running" };
                    } else if (ask.ask === "watch" && ask.kind === "machine") {
                        answer = { id: ask.id, records: root.machines };
                    } else if (ask.ask === "watch" && ask.kind in lists) {
                        root.watches[ask.kind] = { socket: asked, id: ask.id };
                        answer = { id: ask.id, records: lists[ask.kind] };
                    } else {
                        root.fail(`an ask the quests have no need of: ${line}`);
                        answer = { id: ask.id, error: { code: "unknown-ask", message: line } };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                    if (ask.ask === "watch")
                        root.toldLate(ask.kind);
                }
            }
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function first() {
        if (root.waiting !== "the first reading")
            return;
        root.waiting = "the first cases";
        try {
            const open = theQuests.everything();
            expect("the open quests, driving first",
                names(open) === "Bootstrap the laptop, Retune the prompt, Wait on the stick", names(open));
            const [laptop, prompt, stick] = open;
            expect("a quest says its priority", laptop.side === "driving", laptop.side);
            expect("a subquest says its kind too", prompt.side === "subquest \u00b7 queued", prompt.side);
            expect("a blocked one wears waiting's mark", stick.glyph === Theme.glyph.pending, stick.glyph);
            expect("an active one wears its own", laptop.glyph === Theme.glyph.active, laptop.glyph);
            expect("one reads its summary's first paragraph",
                laptop.reads === "What Bootstrap the laptop is about.", laptop.reads);
            expect("one with no session is its own", laptop.is === "quest:QEST-20261001-LAPTOP",
                laptop.is);
            expect("only the open ones served", theQuests.looksThrough().length === 3
                && !("row:quests:QEST-20250105-BANNER" in theQuests.served.held),
                names(theQuests.looksThrough()));
            expect("a search waits on the daemon", theQuests.awaits("banner"), "it doesn't");
            // The daemon hears the ended quests on a follow of their own, so
            // on a loaded builder the first search may come before them.
            onceFound("banner", banner => {
                expect("a finished one, found by typing",
                    names(banner) === "Tune the banner" && banner[0].glyph === Theme.glyph.done
                        && banner[0].side === "5 Jan 2025", `${names(banner)} ${banner[0]?.side}`);
                expect("and read as an open one is",
                    banner[0].reads === "What Tune the banner is about.", banner[0].reads);
                expect("its acts an open one's", names(banner[0].verbs()).startsWith(
                    "Start Here, Orchestrate, ID, Status, Last Touched, Folder"),
                    names(banner[0].verbs()));
                expect("answered, a search waits no more", !theQuests.awaits("banner"),
                    "it waits");
                // Stepped into from home, Quests holds the one found, so
                // it's selected there; closing lets it go.
                expect("held in Quests while it's found", names(theQuests.everything())
                    === "Bootstrap the laptop, Retune the prompt, Wait on the stick"
                        + ", Tune the banner",
                    names(theQuests.everything()));
                theLauncher.open = true;
                theLauncher.open = false;
                expect("let go as the launcher closes", theQuests.everything().length === 3,
                    names(theQuests.everything()));
                expect("an open one titled as its row says",
                    theQuests.titleOf("QEST-20261001-LAPTOP") === "Bootstrap the laptop",
                    theQuests.titleOf("QEST-20261001-LAPTOP"));
                // Named by its id, as Machine's disk names what fills it,
                // a finished one's title is asked of the daemon.
                expect("a finished one's title asked, not known yet",
                    theQuests.titleOf("QEST-20250105-BANNER") === undefined,
                    theQuests.titleOf("QEST-20250105-BANNER"));
                root.waiting = "a finished one's title";
                root.whenTitled = () => {
                    expect("a finished one titled as the daemon answers",
                        theQuests.titleOf("QEST-20250105-BANNER") === "Tune the banner",
                        theQuests.titleOf("QEST-20250105-BANNER"));
                    root.waiting = "the first cases";
                    root.searchedThe(laptop, prompt);
                };
            });
        } catch (error) {
            fail(error.message);
        }
    }

    // A case waiting on the titles the daemon answers.
    property var whenTitled: null
    Connections {
        target: theQuests

        function onTitlesHeard() {
            const then = root.whenTitled;
            root.whenTitled = null;
            if (then)
                Qt.callLater(() => {
                    try {
                        then();
                    } catch (error) {
                        root.fail(error.message);
                    }
                });
        }
    }

    // What "the" and an id find.
    function searchedThe(laptop, prompt) {
        try {
            // Each matches "the" as well, and the finished one is the
            // shortest, yet the open ones come first.
            whenFound("the", the => {
                expect("the open ones before the finished one",
                    the.length === 4 && names(the).endsWith(", Tune the banner"), names(the));
                whenFound("20261003", byId => {
                    expect("found by its id", names(byId) === "Retune the prompt", names(byId));
                    root.acts(laptop, prompt);
                });
            });
        } catch (error) {
            fail(error.message);
        }
    }

    // What Return and the right arrow do on an open quest.
    function acts(laptop, prompt) {
        try {
            laptop.run();
            expect("starting a session in its newest tree",
                JSON.stringify(Sessions.started)
                    === JSON.stringify(["QEST-20261001-LAPTOP", `${root.here}/newer`]),
                JSON.stringify(Sessions.started));
            prompt.run();
            expect("starting one where its newest session ran",
                JSON.stringify(Sessions.started)
                    === JSON.stringify(["QEST-20261003-PROMPT", `${root.here}/session`]),
                JSON.stringify(Sessions.started));

            // A day this year goes without its year.
            const touched = new Date().getFullYear() === 2026 ? "21 Oct" : "21 Oct 2026";
            const facts = prompt.verbs();
            // Each fact named first, as every part is, then what it says.
            expect("stepping in", names(facts)
                === "Start Here, Orchestrate, ID, Status, Last Touched, Part Of, Folder",
                names(facts));
            // Return says the act that leads, so stepping in never lifts a
            // second one before it (Catalogue.js's leading).
            expect("Return says Start Here's verb", prompt.verb === facts[0].verb,
                `${prompt.verb} and ${facts[0].verb}`);
            // Orchestrate starts one that orchestrates it, where Start Here
            // would start.
            facts[1].run();
            expect("orchestrating it from where it would start",
                JSON.stringify(Sessions.started) === JSON.stringify(
                    ["QEST-20261003-PROMPT", `${root.here}/session`, "orchestrating"]),
                JSON.stringify(Sessions.started));
            // One the harness lists as orchestrating already reopens on its
            // own row in Agents, so it isn't offered again.
            theLauncher.sources = [{ settings: { name: "orchestrators" },
                listed: [{ quest: "QEST-20261003-PROMPT", address: "PROMPT" }] }];
            expect("no Orchestrate where it orchestrates already",
                !names(prompt.verbs()).includes("Orchestrate"), names(prompt.verbs()));
            theLauncher.sources = [];
            const sides = facts.slice(2).map(row => row.side).join(", ");
            expect("what each says", sides
                === `QEST-20261003-PROMPT, active \u00b7 queued, ${touched}, `
                + "Bootstrap the laptop, ~/session", sides);
            const folder = facts[facts.length - 1];
            expect("its folder copied whole", folder.copy === `${root.here}/session`, folder.copy);
            expect("a fact copies itself", facts[2].verb === "copy", facts[2].verb);
            // What y takes is what its first fact copies, made without its
            // facts, since y over hundreds asks every row.
            const taken = typeof prompt.operands === "function" ? prompt.operands() : [];
            expect("y takes its id, as its first fact copies it", taken.length === 1
                && taken[0].verb === "copy" && taken[0].copies === facts[2].copies(),
                JSON.stringify(taken.map(one => one.copies)));
            expect("with this machine alone, Start Here holds no step",
                facts[0].verbs === null && facts[0].note === "", facts[0].note);
            expect("the node asked where sessions may go", Sessions.lookedAround > 0,
                Sessions.lookedAround);
            expect("a quest's title found by its id",
                theQuests.titleOf("QEST-20261004-STICK") === "Wait on the stick",
                theQuests.titleOf("QEST-20261004-STICK"));

            // With another machine the realm links, out of reach now.
            Sessions.places = [
                { name: "alpha", cannot: "", rather: "" },
                { name: "beta", cannot: "out of reach", rather: "" },
                { name: "gamma", cannot: "", rather: "" }
            ];
            Sessions.fits = "alpha";
            const start = prompt.verbs()[0];
            expect("Start Here says where Return starts one", start.note === "this machine",
                start.note);
            const machines = start.verbs();
            expect("stepping into the machines, this one first, Wherever It Fits last",
                names(machines) === "alpha, beta, gamma, Wherever It Fits", names(machines));
            const [here, away, other, anywhere] = machines;
            expect("this machine says so", here.note === "this machine" && !here.gone, here.note);
            expect("one that can't take it stays, dimmed, saying why",
                away.gone && away.note === "out of reach", `${away.gone} ${away.note}`);
            expect("one that can take it says how it stands, as Machines says it",
                !other.gone && other.note === "ready \u00b7 on battery 40%", other.note);
            expect("Wherever It Fits says where", anywhere.note === "on alpha", anywhere.note);
            here.run();
            expect("starting it on the machine chosen",
                JSON.stringify(Sessions.startedOn)
                    === JSON.stringify(["alpha", "QEST-20261003-PROMPT", `${root.here}/session`]),
                JSON.stringify(Sessions.startedOn));
            anywhere.run();
            expect("Wherever It Fits leaves it to the node", Sessions.startedOn[0] === "",
                JSON.stringify(Sessions.startedOn));
            Sessions.places = [];

            // A session with it loaded, as the node tells its sessions and
            // Sessions holds its window.
            Sessions.running = [{
                address: "WCJQSG",
                window: { id: 7 },
                presence: { owns: ["QEST-20261003-PROMPT"] }
            }];
            root.waiting = "its session heard";
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "running",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } }] });
        } catch (error) {
            fail(error.message);
        }
    }

    // Return goes to the session that has it loaded, and its row is that
    // session's window.
    function sessionHeard() {
        if (root.waiting !== "its session heard")
            return;
        root.waiting = "the session's cases";
        try {
            const held = theQuests.everything()[1];
            expect("a loaded one names its session",
                held.side === "subquest \u00b7 queued \u00b7 WCJQSG" && held.verb === "go to", held.side);
            expect("a loaded one is its session's window", held.is === "window:7", held.is);
            held.run();
            expect("going to its session", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":7}}',
                JSON.stringify(Niri.acted));
            expect("its step says so", held.verbs()[0].name === "Go To Its Session" && held.verbs()[0].verb === "go to",
                held.verbs()[0].name);

            // Open at home as a kept opening fills in, Quests isn't wanted
            // before its turn to wake, and its session parks, as the node
            // tells sessions parking and waking all day.
            root.told = [];
            root.counted = theQuests.tally.rows;
            root.waiting = "told away";
            theLauncher.open = true;
            theLauncher.wanting = false;
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "parked",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } }] });
        } catch (error) {
            fail(error.message);
        }
    }

    // A moment for what's told away to settle, since rows made ahead of the
    // step back are made off the frame that draws it.
    Timer {
        id: settled

        interval: 200
        onTriggered: root.filledAtHome()
    }

    // Filling in at home, or under a viewer's command line, no row is made
    // ahead, in the frames the opening keeps short; stepped into another
    // place, the row told is.
    function filledAtHome() {
        if (root.waiting !== "told away")
            return;
        try {
            expect("no row made ahead as an opening fills in at home",
                theQuests.tally.rows === root.counted,
                `${theQuests.tally.rows - root.counted} rows: ${theQuests.tally.lately.join(", ")}`);
            // A chord, or a kept opening, holds its place's step as it
            // opens, before that place has woken, and then as it wakes its
            // sources: each alone.
            theLauncher.opening = true;
            theLauncher.held = [{ row: { from: theLauncher } }];
            root.settleThen(root.filledOpening);
        } catch (error) {
            fail(error.message);
        }
    }

    function filledOpening() {
        try {
            expect("nor as an opening holds another place's step",
                theQuests.tally.rows === root.counted, rowsSince());
            theLauncher.waking = true;
            theLauncher.opening = false;
            root.settleThen(root.filledInPlace);
        } catch (error) {
            fail(error.message);
        }
    }

    function filledInPlace() {
        try {
            expect("nor as an opening wakes another place's sources",
                theQuests.tally.rows === root.counted, rowsSince());
            theLauncher.viewing = true;
            theLauncher.waking = false;
            settledViewing.start();
        } catch (error) {
            fail(error.message);
        }
    }

    Timer {
        id: settledViewing

        interval: 200
        onTriggered: root.viewed()
    }

    function viewed() {
        try {
            expect("nor under a viewer's command line", theQuests.tally.rows === root.counted,
                `${theQuests.tally.rows - root.counted} rows`);
            theLauncher.viewing = false;
            settledElsewhere.start();
        } catch (error) {
            fail(error.message);
        }
    }

    Timer {
        id: settledElsewhere

        interval: 200
        onTriggered: root.steppedBack()
    }

    // Stepped back home, Quests is wanted again, and a quest's row told
    // while it wasn't is drawn as it is now, with nothing made on the step:
    // Joel's "\u2190 now" made 31 to 37 rows at each, all told away, as
    // its session parked or woke or its folder moved (2026-11-23).
    function steppedBack() {
        if (root.waiting !== "told away")
            return;
        try {
            expect("the row told made ahead in another place", theQuests.tally.rows > root.counted,
                "none made");
            // Its session runs again while the launcher stays there, as
            // an opening there wakes its sources.
            root.counted = theQuests.tally.rows;
            root.waiting = "told elsewhere";
            root.told = [];
            root.toldWaits = root.toldElsewhere;
            theLauncher.waking = true;
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "running",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } }] });
        } catch (error) {
            fail(error.message);
        }
    }

    // Told while already in another place, the row is made there once its
    // sources have woken, with nothing else changed.
    function toldElsewhere() {
        try {
            expect("no row made while another place's sources wake",
                theQuests.tally.rows === root.counted, rowsSince());
            theLauncher.waking = false;
            root.settleThen(root.wokenElsewhere);
        } catch (error) {
            fail(error.message);
        }
    }

    // Owed more rows than a turn makes, one a frame, as Joel's 31 to 37
    // and 224 are eight a frame: a session holds only the first quest it
    // owns (modules/quests/src/holder.rs), so another, LPTOPS, takes the
    // laptop's, and both rows are told.
    function laptopSession(standing) {
        return { key: "session:LPTOPS", standing,
            presence: { addresses: ["LPTOPS"], owns: ["QEST-20261001-LAPTOP"] } };
    }
    function wokenElsewhere() {
        try {
            expect("a row told while in another place made ahead there once woken",
                theQuests.tally.rows > root.counted, "none made");
            // A turn set to make none still makes one, rather than frames
            // spun making nothing.
            theQuests.served.aheadEachTurn = 0;
            theLauncher.framesHeld = true;
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.waiting = "a batch owed";
            root.toldWaits = root.batchOwed;
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "parked",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } },
            laptopSession("running")] });
        } catch (error) {
            fail(error.message);
        }
    }

    function batchOwed() {
        try {
            const told = ["row:quests:QEST-20261001-LAPTOP", "row:quests:QEST-20261003-PROMPT"];
            expect("both rows told", told.every(key => root.told.includes(key)),
                root.told.join(", "));
            expect("none made before the frame", theQuests.tally.rows === root.counted,
                rowsSince());
            theLauncher.drawFrame();
            expect("one made a frame", theQuests.tally.rows === root.counted + 1, rowsSince());
            theLauncher.drawFrame();
            expect("the next the frame after", theQuests.tally.rows === root.counted + 2,
                rowsSince());
            expect("nor a turn past those owed", theLauncher.framed.length === 0
                && Object.keys(theQuests.served.owedAhead).length === 0,
                `${theLauncher.framed.length} waiting, `
                + `${Object.keys(theQuests.served.owedAhead).join(", ")} owed`);
            // Told again, both are made in the one frame, the one that
            // throws as it's made among them.
            theQuests.served.aheadEachTurn = 8;
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.waiting = "a row that throws";
            root.toldWaits = root.aRowThrows;
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "running",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } },
            laptopSession("parked")] });
        } catch (error) {
            fail(error.message);
        }
    }

    // A row that throws as it's made ahead, the first owed, once it's
    // made, is let go for its step back, not left made, and the rest made
    // still, none left waiting on a frame; its making is felt as Quests'
    // ahead, as a frame it stalls says who held it.
    function aRowThrows() {
        const served = theQuests.served;
        const making = served.make;
        const owed = Object.keys(served.owedAhead);
        const [throwing, rest] = owed;
        // Each time the row's key was read, and threw.
        let threw = 0;
        try {
            expect("both owed", owed.length === 2, owed.join(", "));
            // The row made of it throws as its key is read, which rowAt
            // does once it's written it as made, to name it as made of
            // its record, and which letting it go does too. It's made as
            // ever, so it's counted, and given back as a row of its own
            // with the getter from the start: turned into a getter in
            // place, a field breaks every row of its shape (Member.js).
            served.make = (row, key) => {
                const madeRow = making(row, key);
                if (key !== throwing)
                    return madeRow;
                return Object.defineProperty({}, "key", { get: () => {
                    threw += 1;
                    throw new Error("a row broke");
                } });
            };
            theLauncher.felt = [];
            theLauncher.drawFrame();
            served.make = making;
            expect("the rest made, though the first owed threw",
                theQuests.tally.rows === root.counted + 2 && rest in served.made, rowsSince());
            // Made is all rowAt wrote of it before it threw.
            expect("the one that threw let go for its step back, not left made",
                threw > 0 && !(throwing in served.made),
                JSON.stringify({ threw, made: throwing in served.made }));
            expect("and none left waiting on a frame", theLauncher.framed.length === 0
                && Object.keys(served.owedAhead).length === 0,
                `${theLauncher.framed.length} waiting, `
                + `${Object.keys(served.owedAhead).join(", ")} owed`);
            expect("felt as Quests' ahead", theLauncher.felt.includes("quests ahead"),
                theLauncher.felt.join(", "));
            // Its step back makes it whole.
            served.rowAt(throwing);
            expect("made whole once wanted", throwing in served.made, "not made");
            // The laptop's told again, it waits on the next frame as the
            // launcher shuts; the prompt's stays made, to be told shut.
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.waiting = "a batch waiting";
            root.toldFor = "row:quests:QEST-20261001-LAPTOP";
            root.toldWaits = root.batchWaiting;
            tell("session", { changed: [laptopSession("running")] });
        } catch (error) {
            served.make = making;
            fail(error.message);
        }
    }

    // Shut with a batch waiting on its frame, the close runs it in its own
    // turn, which makes none, though the rows' drawn hasn't followed the
    // close yet, as the order Qt tells the two in leaves open, so the row
    // stays owed; a row made, told while shut with that place's step still
    // held, as a kept place leaves it, is owed, not made, until the
    // launcher opens there again.
    function batchWaiting() {
        const prompt = "row:quests:QEST-20261003-PROMPT";
        try {
            expect("a batch waits on the frame, the prompt's row made and not owed",
                theLauncher.framed.length > 0
                && JSON.stringify(Object.keys(theQuests.served.owedAhead))
                    === JSON.stringify(["row:quests:QEST-20261001-LAPTOP"])
                && prompt in theQuests.served.made,
                `${theLauncher.framed.length} waiting, `
                + `${Object.keys(theQuests.served.owedAhead).join(", ")} owed`);
            theQuests.served.drawn = true;
            theLauncher.open = false;
            theQuests.served.drawn = Qt.binding(() => theLauncher.open);
            // The close runs what waits on a frame, so the batch has run by now: one still
            // waiting would run in a later case, and this would prove nothing of makeAhead's
            // guard on the launcher's open.
            expect("a close with a batch waiting runs it and makes none",
                theLauncher.framed.length === 0 && !theQuests.served.aheadWaiting
                && theQuests.tally.rows === root.counted,
                `${theLauncher.framed.length} waiting, ${rowsSince()}`);
            theLauncher.framesHeld = false;
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.toldFor = prompt;
            root.toldWaits = root.toldShut;
            // The prompt's session parks, told shut, and the laptop's
            // ends, so its quest is held by none again for the cases after.
            tell("session", { changed: [{ key: "session:WCJQSG", standing: "parked",
                presence: { addresses: ["WCJQSG"], owns: ["mesh", "QEST-20261003-PROMPT"] } },
            laptopSession("ended")] });
        } catch (error) {
            fail(error.message);
        }
    }

    function toldShut() {
        try {
            expect("no row made shut, though a step stays held", theQuests.tally.rows === root.counted,
                `${theQuests.tally.rows - root.counted} rows`);
            const served = theQuests.served;
            const prompt = "row:quests:QEST-20261003-PROMPT";
            expect("the row made, told shut, owed and not made",
                prompt in served.owedAhead && !(prompt in served.made),
                JSON.stringify({ owed: prompt in served.owedAhead, made: prompt in served.made }));
            // The stand-in itself, which the cases' frames are drawn by:
            // shut, an opening that ends unopened runs what waited on a
            // frame then, as Launcher.qml's unopened does, so a case
            // can't pass on work the stand-in never ran. Launcher.qml's
            // own is out of this check's reach.
            let ran = false;
            theLauncher.opening = true;
            theLauncher.afterFrame(() => ran = true);
            const heldBack = !ran;
            theLauncher.opening = false;
            expect("the stand-in's opening holds what waits on a frame, run once it ends unopened",
                heldBack && ran, JSON.stringify([heldBack, ran]));
            theLauncher.open = true;
            root.settleThen(root.reopenedElsewhere);
        } catch (error) {
            fail(error.message);
        }
    }

    function reopenedElsewhere() {
        try {
            const before = theQuests.tally.rows;
            const served = theQuests.served;
            expect("the rows owed at the close and told shut made ahead once open there again",
                before - root.counted === 2 && "row:quests:QEST-20261003-PROMPT" in served.made
                && "row:quests:QEST-20261001-LAPTOP" in served.made
                && Object.keys(served.owedAhead).length === 0,
                `${before - root.counted} made, ${Object.keys(served.owedAhead).join(", ")} owed`);
            theLauncher.held = [];
            theLauncher.wanting = true;
            const prompt = theQuests.everything()[1];
            expect("the row told away drawn as it is now",
                prompt.verb !== "go to" && prompt.side.endsWith("WCJQSG"),
                `${prompt.verb} ${prompt.side}`);
            expect("the step back makes no row", theQuests.tally.rows === before,
                `${theQuests.tally.rows - before} rows: ${theQuests.tally.lately.join(", ")}`);
            heardShutNext();
        } catch (error) {
            fail(error.message);
        }
    }

    // Shut, the node tells every quest again, as the harness writes its
    // list.
    function heardShutNext() {
        try {
            theLauncher.open = false;
            root.made = theQuests.looksThrough();
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.waiting = "heard shut";
            theLauncher.wanting = false;
            // The node tells every quest again as the harness writes its
            // list, heard anew, and the new one whole.
            tell("quest", { changed: root.open.map(one => ({ key: one.key, heard: 2 })).concat([
                quest("QEST-20261020-NEWONE", "Newly made", "quest", { status: "active",
                    priority: "someday", rank: 1, updated: "2026-10-20" })
            ]) });
        } catch (error) {
            fail(error.message);
        }
    }

    // Heard while shut, nobody sees a quest's row, so none is made, and
    // Joel's shut launcher made 184 in 240 s (2026-11-10); wanted again,
    // the new one is made.
    function heardShut() {
        if (root.waiting !== "heard shut")
            return;
        try {
            expect("no row made shut", theQuests.tally.rows === root.counted,
                `${theQuests.tally.rows - root.counted} rows`);
            expect("the rows last made still given", theQuests.all.length === 3,
                `${theQuests.all.length} rows`);
            // Every quest was told again as the list was written, and
            // every quest's row was once made again at each, 1,063 on
            // Joel's shell (2026-11-10): the daemon tells only the new one.
            const told = root.told.slice().sort().join(", ");
            expect("only the new quest's row and the count told",
                told === "row:quests:QEST-20261020-NEWONE, summary:quests", told);
            root.waiting = "the quest made since";
            theLauncher.wanting = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function since() {
        if (root.waiting !== "the quest made since")
            return;
        root.waiting = "the change heard";
        whenFound("newly", found => root.madeSince(found));
    }

    function madeSince(found) {
        try {
            expect("a quest made since", names(found) === "Newly made", names(found));
            expect("after the open ones it ranks below", theQuests.everything()[3]?.name
                === "Newly made", names(theQuests.everything()));
            const now = theQuests.looksThrough();
            const remade = root.made.filter(row => !now.includes(row));
            expect("every other quest's row kept", remade.length === 0, names(remade));
            expect("one row made, for the new quest", theQuests.tally.rows - root.counted === 1,
                `${theQuests.tally.rows - root.counted} rows`);
            // The list written again with nothing moved but when each was
            // heard, then the new one let go, then the ended one retired
            // and retitled: only the one let go and the count are told,
            // since the ended one isn't served, and is found as it is now.
            root.made = now;
            root.counted = theQuests.tally.rows;
            root.told = [];
            root.waiting = "only those heard";
            tell("quest", { changed: root.open.map(one => ({ key: one.key, heard: 3 }))
                .concat([{ key: "quest:QEST-20261020-NEWONE", heard: 3 }]) });
            tell("quest", { gone: ["quest:QEST-20261020-NEWONE"] });
            tell("ended-quest", { changed: [{ key: "ended-quest:QEST-20250105-BANNER",
                status: "retired", title: "Retune the banner" }] });
        } catch (error) {
            fail(error.message);
        }
    }

    // A change tells only the rows it moves, and the rest are kept.
    function onlyThoseHeard() {
        if (root.waiting !== "only those heard")
            return;
        root.waiting = "the retired one found";
        onceFound("retune banner", found => root.retiredFound(found));
    }

    function retiredFound(found) {
        root.waiting = "the end";
        try {
            const told = Array.from(new Set(root.told)).sort().join(", ");
            expect("told only the one let go and the count", told
                === "row:quests:QEST-20261020-NEWONE, summary:quests", told);
            const now = theQuests.looksThrough();
            const banner = found[0];
            expect("the retired one found as it is now", names(found) === "Retune the banner"
                && banner.glyph === Theme.glyph.cancelled, `${names(found)} ${banner.glyph}`);
            const remade = root.made.filter(row => !now.includes(row)
                && row.key !== "quest:QEST-20261020-NEWONE" && row.key !== banner.key);
            expect("every other quest's row kept", remade.length === 0, names(remade));
            expect("one row made, for the one retired found",
                theQuests.tally.rows - root.counted === 1,
                `${theQuests.tally.rows - root.counted} rows`);
            expect("counted again", theQuests.counted() === "3 open, 1 driving",
                theQuests.counted());
            Qt.callLater(root.movedCases);
        } catch (error) {
            fail(error.message);
        }
    }

    // The asks file the daemon reads for what moved, which the check
    // writes before the daemon starts, and a case writes again.
    FileView {
        id: asksFile
        path: Quickshell.env("ASKS")
        blockWrites: true
    }
    readonly property string askedAt: "2026-11-20T10:00:00-05:00"

    // What Moved as Quests makes it from what the daemon serves: nothing
    // of the asks there at its start; an ask landing for an open quest is
    // that quest's own row, named and acted on as Quests' is, saying what
    // landed; a quest's own row changing tells what moved again; and given
    // back as looked at, it goes.
    function movedCases() {
        root.waiting = "what moved first";
        try {
            expect("nothing of the asks there at the start", theQuests.moved().length === 0,
                JSON.stringify(theQuests.served.held["moved:quests"]));
            const revision = theQuests.movedRevision;
            theQuests.served.heard(["row:quests:QEST-20261001-LAPTOP"], []);
            theQuests.served.heard(["summary:quests"], []);
            expect("a quest's own row tells what moved again, and nothing else does",
                theQuests.movedRevision === revision + 1,
                `${revision} to ${theQuests.movedRevision}`);
            const first = { at: root.askedAt, owner: "LAPTOP", words: "the first",
                landed: root.askedAt };
            const next = { at: root.askedAt, owner: "LAPTOP (PROMPT)", words: "the dock",
                landed: root.askedAt };
            root.waiting = "what moved heard";
            asksFile.setText(`${JSON.stringify(first)}\n${JSON.stringify(next)}\n`);
        } catch (error) {
            fail(error.message);
        }
    }
    Connections {
        target: theQuests

        function onMovedRevisionChanged() {
            const moved = theQuests.moved();
            if (root.waiting === "what moved heard" && moved.length === 1)
                Qt.callLater(root.movedHeard);
            if (root.waiting === "what moved looked at" && moved.length === 0)
                Qt.callLater(() => Qt.exit(0));
        }
    }

    function movedHeard() {
        if (root.waiting !== "what moved heard")
            return;
        root.waiting = "what moved looked at";
        try {
            const row = theQuests.moved()[0];
            const own = theQuests.looksThrough()
                .find(one => one.key === "quest:QEST-20261001-LAPTOP");
            expect("the quest that moved is its own row, named as it is",
                row.key === own.key && row.name === "Bootstrap the laptop"
                    && row.name === own.name,
                JSON.stringify([row.key, row.name]));
            expect("saying what landed where its note was", row.note === "landed: the dock",
                row.note);
            expect("acted on as the quest's own row is",
                JSON.stringify(row.verbs().map(verb => verb.name))
                    === JSON.stringify(own.verbs().map(verb => verb.name)),
                JSON.stringify(row.verbs().map(verb => verb.name)));
            const seen = theQuests.movedSeen();
            expect("what it shows of the ask that moved", seen.length === 1
                && seen[0].key === `${root.askedAt}\tthe dock`, JSON.stringify(seen));
            theQuests.looked(seen);
        } catch (error) {
            fail(error.message);
        }
    }
}
