// Tuple's calls in the launcher: each row Tuple says can be done now
// (Tuple.qml's controls), such as muting Joel in the call he's in or
// joining the call whose link was just copied, found anywhere by name,
// and listed as Tuple's own acts inside Programs › Tuple (programActs,
// which Applications.qml asks every provider that has it). Inside, as
// VKCF3Z's design orders them (RSCH-20261028-2NK1E5, Components): opening
// Joel's room first, so Return twice does it on purpose, since Tuple has
// no window of its own and its row steps in; then the call that rings or
// is live, showing it, whether Joel's Muted, sharing and leaving; then
// joining the call whose link is copied; and copying his room's link
// last of all, after Tuple's Recent, Notices and Shortcuts, as every Copy
// is (Applications.qml). A ringing or live call is in Now as well.
//
// Tuple's Recent (recentOf, which Applications.qml asks every provider
// that has it), as VKCF3Z placed it: each call that ended and each
// missed, newest first, as tuple-call keeps nine of them (call/src/
// recent.rs), each named by who it was with, then how long it went on,
// or "missed" in its place, and when it began: "Ana  42 min \u00b7
// yesterday 14:10". At its right, how the first of them stands in Tuple
// now, as their row in People says it. Return calls them again through
// Tuple, joining their call when they're in one, as People's Call Them
// does, and closes the launcher; the right arrow lists, after that,
// each of them as their one row in People, where People knows them.
// The row itself is a call's, not a person's: it says what the call
// was, a call may have had several people, and its Return calls where
// a person's steps in (People.qml).
import qs
import QtQuick
import "../Tuple.js" as Log

Provider {
    id: provider

    what: "call"

    // Tuple's desktop entry hides itself, so Programs lists it for these.
    hiddenPrograms: ["tuple"]

    // :set nomic mutes Joel in the call he's in and :set mic lets him be
    // heard, as its Muted row does, while Tuple says which he is.
    setOptions: !Tuple.calling || Tuple.muted === null ? ({}) : ({
        mic: {
            kind: "switch",
            now: () => Tuple.muted === false,
            set: to => {
                Tuple.mute(!to);
                return "";
            }
        }
    })

    things: Tuple.controls.map(control => thing(control))
    // So Programs › Tuple, held while a call starts or ends, is drawn again.
    onThingsChanged: changed()

    // Where each of Tuple's rows goes inside it, by its key.
    readonly property var places: ["tuple:open", "tuple:ringing", "tuple:show", "tuple:mute",
        "tuple:share", "tuple:leave", "tuple:join", "tuple:link"]

    function programActs(program) {
        if (program !== "tuple")
            return [];
        return things.slice().sort((a, b) => places.indexOf(a.key) - places.indexOf(b.key));
    }

    function recentOf(program) {
        return program === "tuple" ? Tuple.recent.map(past => provider.pastRow(past)) : [];
    }

    // So Programs › Tuple's Recent, held while a call ends or is missed, is
    // drawn again.
    Connections {
        target: Tuple

        function onRecentChanged() {
            provider.changed();
        }
    }

    // What a past call's row says, in the vocabulary's words, with the
    // locale's names of days, as Qt gives them.
    readonly property var said: Object.assign({}, Words.of("tuple:call").said ?? {}, {
        someone: Words.name("tuple:call", "unknown"),
        with: Words.name("tuple:call:with"),
        email: Words.name("tuple:call:email"),
        emails: Words.name("tuple:call:email", "many"),
        length: Words.name("tuple:call:length"),
        rang: Words.name("tuple:call:rang"),
        ended: Words.name("tuple:call:ended"),
        date: (date, format) => Qt.formatDate(date, format)
    })

    // The launcher's People, which knows how each of Tuple's contacts
    // stands, or null on a machine without it.
    function people() {
        return (launcher?.sources ?? []).find(source => typeof source.personOf === "function")
            ?? null;
    }

    // Someone a past call was with, as Tuple's contact is called: their
    // address, or Tuple's id for one who has none, and how they stand now
    // where People has heard from Tuple, so one in a call is joined.
    function contact(someone) {
        const email = String(someone.email ?? "").trim().toLowerCase();
        const entry = people()?.entryOf({ emails: email === "" ? [] : [email],
            key: `tuple:${someone.id}` }, "tuple") ?? null;
        return Object.assign({ id: someone.id, email: email, status: "" }, entry ?? {});
    }

    // One past call's row. A missed call from a caller Tuple's log didn't
    // number has nobody to call again, so it stays, dimmed, saying what
    // it can.
    function pastRow(past) {
        const first = past.people.length > 0 ? contact(past.people[0]) : null;
        const busy = first?.status === "busy";
        const book = people();
        // Its acts, Return's leading as the launcher leads every thing's,
        // make it a thing holding acts of its own, which Return does from
        // any mode, as a meeting's joins its call (Catalogue.js's
        // endsSomething), even with no one People knows to list after it.
        const theirs = () => past.people.map(one => book?.personOf(one.email) ?? null)
            .filter(Boolean);
        const now = Date.now();
        return thing(Object.assign({
            key: `tuple:call:${past.missed ? "missed" : "ended"}:${past.since}`,
            name: Log.who(past.people, said),
            note: Log.note(past, now, said),
            drawer: Log.details(past, now, said),
            side: first === null ? "" : TuplePeople.how(first).said,
            glyph: Words.glyph("tuple:call", busy ? "busy" : ""),
            hue: "person",
            verb: Words.verb("tuple:call", busy ? "busy" : ""),
            keywords: past.people.map(one => one.email).filter(Boolean).join(" ") || null,
            at: past.since,
            later: true,
            verbs: theirs,
            run: () => TuplePeople.pair(first)
        }, first !== null ? {} : { gone: true, run: () => {} }));
    }
}
