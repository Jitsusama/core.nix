// What the card of Joel's line to the one standing at Agents' top says, run
// in Qt's own JavaScript engine, the one the shell runs it in: the
// notify-send it sends, sent and then answered in place, the reply cut to
// whole words, and which talk its line began. The
// `the-void-answers-on-one-card` check copies modules/quickshell/Told.js
// and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen told.qml`.
import QtQuick
import "Expect.js" as Expect
import "Told.js" as Told

QtObject {
    Component.onCompleted: {
        try {
            Expect.same("no card is made for his line, his answer reading in Agents instead",
                Told.carded(), false);
            const card = { id: 0, from: "The Void", said: "sent", open: "read it in Agents",
                line: "is the beacon lit?", reply: "" };
            Expect.same("sent, it's a new low card signed by the void, his line its own",
                Told.command(card), ["notify-send", "--print-id", "--app-name=pi",
                    "--urgency=low", "--hint=string:x-from:The Void",
                    "--hint=string:x-said:sent", "--action=default=read it in Agents", "--",
                    "is the beacon lit?"]);
            const answered = Object.assign({}, card, { id: 41, said: "answered",
                reply: "Lit <now> & bright" });
            Expect.same("answered, it takes the sent card's place, the reply plain",
                Told.command(answered).filter(word => word.startsWith("--replace")
                    || word.startsWith("--hint=string:x-said") || word.startsWith("Lit")),
                ["--replace-id=41", "--hint=string:x-said:answered",
                    "Lit &lt;now&gt; &amp; bright"]);
            // Filed, it can be taken back, an act of its own after opening
            // it, which goes once taken (VKCF3Z, 2026-11-12 [da486023]).
            const undoable = Object.assign({}, answered, { reply: "→ Delta", undo: "22:54",
                takes: "Take It Back" });
            Expect.same("one the void can undo offers Take It Back, after opening it",
                Told.command(undoable).filter(word => word.startsWith("--action")),
                ["--action=default=read it in Agents", "--action=take=Take It Back"]);
            Expect.same("one it can't offers only opening it",
                Told.command(answered).filter(word => word.startsWith("--action")),
                ["--action=default=read it in Agents"]);
            const taken = Object.assign({}, undoable, { said: "taken", undo: "" });
            Expect.same("taken back, it says so where sent was said, what was filed kept",
                Told.command(taken).filter(word => !word.startsWith("--print")
                    && !word.startsWith("--app") && !word.startsWith("--urgency")),
                ["notify-send", "--replace-id=41", "--hint=string:x-from:The Void",
                    "--hint=string:x-said:taken",
                    "--action=default=read it in Agents", "--", "is the beacon lit?", "→ Delta"]);
            const refused = Object.assign({}, taken, { said: "told", told: "nothing filed then" });
            Expect.same("any other answer is what it says, never a silent card",
                Told.command(refused).filter(word => word.includes("x-said")
                    || word.includes("x-told")),
                ["--hint=string:x-said:told", "--hint=string:x-told:nothing filed then"]);
            Expect.same("the void's own undone is taken back", Told.takenBy(" Undone. "),
                { said: "taken", told: "" });
            Expect.same("anything else it says is told as it said it, in the card's few words",
                Told.takenBy("There is no line at 22:54 in today's file to take back, sorry"),
                { said: "told", told: "There is no line at 22:54 in today's..." });
            Expect.same("a line opening with a dash is never an option",
                Told.command(Object.assign({}, card, { line: "-v?" })).slice(-2), ["--", "-v?"]);

            Expect.same("a reply that fits is said whole, its spaces one",
                Told.cut("  the beacon\n is   lit ", "more in the talk"), "the beacon is lit");
            const long = Array.from({ length: 80 }, (_, at) => `word${at}`).join(" ");
            const cut = Told.cut(long, "more in the talk");
            Expect.same("a long reply ends in three plain dots and where the rest is",
                cut.endsWith(" ... more in the talk"), true);
            Expect.same("it's cut between words, never inside one",
                cut.replace(" ... more in the talk", "").split(" ")
                    .every((word, at) => word === `word${at}`), true);
            Expect.same("it fills the card's six lines of 36 and no more",
                Told.laid(cut.split(" ")).length, 6);
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
