// Joel's timers and the stopwatch (HeldTimers.qml), in Now and the margin:
// true while a timer runs, is paused or has rung, so none is left
// forgotten, or while the stopwatch runs (VKCF3Z, 2026-11-08). One rung
// wants Joel now, leading Now in the alert; the rest are left on. Its side
// says only how the timer Timers lists first stands, as Timers says it,
// "9:50", "paused · 9:50" or "rang 03:27", or the stopwatch's count where
// it alone runs; its parts name them in that order, the stopwatch last,
// after its title as a place's row says what it holds, giving way by
// whole names from the end (VKCF3Z, 2026-11-08). Return goes into
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
    // The moment it says, read afresh at each tick, as the launcher opens
    // and as the stopwatch's count grows a figure, since a clock stopped
    // while it was closed holds an old date.
    readonly property real moment: {
        void clock.date;
        void HeldTimers.showing;
        void grown;
        return Date.now();
    }
    readonly property var standing: Timing.onNow(HeldTimers.timers, watch, moment)
    // Its clock ticks each second only while the launcher is open and a
    // timer counts down (Timing.js's ticks): the margin wears only the
    // mark, which doesn't say the time, and the stopwatch alone is drawn
    // afresh by the launcher instead.
    readonly property bool counting: Timing.ticks(HeldTimers.timers, HeldTimers.showing)
    readonly property bool watchAlone: standing.first === null && watch?.state === "running"

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
    counts: watchAlone ? at => Timing.tenths(Timing.counted(watch, at)) : null
    widest: watchAlone ? Timing.widest(Timing.counted(watch, moment)) : ""
    // When the stopwatch's count last grew a figure, so its room is said
    // again then (Timing.grows).
    property real grown: 0
    parts: standing.named.map(record => piece.named(record))
    hint: `${title} ${value}, ${parts.join(", ")}`
    // The daemon signs its notices with the place's name (home.nix's
    // timerWords).
    app: title
    actions: [{
        glyph: Words.glyph("verb:on:timers:show"),
        name: Words.name("verb:on:timers:show"),
        verb: Words.verb("verb:on:timers:show"),
        run: shell => shell.launcher.into("timers")
    }]

    Timer {
        repeat: true
        running: piece.watchAlone
        onRunningChanged: {
            if (running)
                interval = piece.untilWider();
        }
        onTriggered: {
            piece.grown = Date.now();
            interval = piece.untilWider();
        }
    }

    function untilWider() {
        const count = Timing.counted(watch, Date.now());
        return Math.max(1, Timing.grows(count) - count);
    }

    SystemClock {
        id: clock
        enabled: piece.counting
        precision: SystemClock.Seconds
    }
}
