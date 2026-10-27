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

    function today() {
        const rows = now.everything();
        return rows[rows.length - 1];
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
        const date = Qt.formatDate(new Date(), "dddd d MMMM");
        let row = today();
        expect("Today is Now's last row", row.name === "Today" && row.key === "on:today",
            JSON.stringify(row));
        expect("Today says the date at its right", row.side === date, row.side);
        expect("nothing counted while only Today is there", now.counted() === "nothing",
            now.counted());
        expect("the weather isn't asked before the launcher opens", weather.asked === 0,
            weather.asked);
        expect("the note says the meeting before the weather is read",
            row.note === "Standup in 12 min", row.note);

        launcher.open = true;
        row = today();
        expect("opening reads the weather, then the meeting",
            row.note === "7\u00b0 \u00b7 cloudy \u00b7 Standup in 12 min", row.note);
        expect("Return is the meeting's", row.verb === "join" && row.later === true,
            JSON.stringify(row));
        row.run();
        expect("Return joins it", calendar.joined, "not joined");
        const inside = row.verbs().map(each => each.name);
        expect("a step in holds the date, the weather and the meeting",
            JSON.stringify(inside) === JSON.stringify(["Date", "Weather", "Standup"]),
            JSON.stringify(inside));

        weather.asking = true;
        weather.answered();
        expect("asking again keeps what it knew",
            today().note === "7\u00b0 \u00b7 cloudy \u00b7 Standup in 12 min", today().note);
        weather.asking = false;
        weather.sky = "9\u00b0 \u00b7 rain";
        weather.answered();
        expect("a forecast heard says the new sky",
            today().note === "9\u00b0 \u00b7 rain \u00b7 Standup in 12 min", today().note);

        calendar.next = null;
        row = today();
        expect("without a meeting, only the weather", row.note === "9\u00b0 \u00b7 rain",
            row.note);
        expect("and Return steps in", row.verb === "more", row.verb);
        const weatherRow = row.verbs()[1];
        expect("the weather's row holds its own parts",
            weatherRow.verbs().map(each => each.name).join() === "Now",
            JSON.stringify(weatherRow));
    }
}
