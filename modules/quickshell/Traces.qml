// Each opening of the launcher as a trace (launcher/Traces.js shapes it):
// the opening its root span, and each step in, step back, act and search a
// span under it, ended once the list has settled and holding where its
// first row was laid. The last spans are kept in memory, read with
// `qs ipc call launcher traced`, and each opening is sent whole to the
// collector as it closes, as one line on its socket; with no collector, it's
// kept here alone. A list settled with space above its first row is said
// in the journal with its trace's id, so the void Joel saw can be found in
// the trace that holds it.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Traces.js" as Traces

Scope {
    id: root

    // How many spans to keep: an opening makes a few dozen.
    property int remember: 2000
    // The collector's socket (5FWIUS's, 2026-10-31), which answers nothing.
    property string collector: `${Quickshell.env("XDG_RUNTIME_DIR")}/collector/lines.sock`

    // The opening under way, or null while closed; the spans begun in it
    // waiting on the list to settle; and every span kept, newest last.
    property var opening: null
    property var waiting: []
    property var spans: []

    function opened(about) {
        opening = Traces.begun("open", Date.now(), null, about);
        waiting = [];
    }

    // A span begun in the opening, ended as its list settles.
    function began(name, about) {
        if (opening === null)
            return;
        waiting = waiting.concat([Traces.begun(name, Date.now(), opening, about)]);
    }

    // The list settled, laid out as `laid` says: each span waiting ends
    // with it.
    function settled(laid) {
        if (opening === null || waiting.length === 0)
            return;
        const now = Date.now();
        for (const span of waiting) {
            const done = Traces.ended(span, now, laid);
            keep(done);
            if (Traces.voided(done.about))
                console.info(`launcher: ${done.name} settled with its first row`
                    + ` ${done.about.firstAt} px below the top (trace ${done.trace})`);
        }
        waiting = [];
    }

    function closed(about) {
        if (opening === null)
            return;
        const now = Date.now();
        const trace = opening.trace;
        for (const span of waiting)
            keep(Traces.ended(span, now, { settled: false }));
        keep(Traces.ended(opening, now, about));
        opening = null;
        waiting = [];
        send(spans.filter(span => span.trace === trace));
    }

    function keep(span) {
        spans = Traces.kept(spans, span, remember);
    }

    // The spans kept, as JSON, for `qs ipc call launcher traced`.
    function traced() {
        return JSON.stringify(spans);
    }

    // One opening sent to the collector, on a connection of its own made
    // only to send it and let go after, or as soon as nothing answers: a
    // socket refused once never tries again by itself (Daemon.qml), so one
    // kept across openings would stop sending once the collector was down.
    // Connected once made, since one refused while it's still being made
    // couldn't be let go (FileChooser.qml).
    function send(each) {
        unsent = unsent.concat([`${JSON.stringify(Traces.request(each))}\n`]);
        // Not preloaded, a reload only marks the file stale: asking for its
        // text is what starts the look, off the main thread, answered by
        // loaded or loadFailed. One already under way answers for this too.
        listening.reload();
        listening.text();
    }

    // The lines waiting on the look at the collector's socket.
    property var unsent: []

    // Whether the collector's socket is there, looked at as each opening is
    // sent: a connection to none warns in the journal, and the collector
    // being down isn't a fault of the shell's (Joel, 2026-10-29: "gracefully
    // handle them being down"). A socket isn't a file, so it fails to load as
    // one; a path with nothing there fails as not found, and the lines go. A
    // single stat each, watching nothing. Nothing here may start another
    // look from loaded or loadFailed: the one finishing would drop it.
    FileView {
        id: listening

        path: root.collector
        preload: false
        printErrors: false
        onLoadFailed: error => root.sendOrDrop(error === FileViewError.NotAFile)
        onLoaded: root.sendOrDrop(false)
    }

    function sendOrDrop(there) {
        const lines = unsent;
        unsent = [];
        if (!there)
            return;
        for (const line of lines) {
            const made = sender.createObject(root, { line, path: root.collector });
            made.connected = true;
        }
    }

    Component {
        id: sender

        Socket {
            id: socket

            required property string line

            connected: false
            onConnectionStateChanged: {
                if (!socket.connected)
                    return;
                socket.write(socket.line);
                socket.flush();
                socket.connected = false;
                socket.destroy();
            }
            onError: socket.destroy()
        }
    }
}
