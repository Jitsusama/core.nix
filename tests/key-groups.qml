// Shortcuts' groups, run in Qt's own JavaScript engine, the one the shell
// runs them in: the-launcher-groups-every-shortcut copies
// launcher/KeyGroups.js beside this file with an account's own binds, every
// layer's, as layers.json, and its vocabulary, as words.json, and runs
// `qml -platform offscreen key-groups.qml`. Every desktop chord falls in a
// group, each chord of a grouped layer is listed once, the chords below
// land where Joel would look for them, a chord niri adds tomorrow finds a
// group without being told, a bind that names its group is put there, a
// short layer stays whole, a big group is split into groups of its own, so
// no level lists more than about twenty chords and Code's and Problems'
// groups hold a dozen or fewer, and every group the table lists is named in
// the vocabulary, a desktop group with a glyph. It prints each group's
// count.
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

    // No level lists more chords of its own than this.
    readonly property int most: 20
    // Nor does a group inside Neovim's Code or Problems hold more than this.
    readonly property int dozen: 12

    // Every place a layer lists a chord, each group's said to `counts`, and
    // the level each chord is listed in to `where`, by its place.
    function placed(arranged, path, counts, where) {
        let out = arranged.loose.slice();
        for (const at of arranged.loose)
            where[at] = path;
        if (arranged.loose.length > most)
            counts.push(`FAILED ${path} lists ${arranged.loose.length} chords of its own`);
        for (const group of arranged.groups) {
            const inner = `${path}/${group.id}`;
            counts.push(`${inner} ${group.at.length}`);
            if (/^neovim\/(code|problems)\//.test(inner) && group.at.length > dozen)
                counts.push(`FAILED ${inner} holds ${group.at.length}, more than ${dozen}`);
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

        // Each chord of a layer is listed once, and a desktop chord is never
        // left out of the groups.
        const counts = [];
        const levels = {};
        for (const layer of Object.keys(layers)) {
            const binds = layers[layer];
            const arranged = KeyGroups.arrange(layer, binds);
            const listed = {};
            const places = placed(arranged, layer, counts, listed);
            levels[layer] = {};
            binds.forEach((bind, at) => {
                levels[layer][`${bind.chord} ${bind.when}`.trim()] = listed[at];
            });
            expect(`${layer}'s chords, each once`, places.slice().sort((a, b) => a - b),
                binds.map((bind, at) => at));
            if (layer === "desktop")
                expect("the desktop's chords in no group",
                    arranged.loose.map(at => binds[at].chord), []);
        }
        for (const line of counts)
            if (line.startsWith("FAILED"))
                failures.push(line);
        console.warn(counts.join("\n"));

        // The desktop's chords where Joel would look for them.
        const desktop = {};
        for (const bind of layers.desktop ?? [])
            desktop[bind.chord] = KeyGroups.groupOf("desktop", bind);
        const where = {
            "Super+W": "windows",
            "Super+Up": "windows",
            "Super+G": "windows",
            "Super+Alt+Return": "windows",
            "Super+Left": "columns",
            "Super+Shift+Left": "columns",
            "Super+R": "columns",
            "Super+3": "workspaces",
            "Super+Shift+3": "workspaces",
            "Super+A": "workspaces",
            "Super+Ctrl+Left": "screens",
            "Super+Ctrl+Alt+Left": "screens",
            "Super+Ctrl+Shift+Page_Up": "screens",
            "Super+Return": "programs",
            "Super+Shift+Return": "programs",
            "Super+Space": "launcher",
            "Super+Shift+Slash": "launcher",
            "Super+N": "launcher",
            "Print": "capture",
            "Shift+Print": "capture",
            "Ctrl+Print": "capture",
            "Super+Print": "capture",
            "XF86AudioRaiseVolume": "sound",
            "XF86AudioPlay": "sound",
            "XF86MonBrightnessUp": "brightness",
            "Super+D": "dictation",
            "Super+L": "session",
            "XF86ScreenSaver": "session"
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
        expect("quitting", KeyGroups.groupOf("desktop", niri("quit")), "session");
        expect("a new program", KeyGroups.groupOf("desktop", spawn(["firefox"])), "programs");
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

        // Neovim's keys by its leader's letters, and the window they work in.
        const neovim = {};
        for (const bind of layers.neovim ?? [])
            neovim[`${bind.chord} ${bind.when}`.trim()] = KeyGroups.groupOf("neovim", bind);
        const neovimWhere = {
            "Space f Space": "files",
            "Space b w": "open",
            "Space T Return": "tabs",
            "Space 3": "tabs",
            "Space Left": "splits",
            "[ d": "problems",
            "] t": "tests",
            "Space g c Return": "git",
            "Space g h p Space": "github",
            "Return in the outline": "outline",
            "Return in the file browser": "files",
            "g d in code": "code"
        };
        for (const chord of Object.keys(neovimWhere))
            expect(`Neovim's ${chord}`, neovim[chord] ?? "not bound", neovimWhere[chord]);

        // A big group's chords where Joel would look for them, the level
        // each is listed in.
        const deep = {
            desktop: {
                "Super+3": "desktop/workspaces/going",
                "Super+Page_Down": "desktop/workspaces/going",
                "Super+Shift+3": "desktop/workspaces/column",
                "Super+Alt+3": "desktop/workspaces/window",
                "Super+A": "desktop/workspaces"
            },
            neovim: {
                "Space f Space": "neovim/files/finding",
                "Space f e": "neovim/files/browser",
                "Return in the file browser": "neovim/files/browser",
                "g d in code": "neovim/code/definitions",
                "Space c c i in code": "neovim/code/definitions",
                "Space c a in code": "neovim/code/hints",
                "Space c R in code": "neovim/code/refactoring",
                "Space c f": "neovim/code/formatting",
                "a f selecting, or after an operator": "neovim/code/selecting",
                "] f": "neovim/code/through",
                "Space k a in code": "neovim/code/server",
                "Space e t d": "neovim/problems/showing",
                "] d": "neovim/problems/through",
                "Return in the problems list": "neovim/problems/opening",
                "R in the problems list": "neovim/problems/list",
                "z M in the problems list": "neovim/problems/folding",
                "Space c o o": "neovim/outline/showing",
                "} in the outline": "neovim/outline/symbols",
                "z a in the outline": "neovim/outline/folding",
                "Return walking the outline": "neovim/outline/walking",
                "] g in a file git tracks": "neovim/git/changes",
                "Space g s A in a file git tracks": "neovim/git/staging",
                "Space g c Return": "neovim/git/commits",
                "Space g t Return": "neovim/git/branches",
                "Space g a s": "neovim/git/stashes",
                "Space g n p": "neovim/git/remote",
                "c c in git's status": "neovim/git/status",
                "Space g h p Space": "neovim/github/pulls/finding",
                "Space g h m o in a pull request": "neovim/github/pulls/pull",
                "Space g h m s in a pull request": "neovim/github/pulls/merging",
                "Space g h v s in a pull request": "neovim/github/pulls/reviewing",
                "Space g h l a in a pull request": "neovim/github/pulls/people",
                "Space g h T r in a pull request": "neovim/github/pulls/comments",
                "Space g h t + in a pull request": "neovim/github/pulls/reactions",
                "Space g h i Space": "neovim/github/issues/finding",
                "Space g h b w in a GitHub issue": "neovim/github/issues/issue",
                "Space g h t + in a GitHub issue": "neovim/github/issues/reactions",
                "Space g h c s in a review's thread": "neovim/github/reviews/threads/comments",
                "Space g h t + in a review's thread": "neovim/github/reviews/threads/reactions",
                "] v in a review's thread": "neovim/github/reviews/threads",
                "Space g h v S in a review's change": "neovim/github/reviews/diffs",
                "j in a review's changed files": "neovim/github/reviews/files",
                "Ctrl+A submitting a review": "neovim/github/reviews/submitting",
                "Return in a workflow run": "neovim/github/runs",
                "Space g h r f": "neovim/github/repositories",
                "Space g h g Space": "neovim/github/gists",
                "Ctrl+R finding GitHub's notifications": "neovim/github/notifications",
                "Space g h m q Space": "neovim/github/keys"
            }
        };
        for (const layer of Object.keys(deep))
            for (const chord of Object.keys(deep[layer]))
                expect(`${layer}'s ${chord}`, (levels[layer] ?? {})[chord] ?? "not bound",
                    deep[layer][chord]);

        // Every group the table lists is named in the vocabulary, inside
        // Shortcuts, and a desktop group wears a glyph.
        for (const path of KeyGroups.paths()) {
            const id = `shortcuts:${path.split("/").join(":")}`;
            const entry = known[id];
            expect(`${id} named`, !!entry && (entry.name ?? "") !== "", true);
            expect(`${id} inside Shortcuts`, entry ? entry.in : "", "shortcuts");
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
