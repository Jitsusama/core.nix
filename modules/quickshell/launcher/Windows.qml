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
// says of its windows comes ready from the windows daemon (modules/windows),
// which hears niri's event stream and tells of only the windows a change
// moved. Until its first answer, and while it's down, what has the keys
// and the workspaces a verb offers still come from Niri.qml, which hears
// the stream too until the shell's last reader of it moves here.
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
import "HeldWindows.js" as HeldWindows
import "KeyLayers.js" as KeyLayers
import "Order.js" as Order

Provider {
    id: provider

    what: "window"

    // The programs a terminal's window can be running, by the layer of keys
    // its title gives it (KeyLayers.js's over), as their glyphs know them.
    readonly property var inTerminal: ({ pi: "pi", neovim: "nvim", btop: "btop" })

    // A window's glyph: its program's, by its entry's id, or, in a
    // terminal, that of the program its title says it's running.
    function glyphOf(window, entry) {
        const running = inTerminal[KeyLayers.over(window.app_id ?? "", window.title ?? "")[0]];
        return programGlyph(running ?? entry ?? window.app_id);
    }

    // Before the daemon's first answer it holds no windows it can say are
    // all there are, so it says it's still reading rather than none; and
    // with the daemon down it says so, rather than reading forever or
    // saying there are none (Daemon.qml's gone).
    readonly property bool silent: daemon.state === "not answering"
    collection: Words.collection("windows", Object.assign({ searched: false },
        silent ? { empty: Words.of("windows", "silent").empty }
            : daemon.answered ? {} : { empty: "\u2026" }))

    // Its row says so too while the daemon is down, since the rows it
    // keeps through the gap may no longer be true.
    function noted() {
        return silent ? Words.of("windows").said.silent : "";
    }
    stillReading: daemon.live && !daemon.answeredThisWatch && daemon.state !== "not answering"

    // An open list follows windows opening, closing and being renamed, and
    // only those, not each event niri sends.
    onThingsChanged: changedTo(things)

    // The window Joel's in is left out. Until this watch's first answer the
    // rows are those the last opening made, and a window picked from them
    // then moved the keys unheard, since the watch ended as the launcher
    // closed; so niri's own word on where the keys are holds until the
    // answer, or Super+Tab then Return would go to the window he's in.
    function everything() {
        const heard = daemon.answeredThisWatch;
        const others = things.filter(window => window.is !== "windows:this"
            && (heard ? !window.focused : window.id !== Niri.current));
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

    // The windows as the daemon says them, in the order they're listed.
    // Each event niri sent once walked every window here, all 75 of Joel's,
    // launcher shut or open (2026-11-10); the daemon now tells of only the
    // windows a change moved, and only while they're wanted (Provider.qml's
    // wanted), so a shut launcher hears nothing and keeps the rows it last
    // made, and its first frame draws them.
    readonly property var held: HeldWindows.kept()
    // Each window's row by its record's key, and This Window's, made again
    // only for a record that changed. Plain objects, so keeping one
    // notifies nothing.
    readonly property var kept: ({})
    readonly property var thisOne: ({ key: "", row: null })
    // The windows whose rows say they wait on Joel, by id.
    readonly property var waitingOf: ({ ids: {} })

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        // The workspaces and the desktop too, which a window's Move to
        // Workspace offers from (workspacesFor).
        kind: ["window", "workspace", "desktop"]
        live: provider.wanted
        paced: provider.launcher
        answersWhole: true
        onHeard: (changed, gone) => provider.hear(changed, gone)
    }

    // A window waiting on Joel says so, which comes from Sessions rather
    // than the daemon, so only the rows whose waiting turned are made again.
    Connections {
        target: Sessions
        function onWaitingWindowsChanged() {
            provider.waitingTurned();
        }
    }
    onWantedChanged: waitingTurned()

    // Takes what the daemon said, making again only the rows of windows
    // whose records differ, so a whole answer as the launcher opens makes
    // nothing anew that reads the same. How many records it heard of is
    // counted (Provider.qml's tally), for the check that a change is heard
    // only of the windows it moved, never all of them.
    readonly property var windowsDaemon: daemon
    function hear(changed, gone) {
        const windowsOnly = keys => keys.filter(key => key.startsWith("window:"));
        const told = windowsOnly(changed);
        const went = windowsOnly(gone);
        provider.tally.looked += told.length + went.length;
        const records = told.map(key => daemon.held[key]).filter(record => record);
        remake(HeldWindows.take(provider.held, records, went), went);
    }

    function waitingTurned() {
        if (!provider.wanted)
            return;
        const now = {};
        for (const id of Sessions.waitingWindows.split(" "))
            now[id] = true;
        const was = provider.waitingOf.ids;
        provider.waitingOf.ids = now;
        const turned = Object.keys(provider.held.records).filter(key => {
            const id = String(provider.held.records[key].id);
            return (id in now) !== (id in was);
        });
        provider.tally.looked += turned.length;
        remake(turned, []);
    }

    // Makes again the rows of these keys and lets those gone go, then lists
    // them as held, This Window first.
    function remake(keys, gone) {
        if (keys.length === 0 && gone.length === 0)
            return;
        for (const key of gone) {
            delete provider.kept[key];
            if (key === provider.thisOne.key)
                Object.assign(provider.thisOne, { key: "", row: null });
        }
        for (const key of keys) {
            const record = provider.held.records[key];
            const one = provider.named(record);
            provider.kept[key] = provider.ownRow(one, provider.saidOf(record));
            // The window Joel's in is This Window too, made from the same.
            if (record.focused)
                Object.assign(provider.thisOne, { key: key, row: provider.thisWindow(one) });
            else if (key === provider.thisOne.key)
                Object.assign(provider.thisOne, { key: "", row: null });
        }
        provider.things = (provider.thisOne.row ? [provider.thisOne.row] : [])
            .concat(provider.held.order.map(key => provider.kept[key]));
    }

    // A window's names, as its record says them: its program's entry by id,
    // the program's own name, and its title tidied of the program's.
    function named(record) {
        return {
            window: provider.held.windows[record.key],
            entry: record.entry,
            program: record.program,
            title: record.named,
            untitled: record.untitled
        };
    }

    // What a window's row says of where it is and how it stands.
    function saidOf(record) {
        const workspace = record.workspace === null ? null
            : { id: record.workspace, idx: record.workspace_idx };
        return {
            where: provider.placeOf(provider.held.windows[record.key], workspace,
                record.elsewhere ? null : workspace, record.alike),
            workspace: workspace ? workspace.idx : null,
            focused: record.focused,
            waiting: String(record.id) in provider.waitingOf.ids
        };
    }

    // What Shift+Return opens below a window: another of its program, its
    // entry found as it's pressed, since the record names it by id.
    function belowOf(window, entry) {
        return entry === null ? null : () => {
            const found = DesktopEntries.byId(entry);
            if (found)
                provider.launcher.grid.under(window.id, provider.command(found));
        };
    }

    // A window's own row, from its names (`named`) and what its row says
    // of where it is (`said`).
    function ownRow(one, said) {
        const window = one.window;
        const entry = one.entry;
        const program = one.program;
        const title = one.title;
        const below = provider.belowOf(window, entry);
        return thing({
            // What learning knows it by, which outlives the window.
            thing: true,
            key: `window:${window.app_id}:${title}`,
            // What it is now: this window, or its program when the
            // program's name is all its title says.
            is: one.untitled && entry ? `program:${entry}` : `window:${window.id}`,
            id: window.id,
            program: entry ?? window.app_id ?? "",
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
        const below = provider.belowOf(window, one.entry);
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

    // The lit screens a window isn't on, as Screens names them, or none when
    // there's one screen, or no Screens to ask.
    function screensFor(window) {
        const displays = (provider.launcher?.sources ?? [])
            .find(source => source.settings.name === "displays") ?? null;
        const here = daemon.held[`workspace:${window.workspace_id}`]?.output ?? "";
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

    // The workspaces a window could go to, on its own screen in order, each
    // saying what's on it, as the windows daemon holds them. niri always
    // keeps an empty one last, which is how a new workspace is made, so that
    // one is offered as new.
    function workspacesFor(window) {
        const said = daemon.held;
        const current = said.desktop?.current ?? null;
        // The chord that moves the focused window there, to learn next time.
        const chordTo = idx => {
            const does = `Move the window to workspace ${idx}`;
            const action = Shortcuts.actions.find(a => a.does === does);
            return action ? action.chord.map(key => key.text).join("+") : "";
        };
        return HeldWindows.spacesFrom(said, window.workspace_id).map(space => {
            const chord = window.id === current ? chordTo(space.idx) : "";
            return thing({
                key: `workspace:${space.idx}`,
                verb: "move",
                name: space.new ? "A new workspace" : space.name ?? `Workspace ${space.idx}`,
                count: space.focused ? "here" : "",
                glyph: space.new ? Theme.glyph.newWorkspace : Theme.glyph.workspace,
                hue: "machine",
                side: space.new ? ""
                    : space.inside.length === 0 ? "empty" : space.inside.join(", "),
                note: chord === "" ? "" : `next time ${chord}`,
                run: () => Niri.act({
                    MoveWindowToWorkspace: {
                        window_id: window.id,
                        reference: { Id: space.id },
                        focus: true
                    }
                })
            });
        });
    }
}
