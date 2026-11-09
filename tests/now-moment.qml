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
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
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

    // The launcher's placing of an answered thing, as Launcher.qml's
    // resolved gives it: named by the thing, what it says beside, its
    // parts a step in. Each row it gives is marked, so the test knows Now
    // showed the launcher's row and not one of its own.
    QtObject {
        id: theLauncher

        property bool open: false
        readonly property var sources: [weather, calendar, cameras, power]
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

    // The cases run once whereabouts has answered, with here and the
    // clock's zone, which it does apart from the shell's start.
    readonly property bool ready: Location.read && Location.clockZone !== ""
    property bool started: false
    onReadyChanged: begin()
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
        let status = 0;
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            status = 1;
        }
        Qt.callLater(() => Qt.exit(status));
    }

    function run() {
        const time = Qt.formatDateTime(new Date(), "HH:mm");
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
        expect("the time says the minute", rows[0].side === time, rows[0].side);
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

        // The location says its town, and that the place keeps another zone
        // than the clock, in the colour of what wants a look.
        const location = rows[2];
        expect("the location says its town", location.side === "Kelpwater, Westmark",
            location.side);
        expect("and that the place keeps another zone",
            location.note === "keeps America/Toronto \u00b7 not following"
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
        // Found, since the record doesn't say whether from Wi-Fi or the
        // network's address, and naming no daemon (VKCF3Z, 2026-11-09).
        expect("it knows it found where it is, naming no source it can't tell",
            inside[2].side === "found, within 26 km", inside[2].side);
        // Its plate: how it was found, its town, then when, how near, where
        // on a map to no more places than it knows, its country and its
        // zone last as the identifier it is (VKCF3Z, 2026-11-08).
        const plate = location.thing === true ? location.drawer() : null;
        expect("its plate says it was found, titled by its town",
            plate?.eyebrow === "found \u00b7 location" && plate?.title === "Kelpwater",
            JSON.stringify(plate));
        expect("its plate's facts, from what moves most, the zone an identifier",
            JSON.stringify(plate.facts.map(fact => [fact.label, fact.identifies]))
                === JSON.stringify([["Located", false], ["Accuracy", false],
                    ["Coordinates", false], ["Country", false], ["Zone", true]]),
            JSON.stringify(plate.facts));
        expect("its plate names what it can't say, and why",
            plate.body === "Neither Wi-Fi nor a network's address gives altitude, speed or "
                + "heading.", plate.body);
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
    }
}
