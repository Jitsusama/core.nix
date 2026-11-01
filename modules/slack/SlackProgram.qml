// What Slack's program row in Programs says while Slack can't be checked,
// which Now doesn't, since it doesn't want Joel (VKCF3Z, b8c8920e): its
// note says signed out, or since when it can't be checked, or that Slack
// can't be reached (`programNote`, which Applications.qml asks every
// provider that has it); and signed out, its first act after Return's own
// is Sign In, as every signed-out source's is (VKCF3Z, 347abdb7), which
// signs in again in pi, whose Slack sign-in (/slack-auth) writes the file
// slack-unreads reads within half a minute. What stands is
// SlackUnreads.qml's, so this keeps nothing of its own.
import qs
import QtQuick

Provider {
    id: provider

    what: "program"

    // The desktop entry's id Slack's program row is made from.
    readonly property string program: "slack"
    readonly property string note: SlackUnreads.programNote
    // So Programs, read as the corpus is joined, says the note as it
    // stands now.
    onNoteChanged: changed()

    function programNote(asked) {
        return asked === program ? note : "";
    }

    function programActs(asked) {
        if (asked !== program || SlackUnreads.stands !== "signedOut")
            return [];
        const key = "verb:slack:sign-in";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            leads: true,
            run: () => Sessions.tell("/slack-auth", Sessions.home)
        })];
    }
}
