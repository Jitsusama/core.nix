// A status file a watcher writes whole, with its heartbeat in it at least
// every so often while it runs: what it says, read as `parse` reads it, or
// null when there's none that reads, and whether the watcher has gone
// silent, no heartbeat for `silence` seconds.
//
// Nothing ticks. The file's watch hears each new one, since it watches the
// folder too, and each heartbeat sets a deadline that long after it. The
// deadline counts the machine's running time, which a suspend stops as it
// stops the watcher, so waking gives the watcher as long again.
//
// A write that says only when (its heartbeat, its `at`) moves the deadline
// and changes nothing said, so what reads `said` re-runs only when the
// watcher says something new. The machine queue is rewritten about 13
// times a second, its `at` alone changing in nine writes of ten
// (2026-10-29); each was a new `said` for every reader.
//
// While `held`, as a file nothing shown reads is, it isn't watched at all,
// and is read once it's let go, so a write costs the shell nothing. Under
// load the machine queue's pressure changes in nearly every write, 12 a
// second, each about 20 wakeups and 14 ms of the shell's with the
// launcher closed (15J7N3, 2026-10-30); only noted, each still woke it,
// 54 of the 69 folder events Joel's closed shell heard in 30 s
// (2026-10-31). Its silence isn't kept meanwhile: the read as it's let go
// says it.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: status

    required property string path
    property real silence: 120
    property var parse: text => {
        try {
            const said = JSON.parse(text);
            return typeof said === "object" && said !== null ? said : null;
        } catch (error) {
            return null;
        }
    }
    property var said: null
    property bool silent: false
    // When it last said it was running, in seconds since the epoch.
    property real beat: 0
    // The keys a watcher rewrites each time that say only when it wrote,
    // which no reader draws: left out when telling whether it said
    // something new.
    property var beats: ["heartbeat", "at"]
    // What it said last, without its beats, as JSON.
    property string saying: ""
    property bool held: false
    onHeldChanged: {
        if (held)
            quiet.stop();
        else
            file.reload();
    }

    function hear(now) {
        beat = now?.heartbeat ?? 0;
        let told = "";
        if (now !== null) {
            const kept = {};
            for (const key of Object.keys(now)) {
                if (!beats.includes(key))
                    kept[key] = now[key];
            }
            told = JSON.stringify(kept);
        }
        if (now === null || told !== saying) {
            saying = told;
            said = now;
        }
        heard();
    }

    function heard() {
        const age = Date.now() / 1000 - beat;
        quiet.stop();
        silent = said !== null && age > silence;
        if (said !== null && !silent && !held) {
            quiet.interval = Math.max(1, (silence - age) * 1000);
            quiet.start();
        }
    }

    Timer {
        id: quiet
        onTriggered: status.silent = true
    }

    FileView {
        id: file
        path: status.path
        watchChanges: !status.held
        printErrors: false
        onFileChanged: reload()
        onLoaded: status.hear(status.parse(file.text()))
        onLoadFailed: status.hear(null)
    }
}
