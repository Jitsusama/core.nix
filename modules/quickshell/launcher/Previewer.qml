// One file-preview kept for a provider's drawers, asked a line at a time
// (`serve`), so nothing is started for each drawer shown; it waits on its
// input, costing nothing, while the launcher's shut. Capture's things
// taken and Neovim's files ask it as their drawers draw, where each once
// started a `file-preview show` of its own (Y2YBTS, 2026-11-10).
//
// A stopgap under the shell's data rule, that the launcher holds no data:
// the screen-text daemon (modules/screen-text), which already reads every
// thing taken, is to say each one's facts in its own record, and
// file-preview to answer Neovim's files over a socket held open, so this
// goes (3TXW9E, by Fri 13 Nov).
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick
import "Captured.js" as Captured

Scope {
    id: previewer

    // file-preview as the provider's settings name it; none, and nothing
    // is previewed.
    required property var command
    readonly property bool can: (command ?? []).length > 0

    // What file-preview said of each file, by its path (null where it
    // couldn't), when the file had changed as it was asked, and which were
    // asked; the file it's asked of now, and the next wanted while it
    // answers or starts. At most `most` files are kept, the oldest let go
    // first, so a shell that runs for days doesn't keep every file it was
    // ever shown.
    property var previews: ({})
    property var changes: ({})
    property var asked: ({})
    property string asking: ""
    property string next: ""
    property int most: 64

    // Whether file-preview stopped before answering anything, so drawers
    // say it isn't answering rather than lacking their facts unsaid; it's
    // said once in the journal, and cleared by its next answer.
    property bool silent: false

    // What file-preview said of a file, asking a frame later where it's
    // not been asked, never while a drawer's binding reads, since that
    // would change what it reads; undefined until it answers. A file
    // changed since it was asked (`changed`, its change time, where the
    // provider knows it) is asked again, as one rewritten in place is,
    // what was said of it dropped where it's held, telling no binding, as
    // the answer that comes tells them all.
    function shown(path, changed) {
        if (changed !== undefined && previewer.changes[path] !== changed) {
            previewer.changes[path] = changed;
            delete previewer.asked[path];
            delete previewer.previews[path];
        }
        const said = previewer.previews[path];
        if (said === undefined && previewer.can && !previewer.asked[path]) {
            previewer.asked[path] = true;
            Qt.callLater(() => previewer.preview(path));
        }
        return said;
    }

    // Everything said and asked forgotten, for files that may have changed.
    function forget() {
        previewer.previews = {};
        previewer.changes = {};
        previewer.asked = {};
    }

    // Only the newest wanted waits, so one it replaces is no longer asked,
    // and its drawer asks again when it's next shown rather than never.
    function preview(path) {
        if (!serving.ready || previewer.asking !== "") {
            if (previewer.next !== "" && previewer.next !== path)
                delete previewer.asked[previewer.next];
            previewer.next = path;
            Qt.callLater(previewer.start);
            return;
        }
        previewer.asking = path;
        // A path with a line end in it can't be asked as a line, so it has
        // nothing to show.
        if (path.includes("\n"))
            Qt.callLater(previewer.previewed, "");
        else
            serving.write(`show ${path}\n`);
    }

    // file-preview's answer to the one asked, then the next wanted. The
    // newest answer goes last, so the oldest kept is the first let go.
    function previewed(line) {
        const path = previewer.asking;
        previewer.asking = "";
        if (path !== "") {
            const kept = Object.assign({}, previewer.previews);
            delete kept[path];
            kept[path] = Captured.shownFrom(line);
            for (const old of Object.keys(kept).slice(0, -previewer.most)) {
                delete kept[old];
                delete previewer.asked[old];
                delete previewer.changes[old];
            }
            previewer.previews = kept;
        }
        previewer.askNext();
    }

    // Started a frame later, never inside its own exit, and only while one
    // is still wanted.
    function start() {
        if (!serving.ready && previewer.next !== "")
            serving.running = true;
    }

    function askNext() {
        if (previewer.next === "" || previewer.asking !== "")
            return;
        const next = previewer.next;
        previewer.next = "";
        previewer.preview(next);
    }

    // Started as the first drawer wants it and kept. One that stops is
    // started again for what's wanted next; one that stops before
    // answering anything is given up on for what's wanted, so one that
    // can't start isn't started again and again, and said to be silent.
    Process {
        id: serving

        property bool ready: false
        property bool answered: false

        command: (previewer.command ?? []).concat(["serve"])
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                serving.answered = true;
                previewer.silent = false;
                previewer.previewed(line);
            }
        }
        onStarted: {
            ready = true;
            answered = false;
            Qt.callLater(previewer.askNext);
        }
        onExited: (code, status) => {
            ready = false;
            if (!answered) {
                previewer.next = "";
                if (!previewer.silent) {
                    console.warn(`file-preview stopped before answering (${code}); `
                        + "its drawers say it isn't answering");
                }
                previewer.silent = true;
            }
            if (previewer.asking !== "")
                previewer.previewed("");
            else
                Qt.callLater(previewer.start);
        }
    }
}
