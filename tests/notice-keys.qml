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
    function typed(keys, mode) {
        let standing = Keys.rest();
        const acts = [];
        let ours = false;
        for (const key of keys) {
            const read = Keys.read(standing, key, mode);
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
            ["Return runs, as Enter", ["Return", "Enter"], ["run", "run"]],
            ["dd forgets, as Delete and Backspace", ["d", "d", "Delete", "Backspace"],
                ["forget", "forget", "forget"]],
            ["dG forgets all, as Shift+Delete", ["d", "G", "Shift+Delete"],
                ["forgetAll", "forgetAll"]],
            ["DD is no key of vi's", ["D", "D"], []],
            ["yy copies", ["y", "y"], ["copy"]],
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
            // Shift pressed for dG's G is a key of its own, with no name.
            ["a key with no name changes nothing", ["d", "", "G", "3", "", "j"],
                ["forgetAll", "3next"]],
            ["a count goes before dd and yy", ["3", "d", "d", "2", "y", "y"],
                ["3forget", "2copy"]],
            ["u brings back", ["u"], ["undo"]],
            ["/ and ? search, n and N again", ["/", "?", "n", "N", "3", "n"],
                ["searchForward", "searchBack", "again", "againBack", "3again"]],
            ["V selects", ["V"], ["select"]],
            ["Space ? lists the keys", ["Space", "?"], ["keys"]]
        ];
        for (const [what, keys, want] of doing)
            holds(what, typed(keys).acts, want);

        // [what, the keys typed while cards are selected, the acts done]
        const selecting = [
            ["d forgets them at once, as x and Delete", ["d", "x", "Delete"],
                ["forget", "forget", "forget"]],
            ["y copies them at once", ["y"], ["copy"]],
            ["j and k carry it on, with a count", ["2", "j", "k"], ["2next", "previous"]],
            ["gg and G go to its ends", ["g", "g", "G"], ["newest", "oldest"]],
            ["V and Escape end it", ["V", "Escape"], ["select", "back"]],
            ["l, u and / are no keys of a selection", ["l", "u", "/"], []]
        ];
        for (const [what, keys, want] of selecting)
            holds(what, typed(keys, "visual").acts, want);
        holds("Space waits for its second", typed(["Space"]).ours, true);

        // A search typed key by key, from nothing.
        function searched(keys) {
            let typing = { pattern: "", ends: "" };
            for (const key of keys)
                typing = Keys.searching(typing.pattern, key);
            return typing;
        }
        holds("letters type and Space a space", searched(["B", "u", "Space", "l"]),
            { pattern: "Bu l", ends: "" });
        holds("Backspace takes the last back", searched(["a", "b", "Backspace"]),
            { pattern: "a", ends: "" });
        holds("Backspace with nothing left cancels", searched(["a", "Backspace", "Backspace"]),
            { pattern: "", ends: "cancelled" });
        holds("Return keeps it", searched(["a", "Return"]), { pattern: "a", ends: "found" });
        holds("Escape cancels it", searched(["a", "Escape"]).ends, "cancelled");
        holds("a named key types nothing", searched(["a", "Down"]).pattern, "a");

        holds("lowercase finds either case", Keys.found("build", "Build log, build"),
            [[0, 5], [11, 16]]);
        holds("a capital finds only itself", Keys.found("Build", "build Build"), [[6, 11]]);
        holds("nothing sought finds nothing", Keys.found("", "build"), []);
        holds("a match is washed, the rest escaped",
            Keys.washed("a<b\nb", "b", { ink: "#fff", ground: "#000" }),
            "a&lt;<span style=\"color:#fff;background-color:#000\">b</span><br>"
                + "<span style=\"color:#fff;background-color:#000\">b</span>");

        holds("a search goes to the next card holding it", Keys.next([true, false, true], true), 2);
        holds("back, to the one before", Keys.next([false, true, false], false), 1);
        holds("round to the front when only it holds it", Keys.next([true, false], true), 0);
        holds("none when no card holds it", Keys.next([false, false], true), -1);

        holds("a list that fits is All", Keys.ruler(0, 5, 5), "All");
        holds("at its start, Top", Keys.ruler(0, 5, 13), "Top");
        holds("at its end, Bot", Keys.ruler(8, 5, 13), "Bot");
        holds("between, how far through", Keys.ruler(2, 5, 13), "25%");

        const listed = Keys.keyList();
        holds("the keys list each line's first keys", listed[0], {
            name: "Turn the cards",
            chord: [{ text: "j", after: false }, { text: "k", after: true }]
        });
        holds("a sequence is listed as typed", listed.find(line => line.name === "Forget it").chord,
            [{ text: "d", after: false }, { text: "d", after: true }]);
        holds("Return is listed by its mark", listed.find(line => line.name === "Do it").chord,
            [{ text: "\u21b5", after: false }]);

        holds("a key begun is the stack's", typed(["d"]).ours, true);
        holds("a key it doesn't know isn't", typed(["x"]).ours, false);
        holds("Return with Ctrl isn't, since q closes", typed(["Ctrl+Return"]).ours, false);
        holds("a key with no name isn't", typed(["d", ""]).ours, false);
        holds("a key ending a begun sequence badly is still the stack's", typed(["g", "x"]).ours,
            true);
        holds("nothing begun is left after an act", typed(["5", "j"]).standing, Keys.rest());

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
