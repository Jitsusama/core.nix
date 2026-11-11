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
import Quickshell
import Quickshell.Wayland
import QtQuick

Singleton {
    readonly property bool now: idle.isIdle

    IdleMonitor {
        id: idle
        timeout: 600
        respectInhibitors: true
    }
}
