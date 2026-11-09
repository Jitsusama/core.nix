// Joel's timers and the stopwatch (HeldTimers.qml), in Now and the margin:
// true while a timer runs, is paused or has rung, so none is left
// forgotten, or while the stopwatch runs (VKCF3Z, 2026-11-08). One rung
// wants Joel now, leading Now in the alert; the rest are left on. It's a
// place, stepped into for rows of its own, so it says nothing after its
// name: at its right, the one on as its own row in Timers says it, or of
// several how many run and the first's (Timing.nowItems), and in its
// drawer each by name in Timers' order, the stopwatch last (VKCF3Z,
// 5fa2cbda). Return goes into Timers, where each is the row it is there.
// The daemon says a rung one's notice itself, so the mark waits while
// that card shows.
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
    readonly property var items: Timing.nowItems(standing, watch, moment, said,
        Words.of("timers").said?.several ?? "")
    value: items.map(item => item.said).join(" \u00b7 ")
    valueItems: items
    // The stopwatch alone counts to the tenth, drawn afresh by the
    // launcher (Launcher.qml's counts), as its row in Timers is.
    counts: watchAlone ? at => Timing.tenths(Timing.counted(watch, at)) : null
    widest: watchAlone ? Timing.widest(Timing.counted(watch, moment)) : ""
    // When the stopwatch's count last grew a figure, so its room is said
    // again then (Timing.grows).
    property real grown: 0
    heldNames: standing.named.map(record => piece.named(record))
    hint: `${title} ${value}, ${heldNames.join(", ")}`
    // The daemon signs its notices with the place's name (home.nix's
    // timerWords).
    app: title
    actions: [{
        glyph: Words.glyph("verb:on:timers:show"),
        name: Words.name("verb:on:timers:show"),
        verb: Words.verb("verb:on:timers:show"),
        run: shell => shell.launcher.into("timers")
    }]

    // Only while the launcher is open, as the clock is, so the margin never
    // wakes for it closed; its wait is worked out again whenever it starts,
    // the stopwatch changes or the count grows.
    Timer {
        repeat: true
        running: piece.watchAlone && HeldTimers.showing
        interval: {
            void piece.grown;
            return running ? piece.untilWider() : 1000;
        }
        onTriggered: piece.grown = Date.now()
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
