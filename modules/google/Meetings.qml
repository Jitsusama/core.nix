// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from today
// to a week on as Calendar.qml holds them, so Programs › Calendar lists
// them after the program's own acts, and a meeting's name finds it from
// anywhere. Each is the meeting's one row (Calendar.row), the same as
// every other surface shows it, with the same acts and details. `next` is
// the next meeting's row, which Now shows.
//
// Calendar's Recent (recentOf, which Applications.qml asks every provider
// that has it) is the meetings Joel held lately, newest first by when
// each started (Calendar.lately), each that same row, which says at its
// side when it was held, as the launcher says when anywhere ("yesterday",
// "14:00"), and its times below. Return does what it does on any meeting
// over: it steps in to Show in Calendar, since it can't be joined; the
// right arrow lists every act. It stands, "none" while it holds nothing
// (keepsRecentOf), since the program keeps a month of them.
import qs
import QtQuick

Provider {
    id: provider

    what: "meeting"

    readonly property var shown: Calendar.meetings.filter(meeting => meeting.end > Calendar.now)

    // Made again each minute, as `shown` is, so Return turns to joining
    // once a meeting is ten minutes off.
    things: shown.map(meeting => Calendar.row(meeting, provider))
    onThingsChanged: changedTo(things)

    // The next meeting's row, for Now, which finds this provider by its
    // name and shows the same row, with the same acts: only while the
    // account is ready and the meeting is on, starts today or starts
    // within the hour, or else null, as on a machine without google, where
    // this provider isn't there at all.
    // While the launcher is open, who's in each Meet call is asked of Meet,
    // so Now's call says it (Calendar.qml's watchingCalls); never while
    // it's closed.
    readonly property bool callsWatched: launcher !== null && launcher.open === true
    onCallsWatchedChanged: Calendar.watchingCalls += callsWatched ? 1 : -1

    // As many as a Recent lists (launcher/Recent.js), so none is made only
    // to be dropped.
    readonly property int most: 9

    function recentOf(program) {
        if (program !== "google-calendar")
            return [];
        return Calendar.lately.slice(0, most).map(meeting => Object.assign(
            Calendar.row(meeting, provider), { at: meeting.start }));
    }

    function keepsRecentOf(program) {
        return program === "google-calendar";
    }

    // So Programs › Calendar, held as a meeting ends or its side moves on,
    // draws its Recent again, and only then.
    readonly property string recentSaid: JSON.stringify(Calendar.lately.slice(0, most)
        .map(meeting => [meeting.id, Calendar.when(meeting)]))
    onRecentSaidChanged: changed()

    readonly property var next: Google.account === "ready" && Calendar.upNext(Calendar.next)
        ? things.find(row => row.key === `calendar:meeting:${Calendar.next.id}`) ?? null
        : null
}
