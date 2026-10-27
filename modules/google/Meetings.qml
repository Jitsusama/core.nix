// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from today
// to a week on as Calendar.qml holds them, so Programs › Calendar lists
// them after the program's own acts, and a meeting's name finds it from
// anywhere. Return joins a meeting's call, which takes Joel out of the
// launcher, or opens its page in Google Calendar when it has none; the
// right arrow lists Join the Meeting, Open in Calendar and Copy the Link,
// each there only when it can be done. Each row says when the meeting is
// at the right and its times below; its names and words are the
// vocabulary's (calendar:*, Words.qml).
import qs
import QtQuick

Provider {
    id: provider

    what: "meeting"

    readonly property var shown: Calendar.meetings.filter(meeting => meeting.end > Calendar.now)

    things: shown.map(meeting => made(meeting))
    onThingsChanged: changedTo(things)

    function made(meeting) {
        const joins = !!meeting.link;
        const name = meeting.title !== "" ? meeting.title
            : Words.name("calendar:meeting", "untitled");
        return thing({
            key: `calendar:meeting:${meeting.id}`,
            of: "program:google-calendar",
            part: name,
            name: name,
            verb: joins ? Words.verb("verb:calendar:join") : Words.verb("verb:calendar:open"),
            glyph: Words.glyph("calendar:meeting"),
            hue: "program",
            side: Calendar.when(meeting),
            note: Calendar.span(meeting),
            later: true,
            run: () => joins ? Calendar.join(meeting) : Calendar.open(meeting),
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
        if (meeting.link)
            rows.push(copyRow({
                key: "verb:calendar:copy",
                name: Words.name("verb:calendar:copy"),
                copy: meeting.link,
                glyph: Words.glyph("verb:calendar:copy"),
                figure: false
            }));
        return rows;
    }
}
