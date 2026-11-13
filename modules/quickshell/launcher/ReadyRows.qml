pragma ComponentBehavior: Bound
// One place's rows as its daemon serves them ready (ready-rows.md, the ready-row form): held
// over one socket kept open, each record as the crate's Kept wraps it, `{ key, kind: "row", row
// }`, placed by its `order` as a change names it and never sorted, and made into what the place
// draws once a change to it, never once an ask (Joel, 2026-11-10: "the daemons need to be the
// data layer, and they need to serve the launcher"). What reads the clock, a time at a row's
// side or in its drawer, is worded here (ReadyRows.js), and only the rows that say one are made
// again as the minute turns, and only while they're drawn. Records of other kinds the place
// watches beside its rows, a summary, a head or a drawer served apart, are held by key and never
// placed. A row is made as every place's are by `thing`, its served acts its verbs, which ask
// back on the same socket, copy, open a link or step within the launcher, so a place moving
// onto it keeps no act of its own.
import QtQuick
import Quickshell
import qs
import "Found.js" as Found
import "ReadyRows.js" as Ready
import "Moved.js" as Moved
import "Beneath.js" as Beneath
import "../Fuzzy.js" as Fuzzy

Scope {
    id: ready

    // The daemon's socket, and the place its rows are watched for.
    required property string socket
    required property string place
    // The kinds of record watched beside the rows, as `summary`.
    property var beside: []
    // What else narrows the watch, as the daemon reads it.
    property var watching: ({})
    // The keys of rows the place is about to draw more of, its selected row's, so the daemon
    // reads their drawers and inside rows only while they're ahead (ready-rows.md's Notes).
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
    // Whether what it held stays through a gap in the daemon, as Daemon.qml's: a place whose
    // records may not be Joel's to see once it answers again, as Google's, lets them go.
    property bool keptThroughGap: true
    // Whether what it makes is drawn now, so a row saying a time is said
    // again as the minute turns: the place's launcher open.
    property bool drawn: false
    // What the place makes of a row, given the row the record holds and the
    // record's key; by default the row as served.
    property var make: (row, key) => row
    // A place's own make of an act, `(key, act, row)` to its row, else null: Agents' Talk
    // asks for words before its ask is sent, which no served act form says.
    property var actMade: null
    // The place's provider, which makes its rows as every place's are (`thing`) and holds what
    // an act asks drawn until it's done. Not named `provider`, which would hide the place's own
    // id of that name inside its ReadyRows.
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
    // Each row as it was served when it was last made, by key, so one told
    // again with only its order moved keeps what was made (Moved.js).
    readonly property var madeFrom: ({})
    // The acts served in a form the launcher refuses, by key, each
    // journalled once (ReadyRows.js's misformed).
    readonly property var misformed: ({})
    // Which row names each drawer served apart, by the drawer's key, so
    // the row is made again once its drawer changes.
    readonly property var drawers: ({})
    // The act keys each row has served more than once, by the row's key, so they're told apart
    // for as long as it's held (ReadyRows.js's actKeys).
    readonly property var repeats: ({})
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
            keptThroughGap: ready.keptThroughGap
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
        for (const store of [made, madeOf, madeFrom, drawers, ticking, clocked, repeats, lending,
            typedHeld]) {
            for (const key of Object.keys(store))
                delete store[key];
        }
        timed = 0;
        eachMinute = 0;
        keys = [];
        placings += 1;
        ended();
        typedAgain();
    }

    // Each key a change names let go of what was made of it, and the rows placed again; a row
    // told with only its order moved keeps what was made of it and only moves. Its lists come
    // through the signal converted, so they're copied by index.
    function took(changed, gone) {
        const said = [];
        const dropped = [];
        const moved = {};
        for (let at = 0; at < changed.length; at += 1) {
            const key = changed[at];
            if (key in made && Moved.movedOnly(madeFrom[key], served(key))) {
                moved[key] = true;
                madeFrom[key] = served(key);
            }
            said.push(key);
        }
        for (let at = 0; at < gone.length; at += 1)
            dropped.push(gone[at]);
        for (const key of dropped)
            delete repeats[key];
        for (const key of said.concat(dropped)) {
            if (key in drawers)
                delete made[drawers[key]];
            if (moved[key] === true)
                continue;
            delete made[key];
            delete madeFrom[key];
            delete lending[key];
            delete clocked[key];
            if (ticking[key] === "minute")
                eachMinute -= 1;
            if (key in ticking)
                timed -= 1;
            delete ticking[key];
        }
        for (const key of said) {
            if (moved[key] === true)
                continue;
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
        const before = keys;
        keys = Ready.placed(keys, held, said, dropped);
        placings += 1;
        // What was found was found among the rows before, so it's asked
        // again of these, unless none of them moved (Found.reranks).
        if (found.words !== null && Found.reranks(before, keys, said.concat(dropped)))
            ended();
        heard(said, dropped);
    }

    // The row the record of `key` holds, as served or typed for, or null.
    function served(key) {
        return Ready.rowOf(held[key] ?? typedHeld[key]);
    }

    // Which record each row made was made of, by the row's key, for its
    // stamp. A plain object, so writing it as a row's made notifies nothing.
    readonly property var madeOf: ({})

    // What a row made here and its parts are made of (ReadyRows.js's
    // stampOf), for a place's spotsStampOf: null for a row not the one
    // made of its record now, so a row left from before is never kept.
    function stampOf(row) {
        const key = madeOf[row?.key];
        return key !== undefined && made[key] === row ? Ready.stampOf(keys, held, key) : null;
    }

    // What the place made of the row of `key`, made now if not yet: the
    // row another place lends for it, where one does, else its own.
    function rowAt(key) {
        if (!(key in made)) {
            const row = served(key);
            const lent = row ? lentOf(row, key) : null;
            made[key] = lent ?? (row && row.lentOnly !== true ? ready.make(row, key) : null);
            if (made[key])
                madeOf[made[key].key] = key;
            madeFrom[key] = row;
            if (key in ticking)
                clocked[key] = clockSaid(row);
        }
        return made[key];
    }

    // A row served with `lends` is one thing another place makes the row of, as a meeting's
    // guest is People's person (ready-rows.md's lends): the first lender's row, so one thing has
    // one row, made by its own place; where none lends it the served row stands, or with
    // `lentOnly` none does. Asked once as the row is made, each lender answering from what it
    // holds, never by asking anyone.
    readonly property var lending: ({})
    function lentOf(row, key) {
        if (typeof row.lends !== "string" || row.lends === "")
            return null;
        lending[key] = true;
        for (const source of lenders) {
            const lent = source.lent(row.lends);
            if (lent)
                return lent;
        }
        return null;
    }

    // The launcher's places that lend rows, and what each says of what it
    // lends, so a lender arriving, or what one lends changing, makes again
    // only the rows lent, the rest kept as they were made.
    readonly property var lenders: (maker?.launcher?.sources ?? [])
        .filter(source => source && source !== maker && typeof source.lent === "function")
    readonly property string lentSaid: lenders.map(source => source.lentRevision ?? 0).join(",")
    onLentSaidChanged: {
        const lent = Object.keys(lending);
        if (lent.length === 0)
            return;
        for (const key of lent) {
            delete lending[key];
            delete made[key];
            delete madeFrom[key];
        }
        placings += 1;
    }

    // What each row saying a time said of the clock as it was made, by key,
    // so a turn of the clock makes again only the rows whose words it
    // moved: a document opened two days ago reads the same all day, and
    // every opening made all 27 of Joel's again (2026-11-10).
    readonly property var clocked: ({})
    function clockSaid(row) {
        return `${sideOf(row)}\u0000${noteOf(row)}`;
    }

    // Every row of the place's own as the place made it, in the daemon's
    // order, or with `parent` the key of a row, the rows inside it.
    function rows(parent) {
        void placings;
        const typed = parent in typedHeld && !(parent in held)
            ? Ready.inside(Object.keys(typedHeld), typedHeld, parent) : [];
        return Ready.inside(keys, held, parent ?? "").concat(typed).map(key => rowAt(key))
            .filter(row => row !== null);
    }

    // The rows inside the row of `key`, as `rows` gives them.
    function inside(key) {
        return rows(key);
    }

    // What the daemon last found for what was typed, as it ranks it (ready-rows.md's search):
    // the words it answered, the keys of the rows it found, best first, and the letters of each
    // one's name to bring forward. Each key's words are asked at once, never waiting on the
    // answer before, and only an answer to what's typed now is taken, so a key costs no round
    // trip behind another. Its words are null until an answer comes, and again once the rows
    // change. The rows it found that the watch never served come with it, each made as the
    // place makes a row (`foundMade`) and let go with the answer.
    property var found: Found.foundOf(null, null)
    readonly property var foundMade: ({})
    property string asking: ""
    // Whether the daemon said it can't search, so the place matches its
    // rows itself, as it did before, said once in the journal.
    property bool searchless: false
    // Said once an answer to `words` is held, for the place to show it.
    signal searched(string words)
    // The keys found before their rows were served, each said once.
    readonly property var unserved: ({})

    // The daemon's answer to `words` if it's held, else null, asking it,
    // for at most `most` rows where the place shows no more.
    function search(words, most) {
        if (found.words === words)
            return found;
        if (asking !== words && !searchless && link) {
            asking = words;
            const ask = { ask: "search", place: ready.place, words: words };
            link.find(most > 0 ? Object.assign(ask, { most: most }) : ask,
                answer => ready.answered(words, answer));
        }
        return null;
    }

    // The search over, as the launcher closing ends it, so what it found
    // is let go.
    function ended() {
        found = Found.foundOf(null, null);
        for (const key of Object.keys(foundMade))
            delete foundMade[key];
    }

    // An answer taken, unless it's to words typed over since. One that
    // didn't come, the daemon gone, is asked again with the next key.
    function answered(words, answer) {
        if (asking === words)
            asking = "";
        if (answer === null)
            return;
        if (answer.error) {
            if (!searchless)
                console.warn(`launcher: ${place}'s daemon can't search, so the launcher `
                    + `matches its rows itself: ${JSON.stringify(answer.error)}`);
            searchless = true;
            searched(words);
            return;
        }
        ended();
        found = Found.foundOf(words, answer);
        searched(words);
    }

    // What the daemon made of words typed inside a step (ready-rows.md's within), read by
    // `served` and `rows` beside the watch's, its keys steady so no row moves under the cursor.
    property var typing: ({ within: "", words: null, keys: [], refused: false })
    readonly property var typedHeld: ({})
    property string typedAsking: ""
    property double typedAt: 0

    // The rows made of `words` typed inside `within`'s row, or null where it
    // can't answer; its last answer there stays till the next (`searched`).
    function typedIn(within, words) {
        const asked = `${within}\n${words}`;
        if (typedAsking !== asked && !(typing.within === within && typing.words === words)) {
            if (!link)
                return null;
            typedAsking = asked;
            typedAt = Date.now();
            link.find({ ask: "search", place: ready.place, words: words, within: within },
                answer => ready.typedAnswered(within, words, answer));
        }
        if (typing.within !== within)
            return [];
        return typing.refused ? null
            : typing.keys.map(key => rowAt(key)).filter(row => row !== null);
    }

    // An answer taken, and how long it took, unless typed over since.
    function typedAnswered(within, words, answer) {
        if (typedAsking !== `${within}\n${words}`)
            return;
        typedAsking = "";
        console.info(`launcher: ${place} answered "${words}" typed in ${within} `
            + `${Date.now() - typedAt} ms after its key`);
        const refused = answer === null || !!answer.error;
        if (refused && !typing.refused)
            console.warn(`launcher: ${place}'s daemon didn't answer "${words}" typed in `
                + `${within}: ${JSON.stringify(answer?.error ?? null)}`);
        const records = refused ? {} : answer.rows ?? {};
        for (const key of Object.keys(typedHeld)) {
            if (!(key in records) && !(key in held))
                delete made[key];
            delete typedHeld[key];
        }
        Object.assign(typedHeld, records);
        typing = { within, words, keys: refused ? [] : answer.keys ?? [], refused };
        searched(words);
    }

    // What's typed asked of the daemon again.
    function typedAgain() {
        typing = { within: "", words: null, keys: [], refused: typing.refused };
        typedAsking = "";
    }

    // The rows an answer found, in its order, each as the place made it.
    // A key whose row is served by neither the watch nor the answer is
    // passed over, said once.
    function foundRows(answer) {
        const out = [];
        for (const key of answer.keys) {
            const record = Found.foundRecord(answer, held, key);
            let row = null;
            if (record !== null && record === held[key]) {
                row = rowAt(key);
            } else if (record !== null) {
                if (!(key in foundMade))
                    foundMade[key] = ready.make(Ready.rowOf(record), key);
                row = foundMade[key];
            }
            if (row !== null) {
                out.push(row);
            } else if (!(key in unserved)) {
                unserved[key] = true;
                console.warn(`launcher: ${place}'s search found ${key}, which isn't served yet`);
            }
        }
        return out;
    }

    // The words a time is said in, the launcher's own for each form (words.json's field.times),
    // with the calendar's names of days and months and, for a time somewhere else, `zoneSaid`
    // where the place gives one, else the zone's name.
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

    // A row's side as its items, each drawn as it is, its times said as of
    // now.
    function itemsOf(row) {
        return Ready.sideItems(row?.side, Date.now(), timeWords);
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

    // A drawer naming its row's acts by their words, Return's by its mark, never making one a row
    // for it: arrows between Agents' sessions made theirs (keeps-up, 2026-11-12).
    function actsNamed(drawer, acts, first) {
        const named = (one, at) => at === 0 && first ? first.name : one.name || Words.name(one.key);
        return !drawer || acts.length === 0 ? drawer : Object.assign({}, drawer,
            { acts: acts.map((one, at) => ({ name: named(one, at), key: at ? "" : "\u21b5" })) });
    }

    // A served row, the record of `key`'s, as a place draws one, made by its provider: its
    // words, its drawer read as it's shown, and its acts as its verbs, Return doing the first.
    // A row whose acts step into rows steps into those, Return's `into` first. `fields` are the
    // place's own, over these, so a place's `make` is `(row, key) => ready.thing(row, key,
    // {...})`; the row is given rather than looked up, as an act's `becomes` is made by it too.
    // It's a collection's member, its name made ready to match only once it's searched
    // (Provider.qml's member). Null without a provider or a row.
    function thing(row, key, fields) {
        if (!row || !maker)
            return null;
        const acts = actsOf(row);
        const first = acts[0] ?? null;
        const steps = acts.some(one => one.step === "rows");
        const own = one => typeof actMade === "function" ? actMade(key, one, row) : null;
        const firstOwn = first ? own(first) : null;
        // Its acts as rows, made once it's first asked for them and kept
        // with the row, so an opening asking the cursor's row for its keys
        // again makes none; a row made again makes its acts again.
        let acted = null;
        const verbs = () => {
            if (steps)
                return Beneath.led(inside(key), first?.into ? rowAt(first.into) : null);
            if (acted === null) {
                repeats[key] = repeats[key] ?? {};
                const kept = Ready.actKeys(acts, repeats[key]);
                acted = acts.map((one, at) => (at === 0 ? firstOwn : own(one))
                    ?? actRow(key, one, kept[at]));
            }
            return acted;
        };
        // How a Return leaves the launcher is ReadyRows.js's leaving's. A row whose Return asks
        // is that act's for the key language, so its command is read from the act's words
        // (CommandBinding.wordsFor), as Firmware's Install Now ends something and writes
        // `install!` on the line rather than running at the first Return (RCTRF8, 2026-11-11),
        // unless it shuts the launcher (ReadyRows.js's wordsOf).
        const words = Ready.wordsOf(first, row.kind);
        const asking = words === "" ? {} : { wordsId: words };
        // One holding only what the keys reach opens no list of its acts:
        // Return does the first, and its acts are still y's and d's.
        const acting = acts.length === 0 ? {} : Object.assign({
            verb: verbOf(first),
            run: () => act(key, first),
            actsOnly: Beneath.actsOnly(acts, row.actsOnly)
        }, Ready.leaving(first), firstOwn ? { returns: firstOwn } : {},
            // A row that is an act is asked as the place makes it, Close saying what stays.
            row.kind === "act" && firstOwn
                ? { askedAs: firstOwn.askedAs, leaves: firstOwn.leaves } : {});
        // A row asking Joel to be sure, or that is an act (an orchestration's Stop), holds its
        // act alone, as the shell's own do, so the key language reads that act's command
        // (Catalogue.endsSomething) rather than a row of verbs; an act stepping into rows, as a
        // meeting's Show in Calendar or Cancel the daemon serves, holds those (RTOIMQ, 2026-11-12).
        if (acts.length > 0 && row.sure !== true && (row.kind !== "act" || steps))
            Object.assign(acting, { verbs: verbs, targets: verbs });
        // A row whose Return copies is a value, drawn as Provider.qml's
        // copyRow draws one, and what it copies is what y joins.
        const copying = Ready.actForm(first) !== "copy" ? {} : {
            figure: true,
            plain: true,
            now: true,
            copies: () => first.copy
        };
        const items = itemsOf(row);
        return maker.member(Object.assign({
            key: row.thing ?? key,
            thing: !!row.drawer,
            name: row.name ?? "",
            // Made its characters once, here (ReadyRows.js's glyphOf).
            glyph: Ready.glyphOf(row.glyph ?? "", Theme.glyph),
            hue: row.hue ?? "",
            side: Ready.sideJoined(items),
            // Each item, so the row keeps an item's width and draws its hue.
            sideItems: items,
            note: noteOf(row),
            // Its bar, strip or spark, { shape, values, warn }, as served.
            graph: row.graph ?? null,
            reads: row.reads ?? "",
            state: Ready.stateOf(row),
            unfinished: row.unfinished === true,
            // A row a step in that asks Joel to be sure, so its Return does
            // its act (Launcher.lands), as an act that can't be taken back is.
            sure: row.sure === true,
            // An act done with, drawn dimmed, its acts none, so Return asks
            // nothing (RCTRF8's Firmware, 2026-11-11).
            gone: row.gone === true,
            // Only a row serving a drawer holds one, so a row without, as
            // Power's Charger is, isn't taken for a thing missing its say
            // (goes-places' undrawn, KP0L6A's 439b18d9).
            drawer: row.drawer ? () => actsNamed(drawerOf(row), acting.verbs && !steps ? acts : [],
                firstOwn) : undefined
        }, acting, asking, copying, fields ?? {}, {
            // The words it's found by beyond its name, the daemon's and the
            // place's, made ready to match as the search reads them
            // (Launcher.qml's hiddenWords), once for each text.
            keywords: keywordsMade(Ready.keywordsOf(row.keywords, fields?.keywords))
        }));
    }

    function keywordsMade(words) {
        return words === "" ? null : Fuzzy.kept(words);
    }

    // The acts of a row the launcher can do here: a step into the drawer only where the launcher
    // steps into one, so nothing offered does nothing. Going to a program is always offered,
    // since the daemon serves it only where it found the program's entry. An open in a program
    // keeps that program's name as Programs shows it (`program`), read while its entry's here,
    // so one gone by Return is named in saying it opened nothing.
    function actsOf(row) {
        const launcher = maker?.launcher;
        for (const one of row?.acts ?? []) {
            const why = Ready.misformed(one);
            if (why !== "" && !(one.key in misformed)) {
                misformed[one.key] = true;
                console.info(`launcher: ${one.key} refused: ${why}`);
            }
        }
        const named = {};
        const here = id => {
            const entry = DesktopEntries.byId(id);
            if (entry)
                named[id] = entry.name;
            return entry !== null;
        };
        return Ready.actsOf(row, here)
            .filter(one => one.step !== "drawer" || typeof launcher?.intoDrawer === "function")
            .map(one => Ready.openedIn(one) in named
                ? Object.assign({ program: named[Ready.openedIn(one)] }, one) : one);
    }

    // What a program a link opens in runs, by its desktop entry's id, as
    // the launcher runs it (Provider.qml's command), or null where it's gone.
    function commandOf(id) {
        const entry = DesktopEntries.byId(id);
        return entry && maker ? maker.command(entry) : null;
    }

    // The verb an act says, words.json's for its key.
    function verbOf(one) {
        return Ready.verbOf(one, Words.verb(one.key));
    }

    // An act as a row of its own, among a row's verbs, kept by `kept`
    // (ReadyRows.js's actKeys), its words by its own key.
    function actRow(key, one, kept) {
        const said = Ready.actWorded(one, { name: Words.name(one.key),
            glyph: Words.glyph(one.key), actKey: Words.actKey(one.key) }, Theme.glyph);
        // `said` gives its name, glyph and side, and the key normal does it
        // by on its row, as Asks' r refuses (Launcher.qml's doKeyed).
        return maker.thing(Object.assign({
            key: kept ?? one.key,
            wordsId: one.key,
            what: "",
            verb: verbOf(one),
            hue: "action",
            stays: one.step === "rows",
            now: Ready.leaving(one).now,
            later: Ready.leaving(one).later,
            copies: typeof one.copy === "string" ? () => one.copy : undefined,
            run: () => act(key, one)
        }, said));
    }

    // Does an act the row of `key` serves: copies its words, opens its link
    // through niri, steps within the launcher, or asks it of the daemon.
    function act(key, one) {
        const launcher = maker?.launcher;
        switch (Ready.actForm(one)) {
        case "copy":
            if (one.copy !== "")
                Quickshell.clipboardText = one.copy;
            alsoAsked(one);
            return;
        case "open": {
            const opened = Ready.opened(one, id => commandOf(id), Words.field.becoming ?? {});
            if (opened.run === null) {
                notOpened(key, one, opened.why);
                return;
            }
            Niri.spawn(opened.run);
            alsoAsked(one);
            return;
        }
        case "go": {
            // The entry's looked up here, at Return, never as rows are
            // made, and only where no window of it is open.
            const where = Ready.going(one, Niri.byApp, Niri.byApp[one.go.toLowerCase()]
                ? null : DesktopEntries.byId(one.go));
            if (where?.window !== undefined)
                Niri.act({ FocusWindow: { id: where.window } });
            else if (where?.entry && maker)
                Niri.spawn(maker.command(where.entry));
            alsoAsked(one);
            return;
        }
        case "step":
            if (one.step === "rows")
                launcher?.enter(Beneath.entered(rowAt(key), one.into ? rowAt(one.into) : null));
            else
                launcher?.intoDrawer(rowAt(key));
            return;
        case "ask":
            if (Ready.closes(one))
                shut(key, one);
            else
                asked(key, one);
        }
    }

    // An open whose program went between its rows being listed and Return opens nothing and says
    // so as a refused ask does, on the next opening, since the launcher has gone by then; nothing
    // was done, so nothing else it asks is sent.
    function notOpened(key, one, why) {
        const row = served(key);
        const shown = row?.thing ?? key;
        console.info(`launcher: ${one.key} opened nothing, its ${Ready.openedIn(one)} gone`);
        maker?.becomes(shown, rowAt(key), row?.name ?? "", verbOf(one), { done: () => false });
        maker?.refused(shown, why);
    }

    // Sends what an act the shell did asks besides, once it's done, as a
    // notice gone to is read (ReadyRows.js's alsoOf). Nothing waits on it:
    // its row moves as the daemon tells the change, so a refusal goes to
    // the journal alone, the row as it was saying it on screen.
    function alsoAsked(one) {
        const also = Ready.alsoOf(one);
        if (also === null)
            return;
        link.ask(also, answer => {
            const why = Ready.refusal(answer, Words.field.becoming ?? {});
            if (why !== "")
                console.info(`launcher: ${one.key}'s also refused: ${why}`);
        });
    }

    // Asks an act that shuts the launcher, as a paste does, which holds
    // nothing drawn, since nothing is drawn by the time it's answered. A
    // refusal is journalled at once and told as Joel next opens the
    // launcher, in the daemon's words, so nothing fails silently.
    function shut(key, one) {
        const name = served(key)?.name ?? "";
        const refused = why => {
            console.info(`launcher: ${one.key} refused: ${why}`);
            maker?.launcher?.toldNot(name, [one.ask.act, verbOf(one)], why);
        };
        const words = Words.field.becoming ?? {};
        if (!link) {
            refused(Ready.refusal(null, words));
            return;
        }
        link.ask(one.ask, answer => {
            const why = Ready.refusal(answer, words);
            if (why !== "")
                refused(why);
        });
    }

    // Asks an act of the daemon on the rows' own connection, its row drawn at once as the act
    // says it becomes, made as the place makes its rows, until the daemon serves it so. An act
    // that says nothing of how its row reads holds the row as it is until it's answered. Either
    // is taken back with the daemon's words if it's refused, or the launcher's if it isn't
    // answered, so nothing fails silently (launcher/Becoming.js).
    function asked(key, one) {
        const row = served(key);
        if (!row || !link)
            return;
        // What it becomes may be another row, as a charge limit's stop
        // becomes the limit's own row, so that row is the one held.
        const into = one.becomes ?? null;
        const becomesKey = into?.key ?? key;
        const shown = into ? (into.thing ?? becomesKey) : (row.thing ?? key);
        let answered = false;
        const becomes = into ? ready.make(into, becomesKey) : rowAt(key);
        const how = one.becomes ? undefined : { done: () => answered };
        maker?.becomes(shown, becomes, row.name ?? "", [one.ask.act, verbOf(one)], how);
        const words = Words.field.becoming ?? {};
        const send = ask => link.ask(ask, answer => {
            const why = Ready.refusal(answer, words);
            if (why === "")
                answered = true;
            else
                maker?.refused(shown, why);
        });
        if (one.token !== true) {
            send(one.ask);
            return;
        }
        // Asked as the act is done, never ahead, since niri gives a token
        // only for the input just made; without one it isn't sent, as its
        // sender couldn't come forward, and says so.
        Quickshell.activationToken(token => {
            const ask = Ready.tokened(one, token);
            if (ask === null)
                maker?.refused(shown, words.couldNot ?? "");
            else
                send(ask);
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
        let moved = false;
        for (const key of Object.keys(ticking)) {
            if (!(key in made) || (!daily && ticking[key] !== "minute"))
                continue;
            if (key in clocked && clocked[key] === clockSaid(served(key)))
                continue;
            delete made[key];
            moved = true;
        }
        if (moved)
            placings += 1;
    }
}
