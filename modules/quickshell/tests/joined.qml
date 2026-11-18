// How the launcher joins its sources' rows into the list a search finds
// from, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-joins-only-what-changed` check copies
// modules/quickshell/launcher/Joined.js beside this file and runs
// `qml -platform offscreen joined.qml`. An opening joins the list again
// each time a daemon answers, so a source that hasn't changed is never
// walked again, and the rows found are the same ones either way.
import QtQuick
import "Joined.js" as Joined

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

    function source(keys) {
        return { things: keys.split(" ").map(key => ({ key: key, name: key.split(":")[1] })) };
    }

    function run() {
        const failures = [];
        const readied = [];
        const ready = row => {
            readied.push(row.key);
            return row;
        };
        const joined = (sources, standIns, before, now) => sources.reduce((all, one) =>
            all.concat(Joined.rowsOf(one, standIns, { before: before, now: now }, ready)), []);
        const programs = source("program:a program:b action:Open");
        const windows = source("window:1");
        const standIns = ["Open"];

        const first = new Map();
        const once = joined([programs, windows], standIns, new Map(), first);
        const keysOf = rows => rows.map(row => row.key).join(" ");
        if (keysOf(once) !== "program:a program:b window:1")
            failures.push(`the first joining found "${keysOf(once)}"`);
        if (readied.join(" ") !== "program:a program:b window:1")
            failures.push(`the first joining made ready "${readied.join(" ")}", not each row once`);

        readied.length = 0;
        windows.things = [{ key: "window:2", name: "window:2" }];
        const second = new Map();
        const again = joined([programs, windows], standIns, first, second);
        if (readied.join(" ") !== "window:2")
            failures.push(`a joining with only windows changed made ready "${readied.join(" ")}"`);
        if (again[0] !== once[0] || again[1] !== once[1])
            failures.push("a source that hadn't changed gave other rows than it had");
        if (keysOf(again) !== "program:a program:b window:2")
            failures.push(`the second joining found "${keysOf(again)}"`);

        readied.length = 0;
        const third = new Map();
        joined([programs], ["Open", "Close"], second, third);
        if (readied.join(" ") !== "program:a program:b")
            failures.push(`new stand-ins made ready "${readied.join(" ")}", not the source again`);
        if (third.has(windows))
            failures.push("a source that's gone was still kept");

        const plain = new Map();
        const choices = [
            { key: "program:kitty", name: "Kitty  Terminal" },
            { key: "action:kitty-new", name: "New Window", of: "program:kitty" },
            { key: "action:kitty-tab", name: "New Tab", of: "program:kitty" },
            { key: "window:kitty", name: "kitty terminal" }
        ];
        const plainly = name => name.trim().toLowerCase().replace(/\s+/g, " ");
        const indexed = Joined.indexes(choices, plainly, plain);
        if (indexed.keyed["window:kitty"] !== choices[3])
            failures.push("a row wasn't found by its key");
        const named = (indexed.namedAs["kitty terminal"] ?? []).join(" ");
        if (named !== "program:kitty window:kitty")
            failures.push(`"kitty terminal" named "${named}", not both rows in their order`);
        const parts = (indexed.partsOfRows["program:kitty"] ?? []).map(row => row.key).join(" ");
        if (parts !== "action:kitty-new action:kitty-tab")
            failures.push(`kitty's parts were "${parts}"`);
        let said = 0;
        Joined.indexes(choices, name => {
            said += 1;
            return plainly(name);
        }, plain);
        if (said !== 0)
            failures.push(`indexing the same rows again said ${said} names plainly again`);
        const fresh = new Map();
        for (let i = 0; i < 100; i++)
            fresh.set(`gone ${i}`, `gone ${i}`);
        Joined.indexes(choices, plainly, fresh);
        if (fresh.size > 4)
            failures.push(`${fresh.size} names were kept for 4 rows, names long gone among them`);

        for (const failure of failures)
            console.warn(failure);
        console.log(`joined: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
