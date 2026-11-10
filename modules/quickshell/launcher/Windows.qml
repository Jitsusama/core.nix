// Every open window, found by its own title, the most recently used first,
// each saying its program and, when that's another, the workspace it's on;
// windows whose titles read the same say their column as well, and how
// far down it, so no two rows read alike. A window titled only by its program is that program's
// row (Applications.qml), which lists its windows. Its program's name
// finds the program, not each of its windows, so "kitty" is one row and
// not every terminal open (Joel, 2026-10-23). Return goes to it, and
// Shift+Return opens another of its program below it. Its verbs move it to
// another workspace, float or tile it, copy its title or close it. A pi
// window is found by its work too, its quest's title and what its session
// says it's on, and one waiting on Joel says so beside its name. What niri
// says of its windows comes through Niri, from niri's own event stream, so
// finding a window costs no process.
//
// They're also a collection, which Super+Tab opens on its own: every window
// but this one, the one used last first, so Super+Tab and Return go back to
// it and a few letters find any other, there by its program's name too,
// since everything there is a window. Those waiting on Joel come next,
// before the rest, so what needs him is a key away. It adds no row of its own to the
// search, where each window is already found by itself.
import Quickshell
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../PiSource.js" as Pi
import "KeyLayers.js" as KeyLayers
import "Order.js" as Order

Provider {
    id: provider

    what: "window"

    // The programs a terminal's window can be running, by the layer of keys
    // its title gives it (KeyLayers.js's over), as their glyphs know them.
    readonly property var inTerminal: ({ pi: "pi", neovim: "nvim", btop: "btop" })

    // A window's glyph: its program's, or, in a terminal, that of the
    // program its title says it's running.
    function glyphOf(window, entry) {
        const running = inTerminal[KeyLayers.over(window.app_id ?? "", window.title ?? "")[0]];
        return programGlyph(running ?? (entry ? entry.id : window.app_id));
    }

    collection: Words.collection("windows", { searched: false })

    // An open list follows windows opening, closing and being renamed, and
    // only those, not each event niri sends.
    onThingsChanged: changedTo(things)

    function everything() {
        const others = things.filter(window => !window.focused);
        const rest = others.slice(1);
        return others.slice(0, 1).concat(rest.filter(window => window.waiting),
            rest.filter(window => !window.waiting));
    }

    // Every window's key as its row says it, and This Window's, for a pick
    // (Launcher.qml's knowing), read from niri with no row made: unwanted,
    // the rows are those last made, or none, so a pick would forget a
    // window opened since. A window's row asks for no confirming.
    function keysKnown() {
        const keys = Object.values(Niri.windows).map(window => {
            const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
            const program = entry ? entry.name : (window.app_id ?? "");
            const title = provider.tidy(window.title ?? "", program) || program;
            return { key: `window:${window.app_id}:${title}` };
        });
        return Niri.current === null ? keys : keys.concat([{ key: "windows:this" }]);
    }

    // What a pi window is about, ready to match, kept while it reads the
    // same, so a key prepares nothing new.
    property var abouts: new Map()
    readonly property int aboutsKept: 64
    function about(id) {
        const text = Sessions.aboutOf(id);
        if (text === "")
            return null;
        let prepared = abouts.get(text);
        if (prepared === undefined) {
            if (abouts.size > aboutsKept)
                abouts.clear();
            prepared = Fuzzy.prepare(text);
            abouts.set(text, prepared);
        }
        return prepared;
    }

    // How a window matches the words: by its title, else by its program or
    // its work, each worth a quarter as much, or not at all.
    function matched(window, words, need) {
        const tries = [
            { item: window.prepared, worth: 1 },
            { item: window.owner, worth: 4 },
            { item: about(window.id), worth: 4 }
        ];
        for (const one of tries) {
            if (one.item !== null && (one.item.mask & need) === need) {
                const hit = Fuzzy.match(one.item, words, false);
                if (hit.score >= 0)
                    return { score: hit.score / one.worth, grade: hit.grade };
            }
        }
        return null;
    }

    // Found by title, or by program or work more loosely; the tidiest
    // matches kept, and among equals the one used last.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const others = everything();
        const found = [];
        others.forEach((window, recency) => {
            const hit = matched(window, words, need);
            if (hit !== null)
                found.push({ item: window.prepared, score: hit.score, grade: hit.grade,
                    recency: recency, choice: window });
        });
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = Order.sorted(found.filter(f => f.grade === grade),
            (a, b) => b.score - a.score || a.recency - b.recency);
        return { rows: kept.map(f => f.choice), grade: grade };
    }

    // Each window's row, by its id, with what it was made from. Every event
    // niri sends moves its revision, and a focus moving changes only the two
    // windows it moved between, yet every window's row was made again: 348
    // rows over 8 changes in one closing on Joel's shell (2026-11-04). A
    // row is now made again only when its window's record, its workspace's
    // place or what its row says of it changed. A plain object, so keeping
    // one notifies nothing.
    readonly property var kept: ({})
    // The rows as they were last made, held while nobody wants them.
    readonly property var madeRows: ({ rows: [] })

    // Made only while they're wanted (Provider.qml's wanted), since with the
    // launcher shut every event niri sent walked all of Joel's 75 windows and
    // made 657 rows in four minutes, for nobody (2026-11-10). Unwanted, it
    // reads nothing of niri, so no event wakes it, and says the rows last
    // made until it's wanted again.
    things: {
        if (!provider.wanted)
            return provider.madeRows.rows;
        void Niri.revision;
        const waiting = Sessions.waitingWindows.split(" ");
        const here = Niri.workspace();
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        const named = Object.values(Niri.windows)
            .sort((a, b) => when(b) - when(a))
            .map(window => {
                const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
                const program = entry ? entry.name : (window.app_id ?? "");
                const own = provider.tidy(window.title ?? "", program);
                return {
                    window: window,
                    entry: entry,
                    program: program,
                    title: own || program,
                    untitled: own === "" || own === program
                };
            });
        // How many windows each title is, so those alike say their column.
        const alike = {};
        for (const one of named)
            alike[one.title] = (alike[one.title] ?? 0) + 1;
        const next = {};
        const rows = named.map(one => {
            const window = one.window;
            const workspace = Niri.workspaces[window.workspace_id];
            const said = {
                where: provider.placeOf(window, workspace, here, alike[one.title]),
                workspace: workspace ? workspace.idx : null,
                focused: window.id === Niri.current,
                waiting: waiting.includes(String(window.id))
            };
            const from = { window: Object.assign({}, window),
                said: [one.entry ? one.entry.id : "", one.program, said] };
            const before = provider.kept[window.id];
            next[window.id] = provider.madeFrom(before, from)
                ? before : { from: from, row: provider.ownRow(one, said) };
            // The window Joel's in is This Window too, made from the same.
            if (said.focused) {
                const was = provider.kept.this;
                next.this = provider.madeFrom(was, from)
                    ? was : { from: from, row: provider.thisWindow(one) };
            }
            return next[window.id].row;
        });
        for (const id of Object.keys(provider.kept))
            delete provider.kept[id];
        Object.assign(provider.kept, next);
        provider.madeRows.rows = (next.this ? [next.this.row] : []).concat(rows);
        return provider.madeRows.rows;
    }

    // What niri says of a window that its row never says, left out of what
    // the row is made from: when it was focused orders the rows and whether
    // it is focused is `said`, both changing at every move of the focus,
    // and its layout, but for the column `said` gives, changes at every
    // scroll of the view.
    readonly property var unsaid: ["focus_timestamp", "is_focused", "layout"]

    // Whether a row kept was made from what a window says now. It's
    // compared in place rather than as text, since the text of Joel's 75
    // windows, about 30,000 characters, was made again at each change niri
    // told of (2026-11-10). The window is kept as a copy, since Niri.qml
    // sets its focus, urgency and layout in place.
    function madeFrom(kept, from) {
        return kept !== undefined && Pi.fieldsAlike(kept.from.window, from.window, unsaid)
            && Pi.alike(kept.from.said, from.said);
    }

    // A window's own row, from what niri says of it and what its row says
    // of where it is (`said`).
    function ownRow(one, said) {
        const window = one.window;
        const entry = one.entry;
        const program = one.program;
        const title = one.title;
        const below = entry
            ? () => provider.launcher.grid.under(window.id, provider.command(entry)) : null;
        return thing({
            // What learning knows it by, which outlives the window.
            thing: true,
            key: `window:${window.app_id}:${title}`,
            // What it is now: this window, or its program when the
            // program's name is all its title says.
            is: one.untitled && entry ? `program:${entry.id}` : `window:${window.id}`,
            id: window.id,
            program: entry ? entry.id : (window.app_id ?? ""),
            owner: Fuzzy.prepare(program),
            verb: "go to",
            name: title,
            glyph: provider.glyphOf(window, entry),
            hue: "program",
            side: [title === program ? "" : program, said.where]
                .filter(part => part !== "").join(" \u00b7 "),
            context: said.workspace !== null ? `on workspace ${said.workspace}` : "",
            focused: said.focused,
            waiting: said.waiting,
            count: said.waiting ? "waiting on you" : "",
            run: () => Niri.act({ FocusWindow: { id: window.id } }),
            below: below,
            verbs: () => provider.verbs(window, title, program, below),
            operands: () => provider.operands(window, title)
        });
    }

    // The window Joel's in, as Desktop's first row (the approved proposal):
    // named This window, its title beside, and Return or the right arrow
    // holding what can be done to it, all but going to it, since it's the
    // one he's in, and with making its column the screen's full width,
    // which niri does to the column in front, after floating it.
    function thisWindow(one) {
        const window = one.window;
        const entry = one.entry;
        const below = entry
            ? () => provider.launcher.grid.under(window.id, provider.command(entry)) : null;
        return thing({
            thing: true,
            key: "windows:this",
            is: "windows:this",
            verb: Words.verb("windows:this"),
            name: Words.name("windows:this"),
            glyph: Words.glyph("windows:this"),
            hue: "program",
            side: one.title,
            // The one Super+Tab leaves out, as it's where Joel is.
            focused: true,
            operands: () => provider.operands(window, one.title),
            verbs: () => {
                const all = provider.verbs(window, one.title, one.program, below)
                    .filter(row => row.key !== "verb:switch" && row.key !== "verb:bring");
                const floated = all.findIndex(row => row.key === "verb:float");
                all.splice(floated + 1, 0, provider.act("verb:full-width", "", {
                    run: () => Niri.act({ MaximizeColumn: {} })
                }));
                return all;
            }
        });
    }

    // An act on a window, named, worded and worn as the vocabulary says
    // (Words.qml), in the state it's in: one word for Return, a short name.
    function act(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            glyph: Words.glyph(key, state),
            hue: "action"
        }, Words.ending(key), fields));
    }

    // What can be done to a window, as its own row holds it, for the same
    // window shown elsewhere, as among its program's (Applications.qml), so
    // one thing has one set of insides wherever it shows (Joel, 2026-10-26).
    function actsOf(window, entry) {
        const program = entry ? entry.name : (window.app_id ?? "");
        const title = provider.tidy(window.title ?? "", program) || program;
        const below = entry
            ? () => provider.launcher.grid.under(window.id, provider.command(entry)) : null;
        return provider.verbs(window, title, program, below);
    }

    // What can be done to a window, the likeliest first.
    function verbs(window, title, program, below) {
        const id = window.id;
        const here = Niri.workspace();
        const all = [
            act("verb:switch", "", {
                run: () => Niri.act({ FocusWindow: { id: id } })
            })
        ];
        // Brought to the workspace in front, for one that's elsewhere.
        if (here && window.workspace_id !== here.id)
            all.push(act("verb:bring", "", {
                run: () => Niri.act({
                    MoveWindowToWorkspace: {
                        window_id: id,
                        reference: { Id: here.id },
                        focus: true
                    }
                })
            }));
        all.push(act("verb:move", "", {
            side: "\u2026",
            ask: "which workspace?",
            chip: "move to",
            targets: () => provider.workspacesFor(window)
        }));
        // Another screen, where there's one to go to, each named as Screens
        // names it (Displays.qml).
        const screens = provider.screensFor(window);
        if (screens.length > 0)
            all.push(act("verb:screen", "", {
                side: "\u2026",
                ask: "which screen?",
                chip: "move to",
                targets: () => screens
            }));
        all.push(act("verb:size", "", {
            side: "\u2026",
            ask: "how wide?",
            chip: "size",
            targets: () => provider.sizesFor(window)
        }));
        if (below)
            all.push(act("verb:below", "", {
                // What it opens, so the name needn't say it.
                side: program,
                chord: provider.launcher.belowChord,
                run: below
            }));
        all.push(
            act("verb:float", window.is_floating ? "floating" : "", {
                run: () => Niri.act({ ToggleWindowFloating: { id: id } })
            }),
            // Said while the launcher still holds the keys, which a
            // Wayland program needs to set the clipboard.
            act("verb:title", "", Object.assign({ now: true }, titling(window, title))),
            act("verb:close", "", closing(window))
        );
        return all;
    }

    // What d and y take of a window: its Close and its Copy the Title, as plain
    // fields rather than rows made searchable, since d or y over a place
    // of hundreds asks every row (Launcher.qml's actionsOf).
    function operands(window, title) {
        return [
            Object.assign({ key: "verb:title", verb: Words.verb("verb:title") },
                titling(window, title)),
            Object.assign({ key: "verb:close" }, Words.ending("verb:close"), closing(window))
        ];
    }

    function titling(window, title) {
        return {
            copies: () => window.title ?? title,
            run: () => Quickshell.clipboardText = window.title ?? title
        };
    }

    function closing(window) {
        return { run: () => Niri.act({ CloseWindow: { id: window.id } }) };
    }

    // The workspaces a window could go to, on its own screen in order, each
    // saying what's on it. niri always keeps an empty one last, which is
    // how a new workspace is made, so that one is offered as new.
    // The lit screens a window isn't on, as Screens names them, or none when
    // there's one screen, or no Screens to ask.
    function screensFor(window) {
        const displays = (provider.launcher?.sources ?? [])
            .find(source => source.settings.name === "displays") ?? null;
        const from = Niri.workspaces[window.workspace_id];
        const here = from ? from.output : "";
        const others = displays ? displays.lit.filter(output => output.name !== here) : [];
        return others.map(output => thing({
            key: `screen:${output.name}`,
            verb: "move",
            name: displays.named(output),
            glyph: Theme.glyph.display,
            hue: "machine",
            run: () => Niri.act({ MoveWindowToMonitor: { id: window.id, output: output.name } })
        }));
    }

    // The widths a window can be given, as shares of its screen, niri's
    // own proportions in percent.
    readonly property var widths: [["third", 100 / 3], ["half", 50], ["two-thirds", 200 / 3]]

    function sizesFor(window) {
        return widths.map(width => thing({
            key: `verb:size:${width[0]}`,
            verb: Words.verb(`verb:size:${width[0]}`),
            name: Words.name(`verb:size:${width[0]}`),
            glyph: Words.glyph("verb:size"),
            hue: "action",
            run: () => Niri.act({
                SetWindowWidth: { id: window.id, change: { SetProportion: width[1] } }
            })
        }));
    }

    function workspacesFor(window) {
        const from = Niri.workspaces[window.workspace_id];
        const output = from ? from.output : null;
        const here = Niri.workspace();
        const spaces = Object.values(Niri.workspaces)
            .filter(space => space.output === output)
            .sort((a, b) => a.idx - b.idx);
        const names = space => Object.values(Niri.windows)
            .filter(other => other.workspace_id === space.id)
            .map(other => {
                const entry = other.app_id ? DesktopEntries.heuristicLookup(other.app_id) : null;
                return entry ? entry.name : (other.app_id ?? "");
            });
        // The chord that moves the focused window there, to learn next time.
        const chordTo = idx => {
            const does = `Move the window to workspace ${idx}`;
            const action = Shortcuts.actions.find(a => a.does === does);
            return action ? action.chord.map(key => key.text).join("+") : "";
        };
        return spaces.map((space, at) => {
            const inside = names(space);
            const last = at === spaces.length - 1 && inside.length === 0;
            const chord = window.id === Niri.current ? chordTo(space.idx) : "";
            return thing({
                key: `workspace:${space.idx}`,
                verb: "move",
                name: last ? "A new workspace" : space.name ?? `Workspace ${space.idx}`,
                count: here && space.id === here.id ? "here" : "",
                glyph: last ? Theme.glyph.newWorkspace : Theme.glyph.workspace,
                hue: "machine",
                side: last ? "" : inside.length === 0 ? "empty" : inside.join(", "),
                note: chord === "" ? "" : `next time ${chord}`,
                run: () => Niri.act({
                    MoveWindowToWorkspace: {
                        window_id: window.id,
                        reference: { Id: space.id },
                        focus: true
                    }
                })
            });
        }).filter(target => target.key !== `workspace:${from ? from.idx : -1}`);
    }
}
