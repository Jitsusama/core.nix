// How the launcher says where a pi session stands and lays a list of them
// out, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-lays-sessions-under-their-orchestrators` check copies
// modules/quickshell/launcher/SessionRow.js beside this file and runs
// `qml -platform offscreen session-row.qml`. Each list of sessions keeps
// one order of how they stand (needs you, working, in line, listening,
// paused), and Agents sits each session under the one orchestrating it, a
// step in, an orchestration standing where its most urgent row would
// (QEST-20261026-15J7N3's agents-tree-look.md).
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
            ["none at all is off", null, false, false, "off"]
        ];
        for (const [what, presence, asks, queued, want] of states) {
            const got = SessionRow.state(presence, asks, queued);
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        const order = ["paused", "listening", "off", "inLine", "needs", "working"]
            .sort((a, b) => SessionRow.rank(a) - SessionRow.rank(b)).join(" ");
        if (order !== "needs working inLine listening paused off")
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
        // be stopped, one paused resumed.
        const acts = [
            ["working", true, "switch talk pause stop"],
            ["inLine", false, "talk pause stop"],
            ["listening", true, "switch talk pause"],
            ["needs", true, "switch talk pause"],
            ["paused", true, "switch talk resume"]
        ];
        for (const [state, windowed, want] of acts) {
            const got = SessionRow.acts(state, windowed).join(" ");
            if (got !== want)
                failures.push(`the acts of one ${state}: "${got}", not "${want}"`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
