// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from today
// to a week on as Calendar.qml holds them, so Programs › Calendar lists
// them after the program's own acts, and a meeting's name finds it from
// anywhere. Each is the meeting's one row (Calendar.row), the same as
// every other surface shows it, with the same acts and details. `next` is
// the next meeting's row, which Now shows. Calendar's own Recent is the
// meetings Joel had in the last month (recentOf, which Applications.qml
// asks every provider that has it), as Google.js lays them out, each its
// one row too.
import qs
import QtQuick
import "Google.js" as Had

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

    readonly property var next: Google.account === "ready" && Calendar.upNext(Calendar.next)
        ? things.find(row => row.key === `calendar:meeting:${Calendar.next.id}`) ?? null
        : null

    // The desktop entry whose Recent this gives (home.nix).
    readonly property string program: "google-calendar"

    // Calendar's Recent: each meeting he had in the month, those held
    // before today and today's that are over, newest first, a series once
    // at its latest, each with when it ended, so Recent.js merges it by
    // time. Made only as it's asked, as Calendar is stepped into.
    function recentOf(asked) {
        if (asked !== program)
            return [];
        return Had.hadMeetings(Google.held.concat(Calendar.meetings), Calendar.now)
            .map(one => Object.assign(Calendar.row(one.meeting, provider), { at: one.at }));
    }

    // Whether a program's Recent is this one's to give, so Calendar's
    // stands, saying "none", while it holds nothing.
    function keepsRecentOf(asked) {
        return asked === program;
    }
}
