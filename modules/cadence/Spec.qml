// Spec in the launcher: how the agents deliver against the bars Joel set,
// read on ten axes. It sits inside Agents, after Budgets, since it's a
// reading of the agents, as Budgets is (VKCF3Z, 2026-11-14 [efa2d961]):
// Agents' row says the head, "14 readings out of spec" or "all in spec", and a step
// in lists the axes, each stepping in to its readings, worst first as the
// daemon orders them. It earns a row on Home by Joel's use, never ahead of
// it.
//
// The axes and the head are cadence-spec's (./spec), served ready over its
// socket (ReadyRows.qml), so nothing is read, worded or ordered here
// (Joel, 2026-11-10: "the daemons need to be the data layer"). The daemon
// reads the axes only while they're watched, and they're watched only
// while the launcher is open (1JQPPC, #4516).
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/cadence-spec.sock`
        place: "spec"
        beside: ["head"]
        live: provider.launcher?.open ?? false
        paced: provider.launcher
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => ready.thing(row, key, { hue: row.hue || "agent" })
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready

    // The head as the daemon says it, `{ said, glyph }`, or null before it
    // has answered or where cadence-spec isn't running.
    function head() {
        void ready.placings;
        return ready.held["head:spec"]?.head ?? null;
    }

    // The axes, each a step in to its readings (ReadyRows.qml's rows).
    function axes() {
        return ready.rows();
    }
}
