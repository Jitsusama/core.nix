// The one file-preview the launcher's chooser of files (FileChooser.qml)
// keeps for an ask, and what it said: each file or folder selected's
// preview for its plate, and how many items each folder read holds shows,
// for its row. The chooser makes it once, binding the ask, the entries it
// knows and the program to run, and hears `counted` as counts are to be
// said. Unlike Previewer.qml, which asks the programs daemon's
// file-preview for Capture's and Neovim's drawers, this one runs its own,
// one for each ask, asked one line at a time (VKCF3Z, 2026-11-03), so
// nothing is started for each file selected (Y2YBTS, 2026-11-03).
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: piece

    // Whether the chooser holds an ask; nothing is asked without one.
    property bool asking: false
    // Each file and folder the chooser's folders read hold, by its path,
    // for the change time a preview is asked at.
    property var known: ({})
    // The file-preview to run, as the chooser's settings give it, or
    // nothing where nothing previews.
    property var program: null

    // How many items each folder read holds shows, by its path, and what
    // file-preview said of each file or folder selected (false where it
    // couldn't), with the change time it was asked at.
    property var counts: ({})
    property var previews: ({})
    property var askedAt: ({})
    // The file whose preview is wanted next, while another is asked, the
    // folders still to count, and what file-preview is answering now.
    property string previewNext: ""
    property var toCount: []
    property var outstanding: null
    // How long counts heard while more are still to come are held before
    // they're said, in ms. Each folder read is counted in one answer, and
    // each change makes every row afresh, so opening a folder four deep
    // remade its rows once for each folder's counts on the way; now the
    // counts of the folders read together are said once they're all
    // heard, or this long after the first where it takes longer.
    property int countsHeldFor: 200
    // Whether counts heard are held, waiting on more; read by its checks.
    readonly property bool holdingCounts: countsHeld.running

    // Said once the counts heard are to be said, so every folder's row is
    // made again with them.
    signal counted

    // What file-preview said of a file or a folder for its plate, or
    // undefined before it has. One not yet asked, or that changed since it
    // was asked, is asked a turn later, never while a drawer's binding
    // reads, since that would change what it reads.
    function shown(path) {
        const entry = known[path] ?? null;
        const said = previews[path];
        if (program && (said === undefined || entry && askedAt[path] !== entry.modified))
            Qt.callLater(piece.preview, path);
        return said;
    }

    // Wants file-preview's word on a file, asked once it's answered what
    // it's asked now, in place of any wanted before it.
    function preview(path) {
        const entry = known[path] ?? null;
        if (!asking || path in previews && (!entry || askedAt[path] === entry.modified))
            return;
        previewNext = path;
        askNext();
    }

    // Wants what each folder inside one just read shows counted, so a
    // folder's row can say it.
    function count(path) {
        if (!program || !asking || toCount.includes(path))
            return;
        toCount = toCount.concat([path]);
        askNext();
    }

    // Asks file-preview one thing at a time, the file wanted first, then
    // the folders to count, starting it again if it's stopped. A path
    // with a line end in it can't be asked as a line, so it's said to
    // have nothing to show.
    function askNext() {
        if (!program || !asking || outstanding !== null)
            return;
        if (!previewer.ready) {
            previewer.running = true;
            return;
        }
        if (previewNext !== "") {
            const path = previewNext;
            previewNext = "";
            askedAt = Object.assign({}, askedAt, { [path]: known[path]?.modified });
            put("show", path);
        } else if (toCount.length > 0) {
            const path = toCount[0];
            toCount = toCount.slice(1);
            put("items", path);
        }
    }

    function put(verb, path) {
        outstanding = { verb: verb, path: path };
        if (path.includes("\n"))
            Qt.callLater(piece.previewerSaid, "");
        else
            previewer.write(`${verb} ${path}\n`);
    }

    // file-preview's answer to what it was asked, nothing where it
    // couldn't say, then the next thing to ask.
    function previewerSaid(line) {
        const was = outstanding;
        outstanding = null;
        if (was === null || !asking)
            return;
        let said = false;
        try {
            said = JSON.parse(line);
        } catch (error) {
            said = false;
        }
        if (said && "error" in said)
            said = false;
        if (was.verb === "show") {
            previews = Object.assign({}, previews, { [was.path]: said });
        } else if (said || countsHeld.running) {
            // A last answer that says nothing still says what was held.
            if (said)
                counts = Object.assign({}, counts, said);
            if (toCount.length === 0) {
                countsHeld.stop();
                piece.counted();
            } else if (!countsHeld.running) {
                countsHeld.start();
            }
        }
        Qt.callLater(piece.askNext);
    }

    // Lets go of what was said of each file and folder, and stops
    // file-preview, as the ask ends and its folders read go.
    function forget() {
        counts = {};
        previews = {};
        askedAt = {};
        toCount = [];
        outstanding = null;
        countsHeld.stop();
        previewer.running = false;
    }

    // Says the counts heard so far, once a batch has been held long
    // enough.
    Timer {
        id: countsHeld

        interval: piece.countsHeldFor
        onTriggered: piece.counted()
    }

    // One file-preview for an ask, started as the first thing is wanted of
    // it, its folders read as it waits, and stopped as the ask ends
    // (forget), asked a line at a time, so nothing is started for each
    // file selected or folder read.
    // One that stops is started again for what's wanted next; one that
    // stops before answering anything is given up on for what's wanted,
    // so a file-preview that can't start isn't started again and again.
    Process {
        id: previewer

        property bool ready: false
        property bool answered: false

        command: (piece.program ?? []).concat(["serve"])
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                previewer.answered = true;
                piece.previewerSaid(line);
            }
        }
        onStarted: {
            ready = true;
            answered = false;
            Qt.callLater(piece.askNext);
        }
        onExited: {
            const gaveUp = !answered;
            ready = false;
            if (piece.outstanding !== null)
                piece.previewerSaid("");
            if (gaveUp) {
                piece.previewNext = "";
                piece.toCount = [];
            }
            Qt.callLater(piece.askNext);
        }
    }
}
