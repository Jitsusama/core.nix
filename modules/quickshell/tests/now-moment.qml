// Drives Now's own rows (modules/quickshell/launcher/Now.qml) through
// Quickshell itself, with the margin's pieces, the agents' waiting list,
// the launcher, the weather and the calendar stood in, and Location.qml's
// own, reading whereabouts itself over its socket, with no GeoClue to ask
// and a link for the clock's zone (the check says how). After
// what's on come the time, the date, the location, the weather and the
// next meeting, with no Today to step into. The time, the date and the
// weather are each the row the launcher places for that thing (its
// `resolved`), never one Now draws itself; the location says its town,
// and where the place keeps another zone than the clock, says so; the
// weather is read once the launcher opens and again when it hears from
// Open-Meteo, keeping what it knew while it asks again; the meeting is the
// calendar's own row; and Now counts nothing while only they are there.
// For You, asked again what it holds, makes none of its rows again: its
// looks, its to-dos, the sessions wanting Joel and its bands' own rows,
// only one whose file or what it says changed once one does; and what's
// on, made again as any one piece moves, makes only that piece's rows
// again. Its Lately rows are kept too, so one notice arriving makes only
// Desktop's row again, History stepped into again hands back the rows it
// held, one read making only its row again, and a session's move that
// changes nothing For You
// reads of it, its want the same, makes none; For You held still hears a
// change its own row doesn't count, once the change is done. For You is
// banded once for each change, asked again it bands nothing, and the
// files an opening reads are taken together, one read as it was not at all.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
// History lists the looks done with as the to-do daemon serves them,
// making none itself, among the notices by when each was done, and has
// its rows made while the launcher is shut, so a step finds them made.
// Its done rows are kept while each reads the same, made again or not.
// Long because its cases run in turn on one Now, its stand-ins and its
// stand-in notices and to-do daemons, which each file split from it would
// build again.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // The weather provider as Now reads it: an answer to its first word,
    // and `answered` when a forecast arrives.
    QtObject {
        id: weather

        signal answered
        readonly property var settings: ({ name: "weather" })
        property string sky: "7\u00b0 \u00b7 cloudy"
        property bool asking: false
        property int asked: 0

        function answer(typed) {
            asked += 1;
            if (typed !== "weather")
                return null;
            if (asking)
                return { headline: "\u2026", rows: [], note: "asking Open-Meteo", asking: true };
            return {
                headline: sky,
                rows: [{ name: "Conditions", side: sky, glyph: "sky" }],
                note: "Kelpwater \u00b7 Open-Meteo"
            };
        }
    }

    // The calendar provider's next meeting, as its row gives it.
    QtObject {
        id: calendar

        readonly property var settings: ({ name: "calendar" })
        property bool joined: false
        property var next: ({
            key: "calendar:meeting:standup",
            name: "Standup",
            side: "in 12 min",
            verb: "join",
            later: true,
            run: () => calendar.joined = true
        })
    }

    // A place that keeps its rows as a provider's things, as Cameras does,
    // whose row's drawer a piece of Now's borrows (Piece.qml's drawnBy).
    QtObject {
        id: cameras

        readonly property var settings: ({ name: "cameras" })
        readonly property var things: [{
            key: "camera:front",
            drawer: () => ({ eyebrow: "built in \u00b7 camera", facts: [] })
        }]
    }

    // A place that lists every row it answers with, each holding its drawer
    // made, as Power does.
    QtObject {
        id: power

        readonly property var settings: ({ name: "power" })
        readonly property var all: [{
            key: "power:energy",
            drawer: { eyebrow: "held \u00b7 battery", facts: [] }
        }]
    }

    // Agents, as For You reads it: each session wanting Joel as Agents'
    // row for it, kept as Agents keeps its own, so a session's row For
    // You makes is made again only when For You's own making is; what each
    // asks as the agents daemon serves it (wants), and each answer asked
    // of it kept as it was sent, said put or not as the case sets (puts).
    QtObject {
        id: agents

        readonly property var settings: ({ name: "agents" })
        readonly property var toReopen: null
        readonly property var rows: ({})
        property var wants: ({})
        property var answered: []
        property bool puts: true
        // What it gives For You, as a source's `forYou` (ForYou.js's given).
        property var forYou: []

        function wantOf(address) {
            return wants[address] ?? null;
        }

        function answerWant(address, fields, then) {
            answered = answered.concat([{ address: address, fields: fields }]);
            then(puts);
        }

        // Moved when a row's words move, as Agents' rows say they have.
        property int remade: 0

        function made(session) {
            void remade;
            if (!(session.address in rows))
                rows[session.address] = { key: `agent:${session.address}`,
                    name: session.title, side: "" };
            return rows[session.address];
        }

        // How a session stands, read from its row's record as Agents'
        // own reads it, without making a row.
        function standingOf(address) {
            void remade;
            const row = rows[address];
            return row ? { name: row.name, doing: row.doing ?? "" } : null;
        }

        // An act of Agents' own on a session, as its drawer offers it,
        // saying whom it goes to.
        function sessionAct(session, id) {
            return { key: `verb:agent:${id}`, name: id, to: session.address };
        }

        // Whether a session has a window, as a look's Go To asks: as the
        // case gives it one.
        function windowedOf(session) {
            return !!session?.window;
        }
    }

    // The launcher's placing of an answered thing, as Launcher.qml's
    // resolved gives it: named by the thing, what it says beside, its
    // parts a step in. Each row it gives is marked, so the test knows Now
    // showed the launcher's row and not one of its own.
    QtObject {
        id: theLauncher

        property bool open: false
        // Whether it's drawing its first frame, before `open` follows.
        property bool opening: false
        // Whether a source's rows are wanted (Provider.qml's wanted), as
        // they aren't while the launcher is shut or open elsewhere.
        property bool wanting: true
        // Nothing is held, so no step gathers the notices.
        property var held: []

        function wants(source) {
            return wanting;
        }
        // Nothing is drawn here, so what waits on a frame runs at once.
        function afterFrame(then) {
            then();
        }
        // What the foot was last told.
        property var said: []

        function say(keys) {
            said = keys;
        }
        // Set without one only by a case that asks what's left without it.
        property var sources: [weather, calendar, cameras, power, pastNotices, agents]
        property int placed: 0

        function resolved(id) {
            placed += 1;
            const now = new Date();
            const row = (name, side, parts) => ({
                key: "",
                is: `answer:${id}`,
                answered: true,
                placed: name,
                name: name,
                side: side,
                verb: "copy",
                fromLauncher: true,
                verbs: () => parts
            });
            if (id === "time")
                return row("Time", Qt.formatDateTime(now, "HH:mm"), [{ name: "Zone" }]);
            if (id === "today")
                return row("Date", Qt.formatDate(now, "dddd d MMMM yyyy"), [{ name: "Week" }]);
            if (id !== "weather")
                return null;
            const said = weather.answer("weather");
            return said && said.rows.length > 0 ? row("Weather", said.headline, said.rows) : null;
        }
    }

    Launcher.Now {
        id: now

        launcher: theLauncher
        settings: ({ name: "on" })
    }

    // The notices' own rows, which Now's Desktop and History hold.
    Launcher.PastNotices {
        id: pastNotices

        launcher: theLauncher
        settings: ({ name: "notices" })
    }

    // Run once whereabouts answered: here, the clock's zone and Location's rows.
    readonly property bool ready: Location.read && Location.clockZone !== ""
        && now.whereaboutsRows.keys.length > 0
    property bool started: false
    onReadyChanged: begin()

    // When the cases' looks came: an hour before they run, as a look a
    // session has just put up, since a fixed date grows three days old and
    // goes quiet (Looks.js's QUIET_AFTER).
    readonly property string lookedAt: new Date(Date.now() - 60 * 60 * 1000).toISOString()
    Component.onCompleted: Qt.callLater(root.begin)

    function begin() {
        if (!ready || started)
            return;
        started = true;
        Qt.callLater(root.cases);
    }

    // Now's own rows, after what's on.
    function moment() {
        return now.everything().filter(row => String(row.is ?? "").startsWith("answer:")
            || row.key === "on:location" || row.key === calendar.next?.key);
    }

    function names() {
        return moment().map(row => row.name);
    }

    function cases() {
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            Qt.callLater(() => Qt.exit(1));
            return;
        }
        Qt.callLater(root.afterwards);
    }

    // The cases that hold once what a change deferred has run, then once
    // the looks the to-do daemon's stream tells at once are taken,
    // together: each taken alone parsed every look and banded For You
    // afresh, 166 times in each of Joel's openings (2026-11-10).
    function afterwards() {
        if (tried(() => {
            heldHears();
            bandedAtReads = now.bandsKept.times;
            const told = {};
            for (const id of ["e", "f", "g"])
                told[`look:${id}`] = { kind: "look", look: { v: 1, id: id, kind: "page",
                    at: root.lookedAt, title: id, why: "Because.",
                    target: "https://example.com", state: "waiting", session: "" } };
            now.looksTold(told);
        }))
            Qt.callLater(root.readsTaken);
    }
    property int bandedAtReads: 0

    // The to-do daemon's records of looks whose texts are `texts`, by file,
    // as its stream tells them.
    function records(texts) {
        const told = {};
        for (const text of Object.values(texts)) {
            const look = JSON.parse(text);
            told[`look:${look.id}`] = { kind: "look", look: look };
        }
        return told;
    }

    function readsTaken() {
        const banded = () => now.bandsKept.times - bandedAtReads;
        if (tried(() => expect("and are taken together, For You banded once",
            JSON.stringify(now.looks.waiting.map(one => one.file))
                === JSON.stringify(["e.json", "f.json", "g.json"]) && banded() === 1
                && now.forYouBands.look.length === 3,
            JSON.stringify([Object.keys(now.lookTexts), banded()]))))
            noticesServed();
    }

    // Now's Desktop holds the notices unread as the notices daemon
    // serves their rows, and its History those done with, each made again
    // only once the daemon says its notice changed: every notice's was
    // made anew at any change, 488 on Joel's shell as he stepped into
    // Desktop (2026-11-10). Desktop's rows are watched while the launcher
    // is open, so it's open for these.
    // Each notice came over an hour ago, so its time says the clock's
    // hour and minute, words only the day's turning moves: one said in
    // minutes is said again as the minute turns, which a busy builder
    // let happen between a case's counts, so they read 4 for 1.
    function noticeOf(id, read) {
        const at = Date.now() - 3600000 - id * 60000;
        return { id: id, at: at, app: "Tester", summary: `Notice ${id}`,
            body: "said", read: read, missed: false, readAt: read ? at : null };
    }
    property var told: []
    property var madeBefore: []
    // What the case waits to have been served, or "" while it waits on
    // nothing.
    property string waiting: ""
    // How many times the notices daemon was asked for the rows past the
    // page once the launcher had closed, so History made ahead is seen to
    // ask for none while it's shut.
    property int rowsAskedShut: -1
    // When the launcher opened again, so the case waits a while, not a
    // count of ticks a slow builder stretches, for History to fill in.
    property real reopenedAt: 0
    function madeNow() {
        return [pastNotices.tally.rows, now.tally.rows];
    }
    function since(before) {
        return madeNow().map((count, at) => count - before[at]);
    }
    function stepIn() {
        const held = now.leadsTo("notices").slice(-1)[0].verbs();
        return { held: held, history: held[held.length - 1].verbs() };
    }
    // Whether History holds notice `id`, as it holds only the read.
    function inHistory(id) {
        return pastNotices.pastRows(["desktop", "slack"]).some(row => row.notice === id);
    }
    function noticesServed() {
        const told = [];
        for (let id = 1; id <= 30; id++)
            told.push(noticeOf(id, id > 3));
        root.told = told;
        theLauncher.open = true;
        Notices.history = told;
        noticesDaemon.unserved = [Object.assign(noticeOf(31, true),
            { summary: "Parcel delivered", body: "at the door" })];
        noticesDaemon.notices = told;
        waiting = "the notices served";
        heardNotices();
    }
    function heardNotices() {
        if (waiting === "the notices served" && pastNotices.desktopHolds(1)
            && inHistory(30))
            firstNotices();
        else if (waiting === "one read" && pastNotices.desktopRows([2]).length === 0
            && inHistory(2))
            oneRead();
    }
    // The daemon's answer to a search come, the words it answered.
    function heardFound() {
        if (waiting === "notice searched")
            noticeFound();
        else if (waiting === "notice 1 searched")
            noticeOneFound();
        else if (waiting === "parcel searched")
            parcelFound();
    }
    property var listed: []
    function firstNotices() {
        waiting = "";
        if (!tried(() => {
            const desktop = stepIn();
            expect("Now's Desktop holds the notices unread, then Mark Them Read and History",
                JSON.stringify(desktop.held.map(row => row.name)) === JSON.stringify([
                    "Notice 1", "Notice 2", "Notice 3", "Mark Them Read", "History"])
                    && desktop.history.length === 27,
                JSON.stringify([desktop.held.map(row => row.name), desktop.history.length]));
            const before = madeNow();
            const again = stepIn();
            expect("stepped into again, it makes none of the notices' rows again",
                since(before)[0] === 0, JSON.stringify(since(before)));
            // History hands back the rows it held, so the launcher shows
            // none anew: all 594 to 679 of Joel's were new at each step,
            // 81 to 152 ms to the frame (2026-11-13).
            const fresh = again.history.filter(row => !desktop.history.includes(row));
            expect("stepped into again, History hands back the rows it held",
                again.history.length === desktop.history.length && fresh.length === 0,
                `${fresh.length} of ${again.history.length} new`);
            const whole = now.historyRow.verbs();
            const wholeFresh = now.historyRow.verbs().filter(row => !whole.includes(row));
            expect("For You's History asked again hands back the rows it held",
                whole.length > 0 && wholeFresh.length === 0,
                `${wholeFresh.length} of ${whole.length} new`);
            // Each notice's row is the one the daemon served, muted and
            // For You's as it came (row.rs), with nothing made of it here.
            const served = new Set(pastNotices.pastRows(["desktop", "slack"]));
            const notices = whole.filter(row => row.notice !== undefined);
            const unlike = notices.filter(row => !served.has(row) || row.hue !== "muted"
                || row.forYou?.kind !== "notice" || row.forYou?.source !== row.pastSource);
            expect("For You's History lists each notice's row as the daemon served it",
                notices.length === 27 && unlike.length === 0,
                JSON.stringify([notices.length, unlike.map(row => [row.key, row.hue])]));
            // Desktop lists every notice's row until its own place tells,
            // a frame after the launcher opens, so Now says it changed
            // once it does, or a step into Desktop goes on saying
            // "unread" until something else changes (draws-still's dd).
            const toldBefore = root.nowChanged;
            pastNotices.changed();
            expect("Now says it changed as the notices' rows do",
                root.nowChanged === toldBefore + 1, root.nowChanged - toldBefore);
            const unanswered = pastNotices.search("notice").rows;
            expect("a search lists no notice until the daemon answers it",
                unanswered.length === 0, `${unanswered.length} listed`);
        }))
            return;
        waiting = "notice searched";
    }
    function noticeFound() {
        waiting = "";
        if (!tried(() => {
            listed = pastNotices.search("notice").rows;
            expect("a search lists the notices the daemon found, as it ordered them",
                JSON.stringify(listed.map(row => row.notice))
                    === JSON.stringify(told.map(one => one.id)),
                JSON.stringify(listed.map(row => row.notice)));
            pastNotices.search("notice 1");
        }))
            return;
        waiting = "notice 1 searched";
    }
    function noticeOneFound() {
        waiting = "";
        if (!tried(() => {
            const again = pastNotices.search("notice 1").rows;
            const copied = again.filter(row => !listed.includes(row));
            expect("a key typed on copies none of the notices it listed before",
                again.length > 0 && copied.length === 0,
                `${copied.length} of ${again.length} copied again`);
            pastNotices.search("notice");
            expect("the daemon is asked each search once, one asked again kept",
                JSON.stringify(noticesDaemon.searches) === JSON.stringify(["notice", "notice 1"]),
                JSON.stringify(noticesDaemon.searches));
        }))
            return;
        const read = told.map(one => one.id === 2
            ? Object.assign({}, one, { read: true, readAt: Date.now() }) : one);
        // Stepped into once as things stand, since Desktop's own rows may
        // have come while the searches were answered, so what's counted is
        // what the one read makes again, not rows Desktop made first.
        // Two looks done with, in For You's History beside the notices.
        looksBefore = now.lookTexts;
        const doneLook = id => JSON.stringify({ v: 1, id: id, kind: "page",
            at: root.lookedAt, resolvedAt: root.lookedAt, target: "https://example.com",
            title: `Looked at ${id}`, why: "Because.", state: "opened" });
        now.lookTexts = { "d.json": doneLook("d"), "e.json": doneLook("e") };
        weighedBefore = now.looksWeighed.times;
        now.historyRows();
        historyBefore = stepIn().history;
        steppedBefore = heldMade();
        madeBefore = madeNow();
        Notices.history = read;
        noticesDaemon.notices = read;
        waiting = "one read";
        heardNotices();
    }
    // What stepping in held before the read, to name the rows a read made
    // again where it fails: the case failed 3 of 18 gates under load
    // saying only how many (Y2YBTS, 2026-11-12).
    property var steppedBefore: null
    // History's rows as stepped into before the read.
    property var historyBefore: []
    // The looks as they were before the case's own, and how many times a
    // look's side had been worked out as History held them.
    property var looksBefore: ({})
    property int weighedBefore: 0
    // Every notice's made row, by notice, as search holds them (Desktop's
    // are copied as they're listed, so can't be told apart this way).
    // Taken before a count and after its read, so making none adds to it.
    function heldMade() {
        const held = { every: {} };
        for (const row of pastNotices.all())
            held.every[row.notice] = row;
        held.history = {};
        for (const row of pastNotices.pastRows(["desktop", "slack"]))
            held.history[row.notice] = row;
        return held;
    }
    function remadeSince(before, after) {
        const out = [];
        for (const place of Object.keys(after)) {
            const was = before[place] ?? {};
            for (const key of Object.keys(after[place])) {
                if (was[key] !== after[place][key])
                    out.push(place + " " + key + (key in was ? "" : " (new)"));
            }
            for (const key of Object.keys(was)) {
                if (!(key in after[place]))
                    out.push(place + " " + key + " (gone)");
            }
        }
        return out;
    }
    function oneRead() {
        waiting = "";
        if (tried(() => {
            const stepped = stepIn();
            const made = since(madeBefore);
            const after = heldMade();
            // Its row in the mixed place reads otherwise, and History,
            // served as a place of its own, gains it: two, and none more.
            expect("one notice read, stepping in again makes only its rows again",
                made[0] === 2,
                `${JSON.stringify(made)}, made again: `
                    + JSON.stringify(remadeSince(steppedBefore, after)));
            const fresh = stepped.history.filter(row => !historyBefore.includes(row));
            expect("one notice read, History makes only its row again",
                fresh.length === 1 && fresh[0].notice === 2,
                JSON.stringify(fresh.map(row => row.key)));
            // For You's History holds the looks done with too, as the
            // to-do daemon serves their rows, so the shell works out none
            // of their sides, stepping in first or again: Joel's 470 made
            // here took 104 to 226 ms of a fresh shell's first step
            // (2026-11-14).
            const history = now.historyRows();
            const held = history.filter(row => row.key.startsWith("on:look:"));
            expect("For You's History lists the looks as served, making none itself",
                held.length === 2 && now.looksWeighed.times === weighedBefore,
                `${held.length} looks, ${now.looksWeighed.times - weighedBefore} sides made`);
            // Its Return opens the look as the shell holds it, so one gone
            // quiet is written opened, though the daemon names no file.
            expect("a served look opens as the shell holds it",
                now.lookHeld({ id: "d" }).file === "d.json",
                JSON.stringify(now.lookHeld({ id: "d" })));
            const dated = history.filter(row => typeof row.doneAt === "number");
            expect("For You's History joins the looks and notices by when each was done",
                dated.every((row, at) => at === 0 || dated[at - 1].doneAt >= row.doneAt),
                JSON.stringify(dated.map(row => [row.key, row.doneAt])));
            now.lookTexts = looksBefore;
            expect("a search answered before the notices changed waits for its answer again",
                pastNotices.awaits("notice") === true, JSON.stringify(pastNotices.answers.kept));
            expect("a search answered before the notices changed lists none of them meanwhile",
                pastNotices.search("notice").rows.length === 0,
                JSON.stringify(pastNotices.search("notice").rows.map(row => row.key)));
            pastNotices.search("parcel");
        }))
            waiting = "parcel searched";
    }
    // A notice the place never served, as a page served leaves the rest,
    // is listed from the row its answer brings.
    function parcelFound() {
        waiting = "";
        if (tried(() => {
            const found = pastNotices.search("parcel").rows;
            expect("a search lists a notice the place never served, from its answer's row",
                JSON.stringify(found.map(row => [row.notice, row.name]))
                    === JSON.stringify([[31, "Parcel delivered"]]),
                JSON.stringify(found.map(row => [row.notice, row.name])));
            expect("and listed again, it isn't made again",
                pastNotices.search("parcel").rows[0] === found[0], "made again");
        }))
            historyPastPage();
    }
    // For You's History lists a notice past the place's page with the row
    // the daemon's `rows` answer brought, asked as History was first
    // stepped into and again as a read joined it, and let go as the
    // launcher closes.
    function historyPastPage() {
        const parcel = noticesDaemon.unserved[0];
        Notices.history = noticesDaemon.notices.concat([parcel]);
        if (tried(() => {
            const history = stepIn().history;
            const row = history.find(one => one.name === "Parcel delivered") ?? null;
            expect("History lists a notice past the page as its notice's row",
                row !== null && row.key === "notice:31" && row.notice === 31,
                JSON.stringify(history.map(one => [one.key, one.name])));
            // Once as History was first stepped into, and once more as the
            // notice read joined it, which what was kept didn't list.
            expect("the rows past the page were asked for once, and again for the read",
                noticesDaemon.rowsAsked === 2, `${noticesDaemon.rowsAsked} asks`);
        }))
            sendersCounted();
    }
    // Each sender's notices are counted past the page too, as a program's
    // own row counts its (Applications.qml), as the daemon counts them
    // beside its rows: counted from every row, Joel's 1,000 were made for
    // two fields each, 97 ms of one program's acts (2026-11-14).
    function sendersCounted() {
        if (tried(() => {
            const made = pastNotices.tally.rows;
            const asked = noticesDaemon.rowsAsked;
            const counts = pastNotices.sentCounts();
            const sent = noticesDaemon.notices.concat(noticesDaemon.unserved);
            const want = { count: sent.length, unread: sent.filter(one => !one.read).length };
            expect("each sender's notices are counted, those past the page too",
                JSON.stringify(counts.Tester) === JSON.stringify(want),
                `${JSON.stringify(counts)} for ${JSON.stringify(want)}`);
            expect("and counting them makes none of their rows",
                pastNotices.tally.rows === made, `${pastNotices.tally.rows - made} made`);
            expect("nor asks the daemon for the rows past the page",
                noticesDaemon.rowsAsked === asked, `${noticesDaemon.rowsAsked - asked} asks`);
            expect("asked again over the same notices, it gives the same count",
                pastNotices.sentCounts() === counts, "counted again");
            theLauncher.open = false;
            expect("and the rows past the page go as the launcher closes",
                pastNotices.pastWhole.kept === null,
                JSON.stringify(pastNotices.pastWhole.kept?.keys));
            root.rowsAskedShut = noticesDaemon.rowsAsked;
            expect("and nothing's on its way as it closes",
                root.asksNone(root.rowsAskedShut), root.asksSaid(root.rowsAskedShut));
            // A look done with while the launcher is shut, which the case
            // then waits to find among History's rows made, asking for none.
            looksDaemon.looks = looksDaemon.looks.concat([{ id: "f", kind: "page",
                title: "Looked at f", why: "Because.", how: "opened",
                doneAt: Date.parse(root.lookedAt) + 1000 }]);
        }))
            waiting = "made ahead";
    }
    // History's rows as last made, read without making any.
    function historyMade() {
        return (now.historyKept.rows[""]?.row ?? []).map(row => row.key);
    }
    // History's rows are made while the launcher is shut, once the to-do
    // daemon tells a change, so a fresh shell's first step finds them
    // made: made at the step, Joel's 564 took 80 ms of 137 (2026-11-14).
    function madeAhead() {
        waiting = "";
        if (tried(() => {
            const made = now.historyKept.rows[""].row;
            expect("a step into History finds its rows made while the launcher was shut",
                now.historyRows() === made, "made again at the step");
            // Each answer come while it's shut is let go, so asking for one
            // made History's 1.7 MB of Joel's arrive for nothing, about once
            // a minute (the census, 2026-11-18).
            expect("and made shut, it asks for none of the rows past the page",
                root.asksNone(root.rowsAskedShut), root.asksSaid(root.rowsAskedShut));
            // A pick learns as the launcher closes, reading every
            // collection's keys, so its notices' are read without asking.
            // It's opened for this, as a pick is made while it's still
            // open: shut, nothing would ask whatever the keys were read by.
            theLauncher.open = true;
            const keys = pastNotices.keysKnown().map(one => one.key);
            expect("a pick's keys ask for none of the rows past the page",
                root.asksNone(root.rowsAskedShut), root.asksSaid(root.rowsAskedShut));
            const page = pastNotices.all().map(row => row.key);
            expect("and they're the page's keys, as everything() gave",
                keys.length > 0 && keys.join() === page.join(), keys.join());
            // Drawing its first frame, before `open` follows, it isn't shut:
            // History asks for the rows past its page, and keeps them.
            theLauncher.open = false;
            theLauncher.opening = true;
            pastNotices.pastRows(["desktop", "slack"]);
            expect("opening, History asks for the rows past its page",
                pastNotices.pastWhole.asking, root.asksSaid(root.rowsAskedShut));
        }))
            waiting = "opening";
    }
    // The answer to History's ask made as the launcher draws its first
    // frame, come before `open` follows, is kept rather than let go, and
    // stays once the frame is drawn, as the launcher fills in (filled):
    // `open` turns true, then `opening` false.
    Timer {
        running: root.waiting === "opening"
        interval: 50
        repeat: true
        onTriggered: {
            if (pastNotices.pastWhole.asking)
                return;
            root.waiting = "";
            if (root.tried(() => {
                root.expect("and its answer come while opening is kept",
                    pastNotices.pastWhole.kept !== null, "let go");
                theLauncher.open = true;
                theLauncher.opening = false;
                root.expect("and filled in, it keeps what came while opening",
                    pastNotices.pastWhole.kept !== null, "let go");
                // Shut, then opening again, History asks anew.
                theLauncher.open = false;
                theLauncher.opening = true;
                pastNotices.pastRows(["desktop", "slack"]);
                root.expect("opening again, History asks for the rows past its page",
                    pastNotices.pastWhole.asking, root.asksSaid(root.rowsAskedShut));
            }))
                root.waiting = "unfilled";
        }
    }
    // An opening shut again before its first frame, as a second press of
    // the chord does, lets go of what came while it opened.
    Timer {
        running: root.waiting === "unfilled"
        interval: 50
        repeat: true
        onTriggered: {
            if (pastNotices.pastWhole.asking)
                return;
            root.waiting = "";
            if (root.tried(() => {
                root.expect("and its answer come while opening again is kept",
                    pastNotices.pastWhole.kept !== null, "let go");
                // Shut again before its first frame, `open` never changed.
                theLauncher.opening = false;
                root.expect("shut before its first frame, it lets that answer go",
                    pastNotices.pastWhole.kept === null, "still held");
                // So opened again, its step asks anew for what it lists.
                root.reopenedAt = Date.now();
                theLauncher.open = true;
                root.stepIn();
            }))
                root.waiting = "reopened";
        }
    }
    // What was asked since `since`, for a failure to say which of
    // asksNone's three it was.
    function asksSaid(since) {
        return `${noticesDaemon.rowsAsked - since} asks, asking ${pastNotices.whole.asking}`
            + ` and ${pastNotices.pastWhole.asking} past`;
    }
    // Whether the notices daemon was asked for no rows past the page since
    // `since`, neither answered nor on their way, so the count read on the
    // daemon's side can't pass for an ask it hasn't read yet.
    function asksNone(since) {
        return noticesDaemon.rowsAsked === since && !pastNotices.whole.asking
            && !pastNotices.pastWhole.asking;
    }
    // Opened again, a step into History asks for the rows past its page and
    // lists them, as it did before the launcher shut.
    Timer {
        running: root.waiting === "reopened"
        interval: 50
        repeat: true
        onTriggered: {
            // Tried, so a step that throws says so at once rather than
            // waiting out the watchdog.
            root.tried(() => {
                const history = root.stepIn().history;
                if (history.some(one => one.name === "Parcel delivered")) {
                    root.waiting = "";
                    root.cellNoticeServed();
                } else if (Date.now() - root.reopenedAt > 15000) {
                    root.waiting = "";
                    root.expect("opened again, History lists past its page",
                        false, JSON.stringify(history.map(one => one.name)));
                }
            });
        }
    }
    // A session's own notice stands under it among what needs Joel now as
    // the row Desktop's place serves for it (row.rs's placed), so it's
    // shown as well as counted: counted alone, the tray and For You said
    // "1 to do" over a row that wasn't there.
    function cellNoticeServed() {
        Sessions.running = [{ address: "KELP01", title: "Session K",
            presence: { session: "s-k", addresses: ["KELP01", "9F3A2B1C"] } }];
        const notice = { id: 40, at: Date.now(), app: "pi", summary: "Session K is done",
            body: "said", read: false, urgency: "critical",
            cell: "mycelium:cell/9F3A2B1C@heron#notice:40" };
        // What the tray counted before, as the looks still waiting.
        trayBefore = ForYouHolds.count;
        Notices.history = Notices.history.concat([notice]);
        noticesDaemon.notices = noticesDaemon.notices.concat([notice]);
        waiting = "cell notice served";
    }
    property int trayBefore: 0
    function cellNoticeRow() {
        waiting = "";
        const noticed = () => now.forYouBands.now.filter(one => one.kind === "notice");
        const tray = () => ForYouHolds.count - trayBefore;
        const counts = () => JSON.stringify([noticed().length, tray()]);
        if (tried(() => {
            const keys = now.waitingRows().map(one => one.key);
            const row = now.waitingRows().find(one => one.key === "notice:40") ?? null;
            expect("a session's notice served stands under it among what needs Joel as its row",
                row !== null && tray() === 1 && keys.includes("on:heading:cell:KELP01")
                    && keys.indexOf("on:heading:cell:KELP01") === keys.indexOf("notice:40") - 1,
                `${counts()} ${JSON.stringify(keys)}`);
            expect("which :filter holds as a thing to do from the desktop",
                row.forYou?.kind === "do" && row.forYou?.source === "desktop",
                JSON.stringify(row.forYou ?? null));
            expect("and home's row names it", now.homeSays() === "Session K is done",
                now.homeSays());
            // Its session's want put off, as Sessions says it, the notice is
            // its want's to say, so it stands nowhere and counts nothing.
            Sessions.putOff = { KELP01: true };
            expect("its session's want put off, it stands nowhere and counts nothing",
                noticed().length === 0 && tray() === 0, counts());
            Sessions.putOff = {};
            // Acted on from its row, the daemon reads it and puts it away
            // (daemon.rs's invoke), and it stays where it stood while For
            // You is open, as one read does.
            const done = Date.now();
            const acted = one => one.id !== 40 ? one
                : Object.assign({}, one, { read: true, readAt: done, left: "dismissed",
                    leftAt: done });
            Notices.history = Notices.history.map(acted);
            noticesDaemon.notices = noticesDaemon.notices.map(acted);
            expect("acted on from its row, it stays where it stood, its row with it",
                noticed().length === 1 && tray() === 1
                    && now.waitingRows().some(one => one.key === "notice:40"), counts());
            // Put away unread, then withdrawn by its session while For You
            // is open, which reads it and leaves it dismissed (held.rs's
            // close), it goes at once, since Joel never read it (Lately.js's
            // readHow).
            Notices.history = Notices.history.map(one => one.id !== 40 ? one
                : Object.assign({}, one, { read: false, readAt: null }));
            Notices.history = Notices.history.map(one => one.id !== 40 ? one
                : Object.assign({}, one, { read: true, readAt: Date.now(), withdrawn: true }));
            expect("withdrawn by its session while For You is open, it goes at once",
                noticed().length === 0 && tray() === 0, counts());
            // Shut, it's gone, and the tray counts it no more.
            theLauncher.open = false;
            expect("the launcher shut, it's gone, and the tray counts it no more",
                noticed().length === 0 && tray() === 0, counts());
            // Raised again with the launcher shut, the tray counts it.
            Notices.history = Notices.history.map(one => one.id !== 40 ? one
                : Object.assign({}, one, { read: false, readAt: null, left: undefined,
                    leftAt: undefined, withdrawn: undefined }));
            expect("raised with the launcher shut, the tray counts it", tray() === 1, counts());
        }))
            Qt.exit(0);
    }
    Timer {
        running: root.waiting === "cell notice served"
        interval: 50
        repeat: true
        onTriggered: {
            if (pastNotices.desktopHolds(40))
                root.cellNoticeRow();
        }
    }
    Timer {
        running: root.waiting === "made ahead"
        interval: 50
        repeat: true
        onTriggered: {
            if (root.historyMade().includes("on:look:f"))
                root.madeAhead();
        }
    }

    // The notices daemon, serving the rows of the notices a case gives.
    NoticesDaemon {
        id: noticesDaemon
    }
    // The to-do daemon, serving History's rows of the looks done with,
    // those the History case's look files hold.
    LooksDaemon {
        id: looksDaemon

        looks: ["d", "e"].map(id => ({ id: id, kind: "page",
            title: `Looked at ${id}`, why: "Because.", how: "opened",
            doneAt: Date.parse(root.lookedAt) }))
    }
    Connections {
        target: pastNotices

        // Desktop's own rows come a frame after the launcher opens, and
        // until then its rows are taken from every notice: a case starting
        // on those had Desktop's own land over the read and make them
        // again, 4 rows for 1, in 3 of 18 gates on a loaded builder. Its
        // telling is a change, so the case waits on it here.
        function onChanged() {
            root.heardNotices();
        }

        function onFoundMore() {
            root.heardFound();
        }
    }
    Timer {
        running: root.waiting !== ""
        interval: 20000
        onTriggered: {
            console.warn(`FAIL never served: ${root.waiting}`);
            Qt.exit(1);
        }
    }

    // Whether the cases held, exiting 1 saying which didn't.
    function tried(cases) {
        try {
            cases();
            return true;
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            Qt.exit(1);
            return false;
        }
    }

    // How many times Now said its rows changed.
    property int nowChanged: 0
    // What Now said moved, the last time it said so (Provider.qml's
    // changedWhy).
    property string nowMoved: ""
    // What it said moved each time, in order, since the case set it
    // going cleared them.
    property var nowSaid: []
    // The minute heldHears' change was made in, so a turn of the real clock
    // before it's heard is told from a second change of For You's.
    property real heldMinute: -1
    // And whether it said none of what moved is shown (changedUnseen).
    property bool nowUnseen: false
    // How many times what For You lists from moved (Now.qml's forYouHeard).
    property int forYouHeardTimes: 0

    Connections {
        target: now

        function onForYouHeardChanged() {
            root.forYouHeardTimes += 1;
        }

        function onChanged() {
            root.nowChanged += 1;
            root.nowSaid.push(now.changedWhy);
            root.nowMoved = now.changedWhy;
            root.nowUnseen = now.changedUnseen;
        }
    }

    function run() {
        // The minute the row was placed in may have turned since, as it did
        // on a busy builder (its row said 09:09 as it turned, 2026-11-13), so the one before
        // now says the minute too.
        const time = [Date.now() - 60000, Date.now()].map(at => Qt.formatDateTime(new Date(at),
            "HH:mm"));
        const date = Qt.formatDate(new Date(), "dddd d MMMM yyyy");
        let rows = moment();
        expect("no Today to step into",
            !now.everything().some(row => row.key === "on:today" || row.name === "Today"),
            JSON.stringify(now.everything().map(row => row.name)));
        expect("the time, the date and the location lead Now's own rows, the meeting after them",
            JSON.stringify(names()) === JSON.stringify(["Time", "Date", "Location", "Standup"]),
            JSON.stringify(names()));
        expect("the time and the date are the launcher's own rows for them",
            rows[0].fromLauncher === true && rows[0].is === "answer:time"
                && rows[1].fromLauncher === true && rows[1].is === "answer:today",
            JSON.stringify(rows.slice(0, 2)));
        expect("the time says the minute", time.includes(rows[0].side), rows[0].side);
        expect("the date says the day", rows[1].side === date, rows[1].side);
        expect("and each holds its parts, as it does anywhere",
            rows[0].verbs()[0].name === "Zone" && rows[1].verbs()[0].name === "Week",
            JSON.stringify(rows.slice(0, 2)));
        // Calm, home's Now says nothing at its right (VKCF3Z, bd5cb965).
        expect("nothing said at home while only they are there",
            now.counted() === "" && now.homeSays() === "",
            JSON.stringify([now.counted(), now.homeSays()]));
        expect("the weather isn't asked before the launcher opens", weather.asked === 0,
            weather.asked);
        // The plate holds only the moment: calm it says nothing, as the row
        // does, and what waits is the row's to say (VKCF3Z, 2026-11-09).
        expect("calm, the plate says nothing", (now.placePlate().say ?? "") === "",
            now.placePlate().say);
        const asking = { title: "Answer the ask", minutes: 2 };
        WaitingOnYou.now = [asking];
        WaitingOnYou.foremost = asking;
        expect("what waits is the row's to say, and the plate's no more",
            now.homeSays() !== "" && (now.placePlate().say ?? "") === "",
            JSON.stringify([now.homeSays(), now.placePlate().say]));
        WaitingOnYou.now = [];
        WaitingOnYou.foremost = null;
        // The plate's clock says only what timekeeping said: chrony is asked only while home
        // shows, so a plate read shut, or the moment after opening, has no reading yet, and
        // said nothing keeps the time while chrony kept it (P9R1KI, 2026-11-15). It's left out
        // until the reader answers, and said once the reader refused.
        const clockSays = () => Array.from(now.placePlate().groups)
            .reduce((all, group) => all.concat(Array.from(group)), [])
            .filter(fact => fact.label === Words.name("on:plate:clock")).map(fact => fact.value);
        expect("unanswered, the plate's clock says nothing of how it's kept",
            clockSays().length === 0, JSON.stringify(clockSays()));
        Timekeeping.refused = true;
        const refusedSays = clockSays();
        Timekeeping.refused = false;
        expect("refused, the plate's clock says nothing keeps the time",
            JSON.stringify(refusedSays) === JSON.stringify(["nothing keeps the time"]),
            JSON.stringify(refusedSays));

        // The location says its town, and that the place keeps another zone
        // than the clock, in the colour of what wants a look, and why the
        // clock didn't follow: found only to 26 km, with nothing said of
        // the zone round it, it's wider than the clock follows.
        const location = rows[2];
        expect("the location says its town", location.side === "Kelpwater, Westmark",
            location.side);
        expect("and that the place keeps another zone",
            location.note
                === "keeps America/Toronto \u00b7 not following \u00b7 found within 26 km"
                && location.hue === "strong", `${location.note} ${location.hue}`);
        const inside = location.verbs();
        // In the design's order: its town and country, how it knows and
        // when it was located, setting it by hand, and Copy of its
        // position last; Find It Again only once a place was set by hand.
        expect("inside, its town, country, how it knows, when, setting it by hand, and Copy",
            JSON.stringify(inside.map(row => row.name))
                === JSON.stringify(["Town", "Country", "How It Knows", "Located",
                    "Set It by Hand", "Copy"]),
            JSON.stringify(inside.map(row => [row.name, row.side])));
        expect("its country is the one whereabouts named", inside[1].side === "Canada",
            inside[1].side);
        // Found, naming no daemon; its source is the plate's eyebrow
        // (VKCF3Z, 2026-11-09).
        expect("it knows it found where it is, naming no daemon",
            inside[2].side === "found, within 26 km", inside[2].side);
        // Its plate: what found it, its town, then when, how near, where
        // on a map to no more places than it knows, its country and its
        // zone last as the identifier it is (VKCF3Z, 2026-11-08 and 09).
        const plate = location.thing === true ? location.drawer() : null;
        expect("its plate says Wi-Fi found it, titled by its town",
            plate?.eyebrow === "via Wi-Fi \u00b7 location" && plate?.title === "Kelpwater",
            JSON.stringify(plate));
        expect("its plate's facts, from what moves most, the zone an identifier",
            JSON.stringify(plate.facts.map(fact => [fact.label, fact.identifies]))
                === JSON.stringify([["Located", false], ["Accuracy", false],
                    ["Coordinates", false], ["Country", false], ["Zone", true]]),
            JSON.stringify(plate.facts));
        expect("its plate names what its source can't say",
            plate.body === "Wi-Fi gives no altitude, speed or heading.", plate.body);
        expect("a position within 26 km says a tenth of a degree",
            plate.facts[2].value === "12.3\u00b0 N \u00b7 98.8\u00b0 W"
                && plate.facts[1].value === "within 26 km",
            JSON.stringify(plate.facts));
        // Where the place keeps the clock's own zone, the note says how near
        // it is and the plate leaves Accuracy to it; with no fix once
        // Location has read, it's not found yet (Y2YBTS's review of #2493).
        const clockKept = Location.clockZone;
        const found = Location.here;
        Location.clockZone = found.zone;
        const matching = now.location;
        expect("where the zones match, the note says how near",
            matching.note === "within 26 km" && matching.hue === "voice",
            `${matching.note} ${matching.hue}`);
        expect("and its plate leaves Accuracy to the note",
            JSON.stringify(matching.drawer().facts.map(fact => fact.label))
                === JSON.stringify(["Located", "Coordinates", "Country", "Zone"]),
            JSON.stringify(matching.drawer().facts));
        Location.here = null;
        expect("with no fix, not found yet", now.location.note === "not found yet",
            now.location.note);
        Location.here = found;
        Location.clockZone = clockKept;
        expect("it's found by its name too, Now being its home",
            now.things.some(row => row.key === "on:location"),
            JSON.stringify(now.things.map(row => row.key)));
        expect("nowhere, or a position that isn't one, is no location",
            Location.hereOf({ key: "here:laptop", latitude: null, longitude: null }) === null
                && Location.hereOf({ latitude: "1", longitude: 2 }) === null
                && Location.hereOf(null) === null, "it was read");
        expect("the daemon's record is here in the units the weather reads",
            Location.here?.accuracy === 26000 && Location.here?.when === 1790000000
                && Location.here?.altitude === null && Location.here?.hand === false,
            JSON.stringify(Location.here));
        expect("the clock's zone is the one its link names",
            Location.clockZone === "America/Halifax" && Location.zoneDiffers,
            Location.clockZone);

        expect("a piece borrows its place's row's drawer, and none the place doesn't list",
            now.borrowed({ place: "cameras", key: "camera:front" })?.eyebrow
                === "built in \u00b7 camera"
                && now.borrowed({ place: "cameras", key: "camera:rear" }) === null
                && now.borrowed({ place: "sound", key: "sound:mic" }) === null, "it was drawn");
        expect("and a drawer its place's row holds made",
            now.borrowed({ place: "power", key: "power:energy" })?.eyebrow
                === "held \u00b7 battery", "it wasn't drawn");
        expect("a drawer's clock rests while the launcher is closed", !now.drawing, "it ran");
        theLauncher.open = true;
        expect("and turns while it's open", now.drawing, "it rested");
        expect("opening reads the weather, before the meeting",
            JSON.stringify(names())
                === JSON.stringify(["Time", "Date", "Location", "Weather", "Standup"]),
            JSON.stringify(names()));
        rows = moment();
        expect("the weather says the sky", rows[3].side === "7\u00b0 \u00b7 cloudy", rows[3].side);
        expect("the weather is the launcher's own row for it",
            rows[3].fromLauncher === true && rows[3].is === "answer:weather",
            JSON.stringify(rows[3]));
        const meeting = rows[4];
        expect("the meeting's Return is its own", meeting.verb === "join" && meeting.later === true,
            JSON.stringify(meeting));
        meeting.run();
        expect("Return joins it", calendar.joined, "not joined");

        weather.asking = true;
        weather.answered();
        expect("asking again keeps what it knew", moment()[3].side === "7\u00b0 \u00b7 cloudy",
            moment()[3].side);
        weather.asking = false;
        weather.sky = "9\u00b0 \u00b7 rain";
        weather.answered();
        expect("a forecast heard says the new sky", moment()[3].side === "9\u00b0 \u00b7 rain",
            moment()[3].side);
        expect("the weather's row holds its own parts",
            moment()[3].verbs().map(each => each.name).join() === "Conditions",
            JSON.stringify(moment()[3]));

        calendar.next = null;
        expect("without a meeting, no meeting row",
            JSON.stringify(names()) === JSON.stringify(["Time", "Date", "Location", "Weather"]),
            JSON.stringify(names()));

        // For You's row and the margin's tray say one count, a critical
        // notice unread among it, which home's row names (Y2YBTS,
        // 2026-11-09); its card up, it's the card's to say.
        Notices.history = [{ id: 7, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: false }];
        expect("a critical notice unread is one For You holds to do, as its tray counts",
            now.waiting.side === "1 to do" && ForYouHolds.count === 1,
            JSON.stringify([now.waiting.side, ForYouHolds.count]));
        expect("and home's row names it", now.homeSays() === "Battery at 5%", now.homeSays());
        Notices.live = { "7": {} };
        expect("its card up, neither counts it",
            now.waitsNow === 0 && ForYouHolds.count === 0 && now.homeSays() === "",
            JSON.stringify([now.waitsNow, ForYouHolds.count, now.homeSays()]));
        Notices.live = {};
        // One a session raised, naming its cell, stands among what needs
        // Joel now under that session (What Needs Me, 42P0R4, 2026-11-19),
        // and is still counted once. It's pi's, as away mode signs it, which
        // Desktop leaves to the session's want; this one wants nothing.
        const raising = { address: "KELP01", title: "Session K",
            presence: { session: "s-k", addresses: ["KELP01", "9F3A2B1C"] } };
        Sessions.running = [raising];
        Notices.history = [{ id: 8, at: Date.now(), app: "pi", summary: "Session K is done",
            urgency: "critical", read: false, cell: "mycelium:cell/9F3A2B1C@heron#notice:8" }];
        const noticed = () => now.forYouBands.now.filter(one => one.kind === "notice");
        const joined = noticed();
        expect("a session's notice stands under it among what needs Joel, counted once",
            now.waiting.side === "1 to do" && ForYouHolds.count === 1 && joined.length === 1
                && joined[0].cell() === raising,
            JSON.stringify([now.waiting.side, ForYouHolds.count, joined.length,
                joined[0]?.cell()?.address ?? null]));
        // Said anew as it was, as each notice is when another comes or one
        // is read, For You bands nothing again.
        const bandsJoined = now.forYouBands;
        Notices.history = [Object.assign({}, Notices.history[0]), { id: 9, at: Date.now(),
            app: "Slack", summary: "A message", urgency: "normal", read: false }];
        expect("its notice said anew, another beside it, bands nothing again",
            now.forYouBands === bandsJoined, String(noticed().length));
        // One not critical, as away mode's are, stands there too, and is
        // counted there and in the tray alike, once, as every row among
        // what needs Joel now is (Holding.js's joined).
        const raised = Object.assign({}, Notices.history[0], { urgency: "normal" });
        Notices.history = [raised];
        expect("a notice not critical stands under its session, counted once by both",
            noticed().length === 1 && ForYouHolds.count === 1 && now.waiting.side === "1 to do",
            JSON.stringify([noticed().length, ForYouHolds.count, now.waiting.side]));
        // Beside another program's critical notice, each counts once, and
        // home's row says the one it doesn't name as one more.
        Notices.history = [raised, { id: 7, at: Date.now(), app: "power",
            summary: "Battery at 5%", urgency: "critical", read: false }];
        expect("beside another's critical notice, each counts once",
            ForYouHolds.count === 2 && now.waiting.side === "2 to do"
                && now.homeSays().endsWith("1 more"),
            JSON.stringify([ForYouHolds.count, now.waiting.side, now.homeSays()]));
        // Read while For You is open, it stays where it stood, so no row
        // moves under the cursor, and the list is the one it was.
        Notices.history = [raised];
        const bandsRaised = now.forYouBands;
        Notices.history = [Object.assign({}, raised, { read: true, readHow: "read",
            readAt: Date.now() })];
        expect("read while For You is open, it stays where it stood",
            noticed().length === 1 && now.forYouBands === bandsRaised, String(noticed().length));
        // A low one, as away mode's saying a session rests, asks nothing.
        Notices.history = [Object.assign({}, raised, { urgency: "low" })];
        expect("a low notice stands nowhere and counts nothing",
            noticed().length === 0 && ForYouHolds.count === 0,
            JSON.stringify([noticed().length, ForYouHolds.count]));
        Notices.history = [raised];
        // Once it asks Joel something, its want's row says it, so it leaves,
        // and the want is what's counted, once.
        Sessions.wanting = [Object.assign({}, raising, { presence: Object.assign({},
            raising.presence, { wants: { kind: "question", said: "Which?" } }) })];
        expect("a notice of a session wanting Joel is its want's to say",
            noticed().length === 0 && ForYouHolds.count === 1 && now.waiting.side === "1 to do",
            JSON.stringify([noticed().length, ForYouHolds.count, now.waiting.side]));
        // A critical one too: Desktop lists none of pi's, so it counts as
        // nothing apart from the want.
        Notices.history = [Object.assign({}, raised, { urgency: "critical" })];
        expect("a critical notice of a session wanting Joel is its want's to say too",
            noticed().length === 0 && ForYouHolds.count === 1 && now.waiting.side === "1 to do",
            JSON.stringify([noticed().length, ForYouHolds.count, now.waiting.side]));
        Sessions.wanting = [];
        Sessions.running = [];
        Notices.history = [];

        // For You is asked what it holds as it's stepped into and again as
        // anything in Now changes while it's held, and each look it lists is
        // kept while its file reads the same: made anew each time, 105 of
        // them made 2,535 rows in one opening on Joel's shell (2026-11-09).
        const lookAt = (id, title, session) => JSON.stringify({ v: 1, id: id, kind: "page",
            at: root.lookedAt, target: "https://example.com", title: title,
            why: "Because.", state: "waiting", session: session ?? "" });
        now.lookTexts = { "a.json": lookAt("a", "Read A", "s-a"), "b.json": lookAt("b", "Read B"),
            "c.json": lookAt("c", "Read C") };
        const looked = () => now.waiting.verbs().find(row => row.key
            === "on:waitingOnYou:kind:look")?.verbs() ?? [];
        const first = looked();
        expect("For You lists the looks to look at",
            JSON.stringify(first.map(row => row.name))
                === JSON.stringify(["Read A", "Read B", "Read C"]),
            JSON.stringify(first.map(row => row.name)));
        const madeBefore = now.tally.rows;
        const again = looked();
        expect("asked again, it makes none of its looks again",
            again.every((row, at) => row === first[at]),
            JSON.stringify(again.map((row, at) => row === first[at])));
        const madeAgain = now.tally.rows - madeBefore;
        // Banding For You is as many milliseconds as it holds items, and
        // its count, its rows and its filter each banded it afresh.
        const bandedBefore = now.bandsKept.times;
        looked();
        now.filterable();
        expect("asked again with nothing changed, For You bands nothing again",
            now.bandsKept.times === bandedBefore, now.bandsKept.times - bandedBefore);
        // The stream tells every look again as the daemon restarts.
        const looksBefore = now.looks;
        now.looksTold(records(now.lookTexts));
        expect("the looks told again as they stand change nothing", now.looks === looksBefore,
            JSON.stringify(Object.keys(now.lookTexts)));
        now.lookTexts = Object.assign({}, now.lookTexts, { "b.json": lookAt("b", "Read B Now") });
        const changed = looked();
        expect("a look whose file changed is made again, and only it",
            changed[0] === first[0] && changed[1] !== first[1] && changed[1].name
                === "Read B Now" && changed[2] === first[2],
            JSON.stringify(changed.map((row, at) => [row.name, row === first[at]])));
        expect("asking again made nothing, and then only it and To Look At's row holding it",
            madeAgain === 0 && now.tally.rows - madeBefore - madeAgain === 2,
            JSON.stringify([madeAgain, now.tally.rows - madeBefore - madeAgain]));
        // The sessions are said anew at each step and each focus move, the
        // same ones wanting Joel; only who wants him is what a look reads.
        Sessions.wanting = [];
        const stepped = looked();
        expect("a session moving, the same wanting Joel, makes none of its looks again",
            stepped.every((row, at) => row === changed[at]),
            JSON.stringify(stepped.map((row, at) => row === changed[at])));
        // A session wanting Joel blocks the look it left, which needs him
        // now and says so at its side, and changes nothing another look
        // says.
        Sessions.wanting = [{ address: "LEFTA", title: "Session A",
            presence: { session: "s-a", addresses: [] } }];
        const wanted = looked();
        const lifted = now.waiting.verbs().find(row => row.key === "on:look:a");
        expect("one more wanting Joel makes only the look it left again",
            lifted !== undefined && lifted !== changed[0] && lifted.side !== changed[0].side
                && wanted.length === 2 && wanted[0] === changed[1] && wanted[1] === changed[2],
            JSON.stringify([lifted?.side, wanted.map(row => [row.name, row.side,
                changed.includes(row)])]));
        Sessions.wanting = [];
        // A look says who asks and what, Return answering with its first
        // choice, its choices first in its step, and its drawer what happens
        // if Joel does nothing, with no weight (Joel, 2026-11-19: "I almost
        // never know what I am supposed to do with it").
        const asking = JSON.stringify({ v: 1, id: "q", kind: "pr", at: root.lookedAt,
            target: "https://example.com/pull/1", title: "History overlays stop rescanning",
            why: "Each read re-listed an empty folder.", state: "waiting", address: "leftq",
            asker: "The trunk disk incident", ask: "Merge it now, or hold it for review?",
            ifNothing: "It waits, unmerged, and the rescans go on.",
            choices: [{ value: "merge", label: "Queue It" }, { value: "hold", label: "Hold It" }] });
        now.lookTexts = { "q.json": asking, "a.json": lookAt("a", "Read A") };
        const askingRow = () => looked().find(row => row.key === "on:look:q");
        const asked = askingRow();
        const stepKeys = (asked?.verbs() ?? []).map(act => act.key);
        expect("a look's row says who asks, then what, its choices first in its step",
            asked?.note === "The trunk disk incident"
                && asked?.side === "Merge it now, or hold it for review?"
                && JSON.stringify(stepKeys.slice(0, 3)) === JSON.stringify([
                    "verb:on:look:answer:q:merge", "verb:on:look:answer:q:hold",
                    "verb:on:look:open:q"]),
            JSON.stringify([asked?.note, asked?.side, stepKeys]));
        const askedDrawer = asked?.drawer() ?? {};
        const factOf = label => (askedDrawer.facts ?? []).find(each => each.label === label)
            ?.value ?? null;
        expect("its drawer says who asks, its choices, Return's marked, and if nothing, no weight",
            askedDrawer.eyebrow === "The trunk disk incident"
                && factOf("Choices") === "\u21b5 Queue It \u00b7 Hold It"
                && factOf("If You Do Nothing") === "It waits, unmerged, and the rescans go on."
                && factOf("Weight") === null && factOf("Clears When") === null,
            JSON.stringify([askedDrawer.eyebrow, askedDrawer.facts]));
        const plainDrawer = looked().find(row => row.key === "on:look:a")?.drawer() ?? {};
        expect("a look whose session says nothing of it waits three days, then goes quiet",
            (plainDrawer.facts ?? []).find(each => each.label === "If You Do Nothing")?.value
                === "it waits here 3 days, then goes quiet into History",
            JSON.stringify(plainDrawer.facts));
        const steeredBefore = Sessions.steered.length;
        asked?.run();
        expect("Return answers with its first choice",
            Sessions.steered.length === steeredBefore + 1
                && Sessions.steered[steeredBefore]?.fields?.value === "merge"
                && askingRow()?.side === "answered Queue It",
            JSON.stringify([Sessions.steered.slice(steeredBefore), askingRow()?.side]));
        now.lookAnswers = {};
        now.lookTexts = {};

        // What's on is made again as any one fact moves, and each piece's
        // row, with its acts, is kept while what it shows reads the same:
        // every lit piece's was made anew at each move, about 12 rows a
        // change on Joel's shell (2026-11-10).
        let ran = "";
        const piece = (title, value, act) => ({ present: true, active: true, group: "on",
            title: title, value: value, actions: [{ name: "Let It Sleep", run: act }] });
        const awake = { entry: { mark: "awake" }, piece: piece("Awake", "on",
            () => ran = "awake") };
        const cast = { entry: { mark: "casting" }, piece: piece("Casting", "to the TV",
            () => ran = "cast") };
        const onRow = key => now.everything().find(row => row.key === key);
        Facts.all = [awake, cast];
        const awakeRow = onRow("on:awake");
        const castRow = onRow("on:casting");
        const onBefore = now.tally.rows;
        Facts.all = [awake, cast];
        expect("what's on said again as it was makes none of its rows again",
            now.tally.rows === onBefore && onRow("on:awake") === awakeRow
                && onRow("on:casting") === castRow, now.tally.rows - onBefore);
        cast.piece = piece("Casting", "to the speaker", () => ran = "speaker");
        Facts.all = [awake, cast];
        expect("one piece changed makes only its row and its act again",
            now.tally.rows - onBefore === 2 && onRow("on:awake") === awakeRow
                && onRow("on:casting") !== castRow
                && onRow("on:casting").side === "to the speaker", now.tally.rows - onBefore);
        // A kept row's act runs the piece's act as it is now, though the
        // piece was made anew saying the same, as a thing a piece stands
        // for is (Facts.js).
        awake.piece = piece("Awake", "on", () => ran = "awake now");
        Facts.all = [awake, cast];
        onRow("on:awake").run();
        expect("a kept row runs its piece's act as it is now",
            onRow("on:awake") === awakeRow && ran === "awake now", ran);
        Facts.all = [];

        // For You's to-dos, the sessions wanting Joel and its bands' own
        // rows are kept as its looks are, while each reads the same: each
        // was made anew whenever For You or a step in it was held as
        // anything in Now changed, with what's on making 611 rows in one
        // opening on Joel's shell (2026-11-10). The node says its to-dos
        // anew at each revision, so they're kept by what they say.
        const toDo = (title, deadline) => ({ v: 2, title: title, ready: true,
            raisedAt: Date.now() - 2 * 3600000, deadline: deadline ?? null });
        WaitingOnYou.now = [toDo("Sign In"), toDo("Say Yes")];
        now.lookTexts = { "d.json": lookAt("d", "Read D") };
        Sessions.wanting = [{ address: "WANTB", title: "Session B", presence: {
            session: "s-b", addresses: [], wants: { kind: "question", said: "Which?" } } }];
        // The sessions stand in Agents Asking, read here beside the rest.
        const agentsAsking = "on:waitingOnYou:kind:agent";
        const forYou = () => now.waiting.verbs().reduce((all, row) => all.concat([row],
            row.key === agentsAsking ? row.verbs() : []), []);
        const keys = rows => JSON.stringify(rows.map(row => row.key).sort());
        const heldFirst = forYou();
        expect("For You holds the session, the to-dos and To Look At", keys(heldFirst)
            === JSON.stringify(["on:heading:cell:WANTB", "on:heading:cell:yours",
                "on:waiting:Say Yes", "on:waiting:Sign In", "on:waiting:want:WANTB",
                "on:waitingOnYou:kind:look"]), keys(heldFirst));
        // What needs Joel now stands by the cell behind it (What Needs Me,
        // 2026-11-18): the session's heading, then what it wants, which
        // blocks it and so weighs most, then Joel's own heading over the
        // to-dos no session is behind, and To Look At after them all.
        const placed = key => heldFirst.findIndex(row => row.key === key);
        const heldOrder = ["on:heading:cell:WANTB", "on:waiting:want:WANTB",
            "on:heading:cell:yours"].map(placed);
        const toDosAt = ["on:waiting:Say Yes", "on:waiting:Sign In"].map(placed);
        expect("each cell's heading over its rows, the heaviest cell first",
            heldOrder.every((at, index) => at >= 0 && (index === 0 || at > heldOrder[index - 1]))
                && toDosAt.every(at => at > heldOrder[2])
                && placed("on:waitingOnYou:kind:look") > Math.max(...toDosAt),
            heldFirst.map(row => row.key));
        const same = (rows, from) => rows.length === from.length
            && rows.every((row, at) => row === from[at]);
        let forYouBefore = now.tally.rows;
        const heldAgain = forYou();
        expect("asked again, For You makes none of its rows again",
            same(heldAgain, heldFirst) && now.tally.rows === forYouBefore,
            now.tally.rows - forYouBefore);
        WaitingOnYou.now = [toDo("Sign In"), toDo("Say Yes")];
        forYouBefore = now.tally.rows;
        const toldAgain = forYou();
        expect("its to-dos said anew as they were, it makes none of its rows again",
            same(toldAgain, heldFirst) && now.tally.rows === forYouBefore,
            now.tally.rows - forYouBefore);
        WaitingOnYou.now = [toDo("Sign In"), toDo("Say Yes", Date.now() + 3600000)];
        forYouBefore = now.tally.rows;
        const due = forYou();
        const dueRow = due.find(row => row.key === "on:waiting:Say Yes");
        expect("one to-do now due makes only its row again",
            now.tally.rows - forYouBefore === 1 && dueRow !== undefined
                && heldFirst.indexOf(dueRow) === -1
                && due.filter(row => row !== dueRow).every(row => heldFirst.includes(row)),
            JSON.stringify([now.tally.rows - forYouBefore, due.map(row => row.key)]));
        // A session's window moving keeps its row, which reads none of it,
        // but Go To goes to the session as it is now, not as it was.
        const movedB = Object.assign({}, Sessions.wanting[0], { window: 7 });
        Sessions.wanting = [movedB];
        const wantKept = forYou().find(row => row.key === "on:waiting:want:WANTB");
        wantKept.verbs().find(act => act.key.startsWith("verb:on:waiting:go:")).run();
        expect("a kept session's Go To goes to it as it is now",
            heldFirst.includes(wantKept) && Sessions.wentTo === movedB,
            JSON.stringify(Sessions.wentTo));
        // A question holds its session's ask open, and words said to it
        // wait for that to end (lib/mesh/steer.ts), so asking offers no
        // Talk; ended blocked, its run is over and Talk reaches it at once,
        // after Deal With It, before Go To.
        const actsOf = row => row.verbs().map(act => act.key.replace(/:on:waiting:want:.*$/u, ""));
        expect("a session asking a question offers no Talk, its words held behind the ask",
            JSON.stringify(actsOf(wantKept).slice(0, 2))
                === JSON.stringify(["verb:on:waiting:deal", "verb:on:waiting:go"])
                && !actsOf(wantKept).includes("verb:agent:talk"),
            JSON.stringify(actsOf(wantKept)));
        Sessions.wanting = [Object.assign({}, movedB, { presence: Object.assign({},
            movedB.presence, { wants: { kind: "blocked", said: "Which?" } }) })];
        const blockedB = forYou().find(row => row.key === "on:waiting:want:WANTB");
        expect("a session ended blocked offers Talk to it, after Deal With It, its row made anew",
            JSON.stringify(actsOf(blockedB).slice(0, 3)) === JSON.stringify(["verb:on:waiting:deal",
                "verb:agent:talk", "verb:on:waiting:go"])
                && blockedB.verbs()[1].to === "WANTB" && blockedB !== wantKept,
            JSON.stringify(actsOf(blockedB)));
        Sessions.wanting = [movedB];
        const askedAgain = forYou().find(row => row.key === "on:waiting:want:WANTB");
        expect("asking again, it offers no Talk once more",
            !actsOf(askedAgain).includes("verb:agent:talk"), JSON.stringify(actsOf(askedAgain)));
        Sessions.wentTo = null;
        // Joel's own report: a plain ask's Put It Off names the ask the
        // row shows, and one asked anew makes the row again with it.
        Sessions.knowns = { WANTB: "WANTB question@1" };
        const askedB = forYou().find(row => row.key === "on:waiting:want:WANTB");
        const putOffB = askedB.verbs().find(act => act.key.startsWith("verb:on:waiting:notNow:"));
        Sessions.knowns = { WANTB: "WANTB question@2" };
        // The row Joel read, pressed after the session asked anew, names
        // what it showed, so the daemon can refuse it.
        putOffB.run();
        const askedAnew = forYou().find(row => row.key === "on:waiting:want:WANTB");
        askedAnew.verbs().find(act => act.key.startsWith("verb:on:waiting:notNow:")).run();
        expect("a plain ask's Put It Off names the ask its row shows, last and staying",
            JSON.stringify(Sessions.putOffAsked)
                === '["WANTB WANTB question@1","WANTB WANTB question@2"]'
                && askedAnew !== askedB && putOffB.stays === true
                && askedB.verbs()[askedB.verbs().length - 1].key === putOffB.key,
            JSON.stringify(Sessions.putOffAsked));
        Sessions.putOffAsked = [];
        Sessions.knowns = {};
        // Two asking stand together in Agents Asking, never loose among the
        // to-dos (Joel, 2026-11-18 10:18), ahead of them, since each blocks
        // its session, in the hue of what it holds and counting them; one
        // alone stood as itself, above. Asked last: B's rows all go into
        // Agents Asking, so its heading goes, and a later pass makes it anew.
        WaitingOnYou.now = [toDo("Sign In"), toDo("Say Yes")];
        Sessions.wanting = Sessions.wanting.concat([{ address: "WANTC", title: "Session W",
            presence: { session: "s-w", addresses: [], wants: { kind: "question",
                said: "Which?" } } }]);
        const led = now.waiting.verbs();
        const gathered = led[0]?.verbs?.() ?? [];
        expect("two asking lead For You in Agents Asking, in its hue, counted",
            led[0]?.key === agentsAsking && gathered.length === 2
                && led.every(row => !row.key.startsWith("on:waiting:want:"))
                && led[0].hue === (gathered[0].hue ?? "") && led[0].hue !== "muted"
                && led[0].side === "2",
            JSON.stringify(led.map(row => `${row.key} ${row.hue} ${row.side}`)));
        WaitingOnYou.now = [];
        Sessions.wanting = [];
        now.lookTexts = {};
        cellsHeaded(forYou, toDo, lookAt);
        ownToDoLeads(forYou);
        answersInPlace(forYou);
        everyAskHasAWayOut(forYou, toDo);
        doneKept(toDo);
        shut(piece, toDo);
        remakesWhatChanged(toDo);
    }

    // What needs Joel now stands under the cell behind each (What Needs Me,
    // 2026-11-18): a look a session left under that session's heading,
    // beside what it wants, and each heading saying its cell as Agents'
    // row for it does, its name and how the agents daemon says it stands,
    // Joel's own heading over the to-dos no session is behind.
    // A session whose want is an action asks it as one of its own to-dos,
    // not in Agents Asking: that to-do leads its cell, under its heading,
    // though a look it left long before outweighs it, and its standing
    // moving is heard though it asks nothing as an agent.
    function ownToDoLeads(forYou) {
        const acted = { address: "ACTED", title: "Session E", presence: { session: "s-e",
            owns: ["QEST-E"], addresses: ["ACTED"], wants: { kind: "action", said: "Sign it" } } };
        Sessions.running = [acted];
        Sessions.wanting = [acted];
        agents.rows.ACTED = { key: "agent:ACTED", name: "Session E", side: "",
            doing: "listening" };
        const form = { v: 2, title: "Sign The Form", quest: "QEST-E", ready: false,
            raisedAt: Date.now() };
        WaitingOnYou.sessionOf = { "Sign The Form": acted };
        WaitingOnYou.now = [form];
        now.lookTexts = { "e.json": JSON.stringify({ v: 1, id: "e", kind: "page",
            at: new Date(Date.now() - 7 * 3600000).toISOString(),
            target: "https://example.com", title: "Read E", why: "Because.",
            state: "waiting", session: "s-e" }) };
        const shown = () => forYou().map(row => [row.key, row.heading === true ? row.name : ""]);
        const read = shown();
        expect("a session's own to-do it asks leads its cell, under its heading",
            JSON.stringify(read) === JSON.stringify([
                ["on:heading:cell:ACTED", "Session E \u00b7 listening"],
                ["on:waiting:Sign The Form", ""], ["on:look:e", ""]]),
            JSON.stringify(read));
        // Inside, Talk to its session follows dealing with it there, so
        // what's working on it is reached from it (42P0R4, 2026-11-19); one
        // done in a window of its own leads with going there.
        const actsOf = () => forYou().find(row => row.key === "on:waiting:Sign The Form").verbs()
            .slice(0, 3).map(act => [act.name, act.to ?? ""]);
        const formActs = actsOf();
        expect("a session's to-do offers Talk to it after Deal With It, then Go To",
            JSON.stringify(formActs) === JSON.stringify(
                [["Deal With It", ""], ["talk", "ACTED"], ["Go To", ""]]),
            JSON.stringify(formActs));
        WaitingOnYou.now = [Object.assign({}, form, { window: "forms" })];
        const ownActs = actsOf();
        expect("one done in a window of its own leads with Go To, then Deal With It and Talk",
            JSON.stringify(ownActs) === JSON.stringify(
                [["Go To", ""], ["Deal With It", ""], ["talk", "ACTED"]]),
            JSON.stringify(ownActs));
        WaitingOnYou.now = [form];
        // While it asks a question or a yes, no row offers Talk to it, the
        // look it left included: its words would wait for the answer they
        // were meant to shape.
        const talksOf = key => forYou().find(row => row.key === key)?.verbs()
            .filter(act => act.name === "talk").length;
        const talksActing = [talksOf("on:waiting:Sign The Form"), talksOf("on:look:e")];
        const asked = Object.assign({}, acted, { presence: Object.assign({}, acted.presence,
            { wants: { kind: "question", said: "Which?" } }) });
        Sessions.running = [asked];
        Sessions.wanting = [asked];
        WaitingOnYou.sessionOf = { "Sign The Form": asked };
        // Its want, asked of Agents Asking, stands as a row of its own.
        const talksAll = () => [talksOf("on:waiting:Sign The Form"), talksOf("on:look:e"),
            talksOf("on:waiting:want:ACTED")];
        const talksAsking = talksAll();
        const approved = Object.assign({}, acted, { presence: Object.assign({},
            acted.presence, { wants: { kind: "approval", said: "May I?" } }) });
        Sessions.running = [approved];
        Sessions.wanting = [approved];
        WaitingOnYou.sessionOf = { "Sign The Form": approved };
        const talksYes = talksAll();
        // Parked, it runs no ask: woken, it reads its inbox as it starts,
        // so Talk reaches it, waking it as Agents' own row does.
        Sessions.running = [];
        Sessions.parked = [approved];
        const talksParked = talksAll();
        Sessions.parked = [];
        expect("no row offers Talk to a session while it asks or wants a yes, but parked",
            JSON.stringify([talksActing, talksAsking, talksYes, talksParked])
                === "[[1,1],[0,0,0],[0,0,0],[1,1,1]]",
            JSON.stringify([talksActing, talksAsking, talksYes, talksParked]));
        // With Agents not there to talk through, its to-do offers no Talk,
        // rather than one that goes nowhere, and keeps its own acts; its
        // want has no row, the row being Agents' (forYouItems).
        const blocked = Object.assign({}, acted, { presence: Object.assign({},
            acted.presence, { wants: { kind: "blocked", said: "Stuck" } }) });
        Sessions.running = [blocked];
        Sessions.wanting = [blocked];
        WaitingOnYou.sessionOf = { "Sign The Form": blocked };
        const withAgents = talksAll();
        const sources = theLauncher.sources;
        theLauncher.sources = sources.filter(one => one !== agents);
        const namesOf = key => forYou().find(row => row.key === key)?.verbs()
            .map(act => act.name) ?? null;
        const without = [namesOf("on:waiting:Sign The Form"), namesOf("on:waiting:want:ACTED")];
        theLauncher.sources = sources;
        expect("with Agents gone, its to-do offers no Talk, keeping its own acts, and no want row",
            JSON.stringify([withAgents, without]) === JSON.stringify([[1, 1, 1],
                [["Deal With It", "Go To", "Mark It Done", "Put It Off"], null]]),
            JSON.stringify([withAgents, without]));
        // Its look offers Talk and then Go To where it has a window, and only
        // Go To while it asks, among the look's own Open and Clear.
        const lookActs = () => forYou().find(row => row.key === "on:look:e").verbs()
            .map(act => act.key)
            .filter(key => key === "verb:agent:talk" || key.startsWith("verb:on:waiting:go:"));
        const windowed = session => Object.assign({}, session, { window: 3 });
        Sessions.running = [windowed(acted)];
        Sessions.wanting = [windowed(acted)];
        const actingLook = lookActs();
        Sessions.running = [windowed(asked)];
        Sessions.wanting = [windowed(asked)];
        const askingLook = lookActs();
        expect("a look offers Talk to its windowed cell, then Go To, and only Go To as it asks",
            JSON.stringify([actingLook, askingLook]) === JSON.stringify([
                ["verb:agent:talk", "verb:on:waiting:go:on:look:e"],
                ["verb:on:waiting:go:on:look:e"]]),
            JSON.stringify([actingLook, askingLook]));
        Sessions.running = [acted];
        Sessions.wanting = [acted];
        WaitingOnYou.sessionOf = { "Sign The Form": acted };
        const heardBefore = root.forYouHeardTimes;
        agents.rows.ACTED.doing = "working";
        agents.remade += 1;
        const working = shown();
        expect("its standing moving is heard, though it asks nothing as an agent",
            root.forYouHeardTimes > heardBefore
                && JSON.stringify(working[0]) === JSON.stringify(
                    ["on:heading:cell:ACTED", "Session E \u00b7 working"]),
            JSON.stringify([root.forYouHeardTimes - heardBefore, working]));
        const heardWorking = root.forYouHeardTimes;
        agents.remade += 1;
        shown();
        expect("its standing said again as it was leaves For You alone",
            root.forYouHeardTimes === heardWorking,
            String(root.forYouHeardTimes - heardWorking));
        Sessions.running = [];
        Sessions.wanting = [];
        delete agents.rows.ACTED;
        WaitingOnYou.sessionOf = {};
        WaitingOnYou.now = [];
        now.lookTexts = {};
    }

    function cellsHeaded(forYou, toDo, lookAt) {
        const left = { address: "LEFTC", title: "Session C", presence: { session: "s-c",
            addresses: [], wants: { kind: "question", said: "Which?" } } };
        Sessions.running = [left];
        Sessions.wanting = [left];
        agents.rows.LEFTC = { key: "agent:LEFTC", name: "Session C", side: "",
            doing: "needs you" };
        WaitingOnYou.now = [toDo("Sign In")];
        now.lookTexts = { "c.json": lookAt("c", "Read C", "s-c") };
        const shown = () => forYou().map(row => [row.key, row.heading === true ? row.name : ""]);
        const read = shown();
        // What it asks leads its cell, as what holds it up, though the look
        // it left an hour ago weighs as much.
        expect("a look a session left stands under its heading, each heading saying its cell",
            JSON.stringify(read) === JSON.stringify([
                ["on:heading:cell:LEFTC", "Session C \u00b7 needs you"],
                ["on:waiting:want:LEFTC", ""], ["on:look:c", ""],
                ["on:heading:cell:yours", "yours alone"], ["on:waiting:Sign In", ""]]),
            JSON.stringify(read));
        // Two asking stand together in Agents Asking, first, above every
        // heading, since a heading claims what's under it (Y2YBTS,
        // 2026-11-18 15:49); the cell that left the look keeps its heading
        // over the look alone, and no ask stands loose.
        const asking = { address: "WANTD", title: "Session D", presence: { session: "s-d",
            addresses: [], wants: { kind: "question", said: "Which?" } } };
        Sessions.running = [left, asking];
        Sessions.wanting = [left, asking];
        const gathered = shown();
        expect("two asking stand first in Agents Asking, above the headings",
            JSON.stringify(gathered) === JSON.stringify([
                ["on:waitingOnYou:kind:agent", ""],
                ["on:waiting:want:LEFTC", ""], ["on:waiting:want:WANTD", ""],
                ["on:heading:cell:LEFTC", "Session C \u00b7 needs you"], ["on:look:c", ""],
                ["on:heading:cell:yours", "yours alone"], ["on:waiting:Sign In", ""]]),
            JSON.stringify(gathered));
        // A cell's standing moving alone, what it asks and left as they
        // were: For You hears it, so a heading held never goes on saying
        // how its cell stood before. It still asks, since a look whose
        // session wants nothing stands with the looks, under no heading.
        Sessions.running = [left];
        Sessions.wanting = [left];
        agents.rows.LEFTC.doing = "listening";
        agents.remade += 1;
        const listening = shown();
        const heardBefore = root.forYouHeardTimes;
        agents.rows.LEFTC.doing = "working";
        agents.remade += 1;
        const working = shown();
        expect("a cell's standing moving alone is heard, and its heading says it",
            root.forYouHeardTimes > heardBefore
                && listening[0][0] === "on:heading:cell:LEFTC"
                && listening[0][1] === "Session C \u00b7 listening"
                && JSON.stringify(working.slice(0, 3)) === JSON.stringify([
                    ["on:heading:cell:LEFTC", "Session C \u00b7 working"],
                    ["on:waiting:want:LEFTC", ""], ["on:look:c", ""]]),
            JSON.stringify([root.forYouHeardTimes - heardBefore, listening, working]));
        Sessions.running = [];
        Sessions.wanting = [];
        delete agents.rows.LEFTC;
        WaitingOnYou.now = [];
        now.lookTexts = {};
    }

    // History's done rows, the to-dos Joel said were done, what a piece
    // finished and what a source gave that's done, are each kept while
    // what it's made of is the same: any change History heard made every
    // one anew, 10 to-dos' rows about every 4 minutes on Joel's shell
    // with the launcher shut (RCTRF8, 2026-11-15).
    function doneKept(toDo) {
        const hour = 3600000;
        const at = Date.now();
        // Each said as the node keeps it, so said anew it reads the same:
        // toDo stamps when it was raised by the clock as it's called.
        const signed = toDo("Signed In");
        const yes = toDo("Said Yes");
        const anew = one => Object.assign({}, one);
        WaitingOnYou.asked = [signed, yes];
        WaitingOnYou.settled = { done: { "Signed In": at - hour, "Said Yes": at - 2 * hour } };
        const mail = finished => ({ entry: { mark: "mail" }, piece: { present: true,
            active: true, group: "on", title: "mail", value: "", lately: "mail", came: [],
            actions: [], finished: finished } });
        const lunch = { subject: "Lunch", done: { how: "read", at: at - hour / 2 } };
        Facts.all = [mail([lunch])];
        agents.forYou = [{ row: { key: "agent:merged", name: "Merged" },
            done: { how: "merged", at: at - 3 * hour } }];
        const done = () => now.historyRows().filter(row => row.key.startsWith("on:waiting:done:")
            || row.key.startsWith("on:waiting:came:") || row.key === "agent:merged");
        const first = done();
        expect("History holds the to-dos done, what Mail finished and what Agents gave done",
            JSON.stringify(first.map(row => row.key)) === JSON.stringify([
                `on:waiting:came:mail:${at - hour / 2}`, "on:waiting:done:Signed In",
                "on:waiting:done:Said Yes", "agent:merged"]),
            JSON.stringify(first.map(row => row.key)));
        // The node says its to-dos anew at each revision, and one more
        // waits now, so History is made again, but none of its done rows.
        let before = now.tally.rows;
        WaitingOnYou.asked = [anew(signed), anew(yes), toDo("Sign Up")];
        const again = done();
        const remade = again.filter(row => !first.includes(row)).map(row => row.key);
        expect("History made again keeps every done row that reads the same",
            remade.length === 0 && again.length === first.length && now.tally.rows === before,
            JSON.stringify([now.tally.rows - before, remade]));
        // One said done at another time makes only its own row again.
        before = now.tally.rows;
        WaitingOnYou.settled = { done: { "Signed In": at - hour, "Said Yes": at - 4 * hour } };
        const moved = done().filter(row => !first.includes(row)).map(row => row.key);
        expect("a to-do said done at another time makes only its row again",
            JSON.stringify(moved) === JSON.stringify(["on:waiting:done:Said Yes"])
                && now.tally.rows - before === 1,
            JSON.stringify([now.tally.rows - before, moved]));
        WaitingOnYou.asked = [];
        WaitingOnYou.settled = {};
        Facts.all = [];
        agents.forYou = [];
    }

    // With the launcher shut, a piece moving, a to-do coming, one for
    // later, a session wanting Joel and here moving make none of Now's
    // rows, nor does a pick asking every collection what it holds
    // (Launcher.qml's knowing): what's on, the location, For You, Later
    // and History were each made again, 199 in four minutes on Joel's
    // shell (YQZGPC, 2026-11-10). The tray's count and home's row say it
    // all the same. Wanted again, they're made once, as they stand.
    function shut(piece, toDo) {
        const tv = { entry: { mark: "casting" }, piece: piece("Casting", "to the TV", () => {}) };
        Facts.all = [tv];
        const castRow = () => now.everything().find(row => row.key === "on:casting");
        expect("wanted, what's on is listed", castRow()?.side === "to the TV",
            JSON.stringify(castRow()));
        const place = now.location.side;
        theLauncher.wanting = false;
        const before = now.tally.rows;
        Facts.all = [{ entry: tv.entry, piece: piece("Casting", "to the speaker", () => {}) }];
        WaitingOnYou.now = [toDo("Sign In")];
        WaitingOnYou.later = [toDo("Renew It")];
        Sessions.wanting = [{ address: "WANTC", title: "Session C", presence: {
            session: "s-c", addresses: [], wants: { kind: "question", said: "Which?" } } }];
        const here = Location.here;
        Location.here = Object.assign({}, here, { place: "Saltmarsh, Westmark" });
        const held = now.everything().concat(now.things);
        expect("shut, what moves makes none of Now's rows",
            now.tally.rows === before && castRow()?.side === "to the TV"
            && now.location.side === place,
            `${now.tally.rows - before} made, ${held.length} held`);
        expect("and the tray and home's row count what waits",
            ForYouHolds.count === 2 && now.homeSays() === "Session C needs you \u00b7 1 more",
            JSON.stringify([ForYouHolds.count, now.homeSays()]));
        const placedShut = theLauncher.placed;
        const minuteShut = now.timedMinute;
        theLauncher.wanting = true;
        now.retimeIfStale();
        // An opening retimes already, unless the real clock turned a minute.
        const turned = now.timedMinute !== minuteShut;
        expect("wanted again, the time is placed again only if a minute turned",
            theLauncher.placed - placedShut === (turned ? 2 : 0),
            `${theLauncher.placed - placedShut} placed, a minute turned: ${turned}`);
        const made = now.tally.rows;
        expect("wanted again, they're made as they stand",
            made > before && castRow()?.side === "to the speaker"
            && now.location.side === "Saltmarsh, Westmark"
            && now.waiting.side === "2 to do" && now.laterRow?.side === "1",
            JSON.stringify([made - before, castRow()?.side, now.location.side,
                now.waiting.side, now.laterRow?.side]));
        now.everything();
        expect("and once", now.tally.rows === made, now.tally.rows - made);
        Location.here = here;
        Facts.all = [];
        WaitingOnYou.now = [];
        WaitingOnYou.later = [];
        Sessions.wanting = [];
    }

    // For You is asked what it holds again at each change in Now while it's
    // held, and each asking made its Lately rows anew, Mail's and Slack's
    // with their acts and Desktop's, and its own row at any count, a
    // session's every move among them: 7 rows an asking on a For You of two
    // pieces and the desktop's notices, a notice arriving or nothing
    // changed alike (2026-11-10). Now only a row a change names is made
    // again, and a session's move that changes nothing For You reads, as
    // its `told` says, reaches none of its rows.
    // A session asking one question with its answers given is answered
    // from For You (VKCF3Z, 2026-11-12): its row says it needs his answer
    // and how many it offers, inside are the answers, the recommended
    // first, then Deal With It set apart and In Your Own Words, then Put
    // It Off apart, and an answer's Return asks the agents daemon to answer
    // it at once, which finds the want and steers it (wants.rs, proven
    // there). So does a yes asked of him, once the daemon serves its Yes
    // and No (WCJQSG, 2026-11-16).
    function answersInPlace(forYou) {
        const since = "2026-11-12T07:00:00Z";
        Sessions.wanting = [{ address: "WANTE", title: "Session E", presence: {
            session: "s-e", addresses: ["WANTE"], wants: { kind: "question", since: since,
                said: "Push the branch?" } } }];
        // As the daemon serves them: the one it recommends first.
        agents.wants = { WANTE: { answers: { want: "question@2026-11-12T07:00:00.000Z",
            own: true, choices: [{ value: "push", label: "Yes, push it",
                description: "Its checks passed at its head.", recommended: true },
            { value: "wait", label: "Not yet", description: "", recommended: false }] } } };
        Sessions.knowns = { WANTE: "WANTE question@1" };
        const row = forYou().find(one => one.key === "on:waiting:want:WANTE");
        expect("a want with answers says how many",
            row?.side === "needs your answer \u00b7 2 choices", JSON.stringify(row?.side));
        const drawer = row.drawer();
        expect("its drawer holds the question and each answer",
            JSON.stringify(drawer.facts.map(one => [one.label, one.value]))
                === JSON.stringify([["You Were Asked", "Push the branch?"],
                    ["Recommended", "Yes, push it \u00b7 Its checks passed at its head."],
                    ["Choice", "Not yet"], ["Session", "Session E"]]),
            JSON.stringify(drawer.facts));
        const inside = row.verbs();
        // No Talk: its ask is open, so words said to it would wait for
        // the answer they were meant to shape.
        expect("inside, the recommended answer first, then Deal With It apart, Go To the "
            + "session, Put It Off last",
            JSON.stringify(inside.map(one => [one.name, one.side ?? "", !!one.gap]))
                === JSON.stringify([["Yes, push it", "recommended", false],
                    ["Not yet", "", false], ["Deal With It", "", true],
                    ["In Your Own Words", "", false],
                    ["Go To", "Session E", false], ["Put It Off", "for an hour", true]]),
            JSON.stringify(inside.map(one => [one.name, one.side, one.gap])));
        expect("an answer copies its words and its drawer leads with what it means",
            inside[0].copies === "Yes, push it"
                && inside[0].drawer().body === "Its checks passed at its head."
                && inside[1].drawer().body === "Push the branch?",
            JSON.stringify([inside[0].copies, inside[0].drawer(), inside[1].drawer()]));
        // An answer the node couldn't put says nothing of being answered,
        // since the session never heard it.
        agents.puts = false;
        inside[0].run();
        expect("an answer never put isn't said to be answered",
            agents.answered.length === 1 && theLauncher.said.length === 0,
            JSON.stringify([agents.answered, theLauncher.said]));
        agents.puts = true;
        inside[0].run();
        const answered = agents.answered[1] ?? {};
        expect("its Return asks the daemon to answer the session, and once put the foot says so",
            agents.answered.length === 2 && answered.address === "WANTE"
                && JSON.stringify(answered.fields) === JSON.stringify({ value: "push" })
                && JSON.stringify(theLauncher.said) === JSON.stringify([["answered Yes, push it",
                    ""]]),
            JSON.stringify([agents.answered, theLauncher.said]));
        expect("the shell steers nothing itself", Sessions.steered.length === 0,
            JSON.stringify(Sessions.steered));
        inside[3].given("after lunch");
        expect("his own words go as words",
            JSON.stringify(agents.answered[2]?.fields) === JSON.stringify({ other: "after lunch" }),
            JSON.stringify(agents.answered[2]));
        const named = name => inside.find(one => one.name === name);
        Sessions.wentTo = null;
        named("Go To").run();
        expect("Go To goes to the session that asked, answering nothing and filling nothing",
            Sessions.wentTo?.address === "WANTE" && agents.answered.length === 3
                && Sessions.steered.length === 0,
            JSON.stringify([Sessions.wentTo?.address ?? null, agents.answered.length,
                Sessions.steered]));
        Sessions.wentTo = null;
        named("Put It Off").run();
        expect("Put It Off asks the daemon to put off its session's want, staying open",
            JSON.stringify(Sessions.putOffAsked) === '["WANTE WANTE question@1"]'
                && named("Put It Off").stays === true,
            JSON.stringify(Sessions.putOffAsked));
        Sessions.putOffAsked = [];
        // Parked, it runs no ask, and woken it reads its inbox as it
        // starts, so Talk to it stands just before Go To (talkTo).
        Sessions.parked = [Sessions.wanting[0]];
        const parkedRow = forYou().find(one => one.key === "on:waiting:want:WANTE");
        const parkedVerbs = parkedRow.verbs();
        const parkedActs = parkedVerbs.map(one => one.name);
        parkedVerbs[parkedActs.indexOf("Go To")].run();
        const parkedWoke = Sessions.woke?.address ?? null;
        Sessions.parked = [];
        Sessions.wentTo = null;
        Sessions.woke = null;
        expect("parked, its answers offer Talk to it just before Go To, which wakes it",
            parkedRow.side === "needs your answer \u00b7 2 choices" && parkedActs.includes("talk")
                && parkedActs.indexOf("talk") === parkedActs.indexOf("Go To") - 1
                && parkedVerbs[parkedActs.indexOf("talk")].to === "WANTE"
                && parkedWoke === "WANTE",
            JSON.stringify([parkedActs, parkedVerbs[parkedActs.indexOf("talk")].to, parkedWoke]));
        Sessions.knowns = {};
        // A yes asked of him, as the harness's approval gate asks it once
        // its want carries Yes and No, is answered in its row the same way.
        Sessions.wanting = [{ address: "WANTF", title: "Session F", presence: {
            session: "s-f", addresses: [], wants: { kind: "approval", since: since,
                said: "Run tool_gateway_enable_backend" } } }];
        agents.wants = { WANTF: { answers: { want: "approval@2026-11-12T07:00:00.000Z",
            own: false, choices: [{ value: "yes", label: "Yes", description: "",
                recommended: false }, { value: "no", label: "No", description: "",
                recommended: false }] } } };
        const approval = forYou().find(one => one.key === "on:waiting:want:WANTF");
        const asked = approval?.verbs() ?? [];
        expect("a yes asked with its choices says so, and holds them, Deal With It, Go To "
            + "and Put It Off",
            approval?.side === "needs your answer \u00b7 2 choices"
                && JSON.stringify(asked.map(one => one.name))
                    === '["Yes","No","Deal With It","Go To","Put It Off"]',
            JSON.stringify([approval?.side, asked.map(one => one.name)]));
        asked[0]?.run();
        expect("its Yes asks the daemon to answer it",
            agents.answered[3]?.address === "WANTF"
                && JSON.stringify(agents.answered[3]?.fields) === JSON.stringify({ value: "yes" }),
            JSON.stringify(agents.answered[3]));
        agents.answered = [];
        agents.wants = {};
        theLauncher.said = [];
        Sessions.wanting = [];
    }

    // Every row Now makes for For You as Joel's to do offers inside a way to
    // do it and Put It Off: a to-do, a session's plain ask, one with its
    // answers, a yes. Joel found a session's yes he could neither answer nor
    // put off on 16 Nov ("i can't get this highlighted notice to go away"),
    // so a row with answers offers each one, and each session's Put It Off
    // puts off the ask it showed, though the session asked anew since. Rows
    // from other sources stay outside it: Agents' sessions to reopen go by
    // Reopen or Forget and put off only the last boot's, and Sound's offer
    // by No Thanks.
    function everyAskHasAWayOut(forYou, toDo) {
        const since = "2026-11-17T07:00:00Z";
        const asks = (address, kind, said) => ({ address: address, title: `Session ${address}`,
            presence: { session: `s-${address}`, addresses: [], wants: { kind: kind, since: since,
                said: said } } });
        WaitingOnYou.now = [toDo("Sign In")];
        Sessions.wanting = [asks("WANTG", "question", "Which branch?"),
            asks("WANTH", "approval", "Run tool_gateway_enable_backend"),
            asks("WANTI", "question", "Push it?")];
        agents.wants = {
            WANTH: { answers: { want: "approval@2026-11-17T07:00:00.000Z", own: false,
                choices: [{ value: "yes", label: "Yes", description: "", recommended: false },
                    { value: "no", label: "No", description: "", recommended: false }] } },
            WANTI: { answers: { want: "question@2026-11-17T07:00:00.000Z", own: true,
                choices: [{ value: "push", label: "Push", description: "",
                    recommended: true }] } }
        };
        Sessions.knowns = { WANTG: "WANTG question@1", WANTH: "WANTH approval@1",
            WANTI: "WANTI question@1" };
        const putOff = act => /^verb:on:waiting:notNow:/.test(act.key)
            && typeof act.run === "function";
        const answer = act => act.key.startsWith("verb:on:waiting:answer:")
            && typeof act.run === "function";
        const does = act => !putOff(act)
            && (typeof act.run === "function" || typeof act.given === "function");
        const served = { "on:waiting:want:WANTH": 2, "on:waiting:want:WANTI": 1 };
        // What agents ask is Joel's to do as well, though its own kind.
        const doing = forYou().filter(row => ["do", "agent"].includes(row.forYou?.kind));
        // What each row lacks, so a red run says which half went missing.
        const lacks = doing.map(row => {
            const inside = typeof row.verbs === "function" ? row.verbs() : [];
            const answers = inside.filter(answer).length;
            return [row.key, inside.some(putOff) ? "" : "put off",
                inside.some(does) && answers === (served[row.key] ?? 0) ? "" : `do (${answers})`];
        }).filter(([, off, way]) => off !== "" || way !== "");
        expect("every row Now gives For You as Joel's to do has a way to do it and to put it off",
            JSON.stringify(doing.map(row => row.key).sort()) === JSON.stringify([
                "on:waiting:Sign In", "on:waiting:want:WANTG", "on:waiting:want:WANTH",
                "on:waiting:want:WANTI"]) && lacks.length === 0,
            JSON.stringify([doing.map(row => row.key), lacks]));
        // Each session asks anew before Joel presses; he put off what he read.
        Sessions.knowns = { WANTG: "WANTG question@2", WANTH: "WANTH approval@2",
            WANTI: "WANTI question@2" };
        for (const row of doing.filter(one => one.key.startsWith("on:waiting:want:"))) {
            row.verbs().find(putOff).run();
        }
        expect("each session's Put It Off in For You puts off the ask its row showed",
            JSON.stringify(Sessions.putOffAsked.slice().sort()) === JSON.stringify([
                "WANTG WANTG question@1", "WANTH WANTH approval@1", "WANTI WANTI question@1"]),
            JSON.stringify(Sessions.putOffAsked));
        WaitingOnYou.now = [];
        Sessions.wanting = [];
        Sessions.knowns = {};
        Sessions.putOffAsked = [];
        agents.wants = {};
    }

    function remakesWhatChanged(toDo) {
        let ran = "";
        const lately = (source, who, act) => ({ entry: { mark: source }, piece: {
            present: true, active: true, group: "on", title: source, value: "",
            lately: source, came: [{ who: who, at: Date.now() - 60000 }],
            actions: [{ name: "Open It", run: act }, { name: "Mark It Read", run: () => {} }]
        } });
        const notice = id => ({ id: id, at: Date.now() - id * 60000, app: "Tester",
            summary: `Notice ${id}`, body: "said", glyph: 0xf0f3, hue: "program",
            read: id > 3, missed: false, readAt: id > 3 ? Date.now() - id * 60000 : null });
        const told = [];
        for (let id = 2; id <= 20; id++)
            told.push(notice(id));
        const session = (busy, said, digest) => ({ address: "WANTD", title: "Session D",
            told: digest, presence: { session: "s-d", addresses: [], busy: busy,
                wants: { kind: "question", said: said, since: "2026-11-10T12:00:00Z" } } });
        // Lately's places are fixed while the launcher is open, and none
        // leaves For You then, so these start shut, as Now hears it, and
        // say when it opens (openedAt).
        const wasOpenedAt = now.openedAt;
        now.openedAt = Number.POSITIVE_INFINITY;
        Facts.all = [lately("mail", "Ann", () => ran = "mail"), lately("slack", "Bo", () => {})];
        Notices.history = told;
        WaitingOnYou.now = [toDo("Sign In")];
        WaitingOnYou.later = [toDo("Renew It")];
        Sessions.wanting = [session(false, "Which?", "d1")];
        // Agents Asking's sessions read beside the rest, and Now's own rows,
        // where what asks nothing of Joel stands.
        const inForYou = () => now.waiting.verbs().reduce((all, row) => all.concat([row],
            row.key === "on:waitingOnYou:kind:agent" ? row.verbs() : []), []);
        const forYou = () => inForYou().concat(now.everything());
        const byKey = rows => rows.reduce((keyed, row) => Object.assign(keyed,
            { [row.key]: row }), {});
        // Mail, Slack and Desktop ask nothing of Joel, so they stand in
        // Now's own rows, not What Needs Me (Joel, 2026-11-19: "I almost
        // never know what I am supposed to do with it").
        const asksNothing = ["on:mail", "on:slack", "on:waiting:desktop"];
        const keysOf = rows => rows.map(row => row.key);
        expect("Mail, Slack and Desktop stand in Now's own rows, not For You",
            asksNothing.every(key => keysOf(now.everything()).includes(key)
                && !keysOf(inForYou()).includes(key)),
            JSON.stringify([keysOf(inForYou()), keysOf(now.everything())]));
        // They come right after the moment's, before every piece's group,
        // so what Joel reads first under the time is what came for him.
        const moment = now.momentAt(now.location).length;
        expect("Mail, Slack and Desktop come right after the moment's rows",
            JSON.stringify(keysOf(now.everything()).slice(1 + moment, 4 + moment).sort())
                === JSON.stringify(asksNothing.slice().sort()),
            JSON.stringify([moment, keysOf(now.everything())]));
        // A critical notice unread is one to do, so Desktop, which holds
        // it, stands in For You while it waits, and leaves Now's rows.
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: false }].concat(told);
        expect("a critical notice unread keeps Desktop in For You, and out of Now's rows",
            keysOf(inForYou()).includes("on:waiting:desktop")
                && !keysOf(now.everything()).includes("on:waiting:desktop")
                && now.leadsTo("notices")[0] === now.waiting
                && now.leadsTo("notices")[1].key === "on:waiting:desktop"
                && now.leadsTo("notices").length === 2,
            JSON.stringify([keysOf(inForYou()), now.leadsTo("notices").map(row => row.key)]));
        // Read elsewhere while Joel looks, which Lately doesn't hold still,
        // it no longer waits, but Desktop keeps the place he found it in.
        now.openedAt = Date.now() - 1000;
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: true, readAt: Date.now(), readHow: "read on the phone" }]
            .concat(told);
        expect("a critical notice read while the launcher's open keeps Desktop in For You",
            keysOf(inForYou()).includes("on:waiting:desktop"), JSON.stringify(keysOf(inForYou())));
        // Withdrawn by its sender, or woken back onto the screen, it no
        // longer waits, but Desktop keeps the place Joel found it in until
        // the launcher closes, whatever moved the notice.
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: true, withdrawn: true, readAt: Date.now() }].concat(told);
        expect("a critical notice withdrawn while the launcher's open keeps Desktop in For You",
            keysOf(inForYou()).includes("on:waiting:desktop"), JSON.stringify(keysOf(inForYou())));
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: false }].concat(told);
        Notices.live = { "1": {} };
        expect("a critical notice woken onto the screen while it's open keeps Desktop in For You",
            keysOf(inForYou()).includes("on:waiting:desktop"), JSON.stringify(keysOf(inForYou())));
        now.openedAt = Number.POSITIVE_INFINITY;
        // Opened with its card up, Desktop stands in Now's rows. Once the
        // card's put away unread it waits, so Desktop comes into For You,
        // which counts it, but not while Joel stands inside Desktop or in
        // Now's own rows, where it would leave from under him.
        expect("a critical notice with its card up leaves Desktop in Now's rows",
            !keysOf(inForYou()).includes("on:waiting:desktop")
                && keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        now.openedAt = Date.now();
        const wasHeld = theLauncher.held;
        const nowStep = { row: { from: now, key: "on" } };
        theLauncher.held = [nowStep,
            { row: { key: "on:waiting:desktop", forYou: { source: "desktop" } } }];
        Notices.live = {};
        expect("a critical card put away while Joel's inside Desktop leaves it in Now's rows",
            !keysOf(inForYou()).includes("on:waiting:desktop")
                && keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        theLauncher.held = [nowStep];
        expect("stepped back out onto Now's rows, Desktop still stands where he left it",
            keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(now.everything())));
        // The launcher asks a step's rows before it holds the step, so For
        // You lists Desktop on the first asking, not a refresh later.
        expect("and For You, asked as he steps into it, lists Desktop at once",
            keysOf(inForYou()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        // One step down another of Now's rows, - climbs back to Now's rows,
        // so Desktop stays there too.
        theLauncher.held = [nowStep, { row: { key: "on:mail", forYou: { source: "mail" } } }];
        expect("inside Mail, Desktop still stands in the Now rows - climbs back to",
            keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(now.everything())));
        const forYouStep = { row: { key: "on:waitingOnYou" } };
        theLauncher.held = [nowStep, forYouStep];
        expect("and once he steps into For You, Desktop is there, which counts it",
            keysOf(inForYou()).includes("on:waiting:desktop")
                && !keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        // Read inside Desktop before he steps into For You, the tray no
        // longer counts it, so nothing comes into For You for it.
        now.openedAt = Number.POSITIVE_INFINITY;
        Notices.live = { "1": {} };
        now.openedAt = Date.now() - 1000;
        theLauncher.held = [nowStep,
            { row: { key: "on:waiting:desktop", forYou: { source: "desktop" } } }];
        Notices.live = {};
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: true, readAt: Date.now(), readHow: "read" }].concat(told);
        theLauncher.held = [nowStep, forYouStep];
        expect("a critical notice read inside Desktop brings nothing into For You",
            !keysOf(inForYou()).includes("on:waiting:desktop")
                && keysOf(now.everything()).includes("on:waiting:desktop"),
            JSON.stringify([keysOf(inForYou()), keysOf(now.everything())]));
        // Found by name from home, For You is the one step held, and it's
        // Now's own row, so Desktop comes in there and stays once it has.
        now.openedAt = Number.POSITIVE_INFINITY;
        now.openedAt = Date.now() - 1000;
        theLauncher.held = [{ row: { from: now, key: "on:waitingOnYou" } }];
        Notices.live = { "1": {} };
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: false }].concat(told);
        Notices.live = {};
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: true, readAt: Date.now(), readHow: "read on the phone" }]
            .concat(told);
        expect("in For You found by name, Desktop stays once its notice is read elsewhere",
            keysOf(inForYou()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        theLauncher.held = wasHeld;
        now.openedAt = Number.POSITIVE_INFINITY;
        Notices.history = [{ id: 1, at: Date.now(), app: "power", summary: "Battery at 5%",
            urgency: "critical", read: true, readAt: Date.now() }].concat(told);
        expect("and once the launcher's shut, Desktop goes back to Now's rows",
            !keysOf(inForYou()).includes("on:waiting:desktop"),
            JSON.stringify(keysOf(inForYou())));
        // A critical one Slack's row holds is one to do in Slack's row, so
        // Slack's row stands in For You with it, and Desktop stays put.
        Notices.history = [{ id: 2, at: Date.now(), app: "Slack", summary: "Pager",
            urgency: "critical", read: false }].concat(told);
        expect("a critical notice Slack's row holds brings Slack's row, not Desktop, to For You",
            !keysOf(inForYou()).includes("on:waiting:desktop")
                && keysOf(inForYou()).includes("on:slack")
                && !keysOf(now.everything()).includes("on:slack")
                && now.leadsTo("notices").length === 1,
            JSON.stringify([keysOf(inForYou()), now.leadsTo("notices").map(row => row.key)]));
        // Slack's daemon answering, its notices stand for nothing in its
        // row, but a critical one still counts, so its row comes too.
        Facts.all = [lately("mail", "Ann", () => ran = "mail"), Object.assign(
            lately("slack", "Bo", () => {}), { piece: Object.assign(lately("slack", "Bo",
                () => {}).piece, { answers: true }) })];
        expect("a critical Slack notice while Slack's daemon answers brings Slack's row too",
            keysOf(inForYou()).includes("on:slack"), JSON.stringify(keysOf(inForYou())));
        Facts.all = [lately("mail", "Ann", () => ran = "mail"), lately("slack", "Bo", () => {})];
        Notices.history = told;
        expect("once it's gone, Slack's row goes back to Now's, and Super+N to Desktop there",
            !keysOf(inForYou()).includes("on:slack")
                && !keysOf(inForYou()).includes("on:waiting:desktop")
                && now.leadsTo("notices").length === 1
                && now.leadsTo("notices")[0].key === "on:waiting:desktop",
            JSON.stringify(now.leadsTo("notices").map(row => row.key)));
        now.openedAt = wasOpenedAt;
        const first = byKey(forYou());
        const mail = first["on:mail"];
        let before = now.tally.rows;
        const again = byKey(forYou());
        expect("asked again with nothing changed, For You makes none of its rows",
            now.tally.rows === before && Object.keys(first).every(key => again[key] === first[key]),
            now.tally.rows - before);
        // A piece said anew as it was keeps its row, and its act runs the
        // piece as it is now.
        Facts.all = [lately("mail", "Ann", () => ran = "mail now"), lately("slack", "Bo",
            () => {})];
        before = now.tally.rows;
        const said = byKey(forYou());
        said["on:mail"].verbs()[0].run();
        expect("Mail said anew as it was keeps its row, and its act is Mail's as it is now",
            said["on:mail"] === mail && ran === "mail now", `${ran}, ${said["on:mail"] === mail}`);
        const waitingRow = now.waiting;
        // Counted from before it comes, since Now's own rows, where Desktop
        // stands, are made as it does.
        before = now.tally.rows;
        Notices.history = [notice(1)].concat(told);
        const came = byKey(forYou());
        const remade = Object.keys(came).filter(key => came[key] !== first[key]);
        expect("one notice arriving makes only Desktop's row again",
            now.tally.rows - before === 1 && JSON.stringify(remade)
                === JSON.stringify(["on:waiting:desktop"]) && now.waiting === waitingRow,
            JSON.stringify([now.tally.rows - before, remade]));
        const desktop = () => now.everything().find(row => row.key === "on:waiting:desktop")
            .verbs();
        desktop();
        before = now.tally.rows;
        desktop();
        expect("Desktop stepped into again makes none of its rows", now.tally.rows === before,
            now.tally.rows - before);
        // A session working, its want the same, is a shown-only change, its
        // digest unmoved: none of For You's rows, nor its count, hear it.
        const bands = now.forYouBands;
        before = now.tally.rows;
        Sessions.wanting = [session(true, "Which?", "d1")];
        expect("a session's shown-only move reaches none of For You's rows",
            now.tally.rows === before && now.forYouBands === bands && now.waiting === waitingRow,
            JSON.stringify([now.tally.rows - before, now.forYouBands === bands]));
        // Without a digest, it's told by what For You reads of it.
        Sessions.wanting = [session(false, "Which?")];
        const undigested = now.forYouBands;
        Sessions.wanting = [session(true, "Which?")];
        expect("and without a digest, by what For You reads of it",
            now.forYouBands === undigested, "its bands were made again");
        Sessions.wanting = [session(false, "Which one?", "d2")];
        const asked = byKey(forYou())["on:waiting:want:WANTD"];
        expect("its want changed, its row says so",
            now.forYouBands !== undigested && asked?.name === "Session D"
                && asked !== first["on:waiting:want:WANTD"], JSON.stringify(asked?.name));
        // For You held, a to-do said otherwise, its count the same, leaves
        // For You's own row as it was, so Now asks once the change is done
        // whether what's held reads otherwise (heldHears).
        theLauncher.open = true;
        theLauncher.held = [{ row: { from: now, key: "on" } },
            { row: { key: "on:waitingOnYou" } }];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        root.nowSaid = [];
        root.heldMinute = now.minuteNow();
        WaitingOnYou.now = [toDo("Sign In Again")];
        expect("a to-do said otherwise, its count the same, keeps For You's own row",
            now.waiting === waitingRow && root.nowChanged === 0, root.nowChanged);
    }

    // The notices moving, with For You held, are asked with For You's own
    // after the change rather than said outright: a session's notice updated
    // as its work turned just after Return into For You was drawn as
    // Return's act (the niri case, 2026-11-18). Deeper, where the notices
    // themselves are shown, and with nothing held, they're said outright;
    // so too with Desktop or what's on screen held right under Now, where
    // Super+N leads with no critical notice waiting (round 7, F77), though
    // not with another of Now's rows there, Mail's (round 8, F80).
    function noticesWeighed(into) {
        const outright = (name, held, said) => {
            theLauncher.held = held;
            root.nowChanged = 0;
            root.nowMoved = "";
            now.noticesMoved("the notices' rows");
            expect(name, root.nowChanged === said
                && root.nowMoved === (said === 1 ? "the notices' rows" : ""),
                JSON.stringify([root.nowChanged, root.nowMoved]));
        };
        outright("For You held, the notices moving aren't said outright",
            into(now.waiting.verbs()), 0);
        outright("held below For You, they are", into(now.waiting.verbs()).concat([
            { row: { key: "on:waiting:screen" }, rows: [] }]), 1);
        const underNow = key => [{ row: { from: now, key: "on" } },
            { row: { key: key }, rows: [] }];
        outright("held right under Now, what's on screen, they are",
            underNow("on:waiting:screen"), 1);
        outright("and Desktop right under Now, they are", underNow("on:waiting:desktop"), 1);
        outright("but Mail right under Now, they aren't", underNow("piece:mail"), 0);
        outright("as they are with nothing held", [], 1);
    }

    function heldHears() {
        // Told once by what Now compares, which says what moved. Now says
        // the notices' rows changed outright with nothing held, naming
        // nothing, and held asks them with For You's own, so their daemon
        // answering late is no second telling of For You: counting
        // every change read 2 on loaded builders (train run 23a23e5d and
        // three more, 16 and 17 November), as it does with the notices'
        // rows changed just after the case. A minute turning while the
        // cases held the thread remakes the location row, which reads the
        // clock, before For You is asked, and that change tells it as Now's
        // own rows.
        const moved = root.nowSaid;
        const told = moved.filter(why => why !== "");
        const turned = now.minuteNow() !== root.heldMinute;
        expect("and For You held hears it once the change is done", told.length === 1
            && (told[0] === "the rows it holds" || turned && told[0] === "its own rows"),
            JSON.stringify([moved, turned]));
        // Said last with nothing held, Now stepped into For You since
        // compares what For You shows, as the launcher made it stepping
        // in: Joel's journal timed For You drawn again as Return's act
        // after 73 of 81 steps into it (2026-11-15).
        const into = rows => [{ row: { from: now, key: "on" } },
            { row: { key: "on:waitingOnYou" }, rows: rows }];
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        theLauncher.held = into(now.waiting.verbs());
        now.changedTo(now.everything());
        expect("stepped into For You, a change that moves none of its rows says nothing",
            root.nowChanged === 0, root.nowChanged);
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        theLauncher.held = into([]);
        now.changedTo(now.everything());
        expect("and one whose rows differ from what For You shows says so",
            root.nowChanged === 1, root.nowChanged);
        // Each change says what moved while it's heard, and nothing once
        // it's said, so a change said outright names none.
        expect("saying it was the rows For You holds",
            root.nowMoved === "the rows it holds as it stepped", root.nowMoved);
        expect("and nothing once said", now.changedWhy === "", now.changedWhy);
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowMoved = "";
        now.changedTo([]);
        expect("a change with nothing held says its own rows moved",
            root.nowMoved === "its own rows", root.nowMoved);
        now.changedTo(now.everything());
        theLauncher.held = into([]);
        root.nowMoved = "";
        now.changedTo([]);
        expect("and stepped in, one moving its own rows says so",
            root.nowMoved === "its own rows as it stepped", root.nowMoved);
        // For You held, one of Now's other rows moving is drawn nowhere,
        // so it's no key's act: a session's work turning just after Return
        // was timed as Return's (the niri case, 16 November).
        const another = { key: "on:another", name: "Another" };
        const said = (name, changes, unseen) => expect(name,
            root.nowChanged === changes && root.nowUnseen === unseen,
            JSON.stringify([root.nowChanged, root.nowUnseen]));
        const forYouAs = made => now.everything().map(row => row.key === "on:waitingOnYou"
            ? Object.assign({}, row, made) : row);
        theLauncher.held = into(now.waiting.verbs());
        now.changedTo(now.everything());
        root.nowChanged = 0;
        now.changedTo(now.everything().concat([another]));
        said("For You held, another of Now's rows moving says none of it is shown", 1, true);
        expect("and not unseen once said", !now.changedUnseen, "it stayed unseen");
        now.changedTo(forYouAs({ name: "For You Renamed" }));
        said("but For You's own line moving is shown", 2, false);
        now.changedTo(now.everything());
        said("as is For You put back", 3, false);
        now.changedTo(forYouAs({ verbs: () => [] }));
        said("and what For You holds moving, its own line the same", 4, false);
        now.changedTo(now.everything());
        // A held row gone from the rows is never unseen, since what the
        // launcher draws for it can't be read: Windows' agents' fold isn't
        // among its things, and its keys' acts went untimed.
        now.changedTo(now.everything().filter(row => row.key !== "on:waitingOnYou"));
        said("For You gone from Now's rows is shown", 6, false);
        // This one pins the guard: the change before left nothing held to
        // compare with, so a gone row read as "" would match it.
        now.changedTo(now.everything().filter(row => row.key !== "on:waitingOnYou")
            .concat([another]));
        said("and so is a change after, while it's still gone", 7, false);
        // Stepping, what For You holds is compared with what the launcher
        // made as it stepped, and its own line with what it read before.
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        theLauncher.held = into(now.waiting.verbs());
        now.changedTo(now.everything().concat([another]));
        said("stepped into For You, another row moving says none of it is shown", 1, true);
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        theLauncher.held = into(now.waiting.verbs());
        now.changedTo(forYouAs({ name: "For You Renamed" }).concat([another]));
        said("but For You's own line moving as it's stepped into is shown", 1, false);
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        theLauncher.held = into([]);
        now.changedTo(now.everything().concat([another]));
        said("as is what it holds differing from what the launcher made", 1, false);
        theLauncher.held = [];
        now.changedTo(now.everything());
        root.nowChanged = 0;
        now.changedTo(now.everything().concat([another]));
        said("with nothing held, any row moving is shown", 1, false);
        const gone = () => now.everything().filter(row => row.key !== "on:waitingOnYou");
        now.changedTo(gone());
        root.nowChanged = 0;
        theLauncher.held = into(now.waiting.verbs());
        now.changedTo(gone().concat([another]));
        said("stepped into For You while it's gone, another row moving is shown", 1, false);
        // For You's rows are asked once a change, held or stepped into:
        // waitingRows() is costly, and two asks could read across a turn.
        let asks = 0;
        const counted = forYouAs({ verbs: () => {
            asks += 1;
            return now.waiting.verbs();
        } });
        now.changedTo(now.everything());
        asks = 0;
        now.changedTo(counted);
        expect("For You held, a change asks what it holds once", asks === 1, asks);
        theLauncher.held = [];
        now.changedTo(now.everything());
        theLauncher.held = into(now.waiting.verbs());
        asks = 0;
        now.changedTo(counted);
        expect("and stepped into, once too", asks === 1, asks);
        noticesWeighed(into);
        theLauncher.held = [];
        Facts.all = [];
        Notices.history = [];
        WaitingOnYou.now = [];
        WaitingOnYou.later = [];
        Sessions.wanting = [];
    }
}
