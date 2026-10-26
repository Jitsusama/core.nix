// Steer, in Tools: Joel's words to the agent steering the others, so he can
// turn the work without finding its window (Steward.qml). Return asks for
// them in the field, and Return again sends them as his own message and
// closes the launcher; "steer" and the words typed at once do the same in
// one go. With no steward named, there's nothing to steer, and no row.
import qs
import QtQuick

Provider {
    id: provider

    what: "steer"

    function said(id) {
        const words = (Words.of("steer").said ?? {})[id] ?? id;
        return words.replace("{steward}", Steward.steward);
    }

    function send(words) {
        Steward.steer(words);
        if (provider.launcher)
            provider.launcher.open = false;
    }

    things: Steward.steward === "" ? [] : [thing({
        key: "steer",
        verb: Words.verb("steer"),
        name: Words.name("steer"),
        keywords: Words.keywords("steer"),
        glyph: Words.glyph("steer"),
        hue: "action",
        side: Steward.steward,
        note: provider.said("note"),
        ask: Words.of("steer").ask,
        given: words => provider.send(words)
    })]

    // "steer" and what to say, at once.
    function answer(typed) {
        const asked = /^steer\s+(\S[\s\S]*)$/i.exec(typed.trim());
        if (!asked || Steward.steward === "")
            return null;
        const words = asked[1];
        return {
            headline: provider.said("sent"),
            verb: Words.command("command:steer").verb,
            run: () => provider.send(words),
            rows: [],
            note: provider.said("note")
        };
    }
}
