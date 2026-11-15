// How pi, as an agent source, sorts and says its sessions, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `pi-tells-of-its-sessions-as-an-agent-source` check copies
// modules/quickshell/PiSource.js beside this file and runs
// `qml -platform offscreen pi-source.qml`, over session records as the
// realm's mycelium node serves them, each saying how it stands: running,
// here or elsewhere, parked or closed, and passed over when it says
// nothing or has no address. Each is said as the contract's record, with
// OpenTelemetry's gen_ai names and pi's own under `pi.`, leaving out what
// pi didn't say. With the launcher shut, a change whose records carry no
// new digest of what's read shut moves nothing. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import QtQuick
import "PiSource.js" as Pi

QtObject {
    Component.onCompleted: {
        try {
            run();
            console.log("every source case holds");
            Qt.exit(0);
        } catch (error) {
            console.warn(`FAIL ${error}`);
            Qt.exit(1);
        }
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function presence(id, address, more) {
        return Object.assign({ session: id, addresses: [address], cwd: `/home/joel/src/${id}` },
            more ?? {});
    }

    function held() {
        const at = "2026-11-08T01:00:00Z";
        return {
            "session:s-busy": { state: "busy", standing: "running", here: true, at,
                window: { id: 7 }, presence: presence("s-busy", "A001", { busy: true,
                    questInfo: { id: "QEST-1", title: "Lights", short: "AAA111", status: "active" },
                    orchestratedBy: "P9R1KI",
                    stepDetail: { kind: "tool", name: "bash", detail: "nix fmt", since: at },
                    usage: { "gen_ai.request.model": "claude", last: { "gen_ai.usage.input_tokens": 2 },
                        "pi.cost.usd": { last: 0.05, today: 1.25 } } }) },
            "session:s-asks": { state: "idle", standing: "running", here: true, at,
                presence: presence("s-asks", "A002", { wants: [{ kind: "question" }] }) },
            "session:s-away": { state: "idle", standing: "running", here: false,
                machine: "lantern", at, attach: ["kitty"], presence: presence("s-away", "A003") },
            "session:s-out": { state: "unreachable", standing: "running", here: false,
                machine: "lantern", at, attach: ["kitty"], presence: presence("s-out", "A004") },
            "session:s-old": { state: "idle", here: false, machine: "lantern", at,
                presence: presence("s-old", "A005") },
            "session:s-nap": { state: "ended", standing: "parked", here: true, at,
                presence: presence("s-nap", "A006", { state: "parked" }) },
            "session:s-done": { state: "ended", standing: "closed", here: true, at,
                presence: presence("s-done", "A008") },
            "session:s-mute": { state: "busy", standing: "running", here: true, at,
                presence: { session: "s-mute" } }
        };
    }

    function run() {
        const sorted = Pi.sorted(held());
        const ids = list => list.map(one => one.file).join(" ");
        expect("those the node says run are kept, here and elsewhere",
            ids(sorted.live) === "s-asks s-away s-busy s-out", ids(sorted.live));
        expect("those it says are parked rest",
            ids(sorted.resting) === "s-nap", ids(sorted.resting));
        expect("those it says closed are ending",
            ids(sorted.ending) === "s-done", ids(sorted.ending));
        const out = sorted.live.find(one => one.file === "s-out");
        expect("one out of reach is kept, saying so, with nothing to join it by",
            out.reachable === false && out.attach === null && out.here === false,
            JSON.stringify(out));
        const away = sorted.live.find(one => one.file === "s-away");
        expect("one elsewhere says where, how to join it and when it changed",
            away.machine === "lantern" && away.attach[0] === "kitty"
                && away.changed === Date.parse("2026-11-08T01:00:00Z"), JSON.stringify(away));

        // One pi's host runs here keeps the words the node gives to view
        // it, which Go To and Reopen Them All open its viewer by.
        const viewing = ["mycelium", "--realm", "work", "attach", "--session", "s-host"];
        const hosted = Pi.sorted({ "session:s-host": { state: "idle", standing: "running",
            here: true, at: "2026-11-08T01:00:00Z", attach: viewing,
            presence: presence("s-host", "A009", { hosted: { host: "work-background",
                conversation: "s-host" } }) } }).live[0];
        expect("one hosted here keeps the words to view it",
            JSON.stringify(hosted.attach) === JSON.stringify(viewing),
            JSON.stringify(hosted));
        // And asleep in the host, since viewing it is what wakes it there.
        const sleeping = Pi.sorted({ "session:s-host": { state: "ended", standing: "parked",
            here: true, at: "2026-11-08T01:00:00Z", attach: viewing,
            presence: presence("s-host", "A009", { state: "parked",
                hosted: { host: "work-background", conversation: "s-host" } }) } }).resting[0];
        expect("one asleep in the host keeps the words to view it",
            JSON.stringify(sleeping?.attach) === JSON.stringify(viewing),
            JSON.stringify(sleeping));

        const records = Pi.sessions(sorted, "skiff");
        const of = id => records.find(one => one["gen_ai.conversation.id"] === id);
        const states = ["s-busy", "s-asks", "s-away", "s-out", "s-nap", "s-done"]
            .map(id => of(id).state).join(" ");
        expect("each says how it stands in the contract's words",
            states === "working needs listening unreachable parked closed", states);
        const busy = of("s-busy");
        expect("a record keeps its key as Agents' rows do, and names pi as its agent",
            busy.key === "session:s-busy" && busy["gen_ai.agent.name"] === "pi"
                && busy.address === "A001" && busy.machine === "skiff" && busy.here === true,
            JSON.stringify(busy));
        expect("its step, usage and cost are pi's, by OpenTelemetry's names",
            busy.step.name === "bash" && busy.usage["gen_ai.request.model"] === "claude"
                && busy.usage.last["gen_ai.usage.input_tokens"] === 2
                && busy["cost.usd"].today === 1.25, JSON.stringify(busy));
        expect("what only pi has is under pi's name",
            busy["pi.quest"].short === "AAA111" && busy["pi.orchestratedBy"] === "P9R1KI"
                && busy.title === "Lights", JSON.stringify(busy));
        const said = of("s-away");
        expect("what pi didn't say is left out, never guessed",
            !("title" in said) && !("step" in said) && !("usage" in said)
                && !("cost.usd" in said) && !("pi.quest" in said) && said.machine === "lantern",
            JSON.stringify(said));

        // A change the node tells of re-places only the sessions it names:
        // the rest keep their entries and records as the same objects, so
        // whatever compares them in place stops at once, and what's sorted
        // is what sorting everything again would say.
        const now = held();
        const kept = {};
        const made = {};
        const first = Pi.resorted(now, kept, Object.keys(now));
        const firstRecords = Pi.sessions(first, "skiff", made);
        now["session:s-busy"] = Object.assign({}, now["session:s-busy"], {
            presence: presence("s-busy", "A001", { stepDetail: { kind: "tool", name: "edit" } }) });
        delete now["session:s-done"];
        now["session:s-new"] = { state: "idle", standing: "running", here: true,
            at: "2026-11-08T01:00:00Z", presence: presence("s-new", "A010") };
        const again = Pi.resorted(now, kept, ["session:s-busy", "session:s-done",
            "session:s-new"]);
        expect("what's re-placed by the keys heard sorts as everything sorted again",
            Pi.alike(again, Pi.sorted(now)), `${JSON.stringify(again)} against ${
                JSON.stringify(Pi.sorted(now))}`);
        const entry = (sorting, id) => sorting.live.concat(sorting.resting, sorting.ending)
            .find(one => one.file === id);
        const untouched = ["s-asks", "s-away", "s-out", "s-nap"];
        const placedAgain = untouched.filter(id => entry(again, id) !== entry(first, id));
        expect("a session the change didn't name keeps its entry",
            placedAgain.length === 0, placedAgain.join(" "));
        const againRecords = Pi.sessions(again, "skiff", made);
        const record = (records, id) => records.find(one => one["gen_ai.conversation.id"] === id);
        const remade = untouched.filter(id =>
            record(againRecords, id) !== record(firstRecords, id));
        expect("a session the change didn't name keeps its record",
            remade.length === 0, remade.join(" "));
        expect("a session it named is said anew, and one gone is let go",
            record(againRecords, "s-busy").step.name === "edit"
                && record(againRecords, "s-new") !== undefined
                && !("s-done" in made), JSON.stringify(Object.keys(made)));

        // What Sessions.qml makes of each entry is kept while the entry is
        // the same object, so a change makes copies only of what it named:
        // a window moved comes as a new entry, so its copy is new too, with
        // the new window, and one gone is let go.
        let makes = 0;
        const copied = one => {
            makes += 1;
            return { file: one.file, window: one.window?.id ?? null };
        };
        const copies = {};
        const before = Pi.keptBy(first.live, copies, copied);
        const making = makes;
        const moved = Object.assign({}, entry(first, "s-asks"), { window: { id: 9 } });
        const after = Pi.keptBy(first.live.filter(one => one.file !== "s-away")
            .map(one => one.file === "s-asks" ? moved : one), copies, copied);
        const copyOf = (list, id) => list.find(one => one.file === id);
        expect("a copy is kept while its entry stands",
            copyOf(after, "s-busy") === copyOf(before, "s-busy")
                && copyOf(after, "s-out") === copyOf(before, "s-out") && makes === making + 1,
            `${makes - making} made again`);
        expect("an entry made anew is copied anew, as it now stands",
            copyOf(after, "s-asks") !== copyOf(before, "s-asks")
                && copyOf(after, "s-asks").window === 9, JSON.stringify(copyOf(after, "s-asks")));
        expect("one gone is let go", !("s-away" in copies), JSON.stringify(Object.keys(copies)));

        // What the node tells as a session works on: a step moving alone
        // carries no new digest of what's read shut, and anything more does.
        const stepped = { id: 2, changed: [{ key: "session:s-busy", step: "edit",
            presence: { step: "edit" } }] };
        expect("with the launcher shut, a change carrying no new digest moves nothing",
            !Pi.movesShut(stepped), JSON.stringify(stepped));
        const wanting = { id: 2, changed: [{ key: "session:s-busy", told: "00ab",
            presence: { wants: [{ kind: "question" }] } }] };
        expect("one carrying a new digest moves what a shut launcher reads",
            Pi.movesShut(wanting), JSON.stringify(wanting));
        expect("as one gone does",
            Pi.movesShut({ id: 2, changed: [], gone: ["session:s-done"] }), "it moved nothing");
        expect("and a whole answer",
            Pi.movesShut({ id: "all", records: [] }), "it moved nothing");

        // What Sessions.qml keeps each list by, said as JSON would say it.
        const pairs = [[{ a: 1, b: [1, { c: "x" }] }, { b: [1, { c: "x" }], a: 1 }, true],
            [{ a: 1, b: undefined }, { a: 1 }, true], [{ a: null }, {}, false],
            [[1, 2], [1, 2, 3], false], [[], {}, false], [{ a: [1] }, { a: [2] }, false],
            ["1", 1, false], [{ a: { b: 1 } }, { a: { b: 1, c: 2 } }, false]];
        const missaid = pairs.filter(([before, after, same]) => Pi.alike(before, after) !== same
            || Pi.alike(after, before) !== same);
        expect("lists and records are alike only where their JSON would be",
            missaid.length === 0, JSON.stringify(missaid));

        const states2 = [null, { state: "starting" }, { state: "not answering" },
            { state: "running" }].map(link => Pi.state(link)).join(" ");
        expect("the source says whether it answers",
            states2 === "quiet quiet lost answering", states2);
    }
}
