// The notices' stack's keys, read from their one table in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-notices-take-vi-keys` check copies modules/quickshell/NoticeKeys.js
// beside this file and runs `qml -platform offscreen notice-keys.qml`.
import QtQuick
import "NoticeKeys.js" as Keys

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // What the keys do, typed one after another from rest: each act done,
    // with its count when one was typed, and whether the last key was the
    // stack's.
    function typed(keys) {
        let standing = Keys.rest();
        const acts = [];
        let ours = false;
        for (const key of keys) {
            const read = Keys.read(standing, key);
            standing = read.standing;
            ours = read.ours;
            if (read.act !== "")
                acts.push(read.count > 0 ? `${read.count}${read.act}` : read.act);
        }
        return { acts, ours, standing };
    }

    function run() {
        const failures = [];
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // [what, the keys typed, the acts done]
        const doing = [
            ["j and Down turn to the next", ["j", "Down"], ["next", "next"]],
            ["k and Up turn back", ["k", "Up"], ["previous", "previous"]],
            ["l steps in and h out, as the arrows", ["l", "h", "Right", "Left"],
                ["into", "out", "into", "out"]],
            ["Return runs, and with Ctrl closes", ["Return", "Ctrl+Return"], ["run", "runAndClose"]],
            ["dd forgets, as Delete and Backspace", ["d", "d", "Delete", "Backspace"],
                ["forget", "forget", "forget"]],
            ["DD forgets all, as Shift+Delete", ["D", "D", "Shift+Delete"],
                ["forgetAll", "forgetAll"]],
            ["gg goes to the newest and G the oldest", ["g", "g", "G"], ["newest", "oldest"]],
            ["q closes", ["q"], ["close"]],
            ["a count goes before a motion", ["5", "j", "1", "2", "k"], ["5next", "12previous"]],
            ["a count goes before G", ["3", "G"], ["3oldest"]],
            ["a lone 0 counts nothing", ["0", "j"], ["next"]],
            ["a key that ends nothing lets a sequence go", ["d", "j", "d"], []],
            ["Escape at rest steps out", ["Escape"], ["back"]],
            ["Escape cancels a count, then steps out", ["4", "Escape", "j", "Escape"],
                ["next", "back"]],
            ["Escape cancels a key waiting for its second", ["g", "Escape", "g"], []],
            // Shift pressed for the second D is a key of its own, with no name.
            ["a key with no name changes nothing", ["D", "", "D", "3", "", "j"],
                ["forgetAll", "3next"]]
        ];
        for (const [what, keys, want] of doing)
            holds(what, typed(keys).acts, want);

        holds("a key begun is the stack's", typed(["d"]).ours, true);
        holds("a key it doesn't know isn't", typed(["x"]).ours, false);
        holds("a key with no name isn't", typed(["d", ""]).ours, false);
        holds("a key ending a begun sequence badly is still the stack's", typed(["g", "x"]).ours,
            true);
        holds("nothing begun is left after an act", typed(["5", "j"]).standing, Keys.rest());

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
