// Every open window, found by its title and by its program's name, the most
// recently used first, each saying where it is: this window, here on the
// focused workspace, or the workspace it's on. Return goes to it. What niri
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
                    run: () => Niri.act({ FocusWindow: { id: window.id } })
                });
            });
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
