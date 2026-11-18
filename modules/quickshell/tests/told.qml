// What became of Joel's line to the void, as the Super+S line reads it,
// run in Qt's own JavaScript engine, the one the shell runs it in: which
// talk his line began and its reply, where the void says it went, and how
// a take-back stands. The `a-reply-to-the-void-says-where-it-went` check
// copies modules/quickshell/Told.js and tests/lib/Expect.js beside this
// file and runs `qml -platform offscreen told.qml`.
import QtQuick
import "Expect.js" as Expect
import "Told.js" as Told

QtObject {
    Component.onCompleted: {
        try {
            Expect.same("the void's own undone is taken back", Told.takenBy(" Undone. "),
                { said: "taken", told: "" });
            Expect.same("anything else it says is told as it said it, in a line's few words",
                Told.takenBy("There is no line at 22:54 in today's file to take back, sorry"),
                { said: "told", told: "There is no line at 22:54 in today's..." });
            Expect.same("a word longer than a line takes the lines it fills",
                Told.laid(["a".repeat(80), "b"]).length, 3);

            // The void's reply says first where his line went, and when it
            // can be undone, by the time its day file keeps the line at
            // (8GYNJ6, 2026-11-12 [e7d88599]).
            Expect.same("where it went is kept, the undo taken off it to be an act",
                Told.went("→ Delta · undo 22:54\n· 22:54 \"ship it\" → a quest"),
                { reply: "→ Delta\n· 22:54 \"ship it\" → a quest", undo: "22:54" });
            Expect.same("to his orchestrator, the same",
                Told.went("→ your orchestrator · undo 9:05 "),
                { reply: "→ your orchestrator", undo: "9:05" });
            // It says who has it in words rather than with an arrow since QCXZEB's answer of
            // 2026-11-13 [31907e55], and can be undone all the same.
            Expect.same("who has it said in words, the same",
                Told.went("Your orchestrator has it · undo 07:43"),
                { reply: "Your orchestrator has it", undo: "07:43" });
            Expect.same("a to-do for him can't be undone",
                Told.went("→ for you: call the bank"),
                { reply: "→ for you: call the bank", undo: "" });
            Expect.same("nor what it kept", Told.went("kept"), { reply: "kept", undo: "" });
            Expect.same("an undo said later in the reply is only words",
                Told.went("Lit.\n→ Delta · undo 22:54"),
                { reply: "Lit.\n→ Delta · undo 22:54", undo: "" });
            Expect.same("what the void is told to undo it", Told.undoing("22:54"), "undo 22:54");

            const talks = [
                { words: "is the beacon lit?", at: "2026-11-12T17:40:33.636Z",
                    stage: "replying", reply: "Li" },
                { words: "is the beacon lit?", at: "2026-11-12T17:30:00.000Z",
                    stage: "replied", reply: "an older answer" }
            ];
            const since = Date.parse("2026-11-12T17:40:28.500Z");
            const talk = Told.talkOf(talks, "is the beacon lit?", since);
            Expect.same("its talk is the one its line began, not an older same line",
                talk.at, "2026-11-12T17:40:33.636Z");
            Expect.same("one still being written isn't a reply yet", Told.replyOf(talk), "");
            Expect.same("one replied is", Told.replyOf(Object.assign({}, talk,
                { stage: "replied", reply: "Lit." })), "Lit.");
            Expect.same("once over, it's still the reply", Told.replyOf(Object.assign({}, talk,
                { stage: "over", reply: "Lit." })), "Lit.");
            Expect.same("before it's delivered, there's no talk",
                Told.talkOf([], "is the beacon lit?", since), null);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
