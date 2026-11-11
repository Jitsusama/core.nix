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
// `heard` names the keys each message changed and the keys it let go,
// so a source keeps what it makes from them by key, in a plain object,
// and remakes only those, never re-walking `held`. A daemon that sends
// ready rows says each as a record in one envelope: its `key`, an
// `order` the shell places it by without sorting the rest, and the
// fields its delegate draws, worded from words.json as handed to the
// daemon. Whatever reads the clock, as "4 min ago" does, travels as a
// time and is worded here, so no row is rewritten only because time
// passed. The machine daemon's rows (modules/machine) are the first.
//
// A daemon that's down is said, never waited on: `state` is "starting"
// until it first answers, then the daemon's own word, or "not answering"
// while its socket is gone. A socket that refuses is tried again after
// two seconds, and twice as long after each refusal since it last
// answered, a minute at most, so a daemon down for good isn't asked by
// every domain's connection every two seconds; one that isn't there at
// all, as Slack's isn't before Joel signs in, is tried again only once it appears, so a machine without it
// isn't asked, and warned of, every two seconds for as long as it runs.
// What was held stays held through a gap, and is replaced by what the
// daemon says once it answers again, unless `keptThroughGap` says to let
// it go.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: daemon

    // Its socket, as the daemon's unit makes it.
    required property string socket
    // The kind of record it holds, or a list of kinds, as Google's daemon
    // holds meetings, mail, files and its account over one connection;
    // sent as given in both watches, and watched whole unless `watching`
    // narrows it.
    required property var kind
    // What narrows the watch of its kind, as the daemon reads it, such as
    // `{ listed: true }` for only those a source lists, so a kind with
    // thousands of records holds only the few shown with nothing typed.
    property var watching: ({})
    // The keys to watch ahead, as the shell is about to need them; each
    // change sends the watch again, which replaces the one before, and
    // emptied it's unwatched, so a daemon reading something only while
    // a key is ahead, as wifid reads a network's link, stops.
    property var ahead: []
    // Whether its kind is watched now. A daemon that reads only while
    // watched, as the machine daemon reads the processor each second,
    // is watched only while what it says can be seen: false unwatches
    // it, and true watches it again, answered whole at once.
    property bool live: true
    // Whether it's watched until its next whole answer though not live,
    // as `refresh()` asks, so what's held is current once without being
    // watched from then on.
    property bool refreshing: false
    readonly property bool watched: live || refreshing
    // What a watch begun waits on, when it's begun by what Joel pressed:
    // something with `afterFrame(then)`, as the launcher is, which runs
    // `then` once its next frame is drawn, or at once while nothing is
    // shown. A step into Machine makes five of its parts seen at once,
    // and each whole answer took the GUI thread up to 18 ms just before
    // the step's frame, a median 46 ms in all (keeps-pace's loaded VM,
    // 2026-11-10); waiting, the step is drawn from what's held, as a
    // closed launcher's rows are, and the answers land a frame later.
    // Null, as outside the launcher, watches at once.
    property var paced: null
    // Counts each change of whether it's watched, so a watch left waiting
    // on a frame isn't sent once it's no longer the newest.
    property int watchesBegun: 0
    onWatchedChanged: {
        // What's held from an earlier watch may be hours old, so the next
        // watch is unanswered until its own whole answer comes.
        answeredThisWatch = false;
        watchesBegun += 1;
        if (wire === null)
            return;
        if (!watched) {
            tell({ id: "all", ask: "unwatch" });
            return;
        }
        if (paced === null) {
            watchAll();
            return;
        }
        const begun = watchesBegun;
        paced.afterFrame(() => {
            if (begun === watchesBegun && watched)
                watchAll();
        });
    }

    // Reads its kind whole once now, as a daemon reads it when watched,
    // though nothing shows it: for what's made from what's held while
    // the launcher is closed, as what's inside the battery is. A daemon
    // live already has it, and one not answering is read once it does.
    function refresh() {
        if (!live)
            refreshing = true;
    }
    // Whether what was held stays held while the daemon isn't answering,
    // as someone found stays found. A kind true only while the daemon
    // answers, as Slack's unread counts are, lets it go, so no old count
    // is shown as current, and none gone in the gap stays held once the
    // daemon answers again.
    property bool keptThroughGap: true
    // Whether a change says only the fields that differ, one dropped as
    // null, as mycelium's node sends them (its PROTOCOL.md, The Daemon
    // Interface), so each is laid over the record held rather than
    // replacing it. A record first said, or said whole in answer to a
    // watch or a get, still replaces what was held.
    property bool changesDiffer: false
    // Whether the watch's answer is the whole of its kind, as the node's
    // is, so a record held that it doesn't hold went while the daemon
    // wasn't answering, and is let go rather than kept as though it
    // stood. A daemon whose watch is narrowed, and whose gets hold more,
    // leaves it false.
    property bool answersWhole: false
    // Whether the watch of its kind has been answered once, so a kind with
    // no records is known to hold none, rather than not yet said.
    property bool answered: false
    // Whether the watch now running has had its whole answer, so what's
    // held is this watch's rather than one before an unwatch, as a place
    // asked cold a second time is read afresh (Y2YBTS's review of #2433).
    property bool answeredThisWatch: false

    // Every record held, by key: plain fields, read beside `revision`.
    readonly property var held: ({})
    property int revision: 0
    // How it stands: "starting", the daemon's own word, or "not answering".
    property string state: "starting"
    // Each of its sources by name, with the daemon's word for it.
    property var sources: ({})

    // Said once a message, after `held` takes it: the keys it said anew
    // and the keys it let go. A whole answer says every key it holds,
    // and the keys it dropped as gone. A signal rather than a property,
    // so nothing binds to it and re-walks what's held.
    signal heard(var changed, var gone)
    // Said with every line the daemon says, once it's taken, so a domain
    // hears what a daemon tells beyond its records, as the passwords
    // daemon tells a watch what its last check found.
    signal told(var answer)

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
    // the answer, or null when the daemon isn't there to ask, and each
    // answer after one saying `done: false` too.
    function ask(said, then) {
        asked += 1;
        sent(said, then, `ask-${asked}`);
    }

    // Asks for records only this answer needs, as a search does: they're
    // given to `then` and never held, so searching thousands holds no more
    // than what's listed. `then` is given null when the daemon isn't
    // there to ask.
    function find(said, then) {
        asked += 1;
        const id = `find-${asked}`;
        apart[id] = true;
        sent(said, then, id);
    }

    // The answers whose records aren't held, by id.
    property var apart: ({})

    function sent(said, then, id) {
        if (wire === null) {
            delete apart[id];
            if (then)
                then(null);
            return;
        }
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
        if (watched)
            watchAll();
        watchAhead();
    }

    function watchAll() {
        tell(Object.assign({ id: "all", ask: "watch", kind: kind }, watching));
    }

    function watchAhead() {
        if (wire !== null && ahead.length > 0)
            tell({ id: "ahead", ask: "watch", kind: kind, keys: ahead });
    }
    onAheadChanged: {
        if (ahead.length > 0)
            watchAhead();
        else
            tell({ id: "ahead", ask: "unwatch" });
    }

    // Records said whole, each replacing what was held by its key, and
    // keys the daemon says are gone taken out, whether in a gone list or
    // as a record saying `gone`, as the people daemon tells of someone no
    // source knows any more.
    function took(records, gone, changes) {
        const changed = [];
        const dropped = [];
        for (const record of records ?? []) {
            if (typeof record?.key !== "string")
                continue;
            if (record.gone === true) {
                if (record.key in held) {
                    delete held[record.key];
                    dropped.push(record.key);
                }
                continue;
            }
            held[record.key] = changes && changesDiffer && record.key in held
                ? mended(held[record.key], record) : record;
            changed.push(record.key);
        }
        for (const key of gone ?? []) {
            if (key in held) {
                delete held[key];
                dropped.push(key);
            }
        }
        if (changed.length === 0 && dropped.length === 0)
            return;
        revision += 1;
        heard(changed, dropped);
    }

    // A record held with a change laid over it: a new object, so a reader
    // holding the old one still reads it as it was, each field the change
    // says replacing the held one's, and each it says as null let go.
    function mended(was, change) {
        const now = Object.assign({}, was);
        for (const field in change) {
            if (change[field] === null)
                delete now[field];
            else
                now[field] = change[field];
        }
        return now;
    }

    // How many lines it has read, for the check that the shut shell hears
    // nothing of a window resized. Read only when asked, so nothing wakes
    // for it.
    property int linesHeard: 0

    function said(line) {
        linesHeard += 1;
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
            told(answer);
            return;
        }
        // How its sources stand, as a watch is told once that changes,
        // so a sign-in that goes is heard as it goes.
        if (answer.sources !== null && typeof answer.sources === "object"
            && answer.records === undefined && answer.changed === undefined)
            sources = answer.sources;
        const whole = answer.id === "all" && Array.isArray(answer.records);
        if (answer.id in apart)
            delete apart[answer.id];
        else if (whole && answersWhole) {
            const holds = new Set(answer.records.map(record => record?.key));
            took(answer.records, Object.keys(held).filter(key => !holds.has(key)), false);
        } else
            took(answer.records ?? answer.changed, answer.gone, answer.records === undefined);
        if (whole) {
            answered = true;
            answeredThisWatch = live;
            // A daemon that drops before this keeps it refreshing until
            // the next connection's whole answer (greet).
            refreshing = false;
        }
        // An answer saying it isn't done, as each of a join's steps is,
        // keeps its place for the next.
        const then = awaited[answer.id];
        if (then) {
            if (answer.done !== false)
                delete awaited[answer.id];
            then(answer);
        }
        told(answer);
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
                    daemon.refusals = 0;
                    daemon.wire = socket;
                    daemon.greet();
                } else {
                    daemon.gone();
                    daemon.tryAgain();
                }
            }
            onError: failed => {
                daemon.gone();
                if (failed !== daemon.serverNotFound)
                    daemon.tryAgain();
            }
        }
    }

    // QLocalSocket::ServerNotFoundError: nothing at the socket's path.
    readonly property int serverNotFound: 2

    // The socket's file, watched and never read, so one made after the
    // last try is tried at once. Watched only while there's no wire, since
    // the watch covers the socket's folder too, and a folder such as the
    // node's or spend-watch's holds files rewritten every few seconds,
    // each a wake of the shut shell for nothing while the wire holds.
    FileView {
        path: daemon.socket
        preload: false
        printErrors: false
        watchChanges: daemon.wire === null
        onFileChanged: {
            if (daemon.wire === null)
                daemon.connectAgain();
        }
    }

    function connectAgain() {
        again.stop();
        link.active = false;
        link.active = true;
    }

    // Gone: said so, and every answer awaited given none. The watch went
    // with the wire, so what's held waits for the next watch's answer.
    function gone() {
        wire = null;
        state = "not answering";
        answeredThisWatch = false;
        apart = {};
        if (!keptThroughGap)
            took([], Object.keys(held));
        const waiting = awaited;
        awaited = {};
        for (const id in waiting)
            waiting[id](null);
    }

    // How many times it's been refused since it last answered.
    property int refusals: 0

    // How long to wait before trying again, after so many refusals.
    function waitAfter(refused) {
        return Math.min(2000 * 2 ** refused, 60000);
    }

    // A connection dropped says so twice, as its state and as an error,
    // so a try already waiting counts once.
    function tryAgain() {
        if (again.running)
            return;
        again.interval = waitAfter(refusals);
        refusals += 1;
        again.restart();
    }

    Timer {
        id: again

        interval: 2000
        onTriggered: daemon.connectAgain()
    }
}
