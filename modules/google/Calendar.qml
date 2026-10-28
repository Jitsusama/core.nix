pragma Singleton
pragma ComponentBehavior: Bound
// Joel's meetings, as the shell knows them, for the launcher's Today row
// and its Calendar program: those the google program keeps of his Google
// account, as Google.qml reads them. With no account there are none, and
// nothing shows.
//
// `next` is the meeting he's about to go to, or the one going on, the same
// one for the Today row and Now's wants (Calendar.js says which);
// `joinable` says whether Return on any of them joins it; and
// `join` goes into a meeting's call through google-opens (opens.sh), which
// joins a Tuple link through Tuple and opens Google's own in a window of
// their own. `answer` answers an invitation through the program (ask.rs),
// which says the meeting answered as soon as Google takes it. `calls` are
// the Meet calls open in windows of their own, each with its meeting,
// which Call.qml says while one is.
import qs
import Quickshell
import QtQuick
import "Calendar.js" as Said

Singleton {
    id: root

    // Each { id, title, start, end, allDay, answer, link, kind, page },
    // in milliseconds since 1970, from today's start to a week on.
    readonly property var meetings: Google.meetings

    // The clock the next meeting is reckoned by, a minute at a time, as a
    // meeting only ever comes or goes on one, and at once when the
    // meetings change, so one heard after a while away isn't reckoned by
    // the minute before.
    property real now: Date.now()
    onMeetingsChanged: now = Date.now()

    Timer {
        interval: 60 * 1000 - Date.now() % (60 * 1000)
        running: true
        repeat: true
        onTriggered: {
            root.now = Date.now();
            interval = 60 * 1000 - root.now % (60 * 1000);
        }
    }

    readonly property var next: Said.next(meetings, now)

    // When a meeting is, as its row says it: "in 12 min", "at 16:00".
    function when(meeting) {
        return Said.when(meeting, now);
    }

    // Whether Return on a meeting joins its call, as it does only when the
    // call is on or starts within ten minutes; otherwise Return shows it.
    // Every row that stands for a meeting asks this, so none differs.
    function joinable(meeting) {
        return Said.joinable(meeting, now);
    }

    // Whether another notice already told Joel of it, such as Calendar's
    // own page in Chrome, so the shell doesn't say it twice.
    function toldElsewhere(meeting, notices) {
        return Said.toldElsewhere(meeting, notices);
    }

    // Whether it's on or starts today.
    function today(meeting) {
        return Said.today(meeting, now);
    }

    // Its day and times in full: "Monday 26 October, 14:00 to 14:30".
    function dated(meeting) {
        return Said.dated(meeting);
    }

    // Its times: "14:00 to 14:30", or "all day".
    function span(meeting) {
        return Said.span(meeting);
    }

    // Its times and, for an invitation, his answer: "14:00 to 14:30, maybe".
    function about(meeting) {
        return Said.about(meeting);
    }

    // His answer as he'd say it, "going", or nothing for one he holds.
    function answered(meeting) {
        return Said.answered(meeting);
    }

    function invited(meeting) {
        return Said.invited(meeting);
    }

    // The meeting `id` as it's held now, so what's said of one picked a
    // while ago is what the program last said.
    function meeting(id) {
        return meetings.find(each => each.id === id) ?? null;
    }

    // Answers the invitation yes, maybe or no, through the program, which
    // is running whenever there's a meeting to answer.
    function answer(meeting, said) {
        if (meeting)
            Google.ask(Said.answer(meeting, said));
    }

    // Cancels a meeting Joel holds, through the program, which cancels
    // only one it holds as his, and Google tells every guest.
    function cancel(meeting) {
        if (meeting && meeting.holds)
            Google.ask(Said.cancel(meeting));
    }

    // Whether the next meeting is about to start, for Now's wants.
    readonly property bool starting: Said.aboutToStart(next, now)
    // The next meeting while it wants Joel (Starting.qml): about to start,
    // and its call not yet open in a window of its own, when it's on air
    // instead (Call.qml). A call joined through Tuple isn't seen here.
    readonly property var wanting: starting
        && !calls.some(call => call.meeting !== null && call.meeting.id === next.id) ? next : null
    // Whether it's said as a notice by now, a minute before it starts.
    readonly property bool heralded: wanting !== null && Said.heralded(wanting, now)

    function join(meeting) {
        if (!meeting || !meeting.link)
            return;
        Niri.spawn(["google-opens", meeting.link]);
    }

    // Each Meet call open in a window of its own, as { window, code,
    // meeting }, its meeting null when the call isn't on his calendar.
    // niri's revision says when a window opens, closes or is retitled.
    readonly property var calls: {
        Niri.revision;
        return Object.values(Niri.windows)
            .map(window => ({ window: window, code: Said.meetCall(window.app_id) }))
            .filter(call => call.code !== "")
            .map(call => Object.assign(call, { meeting: Said.calledBy(meetings, call.code) }));
    }

    // The meeting's own page in Google Calendar.
    function open(meeting) {
        if (meeting && meeting.page)
            Niri.spawn(["google-opens", meeting.page]);
    }
}
