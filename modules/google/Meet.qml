// Meet's own acts in the launcher, listed after its desktop entry's New
// Meeting (home.nix) and before its Recent, as the program's acts
// (programActs, which Applications.qml asks every provider that has it):
// Join by Code, which asks for a meeting's code, typed or pasted with or
// without its dashes or as its whole link, and joins its call through
// google-opens, in a window of its own. Anything Meet wouldn't know as a
// code is refused, the foot saying so.
import qs
import QtQuick

Provider {
    id: provider

    what: "call"

    // What the foot says of the last code given: joining it, or that it
    // isn't one.
    property string told: ""

    function programActs(program) {
        if (program !== "google-meet")
            return [];
        const key = "verb:meet:code";
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            ask: Words.of(key).ask ?? "",
            chip: Words.name(key),
            state: () => provider.told,
            given: words => provider.join(words)
        })];
    }

    function join(words) {
        const code = Calendar.meetCode(words);
        if (code === "") {
            told = "not a Meet code";
            return;
        }
        told = `joining ${code}`;
        Niri.spawn(["google-opens", `https://meet.google.com/${code}`]);
    }
}
