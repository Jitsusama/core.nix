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
        pace = Trials.unpaced();
    }

    // The frames of the opening under way, counted as each is drawn. Nothing
    // binds to it, so it's changed in place, not handed on.
    property var pace: Trials.unpaced()

    // A frame the card drew at `at`, on a screen refreshing `hz` times a
    // second.
    function drew(at, hz) {
        Trials.drew(pace, at, hz);
    }

    function pressed(kind) {
        trial = Trials.counted(trial, kind);
    }

    function retyped(was, now) {
        trial = Trials.retyped(trial, was, now);
    }

    // One key's search and the time to the frame that drew it, and what the
    // search spent its time on. A key slow enough to feel is said in the
    // journal too, with what was typed, so what's slow can be read as Joel
    // types. Nothing asked for, such as a password, is ever timed.
    function timed(searched, drawn, parts, typed) {
        trial = Trials.timed(trial, searched, drawn, parts);
        if (Trials.felt(drawn))
            console.info(`launcher: "${typed}" took ${drawn} ms to its frame, ${searched} ms`
                + " searching" + (parts ? ` (lists ${parts.lists}${slowest(parts.each)},`
                    + ` rows ${parts.rows}, answers ${parts.answers}${composed(parts.composing)}`
                    + `${parts.before ? `, before ${parts.before}` : ""})` : ""));
    }

    // The answers' time, as the answers asked, the rows placed and the list
    // shown, when the key said them.
    function composed(parts) {
        return parts ? `: asking ${parts.asking}, placing ${parts.placing}, showing ${parts.showing}`
            : "";
    }

    // The three collections that took longest, if any took a millisecond.
    function slowest(each) {
        const named = Object.keys(each ?? {}).filter(name => each[name] > 0)
            .sort((a, b) => each[b] - each[a]).slice(0, 3);
        return named.length === 0 ? "" : `: ${named.map(name => `${name} ${each[name]}`).join(", ")}`;
    }

    function picked(row, typed, depth) {
        trial = Trials.picked(trial, Date.now(), row, { typed: typed, depth: depth });
    }

    function closed() {
        if (!trial)
            return;
        // An opening that stuttered is said in the journal, as a slow key
        // is, so how smooth it moves can be read from Joel's own use.
        if (pace.late > 0)
            console.info(`launcher: ${pace.late} of ${pace.drawn} frames came late while`
                + ` it moved, the longest ${pace.longest} ms after the one before`);
        trials = Trials.kept(trials, Trials.ended(Trials.paced(trial, pace), Date.now()),
            remember);
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
