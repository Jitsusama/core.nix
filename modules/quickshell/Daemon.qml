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
// all, as Slack's isn't before Joel signs in, is tried again as a refusal
// is a few times, about fourteen seconds in all, since a daemon starting
// again takes its socket away and makes it anew, then only once it
// appears, so a machine without it isn't asked, and warned of, every two
// seconds for as long as it runs.
// What was held stays held through a gap, and is replaced by what the
// daemon says once it answers again, unless `keptThroughGap` says to let
// it go.
//
// A daemon no other process may reach, as the passwords daemon is, has no
// socket: given `command`, the shell runs it as its own child for as long
// as the shell runs, and the same lines go over its standard input and
// output. One that ends is started again on the same backoff as a socket
// that refuses, and what it says on its standard error goes to the
// shell's journal, which is why such a daemon says no secret there.
import QtQuick
import Quickshell
import Quickshell.Io
import "Hearing.js" as Hearing

Scope {
    id: daemon

    // Its socket, as the daemon's unit makes it, or empty for one the
    // shell runs itself.
    property string socket: ""
    // The daemon the shell runs as its own child, asked over its standard
    // input and output, for one with no socket; empty for one that has.
    property var command: []
    // What the child's environment adds, as a check names a copier.
    property var environment: ({})
    readonly property bool runsItself: command.length > 0
    // The kind of record it holds, or a list of kinds, as Google's daemon
    // holds meetings, mail, files and its account over one connection;
    // sent as given in both watches, and watched whole unless `watching`
    // narrows it.
    required property var kind
    // What narrows the watch of its kind, as the daemon reads it, such as
    // `{ listed: true }` for only those a source lists, so a kind with
    // thousands of records holds only the few shown with nothing typed.
    property var watching: ({})
    // What the rows the daemon makes are made of, as Programs' are of
    // the hidden programs it lists all the same: said in a settings ask
    // before each watch and again as it changes, and held by the daemon
    // until the next (docs/daemons.md, Settings). Null says none.
    property var settings: null
    onSettingsChanged: settle()
    function settle() {
        if (settings !== null)
            tell(Object.assign({ id: "settings", ask: "settings" }, settings));
    }
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
    // Whether it's connected at all. A daemon that rests once nothing has
    // held it for a while, as pi-verbs ends after five minutes alone, is
    // connected only while what it says can be used, so it's never held
    // awake all day (85PKB2's harness #1436): false lets the connection
    // go and tries nothing again, true connects at once. What was held
    // stays held between, as through a gap.
    property bool connects: true
    onConnectsChanged: {
        if (connects) {
            connectAgain();
            return;
        }
        again.stop();
        link.active = false;
        child.active = false;
        if (wire !== null)
            gone();
    }
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

    // A watch asked otherwise, as one paced while the launcher's shut and
    // unpaced once it opens, is sent again, the same watch told at once
    // what it held back. Paced by a frame, it's sent once the frame is
    // drawn, since the opening draws from what's held. Not sent again,
    // Agents' top level stayed told at most every ten seconds with the
    // launcher open, so a parked orchestrator woken read as closed
    // (2026-11-13). Said as text, so a binding giving the same watch anew
    // sends nothing.
    readonly property string watchingSaid: JSON.stringify(watching)
    onWatchingSaidChanged: {
        if (wire === null || !watched)
            return;
        if (paced === null) {
            watchAll();
            return;
        }
        const begun = watchesBegun;
        const said = watchingSaid;
        paced.afterFrame(() => {
            if (begun === watchesBegun && said === watchingSaid && wire !== null && watched)
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
    // and the keys it let go. A record said as it's held already isn't
    // said anew, so a whole answer says only the keys whose records
    // differ from what was held, and the keys it dropped as gone. A
    // signal rather than a property, so nothing binds to it and re-walks
    // what's held.
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
        // Its answer is heard as from the line that asked, when one did, so
        // a change an answer tells is timed from the line that set it going
        // and not from the answer's (Hearing.js).
        if (then) {
            const from = Hearing.heardAt();
            const by = Hearing.heardBy();
            awaited[id] = answer => Hearing.within(from, () => then(answer), by);
        }
        tell(Object.assign({}, said, { id: id }));
    }

    // The connection while it's open: the socket itself, since the loader
    // hands it out as its item only after it has said it's connected.
    property var wire: null

    function tell(said) {
        if (wire === null)
            return;
        wire.write(`${JSON.stringify(said)}\n`);
        // A child's input is written as the event loop next runs, and
        // has no flush of its own.
        if (!runsItself)
            wire.flush();
    }

    // Once connected: hello, what its rows are made of, every record of
    // its kind, and those ahead.
    function greet() {
        tell({ id: "hello", ask: "hello", version: 1, client: "shell" });
        settle();
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
    // source knows any more. A record said just as it's held is left be,
    // the object held kept and its key not heard: each opening's watch
    // ahead is answered whole, and every record of it told as changed
    // made every reader remake its rows at every opening, though nothing
    // had changed (Capture's Recent, a place's ready rows, People's).
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
            const holds = record.key in held;
            const now = changes && changesDiffer && holds
                ? mended(held[record.key], record) : record;
            if (holds && alike(held[record.key], now))
                continue;
            held[record.key] = now;
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

    // Whether two records say the same, field by field and item by item,
    // as plain values parsed from a line are compared, whatever the order
    // their fields came in.
    function alike(was, now) {
        if (was === now)
            return true;
        if (was === null || now === null || typeof was !== "object"
            || typeof now !== "object" || Array.isArray(was) !== Array.isArray(now))
            return false;
        const fields = Object.keys(was);
        return fields.length === Object.keys(now).length
            && fields.every(field => field in now && alike(was[field], now[field]));
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

    // Each line is marked as heard from when it's read until all it set
    // going has returned, so the launcher times a change from it
    // (Hearing.js).
    // The daemon its lines are heard as from, by its socket's name:
    // "machine" for machine.sock.
    readonly property string heardAs: socket.split("/").pop().replace(/\.sock$/, "")

    function said(line) {
        const was = Hearing.begun(Date.now(), heardAs);
        try {
            hear(line);
        } finally {
            Hearing.ended(was);
        }
    }

    function hear(line) {
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
            refusals = 0;
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
        // A watch's answer is what's held caught up as the watch begins, as
        // each opening watches again, not a change the daemon told, which
        // comes as `changed`. Heard under its line, each opening's catching
        // up was said as a change in every place it woke, 270 to 380 ms from
        // the line (Joel's journal, 2026-11-11), so it's heard unmarked, and
        // `said` puts the mark back once it's heard.
        if (Array.isArray(answer.records) && (whole || answer.id === "ahead"))
            Hearing.begun(0);
        if (answer.id in apart)
            delete apart[answer.id];
        else if (whole && answersWhole) {
            // What's watched ahead is told by its own watch, which a whole answer of the kind never
            // names, so it isn't let go with what went: a drawer served apart, held ahead, went at
            // each opening whose whole answer came after the ahead one's (FEH1HE, 2026-11-12).
            const holds = new Set(answer.records.map(record => record?.key).concat(ahead));
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

        active: daemon.connects && !daemon.runsItself

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
                if (failed !== daemon.serverNotFound || daemon.refusals < daemon.missingTries)
                    daemon.tryAgain();
            }
        }
    }

    // QLocalSocket::ServerNotFoundError: nothing at the socket's path.
    readonly property int serverNotFound: 2

    // How many times a socket that isn't there is tried again before only
    // its appearing is waited for: two, four and eight seconds after, so a
    // daemon starting again is heard though its socket was made after the
    // watch on its file last fired. A file watch follows the file, and a
    // socket taken away and made anew is another file, so the watch alone
    // missed the machine daemon coming back after a case stood in for it
    // (Y2YBTS, 2026-11-12), and its places said nothing until the shell
    // started again.
    readonly property int missingTries: 3

    // The socket's file, watched and never read, so one made after the
    // last try is tried at once. Watched only while there's no wire, since
    // the watch covers the socket's folder too, and a folder such as the
    // node's or spend-watch's holds files rewritten every few seconds,
    // each a wake of the shut shell for nothing while the wire holds.
    FileView {
        path: daemon.socket
        preload: false
        printErrors: false
        watchChanges: daemon.connects && !daemon.runsItself && daemon.wire === null
        // A socket just made is a daemon just started, so its tries count
        // afresh: one bound but not yet listening refuses the first, and
        // counted on from the tries its absence took, the next came 16 s
        // later, past the records check's ten-second step on a busy
        // builder (2026-11-13), and past a place's opening on his machine.
        onFileChanged: {
            if (daemon.connects && daemon.wire === null) {
                daemon.refusals = 0;
                daemon.connectAgain();
            }
        }
    }

    // The daemon the shell runs, one process a run, made again once it
    // ends, so a fresh one starts with nothing of the last.
    LazyLoader {
        id: child

        active: daemon.connects && daemon.runsItself

        Process {
            id: run

            command: daemon.command
            environment: daemon.environment // qmllint disable incompatible-type
            stdinEnabled: true
            running: true
            stdout: SplitParser {
                onRead: line => daemon.said(line)
            }
            stderr: SplitParser {
                onRead: line => console.warn(`${daemon.command[0]}: ${line}`)
            }
            // Its refusals are forgotten only once it answers hello, so
            // one that ends at once waits longer each time it's started.
            onStarted: {
                daemon.wire = run;
                daemon.greet();
            }
            onExited: code => {
                console.warn(`${daemon.command[0]} ended with ${code}, so it starts again`);
                daemon.gone();
                daemon.tryAgain();
            }
        }
    }

    function connectAgain() {
        again.stop();
        if (!connects)
            return;
        const loader = runsItself ? child : link;
        loader.active = false;
        loader.active = true;
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
        if (again.running || !connects)
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
