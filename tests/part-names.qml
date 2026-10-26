// Drives the launcher's answers that name their parts from ids they build
// as they answer (Calculator.qml, Clocks.qml, Today.qml, Waits.qml,
// Vendors.qml, Networks.qml and Characters.qml) through Quickshell itself,
// so every part each can show is named as the vocabulary names it
// (launcher/words.json, Words.qml): an id the vocabulary doesn't know shows
// as the id itself, as "calculator:hex" would, so a typo on either side
// fails here rather than on screen. The time is told with tzdata's zones
// kept beside it, a character with Unicode's names, a network with no
// lookups, and Waits reads what lag-watch would write. Currency names its
// parts by the currencies alone, so it builds no id to check. It exits 0
// once every case holds, and otherwise 1, saying which case didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Calculator {
        id: calculator
    }

    Launcher.Clocks {
        id: clocks

        settings: ({ list: Quickshell.shellPath("zones.jsonl"), zone: "America/Toronto" })
    }

    Launcher.Today {
        id: today

        settings: ({ zone: "America/Toronto" })
    }

    Launcher.Waits {
        id: waits
    }

    Launcher.Vendors {
        id: vendors
    }

    Launcher.Networks {
        id: networks

        settings: ({ lookups: false })
    }

    Launcher.Characters {
        id: characters

        settings: ({ names: Quickshell.shellPath("names.txt") })
    }

    // Each answer, what's typed to show every part it can, and the ids of
    // those parts within its scope.
    readonly property var asked: [
        { scope: "calculator", from: calculator, typed: ["0x1f", "255 in hex"],
            ids: ["binary", "decimal", "hex", "octal"] },
        { scope: "time", from: clocks, typed: ["time in tokyo"], ids: ["apart", "day", "zone"] },
        { scope: "today", from: today, typed: ["date"],
            ids: ["day", "epoch", "utc", "week", "zone"] },
        { scope: "waits", from: waits, typed: ["lag"], ids: ["typical", "work"] },
        { scope: "vendors", from: vendors, typed: ["3c:22:fb:12:34:56"],
            ids: ["colons", "dotted", "kind"] },
        { scope: "networks", from: networks,
            typed: ["10.0.0.5/22", "2001:db8::1/48", "/22", "is 10.0.0.1 in 10.0.0.0/8"],
            ids: ["broadcast", "first", "full", "hosts", "kind", "last", "mask", "network",
                "range", "size", "subnets", "wildcard"] },
        { scope: "characters", from: characters, typed: ["ascii 65", "utf8 \u00e9"],
            ids: ["binary", "bits", "decimal", "escaped", "hex", "length", "name", "percent",
                "point", "points", "utf8"] }
    ]

    // Quickshell hears Qt.exit only once the shell has loaded, so the cases
    // run just after.
    Component.onCompleted: Qt.callLater(root.cases)

    function cases() {
        let status = 0;
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            status = 1;
        }
        Qt.callLater(() => Qt.exit(status));
    }

    function run() {
        for (const each of asked) {
            // What the vocabulary names each of its parts, or nothing for an
            // id it doesn't know.
            const named = each.ids.map(id => Words.things[`${each.scope}:${id}`]?.name ?? "");
            expect(`${each.scope}'s parts in the vocabulary`, named.every(name => name !== ""),
                each.ids.filter((id, at) => named[at] === "").join(", "));
            const shown = [];
            for (const typed of each.typed) {
                const said = each.from.answer(typed);
                expect(`"${typed}" answers`, said !== null && said.rows.length > 0,
                    JSON.stringify(said));
                for (const row of said.rows) {
                    if (!shown.includes(row.name))
                        shown.push(row.name);
                }
            }
            expect(`${each.scope}'s parts named as the vocabulary says`,
                shown.slice().sort().join(", ") === named.slice().sort().join(", "),
                `${shown.join(", ")}, not ${named.join(", ")}`);
        }
    }
}
