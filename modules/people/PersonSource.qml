// What a program that knows people gives the launcher's People place
// (People.qml), each such source a file starting from this one, named in
// jitsusama.people.sources by the module that brings the program:
//
//     PersonSource {
//         people: [{ emails: ["ada@example.com"], name: "Ada Lovelace",
//                    facts: [{ label: "Team", value: "Shell" }], recent: true }]
//         function acts(person, provider) {
//             return [provider.thing({ key: "verb:person:wave", name: "Wave",
//                                      verb: "wave", run: () => … })];
//         }
//     }
//
// Everyone it knows (people), each by every address it knows them by,
// with a name when it has one, facts the drawer shows under the source's
// own labels, and `recent` for someone Joel dealt with lately, who's
// listed with nothing typed. Who it knows changes seldom; how they stand
// (stands) changes often, so it's kept apart, by address: { said, order,
// favourite, recent }, "available" and 0 for someone who can be reached
// now. Given a person as People.qml joins them, { key, email, emails,
// name, facts, recent, spans, from }, `acts` gives what Joel can do with them through
// it, rows made with the People place's provider, `main` on the one way
// it reaches them; `next` the next thing it has with them, a meeting not
// yet over, as { at, row }, or null; and `recent` the things it's had
// with them, each { at, row }, `at` when it was in milliseconds since the
// epoch, so People lists every source's newest first. Each row is drawn
// by that thing's own component (Mail.row, Calendar.row), never one of
// its own. `page` is the link to their page in the source, for one that
// keeps a page for each person, or "": People lists it as one row, Open
// in the source, by the name it's `called`, after the ways to reach them
// (VKCF3Z, a91b2722), so no source draws a page of its own; `looking`
// says whether it's still finding out if it keeps one for them, as Vault
// does while it's asked, so People holds that row's place, dimmed, and
// nothing arrives late under the keys (VKCF3Z, 3bbad443). `favourite`
// is the switch saying whether they're a favourite, for a source that
// keeps favourites, or null. Someone it knows by no
// address it gives by an `id` of its own instead, a person of its alone,
// whose stands are keyed "<its name>:<id>", their key. `woken` is asked
// each time the launcher opens, for a source that only follows what it
// knows while someone might look. A source whose program the people
// daemon runs and says everyone to, as Tuple's does, `daemonHolds`: it gives
// no `people` or `stands` of its own, and People reads from the daemon
// only those it lists and those a search finds, never everyone, asking
// `stood` how each stands from what the source told the daemon of them.
// A source
// that learns its acts for someone only once asked, as Slack's looks
// them up, tells its `place` (People.qml's provider) it changed, so a
// person Joel is inside is made again.
import QtQuick

QtObject {
    // What a source holds of its own, as Slack's holds the reader it asks:
    // a QtObject has no default property for one to go in.
    default property list<QtObject> held

    // Its settings from Nix, as the option gives them.
    property var settings: ({})
    property var people: []
    property var stands: ({})
    // Whether the people daemon holds who it knows, rather than `people`.
    property bool daemonHolds: false
    // The People place it's a source of, given as it's made.
    property var place: null
    // Its name as Joel reads it, in Open in it for a person's page.
    property string called: ""

    function page(person) {
        return "";
    }

    function looking(person) {
        return false;
    }

    function acts(person, provider) {
        return [];
    }

    function next(person, provider) {
        return null;
    }

    function recent(person, provider) {
        return [];
    }

    function favourite(person, provider) {
        return null;
    }

    function woken() {
    }

    // How one the daemon holds stands, from what this source told it of
    // them: { said, order, favourite, recent }, as `stands` holds them.
    function stood(entry) {
        return { said: "", order: 4, favourite: entry.favourite === true,
            recent: entry.recent === true };
    }
}
