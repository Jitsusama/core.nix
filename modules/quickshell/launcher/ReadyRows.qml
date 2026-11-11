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
// of other kinds the place watches beside its rows, a summary, a head or
// a drawer served apart, are held by key and never placed. A row is made
// as every place's are by `thing`, its served acts its verbs, which ask
// back on the same socket, copy, open a link or step within the launcher,
// so a place moving onto it keeps no act of its own.
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
    // Whether the rows are watched now, as Daemon.qml's `live`: a place
    // whose daemon reads only while watched, as Heat's reads its sensors
    // each second, is live only while it's seen, and keeps its rows held
    // meanwhile. `paced` waits a watch begun by a key on the next frame.
    property bool live: true
    property var paced: null
    // Whether what it makes is drawn now, so a row saying a time is said
    // again as the minute turns: the place's launcher open.
    property bool drawn: false
    // What the place makes of a row, given the row the record holds and the
    // record's key; by default the row as served.
    property var make: (row, key) => row
    // The place's provider, which makes its rows as every place's are
    // (`thing`) and holds what an act asks drawn until it's done. Not named
    // `provider`, which would hide the place's own id of that name inside
    // its ReadyRows.
    property var maker: null

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
    // Which row names each drawer served apart, by the drawer's key, so
    // the row is made again once its drawer changes.
    readonly property var drawers: ({})
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
            live: ready.live
            paced: ready.paced
            ahead: ready.ahead
            onHeard: (changed, gone) => ready.took(changed, gone)
        }
    }

    // Not watched, nothing is held, so nothing it made stands.
    onActiveChanged: {
        if (active)
            return;
        for (const store of [made, drawers, ticking]) {
            for (const key of Object.keys(store))
                delete store[key];
        }
        timed = 0;
        eachMinute = 0;
        keys = [];
        placings += 1;
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
            if (key in drawers)
                delete made[drawers[key]];
            delete made[key];
            if (ticking[key] === "minute")
                eachMinute -= 1;
            if (key in ticking)
                timed -= 1;
            delete ticking[key];
        }
        for (const key of said) {
            const drawer = Ready.rowOf(held[key])?.drawer;
            if (typeof drawer === "string")
                drawers[drawer] = key;
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

    // Every row of the place's own as the place made it, in the daemon's
    // order, or with `parent` the key of a row, the rows inside it.
    function rows(parent) {
        void placings;
        return Ready.inside(keys, held, parent ?? "").map(key => rowAt(key))
            .filter(row => row !== null);
    }

    // The rows inside the row of `key`, as `rows` gives them.
    function inside(key) {
        return rows(key);
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

    // A row's drawer as Drawer.qml draws one, its times said as of now, its
    // own or the one served beside it.
    function drawerOf(row) {
        return Ready.drawerOf(Ready.drawerServed(row, held), Date.now(), timeWords);
    }

    // A served row, the record of `key`'s, as a place draws one, made by
    // its provider: its words, its drawer read as it's shown, and its acts
    // as its verbs, Return doing the first. A row whose acts step into rows
    // steps into those inside it. `fields` are the place's own, over these,
    // so a place's `make` is `(row, key) => ready.thing(row, key, {...})`;
    // the row is given rather than looked up, as an act's `becomes` is made
    // by it too. It's a collection's member, its name made ready to match
    // only once it's searched (Provider.qml's member). Null without a
    // provider or a row.
    function thing(row, key, fields) {
        if (!row || !maker)
            return null;
        const acts = actsOf(row);
        const first = acts[0] ?? null;
        const steps = acts.some(one => one.step === "rows");
        const verbs = () => steps ? inside(key) : acts.map(one => actRow(key, one));
        // Only a Return that steps into rows leaves the launcher open; one
        // that copies closes it, as a value's row always has.
        const acting = acts.length === 0 ? {} : {
            verb: verbOf(first),
            stays: first.step === "rows",
            run: () => act(key, first),
            verbs: verbs,
            targets: verbs
        };
        // A row whose Return copies is a value, drawn as Provider.qml's
        // copyRow draws one, and what it copies is what y joins.
        const copying = Ready.actForm(first) !== "copy" ? {} : {
            figure: true,
            plain: true,
            now: true,
            copies: () => first.copy
        };
        const items = Ready.sideItems(row.side, Date.now(), timeWords);
        return maker.member(Object.assign({
            key: row.thing ?? key,
            thing: !!row.drawer,
            name: row.name ?? "",
            glyph: Theme.glyph[row.glyph ?? ""] ?? "",
            hue: row.hue ?? "",
            side: Ready.sideJoined(items),
            // Each item, so the row keeps an item's width and draws its hue.
            sideItems: items,
            note: noteOf(row),
            // Its bar, strip or spark, { shape, values, warn }, as served.
            graph: row.graph ?? null,
            reads: row.reads ?? "",
            unfinished: row.unfinished === true,
            // A row a step in that asks Joel to be sure, so its Return does
            // its act (Launcher.lands), as an act that can't be taken back is.
            sure: row.sure === true,
            drawer: () => drawerOf(row)
        }, acting, copying, fields ?? {}));
    }

    // The acts of a row the launcher can do here: a step into the drawer
    // only where the launcher steps into one, so nothing offered does
    // nothing.
    function actsOf(row) {
        const launcher = maker?.launcher;
        return Ready.actsOf(row).filter(one => one.step !== "drawer"
            || typeof launcher?.intoDrawer === "function");
    }

    // The verb an act says, words.json's for its key.
    function verbOf(one) {
        return Ready.verbOf(one, Words.verb(one.key));
    }

    // An act as a row of its own, among a row's verbs.
    function actRow(key, one) {
        return maker.thing({
            key: one.key,
            what: "",
            verb: verbOf(one),
            name: Words.name(one.key),
            glyph: Words.glyph(one.key),
            hue: "action",
            stays: one.step === "rows",
            copies: typeof one.copy === "string" ? () => one.copy : undefined,
            run: () => act(key, one)
        });
    }

    // Does an act the row of `key` serves: copies its words, opens its link
    // through niri, steps within the launcher, or asks it of the daemon.
    function act(key, one) {
        const launcher = maker?.launcher;
        switch (Ready.actForm(one)) {
        case "copy":
            if (one.copy !== "")
                Quickshell.clipboardText = one.copy;
            return;
        case "open":
            Niri.spawn(["xdg-open", one.open]);
            return;
        case "step":
            if (one.step === "rows")
                launcher?.enter(rowAt(key));
            else
                launcher?.intoDrawer(rowAt(key));
            return;
        case "ask":
            asked(key, one);
        }
    }

    // Asks an act of the daemon on the rows' own connection, its row drawn
    // at once as the act says it becomes, made as the place makes its rows,
    // until the daemon serves it so. An act that says nothing of how its row
    // reads holds the row as it is until it's answered. Either is taken back
    // with the daemon's words if it's refused, or the launcher's if it isn't
    // answered, so nothing fails silently (launcher/Becoming.js).
    function asked(key, one) {
        const row = served(key);
        if (!row || !link)
            return;
        const shown = row.thing ?? key;
        let answered = false;
        const becomes = one.becomes ? ready.make(one.becomes, key) : rowAt(key);
        const how = one.becomes ? undefined : { done: () => answered };
        maker?.becomes(shown, becomes, row.name ?? "", one.ask.act ?? verbOf(one), how);
        link.ask(one.ask, answer => {
            const why = Ready.refusal(answer, Words.field.becoming ?? {});
            if (why === "")
                answered = true;
            else
                maker?.refused(shown, why);
        });
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
