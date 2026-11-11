pragma ComponentBehavior: Bound
// One place's rows as its daemon serves them ready (ready-rows.md, the
// ready-row form): held over one socket kept open, each record as the
// crate's Kept wraps it, `{ key, kind: "row", row }`, placed by its
// `order` as a change names it and never sorted, and made into what the
// place draws once a change to it, never once an ask (Joel, 2026-11-10:
// "the daemons need to be the data layer, and they need to serve the
// launcher"). What reads the clock, a time at a row's side or in its
// drawer, is worded here (ReadyRows.js), and only the rows that say one are
// made again as the minute turns, and only while they're drawn. Records
// of other kinds the place watches beside its rows, a summary or a head,
// are held by key and never placed.
import QtQuick
import Quickshell
import qs
import "ReadyRows.js" as Ready

Scope {
    id: ready

    // The daemon's socket, and the place its rows are watched for.
    required property string socket
    required property string place
    // The kinds of record watched beside the rows, as `summary`.
    property var beside: []
    // What else narrows the watch, as the daemon reads it.
    property var watching: ({})
    // The keys of rows the place is about to draw more of, its selected
    // row's, so the daemon reads their drawers and inside rows only while
    // they're ahead (ready-rows.md's Notes).
    property var ahead: []
    // Whether it's watched: false where there's no runtime folder, as in
    // a test with no session, so nothing is asked of a socket never made.
    property bool active: !!Quickshell.env("XDG_RUNTIME_DIR")
    // Whether what it makes is drawn now, so a row saying a time is said
    // again as the minute turns: the place's launcher open.
    property bool drawn: false
    // What the place makes of a row, given the row the record holds and the
    // record's key; by default the row as served.
    property var make: (row, key) => row

    // Each record by its key, as the daemon last said it, and its state.
    readonly property var link: daemon.item
    readonly property var held: link?.held ?? ({})
    readonly property string state: link?.state ?? "starting"
    // The rows' keys in their order, a new list each change.
    property var keys: []
    // Counts the changes, so a binding reading `rows()` is told of each.
    property int placings: 0
    // Each row as last made, by key, made again only once its record
    // changes or its time is said anew.
    readonly property var made: ({})
    // The rows that say a time, by key, each saying how often it says a
    // new one, "minute" or "day" (ReadyRows.js's ticks), and how many say
    // one at all and how many each minute, so a binding is told.
    readonly property var ticking: ({})
    property int timed: 0
    property int eachMinute: 0
    // The day the rows were last said on, so those saying a day are said
    // again once it turns.
    property string saidOn: Qt.formatDate(new Date(), "yyyy-MM-dd")

    // A change to the records, past placing: the keys changed and gone.
    signal heard(var changed, var gone)

    LazyLoader {
        id: daemon
        active: ready.active

        Daemon {
            socket: ready.socket
            kind: ["row"].concat(ready.beside)
            watching: Object.assign({ place: ready.place }, ready.watching)
            answersWhole: true
            ahead: ready.ahead
            onHeard: (changed, gone) => ready.took(changed, gone)
        }
    }

    // Each key a change names let go of what was made of it, and the rows
    // placed again. Its lists come through the signal converted, so they're
    // copied by index.
    function took(changed, gone) {
        const said = [];
        const dropped = [];
        for (let at = 0; at < changed.length; at += 1)
            said.push(changed[at]);
        for (let at = 0; at < gone.length; at += 1)
            dropped.push(gone[at]);
        for (const key of said.concat(dropped)) {
            delete made[key];
            if (ticking[key] === "minute")
                eachMinute -= 1;
            if (key in ticking)
                timed -= 1;
            delete ticking[key];
        }
        for (const key of said) {
            const every = Ready.ticks(Ready.rowOf(held[key]));
            if (every !== "") {
                ticking[key] = every;
                timed += 1;
            }
            if (every === "minute")
                eachMinute += 1;
        }
        keys = Ready.placed(keys, held, said, dropped);
        placings += 1;
        heard(said, dropped);
    }

    // The row the record of `key` holds, as served, or null.
    function served(key) {
        return Ready.rowOf(held[key]);
    }

    // What the place made of the row of `key`, made now if not yet.
    function rowAt(key) {
        if (!(key in made)) {
            const row = served(key);
            made[key] = row ? ready.make(row, key) : null;
        }
        return made[key];
    }

    // Every row as the place made it, in the daemon's order.
    function rows() {
        void placings;
        return keys.map(key => rowAt(key)).filter(row => row !== null);
    }

    // The words a time is said in, the launcher's own for each form
    // (words.json's field.times), with the calendar's names of days and
    // months and, for a time somewhere else, `zoneSaid` where the place
    // gives one, else the zone's name.
    property var zoneSaid: null
    readonly property var timeWords: Object.assign({
        date: (date, format) => Qt.formatDate(date, format),
        zone: (zone, now) => ready.zoneSaid ? ready.zoneSaid(zone, now) : String(zone ?? "")
    }, Words.field.times ?? {})

    // A time in the ready-row form, said as of now; "" for none.
    function timeSaid(time) {
        const told = typeof time?.at === "number" || !!time?.zone;
        return told ? Ready.timeSaid(time, Date.now(), timeWords) : "";
    }

    // A row's side as one line, its times said as of now.
    function sideOf(row) {
        return Ready.sideSaid(row?.side, Date.now(), timeWords);
    }

    // A row's note, words or a time said as of now.
    function noteOf(row) {
        return Ready.noteSaid(row?.note ?? "", Date.now(), timeWords);
    }

    // A row's drawer as Drawer.qml draws one, its times said as of now.
    function drawerOf(row) {
        return Ready.drawerOf(row?.drawer, Date.now(), timeWords);
    }

    // As the minute turns while drawn, the rows saying a time that can read
    // differently by then are made again, and only those, and those saying
    // a day once the day has turned too; started at the next whole minute,
    // so the words turn as the clock does. Nothing runs while it's shut.
    Timer {
        running: ready.drawn && ready.timed > 0
        repeat: true
        interval: 60000 - Date.now() % 60000
        onTriggered: {
            ready.turned();
            interval = 60000 - Date.now() % 60000;
        }
    }
    onDrawnChanged: {
        if (drawn)
            turned();
    }

    // The rows whose words the clock has moved on since they were made,
    // let go of, so they're made again as they're next drawn.
    function turned() {
        const today = Qt.formatDate(new Date(), "yyyy-MM-dd");
        const daily = today !== saidOn;
        saidOn = today;
        if (eachMinute === 0 && !daily)
            return;
        for (const key of Object.keys(ticking)) {
            if (daily || ticking[key] === "minute")
                delete made[key];
        }
        placings += 1;
    }
}
