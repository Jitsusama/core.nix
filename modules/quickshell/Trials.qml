// What each opening of the launcher took (launcher/Trials.js counts it),
// kept in the shell's state folder beside the picks, so the homes Joel
// tries can be compared by the keys, picks, misses and time each cost him.
// It's written once, as the launcher closes; nothing is spawned, and
// nothing is read but the file itself, once, at start.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Trials.js" as Trials

Scope {
    id: root

    // The home the launcher opens on, which each trial names: "today" is
    // the home as it was before the trials began.
    property string home: "today"
    // How many openings to keep; the oldest go first. At about fifty
    // openings a day that's two months.
    property int remember: 3000

    // The opening under way, or null while closed.
    property var trial: null
    property var trials: []

    function opened() {
        trial = Trials.begun(Date.now(), home);
    }

    function pressed(kind) {
        trial = Trials.counted(trial, kind);
    }

    function retyped(was, now) {
        trial = Trials.retyped(trial, was, now);
    }

    function picked(row, typed, depth) {
        trial = Trials.picked(trial, Date.now(), row, { typed: typed, depth: depth });
    }

    function closed() {
        if (!trial)
            return;
        trials = Trials.kept(trials, Trials.ended(trial, Date.now()), remember);
        trial = null;
        file.setText(JSON.stringify(trials));
    }

    FileView {
        id: file
        path: Quickshell.statePath("launcher-trials.json")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: {
        try {
            const kept = JSON.parse(file.text() || "[]");
            trials = Array.isArray(kept) ? kept : [];
        } catch (e) {
            console.warn(`The launcher's trials couldn't be read, so they start afresh: ${e}`);
            trials = [];
        }
    }
}
