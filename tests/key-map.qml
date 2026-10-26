// Drives the launcher's Shortcuts (modules/quickshell/launcher/KeyMap.qml)
// through Quickshell itself, over an account's own shortcuts, Neovim's and
// pi's among them, with niri stood in for: its top is the layers, pi's named
// Agents and found by that name alone, each saying its count and its first
// groups; a layer holds its groups, then what no group takes; typing finds a
// group by its name before every chord it starts, flat, at the top and
// inside a layer, a group found from the top saying its layer, and one
// found further down saying the group holding it; a group can hold
// groups, Code's each a dozen or fewer; and the window the launcher opened
// over, as the launcher says it as Shortcuts opens, puts its program's
// layers first, its own chord opening on the first of them, until the
// launcher closes; and a key that sends its own modifiers, as the Copilot
// key does, is said as the one key. It exits 0 once every case holds, and otherwise 1, saying which
// case didn't.
import qs
import Quickshell
import QtQuick
import "launcher"
import "launcher/KeyGroups.js" as KeyGroups

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

        launcher: opener
    }

    // The launcher, as far as Shortcuts asks it: whether it's open, and the
    // window it opened over, which each case sets before Shortcuts opens.
    QtObject {
        id: opener

        property bool open: true
        property var under: null

        function cameFrom() {
            return under;
        }
    }

    // Shortcuts opened over a window, or over none, as the launcher opens it.
    function over(appId, title) {
        opener.under = appId === null ? null : { app_id: appId, title: title };
        map.opened();
    }

    Component.onCompleted: Qt.callLater(run)

    function run() {
        try {
            const top = map.everything();
            expect("the layers", names(top).join(", ")
                === "Desktop, Terminal, Neovim, Agents, Picture viewer, Markdown reader",
                names(top).join(", "));
            const desktop = top[0];
            // Counted as rows: two chords for one act are one.
            const binds = KeyGroups.merged("desktop", Shortcuts.layers[0].binds).length;
            expect("the desktop's count and first groups", desktop.count === String(binds)
                && desktop.side === "Windows, Columns", `${desktop.count} ${desktop.side}`);
            // The touchscreen's swipes are the desktop's: one doing what a
            // chord does is that chord's row, the swipe in words in its note,
            // and found by "swipe".
            const below = map.all.filter(one => one.name === "Go to the workspace below"
                && String(one.key).startsWith("shortcut:desktop:"));
            expect("a swipe joins its chord's row", below.length === 1
                && below[0].note.includes("3 fingers up"), below.map(one => one.note).join("; "));
            expect("a swipe found by its word", map.search("swipe below").rows
                .some(one => one.name === "Go to the workspace below"),
                names(map.search("swipe below").rows.slice(0, 3)).join(", "));
            const swiped = map.spelled([{ text: "4", written: "4 fingers down" }, { text: "x" }]);
            expect("a swipe in words", swiped === "4 fingers down", swiped);
            // A chord doing one thing in several places is one row.
            const total = Shortcuts.layers.reduce((all, layer) => all
                + KeyGroups.merged(layer.name, layer.binds).length, 0);
            expect("home counts every chord", map.counted() === String(total).replace(
                /\B(?=(\d{3})+(?!\d))/g, ","), map.counted());
            // A second chord for one act is said in the first's note.
            const spelt = map.spelled([{ text: "Super" }, { text: "L" }, { text: "g", after: true }]);
            expect("a chord as words", spelt === "Super+L g", spelt);
            // The Copilot key, which sends its own Shift and Super, is said
            // as the one key pressed.
            const copilot = Shortcuts.layers[0].binds.find(bind => bind.keys
                === "super+shift+xf86touchpadoff");
            const pressed = map.spelled(copilot?.chord);
            expect("the Copilot key as one key", pressed === "Copilot", pressed);
            // And no two rows read alike: one act in one place is one row.
            const read = map.all.map(one => `${one.name} (${one.note})`);
            const twice = read.filter((one, at) => read.indexOf(one) !== at);
            expect("no two rows read alike", twice.length === 0, twice.join("; "));

            expect("Agents found by its name", map.search("agents").rows[0].name === "Agents",
                names(map.search("agents").rows.slice(0, 3)).join(", "));
            expect("and never by the program's name", map.search("pi").rows.every(one =>
                one.name !== "Agents"), names(map.search("pi").rows.slice(0, 3)).join(", "));
            expect("Agents not by a letter", map.search("p").rows.every(one => one.name
                !== "Agents"), "found");

            const work = map.search("work").rows;
            expect("a group before its chords", work[0].name === "Workspaces"
                && work.some(one => one.name === "Go to workspace 3"),
                names(work.slice(0, 3)).join(", "));
            const tabs = map.search("tabs").rows.filter(one => one.name === "Tabs");
            expect("a group from the top says what holds it",
                tabs.map(one => one.side).join(", ") === "Terminal, Files",
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
                names(workspaces.verbs()).join(", ") === "Focus, The column, The window, "
                + "Above and below, Move the workspace up, Move the workspace down, "
                + "Show every workspace, Stop showing every workspace",
                names(workspaces.verbs()).join(", "));
            const going = row(workspaces.verbs(), "Focus");
            expect("a group holds its chords", going.verbs().some(one => one.name
                === "Go to workspace 3") && going.verbs().every(one => !one.verbs),
                names(going.verbs()).join(", "));
            expect("a group's chords found inside it", workspaces.from.search("3").rows.some(
                one => one.name === "Go to workspace 3"), "not found");

            const neovim = top[2];
            const code = row(neovim.verbs(), "Code");
            expect("Code in groups of a dozen or fewer", code.verbs().every(one => one.verbs
                && Number(one.count) <= 12) && names(code.verbs()).slice(0, 2).join(", ")
                === "Definitions and calls, Hints and fixes", names(code.verbs()).join(", "));
            // Windows, Columns, Workspaces and Screens each have Focus, so
            // each found from above says what holds it.
            const holders = rows => rows.filter(one => one.name === "Focus")
                .map(one => one.side).sort().join(", ");
            const focus = "Columns, Screens, Windows, Workspaces";
            expect("a group found from the top says what holds it",
                holders(map.search("focus").rows) === focus, holders(map.search("focus").rows));
            expect("and found inside its layer",
                holders(desktop.from.search("focus").rows) === focus,
                holders(desktop.from.search("focus").rows));
            // Reacting in a pull request, an issue or a review's thread is
            // one row, saying all three.
            const reacting = map.search("react with a heart").rows.filter(one => one.name
                === "React with a heart");
            expect("one chord in several places, one row", reacting.length === 1
                && reacting[0].note.endsWith("a pull request or a review's thread"),
                reacting.map(one => one.note).join(" | "));
            const own = row(neovim.from.search("code").rows, "Code");
            expect("a level's own group found inside it says its first groups",
                own !== null && own.side === code.side && code.side.startsWith("Definitions"),
                own ? own.side : "not found");
            const github = row(neovim.verbs(), "GitHub");
            expect("a group holding groups", names(github.verbs()).slice(0, 4).join(", ")
                === "Issues, People and labels, Comments, Reactions",
                names(github.verbs()).join(", "));
            const pulls = row(neovim.verbs(), "Changes");
            expect("changes, by what they hold", names(pulls.verbs()).slice(0, 4)
                .join(", ") === "Finding changes, The change, Merging, Checks",
                names(pulls.verbs()).join(", "));
            const held = top[1].verbs();
            const loose = held.findIndex(one => !one.verbs);
            expect("what no group takes, after the groups", loose > 0
                && held.slice(loose).every(one => !one.verbs), names(held).join(", "));

            // Over Neovim in the terminal, its layers lead, and the chord
            // opens on Neovim's, its row saying it's that window's; over
            // pi, Agents; over nothing, the desktop, and nothing to open on.
            // Each is what the launcher says it opened over, read as
            // Shortcuts opens (KeyMap.qml's opened).
            over("kitty", "\u{1F4DD} notes.md");
            expect("over Neovim, its layers first", names(map.everything()).join(", ")
                === "Neovim, Terminal, Desktop, Agents, Picture viewer, Markdown reader",
                names(map.everything()).join(", "));
            const lead = map.leading();
            expect("the chord opens on Neovim's", lead !== null && lead.name === "Neovim"
                && lead.verbs().some(one => one.name === "Code") && lead.note !== "",
                lead ? `${lead.name} ${lead.note}` : "nothing");
            expect("its chords searched first", String(map.all[0].key)
                .startsWith("shortcut:neovim:"), map.all[0].key);
            // A program's own layer, by its desktop entry: Neovim's, though
            // its windows are the terminal's, a window's own by its app id,
            // and the desktop's for one with none; Neovim's layer row says
            // whose it is.
            for (const entry of [["nvim", "neovim"], ["kitty", "terminal"],
                ["swayimg", "swayimg"], ["google-chrome", "desktop"]])
                expect(`${entry[0]}'s own layer`, map.ofProgram(entry[0]) === entry[1],
                    map.ofProgram(entry[0]));
            const neovimKeys = map.layerOf("nvim");
            expect("nvim holds Neovim's keys", neovimKeys !== null && neovimKeys.layer === "neovim",
                neovimKeys ? neovimKeys.layer : "nothing");
            over("kitty", "\u03C0 - core.nix");
            expect("over pi, Agents first", names(map.everything()).slice(0, 3).join(", ")
                === "Agents, Terminal, Desktop", names(map.everything()).join(", "));
            over("kitty", "~");
            expect("over the terminal, its own", map.leading().name === "Terminal"
                && names(map.everything())[1] === "Desktop", names(map.everything()).join(", "));
            // Each of these after the terminal led, so a window read wrong
            // would leave it leading. A program in a window of its own with
            // keys of its own leads with them, then the desktop's.
            over("swayimg", "holiday.jpg");
            expect("over the picture viewer, its own", map.leading()?.name === "Picture viewer"
                && names(map.everything())[1] === "Desktop", names(map.everything()).join(", "));
            over("inlyne", "README.md");
            expect("over the markdown reader, its own", map.leading()?.name === "Markdown reader"
                && names(map.everything())[1] === "Desktop", names(map.everything()).join(", "));
            over("google-chrome", "");
            expect("over another program, the desktop's", map.leading() === null
                && names(map.everything())[0] === "Desktop", names(map.everything()).join(", "));
            over("kitty", "~");
            over(null, "");
            expect("no window, the desktop first", names(map.everything())[0] === "Desktop"
                && map.leading() === null, names(map.everything()).join(", "));
            // What it opened over is that opening's alone: closed, the
            // desktop leads again.
            over("kitty", "~");
            opener.open = false;
            expect("closed, the desktop's again", map.leading() === null
                && names(map.everything())[0] === "Desktop", names(map.everything()).join(", "));
            opener.open = true;

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
