// What Super+S's talk line sends and says, run in Qt's own JavaScript engine,
// the one the shell runs it in: the window a line goes with, the words sent
// with it attached, the quiet line naming who hears it, and the talk sent,
// answered, taken back, all in the line. The
// `the-talk-line-says-what-became-of-its-words` check copies
// modules/quickshell/TalkLine.js, the Told.js it imports and
// tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen talk-line.qml`.
import QtQuick
import "Expect.js" as Expect
import "TalkLine.js" as Said

QtObject {
    Component.onCompleted: {
        try {
            const kitty = { app_id: "kitty", title: "π - Tune the launcher" };
            const session = { to: "TESTQA", name: "Tune the launcher" };
            const rows = Said.attached(kitty, "kitty", session);
            Expect.same("a pi window attaches its window and its session",
                rows.map(row => row.text),
                ["kitty · π - Tune the launcher", "Tune the launcher · TESTQA"]);
            Expect.same("a window with no title attaches only what else it has",
                Said.attached({ app_id: "foot", title: "" }, "Foot", null), []);
            Expect.same("none focused attaches nothing", Said.attached(null, "", null), []);
            Expect.same("his words go first, what's attached under the heading the void reads",
                Said.said("  is this the right fix? ", rows),
                "is this the right fix?\n\nAttached:\n- window: kitty · π - Tune the launcher\n"
                    + "- pi session: Tune the launcher · TESTQA");
            Expect.same("nothing attached sends his words alone",
                Said.said("hello", []), "hello");
            Expect.same("the quiet line names who hears it and the window, the session left to "
                + "the window's title", Said.about("void", rows),
                "void · kitty · π - Tune the launcher");
            Expect.same("with no window it names who hears it alone", Said.about("void", []),
                "void");

            const words = { sent: "sent", taking: "taking it back", taken: "taken back" };
            const since = Date.parse("2026-11-13T09:40:00.000Z");
            const sent = Said.said("file the beacon", rows);
            const talk = Said.begun("VOID", "file the beacon", sent, since);
            Expect.same("Return says it went, under his words", Said.standing(talk, words),
                "sent");
            const delivered = [{ words: sent, at: "2026-11-13T09:40:01.000Z", stage: "delivered",
                reply: "" }];
            Expect.same("a talk not yet replied to changes nothing",
                Said.followed(talk, delivered), null);
            const replied = [Object.assign({}, delivered[0], { stage: "replied",
                reply: "→ Delta · undo 04:58\nFiled as a to-do." })];
            const answered = Said.followed(talk, replied);
            Expect.same("the answer takes sent's place, where it went first, its undo an act",
                [answered.said, Said.standing(answered, words), answered.undo],
                ["answered", "→ Delta\nFiled as a to-do.", "04:58"]);
            Expect.same("the same reply again changes nothing", Said.followed(answered, replied),
                null);
            Expect.same("another talk's reply is never this one's",
                Said.followed(talk, [Object.assign({}, replied[0], { words: "other" })]), null);

            const asked = Said.taking(answered, since + 60000);
            Expect.same(":take tells the void to undo it by its time, and offers it no more",
                [asked.asked, asked.talk.undo, Said.standing(asked.talk, words)],
                ["undo 04:58", "", "taking it back"]);
            Expect.same("nothing to undo takes nothing back",
                Said.taking(Said.followed(talk, [Object.assign({}, replied[0],
                    { reply: "kept" })]), since), null);
            const undone = replied.concat([{ words: "undo 04:58", at: "2026-11-13T09:41:00.000Z",
                stage: "replied", reply: "undone" }]);
            const taken = Said.followed(asked.talk, undone);
            Expect.same("the void's undone reads taken back, in the line",
                [taken.said, Said.standing(taken, words)], ["taken", "taken back"]);
            Expect.same("taken back, it stays so", Said.followed(taken, undone), null);
            const refused = replied.concat([{ words: "undo 04:58",
                at: "2026-11-13T09:41:00.000Z", stage: "replied", reply: "Too late to undo." }]);
            Expect.same("any other answer is what it says, never silent",
                Said.standing(Said.followed(asked.talk, refused), words), "Too late to undo.");
            Expect.same("before any talk it says nothing", Said.standing(null, words), "");

            // Every answer to a line shows under it, the newest in place of one it
            // supersedes, and while someone has it the line says it waits on them
            // (WCJQSG [cf8639ab], Joel at 07:46 on 2026-11-13). The void writes each
            // event as it happens (8GYNJ6 [a9490eb0]).
            const told = { sent: "sent", taken: "taken back", waiting: "waiting on {who}",
                void: "the void", letGo: "{who} let it go" };
            const asking = Said.begun("VOID", "how is it going?", "how is it going?",
                new Date(2026, 10, 13, 7, 42, 10).getTime());
            Expect.same("sent, it waits on the void", Said.exchange(asking, [], told),
                { line: "how is it going?", answers: [], waiting: "waiting on the void",
                    stands: "", by: "", side: "07:42" });
            const routed = Said.followed(asking, [{ words: "how is it going?",
                at: new Date(2026, 10, 13, 7, 42, 20).toISOString(), stage: "replied",
                reply: "Your orchestrator has it \u00b7 undo 07:42" }]);
            Expect.same("its key in the void's day file is the day and the undo's time",
                Said.keyOf(routed), "2026-11-13 07:42");
            Expect.same("an undo's hour said in one figure keys in two",
                Said.keyOf(Object.assign({}, routed, { kept: "9:05" })), "2026-11-13 09:05");
            Expect.same("before the void writes, its answer says who has it, in words",
                Said.exchange(routed, [], told).waiting, "waiting on your orchestrator");
            const written = Said.eventsIn([
                '{"line":"2026-11-13 07:42","msg":"m1","who":"your orchestrator",'
                    + '"event":"routed","at":"07:42"}',
                '{"line":"2026-11-13 07:4',
                '{"line":"2026-11-13 07:43","msg":"m2","who":"your orchestrator",'
                    + '"event":"routed","at":"07:43"}',
                ""].join("\n"));
            Expect.same("a line half written is left out", written.length, 2);
            Expect.same("routed, it waits on who has it", Said.exchange(routed, written, told)
                .waiting, "waiting on your orchestrator");
            const eleven = { line: "2026-11-13 07:42", msg: "m3", re: "m1",
                who: "your orchestrator", event: "answered", at: "07:44",
                text: "Eleven clean updates." };
            const answeredOnce = Said.exchange(routed, written.concat([eleven]), told);
            Expect.same("answered, the answer shows and the wait ends, who answered beside it, "
                    + "and the line still says when he said it",
                [answeredOnce.answers, answeredOnce.waiting, answeredOnce.by, answeredOnce.side],
                [["Eleven clean updates."], "", "your orchestrator", "07:42"]);
            const thirteen = Object.assign({}, eleven, { msg: "m4", supersedes: "m3",
                at: "07:45", text: "Thirteen clean updates." });
            const landed = { line: "2026-11-13 07:42", msg: "m5", who: "your orchestrator",
                event: "landed", at: "07:50", text: "Merged." };
            Expect.same("one that supersedes stands in its place, and a later one under it",
                Said.exchange(routed, written.concat([eleven, landed, thirteen]), told).answers,
                ["Thirteen clean updates.", "Merged."]);
            Expect.same("another line's answers are never this one's",
                Said.exchange(routed, [Object.assign({}, eleven, { line: "2026-11-13 07:43" })],
                    told).answers, []);
            Expect.same("taken up again after an answer, it waits once more",
                Said.exchange(routed, written.concat([eleven, { line: "2026-11-13 07:42",
                    msg: "m6", who: "Delta", event: "taken", at: "07:46" }]), told).waiting,
                "waiting on Delta");
            Expect.same("undone, it reads taken back",
                Said.exchange(routed, written.concat([{ line: "2026-11-13 07:42", msg: "m7",
                    event: "undone", at: "07:47" }]), told).stands, "taken back");
            const toDo = Said.followed(asking, [{ words: "how is it going?",
                at: new Date(2026, 10, 13, 7, 42, 20).toISOString(), stage: "replied",
                reply: "For you: call the bank" }]);
            Expect.same("what can't be undone is the answer itself",
                [Said.exchange(toDo, [], told).answers, Said.exchange(toDo, [], told).waiting],
                [["For you: call the bank"], ""]);
            Expect.same("a line keeps its three newest talks, newest first",
                Said.kept([3, 2, 1], 4), [4, 3, 2]);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
