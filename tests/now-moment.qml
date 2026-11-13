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
// Desktop's row again, and a session's move that changes nothing For You
// reads of it, its want the same, makes none; For You held still hears a
// change its own row doesn't count, once the change is done. For You is
// banded once for each change, asked again it bands nothing, and the
// files an opening reads are taken together, one read as it was not at all.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
// Long because its cases run in turn on one Now, its stand-ins and one
// stand-in notices daemon, which each file split from it would build again.
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
    // You makes is made again only when For You's own making is.
    QtObject {
        id: agents

        readonly property var settings: ({ name: "agents" })
        readonly property var toReopen: null
        readonly property var rows: ({})

        function made(session) {
            if (!(session.address in rows))
                rows[session.address] = { key: `agent:${session.address}`,
                    name: session.title, side: "" };
            return rows[session.address];
        }
    }

    // The launcher's placing of an answered thing, as Launcher.qml's
    // resolved gives it: named by the thing, what it says beside, its
    // parts a step in. Each row it gives is marked, so the test knows Now
    // showed the launcher's row and not one of its own.
    QtObject {
        id: theLauncher

        property bool open: false
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
        readonly property var sources: [weather, calendar, cameras, power, pastNotices, agents]
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

    // The notices' own rows, which For You's Desktop and History hold.
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

    // For You's Desktop holds the notices unread as the notices daemon
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
    function madeNow() {
        return [pastNotices.tally.rows, now.tally.rows];
    }
    function since(before) {
        return madeNow().map((count, at) => count - before[at]);
    }
    function stepIn() {
        const held = now.leadsTo("notices")[1].verbs();
        return { held: held, history: held[held.length - 1].verbs() };
    }
    function served(id, read) {
        return pastNotices.rowsOf([id], "history")[0]?.unread === !read;
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
            && served(30, true))
            firstNotices();
        else if (waiting === "one read" && pastNotices.rowsOf([2], "desktop").length === 0
            && served(2, true))
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
            expect("For You's Desktop holds the notices unread, then Mark Them Read and History",
                JSON.stringify(desktop.held.map(row => row.name)) === JSON.stringify([
                    "Notice 1", "Notice 2", "Notice 3", "Mark Them Read", "History"])
                    && desktop.history.length === 27,
                JSON.stringify([desktop.held.map(row => row.name), desktop.history.length]));
            const before = madeNow();
            stepIn();
            expect("stepped into again, it makes none of the notices' rows again",
                since(before)[0] === 0, JSON.stringify(since(before)));
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
        stepIn();
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
    // Every notice's made row, by notice, as search holds them (Desktop's
    // are copied as they're listed, so can't be told apart this way).
    // Taken before a count and after its read, so making none adds to it.
    function heldMade() {
        const held = { every: {} };
        for (const row of pastNotices.all())
            held.every[row.notice] = row;
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
            stepIn();
            const made = since(madeBefore);
            const after = heldMade();
            expect("one notice read, stepping in again makes only its row again",
                made[0] === 1,
                `${JSON.stringify(made)}, made again: `
                    + JSON.stringify(remadeSince(steppedBefore, after)));
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
    // the daemon's `rows` answer brought, asked once as History was first
    // stepped into, and let go as the launcher closes.
    function historyPastPage() {
        const parcel = noticesDaemon.unserved[0];
        Notices.history = noticesDaemon.notices.concat([parcel]);
        if (tried(() => {
            const history = stepIn().history;
            const row = history.find(one => one.name === "Parcel delivered") ?? null;
            expect("History lists a notice past the page as its notice's row",
                row !== null && row.key === "notice:31" && row.notice === 31,
                JSON.stringify(history.map(one => [one.key, one.name])));
            expect("the rows past the page were asked for once",
                noticesDaemon.rowsAsked === 1, `${noticesDaemon.rowsAsked} asks`);
            theLauncher.open = false;
            expect("and they go as the launcher closes",
                pastNotices.whole.kept === null, JSON.stringify(pastNotices.whole.kept?.keys));
        }))
            Qt.exit(0);
    }

    // The notices daemon, serving the rows of the notices a case gives.
    NoticesDaemon {
        id: noticesDaemon
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

    Connections {
        target: now

        function onChanged() {
            root.nowChanged += 1;
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
        const forYou = () => now.waiting.verbs();
        const keys = rows => JSON.stringify(rows.map(row => row.key).sort());
        const heldFirst = forYou();
        expect("For You holds the session, the to-dos and To Look At", keys(heldFirst)
            === JSON.stringify(["on:waiting:Say Yes", "on:waiting:Sign In",
                "on:waiting:want:WANTB", "on:waitingOnYou:kind:look"]), keys(heldFirst));
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
        Sessions.wentTo = null;
        WaitingOnYou.now = [];
        Sessions.wanting = [];
        now.lookTexts = {};
        answersInPlace(forYou);
        shut(piece, toDo);
        remakesWhatChanged(toDo);
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
    // first, then Deal With It set apart and In Your Own Words, and an
    // answer's Return steers the session its value at once.
    function answersInPlace(forYou) {
        const since = "2026-11-12T07:00:00Z";
        Sessions.wanting = [{ address: "WANTE", title: "Session E", presence: {
            session: "s-e", addresses: [], wants: { kind: "question", since: since,
                said: "Push the branch?", ask: { recommended: "push", allowOther: true,
                    choices: [{ value: "wait", label: "Not yet" },
                        { value: "push", label: "Yes, push it",
                            description: "Its checks passed at its head." }] } } } }];
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
        expect("inside, the recommended answer first, then Deal With It apart",
            JSON.stringify(inside.map(one => [one.name, one.side ?? "", !!one.gap]))
                === JSON.stringify([["Yes, push it", "recommended", false],
                    ["Not yet", "", false], ["Deal With It", "", true],
                    ["In Your Own Words", "", false]]),
            JSON.stringify(inside.map(one => [one.name, one.side, one.gap])));
        expect("an answer copies its words and its drawer leads with what it means",
            inside[0].copies === "Yes, push it"
                && inside[0].drawer().body === "Its checks passed at its head."
                && inside[1].drawer().body === "Push the branch?",
            JSON.stringify([inside[0].copies, inside[0].drawer(), inside[1].drawer()]));
        // An answer the node couldn't put says nothing of being answered,
        // since the session never heard it.
        Sessions.puts = false;
        inside[0].run();
        expect("an answer never put isn't said to be answered",
            Sessions.steered.length === 1 && theLauncher.said.length === 0,
            JSON.stringify([Sessions.steered, theLauncher.said]));
        Sessions.puts = true;
        inside[0].run();
        const steered = Sessions.steered[1] ?? {};
        expect("its Return steers the session the answer, and once put the foot says so",
            Sessions.steered.length === 2 && steered.address === "WANTE"
                && steered.fields.act === "answer" && steered.fields.value === "push"
                && steered.fields.want === "question@2026-11-12T07:00:00.000Z"
                && steered.fields.from === "joel"
                && JSON.stringify(theLauncher.said) === JSON.stringify([["answered Yes, push it",
                    ""]]),
            JSON.stringify([Sessions.steered, theLauncher.said]));
        inside[3].given("after lunch");
        expect("his own words go as words",
            Sessions.steered[2]?.fields.other === "after lunch"
                && Sessions.steered[2]?.fields.value === undefined,
            JSON.stringify(Sessions.steered[2]));
        Sessions.steered = [];
        theLauncher.said = [];
        Sessions.wanting = [];
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
        Facts.all = [lately("mail", "Ann", () => ran = "mail"), lately("slack", "Bo", () => {})];
        Notices.history = told;
        WaitingOnYou.now = [toDo("Sign In")];
        WaitingOnYou.later = [toDo("Renew It")];
        Sessions.wanting = [session(false, "Which?", "d1")];
        const forYou = () => now.waiting.verbs();
        const byKey = rows => rows.reduce((keyed, row) => Object.assign(keyed,
            { [row.key]: row }), {});
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
        Notices.history = [notice(1)].concat(told);
        before = now.tally.rows;
        const came = byKey(forYou());
        const remade = Object.keys(came).filter(key => came[key] !== first[key]);
        expect("one notice arriving makes only Desktop's row again",
            now.tally.rows - before === 1 && JSON.stringify(remade)
                === JSON.stringify(["on:waiting:desktop"]) && now.waiting === waitingRow,
            JSON.stringify([now.tally.rows - before, remade]));
        const desktop = () => now.waiting.verbs().find(row => row.key === "on:waiting:desktop")
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
        WaitingOnYou.now = [toDo("Sign In Again")];
        expect("a to-do said otherwise, its count the same, keeps For You's own row",
            now.waiting === waitingRow && root.nowChanged === 0, root.nowChanged);
    }

    function heldHears() {
        expect("and For You held hears it once the change is done", root.nowChanged === 1,
            root.nowChanged);
        theLauncher.held = [];
        Facts.all = [];
        Notices.history = [];
        WaitingOnYou.now = [];
        WaitingOnYou.later = [];
        Sessions.wanting = [];
    }
}
