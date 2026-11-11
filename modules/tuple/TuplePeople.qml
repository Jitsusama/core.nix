pragma Singleton
// What People's Tuple source (TupleSource.qml beside this) does with a
// contact: how one stands, in words, and calling, joining or favouring
// them through Tuple. Who Tuple's contacts are, and how each stands, the
// people daemon follows (tuple-people people, people.rs), never the shell,
// so the shell holds none of the thousands Tuple can list. A contact is
// as the daemon holds it: { email, id, name, status, favourite, recent }.
import qs
import Quickshell
import QtQuick

Singleton {
    id: root

    // How each stands, as which of People's words (words.json) and in the
    // order of who can pair first. Tuple says "unknown" of anyone not a
    // favourite, which says nothing.
    readonly property var statuses: ({
        online: { word: "available", order: 0 },
        busy: { word: "call", order: 1 },
        unavailable: { word: "away", order: 2 },
        offline: { word: "offline", order: 3 }
    })

    function how(stands) {
        const status = statuses[stands ? stands.status : ""];
        return status ? { said: Words.of("people").said[status.word], order: status.order }
            : { said: "", order: 4 };
    }

    // Tuple finds someone by their email; the id Tuple knows them by is
    // the one left when they have none.
    function who(contact) {
        return contact.email || String(contact.id);
    }

    // Calls one who isn't in a call and joins one who is.
    function pair(contact) {
        Niri.spawn(["tuple", "call", contact.status === "busy" ? "join" : "start", who(contact)]);
    }

    // Tells Tuple they're a favourite or not; the daemon hears it from
    // Tuple, and the source shows it at once meanwhile.
    function favour(contact, wanted) {
        Niri.spawn(["tuple", "contacts", wanted ? "favorite" : "unfavorite", who(contact)]);
    }
}
