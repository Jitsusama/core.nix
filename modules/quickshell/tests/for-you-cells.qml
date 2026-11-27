// How For You stands what needs Joel under the cell behind each, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `for-you-stands-each-under-its-cell` check copies
// modules/quickshell/launcher/ForYou.js, ForYouCells.js and Order.js beside
// this file and runs `qml -platform offscreen for-you-cells.qml`. Each case
// is one of What Needs Me's rules (42P0R4 and Y2YBTS, 2026-11-18): a cell's
// rows under it, the cells by their heaviest, what a cell asks leading it;
// each heading in Agents' words for its cell; a session's own notices found
// by the cell they name and standing under it; and every cell's heading
// and rows inside Agents, never at For You's top (Joel, 2026-11-25).
// The case "no session's heading or row stands at For You's top, only Agents
// among his own" is the one that says so.
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
        // What leads For You's rows of what needs him, read without making
        // any, as laidOut lays them out, so home's row can name what
        // Return, Return reaches: his own where it weighs no less than any
        // cell's, else Agents, by the first cell's first (round 5, F43).
        const ownItem = { name: "own", weight: 5 };
        const askedItem = { name: "asked", weight: 5 };
        const lighterItem = { name: "lighter", weight: 3 };
        expect("his own leads where it weighs as much as a cell's",
            Cells.leading([{ cell: null, items: [ownItem] }, { cell: cellA, items: [askedItem] }]),
            { item: ownItem, gathered: false });
        expect("a cell's heavier, Agents leads, by the first cell's first",
            Cells.leading([{ cell: null, items: [lighterItem] },
                { cell: cellA, items: [askedItem] }]), { item: askedItem, gathered: true });
        expect("no cell, his own", Cells.leading([{ cell: null, items: [ownItem] }]),
            { item: ownItem, gathered: false });
        expect("nothing, nothing leads", Cells.leading([]), null);
        // A cell is weighed by its heaviest item, not the one it leads with:
        // byCell puts its ask first whatever it weighs, so an ask at nothing
        // beside a look it left at 5 outweighs his own at 4, as laidOut lays
        // Agents first (#6777's sixth review, F54).
        const askAtNothing = { name: "ask", weight: 0 };
        const leftLook = { name: "left", weight: 5 };
        const ownAtFour = { name: "own", weight: 4 };
        const heaviestBehind = [{ cell: null, items: [ownAtFour] },
            { cell: cellA, items: [askAtNothing, leftLook] }];
        expect("a cell weighs its heaviest, not the item it leads with",
            Cells.leading(heaviestBehind), { item: askAtNothing, gathered: true });
        expect("and laidOut lays Agents first there too",
            Cells.laidOut(heaviestBehind, items => items.map(item => item.name),
                cell => `cell ${cell.address}`, (inside, weight) => `agents ${weight}`)[0],
            "agents 5");
        // What a cell asks leads it, whatever a look it left weighs, since
        // the ask is what holds the session up; the cells stay where their
        // heaviest put them.
        const led = Cells.byCell([{ name: "look", cell: cellA }, { name: "own", cell: null },
            { name: "ask", cell: cellA, asks: true }], item => item.cell,
        item => item.asks === true);
        expect("what a cell asks leads it, the rest in the order they came",
            led.map(group => group.items.map(item => item.name)), [["ask", "look"], ["own"]]);

        // The groups laid out: Joel's own loose, and the cells' gathered into
        // one row, ASKED here, holding each one's heading over its rows. A
        // group whose rows all went as they were made is left out, and with
        // no cell's left nothing is gathered, so no heading stands alone.
        const rowsOf = items => items.filter(item => item.name !== "gone")
            .map(item => item.name);
        const headingOf = cell => `# ${cell?.address ?? "yours"}`;
        const gather = (inside, weight) => ({ asked: inside, weight });
        const laid = groups => Cells.laidOut(groups, rowsOf, headingOf, gather);
        expect("Joel's own loose, each cell's heading over its rows inside the one gathered",
            laid(grouped), [{ asked: ["# AAAAAA", "one", "four", "# BBBBBB", "three"],
                weight: 9 }, "two", "five"]);
        expect("a lone cell is still gathered and named",
            laid(Cells.byCell([{ name: "one" }], () => cellA)),
            [{ asked: ["# AAAAAA", "one"], weight: 0 }]);
        expect("only Joel's own, nothing gathered, no heading over them",
            laid(Cells.byCell([{ name: "two" }, { name: "five" }], () => null)), ["two", "five"]);
        expect("a cell whose rows all went gathers nothing, and leaves Joel's own unheaded",
            laid(Cells.byCell([{ name: "gone" }, { name: "two" }],
                item => item.name === "gone" ? cellA : null)), ["two"]);
        const top = laid(grouped).map(one => typeof one === "string" ? one : "ASKED");
        expect("no session's heading or row stands at For You's top, only Agents"
            + " among his own", [top.filter(one => one.startsWith("#")),
            top.filter(one => ["one", "three", "four"].includes(one)), top],
        [[], [], ["ASKED", "two", "five"]]);
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
            Cells.noticeCell({ cell: "mycelium:cell/9F3A2B1C@heron#notice:8" }, held), raising);
        expect("by its own address, as the daemon names it, its anchor alone after it",
            Cells.noticeCell({ cell: "mycelium:cell/BBBBBB#notice:12" }, held), cellB);
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
            [Cells.cellNamed({ cell: "mycelium:cell/9F3A2B1C#notice-4" }),
                Cells.cellNamed({ cell: "https://x/BBBBBB" }), Cells.cellNamed({})],
            ["9F3A2B1C", "", ""]);
        // Those that join their cell: pi's, unread, kept, from a session
        // held, and not of a cell wanting Joel, whose want's row already
        // says what it asks, nor of one whose want he put off, nor a low
        // one, as away mode's saying a session rests, which asks nothing,
        // nor one expired or withdrawn; one put away unread still joins
        // (below).
        const raised = (id, more) => Object.assign({ id, app: "pi", read: false,
            cell: "mycelium:cell/9F3A2B1C" }, more);
        expect("pi's, unread, not gone by and from a session held not wanting Joel, each joins it",
            Cells.joining([raised(1), raised(2, { read: true }), raised(3, { cell: null }),
                raised(4, { transient: true }), raised(5, { cell: "mycelium:cell/BBBBBB" }),
                raised(6, { app: "Slack" }), raised(7, { cell: "mycelium:cell/GONE0001" }),
                raised(8, { urgency: "low" }), raised(9, { left: "dismissed" }),
                raised(10, { left: "expired" }), raised(11, { left: "withdrawn" })],
            held, [cellB], { putOff: {} }).map(one => one.id), [1, 9]);
        expect("one whose want Joel put off joins nothing",
            Cells.joining([raised(1)], held, [], { putOff: { KELP01: true } }).length, 0);
        expect("another session's want put off, it still joins",
            Cells.joining([raised(1)], held, [], { putOff: { BBBBBB: true } }).length, 1);
        // Put away unread, it's still his to read, as Desktop holds it,
        // until its session withdraws it, which reads it and leaves it
        // dismissed (held.rs's close); gone by itself, it stays missed,
        // which nothing takes back.
        expect("put away unread, it stays; read, withdrawn or gone by itself, not",
            Cells.joining([raised(1, { left: "dismissed", leftAt: 10 }),
                raised(2, { left: "dismissed", leftAt: 3, read: true }),
                raised(3, { left: "expired", leftAt: 10 }),
                raised(4, { left: "dismissed", leftAt: 3, read: true, withdrawn: true,
                    readAt: 20 })], held, [], {})
                .map(one => one.id), [1]);
        expect("nothing told, none", Cells.joining(undefined, undefined, undefined, undefined),
            []);
        // The list kept while it holds the same notices saying the same, so
        // one said anew, or read since For You opened, as Lately.js's
        // holdingStill gives it again unread with its read stamped, makes
        // nothing again;
        // one more, one fewer, another order or new words in one is new.
        const keptOnes = [raised(1), raised(2)];
        expect("the same notices said anew keep the list",
            Cells.alike(keptOnes, [raised(1), raised(2)]) === keptOnes, true);
        expect("one read since For You opened keeps it",
            Cells.alike(keptOnes, [raised(1, { readHow: "read", readAt: 9 }), raised(2)])
                === keptOnes, true);
        const another = [raised(1), raised(3)];
        expect("another notice makes it anew", Cells.alike(keptOnes, another) === another, true);
        const fewer = [raised(1)];
        expect("one fewer makes it anew", Cells.alike(keptOnes, fewer) === fewer, true);
        const turned = [raised(2), raised(1)];
        expect("another order makes it anew", Cells.alike(keptOnes, turned) === turned, true);
        const reworded = [raised(1, { summary: "Run ended" }), raised(2)];
        expect("one replaced with new words makes it anew",
            Cells.alike(keptOnes, reworded) === reworded, true);
        const rebodied = [raised(1, { body: "new" }), raised(2)];
        expect("or a new body", Cells.alike(keptOnes, rebodied) === rebodied, true);
        const pressing = [raised(1, { urgency: "critical" }), raised(2)];
        expect("or another urgency", Cells.alike(keptOnes, pressing) === pressing, true);
        // Raised again once its card closed, it weighs from when it came
        // again; replaced in place, it keeps when it first came (held.rs's
        // arrive) but stands under whichever session it names now.
        const later = [raised(1, { at: 5 }), raised(2)];
        expect("one raised again later makes it anew",
            Cells.alike(keptOnes, later) === later, true);
        const moved = [raised(1, { cell: "mycelium:cell/BBBBBB" }), raised(2)];
        expect("one naming another session makes it anew",
            Cells.alike(keptOnes, moved) === moved, true);
        expect("nothing kept yet, what joined", Cells.alike(undefined, another) === another,
            true);
        // It stands among what needs Joel now, waiting since it came, so
        // under its heading it weighs as long as it's waited and no more.
        const noticed = ForYou.banded([{ kind: "notice",
            thing: Cells.noticeThing({ at: now - hour }) }], now);
        expect("a session's notice needs Joel now, weighed as it's waited",
            [noticed.now.length, noticed.now[0]?.weight],
            [1, ForYou.weigh({ since: now - hour }, now).weight]);

        // Agents stands among Joel's own before the first that weighs
        // less than the heaviest it holds, as agentAt places it (Y2YBTS,
        // 2026-11-18 15:49), however its cells' asks lead their rows.
        const own = [{ name: "two", weight: 9 }, { name: "five", weight: 2 }];
        const asking = (weight, also) => Cells.laidOut(Cells.byCell(own.concat([
            { name: "look", weight, cell: cellA }, { name: "ask", weight: 0, cell: cellA,
                asks: true }], also ?? []), item => item.cell ?? null, item => item.asks === true),
        rowsOf, headingOf, inside => `ASKED ${inside.join(" ")}`);
        expect("before the first lighter row, its ask leading its cell", asking(5),
            ["two", "ASKED # AAAAAA ask look", "five"]);
        expect("heaviest of all, first", asking(10), ["ASKED # AAAAAA ask look", "two", "five"]);
        expect("lightest, last", asking(1), ["two", "five", "ASKED # AAAAAA ask look"]);
        expect("a second cell inside it, after the first, by its heaviest",
            asking(5, [{ name: "three", weight: 4, cell: cellB }]),
            ["two", "ASKED # AAAAAA ask look # BBBBBB three", "five"]);
        expect("nothing of his own, it alone",
            Cells.laidOut(Cells.byCell([{ name: "one", weight: 3 }], () => cellA), rowsOf,
                headingOf, inside => `ASKED ${inside.join(" ")}`), ["ASKED # AAAAAA one"]);
        // It stands where banded's agentAt puts it, a row weighing the same
        // before it, so For You and its :filter, which reads agentAt, list
        // in one order; `all` holds what agents ask among the rest.
        const ask = name => ({ kind: "agent", thing: { kind: "todo", blocks: 1,
            since: now - hour }, name, cell: cellA });
        const tied = ForYou.banded([ask("a"), ask("b"), { kind: "do", thing: { kind: "todo",
            blocks: 1, since: now - hour }, name: "tie" }, { kind: "do",
            thing: { kind: "todo" }, name: "light" }], now);
        const tiedNames = tied.now.map(one => one.name);
        expect("a tie stands before it, where agentAt puts it",
            [Cells.laidOut(Cells.byCell(tied.all, one => one.cell ?? null), rowsOf, headingOf,
                () => "ASKED"),
                tiedNames.slice(0, tied.agentAt).concat(["ASKED"], tiedNames.slice(tied.agentAt))],
            [["tie", "ASKED", "light"], ["tie", "ASKED", "light"]]);
        expect("a lone ask is among all of them too",
            ForYou.banded([ask("a")], now).all.map(one => one.name), ["a"]);

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
