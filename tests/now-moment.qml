// Drives Now's Today row (modules/quickshell/launcher/WhatsOn.qml) through
// Quickshell itself, with the margin's pieces, the agents' waiting list,
// the weather and the calendar stood in. Today is Now's last row, the date
// at its right; the weather is read once the launcher opens and again when
// it hears from Open-Meteo, keeping what it knew while it asks again; with
// a meeting the note says it and Return is the meeting's own, and without
// one Return steps in to the date and the weather; and Now still counts
// nothing while only Today is there. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
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
                rows: [{ name: "Now", side: sky, glyph: "sky" }],
                note: "Lighthouse \u00b7 Open-Meteo"
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

    QtObject {
        id: launcher

        property bool open: false
        readonly property var sources: [weather, calendar]
    }

    Launcher.WhatsOn {
        id: now

        launcher: launcher
        settings: ({ name: "on" })
    }

    Component.onCompleted: Qt.callLater(root.cases)

    // Now's own rows, after what's on: the time, the date, then the
    // weather and the meeting where there are.
    function moment() {
        return now.everything().filter(row => String(row.key).startsWith("on:")
            || row.key === calendar.next?.key);
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
        expect("the time and the date lead Now's own rows, the meeting after them",
            JSON.stringify(names()) === JSON.stringify(["Time", "Date", "Standup"]),
            JSON.stringify(names()));
        expect("the time says the minute", rows[0].side === time, rows[0].side);
        expect("the date says the day", rows[1].side === date, rows[1].side);
        expect("and each copies what it says", rows[0].verb === "copy" && rows[1].verb === "copy",
            `${rows[0].verb} ${rows[1].verb}`);
        expect("nothing counted while only they are there", now.counted() === "nothing",
            now.counted());
        expect("the weather isn't asked before the launcher opens", weather.asked === 0,
            weather.asked);

        launcher.open = true;
        expect("opening reads the weather, before the meeting",
            JSON.stringify(names()) === JSON.stringify(["Time", "Date", "Weather", "Standup"]),
            JSON.stringify(names()));
        rows = moment();
        expect("the weather says the sky", rows[2].side === "7\u00b0 \u00b7 cloudy", rows[2].side);
        const meeting = rows[3];
        expect("the meeting's Return is its own", meeting.verb === "join" && meeting.later === true,
            JSON.stringify(meeting));
        meeting.run();
        expect("Return joins it", calendar.joined, "not joined");

        weather.asking = true;
        weather.answered();
        expect("asking again keeps what it knew", moment()[2].side === "7\u00b0 \u00b7 cloudy",
            moment()[2].side);
        weather.asking = false;
        weather.sky = "9\u00b0 \u00b7 rain";
        weather.answered();
        expect("a forecast heard says the new sky", moment()[2].side === "9\u00b0 \u00b7 rain",
            moment()[2].side);
        expect("the weather's row holds its own parts",
            moment()[2].verbs().map(each => each.name).join() === "Now",
            JSON.stringify(moment()[2]));

        calendar.next = null;
        expect("without a meeting, no meeting row",
            JSON.stringify(names()) === JSON.stringify(["Time", "Date", "Weather"]),
            JSON.stringify(names()));
    }
}
