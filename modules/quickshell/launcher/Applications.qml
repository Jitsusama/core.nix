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
        .map(entry => thing({
            key: `program:${entry.id}`,
            verb: "open",
            name: entry.name,
            keywords: [entry.genericName, entry.comment]
                .concat(Array.from(entry.keywords)).join(" "),
            glyph: programGlyph(entry.id),
            hue: "program",
            side: entry.genericName.toLowerCase(),
            run: () => Niri.spawn(provider.command(entry)),
            below: () => {
                provider.launcher.grid.below("");
                Niri.spawn(provider.command(entry));
            }
        }))

    // What an entry runs, spelled out for niri to spawn: in kitty when the
    // entry asks for a terminal, and from its own directory when it names one.
    function command(entry) {
        const program = entry.runInTerminal ? ["kitty"].concat(entry.command) : entry.command;
        if (entry.workingDirectory === "")
            return program;
        return ["sh", "-c", "cd -- \"$0\" && exec \"$@\"", entry.workingDirectory].concat(program);
    }
}
