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
// their own. A meeting's row and everything done inside it are the google
// daemon's (Meetings.qml); `row` stands only where it serves none.
// `lately` are the meetings held lately and `called` their Meet calls,
// which Calendar's Recent and Meet's list. `calls` are the Meet calls open
// in windows of their own, each with its meeting, which Call.qml says
// while one is.
import qs
import Quickshell
import QtQuick
import "Calendar.js" as Said
import "Google.js" as Account
import "Meet.js" as Calls

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

    // The meetings Joel held lately, newest first (Google.js's
    // heldLately), for Calendar's Recent (Meetings.qml), reckoned again
    // each minute as `now` is, so one that ends joins it then; and those
    // of them on Meet, each once by its code, for Meet's (Meet.qml).
    readonly property var lately: Account.heldLately(Google.held, meetings, now)
    readonly property var called: Calls.called(lately)
    // The same as Calendar's Recent lists them, a series once at its
    // latest, saying how many came before (Google.js's seriesOnce).
    readonly property var had: Account.seriesOnce(lately)

    // When a meeting is, as its row says it: "in 12 min", "at 16:00", and
    // for one over when it was held, "yesterday 15:00", in the words mail
    // says when in.
    function when(meeting) {
        return Said.when(meeting, now, Mail.dates);
    }

    // Whether Return on a meeting joins its call, as it does only when the
    // call is on or starts within ten minutes; otherwise Return shows it.
    // Every row that stands for a meeting asks this, so none differs.
    function joinable(meeting) {
        return Said.joinable(meeting, now);
    }

    // Whether another notice already told Joel what the shell's would by
    // now, such as Calendar's own page in Chrome at the same reminder, so
    // the shell doesn't say it twice.
    function toldElsewhere(meeting, notices) {
        return Said.toldElsewhere(meeting, notices, now);
    }

    // Whether Now holds it as the next meeting (Calendar.js's upNext).
    function upNext(meeting) {
        return Said.upNext(meeting, now);
    }

    // Its times: "14:00 to 14:30", or "all day".
    function span(meeting) {
        return Said.span(meeting);
    }

    // The meeting that wants Joel (Starting.qml): the first coming within
    // the hour, or from its first reminder, until five minutes after its
    // start, whose call isn't open in a window of its own, when it's on air
    // instead (Call.qml). A call joined through Tuple isn't seen here.
    readonly property var wanting: Said.wanting(meetings.filter(meeting => !calls.some(
        call => call.meeting !== null && call.meeting.id === meeting.id)), now)
    // Whether its mark is in the margin, from ten minutes before it starts.
    readonly property bool marked: Said.marked(wanting, now)
    // What its notice tells by now: its last reminder come, or its start.
    readonly property string telling: Said.telling(wanting, now)

    // A meeting's row where the google daemon serves none, as between its
    // meetings and its rows arriving, or for a meeting held that Calendar's
    // Recent leaves out (GoogleSource.qml, Meet.qml): its title, when it is
    // and its details, as the served row has them. Return joins its call
    // when it's on or starts within ten minutes (`joinable`), and otherwise
    // opens its page in Google Calendar, where everything else is done,
    // since answering, moving and cancelling are the daemon's to ask.
    function row(meeting, provider) {
        const joins = root.joinable(meeting);
        const name = root.titled(meeting);
        return provider.thing({
            key: `calendar:meeting:${meeting.id}`,
            what: "meeting",
            component: "meeting",
            of: "program:google-calendar",
            part: name,
            name: name,
            verb: joins ? Words.verb("verb:calendar:join") : Words.verb("verb:calendar:open"),
            glyph: Words.glyph("calendar:meeting"),
            hue: "program",
            side: root.when(meeting),
            // Whether it's today's, so Calendar's row counts today's
            // meetings while they're listed under it (Applications.qml).
            today: Said.dayOf(meeting.start, root.now) === "",
            // Its times and his answer, or nothing once it's over, where
            // its right says when it was (Calendar.js's noted).
            note: Said.noted(meeting, root.now),
            drawer: root.details(meeting),
            later: true,
            run: joins ? () => root.join(meeting) : () => root.open(meeting),
            verbs: () => root.opening(meeting, provider)
        });
    }

    // What a call in Meet's Recent says beside it: who it was with and
    // when it was held (Meet.js's heldSide), in `said`'s words.
    function calledSide(meeting, said) {
        return Calls.heldSide(meeting, now, said);
    }

    // Its details, as the launcher's drawer shows them beside its row, one
    // of a series in Calendar's Recent saying how many came before it.
    function details(meeting) {
        return Said.details(meeting, Words.name("calendar:meeting", "untitled"),
            Account.earlier(meeting.earlier ?? 0, Words.of("calendar:meeting").said));
    }

    function titled(meeting) {
        return meeting.title !== "" ? meeting.title : Words.name("calendar:meeting", "untitled");
    }

    // A meeting's acts where the daemon serves none (`row`): Join the
    // Meeting when it has a call and Open in Google Calendar when it has a
    // page, the two that need nothing asked of the daemon, each leaving the
    // launcher.
    function opening(meeting, provider) {
        const acts = [];
        const act = (key, run) => acts.push(provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            later: true,
            run: run
        }));
        if (meeting.link)
            act("verb:calendar:join", () => root.join(meeting));
        if (meeting.page)
            act("verb:calendar:open", () => root.open(meeting));
        return acts;
    }

    // A Meet code as Joel types or pastes it, written as Meet writes it, or
    // "" for anything Meet wouldn't know.
    function meetCode(typed) {
        return Calls.meetCode(typed);
    }

    function join(meeting) {
        if (!meeting || !meeting.link)
            return;
        Niri.spawn(["google-opens", meeting.link]);
    }

    // Each Meet call open in a window of its own, as { window, code,
    // meeting }, its meeting null when the call isn't on his calendar.
    // niri's revision says when a window opens, closes or is retitled.
    readonly property var calls: {
        void Niri.revision;
        return Object.values(Niri.windows)
            .map(window => ({ window: window, code: Calls.meetCall(window.app_id) }))
            .filter(call => call.code !== "")
            .map(call => Object.assign(call, { meeting: Calls.calledBy(meetings, call.code) }));
    }
    // Their codes, which the google program follows, asking Meet who's in
    // each, only while the launcher is open to say it (Meetings.qml's
    // callsWatched), and asked again within each minute it is, since the
    // program lets a follow go a little after that, so a shell that went
    // away leaves none behind (Google.qml's follow).
    property int watchingCalls: 0
    readonly property string followed: watchingCalls > 0
        ? calls.map(call => call.code).join(" ") : ""
    onFollowedChanged: Google.follow(followed === "" ? [] : followed.split(" "))

    Timer {
        interval: 60_000
        repeat: true
        running: root.followed !== ""
        onTriggered: Google.follow(root.followed.split(" "))
    }

    // The meeting's own page in Google Calendar.
    function open(meeting) {
        if (meeting && meeting.page)
            Niri.spawn(["google-opens", meeting.page]);
    }
}
