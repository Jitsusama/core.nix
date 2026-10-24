// Drives the launcher's Shortcuts (modules/quickshell/launcher/KeyMap.qml)
// through Quickshell itself, over an account's own shortcuts, Neovim's and
// pi's among them, with niri stood in for: its top is the layers, pi's named
// Agents and found by "pi" typed whole, each saying its count and its first
// groups; a layer holds its groups, then what no group takes; typing finds a
// group by its name before every chord it starts, flat, at the top and
// inside a layer, a group found from the top saying its layer, and one
// found further down saying the group holding it; and a group can hold
// groups, Code's each a dozen or fewer. It exits 0 once every case holds, and otherwise 1, saying
// which case didn't.
import qs
import Quickshell
import QtQuick
import "launcher"

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function names(rows) {
        return rows.map(row => row.name);
    }

    // A row of a level by its name.
    function row(rows, name) {
        return rows.find(one => one.name === name) ?? null;
    }

    KeyMap {
        id: map
    }

    Component.onCompleted: Qt.callLater(run)

    function run() {
        try {
            const top = map.everything();
            expect("the layers", names(top).join(", ")
                === "Desktop, Touchscreen, Terminal, Neovim, Agents", names(top).join(", "));
            const desktop = top[0];
            const binds = Shortcuts.layers[0].binds.length;
            expect("the desktop's count and first groups", desktop.count === String(binds)
                && desktop.side === "Windows, Columns", `${desktop.count} ${desktop.side}`);
            expect("a short layer, its chords alone",
                top[1].verbs().every(one => !one.verbs), names(top[1].verbs()).join(", "));
            const total = Shortcuts.layers.reduce((all, layer) => all + layer.binds.length, 0);
            expect("home counts every chord", map.counted() === String(total).replace(
                /\B(?=(\d{3})+(?!\d))/g, ","), map.counted());

            expect("pi found whole", map.search("pi").rows[0].name === "Agents",
                names(map.search("pi").rows.slice(0, 3)).join(", "));
            expect("Agents not by a letter", map.search("p").rows.every(one => one.name
                !== "Agents"), "found");

            const work = map.search("work").rows;
            expect("a group before its chords", work[0].name === "Workspaces"
                && work.some(one => one.name === "Go to workspace 3"),
                names(work.slice(0, 3)).join(", "));
            const tabs = map.search("tabs").rows.filter(one => one.name === "Tabs");
            expect("a group from the top says its layer",
                tabs.map(one => one.side).join(", ") === "Terminal, Neovim",
                tabs.map(one => one.side).join(", "));
            expect("a chord at the top, flat", map.search("open a terminal").rows[0].name
                === "Open a terminal", map.search("open a terminal").rows[0].name);

            const inside = desktop.from;
            expect("the desktop's groups", names(inside.everything()).slice(0, 3).join(", ")
                === "Windows, Columns, Workspaces", names(inside.everything()).join(", "));
            const there = inside.search("work").rows;
            expect("a group inside a layer before its chords", there[0].name === "Workspaces"
                && there.some(one => one.name === "Go to workspace 3")
                && there.every(one => !String(one.key).startsWith("shortcut:neovim")),
                names(there.slice(0, 3)).join(", "));
            const workspaces = row(inside.everything(), "Workspaces");
            expect("a big group holds groups, then what none takes",
                names(workspaces.verbs()).join(", ") === "Going to one, Moving a column, "
                + "Moving a window, Move the workspace up, Move the workspace down, "
                + "Show every workspace", names(workspaces.verbs()).join(", "));
            const going = row(workspaces.verbs(), "Going to one");
            expect("a group holds its chords", going.verbs().some(one => one.name
                === "Go to workspace 3") && going.verbs().every(one => !one.verbs),
                names(going.verbs()).join(", "));
            expect("a group's chords found inside it", workspaces.from.search("3").rows.some(
                one => one.name === "Go to workspace 3"), "not found");

            const neovim = top[3];
            const code = row(neovim.verbs(), "Code");
            expect("Code in groups of a dozen or fewer", code.verbs().every(one => one.verbs
                && Number(one.count) <= 12) && names(code.verbs()).slice(0, 2).join(", ")
                === "Definitions and calls, Hints and fixes", names(code.verbs()).join(", "));
            // Pull requests, Issues and Threads each have Reactions, so each
            // found from above says what holds it.
            const holders = rows => rows.filter(one => one.name === "Reactions")
                .map(one => one.side).sort().join(", ");
            expect("a group found from the top says what holds it",
                holders(map.search("reactions").rows) === "Issues, Pull requests, Threads",
                holders(map.search("reactions").rows));
            expect("and found inside its layer",
                holders(neovim.from.search("reactions").rows) === "Issues, Pull requests, Threads",
                holders(neovim.from.search("reactions").rows));
            const own = row(neovim.from.search("code").rows, "Code");
            expect("a level's own group found inside it says its first groups",
                own !== null && own.side === code.side && code.side.startsWith("Definitions"),
                own ? own.side : "not found");
            const github = row(neovim.verbs(), "GitHub");
            expect("a group holding groups", names(github.verbs()).slice(0, 4).join(", ")
                === "Pull requests, Issues, Reviews, Workflow runs",
                names(github.verbs()).join(", "));
            const held = neovim.verbs();
            const loose = held.findIndex(one => !one.verbs);
            expect("what no group takes, after the groups", loose > 0
                && held.slice(loose).every(one => !one.verbs), names(held).join(", "));

            const lock = map.search("lock the screen").rows[0];
            lock.run();
            expect("a desktop shortcut runs", Niri.spawned.join(" ") === "qs ipc call lock lock",
                Niri.spawned.join(" "));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}
