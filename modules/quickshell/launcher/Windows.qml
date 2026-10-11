// Every open window, found by its title and by its program's name, the most
// recently used first, each saying where it is: this window, here on the
// focused workspace, or the workspace it's on. Return goes to it, and
// Shift+Return opens another of its program below it. Its verbs move it to
// another workspace, float or tile it, copy its title or close it. What niri
// says of its windows comes through Niri, from niri's own event stream, so
// finding a window costs no process.
import Quickshell
import qs
import QtQuick

Provider {
    id: provider

    things: {
        Niri.revision;
        const here = Niri.workspace();
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        return Object.values(Niri.windows)
            .sort((a, b) => when(b) - when(a))
            .map(window => {
                const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
                const program = entry ? entry.name : (window.app_id ?? "");
                const title = provider.tidy(window.title ?? "", program) || program;
                const workspace = Niri.workspaces[window.workspace_id];
                const below = entry
                    ? () => provider.launcher.grid.under(window.id, provider.command(entry)) : null;
                return thing({
                    key: `window:${window.app_id}:${title}`,
                    verb: "switch",
                    name: title,
                    keywords: program,
                    glyph: programGlyph(entry ? entry.id : window.app_id),
                    hue: "program",
                    side: window.is_focused ? "this window"
                        : workspace && here && workspace.id === here.id ? "here"
                        : workspace ? `workspace ${workspace.idx}` : "",
                    context: workspace ? `on workspace ${workspace.idx}` : "",
                    focused: window.is_focused,
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

    // What can be done to a window, the likeliest first.
    function verbs(window, title, program, below) {
        const id = window.id;
        const all = [
            thing({
                key: "verb:switch",
                name: "Switch to it",
                glyph: glyph(0xf05b2),
                hue: "action",
                run: () => Niri.act({ FocusWindow: { id: id } })
            }),
            thing({
                key: "verb:move",
                verb: "choose",
                name: "Move it to a workspace",
                glyph: glyph(0xf11d9),
                hue: "action",
                side: "\u2026",
                ask: "which workspace?",
                chip: "move to",
                targets: () => provider.workspacesFor(window)
            })
        ];
        if (below)
            all.push(thing({
                key: "verb:below",
                name: `Open another ${program} below it`,
                glyph: glyph(0xf03cc),
                hue: "action",
                chord: provider.launcher.belowChord,
                run: below
            }));
        all.push(
            thing({
                key: "verb:float",
                name: window.is_floating ? "Tile it" : "Float it",
                glyph: glyph(0xf2d0),
                hue: "action",
                run: () => Niri.act({ ToggleWindowFloating: { id: id } })
            }),
            thing({
                key: "verb:title",
                name: "Copy its title",
                glyph: glyph(0xf018f),
                hue: "action",
                // Said while the launcher still holds the keys, which a
                // Wayland program needs to set the clipboard.
                now: true,
                run: () => Quickshell.clipboardText = window.title ?? title
            }),
            thing({
                key: "verb:close",
                name: "Close it",
                glyph: glyph(0xf05ad),
                hue: "action",
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
            const chord = window.is_focused ? chordTo(space.idx) : "";
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

    // A title without the program's name trailing it, as Chrome adds
    // " - Google Chrome", since the glyph already says whose window it is.
    function tidy(title, program) {
        if (program === "")
            return title;
        for (const joint of [" - ", " \u2013 ", " \u2014 ", " | "])
            if (title.endsWith(joint + program))
                return title.slice(0, title.length - joint.length - program.length);
        return title;
    }
}
