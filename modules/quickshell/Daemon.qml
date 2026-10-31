pragma ComponentBehavior: Bound
// One daemon's records, held in the shell over the daemon interface
// (RSCH-20261029-0E8050, version 1): one connection to its socket, kept
// open, on which it says hello, watches every record of its kind, and
// watches ahead the keys the shell is about to need, so the daemon asks
// its sources about them before Joel steps in. Whatever the daemon says
// lands in `held` by key, a record replaced whole, never merged, as the
// daemon sends each whole; `revision` counts the changes, so a binding
// reads `held` beside it and is told once a change, not once a record.
// Nothing is started per question: a step-in is a `get` on the same
// connection, answered by id.
//
// A daemon that's down is said, never waited on: `state` is "starting"
// until it first answers, then the daemon's own word, or "not answering"
// while its socket is gone, and the connection is tried again every two
// seconds. What was held stays held through a gap, and is replaced
// whole by what the daemon says once it answers again.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: daemon

    // Its socket, as the daemon's unit makes it.
    required property string socket
    // The kind of record it holds, watched whole.
    required property string kind
    // The keys to watch ahead, as the shell is about to need them; each
    // change sends the watch again, which replaces the one before.
    property var ahead: []

    // Every record held, by key: plain fields, read beside `revision`.
    readonly property var held: ({})
    property int revision: 0
    // How it stands: "starting", the daemon's own word, or "not answering".
    property string state: "starting"
    // Each of its sources by name, with the daemon's word for it.
    property var sources: ({})

    // Said with the keys a change touched, once a change.
    signal heard(var keys)

    // The answers still awaited, by id, each a function given the answer.
    property var awaited: ({})
    property int asked: 0

    // Asks for these keys now, as a step-in does, waiting at most
    // `patience` for its sources; `then` is given the answer, or null when
    // the daemon isn't there to ask.
    function get(keys, patience, then) {
        ask({ ask: "get", keys: keys, patience: patience }, then);
    }

    // Asks anything the daemon answers by id, as a domain's own ask does
    // (Slack's search), its records held as a get's are; `then` is given
    // the answer, or null when the daemon isn't there to ask.
    function ask(said, then) {
        if (wire === null) {
            if (then)
                then(null);
            return;
        }
        asked += 1;
        const id = `ask-${asked}`;
        if (then)
            awaited[id] = then;
        tell(Object.assign({}, said, { id: id }));
    }

    // The connection while it's open: the socket itself, since the loader
    // hands it out as its item only after it has said it's connected.
    property var wire: null

    function tell(said) {
        if (wire === null)
            return;
        wire.write(`${JSON.stringify(said)}\n`);
        wire.flush();
    }

    // Once connected: hello, every record of its kind, and those ahead.
    function greet() {
        tell({ id: "hello", ask: "hello", version: 1, client: "shell" });
        tell({ id: "all", ask: "watch", kind: kind });
        watchAhead();
    }

    function watchAhead() {
        if (wire !== null && ahead.length > 0)
            tell({ id: "ahead", ask: "watch", kind: kind, keys: ahead });
    }
    onAheadChanged: watchAhead()

    // Records said whole, each replacing what was held by its key, and
    // keys the daemon says are gone taken out.
    function took(records, gone) {
        const keys = [];
        for (const record of records ?? []) {
            if (typeof record?.key !== "string")
                continue;
            held[record.key] = record;
            keys.push(record.key);
        }
        for (const key of gone ?? []) {
            if (key in held) {
                delete held[key];
                keys.push(key);
            }
        }
        if (keys.length === 0)
            return;
        revision += 1;
        heard(keys);
    }

    function said(line) {
        let answer;
        try {
            answer = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (answer === null || typeof answer !== "object")
            return;
        if (answer.id === "hello") {
            state = String(answer.state ?? "answering");
            sources = answer.sources ?? {};
            return;
        }
        took(answer.records ?? answer.changed, answer.gone);
        const then = awaited[answer.id];
        if (then) {
            delete awaited[answer.id];
            then(answer);
        }
    }

    // A fresh connection each time, since one refused or dropped doesn't
    // try again by itself.
    LazyLoader {
        id: link

        active: true

        Socket {
            id: socket

            path: daemon.socket
            connected: true
            parser: SplitParser {
                onRead: line => daemon.said(line)
            }
            onConnectionStateChanged: {
                if (connected) {
                    daemon.wire = socket;
                    daemon.greet();
                } else {
                    daemon.gone();
                    again.restart();
                }
            }
            onError: {
                daemon.gone();
                again.restart();
            }
        }
    }

    // Gone: said so, and every answer awaited given none.
    function gone() {
        wire = null;
        state = "not answering";
        const waiting = awaited;
        awaited = {};
        for (const id in waiting)
            waiting[id](null);
    }

    Timer {
        id: again

        interval: 2000
        onTriggered: {
            link.active = false;
            link.active = true;
        }
    }
}
