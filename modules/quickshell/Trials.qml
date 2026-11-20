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
import "launcher/Presented.js" as Presented
import "launcher/Stepped.js" as Stepped
import "launcher/Thread.js" as Thread
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

    // When the opening under way began on KeyTimes's clock, so a step in by
    // a key pressed since is told from the steps an opening makes itself.
    property real openedAt: 0

    function opened(at) {
        trial = Trials.begun(Date.now(), home);
        pace = Trials.unpaced();
        openedAt = at ?? 0;
    }

    // A step in by the key pressed at `pressed` on KeyTimes's clock is a
    // look, where it was pressed since the opening began; a place opened
    // straight into steps by no key of his.
    function wentIn(pressed) {
        trial = Trials.wentIn(trial, openedAt, pressed);
    }

    // The drawer come out for a row (`out`), or put away.
    function drawer(out) {
        trial = Trials.drawerShown(trial, Date.now(), out);
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

    // `asking` while the field holds words asked for, which are never kept.
    function retyped(was, now, asking) {
        trial = Trials.retyped(trial, was, now, asking);
    }

    // One key's search and the time to the frame that drew it, and what the
    // search spent its time on, and then `frame`, how long it waited for the
    // frame that drew it and how long drawing it took, and last what the
    // shell's thread did while it searched. A key slow enough to
    // feel is said in the journal too, with what was typed, so what's slow
    // can be read as Joel types. Nothing asked for, such as a password, is
    // ever timed.
    function timed(searched, drawn, parts, typed, frame) {
        trial = Trials.timed(trial, searched, drawn, parts);
        if (Trials.felt(drawn))
            console.info(`launcher: "${typed}" took ${drawn} ms to its frame, ${searched} ms`
                + " searching" + (parts
                    ? ` (lists ${parts.lists}${Stepped.slowest(parts.each, parts)},`
                    + ` rows ${parts.rows}${matched(parts.rows, parts.within)},`
                    + ` answers ${parts.answers}${composed(parts.composing)}`
                    + `${parts.before ? `, before ${parts.before}${setting(parts.beforeParts)}`
                        : ""}`
                    + `${parts.waited
                        ? `, waiting ${parts.waited} on answers${Trials.behindSaid(parts)}`
                        : ""})` : "")
                + (frame ? `, then ${frame.waiting} waiting for its frame and ${frame.drawing}`
                    + " drawing it" + drawingApart(frame) + collecting(frame, drawn)
                    + framesBeforeSaid(frame)
                    + Thread.said(searched, frame.thread, "its searching") : ""));
    }

    // How much of the key, from its being heard to its frame, Qt's
    // collector spent, in slices between the launcher's own work or all
    // at once, said even when none, so a slow key it didn't hold says so.
    // Said of the key's whole time, which the collector's may fall in any
    // part of.
    function collecting(frame, drawn) {
        return frame.collecting === undefined ? ""
            : `, the collector ${frame.collecting} ms of the ${drawn}`;
    }

    // What drawing it was, by the render thread's times of its frame where
    // they were read: its polish and sync, its render and swap, and the GUI
    // thread hearing the swap late, which the three add up to. The last is
    // said as the swap heard, never the key, since a key's presented line
    // says "before the shell heard it" of the key, and Joel's "wi", 64 ms
    // here, was read as a key heard late (2026-11-12).
    function drawingApart(frame) {
        return frame.syncing === undefined ? "" : ` (${frame.syncing} syncing,`
            + ` ${frame.rendering} rendering, ${frame.heard} before the shell heard it swapped)`;
    }

    // The frames begun before a key's own, read only for a key slow enough
    // to be said, and only where its frame's number held (drawingApart's
    // split), since they're counted back from it.
    function framesBeforeSaid(frame) {
        return frame.syncing === undefined || !frame.timesOf ? ""
            : Trials.framesBeforeSaid(Trials.framesBefore(
                Trials.begunBefore(frame.keyFrame, frame.timesOf), frame.keyHeard));
    }

    // A drawer's time from the work begun for it to the frame that drew it,
    // said in the journal as a slow key is, by the row it's for, so how soon
    // each place's drawer comes can be read from Joel's own use (2026-11-10:
    // "as fast as thought"). `parts` says how much of it was the launcher
    // making it, what of that went on which (Trials.makingSplit), then
    // waiting for its frame to begin, then drawing it. Its key is quoted as
    // the drawer's other lines are (Presented.quoted), so a window's title in
    // it can't end the line early or forge another.
    function drawerTimed(key, drawn, parts) {
        if (Trials.felt(drawn))
            console.info(`launcher: drawer for ${Presented.quoted(key)} took ${drawn} ms`
                + ` to its frame, ${parts.making} ms making${Trials.makingSplit(parts.making, parts.split)},`
                + ` ${parts.waiting} waiting for its frame,`
                + ` ${parts.drawing} drawing it, ${parts.came}`);
    }

    // The answers' time, as the answers asked, the rows placed and the list
    // shown, when the key said them, its showing with what it spent listing,
    // putting the rows in and selecting, then how many rows were new to the
    // list and how long laying them out took, with how many rows that drew,
    // built and taken back from those let go, then how long weighing every
    // listed row's keys took, which a drawn row's cost doesn't say, and
    // reading them, and last where a step back out came to.
    function composed(parts) {
        return parts ? `: asking ${parts.asking}, placing ${parts.placing}, showing ${parts.showing}`
            + Trials.showingSaid(parts, Trials.weighingSaid(parts.weighing)
                + Trials.readingSaid(parts.reading))
            : "";
    }

    // The rows' time, where they took any: matching every choice looked at,
    // ordering those found, making their rows, by whose took any, and the
    // spots and meanings beside them, so a slow key says which grows with
    // what home holds.
    function matched(rows, within) {
        return rows > 0 && within ? ` (matching ${within.matching} of ${within.looked},`
            + ` ordering ${within.ordering} of ${within.found},`
            + ` making ${within.making}${Trials.madeOfKinds(within.madeBy)},`
            + ` spots ${within.spots})` : "";
    }

    // The time before the search, where it took any, by what was set: the
    // search counted, what's typed, its words, then readying the rest a
    // search needs, as a step's words and the ask, each with all that wakes
    // on it.
    function setting(parts) {
        return parts ? ` (counting ${parts.counting}, typed ${parts.typed}`
            + `${Trials.madeWhileTyping(parts.made)},`
            + ` words ${parts.words}, readying ${parts.readying})` : "";
    }

    // What a step spent, its own parts beside those of the search that drew
    // its rows (Stepped.js).
    function stepped(inner, at) {
        return Stepped.stepped(inner, at);
    }

    // A walk stopped short's parts, with what it passed (Stepped.js).
    function passed(parts, passing) {
        return Stepped.passed(parts, passing);
    }

    // What an answer is counted as when it's picked (Trials.js).
    function answerCounted(answers, about) {
        return Trials.answerCounted(answers, about);
    }

    // Each key's time from its press to the screen, kept with the opening
    // under way, and how many never reached it.
    function presented(each, unpresented) {
        trial = Trials.presented(trial, each, unpresented);
    }

    function picked(row, typed, depth) {
        trial = Trials.picked(trial, Date.now(), row, { typed: typed, depth: depth });
    }

    // `typed` is what the field held as it closed, "" where it was asked for.
    function closed(typed) {
        if (!trial)
            return;
        // An opening that stuttered is said in the journal, as a slow key
        // is, so how smooth it moves can be read from Joel's own use.
        if (pace.late > 0)
            console.info(`launcher: ${pace.late} of ${pace.drawn} frames came late while`
                + ` it moved, the longest ${pace.longest} ms after the one before`);
        const said = Trials.said(Trials.ended(Trials.paced(trial, pace), Date.now(), typed));
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
