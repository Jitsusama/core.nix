// Tuple as one of People's sources (PersonSource.qml): everyone Tuple
// lists as a contact, by their email, as TuplePeople.qml follows them, and
// how those Tuple knows stand: available, in a call, away or offline, its
// favourites and those called lately listed with nothing typed. Its main
// act calls one who isn't in a call and joins one who is; it can also make
// them a favourite or not. A contact with no email can't be one person
// across the sources, so isn't in People.
import qs
import QtQuick

PersonSource {
    id: source

    people: TuplePeople.everyone.filter(person => person.email)
        .map(person => ({ emails: [person.email], name: person.name }))

    stands: {
        const by = {};
        for (const id in TuplePeople.standing) {
            const person = TuplePeople.people[id];
            if (!person || !person.email)
                continue;
            const stood = TuplePeople.standing[id];
            const how = TuplePeople.how(stood);
            by[person.email.toLowerCase()] = { said: how.said, order: how.order,
                favourite: stood.favourite, recent: stood.recent };
        }
        return by;
    }

    // The contact Tuple knows by one of a person's addresses, or null.
    function contact(person) {
        return TuplePeople.everyone.find(each => each.email
            && person.emails.includes(each.email.toLowerCase())) ?? null;
    }

    function acts(person, provider) {
        const known = contact(person);
        if (!known)
            return [];
        const stood = TuplePeople.standingOf(known);
        const busy = stood !== null && stood.status === "busy";
        const favourite = stood !== null && stood.favourite;
        return [provider.thing({
            key: busy ? "verb:person:join" : "verb:person:call",
            name: Words.name(busy ? "verb:person:join" : "verb:person:call"),
            verb: Words.verb(busy ? "verb:person:join" : "verb:person:call"),
            side: TuplePeople.how(stood).said,
            glyph: Words.glyph(busy ? "verb:person:join" : "verb:person:call"),
            hue: "action",
            main: true,
            later: true,
            run: () => TuplePeople.pair(known)
        }), provider.thing({
            // A switch, one row whose name never changes: it says yes or no,
            // and Return flips it, keeping the launcher.
            key: "verb:person:favourite",
            name: Words.name("verb:person:favourite"),
            verb: Words.verb("verb:person:favourite", favourite ? "favourite" : ""),
            side: favourite ? "yes" : "no",
            glyph: Words.glyph("verb:person:favourite"),
            hue: "action",
            stays: true,
            run: () => TuplePeople.favour(known, !favourite)
        })];
    }

    function woken() {
        TuplePeople.follow();
    }
}
