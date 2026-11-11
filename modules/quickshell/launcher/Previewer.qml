// What a file shows of itself, for a provider's drawers: Capture's things
// taken and Neovim's files ask it as their drawers draw. The programs
// daemon answers (modules/programs/src/preview.rs), over the one socket
// this holds open, asking the file-preview it keeps, so the shell starts
// nothing for a drawer shown (Joel, 2026-11-10: the daemons are the data
// layer, and the launcher spawns no process to get data). It waits,
// costing nothing, while the launcher's shut.
//
// A stopgap still, as the drawers make their facts from what it says
// here; Programs' ready rows (Thu 12 Nov) carry each drawer's facts
// whole, and this goes.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import QtQuick

Scope {
    id: previewer

    // Whether the provider's drawers preview at all, as its settings say.
    property bool can: false
    // The programs daemon's socket, where a case gives another.
    property string socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`

    // What file-preview said of each file, by its path (null where it
    // couldn't), when the file had changed as it was asked, and which were
    // asked. At most `most` files are kept, the oldest let go first, so a
    // shell that runs for days doesn't keep every file it was ever shown.
    property var previews: ({})
    property var changes: ({})
    property var asked: ({})
    property int most: 64
    // The file asked now, and the next wanted while it's answered or the
    // daemon is reached, so a drawer scrolled past is never asked behind
    // the one shown.
    property string asking: ""
    property string next: ""

    // Whether file-preview isn't answering, so drawers say it isn't
    // rather than lacking their facts unsaid; cleared by its next answer.
    property bool silent: false
    // Whether the daemon is reached, so what's asked is asked at once.
    readonly property bool ready: daemon.state === "answering"

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
        // A daemon that's down can't say, so its drawers say so.
        if (daemon.state === "not answering")
            previewer.silent = true;
        if (daemon.state !== "answering" || previewer.asking !== "") {
            if (previewer.next !== "" && previewer.next !== path)
                delete previewer.asked[previewer.next];
            previewer.next = path;
            return;
        }
        previewer.asking = path;
        const key = `preview:${path}`;
        daemon.get([key], 0, answer => previewer.previewed(path,
            answer === null ? undefined : daemon.held[key]));
    }

    function askNext() {
        if (previewer.next === "" || previewer.asking !== "")
            return;
        const next = previewer.next;
        previewer.next = "";
        previewer.preview(next);
    }

    // The daemon's answer for one asked, then the next wanted. The newest
    // answer goes last, so the oldest kept is the first let go. One
    // unanswered stays asked, so its drawer, drawn again to say so, asks
    // nothing more until the launcher next opens (forget).
    function previewed(path, record) {
        previewer.asking = "";
        const answering = record?.answering === true;
        previewer.silent = !answering;
        if (answering)
            previewer.keep(path, record.shown ?? null);
        previewer.askNext();
    }

    function keep(path, shown) {
        const kept = Object.assign({}, previewer.previews);
        delete kept[path];
        kept[path] = shown;
        for (const old of Object.keys(kept).slice(0, -previewer.most)) {
            delete kept[old];
            delete previewer.asked[old];
            delete previewer.changes[old];
        }
        previewer.previews = kept;
    }

    Daemon {
        id: daemon

        socket: previewer.socket
        kind: "preview"
        live: false

        onStateChanged: previewer.askNext()
    }
}
