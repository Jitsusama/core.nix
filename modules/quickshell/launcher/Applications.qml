// Every application with a desktop entry, found by its name and by what it
// is: "browser" finds Chrome. Return opens it; Shift+Return opens it below
// the focused window.
import Quickshell
import qs
import QtQuick

Provider {
    id: provider

    things: DesktopEntries.applications.values
        .filter(entry => !entry.noDisplay)
        .map(entry => {
            const open = () => Niri.spawn(provider.command(entry));
            const below = () => provider.launcher.grid.under(Niri.focused, provider.command(entry));
            return thing({
                key: `program:${entry.id}`,
                verb: "open",
                name: entry.name,
                keywords: [entry.genericName, entry.comment]
                    .concat(Array.from(entry.keywords)).join(" "),
                glyph: programGlyph(entry.id),
                hue: "program",
                side: entry.genericName.toLowerCase(),
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
