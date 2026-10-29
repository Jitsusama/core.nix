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
// now. Given a person as People.qml joins them, { email, emails, name,
// facts, recent, from }, `acts` gives what Joel can do with them through
// it, rows made with the People place's provider, `main` on the one way
// it reaches them; `next` the next thing it has with them, a meeting not
// yet over, as { at, row }, or null; and `recent` the things it's had
// with them, each { at, row }, `at` when it was in milliseconds since the
// epoch, so People lists every source's newest first. Each row is drawn
// by that thing's own component (Mail.row, Calendar.row), never one of
// its own. `woken` is asked each time the launcher opens, for a source
// that only follows what it knows while someone might look. A source
// that learns its acts for someone only once asked, as Slack's looks
// them up, tells its `place` (People.qml's provider) it changed, so a
// person Joel is inside is made again.
import QtQuick

QtObject {
    // Its settings from Nix, as the option gives them.
    property var settings: ({})
    property var people: []
    property var stands: ({})
    // The People place it's a source of, given as it's made.
    property var place: null

    function acts(person, provider) {
        return [];
    }

    function next(person, provider) {
        return null;
    }

    function recent(person, provider) {
        return [];
    }

    function woken() {
    }
}
