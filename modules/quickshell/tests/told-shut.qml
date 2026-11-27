// What the launcher keeps to say as it next opens, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-word-told-shut-waits-beside-the-others` check copies
// modules/quickshell/launcher/ToldShut.js and tests/lib/Expect.js beside
// this file and runs `qml -platform offscreen told-shut.qml`. Each case is
// a press told while the launcher was shut, and what its next opening says.
import QtQuick
import "ToldShut.js" as ToldShut
import "Expect.js" as Expect

QtObject {
    // What an opening at `now` says, after each of `tells` (told, at,
    // loose) was told in turn, as the foot's words in one line.
    function opened(tells, now) {
        let kept = [];
        for (const one of tells)
            kept = ToldShut.waiting(kept, one.told, one.at, one.loose);
        const due = ToldShut.due(kept, now);
        return { words: due.said.map(said => said.slice(0, 2).join(" ")).join(" | "),
            loose: due.loose, lapsed: due.lapsed };
    }

    readonly property real minute: 60 * 1000
    readonly property var coming: [["Quest A", "may yet come", "alert"]]
    readonly property var asleep: [["Quest B", "is asleep", "alert"]]
    readonly property var came: [["Quest A", "came after all", "text"]]

    Component.onCompleted: Expect.run(() => {
        Expect.same("a word told shut is said as it opens",
            opened([{ told: asleep, at: 0 }], minute).words, "Quest B is asleep");
        Expect.same("a late word about one press leaves a newer press's about another",
            opened([{ told: asleep, at: 0 }, { told: came, at: minute }], 2 * minute).words,
            "Quest B is asleep | Quest A came after all");
        Expect.same("a newer word about the same session takes the older's place",
            opened([{ told: coming, at: 0 }, { told: came, at: minute }], 2 * minute).words,
            "Quest A came after all");
        Expect.same("a word told more than a quarter hour before it opens lapses",
            opened([{ told: asleep, at: 0 }], 15 * minute + 1),
            { words: "", loose: false, lapsed: 1 });
        Expect.same("one told a quarter hour before is said still",
            opened([{ told: asleep, at: 0 }], 15 * minute).words, "Quest B is asleep");
        Expect.same("only the lapsed word goes",
            opened([{ told: asleep, at: 0 }, { told: came, at: 10 * minute }], 20 * minute),
            { words: "Quest A came after all", loose: false, lapsed: 1 });
        Expect.same("words are loose only where each is",
            [opened([{ told: asleep, at: 0, loose: true }], minute).loose,
                opened([{ told: asleep, at: 0, loose: true }, { told: came, at: 0 }],
                    minute).loose],
            [true, false]);
        const started = [["Start New Work", "asked", "text"], ["Start New Work", "up", "text"]];
        Expect.same("a start's words under one mark are each kept",
            opened([{ told: started, at: 0 }], minute).words,
            "Start New Work asked | Start New Work up");
        const kept = ToldShut.waiting(ToldShut.waiting([], asleep, 0), started, 0);
        Expect.same("taking a start's words back leaves another's",
            ToldShut.without(kept, started).map(one => one.said[0]), ["Quest B"]);
    })
}
