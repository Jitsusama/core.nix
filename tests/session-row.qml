// How the launcher says where a pi session stands and lays a list of them
// out, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-lays-sessions-under-their-orchestrators` check copies
// modules/quickshell/launcher/SessionRow.js beside this file and runs
// `qml -platform offscreen session-row.qml`. Each list of sessions keeps
// one order of how they stand (needs you, working, in line, listening,
// paused, parked), and Agents sits each session under the one orchestrating it, a
// step in, an orchestration standing where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md). A row says how it stands
// and what it spent at its right, and its drawer the rest.
import QtQuick
import "SessionRow.js" as SessionRow

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // Sessions said as "address:state>orchestrator", in the order Sessions
    // keeps them: "B:working>A" is B, working, orchestrated by A.
    function sessions(said) {
        return said.split(" ").filter(one => one !== "").map(one => {
            const [own, under] = one.split(">");
            const [address, state] = own.split(":");
            return { address: address, state: state, under: under ?? "" };
        });
    }

    // The laid-out list as it reads: each address, a dot for each step in.
    function read(list, laid) {
        return laid.map(one => `${".".repeat(one.depth)}${list[one.at].address}`).join(" ");
    }

    function run() {
        const failures = [];

        // [what, its presence, whether its want is Joel's, whether it waits
        // its turn in line, how it stands]
        const states = [
            ["one wanting Joel needs him", { busy: true, wants: { kind: "question" } }, true,
                false, "needs"],
            ["a want that isn't his reads as what it does", { busy: true, wants: {} }, false,
                false, "working"],
            ["one at work is working", { busy: true }, false, false, "working"],
            ["one waiting its turn is in line", { busy: true }, false, true, "inLine"],
            ["one idle is listening, never waiting", { busy: false }, false, false,
                "listening"],
            ["one the harness says is waiting is still only listening",
                { busy: false, state: "waiting" }, false, false, "listening"],
            ["one paused is paused", { state: "paused", busy: false }, false, true, "paused"],
            ["one paused that wants Joel needs him", { state: "paused" }, true, false, "needs"],
            ["one parked is parked, never listening", { state: "parked", busy: false }, false,
                false, "parked"],
            ["none at all is off", null, false, false, "off"]
        ];
        for (const [what, presence, asks, queued, want] of states) {
            const got = SessionRow.state(presence, asks, queued);
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        const order = ["paused", "parked", "listening", "off", "inLine", "needs", "working"]
            .sort((a, b) => SessionRow.rank(a) - SessionRow.rank(b)).join(" ");
        if (order !== "needs working inLine listening paused parked off")
            failures.push(`the one order of states: "${order}"`);

        // [what, the sessions in Sessions' order, how they're laid out]
        const trees = [
            ["sessions alone keep the order of how they stand",
                "A:listening B:needs C:working D:paused E:inLine",
                "B C E A D"],
            ["and within a state the order they came in", "A:working B:working", "A B"],
            ["a session follows its orchestrator, a step in",
                "A:working B:working C:working>A", "A .C B"],
            ["its orchestrator's orchestrator two steps in",
                "P:listening O:working>P B:working>O", "P .O ..B"],
            ["and never deeper than two", "P:working O:working>P B:working>O C:working>B",
                "P .O ..B ..C"],
            ["rows under one orchestrator keep the order of how they stand",
                "O:working A:listening>O B:needs>O C:working>O", "O .B .C .A"],
            ["one parked sits under its orchestrator, last",
                "O:parked A:parked>O B:listening>O X:paused", "O .B .A X"],
            ["an orchestration stands where its most urgent row would",
                "X:working O:listening A:needs>O", "O .A X"],
            ["and so does one under it, with its own",
                "O:working P:listening>O Q:working>O B:needs>P", "O .P ..B .Q"],
            ["one whose orchestrator isn't listed stands at the top, not in",
                "A:working B:working>GONE", "A B"],
            ["one naming itself is its own", "A:working>A", "A"],
            ["orchestrators naming each other are cut where they meet",
                "A:working>B B:working>A", "A .B"]
        ];
        for (const [what, said, want] of trees) {
            const list = sessions(said);
            const got = read(list, SessionRow.tree(list));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // Whom each row is under, and whether it orchestrates one listed.
        const list = sessions("O:working A:working>O B:working>GONE");
        const laid = SessionRow.tree(list);
        const said = laid.map(one => `${list[one.at].address}<${one.under}${one.orchestrates
            ? "*" : ""}`).join(" ");
        if (said !== "O<* A<O B<")
            failures.push(`whom each is under: "${said}"`);

        // A session's acts follow how it stands: one at work or in line can
        // be stopped, one paused resumed, one parked only woken or talked
        // to, with no window known to go to.
        const acts = [
            ["working", true, "switch talk pause stop"],
            ["inLine", false, "talk pause stop"],
            ["listening", true, "switch talk pause"],
            ["needs", true, "switch talk pause"],
            ["paused", true, "switch talk resume"],
            ["parked", false, "wake talk"]
        ];
        for (const [state, windowed, want] of acts) {
            const got = SessionRow.acts(state, windowed).join(" ");
            if (got !== want)
                failures.push(`the acts of one ${state}: "${got}", not "${want}"`);
        }

        // A row says how it stands and what it spent today at its right,
        // the spend kept longest, and nothing more: its address and folder
        // are its drawer's (VKCF3Z, 2026-11-06: "working \u00b7 $4.20").
        const side = items => items.map(item => `${item.said}/${item.keep}`).join(" ");
        const sides = [
            ["its state, then its spend, kept longest", "working", "$4.20",
                "working/3 $4.20/4"],
            ["one that has spent nothing says its state alone", "listening", "",
                "listening/3"],
            ["one off says only what it spent", "", "$1", "$1/4"]
        ];
        for (const [what, doing, spent, want] of sides) {
            const got = side(SessionRow.side(doing, spent));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // Its drawer's eyebrow says its address, where it works and since
        // when, and its facts don't say the address again.
        const told = {
            address: "8IDU4E",
            place: "core.nix",
            since: "since 21:52",
            machine: "",
            spent: "$4.20",
            named: id => id
        };
        const drawn = SessionRow.drawer("A Quest", { worksOn: "the row" }, told);
        if (drawn.eyebrow !== "8IDU4E \u00b7 core.nix \u00b7 since 21:52")
            failures.push(`the drawer's eyebrow: "${drawn.eyebrow}"`);
        const facts = drawn.facts.map(fact => fact.label).join(" ");
        if (facts !== "works spent")
            failures.push(`the drawer's facts: "${facts}", not "works spent"`);
        // One whose record names no place, nor when it began, says its
        // address alone.
        const bare = SessionRow.drawer("A Quest", {}, Object.assign({}, told,
            { place: "", since: "" }));
        if (bare.eyebrow !== "8IDU4E")
            failures.push(`a bare drawer's eyebrow: "${bare.eyebrow}"`);

        // Return does the act its state makes harmless, always one of its
        // acts: Go To one at work, Talk to one listening, Resume one paused
        // and Wake one parked; with no window to go to, Talk.
        const returns = [
            ["working", true, "switch"],
            ["inLine", true, "switch"],
            ["needs", true, "switch"],
            ["working", false, "talk"],
            ["listening", true, "talk"],
            ["paused", true, "resume"],
            ["paused", false, "resume"],
            ["parked", false, "wake"]
        ];
        for (const [state, windowed, want] of returns) {
            const got = SessionRow.returns(state, windowed);
            if (got !== want || !SessionRow.acts(state, windowed).includes(got))
                failures.push(`Return on one ${state}${windowed ? "" : " unwindowed"}: `
                    + `"${got}", not "${want}"`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
