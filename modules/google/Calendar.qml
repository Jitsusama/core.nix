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
// their own. `row` is a meeting's one row, with its acts and details,
// which every surface showing a meeting takes as it is.
// `answer` answers an invitation through the program (ask.rs),
// which says the meeting answered as soon as Google takes it. `calls` are
// the Meet calls open in windows of their own, each with its meeting,
// which Call.qml says while one is.
import qs
import Quickshell
import QtQuick
import "Calendar.js" as Said
import "Mail.js" as Written
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
    onMeetingsChanged: {
        now = Date.now();
        gave = {};
    }

    // The answer Joel gave each meeting here, by its id, said at once
    // until the program next says how the meetings stand, as Mail's Read
    // is.
    property var gave: ({})

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

    // Whether another notice already told Joel what the shell's would by
    // now, such as Calendar's own page in Chrome at the same reminder, so
    // the shell doesn't say it twice.
    function toldElsewhere(meeting, notices) {
        return Said.toldElsewhere(meeting, notices, now);
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

    // Whether Joel may move a meeting: Google lets him change it, and it
    // isn't all day.
    function movable(meeting) {
        return Said.movable(meeting);
    }

    // Where Move the Meeting offers to take one: [{ name, start, end }].
    function later(meeting) {
        return Said.later(meeting);
    }

    // Which of his other meetings a meeting moved there would overlap.
    function clashes(meeting, start, end) {
        return Said.clashes(meeting, start, end, meetings);
    }

    // When a time typed for a meeting would have it run, or null.
    function typed(text, meeting) {
        return Said.typed(text, meeting, Date.now());
    }

    // Who Google tells of a change to it, as a sure row says it.
    function tellingOfChanges(meeting) {
        return Said.tellingThem(Said.toldOfChanges(meeting));
    }

    // Moves a meeting Google lets Joel change to run from `start` to `end`,
    // through the program, which moves only one it holds as his to change,
    // and Google tells everyone on it.
    function move(meeting, start, end) {
        if (Said.movable(meeting))
            Google.ask(Said.move(meeting, start, end));
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

    // A meeting's row, made with the provider of the surface showing it,
    // whose `thing`, `copyRow` and launcher it uses, so every surface shows
    // the same one: its title, when it is at the right and its times
    // below, with his answer to an invitation, and its details beside it.
    // Return joins its call only when the call is on or starts within ten
    // minutes (`joinable`), which takes Joel out of the launcher; otherwise
    // it sees the meeting, a step in, keeping the launcher, so a meeting
    // he only wants to look at is never joined. The right arrow lists
    // every act. Its names and words are the vocabulary's (calendar:*).
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
            verb: joins ? Words.verb("verb:calendar:join") : Words.verb("verb:calendar:see"),
            glyph: Words.glyph("calendar:meeting"),
            hue: "program",
            side: root.when(meeting),
            note: root.about(meeting),
            drawer: Said.details(meeting, Words.name("calendar:meeting", "untitled")),
            later: joins,
            stays: !joins,
            run: joins ? () => root.join(meeting)
                : () => provider.launcher?.enter(root.seeing(meeting, provider, name)),
            verbs: () => root.acts(meeting, provider)
        });
    }

    function titled(meeting) {
        return meeting.title !== "" ? meeting.title : Words.name("calendar:meeting", "untitled");
    }

    // Its acts, in the one order every surface lists them (VKCF3Z's
    // design, RSCH-20261028-2NK1E5): Join the Meeting when it has a call,
    // Write to the Guests when anyone else is asked, See It, then, for an
    // invitation, Going, Maybe and Not Going, inline since answering is
    // what's done most after joining, each keeping the launcher, the one
    // he gave noted as his answer; for one Google lets him
    // change, Move the Meeting, to a later time or one typed there, which
    // asks once, naming who Google tells; Attachments, the files on it,
    // while it has any; and for one Joel holds, Cancel
    // the Meeting, which asks once, since Google mails everyone asked; and
    // last, as copying always is, Copy the Link.
    function acts(meeting, provider) {
        const rows = [];
        if (meeting.link)
            rows.push(provider.thing({
                key: "verb:calendar:join",
                name: Words.name("verb:calendar:join"),
                verb: Words.verb("verb:calendar:join"),
                glyph: Words.glyph("verb:calendar:join"),
                hue: "action",
                later: true,
                run: () => root.join(meeting)
            }));
        const writing = Written.toTheGuests(meeting);
        if (writing !== "")
            rows.push(provider.thing({
                key: "verb:calendar:write",
                name: Words.name("verb:calendar:write"),
                verb: Words.verb("verb:calendar:write"),
                glyph: Words.glyph("verb:calendar:write"),
                hue: "action",
                later: true,
                run: () => Niri.spawn(["google-opens", writing])
            }));
        rows.push(root.seeing(meeting, provider));
        if (root.invited(meeting))
            for (const said of ["yes", "maybe", "no"])
                rows.push(root.answering(meeting, provider, said));
        if (root.movable(meeting))
            rows.push(root.moving(meeting, provider));
        if ((meeting.attachments ?? []).length > 0)
            rows.push(root.attaching(meeting, provider));
        if (meeting.holds)
            rows.push(root.cancelling(meeting, provider));
        if (meeting.link || meeting.page)
            rows.push(root.linking(meeting, provider));
        return rows;
    }

    // Attachments steps in to the files on it, as mail's does (VKCF3Z,
    // 6abd5c3e), counting them as Recent does: each of Docs, Sheets or
    // Slides as the document it is (Drive.qml), the one from Recent when
    // he's opened it lately, which says when, and any other file a row
    // opening its link in Google's window.
    function attaching(meeting, provider) {
        const key = "verb:calendar:attachments";
        const files = meeting.attachments;
        const inside = () => files.map(file => root.attached(file, provider));
        const row = provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            count: String(files.length),
            stays: true,
            verbs: inside,
            targets: inside,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    function attached(file, provider) {
        if (file.id && file.kind)
            return Drive.row(Drive.documents.find(document => document.id === file.id) ?? {
                id: file.id,
                title: file.title,
                kind: file.kind,
                link: file.link,
                viewed: null,
                changed: null,
                owner: null
            }, provider);
        const key = "calendar:attachment";
        return provider.thing({
            key: `${key}:${file.link}`,
            name: file.title,
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "program",
            later: true,
            run: () => Niri.spawn(["google-opens", file.link])
        });
    }

    // Copy the Link copies its call's link, what others join it by, or,
    // for a meeting with no call, its page in Google Calendar; the side
    // says which, as See It's facts do.
    function linking(meeting, provider) {
        const key = "verb:calendar:link";
        const link = meeting.link || meeting.page;
        return provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            side: meeting.link ? "its call" : "its page",
            hue: "action",
            now: true,
            run: () => {
                Quickshell.clipboardText = link;
            }
        });
    }

    // Move the Meeting steps in to the later times it offers, each saying
    // its day and times and the meeting it would clash with, and to one
    // typed there, which Launcher.qml asks for (`typed`) and lists first.
    // Return on a time steps in to a row naming who Google tells, and a
    // second Return moves it, keeping the launcher, as Cancel does.
    function moving(meeting, provider) {
        const key = "verb:calendar:move";
        return provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            targets: () => root.later(meeting)
                .map(time => root.movingTo(meeting, provider, time.name, time.start, time.end)),
            typed: letters => {
                const time = root.typed(letters, meeting);
                return time === null ? [] : [root.movingTo(meeting, provider,
                    root.dated({ start: time.start, end: time.end, allDay: false }),
                    time.start, time.end)];
            }
        });
    }

    // A time to move a meeting to, keyed by when it starts, so the row
    // holds still as more is typed.
    function movingTo(meeting, provider, name, start, end) {
        const key = "verb:calendar:move";
        const dated = root.dated({ start: start, end: end, allDay: false });
        const clash = root.clashes(meeting, start, end);
        return provider.thing({
            key: `calendar:move:${new Date(start).toISOString()}`,
            name: name,
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: clash === null ? "" : `clashes with ${clash.title
                || Words.name("calendar:meeting", "untitled")}`,
            // A typed time is named by its day and times already.
            note: name === dated ? "" : dated,
            targets: () => [provider.thing({
                key: `${key}:sure`,
                name: `${Words.of(key).sure} to ${dated}, ${root.tellingOfChanges(meeting)}`,
                verb: Words.verb(key),
                glyph: Words.glyph(key),
                hue: "alert",
                stays: true,
                run: () => root.move(meeting, start, end)
            })]
        });
    }

    // Cancel asks once, as Quit does (Applications.qml): Return steps in
    // to a row saying who's told, and a second Return cancels it and
    // leaves the launcher open on what's left.
    function cancelling(meeting, provider) {
        const key = "verb:calendar:cancel";
        const asked = (meeting.guests ?? []).length;
        const told = asked === 0 ? "Telling Nobody Else"
            : asked === 1 ? "Telling 1 Guest" : `Telling ${asked} Guests`;
        return provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            targets: () => [provider.thing({
                key: `${key}:sure`,
                name: `${Words.of(key).sure}, ${told}`,
                verb: Words.verb(key),
                glyph: Words.glyph(key),
                hue: "alert",
                stays: true,
                run: () => root.cancel(meeting)
            })]
        });
    }

    // See It: the meeting's details a step in, made as it's stepped into,
    // so they're what the program last said. Stepped into by the meeting's
    // own Return, it's named for the meeting, so the path says which.
    function seeing(meeting, provider, name) {
        const key = "verb:calendar:see";
        const inside = () => root.facts(root.meeting(meeting.id) ?? meeting, provider);
        const row = provider.thing({
            key: key,
            name: name ?? Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            verbs: inside,
            targets: inside,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // What's known of it, each a figure that copies what it says, a
    // person their address, then its page in Google Calendar, where it's
    // changed. What it's about is one line here, copied whole.
    function facts(meeting, provider) {
        const rows = [provider.copyRow({
            key: "fact:meeting:when",
            name: root.dated(meeting),
            side: "when"
        })];
        if (meeting.place)
            rows.push(provider.copyRow({
                key: "fact:meeting:place",
                name: meeting.place,
                side: "where"
            }));
        if (root.invited(meeting))
            rows.push(provider.copyRow({
                key: "fact:meeting:answer",
                name: root.answered(meeting),
                copy: "",
                side: "your answer"
            }));
        if (meeting.link)
            rows.push(provider.copyRow({
                key: "fact:meeting:call",
                name: meeting.link,
                side: "its call"
            }));
        if (meeting.organizer)
            rows.push(provider.copyRow({
                key: "fact:meeting:organizer",
                name: meeting.organizer.name,
                copy: meeting.organizer.email ?? meeting.organizer.name,
                side: "holds it"
            }));
        (meeting.guests ?? []).forEach((guest, at) => rows.push(provider.copyRow({
            key: `fact:meeting:guest:${at}`,
            name: guest.name,
            copy: guest.email ?? guest.name,
            side: root.answered({ answer: guest.answer })
        })));
        if (meeting.about)
            rows.push(provider.copyRow({
                key: "fact:meeting:about",
                name: meeting.about.replace(/\s*\n\s*/g, " "),
                copy: meeting.about,
                side: "what it's about"
            }));
        if (meeting.page)
            rows.push(provider.thing({
                key: "verb:calendar:open",
                name: Words.name("verb:calendar:open"),
                verb: Words.verb("verb:calendar:open"),
                glyph: Words.glyph("verb:calendar:open"),
                hue: "action",
                later: true,
                run: () => root.open(meeting)
            }));
        return rows;
    }

    // An answer is said by its value alone, the one he gave noted "your
    // answer" and none while he hasn't answered (VKCF3Z, 8e3e0a3e). It
    // keeps the launcher, since he's choosing among his meetings, its
    // note moving at once, and the foot says the answer the program last
    // gave.
    function answering(meeting, provider, said) {
        const key = `verb:calendar:${said}`;
        const row = provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => root.answered(root.meeting(meeting.id)),
            run: () => {
                root.gave = Object.assign({}, root.gave, { [meeting.id]: said });
                root.answer(meeting, said);
            }
        });
        // Read as it's shown, since thing() would copy it once.
        return Object.defineProperty(row, "side", {
            enumerable: true,
            get: () => {
                const given = root.gave[meeting.id] ?? root.meeting(meeting.id)?.answer;
                return given === said ? "your answer" : "";
            }
        });
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
        Niri.revision;
        return Object.values(Niri.windows)
            .map(window => ({ window: window, code: Calls.meetCall(window.app_id) }))
            .filter(call => call.code !== "")
            .map(call => Object.assign(call, { meeting: Calls.calledBy(meetings, call.code) }));
    }

    // The meeting's own page in Google Calendar.
    function open(meeting) {
        if (meeting && meeting.page)
            Niri.spawn(["google-opens", meeting.page]);
    }
}
