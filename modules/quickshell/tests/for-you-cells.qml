// How For You stands what needs Joel under the cell behind each, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `for-you-stands-each-under-its-cell` check copies
// modules/quickshell/launcher/ForYou.js, ForYouCells.js and Order.js beside
// this file and runs `qml -platform offscreen for-you-cells.qml`. Each case
// is one of What Needs Me's rules (42P0R4 and Y2YBTS, 2026-11-18): a cell's
// rows under it, the cells by their heaviest, what a cell asks leading it;
// each heading in Agents' words for its cell; a session's own notices found
// by the cell they name and standing under it; and Agents Asking above
// every heading while any shows.
import QtQuick
import "ForYou.js" as ForYou
import "ForYouCells.js" as Cells

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

    function run() {
        const failures = [];
        const expect = (what, got, wanted) => {
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };
        const hour = 3600 * 1000;
        const now = Date.parse("2026-11-08T14:00:00Z");

        // What needs Joel now, grouped by the cell behind each (What Needs
        // Me, 42P0R4 and Y2YBTS, 2026-11-18): a cell's rows under it in the
        // order they came, so heaviest first, the cells by their heaviest,
        // rows with no cell under one group of Joel's own, placed by its
        // heaviest as any cell is.
        const cellA = { address: "AAAAAA", title: "a" };
        const cellB = { address: "BBBBBB", title: "b" };
        const behind = { one: cellA, two: null, three: cellB, four: { address: "aaaaaa" },
            five: null };
        const grouped = Cells.byCell(["one", "two", "three", "four", "five"]
            .map((name, at) => ({ name, weight: 9 - 2 * at })), item => behind[item.name]);
        expect("each cell's rows under it, the cells by their heaviest",
            grouped.map(group => [group.cell?.address ?? null,
                group.items.map(item => item.name)]),
            [["AAAAAA", ["one", "four"]], [null, ["two", "five"]], ["BBBBBB", ["three"]]]);
        expect("a cell with no address is Joel's own",
            Cells.byCell([{ name: "one" }], () => ({ title: "x" }))
                .map(group => group.cell), [null]);
        expect("nothing, no groups", Cells.byCell(undefined, () => cellA), []);
        // What a cell asks leads it, whatever a look it left weighs, since
        // the ask is what holds the session up; the cells stay where their
        // heaviest put them.
        const led = Cells.byCell([{ name: "look", cell: cellA }, { name: "own", cell: null },
            { name: "ask", cell: cellA, asks: true }], item => item.cell,
        item => item.asks === true);
        expect("what a cell asks leads it, the rest in the order they came",
            led.map(group => group.items.map(item => item.name)), [["ask", "look"], ["own"]]);

        // The groups laid out, each one's rows under its heading, a group
        // whose rows all went as they were made left out, and no heading
        // while only Joel's own are left: decided before the rows were made,
        // his own heading stood alone once a cell's rows had all gone.
        const rowsOf = items => items.filter(item => item.name !== "gone")
            .map(item => item.name);
        const headingOf = cell => `# ${cell?.address ?? "yours"}`;
        const laid = groups => Cells.laidOut(groups, rowsOf, headingOf);
        expect("each cell's heading over its rows, Joel's own too",
            laid(grouped), ["# AAAAAA", "one", "four", "# yours", "two", "five", "# BBBBBB",
                "three"]);
        expect("a lone cell is still named", laid(Cells.byCell([{ name: "one" }], () => cellA)),
            ["# AAAAAA", "one"]);
        expect("only Joel's own, no heading over them",
            laid(Cells.byCell([{ name: "two" }, { name: "five" }], () => null)), ["two", "five"]);
        expect("a cell whose rows all went says nothing, and leaves Joel's own unheaded",
            laid(Cells.byCell([{ name: "gone" }, { name: "two" }],
                item => item.name === "gone" ? cellA : null)), ["two"]);
        expect("nothing laid out, nothing", laid([]), []);

        // A heading says its cell as Agents' row for it does, its name and
        // how the agents daemon says it stands, never words made here; its
        // title alone while the daemon says neither. A title is filled in
        // once, as it reads: `$&` and `{doing}` in it stay as they are.
        const said = { cell: "{title} \u00b7 {doing}", yours: "yours alone" };
        expect("a cell's heading, its served name and how it stands",
            Cells.headingName(cellA, { name: "Served A", doing: "needs you" }, said),
            "Served A \u00b7 needs you");
        expect("no row served, its title alone", Cells.headingName(cellA, null, said), "a");
        expect("no title, its address",
            Cells.headingName({ address: "cccccc" }, { doing: "" }, said), "CCCCCC");
        expect("Joel's own heading", Cells.headingName(null, null, said), "yours alone");
        expect("a title filled in as it reads",
            Cells.headingName(cellA, { name: "$& and {doing}", doing: "working" }, said),
            "$& and {doing} \u00b7 working");
        expect("no words, nothing made up", Cells.headingName(cellA, { doing: "working" }, {}),
            "");

        // A desktop notice a session raised names its cell as a link, which
        // finds the session among those held by any address it answers to,
        // so the notice stands under that session's heading; one naming no
        // cell, or one no session holds, finds none.
        const raising = { address: "KELP01", presence: { addresses: ["KELP01", "9F3A2B1C"] } };
        const held = [cellB, raising];
        expect("a notice finds its session by any address it answers to",
            Cells.noticeCell({ cell: "mycelium:cell/9F3A2B1C@heron#notice-4" }, held), raising);
        expect("by its own address, whatever the case, its anchor alone after it",
            Cells.noticeCell({ cell: "mycelium:cell/bbbbbb#done" }, held), cellB);
        expect("by an address alone", Cells.noticeCell({ cell: "mycelium:cell/BBBBBB" }, held),
            cellB);
        expect("not an address another runs on from",
            Cells.noticeCell({ cell: "mycelium:cell/BBBBBB0" }, held), null);
        expect("no cell named, none", Cells.noticeCell({ cell: null }, held), null);
        expect("not a cell's link, none", Cells.noticeCell({ cell: "https://x/BBBBBB" }, held),
            null);
        expect("a session no longer held, none",
            Cells.noticeCell({ cell: "mycelium:cell/GONE0001" }, held), null);
        expect("nothing held, none", Cells.noticeCell({ cell: "mycelium:cell/BBBBBB" }, undefined),
            null);
        expect("the address a notice names, whatever is held",
            [Cells.cellNamed({ cell: "mycelium:cell/9f3a2b1c#notice-4" }),
                Cells.cellNamed({ cell: "https://x/BBBBBB" }), Cells.cellNamed({})],
            ["9F3A2B1C", "", ""]);
        // Those that join their cell: pi's, unread, kept, from a session
        // held, and not of a cell wanting Joel, whose want's row already
        // says what it asks, nor of one whose want he put off, nor a low
        // one, as away mode's saying a session rests, which asks nothing.
        const raised = (id, more) => Object.assign({ id, app: "pi", read: false,
            cell: "mycelium:cell/9F3A2B1C" }, more);
        expect("pi's, unread, kept and from a session held not wanting Joel, each joins it",
            Cells.joining([raised(1), raised(2, { read: true }), raised(3, { cell: null }),
                raised(4, { transient: true }), raised(5, { cell: "mycelium:cell/BBBBBB" }),
                raised(6, { app: "Slack" }), raised(7, { cell: "mycelium:cell/GONE0001" }),
                raised(8, { urgency: "low" })],
            held, [cellB], {}).map(one => one.id), [1]);
        expect("one whose want Joel put off joins nothing",
            Cells.joining([raised(1)], held, [], { KELP01: true }).length, 0);
        expect("nothing told, none", Cells.joining(undefined, undefined, undefined, undefined),
            []);
        // The list kept while it holds the same notices saying the same, so
        // one said anew, or read since For You opened, makes nothing again;
        // one more, one fewer, another order or new words in one is new.
        const keptOnes = [raised(1), raised(2)];
        expect("the same notices said anew keep the list",
            Cells.alike(keptOnes, [raised(1), raised(2)]) === keptOnes, true);
        expect("one read since For You opened keeps it",
            Cells.alike(keptOnes, [raised(1, { read: true }), raised(2)]) === keptOnes, true);
        const another = [raised(1), raised(3)];
        expect("another notice makes it anew", Cells.alike(keptOnes, another) === another, true);
        const fewer = [raised(1)];
        expect("one fewer makes it anew", Cells.alike(keptOnes, fewer) === fewer, true);
        const turned = [raised(2), raised(1)];
        expect("another order makes it anew", Cells.alike(keptOnes, turned) === turned, true);
        const reworded = [raised(1, { summary: "Run ended" }), raised(2)];
        expect("one replaced with new words makes it anew",
            Cells.alike(keptOnes, reworded) === reworded, true);
        expect("nothing kept yet, what joined", Cells.alike(undefined, another) === another,
            true);
        // It stands among what needs Joel now, waiting since it came, so
        // under its heading it weighs as long as it's waited and no more.
        const noticed = ForYou.banded([{ kind: "notice",
            thing: Cells.noticeThing({ at: now - hour }) }], now);
        expect("a session's notice needs Joel now, weighed as it's waited",
            [noticed.now.length, noticed.now[0]?.weight],
            [1, ForYou.weigh({ since: now - hour }, now).weight]);

        // Agents Asking stands first, above every heading, while any shows,
        // since a heading claims what's under it; with none, before the first
        // loose row that weighs less, as agentAt places it (Y2YBTS,
        // 2026-11-18 15:49).
        const asking = weight => ({ rows: ["ASKED"], weight });
        expect("headings shown, it stands first, though lighter than all",
            Cells.laidOut(grouped, rowsOf, headingOf, asking(0.5)), ["ASKED", "# AAAAAA", "one",
                "four", "# yours", "two", "five", "# BBBBBB", "three"]);
        const weighs = [{ name: "two", weight: 9 }, { name: "five", weight: 2 }];
        const ownOnly = Cells.byCell(weighs, () => null);
        expect("no heading, before the first lighter row",
            Cells.laidOut(ownOnly, rowsOf, headingOf, asking(5)), ["two", "ASKED", "five"]);
        expect("no heading, heaviest of all, first",
            Cells.laidOut(ownOnly, rowsOf, headingOf, asking(10)), ["ASKED", "two", "five"]);
        expect("no heading, lightest, last",
            Cells.laidOut(ownOnly, rowsOf, headingOf, asking(1)), ["two", "five", "ASKED"]);
        expect("a cell whose rows all went leaves no heading, so it goes by weight",
            Cells.laidOut(Cells.byCell([{ name: "gone", weight: 9 }, weighs[1]],
                item => item.name === "gone" ? cellA : null), rowsOf, headingOf, asking(1)),
            ["five", "ASKED"]);
        expect("nothing else, it alone", Cells.laidOut([], rowsOf, headingOf, asking(3)),
            ["ASKED"]);
        // With no heading it stands where banded's agentAt puts it, a row
        // weighing the same before it, so For You and its :filter, which
        // reads agentAt, list in one order.
        const ask = name => ({ kind: "agent", thing: { kind: "todo", blocks: 1,
            since: now - hour }, name });
        const tied = ForYou.banded([ask("a"), ask("b"), { kind: "do", thing: { kind: "todo",
            blocks: 1, since: now - hour }, name: "tie" }, { kind: "do",
            thing: { kind: "todo" }, name: "light" }], now);
        const tiedNames = tied.now.map(one => one.name);
        expect("a tie stands before it, where agentAt puts it",
            [Cells.laidOut(Cells.byCell(tied.now, () => null), rowsOf, headingOf,
                { rows: ["ASKED"], weight: tied.agent[0].weight }),
                tiedNames.slice(0, tied.agentAt).concat(["ASKED"], tiedNames.slice(tied.agentAt))],
            [["tie", "ASKED", "light"], ["tie", "ASKED", "light"]]);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            // Qt.exit only asks the engine to stop once this returns.
            Qt.exit(1);
            return;
        }
        console.info("For You stands each under its cell");
        Qt.exit(0);
    }
}
