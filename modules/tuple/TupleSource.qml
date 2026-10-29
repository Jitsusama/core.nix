// Tuple as one of People's sources (PersonSource.qml): everyone Tuple
// lists as a contact, by their email, as TuplePeople.qml follows them, and
// how those Tuple knows stand: available, in a call, away or offline, its
// favourites and those called lately listed with nothing typed. Its main
// act calls one who isn't in a call and joins one who is, and its switch
// keeps them a favourite or not. A contact with no email is still called
// by Tuple's id for them, a person of Tuple's alone, since no other source
// could say who they are.
import qs
import QtQuick

PersonSource {
    id: source

    people: TuplePeople.everyone.map(person => person.email
        ? { emails: [person.email], name: person.name }
        : { emails: [], id: person.id, name: person.name })

    stands: {
        const by = {};
        for (const id in TuplePeople.standing) {
            const person = TuplePeople.people[id];
            if (!person)
                continue;
            const stood = TuplePeople.standing[id];
            const how = TuplePeople.how(stood);
            by[known(person)] = { said: how.said, order: how.order,
                favourite: stood.favourite, recent: stood.recent };
        }
        return by;
    }

    // What People knows a contact as: their email, or Tuple's id for them.
    function known(person) {
        return person.email ? person.email.toLowerCase() : `${settings.name}:${person.id}`;
    }

    // The contact Tuple knows by one of a person's addresses, or as one of
    // its own alone, or null.
    function contact(person) {
        return TuplePeople.everyone.find(each => person.emails.includes(known(each))
            || person.key === known(each)) ?? null;
    }

    function acts(person, provider) {
        const theirs = contact(person);
        if (!theirs)
            return [];
        const stood = TuplePeople.standingOf(theirs);
        const busy = stood !== null && stood.status === "busy";
        return [provider.thing({
            key: busy ? "verb:person:join" : "verb:person:call",
            name: Words.name(busy ? "verb:person:join" : "verb:person:call"),
            verb: Words.verb(busy ? "verb:person:join" : "verb:person:call"),
            side: TuplePeople.how(stood).said,
            glyph: Words.glyph(busy ? "verb:person:join" : "verb:person:call"),
            hue: "action",
            main: true,
            later: true,
            run: () => TuplePeople.pair(theirs)
        })];
    }

    // A switch, one row whose name never changes: it says yes or no, and
    // Return flips it, keeping the launcher, its side and verb read as
    // they're shown, since thing() would copy them once.
    function favourite(person, provider) {
        const theirs = contact(person);
        if (!theirs)
            return null;
        const kept = () => TuplePeople.standingOf(theirs)?.favourite === true;
        const row = provider.thing({
            key: "verb:person:favourite",
            name: Words.name("verb:person:favourite"),
            glyph: Words.glyph("verb:person:favourite"),
            hue: "action",
            stays: true,
            run: () => TuplePeople.favour(theirs, !kept())
        });
        return Object.defineProperties(row, {
            side: { enumerable: true, get: () => (kept() ? "yes" : "no") },
            verb: {
                enumerable: true,
                get: () => Words.verb("verb:person:favourite", kept() ? "favourite" : "")
            }
        });
    }

    function woken() {
        TuplePeople.follow();
    }
}
