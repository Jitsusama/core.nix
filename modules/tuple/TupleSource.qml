// Tuple as one of People's sources (PersonSource.qml): everyone Tuple
// lists as a contact, held by the people daemon (tuple-people people
// says them to it, people.rs), so the shell reads only those Tuple lists
// and those a search finds, and how each stands: available, in a call,
// away or offline, its favourites and those called lately listed with
// nothing typed. Its main act calls one who isn't in a call and joins
// one who is, and its switch keeps them a favourite or not. A contact
// with no email is still called by Tuple's id for them, a person of
// Tuple's alone, since no other source could say who they are.
import qs
import QtQuick

PersonSource {
    id: source

    daemonHolds: true

    // Favourites changed here, by Tuple's id, each { wanted, key }, so a
    // row moves at once rather than when Tuple tells it: each is kept
    // until the daemon's record says what was asked, or isn't held.
    property var favoured: ({})

    Connections {
        target: source.place

        function onRevisionChanged() {
            const kept = {};
            for (const id in source.favoured) {
                const asked = source.favoured[id];
                const entry = source.place.recordOf[asked.key]?.sources?.[source.settings.name];
                if (entry && entry.favourite !== asked.wanted)
                    kept[id] = asked;
            }
            source.favoured = kept;
        }
    }

    // What Tuple told the daemon of them, with a favourite changed here.
    function standingOf(entry) {
        const id = String(entry.id ?? "");
        return id in favoured
            ? Object.assign({}, entry, { favourite: favoured[id].wanted }) : entry;
    }

    function stood(entry) {
        const stands = standingOf(entry);
        const how = TuplePeople.how(stands);
        return { said: how.said, order: how.order, favourite: stands.favourite === true,
            recent: stands.recent === true };
    }

    // The contact Tuple knows them as, or null.
    function contact(person, provider) {
        const entry = provider.entryOf(person, settings.name);
        return entry ? standingOf(entry) : null;
    }

    function acts(person, provider) {
        const theirs = contact(person, provider);
        if (!theirs)
            return [];
        const busy = theirs.status === "busy";
        return [provider.thing({
            key: busy ? "verb:person:join" : "verb:person:call",
            name: Words.name(busy ? "verb:person:join" : "verb:person:call"),
            verb: Words.verb(busy ? "verb:person:join" : "verb:person:call"),
            side: TuplePeople.how(theirs).said,
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
        const theirs = contact(person, provider);
        if (!theirs)
            return null;
        const id = String(theirs.id ?? "");
        const kept = () => (id in source.favoured ? source.favoured[id].wanted
            : theirs.favourite === true);
        // The daemon's key for them: their address, or Tuple's id.
        const key = theirs.email ? `person:${theirs.email.trim().toLowerCase()}`
            : `person:${settings.name}:${id}`;
        const row = provider.thing({
            key: "verb:person:favourite",
            name: Words.name("verb:person:favourite"),
            glyph: Words.glyph("verb:person:favourite"),
            hue: "action",
            stays: true,
            run: () => {
                const wanted = !kept();
                TuplePeople.favour(theirs, wanted);
                source.favoured = Object.assign({}, source.favoured,
                    { [id]: { wanted: wanted, key: key } });
            }
        });
        return Object.defineProperties(row, {
            side: { enumerable: true, get: () => (kept() ? "yes" : "no") },
            verb: {
                enumerable: true,
                get: () => Words.verb("verb:person:favourite", kept() ? "favourite" : "")
            }
        });
    }
}
