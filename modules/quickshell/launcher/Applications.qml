// Every application with a desktop entry, found by its name and by what it
// is: "browser" finds Chrome. A program is one row whether it's running or
// not, saying how many windows it has open: Return switches to the one
// used last, or starts it when none is, and Shift+Return opens a new one
// below the focused window. The right arrow lists its windows by title,
// then what else can be done. A desktop action that opens just the
// program, as Super+Return opens kitty, is its row too: the row answers to
// the action's words ("open a terminal") and the foot teaches its chord,
// so the two aren't listed side by side.
import Quickshell
import qs
import QtQuick

Provider {
    id: provider

    what: "program"

    // The desktop's actions that run exactly this command.
    function opening(command) {
        const run = Array.from(command).join("\u0000");
        return Shortcuts.actions.filter(action => action.run.join("\u0000") === run);
    }

    draws: things.reduce((all, row) => all.concat(row.stands), [])

    // Each program's open windows by its desktop entry's id, the one used
    // last first, followed as niri's windows change. Read when a row is
    // drawn or run, so a window opening never prepares every program again.
    readonly property var running: {
        Niri.revision;
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        const by = {};
        for (const window of Object.values(Niri.windows).sort((a, b) => when(b) - when(a))) {
            const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
            if (entry)
                (by[entry.id] ?? (by[entry.id] = [])).push(window);
        }
        return by;
    }

    // How many windows a program has open, as the row says it beside the
    // name, before what it is.
    function windowCount(entry) {
        const open = (running[entry.id] ?? []).length;
        return open === 0 ? "" : open === 1 ? "1 window" : `${open} windows`;
    }

    // Its windows as rows Return goes to, each by its own title, saying
    // where it is as the Windows collection does: nothing on the workspace
    // in front, and a column when two are titled alike.
    function windowsOf(entry) {
        const windows = running[entry.id] ?? [];
        const here = Niri.workspace();
        const titled = window => provider.tidy(window.title ?? "", entry.name) || entry.name;
        const alike = {};
        for (const window of windows)
            alike[titled(window)] = (alike[titled(window)] ?? 0) + 1;
        return windows.map(window => {
            const workspace = Niri.workspaces[window.workspace_id];
            return thing({
                key: `window:${window.app_id}:${provider.tidy(window.title ?? "", entry.name)}`,
                is: `window:${window.id}`,
                verb: "switch",
                name: titled(window),
                glyph: programGlyph(entry.id),
                hue: "program",
                side: provider.placeOf(window, workspace, here, alike[titled(window)]),
                run: () => Niri.act({ FocusWindow: { id: window.id } })
            });
        });
    }

    things: DesktopEntries.applications.values
        .filter(entry => !entry.noDisplay)
        .map(entry => {
            const start = () => Niri.spawn(provider.command(entry));
            const below = () => provider.launcher.grid.under(Niri.current, provider.command(entry));
            const binds = provider.opening(provider.command(entry));
            const row = thing({
                key: `program:${entry.id}`,
                is: `program:${entry.id}`,
                // A program says more than any one of its windows.
                rich: 1,
                name: entry.name,
                keywords: [entry.genericName, entry.comment]
                    .concat(Array.from(entry.keywords), binds.map(bind => `${bind.does} ${bind.also}`))
                    .join(" "),
                glyph: programGlyph(entry.id),
                hue: "program",
                side: entry.genericName.toLowerCase(),
                chord: binds.length > 0 ? binds[0].chord : [],
                stands: binds.map(bind => bind.does),
                // Then the action's words are what it's called, and count as
                // its name, as a collection's do (Launcher.qml).
                named: binds.length > 0,
                below: below,
                verbs: () => provider.windowsOf(entry).concat([
                    thing({
                        key: "verb:start",
                        name: "Start another",
                        glyph: String.fromCodePoint(0xf03cc),
                        hue: "action",
                        run: start
                    }),
                    thing({
                        key: "verb:below",
                        name: "Start another below this window",
                        glyph: String.fromCodePoint(0xf03cc),
                        hue: "action",
                        chord: provider.launcher.belowChord,
                        run: below
                    })
                ])
            });
            // Whether it's running, and so what Return does, is asked as
            // the row is read, never kept.
            const latest = () => (provider.running[entry.id] ?? [])[0] ?? null;
            Object.defineProperties(row, {
                verb: { enumerable: true, get: () => latest() ? "switch" : "start" },
                count: { enumerable: true, get: () => provider.windowCount(entry) },
                run: {
                    enumerable: true,
                    get: () => {
                        const window = latest();
                        return window ? () => Niri.act({ FocusWindow: { id: window.id } }) : start;
                    }
                }
            });
            return row;
        })
}
