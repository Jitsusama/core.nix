// Shortcuts' groups, run in Qt's own JavaScript engine, the one the shell
// runs them in: the-launcher-groups-every-shortcut copies
// launcher/KeyGroups.js beside this file with an account's own binds, every
// layer's, as layers.json, and its vocabulary, as words.json, and runs
// `qml -platform offscreen key-groups.qml`. One chord doing one thing in
// several of a group's places is one row, and only then; every desktop
// chord falls in a group, each row of a grouped layer is listed once, the
// chords below land where Joel would look for them, a chord niri adds
// tomorrow finds a group without being told, a bind that names its group is
// put there, and a short layer stays whole. Each level is the right size for
// the card's eight rows: no group holds fewer than three chords, none sits
// more than two levels below its layer, and no level lists more than a
// card and a half. Every group the table lists is named in the vocabulary,
// a desktop group with a glyph, and the window the launcher opened over
// puts its program's layers first. It prints each group's count, and each
// level longer than the card.
import QtQuick
import "KeyGroups.js" as KeyGroups

QtObject {
    function read(file) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl(file), false);
        request.send();
        return JSON.parse(request.responseText);
    }

    // A bind as niri's module makes one: a niri action, or what's spawned.
    function niri(action) {
        return { run: ["niri", "msg", "action", action, "--"], keys: "", when: "", does: "" };
    }

    function spawn(command) {
        return { run: command, keys: "", when: "", does: "" };
    }

    // The rows the card shows at once.
    readonly property int card: 8
    // No level lists more rows than this, groups and chords together.
    readonly property int most: 13
    // Nor does a group hold fewer chords than this.
    readonly property int fewest: 3
    // Nor is a group more levels below its layer than this.
    readonly property int deepest: 2

    // Every place a layer lists a row, each group's said to `counts`, and
    // the level each row is listed in to `where`, by its place.
    function placed(arranged, path, counts, where) {
        let out = arranged.loose.slice();
        for (const at of arranged.loose)
            where[at] = path;
        const rows = arranged.groups.length + arranged.loose.length;
        if (rows > most)
            counts.push(`FAILED ${path} lists ${rows} rows, more than ${most}`);
        else if (rows > card)
            counts.push(`${path} lists ${rows} rows, past the card's ${card}`);
        for (const group of arranged.groups) {
            const inner = `${path}/${group.id}`;
            counts.push(`${inner} ${group.at.length}`);
            if (group.at.length < fewest)
                counts.push(`FAILED ${inner} holds ${group.at.length}, fewer than ${fewest}`);
            if (inner.split("/").length - 1 > deepest)
                counts.push(`FAILED ${inner} is more than ${deepest} levels below its layer`);
            const inside = placed(group.inside, inner, counts, where);
            if (JSON.stringify(inside.slice().sort()) !== JSON.stringify(group.at.slice().sort()))
                counts.push(`FAILED ${path}/${group.id} lists other chords inside it`);
            out = out.concat(group.at);
        }
        return out;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what} gave ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };

        const layers = read("layers.json");
        const vocabulary = read("words.json");
        const known = {};
        for (const entry of vocabulary.things)
            known[entry.id] = entry;

        // Each bind of a layer is in one row, the rows doing one thing by
        // one chord in one group; each row is listed once, and a desktop
        // chord is never left out of the groups. Each bind is said, by its
        // chord and where it works, with the level its row is listed in.
        const counts = [];
        const levels = {};
        const rowsOf = {};
        for (const layer of Object.keys(layers)) {
            const binds = layers[layer];
            const rows = KeyGroups.merged(layer, binds);
            rowsOf[layer] = rows;
            const held = rows.reduce((all, row) => all.concat(row.places), []);
            expect(`${layer}'s binds, each in one row`, held.slice().sort((a, b) => a - b),
                binds.map((bind, at) => at));
            for (const row of rows)
                for (const at of row.places)
                    expect(`${layer}'s ${binds[at].chord} ${binds[at].when}, its row`,
                        [[row.chord].concat(row.also).some(chord => JSON.stringify(chord)
                            === JSON.stringify(binds[at].chord)), row.does,
                            KeyGroups.groupOf(layer, row)],
                        [true, binds[at].does, KeyGroups.groupOf(layer, binds[at])]);
            const arranged = KeyGroups.arrange(layer, rows);
            const listed = {};
            const places = placed(arranged, layer, counts, listed);
            levels[layer] = {};
            rows.forEach((row, at) => {
                for (const place of row.places) {
                    const bind = binds[place];
                    levels[layer][`${bind.chord} ${bind.when}`.trim()] = listed[at];
                }
            });
            expect(`${layer}'s rows, each once`, places.slice().sort((a, b) => a - b),
                rows.map((row, at) => at));
            if (layer === "desktop")
                expect("the desktop's chords in no group",
                    arranged.loose.map(at => rows[at].chord), []);
        }
        for (const line of counts)
            if (line.startsWith("FAILED"))
                failures.push(line);
        console.warn(counts.join("\n"));

        // Reacting in a pull request, an issue or a review's thread is one
        // row saying all three; [ C is the code's, GitHub's and a review's.
        const neovimRows = rowsOf.neovim ?? [];
        const reacting = neovimRows.filter(row => row.chord === "Space g h t +");
        expect("one reaction, everywhere it works", reacting.map(row => row.when),
            ["in a GitHub issue, a pull request or a review's thread"]);
        expect("the comment before, by its groups",
            neovimRows.filter(row => row.chord === "[ C").length, 3);
        expect("a place said once", KeyGroups.merged("neovim", [
            { chord: "x", does: "X", keys: "x", when: "in the outline" },
            { chord: "x", does: "X", keys: "x", when: "walking the outline" },
            { chord: "x", does: "X", keys: "x", when: "in the outline" }
        ]).map(row => row.when), ["in the outline or walking the outline"]);
        // Two chords doing one thing in one place are one row, the first
        // drawn and the other said; in another place, a row of their own.
        const twins = KeyGroups.merged("desktop", [
            { chord: "Super+L", does: "Lock the screen", keys: "super+l", when: "" },
            { chord: "XF86ScreenSaver", does: "Lock the screen", keys: "xf86screensaver",
                when: "" }
        ]);
        expect("two chords for one act, one row",
            twins.map(row => [row.chord, row.also]), [["Super+L", ["XF86ScreenSaver"]]]);
        expect("elsewhere, a row of its own", KeyGroups.merged("neovim", [
            { chord: "Space g c Return", does: "Commit", keys: "space g c return", when: "" },
            { chord: "c c", does: "Commit", keys: "c c", when: "in git's status" }
        ]).length, 2);

        // The desktop's chords where Joel would look for them.
        const desktop = {};
        for (const bind of layers.desktop ?? [])
            desktop[bind.chord] = KeyGroups.groupOf("desktop", bind);
        const where = {
            "Super+W": "windows",
            "Super+Up": "windows",
            "Super+G": "windows",
            "Super+Alt+Return": "windows",
            "Super+Return": "windows",
            "Super+Shift+Return": "windows",
            "Super+Left": "columns",
            "Super+Shift+Left": "columns",
            "Super+R": "columns",
            "Super+3": "workspaces",
            "Super+Shift+3": "workspaces",
            "Super+A": "workspaces",
            "Super+Ctrl+Left": "screens",
            "Super+Ctrl+Alt+Left": "screens",
            "Super+Ctrl+Shift+Page_Up": "screens",
            "XF86MonBrightnessUp": "screens",
            "Super+L": "screens",
            "XF86ScreenSaver": "screens",
            "Super+Space": "launcher",
            "Super+Shift+Slash": "launcher",
            "Super+N": "launcher",
            "XF86DoNotDisturb": "launcher",
            "Super+E": "typing",
            "Super+D": "typing",
            "Super+K": "typing",
            "Print": "capture",
            "Shift+Print": "capture",
            "Ctrl+Print": "capture",
            "Super+Print": "capture",
            "XF86AudioRaiseVolume": "sound",
            "XF86AudioPlay": "sound"
        };
        for (const chord of Object.keys(where))
            expect(chord, desktop[chord] ?? "not bound", where[chord]);

        // A chord niri adds tomorrow finds a group of its own accord.
        expect("a new screen action", KeyGroups.groupOf("desktop", niri("focus-monitor-next")),
            "screens");
        expect("a new workspace action",
            KeyGroups.groupOf("desktop", niri("focus-workspace-previous")), "workspaces");
        expect("a new window action",
            KeyGroups.groupOf("desktop", niri("swap-window-left")), "windows");
        expect("a new column action",
            KeyGroups.groupOf("desktop", niri("swap-column-left")), "columns");
        expect("a new program", KeyGroups.groupOf("desktop", spawn(["firefox"])), "windows");
        expect("a new collection",
            KeyGroups.groupOf("desktop", spawn(["launcher-say", "collection", "weather"])),
            "launcher");
        const louder = spawn(["sh", "-c", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 50%"]);
        expect("a new sound key", KeyGroups.groupOf("desktop", louder), "sound");

        // A bind that names its group is put there, at each depth it names.
        const named = Object.assign(niri("focus-monitor-next"), { group: "windows" });
        expect("a named group", KeyGroups.groupOf("desktop", named), "windows");
        const many = [];
        for (let i = 0; i < 20; i++)
            many.push({ run: [], keys: `space g h x${i}`, when: "", does: "", group: "" });
        many[0].group = "github/issues";
        const github = KeyGroups.arrange("neovim", many).groups.find(group => group.id === "github");
        expect("a named group inside a group",
            github ? github.inside.groups.map(group => group.id) : [], ["issues"]);

        // A short layer stays whole.
        const swipes = [niri("focus-column-left"), niri("focus-column-right"),
            niri("open-overview")];
        expect("a short layer", KeyGroups.arrange("desktop", swipes).groups, []);

        // Each row where Joel would look for it, the level it's listed in.
        const deep = {
            desktop: {
                "Super+Up": "desktop/windows/focus",
                "Super+Alt+Left": "desktop/windows/place",
                "Super+F": "desktop/windows/size",
                "Super+Return": "desktop/windows/new",
                "Super+W": "desktop/windows",
                "Super+Left": "desktop/columns/focus",
                "Super+Shift+Home": "desktop/columns/place",
                "Super+R": "desktop/columns/width",
                "Super+T": "desktop/columns",
                "Super+3": "desktop/workspaces/focus",
                "Super+Shift+3": "desktop/workspaces/column",
                "Super+Alt+3": "desktop/workspaces/window",
                "Super+Page_Down": "desktop/workspaces/near",
                "Super+Shift+Up": "desktop/workspaces/near",
                "Super+A": "desktop/workspaces",
                "Super+Ctrl+Left": "desktop/screens/focus",
                "Super+Ctrl+Shift+Left": "desktop/screens/column",
                "Super+Ctrl+Alt+Left": "desktop/screens/window",
                "XF86MonBrightnessUp": "desktop/screens/this",
                "Super+L": "desktop/screens/this",
                "Super+Ctrl+Shift+Page_Up": "desktop/screens",
                "Super+N": "desktop/launcher/notices",
                "XF86DoNotDisturb": "desktop/launcher/notices",
                "Super+Space": "desktop/launcher",
                "Super+E": "desktop/typing",
                "Super+D": "desktop/typing",
                "Ctrl+Print": "desktop/capture",
                // Eight rows once Print and Super+P are one, so read whole.
                "Super+Print": "desktop/capture",
                "XF86AudioPlay": "desktop/sound"
            },
            neovim: {
                "Space f Space": "neovim/files/finding",
                "Space f e": "neovim/files/browser",
                "Return in the file browser": "neovim/files/browser",
                "- in the file browser": "neovim/files/folders",
                "Space b w": "neovim/files/open",
                "Space Left": "neovim/files/splits",
                "Space T Return": "neovim/files/tabs",
                "Space 3": "neovim/files/numbers",
                "Space Space": "neovim/search",
                "Space s p": "neovim/search/text",
                "Space s y": "neovim/search/symbols",
                "Space c Space": "neovim/search/symbols",
                "Space s H": "neovim/search/lists",
                "g s": "neovim/search/jumps",
                "g d in code": "neovim/code/definitions",
                "Space c a in code": "neovim/code/hints",
                "Space c R in code": "neovim/code/refactoring",
                "Space c f": "neovim/code/formatting",
                "Space c s s": "neovim/code/selecting",
                "a f selecting, or after an operator": "neovim/code/pieces",
                "] f": "neovim/code/through",
                "[ C": "neovim/code/through",
                "Space k a in code": "neovim/code/server",
                "Space i l": "neovim/code/server",
                "Space e E": "neovim/problems/finding",
                "Space e t d": "neovim/problems/showing",
                "] d": "neovim/problems/through",
                "Return in the problems list": "neovim/problems/opening",
                "R in the problems list": "neovim/problems/list",
                "z M in the problems list": "neovim/problems/folding",
                "Space c o o": "neovim/outline/showing",
                "Return in the outline": "neovim/outline/symbols",
                "Return walking the outline": "neovim/outline/symbols",
                "} in the outline": "neovim/outline/through",
                "L in the outline": "neovim/outline/holding",
                "z a in the outline": "neovim/outline/folding",
                "h walking the outline": "neovim/outline/walking",
                "Space t t": "neovim/tests/running",
                "] t": "neovim/tests/results",
                "Space t Space": "neovim/tests",
                "] g in a file git tracks": "neovim/git/changes",
                "Space g s A in a file git tracks": "neovim/git/staging",
                "Space g c Return": "neovim/git/commits",
                "Space g t Return": "neovim/git/branches",
                "Space g a s": "neovim/git/stashes",
                "Space g n p": "neovim/git/remote",
                "c c in git's status": "neovim/git/status",
                "Space g Space": "neovim/git",
                "Space g h p Space": "neovim/pulls/finding",
                "Space g h m o in a pull request": "neovim/pulls/pull",
                "Space g h m s in a pull request": "neovim/pulls/merging",
                "Space g h k o in a pull request": "neovim/pulls/checks",
                "Space g h v s in a pull request": "neovim/pulls/reviewing",
                "Space g h T r in a pull request": "neovim/pulls/threads",
                "] C in a review's change": "neovim/pulls/threads",
                "] v in a review's thread": "neovim/pulls/files",
                "j in a review's changed files": "neovim/pulls/files",
                "Space g h v S in a review's change": "neovim/pulls/submitting",
                "Ctrl+A submitting a review": "neovim/pulls/submitting",
                "Space g h i Space": "neovim/github/issues",
                "Space g h b w in a GitHub issue": "neovim/github/issues",
                "Space g h l a in a pull request": "neovim/github/people",
                "Space g h c s in a review's thread": "neovim/github/comments",
                "Space g h c Return in a GitHub issue": "neovim/github/comments",
                "] C in a pull request": "neovim/github/comments",
                "Space g h t + in a review's thread": "neovim/github/reactions",
                "Return in a workflow run": "neovim/github/runs",
                "Space g h r f": "neovim/github/repositories",
                "Space g h g Space": "neovim/github/repositories",
                "Ctrl+R finding GitHub's notifications": "neovim/github/notifications",
                "Space g h Space": "neovim/github",
                "Tab on the command line": "neovim/editor/suggestions",
                "Ctrl+N on the command line": "neovim/editor/commands",
                "Space u c": "neovim/editor/looks",
                "Space i x": "neovim/editor/looks",
                "Space i r": "neovim/editor/insides",
                "Space Q": "neovim/editor",
                "Shift+Escape in a terminal": "neovim/editor"
            },
            pi: {
                "enter": "pi/message",
                "ctrl+-": "pi/editing",
                "ctrl+t": "pi/conversation",
                "pageUp": "pi/scrollback",
                "ctrl+l": "pi/models",
                "escape": "pi"
            },
            terminal: {
                "ctrl+shift+t": "terminal/tabs",
                "ctrl+shift+3": "terminal/numbers",
                "shift+page_up": "terminal/scrollback",
                "ctrl+shift+h": "terminal/clipboard",
                "ctrl+shift+equal": "terminal/text",
                "ctrl+shift+e": "terminal"
            }
        };
        for (const layer of Object.keys(deep))
            for (const chord of Object.keys(deep[layer]))
                expect(`${layer}'s ${chord}`, (levels[layer] ?? {})[chord] ?? "not bound",
                    deep[layer][chord]);

        // The window the launcher opened over puts its program's layers
        // first: kitty's the terminal's, Neovim's or pi's before it when its
        // title says one runs there, as Neovim's titlestring and pi's own
        // title do, and anything else only the desktop's.
        const programs = [
            ["kitty", "\u{1F4DD} notes.md", ["neovim", "terminal", "desktop"]],
            ["kitty", "nvim notes.md", ["neovim", "terminal", "desktop"]],
            ["kitty", "\u03C0 - Y2YBTS A Launcher T\u2026 - core.nix", ["pi", "terminal",
                "desktop"]],
            ["kitty", "pi - core.nix", ["pi", "terminal", "desktop"]],
            ["kitty", "\u03C0 - Q1 Tidy My Vim Config - notes", ["pi", "terminal", "desktop"]],
            ["kitty", "~/src/vim-config", ["terminal", "desktop"]],
            ["kitty", "~/src/core.nix", ["terminal", "desktop"]],
            ["kitty", "pipewire", ["terminal", "desktop"]],
            ["google-chrome", "\u{1F4DD} notes.md", ["desktop"]],
            ["", "", ["desktop"]]
        ];
        for (const program of programs)
            expect(`over ${program[0]} titled ${program[1]}`, KeyGroups.over(program[0],
                program[1]), program[2]);

        // Every group the table lists is named in the vocabulary, inside
        // Shortcuts, no more than two levels below its layer, and a desktop
        // group wears a glyph.
        for (const path of KeyGroups.paths()) {
            const id = `shortcuts:${path.split("/").join(":")}`;
            const entry = known[id];
            expect(`${id} named`, !!entry && (entry.name ?? "") !== "", true);
            expect(`${id} inside Shortcuts`, entry ? entry.in : "", "shortcuts");
            expect(`${id} near its layer`, path.split("/").length - 1 <= deepest, true);
            if (path.startsWith("desktop/"))
                expect(`${id}'s glyph`, !!entry && (entry.glyph ?? "") !== "", true);
        }

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
