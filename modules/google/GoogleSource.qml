// Google as one of People's sources (PersonSource.qml): everyone Joel's
// meetings and mail name, each who held or was asked to a meeting from a
// month ago to a week on and each who sent mail that wants him, which
// google-people tells the people daemon (src/people.rs), so the daemon
// holds them and People reads them from it (`daemonHolds`), never merging
// them on the GUI thread at each mail or meeting. With no account there's
// nobody. Its main act writes them a
// new mail in Mail's window; their next meeting is the first with them
// not yet over, their Unread the mail they sent that wants him, and their
// Recent the meetings with them over in the last month and the mail he
// sent them or read from them in it (Google.held, Google.lately), each
// drawn by its own row (Calendar.row, Mail.row), so a meeting or a mail
// there is the very one it is everywhere else. Mail that still wants him
// is Unread and never Recent, and Recent is what Joel had with them
// (GB56HX).
// Google's own contacts would need a sign-in pi doesn't ask for, so
// who's known is only who's met or written.
import qs
import QtQuick

PersonSource {
    id: source

    daemonHolds: true

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

    function unread(person, provider) {
        return Mail.mail.filter(one => Google.withThem(one, person.emails))
            .map(one => ({ at: one.at, row: Mail.row(one, provider.thing), what: "mail" }));
    }

    // Meetings are where they met, each saying whether it was the two of
    // them alone, and mail what they wrote each other, saying whether Joel
    // sent it, as their page's Last Met and Messages say (People.qml).
    function recent(person, provider) {
        const now = Date.now();
        const meetings = Google.held.concat(Calendar.meetings).filter(meeting => meeting.end
            <= now && Google.withThem(meeting, person.emails))
            .map(meeting => ({ at: meeting.end, row: Calendar.row(meeting, provider),
                kind: "met", one: alone(meeting) }));
        const mail = Google.lately.filter(one => Google.mailedWith(one, person.emails))
            .map(one => ({ at: one.at, row: Mail.row(one, provider.thing), kind: "wrote",
                mine: one.sent === true }));
        return meetings.concat(mail);
    }

    // Whether a meeting was Joel's with one other: who holds it, unless he
    // does, and each guest, Google naming him in neither, are one address.
    function alone(meeting) {
        const others = (meeting.holds ? [] : [meeting.organizer?.email])
            .concat((meeting.guests ?? []).map(guest => guest.email))
            .filter(Boolean).map(email => String(email).toLowerCase());
        return new Set(others).size === 1;
    }
}
