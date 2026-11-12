// The pixels the wallpaper's picture is read at, as Wallpaper.js works
// them out, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `the-wallpaper-is-read-at-its-screens-pixels` check copies
// Wallpaper.js and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen wallpaper.qml`.
//
// A screen niri scales by 1.5 says a ratio of 2, since Wayland gives an
// output's scale as a whole number, so the picture read by the screen's
// ratio was 5120 by 2880 on a 3840 by 2160 screen, for pixels nothing
// shows: on that GPU, read alone, 25 MB more of memory and 48 to 56 MB
// more of the GPU's than at its own pixels (2026-11-12).
import QtQuick
import "Wallpaper.js" as Wallpaper
import "Expect.js" as Expect

QtObject {
    // A 4K screen at 1.5, as Quickshell gives it and niri describes it.
    readonly property var screen: ({ name: "DP-4", width: 2560, height: 1440,
        devicePixelRatio: 2 })
    readonly property var outputs: ({ "DP-4": { name: "DP-4",
        logical: { x: 0, y: 0, width: 2560, height: 1440, scale: 1.5 } } })

    function scaled() {
        Expect.same("a screen at 1.5 reads its picture at its own pixels",
            Wallpaper.pixels(screen, Wallpaper.scaleOf(outputs, screen.name)),
            { width: 3840, height: 2160 });
        Expect.same("a screen at 1 reads it at its logical size",
            Wallpaper.pixels({ name: "eDP-1", width: 1920, height: 1200, devicePixelRatio: 1 },
                Wallpaper.scaleOf({ "eDP-1": { logical: { scale: 1 } } }, "eDP-1")),
            { width: 1920, height: 1200 });
        Expect.same("a scale that doesn't land on a whole pixel rounds to one",
            Wallpaper.pixels({ width: 1707, height: 1067, devicePixelRatio: 2 }, 1.25),
            { width: 2134, height: 1334 });
    }

    function unknown() {
        Expect.same("an output niri doesn't name has no scale",
            Wallpaper.scaleOf(outputs, "HDMI-A-1"), null);
        Expect.same("an answer without outputs has no scale",
            Wallpaper.scaleOf(undefined, "DP-4"), null);
        Expect.same("a scale of nothing isn't one",
            Wallpaper.scaleOf({ "DP-4": { logical: { scale: 0 } } }, "DP-4"), null);
        Expect.same("with no scale, the screen's own ratio stands",
            Wallpaper.pixels(screen, null), { width: 5120, height: 2880 });
        Expect.same("a screen with no ratio reads at its logical size",
            Wallpaper.pixels({ width: 2560, height: 1440, devicePixelRatio: 0 }, null),
            { width: 2560, height: 1440 });
        Expect.same("a screen gone has no pixels", Wallpaper.pixels(null, 1.5),
            { width: 0, height: 0 });
    }

    Component.onCompleted: {
        try {
            scaled();
            unknown();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }
}
