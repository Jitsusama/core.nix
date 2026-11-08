// Each kind of list's one order, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `each-list-keeps-its-kinds-order` check copies
// modules/quickshell/launcher/Order.js beside this file and runs
// `qml -platform offscreen order.qml`. Each case is a promise from
// docs/design.md's table of kinds: what moves a list is what Joel does,
// A to Z is only the last tie, and a choice stays where it stands.
import QtQuick
import "Order.js" as Order

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

    function names(rows) {
        return rows.map(row => row.name).join(" ");
    }

    // The rows sorted in this order, from each place they could start in,
    // so a case can't pass on the order its rows were written in.
    function sorted(rows, order) {
        const each = [rows, rows.slice().reverse(), rows.slice(1).concat(rows.slice(0, 1))];
        const said = each.map(start => names(start.slice().sort(order)));
        return said.every(one => one === said[0]) ? said[0] : `it depends: ${said.join(" | ")}`;
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    function run() {
        const failures = [];
        const byName = row => row.name;

        const programs = Order.returned({ first: row => row.open, last: row => row.at,
            name: byName });
        check(failures, "returned: open first, the last done first, then A to Z",
            sorted([{ name: "b", open: false }, { name: "kitty", open: true, at: 3 },
                { name: "chrome", open: true, at: 9 }, { name: "a", open: false },
                { name: "zed", open: false, at: 5 }, { name: "mpv", open: true }], programs),
            "chrome kitty mpv zed a b");
        const quests = Order.returned({ first: row => row.open, placed: (one, other) =>
            one.priority - other.priority, last: row => row.updated, name: byName });
        check(failures, "returned: those open in the order Joel put them, the rest the last first",
            sorted([{ name: "q1", open: true, priority: 2, updated: "2026-11-07" },
                { name: "q2", updated: "2026-11-01" }, { name: "q3", updated: "2026-11-05" },
                { name: "q4", open: true, priority: 1, updated: "2026-10-01" }], quests),
            "q4 q1 q3 q2");
        check(failures, "returned: numbers in names read by their value",
            sorted([{ name: "Tab 10" }, { name: "tab 9" }, { name: "Tab 1" }],
                Order.returned({ first: () => false, name: byName })), "Tab 1 tab 9 Tab 10");

        check(failures, "a stream: the newest first, the undated last",
            sorted([{ name: "old", at: 1 }, { name: "none" }, { name: "new", at: 7 },
                { name: "mid", at: 4 }], Order.stream(row => row.at)), "new mid old none");
        check(failures, "a stream: ISO times read in order",
            sorted([{ name: "a", at: "2026-11-07T09:00:00Z" }, { name: "b", at: "" },
                { name: "c", at: "2026-11-07T22:00:00Z" }], Order.stream(row => row.at)),
            "c a b");
        check(failures, "a stream: what's still going first",
            sorted([{ name: "done", at: 9 }, { name: "running", at: 1, live: true },
                { name: "older", at: 3 }], Order.stream(row => row.at, row => row.live)),
            "running done older");
        const alike = [{ name: "first", at: 2 }, { name: "second", at: 2 }];
        check(failures, "a stream: those alike in time keep the order they came",
            names(alike.slice().sort(Order.stream(row => row.at))), "first second");

        const states = ["connected", "paired", "seen"];
        const devices = Order.live({ stands: row => states.indexOf(row.state),
            since: row => row.since, name: byName });
        check(failures, "live: by how each stands, then the earliest started, then A to Z",
            sorted([{ name: "mouse", state: "seen" }, { name: "pods", state: "connected",
                since: 9 }, { name: "keys", state: "connected", since: 2 },
            { name: "pad", state: "paired" }, { name: "dial", state: "seen" }], devices),
            "keys pods pad dial mouse");
        const networks = Order.live({ stands: row => row.known ? 0 : 1,
            then: (one, other) => other.strength - one.strength, name: byName });
        check(failures, "live: a kind's own measure before A to Z",
            sorted([{ name: "cafe", strength: 40 }, { name: "home", known: true, strength: 20 },
                { name: "airport", strength: 70 }, { name: "bus", strength: 40 }], networks),
            "home airport bus cafe");

        check(failures, "amounts: the largest first",
            sorted([{ name: "sh", used: 1 }, { name: "chrome", used: 90 },
                { name: "nix", used: 30 }, { name: "a", used: 1 }],
                Order.amounts({ amount: row => row.used, name: byName })), "chrome nix a sh");
        const nearlyFull = (one, other) => Number(other.full) - Number(one.full);
        check(failures, "amounts: worse where it isn't only larger",
            sorted([{ name: "big", full: false }, { name: "boot", full: true },
                { name: "home", full: false }],
                Order.amounts({ worse: nearlyFull, name: byName })), "boot big home");

        const scales = [1, 1.25, 1.5, 2].map(size => ({ name: `${size * 100}%`, size: size,
            chosen: size === 1.5 }));
        const bySize = Order.choices({ natural: (one, other) => one.size - other.size,
            name: byName });
        check(failures, "choices: their natural order",
            sorted(scales, bySize), "100% 125% 150% 200%");
        const chosenFirst = scales.map(one => Object.assign({}, one, { chosen: one.size === 2 }));
        check(failures, "choices: choosing one never moves it",
            sorted(chosenFirst, bySize), sorted(scales, bySize));
        check(failures, "choices: a device by its name",
            sorted([{ name: "Speakers" }, { name: "headphones" }, { name: "HDMI 2" },
                { name: "HDMI 10" }], Order.choices({ name: byName })),
            "HDMI 2 HDMI 10 headphones Speakers");

        const newest = Order.stream(row => row.at);
        check(failures, "broken: nothing where the rows keep it",
            Order.broken([{ at: 3 }, { at: 2 }, { at: 2 }], newest), -1);
        check(failures, "broken: the first row out of its place",
            Order.broken([{ at: 3 }, { at: 1 }, { at: 2 }], newest), 1);

        if (failures.length > 0) {
            console.warn(failures.join("\n"));
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
