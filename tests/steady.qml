// How a list the launcher shows takes a change under it, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-holds-still-as-it-refreshes` check copies
// modules/quickshell/launcher/Steady.js beside this file and runs
// `qml -platform offscreen steady.qml`. Each case is a promise Joel leans on
// while a dozen agents change around him: rows keep the order they're shown
// in, and the selection stays on its row.
import QtQuick
import "Steady.js" as Steady

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

    function rows(said) {
        return said.split(" ").filter(one => one !== "").map(one => {
            const gone = one.endsWith("-");
            const [key, side] = (gone ? one.slice(0, -1) : one).split(":");
            return gone ? { key: key, side: side ?? "", gone: true } : { key: key, side: side ?? "" };
        });
    }

    // Each row as it reads, a row gone marked by a dash after it.
    function read(list) {
        return list.map(row => (row.side === "" ? row.key : `${row.key}:${row.side}`)
            + (row.gone ? "-" : "")).join(" ");
    }

    function run() {
        const failures = [];
        const id = row => row.key;
        // [what, the rows shown, the rows now, how they're shown after]
        const orders = [
            ["nothing changed is as it was", "a b c", "a b c", "a b c"],
            ["a row that would rank first stays where it's shown", "a b c", "c a b", "a b c"],
            ["each row says what it says now", "a:working b c", "c a:waiting b",
                "a:waiting b c"],
            ["a new row comes at the end", "a b", "c a b", "a b c"],
            ["new rows come in their own order", "a", "d c a b", "a d c b"],
            ["a row gone stays where it was, gone", "a b c", "c a", "a b- c"],
            ["and says what it said", "a b:working", "a", "a b:working-"],
            ["everything gone stays, gone", "a b", "", "a- b-"],
            ["a row gone is still gone", "a b-", "a", "a b-"],
            ["a row gone and back is there again", "a b-", "b a", "a b"],
            ["nothing shown takes the rows as they are", "", "b a", "b a"],
            ["rows alike keep their places", "x:1 x:2 y", "y x:3 x:4", "x:3 x:4 y"]
        ];
        for (const [what, shown, now, want] of orders) {
            const got = read(Steady.kept(rows(shown), rows(now), id));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // [what, the rows shown, the rows now, how they're shown while
        // held, how many wait]: in visual, operator-pending or on the
        // command line the rows hold still and a new one waits behind
        // "2 new" until it ends (VKCF3Z's spec), each shown row saying
        // what it says now and one gone staying, as ever.
        const holding = [
            ["nothing new waits for nothing", "a b", "b a", "a b", 0],
            ["a new row waits", "a b", "c a b", "a b", 1],
            ["and two", "a", "d c a", "a", 2],
            ["a row gone stays, gone, a new one waiting", "a b", "c a", "a b-", 1],
            ["each says what it says now", "a:working b", "b a:waiting", "a:waiting b", 0]
        ];
        for (const [what, shown, now, want, waiting] of holding) {
            const held = Steady.held(rows(shown), rows(now), id);
            const got = [read(held.rows), held.waiting];
            if (JSON.stringify(got) !== JSON.stringify([want, waiting]))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify([want, waiting])}`);
        }

        // [what, the rows shown, the rows now, what's left once a key makes
        // the list afresh]: Write in Slack's dimmed row, there until Slack
        // answers, stands where it is through the search a step-in makes.
        const forgets = [
            ["a row that left is forgotten", "a b c", "c a", "a c"],
            ["a row its source dims stays where it stands", "a b c", "a b- c", "a b- c"],
            ["and is forgotten once it leaves", "a b- c", "a c", "a c"],
            ["a row that left stays forgotten", "a b c", "a", "a"]
        ];
        for (const [what, shown, now, want] of forgets) {
            const got = read(Steady.forgotten(Steady.kept(rows(shown), rows(now), id)));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // A row that arrives while the list is shown comes at the end, out
        // of the place a tree would give it, so it reads as it would ranked
        // flat, as a session a step in under its orchestrator does once
        // typed: at no depth, saying whom it's under (Agents.qml). One kept
        // in place, and every row of a list shown afresh, keep their own.
        const nested = key => ({ key: key, side: "", depth: 1,
            ranked: () => ({ key: key, side: "under", depth: 0 }) });
        const depths = list => list.map(row => `${row.key}${row.depth ?? 0}:${row.side}`)
            .join(" ");
        const flats = [
            ["a row arriving late reads flat", [{ key: "a", side: "" }],
                [{ key: "a", side: "" }, nested("b")], "a0: b0:under"],
            ["a row kept in place keeps its depth", [nested("b")], [nested("b")], "b1:"],
            ["a list shown afresh keeps every row's depth", [], [nested("b")], "b1:"]
        ];
        for (const [what, shown, now, want] of flats) {
            const got = depths(Steady.kept(shown, now, id));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        // [what, the rows shown, whether an answer gave way, those that may
        // stand]: an ending is made again, and an answer given only while
        // nothing else was goes once something comes, rather than standing
        // dimmed above it with the selection on it (80ECO2, 2026-11-12).
        const ask = { key: "", side: "", answered: true };
        const ending = { key: "e", side: "", endedBy: {} };
        const standings = [
            ["an ending never stands", [ask, ending], false, ""],
            ["an answer that holds stands", [ask, { key: "a", side: "" }], false, " a"],
            ["one that gave way goes", [ask, { key: "a", side: "" }], true, "a"]
        ];
        for (const [what, shown, gaveWay, want] of standings) {
            const got = read(Steady.standing(shown, gaveWay));
            if (got !== want)
                failures.push(`${what}: "${got}", not "${want}"`);
        }

        const after = rows("a b c d");
        // [what, the row it was on, where it was, the lowest it goes, where it goes]
        const selections = [
            ["it stays on its row", "c", 1, 0, 2],
            ["on the first row too", "a", 0, 0, 0],
            ["its row gone, it stays where it was", "z", 2, 0, 2],
            ["and no further than the last row", "z", 9, 0, 3],
            ["with none, it stays where it was", "", 1, 0, 1],
            ["above the rows, an answer's headline stays", "", -1, -1, -1],
            ["but never above the lowest", "", -1, 0, 0]
        ];
        for (const [what, was, at, lowest, want] of selections) {
            const got = Steady.selection(after, { row: was, at: at, lowest: lowest }, id);
            if (got !== want)
                failures.push(`${what}: ${got}, not ${want}`);
        }
        if (Steady.selection([], { row: "a", at: 0, lowest: 0 }, id) !== 0)
            failures.push("an empty list leaves the selection at the top");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
