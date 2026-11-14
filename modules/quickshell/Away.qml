pragma Singleton
// Whether Joel is away from the machine, for what may change his screen
// only while he's gone: a hosted session's viewer opening by itself
// (Sessions.qml's viewKept and Reopen Them All). Away is nothing typed or
// moved for ten minutes, as the session moves' own rule has it, and never
// while something keeps the screen awake, as a call or a video does, since
// he's most likely there, listening or watching. A shell just started has
// seen no idle yet, so it starts with him there: a switch restarting it
// opened two viewers on a new workspace while he was (OSPHT4 [0762284c],
// 13:41:38 on 11 November).
//
// It says so in $XDG_RUNTIME_DIR/quickshell-away too, "away" or "here",
// written as the shell starts and whenever it changes, for what outside
// the shell waits on him: an agent's question parks on his look list
// rather than holding its session while he's gone (the harness's ask
// tool). A locked screen isn't folded in, since that would also gate
// Reopen Them All; readers read quickshell-locked beside it. The unit
// removes the file when the shell stops (home.nix), so a dead shell
// never says he's away.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

Singleton {
    id: root

    readonly property bool now: idle.isIdle

    onNowChanged: say()
    Component.onCompleted: say()

    function say(): void {
        mark.setText(root.now ? "away\n" : "here\n");
    }

    IdleMonitor {
        id: idle
        timeout: 600
        respectInhibitors: true
    }

    FileView {
        id: mark
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-away`
        printErrors: false
        // A word in memory, so written before anything reads it.
        blockWrites: true
    }
}
