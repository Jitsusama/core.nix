pragma Singleton
// What in Slack wants Joel, read from the runtime folder's
// slack-unreads.json, which slack-unreads writes whole when it changes or
// at least every five minutes and takes with it however it stops. One
// file, read by one watcher. What it says and whether it's current are
// SlackUnreads.js's; the only clock here is one wake at the moment a
// fresh count would go stale, so a quiet machine wakes for nothing else.
import Quickshell
import Quickshell.Io
import QtQuick
import "SlackUnreads.js" as Unreads

Singleton {
    id: root

    readonly property string path: `${Quickshell.env("XDG_RUNTIME_DIR")}/slack-unreads.json`
    // What the file says, { checked, state, conversations }, or null.
    property var told: null
    // The time it was last judged by: set as the file is read, and once
    // more as a fresh count goes stale.
    property real judged: Date.now()
    // "none", "ok", "signedOut" or "unknown" (SlackUnreads.js's standing).
    readonly property string stands: Unreads.standing(told, judged)
    readonly property bool shown: Unreads.shown(told, stands)
    readonly property string value: Unreads.value(told, stands)
    readonly property string context: Unreads.context(told, stands)
    readonly property string hint: Unreads.hint(told, stands)
    // Why there's no count, for Slack's program row (SlackProgram.qml).
    readonly property string programNote: Unreads.programNote(told, stands)
    // Each { id, name, kind, count, link }, only while the count is
    // current, so no old one is offered as Slack's.
    readonly property var conversations: stands === "ok" ? told.conversations : []

    // A conversation's row's words, for the piece, which lives where
    // SlackUnreads.js can't be imported.
    function beside(one) {
        return Unreads.beside(one);
    }

    function note(one) {
        return Unreads.note(one);
    }

    function hear(text) {
        told = Unreads.read(text);
        judged = Date.now();
        const at = Unreads.staleAt(told);
        staling.stop();
        if (at !== null && at > judged) {
            staling.interval = at - judged + 1000;
            staling.start();
        }
    }

    FileView {
        path: root.path
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.hear(text())
        onLoadFailed: root.hear("")
    }

    Timer {
        id: staling
        onTriggered: root.judged = Date.now()
    }
}
