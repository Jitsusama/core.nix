pragma ComponentBehavior: Bound
// Drives the launcher's quests (modules/quickshell/launcher/Quests.qml)
// through Quickshell itself, its rows made by the quests daemon
// (modules/quests) from a stand-in for the realm's node serving the
// harness's two lists, the open quests and the ended ones, and the
// sessions, as mycelium's does (its PROTOCOL.md, The Daemon Interface),
// with niri, pi's sessions and the launcher stood in. With nothing typed it
// lists the open quests, the one driving first, each saying its kind, as
// the harness says it past the node's own, and priority; typing finds an
// ended one too, after the open ones that match as well, and by its id;
// Return opens pi on a quest in the folder its record says, autoloaded,
// unless a session has it loaded, when it goes to that session's window;
// the right arrow steps into its facts, its parent's title among them,
// and y takes its id without making them; Start a Session there steps
// into the machines the realm's node places on, where it has more than
// this one, each that can't take a session dimmed with why, each other
// saying how it stands as Machines says it, from the node's machine
// records, and Wherever It Fits last; and a quest made since is heard,
// shut making no row, the daemon telling only the rows that moved, so a
// list written again with only when each was heard moved tells none and
// every other row is kept. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string here: Quickshell.env("PWD")
    property string waiting: "the first reading"
    // The rows before the store is written again, and how many were made.
    property var made: []
    property int counted: 0
    // The keys the daemon told moved or gone since a case began.
    property var told: []

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
        // Whether it wants the quests' rows (Wanted.js), as it doesn't
        // while it's shut, which a case turns.
        property bool wanting: true
        function wants(source) {
            return wanting;
        }
        // The orchestrators the harness lists, as the launcher's sources
        // hold them, which a case fills.
        property var sources: []
    }

    Launcher.Quests {
        id: theQuests

        launcher: theLauncher
        settings: ({ name: "quests" })
        onAllChanged: {
            root.read();
            if (root.waiting === "the quest made since" && theQuests.all.length > 4)
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
            if (root.waiting === "heard shut" && theQuests.counted() === "4 open, 1 driving")
                Qt.callLater(root.heardShut);
            if (root.waiting === "only those heard"
                && !rows().some(row => row.key === "quest:QEST-20261020-NEWONE")
                && rows().some(row => row.name === "Retune the banner"))
                Qt.callLater(root.onlyThoseHeard);
        }
    }

    // The first cases wait for the quests' rows and their count, and the
    // machines, in whichever order they come.
    readonly property bool machinesHeard: theQuests.machineKind?.answered ?? false
    onMachinesHeardChanged: read()
    function read() {
        if (root.waiting === "the first reading" && theQuests.all.length === 4 && machinesHeard
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

    // The node's watch of each kind, by kind, so the node can tell it more.
    property var watches: ({})

    function tell(kind, change) {
        const watch = root.watches[kind];
        watch.socket.write(`${JSON.stringify(Object.assign({ id: watch.id }, change))}\n`);
        watch.socket.flush();
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

            const banner = theQuests.search("banner").rows;
            expect("a finished one, found by typing",
                names(banner) === "Tune the banner" && banner[0].glyph === Theme.glyph.done
                    && banner[0].side === "5 Jan 2025", `${names(banner)} ${banner[0]?.side}`);
            // Each matches "the" as well, and the finished one is the
            // shortest, yet the open ones come first.
            const the = theQuests.search("the").rows;
            expect("the open ones before the finished one",
                the.length === 4 && names(the).endsWith(", Tune the banner"), names(the));
            const byId = theQuests.search("20261003").rows;
            expect("found by its id", names(byId) === "Retune the prompt", names(byId));

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
            expect("the rows last made still given", theQuests.all.length === 4,
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
        try {
            const found = theQuests.search("newly").rows;
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
            // and retitled: only those two rows and the count are told.
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
        root.waiting = "the end";
        try {
            const told = Array.from(new Set(root.told)).sort().join(", ");
            expect("told only the one let go, the one retired and the count", told
                === "row:quests:QEST-20250105-BANNER, row:quests:QEST-20261020-NEWONE, "
                + "summary:quests", told);
            const now = theQuests.looksThrough();
            const banner = now.find(row => row.name === "Retune the banner");
            expect("the retired one wears its mark", banner.glyph === Theme.glyph.cancelled,
                banner.glyph);
            const remade = root.made.filter(row => !now.includes(row)
                && row.key !== "quest:QEST-20261020-NEWONE" && row.key !== banner.key);
            expect("every other quest's row kept", remade.length === 0, names(remade));
            expect("one row made, for the one retired", theQuests.tally.rows - root.counted === 1,
                `${theQuests.tally.rows - root.counted} rows`);
            expect("counted again", theQuests.counted() === "3 open, 1 driving",
                theQuests.counted());
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}
