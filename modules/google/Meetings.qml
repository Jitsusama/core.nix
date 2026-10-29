// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from today
// to a week on as Calendar.qml holds them, so Programs › Calendar lists
// them after the program's own acts, and a meeting's name finds it from
// anywhere. Each is the meeting's one row (Calendar.row), the same as
// every other surface shows it, with the same acts and details. `next` is
// the next meeting's row, which Now shows.
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
    // account is ready and the meeting is on or starts today, or else
    // null, as on a machine without google, where this provider isn't
    // there at all.
    readonly property var next: Google.account === "ready" && Calendar.today(Calendar.next)
        ? things.find(row => row.key === `calendar:meeting:${Calendar.next.id}`) ?? null
        : null
}
