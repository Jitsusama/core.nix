// Every open window, found by its own title, the most recently used first,
// each saying its program and, when that's another, the workspace it's on;
// windows whose titles read the same say their column as well, and how
// far down it, so no two rows read alike. A window titled only by its program is that program's
// row (Applications.qml), which lists its windows. Its program's name
// finds the program, not each of its windows, so "kitty" is one row and
// not every terminal open (Joel, 2026-10-23). Return goes to it, and
// Shift+Return opens another of its program below it. Its verbs move it to
// another workspace, float or tile it, copy its title or close it. What niri
// says of its windows comes through Niri, from niri's own event stream, so
// finding a window costs no process.
//
// They're also a collection, which Super+Tab opens on its own: every window
// but this one, the one used last first, so Super+Tab and Return go back to
// it and a few letters find any other, there by its program's name too,
// since everything there is a window. It adds no row of its own to the
// search, where each window is already found by itself.
import Quickshell
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "window"

    collection: Words.collection("windows", { searched: false })

    // An open list follows windows opening, closing and being renamed, and
    // only those, not each event niri sends.
    onThingsChanged: changedTo(things)

    function everything() {
        return things.filter(window => !window.focused);
    }

    // Found by title, or by program more loosely; the tidiest matches kept,
    // and among equals the one used last.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const others = everything();
        const found = [];
        others.forEach((window, recency) => {
            const byName = (window.prepared.mask & need) === need
                ? Fuzzy.match(window.prepared, words, false) : { score: -1 };
            if (byName.score >= 0) {
                found.push({ item: window.prepared, score: byName.score, grade: byName.grade,
                    recency: recency, choice: window });
                return;
            }
            if ((window.owner.mask & need) !== need)
                return;
            const byWord = Fuzzy.match(window.owner, words, false);
            if (byWord.score >= 0)
                found.push({ item: window.prepared, score: byWord.score / 4, grade: byWord.grade,
                    recency: recency, choice: window });
        });
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = found.filter(f => f.grade === grade)
            .sort((a, b) => b.score - a.score || a.recency - b.recency);
        return { rows: kept.map(f => f.choice), grade: grade };
    }

    things: {
        Niri.revision;
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
        return named.map(one => {
                const window = one.window;
                const entry = one.entry;
                const program = one.program;
                const title = one.title;
                const workspace = Niri.workspaces[window.workspace_id];
                const where = provider.placeOf(window, workspace, here, alike[title]);
                const below = entry
                    ? () => provider.launcher.grid.under(window.id, provider.command(entry)) : null;
                return thing({
                    // What learning knows it by, which outlives the window.
                    key: `window:${window.app_id}:${title}`,
                    // What it is now: this window, or its program when the
                    // program's name is all its title says.
                    is: one.untitled && entry ? `program:${entry.id}` : `window:${window.id}`,
                    id: window.id,
                    program: entry ? entry.id : (window.app_id ?? ""),
                    owner: Fuzzy.prepare(program),
                    verb: "switch",
                    name: title,
                    glyph: programGlyph(entry ? entry.id : window.app_id),
                    hue: "program",
                    side: [title === program ? "" : program, where]
                        .filter(part => part !== "").join(" \u00b7 "),
                    context: workspace ? `on workspace ${workspace.idx}` : "",
                    focused: window.id === Niri.current,
                    run: () => Niri.act({ FocusWindow: { id: window.id } }),
                    below: below,
                    verbs: () => provider.verbs(window, title, program, below)
                });
            });
    }

    // What's in front of you comes first at home, its verbs one arrow away.
    function home() {
        return things.filter(window => window.focused);
    }

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // An act on a window, named, worded and worn as the vocabulary says
    // (Words.qml), in the state it's in: one word for Return, a short name.
    function act(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            keywords: Words.keywords(key, state),
            glyph: Words.glyph(key, state),
            hue: "action"
        }, fields));
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
        if (below)
            all.push(act("verb:below", "", {
                // What it opens, so the name needn't say it.
                side: program,
                keywords: `${Words.keywords("verb:below")} open another ${program} below it`,
                chord: provider.launcher.belowChord,
                run: below
            }));
        all.push(
            act("verb:float", window.is_floating ? "floating" : "", {
                run: () => Niri.act({ ToggleWindowFloating: { id: id } })
            }),
            act("verb:title", "", {
                // Said while the launcher still holds the keys, which a
                // Wayland program needs to set the clipboard.
                now: true,
                run: () => Quickshell.clipboardText = window.title ?? title
            }),
            act("verb:close", "", {
                run: () => Niri.act({ CloseWindow: { id: id } })
            })
        );
        return all;
    }

    // The workspaces a window could go to, on its own screen in order, each
    // saying what's on it. niri always keeps an empty one last, which is
    // how a new workspace is made, so that one is offered as new.
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
                glyph: glyph(last ? 0xf11da : 0xf11d9),
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
