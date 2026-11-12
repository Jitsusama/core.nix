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
// which every surface showing a meeting takes as it is. `lately` are the
// meetings held lately and `called` their Meet calls, which Calendar's
// Recent and Meet's list.
// `answer` answers an invitation through the program (ask.rs),
// which says the meeting answered as soon as Google takes it. `calls` are
// the Meet calls open in windows of their own, each with its meeting,
// which Call.qml says while one is.
import qs
import Quickshell
import QtQuick
import "Calendar.js" as Said
import "Google.js" as Account
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

    // Its day and times in full: "Monday 26 October \u00b7 14:00 to 14:30".
    function dated(meeting) {
        return Said.dated(meeting);
    }

    // Its times: "14:00 to 14:30", or "all day".
    function span(meeting) {
        return Said.span(meeting);
    }

    // Its times and, for an invitation, his answer: "14:00 to 14:30 \u00b7 maybe".
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

    // Where Move the Meeting offers to take one: [{ name, start, end }],
    // named as the vocabulary names each.
    function later(meeting) {
        return Said.later(meeting).map(time => Object.assign({
            name: Words.name("verb:calendar:move", time.state)
        }, time));
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
            // Whether it's today's, so Calendar's row counts today's
            // meetings while they're listed under it (Applications.qml).
            today: Said.dayOf(meeting.start, root.now) === "",
            // Its times and his answer, or nothing once it's over, where
            // its right says when it was (Calendar.js's noted).
            note: Said.noted(meeting, root.now),
            drawer: root.details(meeting),
            later: joins,
            stays: !joins,
            run: joins ? () => root.join(meeting)
                : () => provider.launcher?.enter(root.seeing(meeting, provider, name)),
            verbs: () => root.acts(meeting, provider)
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

    // Its acts, in the one order every surface lists them (VKCF3Z's
    // design, RSCH-20261028-2NK1E5): Join the Meeting when it has a call,
    // Write to the Guests when anyone else is asked, Show in Calendar, Open the
    // Transcript once its call has been written down, then, for an
    // invitation, Going, Maybe and Not Going, inline since answering is
    // what's done most after joining, each keeping the launcher, the one
    // he gave noted as his answer; for one Google lets him
    // change, Move the Meeting, to a later time or one typed there, which
    // asks once, naming who Google tells; Guests, who holds it and who's
    // asked, each stepping through to the person, when People knows any;
    // Attachments, the files on it, while it has any; and for one Joel
    // holds, Cancel
    // the Meeting, which asks once, since Google mails everyone asked; and
    // last, as copying always is, Copy the Link. They stand only for a
    // meeting the daemon doesn't serve, which lists the same acts as rows
    // of its own (Meetings.qml).
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
        const transcript = root.transcribing(meeting, provider);
        if (transcript)
            rows.push(transcript);
        if (root.invited(meeting))
            for (const said of ["yes", "maybe", "no"])
                rows.push(root.answering(meeting, provider, said));
        if (root.movable(meeting))
            rows.push(root.moving(meeting, provider));
        const guests = root.guesting(meeting, provider);
        if (guests)
            rows.push(guests);
        if ((meeting.attachments ?? []).length > 0)
            rows.push(root.attaching(meeting, provider));
        if (meeting.holds)
            rows.push(root.cancelling(meeting, provider));
        if (meeting.link || meeting.page)
            rows.push(root.linking(meeting, provider));
        return rows;
    }

    // Open the Transcript, for a meeting whose call call-transcripts wrote
    // down: it opens the transcript in the reader, closing the launcher,
    // as Return on the call's own row does, with its summary, or its
    // first lines, in the drawer. It's asked of any provider holding `transcriptOf`
    // (call-transcripts' Transcribed.qml), so a machine without
    // call-transcripts offers none; nor does a call with nothing yet
    // written. It stands only for a meeting the daemon doesn't serve,
    // whose Open the Transcript is the row call-transcripts lends.
    function transcribing(meeting, provider) {
        const calls = (provider.launcher?.sources ?? []).find(source => typeof source.transcriptOf
            === "function");
        const call = calls ? calls.transcriptOf(meeting.id) : null;
        if (!call || call.verb === "more")
            return null;
        const key = "verb:calendar:transcript";
        return provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            later: true,
            run: call.run,
            drawer: call.drawer
        });
    }

    // Guests steps in to who holds it, unless that's Joel, and then each
    // guest, as People's rows, so each steps through to the person (VKCF3Z's
    // design: the guests, before Attachments), counting those on it, so it's
    // there as the meeting's other rows are, each guest's name filling in
    // as People knows them; none with nobody on it, or no People. Every
    // place showing a meeting steps into the daemon's served Guests instead
    // (Meetings.qml's lent), where this marks the place Meetings puts them,
    // and stands only for a meeting not served yet; it goes once every act
    // inside a meeting is the daemon's, by Sun 15 Nov 18:00, as do
    // attaching() and moving() below.
    function guesting(meeting, provider) {
        const emails = Said.guestEmails(meeting);
        const people = () => {
            const found = [];
            for (const email of emails) {
                const person = Google.personOf(provider.launcher, email);
                if (person && !found.some(one => one.key === person.key))
                    found.push(person);
            }
            return found;
        };
        // Counted by the google daemon, each address once
        // (calendar_rows.rs), so it's there with the meeting's other rows.
        const count = meeting.guestsCounted ?? 0;
        if (count === 0)
            return null;
        const key = "verb:calendar:guests";
        const row = provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "person",
            count: String(count),
            stays: true,
            verbs: people,
            targets: people,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // Attachments steps in to the files on it, as mail's does (VKCF3Z,
    // 6abd5c3e), counting them as Recent does: each of Docs, Sheets or
    // Slides as the document it is (Drive.qml), the one from Recent when
    // he's opened it lately, which says when, and any other file a row
    // opening its link in Google's window. The daemon's served Attachments
    // take its place wherever a meeting shows; this goes with guesting().
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
    // says which, as Show in Calendar's facts do.
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
    // second Return moves it, keeping the launcher, as Cancel does. The
    // daemon serves the later times wherever a meeting shows; this goes
    // with guesting(), and Calendar.js's typed() once the daemon answers it.
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
            side: clash === null ? "" : Words.of(key).said.clashes.replace("{title}",
                clash.title || Words.name("calendar:meeting", "untitled")),
            // A typed time is named by its day and times already.
            note: name === dated ? "" : dated,
            targets: () => [provider.thing({
                key: `${key}:sure`,
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
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
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
                name: `${Words.of(key).sure}, ${told}`,
                verb: Words.verb(key),
                glyph: Words.glyph(key),
                hue: "alert",
                stays: true,
                run: () => root.cancel(meeting)
            })]
        });
    }

    // Show in Calendar: the meeting's details a step in, made as it's stepped into,
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
        return provider.gotten(row, {
            side: () => {
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
