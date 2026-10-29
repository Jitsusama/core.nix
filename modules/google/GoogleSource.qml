// Google as one of People's sources (PersonSource.qml): everyone Joel's
// meetings and mail name, each who holds or is asked to a meeting from
// today to a week on and each who sent mail that wants him (Google.js's
// people). With no account there's nobody. Its main act writes them a
// new mail in Mail's window; their next meeting is the first with them
// not yet over, and their Recent the meetings with them that are and the
// mail they sent, each drawn by its own row (Calendar.row, Mail.row), so
// a meeting or a mail there is the very one it is everywhere else.
// Google's own contacts would need a sign-in pi doesn't ask for, so
// who's known is only who's met or written.
import qs
import QtQuick

PersonSource {
    id: source

    people: Google.people(Calendar.meetings, Mail.mail)

    function acts(person, provider) {
        const address = person.emails[0] ?? "";
        const link = Mail.writing(address);
        if (link === "")
            return [];
        return [provider.thing({
            key: "verb:person:mail",
            name: Words.name("verb:person:mail"),
            verb: Words.verb("verb:person:mail"),
            glyph: Words.glyph("verb:person:mail"),
            hue: "action",
            main: true,
            later: true,
            run: () => Niri.spawn(["google-opens", link])
        })];
    }

    function next(person, provider) {
        const now = Date.now();
        const coming = Calendar.meetings.filter(meeting => meeting.end > now
            && Google.withThem(meeting, person.emails))
            .sort((a, b) => a.start - b.start)[0];
        return coming ? { at: coming.start, row: Calendar.row(coming, provider) } : null;
    }

    function recent(person, provider) {
        const now = Date.now();
        const meetings = Calendar.meetings.filter(meeting => meeting.end <= now
            && Google.withThem(meeting, person.emails))
            .map(meeting => ({ at: meeting.end, row: Calendar.row(meeting, provider) }));
        const mail = Mail.mail.filter(one => Google.withThem(one, person.emails))
            .map(one => ({ at: one.at, row: Mail.row(one, provider.thing) }));
        return meetings.concat(mail);
    }
}
