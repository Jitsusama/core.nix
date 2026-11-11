// What each opening of the launcher took (launcher/Trials.js counts it),
// kept in the shell's state folder beside the picks, so the homes Joel
// tries can be compared by the keys, picks, misses and time each cost him.
// A line each, the newest last: as the launcher closes, that opening's line
// goes down a pipe to the file's end, so a close costs the same however
// many are kept. The shell holds none of them; the file is read once, at
// start, to let the oldest go.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Trials.js" as Trials

Scope {
    id: root

    // The home the launcher opens on, which each trial names: "today" is
    // the home as it was before the trials began.
    property string home: "today"
    // How many openings to keep; the oldest go as the shell starts. At
    // about fifty openings a day that's two months.
    property int remember: 3000

    // The opening under way, or null while closed.
    property var trial: null

    // Whether the pipe is running to take a close's line, and the lines said
    // while it wasn't, written once it starts.
    property bool piped: false
    property var waiting: []

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
    // search spent its time on, and then `frame`, how long it waited for the
    // frame that drew it and how long drawing it took. A key slow enough to
    // feel is said in the journal too, with what was typed, so what's slow
    // can be read as Joel types. Nothing asked for, such as a password, is
    // ever timed.
    function timed(searched, drawn, parts, typed, frame) {
        trial = Trials.timed(trial, searched, drawn, parts);
        if (Trials.felt(drawn))
            console.info(`launcher: "${typed}" took ${drawn} ms to its frame, ${searched} ms`
                + " searching" + (parts ? ` (lists ${parts.lists}${slowest(parts.each)},`
                    + ` rows ${parts.rows}, answers ${parts.answers}${composed(parts.composing)}`
                    + `${parts.before ? `, before ${parts.before}` : ""})` : "")
                + (frame ? `, then ${frame.waiting} waiting for its frame and ${frame.drawing}`
                    + " drawing it" + drew(frame) : ""));
    }

    // What drawing it was, by the render thread's times of its frame where
    // they were read: its polish and sync, its render and swap, and the GUI
    // thread hearing the swap late, which the three add up to.
    function drew(frame) {
        return frame.syncing === undefined ? "" : ` (${frame.syncing} syncing,`
            + ` ${frame.rendering} rendering, ${frame.heard} before the shell heard it)`;
    }

    // A drawer's time from the work begun for it to the frame that drew it,
    // said in the journal as a slow key is, by the row it's for, so how soon
    // each place's drawer comes can be read from Joel's own use (2026-11-10:
    // "as fast as thought"). `parts` says how much of it was the launcher
    // making it, then waiting for its frame to begin, then drawing it.
    function drawerTimed(key, drawn, parts) {
        if (Trials.felt(drawn))
            console.info(`launcher: drawer for "${key}" took ${drawn} ms to its frame,`
                + ` ${parts.making} ms making, ${parts.waiting} waiting for its frame,`
                + ` ${parts.drawing} drawing it`);
    }

    // The answers' time, as the answers asked, the rows placed and the list
    // shown, when the key said them, its showing with how many rows were new
    // to the list and how long laying them out took, with how many rows that
    // drew, built and taken back from those let go.
    function composed(parts) {
        return parts ? `: asking ${parts.asking}, placing ${parts.placing}, showing ${parts.showing}`
            + (parts.kept && parts.kept.fresh + parts.kept.laying > 0
                ? ` (${parts.kept.fresh} rows new, ${parts.kept.laying}`
                + ` laying them out${laid(parts.kept)})` : "")
            : "";
    }

    // The rows a laying drew, where it drew any.
    function laid(kept) {
        return (kept.built ?? 0) + (kept.reused ?? 0) > 0
            ? `: ${kept.built} rows built, ${kept.reused} taken back` : "";
    }

    // What a step spent, its own parts beside those of the search that drew
    // its rows (Trials.js).
    function stepped(inner, at) {
        return Trials.stepped(inner, at);
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
        const said = Trials.said(Trials.ended(Trials.paced(trial, pace), Date.now()));
        if (piped)
            keeping.write(said);
        else
            waiting.push(said);
        trial = null;
    }

    // The lines, read once as the shell starts and written only to let the
    // oldest go then; the pipe below adds each close's.
    FileView {
        id: file
        path: Quickshell.statePath("launcher-trials.jsonl")
        blockLoading: true
        printErrors: false
        // The pipe opens the file once it holds what it should, so it adds
        // to the file written rather than one replaced.
        onSaved: keeping.running = true
        onSaveFailed: keeping.running = true
    }

    // The one list the shell kept before it kept lines, read only while
    // there are no lines, to carry it into them.
    FileView {
        id: listed
        blockLoading: true
        printErrors: false
    }

    // What carries each close's line to the file's end, open as long as the
    // shell is, its folder made first on a machine that has kept nothing.
    Process {
        id: keeping
        command: ["sh", "-c", 'mkdir -p "${1%/*}" && exec cat >> "$1"', "trials", file.path]
        stdinEnabled: true
        onStarted: {
            root.piped = true;
            root.waiting.splice(0).forEach(said => keeping.write(said));
        }
        // A pipe that ends would lose every close after it without a word,
        // so it's said and started again, a second on so one that ends at
        // once doesn't spin.
        onExited: (code, status) => {
            root.piped = false;
            console.warn(`launcher: the trials' pipe ended with ${code}, so it starts again`);
            again.restart();
        }
    }

    Timer {
        id: again
        interval: 1000
        onTriggered: keeping.running = true
    }

    Component.onCompleted: {
        const lines = file.text();
        if (lines.trim() === "")
            listed.path = Quickshell.statePath("launcher-trials.json");
        let before = null;
        try {
            before = listed.path === "" ? null : JSON.parse(listed.text() || "[]");
        } catch (e) {
            console.warn(`The launcher's old trials couldn't be read, so none are carried: ${e}`);
        }
        const kept = Trials.carried(lines, before, remember);
        if (kept === null)
            keeping.running = true;
        else
            file.setText(kept);
    }
}
