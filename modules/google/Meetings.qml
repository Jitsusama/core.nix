// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from today
// to a week on as Calendar.qml holds them, so Programs › Calendar lists
// them after the program's own acts, and a meeting's name finds it from
// anywhere. Return joins a meeting's call only when the call is on or
// starts within ten minutes (Calendar.joinable), which takes Joel out of
// the launcher; otherwise it shows the meeting, a step in, keeping the
// launcher, so a meeting he only wants to look at is never joined. The
// right arrow lists Join the Meeting when it has a call, See It, then,
// for an invitation, Answer Yes, Answer Maybe and Answer No, which keep
// the launcher, its foot saying the answer once Google has it. Seeing it
// says its day and times, his answer and its call, and opens it in
// Calendar. Each row says when the meeting is at the right and its times
// below, with his answer to an invitation; its names and words are the
// vocabulary's (calendar:*, Words.qml). `next` is the next meeting's row,
// which Now's Today row shows.
import qs
import QtQuick

Provider {
    id: provider

    what: "meeting"

    readonly property var shown: Calendar.meetings.filter(meeting => meeting.end > Calendar.now)

    // Made again each minute, as `shown` is, so Return turns to joining
    // once a meeting is ten minutes off.
    things: shown.map(meeting => made(meeting))
    onThingsChanged: changedTo(things)

    // The next meeting's row, for Now's Today row, which finds this
    // provider by its name and shows the same row, with the same acts:
    // only while the account is ready and the meeting is on or starts
    // today, or else null, as on a machine without google, where this
    // provider isn't there at all.
    readonly property var next: Calendar.account === "ready" && Calendar.today(Calendar.next)
        ? things.find(row => row.key === `calendar:meeting:${Calendar.next.id}`) ?? null
        : null

    function made(meeting) {
        const joins = Calendar.joinable(meeting);
        const name = meeting.title !== "" ? meeting.title
            : Words.name("calendar:meeting", "untitled");
        return thing({
            key: `calendar:meeting:${meeting.id}`,
            of: "program:google-calendar",
            part: name,
            name: name,
            verb: joins ? Words.verb("verb:calendar:join") : Words.verb("verb:calendar:see"),
            glyph: Words.glyph("calendar:meeting"),
            hue: "program",
            side: Calendar.when(meeting),
            note: Calendar.about(meeting),
            later: joins,
            stays: !joins,
            run: joins ? () => Calendar.join(meeting)
                : () => provider.launcher?.enter(provider.seeing(meeting, name)),
            verbs: () => provider.verbs(meeting)
        });
    }

    function verbs(meeting) {
        const rows = [];
        if (meeting.link)
            rows.push(thing({
                key: "verb:calendar:join",
                name: Words.name("verb:calendar:join"),
                verb: Words.verb("verb:calendar:join"),
                glyph: Words.glyph("verb:calendar:join"),
                hue: "action",
                later: true,
                run: () => Calendar.join(meeting)
            }));
        rows.push(seeing(meeting));
        if (Calendar.invited(meeting))
            for (const said of ["yes", "maybe", "no"])
                rows.push(answering(meeting, said));
        return rows;
    }

    // See It: the meeting's details a step in, made as it's stepped into,
    // so they're what the program last said. Stepped into by the meeting's
    // own Return, it's named for the meeting, so the path says which.
    function seeing(meeting, name) {
        const key = "verb:calendar:see";
        const inside = () => provider.details(Calendar.meeting(meeting.id) ?? meeting);
        const row = thing({
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

    // What's known of it, each a figure that copies what it says, then
    // its page in Google Calendar, where it's changed.
    function details(meeting) {
        const rows = [copyRow({
            key: "fact:meeting:when",
            name: Calendar.dated(meeting),
            side: "when"
        })];
        if (Calendar.invited(meeting))
            rows.push(copyRow({
                key: "fact:meeting:answer",
                name: Calendar.answered(meeting),
                copy: "",
                side: "your answer"
            }));
        if (meeting.link)
            rows.push(copyRow({
                key: "fact:meeting:call",
                name: meeting.link,
                side: "its call"
            }));
        if (meeting.page)
            rows.push(thing({
                key: "verb:calendar:open",
                name: Words.name("verb:calendar:open"),
                verb: Words.verb("verb:calendar:open"),
                glyph: Words.glyph("verb:calendar:open"),
                hue: "action",
                later: true,
                run: () => Calendar.open(meeting)
            }));
        return rows;
    }

    // An answer keeps the launcher, since he's choosing among his
    // meetings, and the foot says the answer the program last gave.
    function answering(meeting, said) {
        const key = `verb:calendar:${said}`;
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => Calendar.answered(Calendar.meeting(meeting.id)),
            run: () => Calendar.answer(meeting, said)
        });
    }
}
