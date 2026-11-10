// How pi, as an agent source, sorts and says its sessions, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `pi-tells-of-its-sessions-as-an-agent-source` check copies
// modules/quickshell/PiSource.js beside this file and runs
// `qml -platform offscreen pi-source.qml`, over session records as the
// realm's mycelium node serves them. Those running are kept, here and on
// another machine heard of this boot; one parked is parked only while its
// marker stands and its boot does; one that ended this boot is closed;
// anything older, or with no address, is passed over. Each is said as the
// contract's record, with OpenTelemetry's gen_ai names and pi's own under
// `pi.`, leaving out what pi didn't say. With the launcher shut, a change
// only to what the launcher alone shows tells nothing new. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
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

    readonly property real bootedAt: Date.parse("2026-11-08T00:00:00Z")
    readonly property string thisBoot: "boot-now"

    function presence(id, address, more) {
        return Object.assign({ session: id, addresses: [address], cwd: `/home/joel/src/${id}` },
            more ?? {});
    }

    function held() {
        const at = "2026-11-08T01:00:00Z";
        const before = "2026-11-07T20:00:00Z";
        return {
            "session:s-busy": { state: "busy", here: true, at, window: { id: 7 },
                presence: presence("s-busy", "A001", { busy: true,
                    questInfo: { id: "QEST-1", title: "Lights", short: "AAA111", status: "active" },
                    orchestratedBy: "P9R1KI",
                    stepDetail: { kind: "tool", name: "bash", detail: "nix fmt", since: at },
                    usage: { "gen_ai.request.model": "claude", last: { "gen_ai.usage.input_tokens": 2 },
                        "pi.cost.usd": { last: 0.05, today: 1.25 } } }) },
            "session:s-asks": { state: "idle", here: true, at,
                presence: presence("s-asks", "A002", { wants: [{ kind: "question" }] }) },
            "session:s-away": { state: "idle", machine: "lantern", at, attach: ["kitty"],
                presence: presence("s-away", "A003") },
            "session:s-out": { state: "unreachable", machine: "lantern", at, attach: ["kitty"],
                presence: presence("s-out", "A004") },
            "session:s-old": { state: "idle", machine: "lantern", at: before,
                presence: presence("s-old", "A005") },
            "session:s-nap": { state: "ended", here: true, at,
                presence: presence("s-nap", "A006", { state: "parked",
                    owner: { bootId: thisBoot } }) },
            "session:s-stale": { state: "ended", here: true, at,
                presence: presence("s-stale", "A007", { state: "parked",
                    owner: { bootId: "boot-gone" } }) },
            "session:s-done": { state: "ended", here: true, at,
                presence: presence("s-done", "A008") },
            "session:s-long": { state: "ended", here: true, at: before,
                presence: presence("s-long", "A009") },
            "session:s-mute": { state: "busy", here: true, at, presence: { session: "s-mute" } }
        };
    }

    function run() {
        const place = { machine: "skiff", bootId: thisBoot, bootedAt,
            markers: { "s-nap": true, "s-stale": true } };
        const sorted = Pi.sorted(held(), place);
        const ids = list => list.map(one => one.file).join(" ");
        expect("those running are kept, here and elsewhere this boot",
            ids(sorted.live) === "s-asks s-away s-busy s-out", ids(sorted.live));
        expect("one parked stands only while its marker and boot do",
            ids(sorted.resting) === "s-nap", ids(sorted.resting));
        expect("one that ended this boot, or parked before it, is closed",
            ids(sorted.ending) === "s-done s-stale", ids(sorted.ending));
        const out = sorted.live.find(one => one.file === "s-out");
        expect("one out of reach is kept, saying so, with nothing to join it by",
            out.reachable === false && out.attach === null && out.here === false,
            JSON.stringify(out));

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
        const away = of("s-away");
        expect("what pi didn't say is left out, never guessed",
            !("title" in away) && !("step" in away) && !("usage" in away)
                && !("cost.usd" in away) && !("pi.quest" in away) && away.machine === "lantern",
            JSON.stringify(away));

        // A session working on: its step, usage and when it was heard move,
        // as most of Joel's sessions' changes do, then it stops being busy.
        const working = held();
        const busyAt = working["session:s-busy"];
        busyAt.presence = Object.assign({}, busyAt.presence, {
            at: "2026-11-08T01:00:09Z", heard: "2026-11-08T01:00:09Z", step: "edit",
            stepDetail: { kind: "tool", name: "edit", detail: "PiSource.js" },
            usage: { last: { "gen_ai.usage.input_tokens": 9 } }
        });
        // One elsewhere working on moves its record's own time with its step.
        const awayAt = working["session:s-away"];
        awayAt.at = "2026-11-08T01:00:09Z";
        awayAt.presence = Object.assign({}, awayAt.presence, { step: "read" });
        const moved = Pi.sorted(working, place);
        expect("with the launcher shut, a step moving alone tells nothing new",
            Pi.tellsAlike(sorted, moved, false), JSON.stringify(moved));
        expect("with the launcher showing it, a step moving tells it",
            !Pi.tellsAlike(sorted, moved, true), JSON.stringify(moved));
        expect("a sorting tells the same as itself sorted again, shown or not",
            Pi.tellsAlike(sorted, Pi.sorted(held(), place), true)
                && Pi.tellsAlike(sorted, Pi.sorted(held(), place), false), "it told a change");
        // A field said in another order, or one left undefined, says the
        // same, as the text compared before did.
        const reordered = Pi.sorted(held(), place);
        reordered.live[0] = Object.assign({ window: reordered.live[0].window },
            reordered.live[0], { unsaid: undefined });
        expect("a field in another order or left undefined tells nothing new",
            Pi.tellsAlike(sorted, reordered, true), JSON.stringify(reordered.live[0]));
        // What Sessions.qml keeps each list by, said as JSON would say it.
        const said = [[{ a: 1, b: [1, { c: "x" }] }, { b: [1, { c: "x" }], a: 1 }, true],
            [{ a: 1, b: undefined }, { a: 1 }, true], [{ a: null }, {}, false],
            [[1, 2], [1, 2, 3], false], [[], {}, false], [{ a: [1] }, { a: [2] }, false],
            ["1", 1, false], [{ a: { b: 1 } }, { a: { b: 1, c: 2 } }, false]];
        const missaid = said.filter(([before, after, same]) => Pi.alike(before, after) !== same
            || Pi.alike(after, before) !== same);
        expect("lists and records are alike only where their JSON would be",
            missaid.length === 0, JSON.stringify(missaid));
        // It stops working and listens, as 14 in 61 of Joel's sessions'
        // changes did, which only the launcher says.
        busyAt.state = "idle";
        busyAt.presence = Object.assign({}, busyAt.presence, { busy: false, state: "listening",
            since: "2026-11-08T01:00:10Z" });
        const listening = Pi.sorted(working, place);
        expect("with the launcher shut, a session moving to listening tells nothing new",
            Pi.tellsAlike(sorted, listening, false), JSON.stringify(listening));
        expect("with the launcher showing it, a session moving to listening tells it",
            !Pi.tellsAlike(moved, listening, true), JSON.stringify(listening));
        busyAt.presence = Object.assign({}, busyAt.presence,
            { wants: [{ kind: "question" }] });
        expect("with the launcher shut, a session coming to want Joel still tells it",
            !Pi.tellsAlike(sorted, Pi.sorted(working, place), false),
            JSON.stringify(Pi.sorted(working, place)));
        expect("what's told never changes what's held",
            Pi.sorted(working, place).live.find(one => one.file === "s-busy")
                .presence.stepDetail.name === "edit", "its step was taken out");
        // It parks, wanting nothing, so only its standing and its list move.
        busyAt.state = "ended";
        busyAt.presence = Object.assign({}, busyAt.presence, { state: "parked",
            owner: { bootId: thisBoot } });
        delete busyAt.presence.wants;
        const parking = Pi.sorted(working, Object.assign({}, place,
            { markers: Object.assign({ "s-busy": true }, place.markers) }));
        expect("with the launcher shut, a session parking still tells it",
            !Pi.tellsAlike(sorted, parking, false), JSON.stringify(parking));

        const states2 = [null, { state: "starting" }, { state: "not answering" },
            { state: "running" }].map(link => Pi.state(link)).join(" ");
        expect("the source says whether it answers",
            states2 === "quiet quiet lost answering", states2);
    }
}
