// Where Windows' acts can send a window, each offered as a row to pick
// once the act is chosen: another of its screen's workspaces, another lit
// screen, or a width. Windows.qml's verbs ask for them only as an act is
// opened, so a place of hundreds of windows makes none of them until then.
// They change with niri's workspaces and screens rather than with the
// windows Windows lists, so they live apart from its rows.
import Quickshell
import qs
import QtQuick
import "HeldWindows.js" as HeldWindows

Scope {
    id: targets

    // Windows, whose windows these move, and whose daemon holds niri's
    // workspaces and screens.
    required property var provider

    // The lit screens a window isn't on, as Screens names them, or none when
    // there's one screen, or no Screens to ask.
    function screensFor(window) {
        const displays = (targets.provider.launcher?.sources ?? [])
            .find(source => source.settings.name === "displays") ?? null;
        const held = targets.provider.windowsDaemon.held;
        const here = held[`workspace:${window.workspace_id}`]?.output ?? "";
        const others = displays ? displays.lit.filter(output => output.name !== here) : [];
        return others.map(output => targets.provider.thing({
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
        return widths.map(width => targets.provider.thing({
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
        const said = targets.provider.windowsDaemon.held;
        const current = said.desktop?.current ?? null;
        // The chord that moves the focused window there, to learn next time.
        const chordTo = idx => {
            const does = `Move the window to workspace ${idx}`;
            const action = Shortcuts.actions.find(a => a.does === does);
            return action ? action.chord.map(key => key.text).join("+") : "";
        };
        const words = Words.of("windows").said;
        return HeldWindows.spacesFrom(said, window.workspace_id).map(space => {
            const chord = window.id === current ? chordTo(space.idx) : "";
            return targets.provider.thing({
                key: `workspace:${space.idx}`,
                verb: "move",
                name: space.new ? "A new workspace" : space.name ?? `Workspace ${space.idx}`,
                count: space.focused ? words.here : "",
                glyph: space.new ? Theme.glyph.newWorkspace : Theme.glyph.workspace,
                hue: "machine",
                side: space.new ? "" : HeldWindows.insideSaid(space.inside, words),
                note: chord === "" ? "" : words.nextTime.replace("{chord}", chord),
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
