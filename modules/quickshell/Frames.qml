pragma Singleton

// How many frames each of the shell's windows drew, by its name, counted
// only while a check asks (`qs ipc call frames start`, then `said`), for the
// budget that a shell showing nothing new draws nothing (docs/design.md,
// RSCH-20261029-0E8050's Motion and Drawing). Each window carries a Counted,
// which hears its frames only while this counts, so between checks nothing
// is heard and nothing costs.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: frames

    property bool counting: false
    // Frames by window name since counting started; a plain object, never
    // bound, since it changes on every frame while counting.
    property var counts: ({})

    function heard(name) {
        counts[name] = (counts[name] ?? 0) + 1;
    }

    IpcHandler {
        target: "frames"

        // Counts from nothing, every window at 0.
        function start(): void {
            frames.counts = {};
            frames.counting = true;
        }

        // The frames each window drew since start, as JSON by its name, and
        // stops counting. A window that drew none isn't named.
        function said(): string {
            frames.counting = false;
            return JSON.stringify(frames.counts);
        }
    }
}
