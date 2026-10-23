// Every application with a desktop entry, found by its name and by what it
// is: "browser" finds Chrome. Return opens it; Shift+Return opens it below
// the focused window. A desktop action that opens just the program, as
// Super+Return opens kitty, is its row too: the row draws that chord and
// answers to the action's words ("open a terminal"), so the two aren't
// listed side by side.
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

    things: DesktopEntries.applications.values
        .filter(entry => !entry.noDisplay)
        .map(entry => {
            const open = () => Niri.spawn(provider.command(entry));
            const below = () => provider.launcher.grid.under(Niri.current, provider.command(entry));
            const binds = provider.opening(provider.command(entry));
            return thing({
                key: `program:${entry.id}`,
                verb: "open",
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
                run: open,
                below: below,
                verbs: () => [
                    thing({
                        key: "verb:open",
                        name: "Open it",
                        glyph: String.fromCodePoint(0xf03cc),
                        hue: "action",
                        run: open
                    }),
                    thing({
                        key: "verb:below",
                        name: "Open it below this window",
                        glyph: String.fromCodePoint(0xf03cc),
                        hue: "action",
                        chord: provider.launcher.belowChord,
                        run: below
                    })
                ]
            });
        })
}
