// Dictation as a row in the launcher's Desktop, beside Capture and Do Not
// Disturb, so it's found where Joel looks rather than known only by its
// key (VKCF3Z's ruling, 2026-11-13: ten takes in fourteen days is a
// findability fault). Its side says how it stands and nothing else;
// Return closes the launcher and starts a take where the focus was, which
// a tap of Super+D ends, so the key keeps its one meaning. Its drawer says
// how, what a terminal gets, and the other key in its fold.
import qs
import QtQuick

Provider {
    id: provider

    what: "dictation"

    readonly property var words: Words.of("dictation")
    readonly property var told: words.said ?? {}

    // How it stands, as the service's watch says it (Dictation.qml):
    // ready while it answers and waits, listening and writing while it
    // works, off while its socket can't be reached.
    readonly property string stands: !Dictation.reached ? "off"
        : Dictation.listening ? "listening"
        : Dictation.transcribing ? "transcribing"
        : "ready"

    function named(id, value) {
        return { label: Words.name(`dictation:${id}`), value: value };
    }

    readonly property var dictating: thing({
        key: "dictation",
        verb: Words.verb("dictation"),
        name: Words.name("dictation"),
        glyph: Words.glyph("dictation"),
        keywords: (words.words ?? []).join(" "),
        side: told[stands] ?? "",
        hue: "voice",
        thing: true,
        drawer: () => ({
            eyebrow: told.eyebrow ?? "",
            title: Words.name("dictation"),
            facts: [named("how", told.how ?? ""), named("terminal", told.terminal ?? ""),
                named("stop", told.stop ?? "")],
            body: (words.tells ?? {}).body ?? "",
            nerd: [named("also", told.also ?? "")]
        }),
        // The launcher closes as it runs, so the take starts where the
        // focus goes back to, and what's said is typed there.
        run: () => Dictation.toggle()
    })

    things: [dictating]
}
