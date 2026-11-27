// Each program's windows for Programs (Applications.qml), heard from the
// windows daemon only while Programs is wanted or the launcher is open,
// and what its rows say of them: how many, the one used last, each one's
// title and its row. Applications.qml keeps every name another file reads
// of this, the tests' windowsDaemon among them, and makes this once.
import Quickshell
import qs
import QtQuick
import "HeldWindows.js" as HeldWindows

Scope {
    id: windowsPart

    // Applications, whose programs' windows these are, which makes their
    // rows.
    required property var provider

    // Each program's open windows by its desktop entry's id, the one used
    // last first, as the windows daemon says them (modules/windows), which
    // names each window's owner: its entry, or for a browser's window of
    // its own opened deeper than its program's own page, that program.
    // Read when a row is drawn or run, so a window opening never prepares
    // every program again. Heard only while it's wanted: every event niri
    // sent once walked all of Joel's windows here, launcher shut or open
    // (2026-11-10).
    property var running: ({})
    readonly property var windowsHeld: HeldWindows.kept()
    readonly property var windowsDaemon: daemon

    // With the daemon down its row says so, since the counts it keeps
    // through the gap may no longer be true. It never says it's still
    // reading (Provider.stillReading), though its counts are the last
    // opening's until this watch's first answer: Programs is a home row,
    // so every terminal's show from home would ask twice, for counts the
    // daemon answers within a frame.
    function noted() {
        return windowsUnanswered ? Words.of("windows").said.silentFrom : "";
    }
    // Whether the windows daemon has been silent a second, as Windows.qml
    // waits too, so a restart doesn't blink it (VKCF3Z, 2026-11-10).
    property bool windowsUnanswered: false
    Connections {
        target: daemon
        function onStateChanged() {
            if (daemon.state === "not answering") {
                windowsQuiet.restart();
            } else {
                windowsQuiet.stop();
                windowsPart.windowsUnanswered = false;
            }
        }
    }
    Timer {
        id: windowsQuiet

        interval: 1000
        onTriggered: windowsPart.windowsQuietFor()
    }
    // The second's silence over: said if the daemon is silent still.
    function windowsQuietFor() {
        windowsUnanswered = daemon.state === "not answering";
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        kind: "window"
        // Watched while the launcher is open too, as Windows' are, so a
        // step home asks for no window again (Windows.qml).
        live: windowsPart.provider.wanted || windowsPart.provider.launcher?.open === true
        paced: windowsPart.provider.launcher
        answersWhole: true
        onHeard: (changed, gone) => windowsPart.hearWindows(changed, gone)
    }

    // Takes what the daemon said, and lists each program's windows again
    // only when a window's record differs, so a whole answer as the
    // launcher opens that reads the same changes nothing. The rows read
    // their counts and Go To as they're drawn, so they're drawn again
    // when a program's windows came or went or another was used last;
    // a title alone changes only what a row stepped into holds.
    function hearWindows(changed, gone) {
        provider.tally.looked += changed.length + gone.length;
        const held = windowsPart.windowsHeld;
        const records = changed.map(key => daemon.held[key]).filter(record => record);
        if (HeldWindows.take(held, records, gone).length === 0 && gone.length === 0)
            return;
        const by = {};
        for (const key of held.order) {
            const owner = held.records[key].owner;
            if (owner !== null)
                (by[owner] ?? (by[owner] = [])).push(held.windows[key]);
        }
        const was = windowsPart.running;
        windowsPart.running = by;
        if (HeldWindows.ownersMoved(was, by))
            provider.changed();
        else
            provider.redrawHeld();
    }

    // When a window was last looked at, in nanoseconds, or 0 for never.
    function focusedAt(window) {
        const stamp = window.focus_timestamp;
        return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
    }

    // How many windows a program has open, as the row says it beside the
    // name, before what it is.
    function windowCount(entry) {
        const open = (running[entry.id] ?? []).length;
        return open === 0 ? "" : open === 1 ? "1 window" : `${open} windows`;
    }

    // A window's title as its rows say it, without the program's name.
    function titled(window, entry) {
        return provider.tidy(window.title ?? "", entry.name) || entry.name;
    }

    // Its windows but the one used last, which Go To already goes to, as
    // rows Return goes to, each by its own title, saying where it is as the
    // Windows collection does: nothing on the workspace in front, and a
    // column when two are titled alike. Each holds what Windows' row for it
    // does (Windows.qml's actsOf), so a window has one set of insides
    // wherever it shows.
    // Whether a program has a window open now.
    function isRunning(entry) {
        return (running[entry.id] ?? []).length > 0;
    }

    // Its windows but the one used last and those a row it has open
    // already stands for (`held`, window ids), so a window and its front
    // tab or its file are never two rows. An agent's window, kitty's though
    // it is, is left to kitty's Agents row, as its tabs are (Kitty.qml):
    // listed, Joel's 112 drew 114 rows and took 474 ms a step into kitty
    // (2026-11-13, VKCF3Z [b31d7a90]).
    function windowsOf(entry, held) {
        return (running[entry.id] ?? []).slice(1)
            .filter(window => window.session !== true && !(held ?? []).includes(window.id))
            .map(window => windowRow(window, entry));
    }

    // One of a program's windows, as its row, by its title.
    function windowRow(window, entry) {
        const listing = (provider.launcher?.sources ?? [])
            .find(source => typeof source.actsOf === "function") ?? null;
        const alike = (running[entry.id] ?? [])
            .filter(other => titled(other, entry) === titled(window, entry)).length;
        return provider.thing({
            thing: true,
            key: `window:${window.app_id}:${provider.tidy(window.title ?? "", entry.name)}`,
            is: `window:${window.id}`,
            verb: "go to",
            name: titled(window, entry),
            glyph: provider.programGlyph(entry.id),
            hue: "program",
            side: provider.placeOf(window, Niri.workspaces[window.workspace_id], Niri.workspace(),
                alike),
            run: () => Niri.act({ FocusWindow: { id: window.id } }),
            verbs: listing ? () => listing.actsOf(window, entry) : null
        });
    }
}
