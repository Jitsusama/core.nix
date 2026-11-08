// Joel's timers and the stopwatch (HeldTimers.qml), in Now and the margin:
// true while a timer runs, is paused or has rung, so none is left
// forgotten, or while the stopwatch runs (VKCF3Z, 2026-11-08). One rung
// wants Joel now, leading Now in the alert; the rest are left on. Its side
// says the timer Timers lists first as Timers says it, "9:50", "paused ·
// 9:50" or "rang 03:27", or the stopwatch's count where it alone runs; its
// note names them in that order, the stopwatch last, each its own item, so
// a long one gives way by whole names from its end. Return goes into
// Timers, where each is the row it is there. The daemon says a rung one's
// notice itself, so the mark waits while that card shows.
import qs
import Quickshell
import QtQuick
import "../launcher/Timing.js" as Timing

Piece {
    id: piece

    readonly property var said: Words.of("timer").said ?? {}
    readonly property var watch: HeldTimers.stopwatch
    // The moment it says, read afresh at each tick and as the launcher
    // opens, since a clock stopped while it was closed holds an old date.
    readonly property real moment: {
        void clock.date;
        void HeldTimers.showing;
        return Date.now();
    }
    readonly property var standing: Timing.onNow(HeldTimers.timers, watch, moment)
    // Its clock ticks each second only while the launcher is open and
    // something counts (Timing.js's ticks): the margin wears only the mark,
    // which doesn't say the time.
    readonly property bool counting: Timing.ticks(HeldTimers.timers, watch, HeldTimers.showing)

    function named(record) {
        if (record.key === "stopwatch")
            return Words.name("stopwatch");
        return record.name !== "" ? record.name.charAt(0).toUpperCase() + record.name.slice(1)
            : Words.name("timer");
    }

    active: standing.on
    wants: standing.rung
    now: standing.rung
    title: Words.name("timers")
    value: standing.first !== null ? Timing.side(standing.first, moment, said)
        : Timing.tenths(Timing.counted(watch, moment))
    // The stopwatch alone counts to the tenth, drawn afresh by the
    // launcher (Launcher.qml's counts), as its row in Timers is.
    counts: standing.first === null && watch?.state === "running"
        ? at => Timing.tenths(Timing.counted(watch, at)) : null
    context: standing.named.map(record => piece.named(record)).join(" \u00b7 ")
    hint: `${title} ${value}, ${context}`
    // The daemon signs its notices with the place's name (home.nix's
    // timerWords).
    app: title
    actions: [{
        glyph: Words.glyph("verb:on:timers:show"),
        name: Words.name("verb:on:timers:show"),
        verb: Words.verb("verb:on:timers:show"),
        run: shell => shell.launcher.into("timers")
    }]

    SystemClock {
        id: clock
        enabled: piece.counting
        precision: SystemClock.Seconds
    }
}
