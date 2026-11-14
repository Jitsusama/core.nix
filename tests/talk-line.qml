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
            const written = Said.eventsOf([
                { line: "2026-11-13 07:42", msg: "m1", who: "your orchestrator",
                    event: "routed", at: "07:42" },
                '{"line":"2026-11-13 07:4',
                '{"line":"2026-11-13 07:43","msg":"m2","who":"your orchestrator",'
                    + '"event":"routed","at":"07:43"}']);
            Expect.same("the node's lines are read whole or from their text, and a line half "
                + "written is left out", written.map(event => event.msg), ["m1", "m2"]);
            Expect.same("no lines served, no events", Said.eventsOf(undefined), []);
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
            // Joel's 16:32 line on 13 November: his orchestrator took it up twice, saying what
            // it would do and then what it did, and the line showed neither (QCXZEB [203d9e2c]).
            const takenUp = [{ line: "2026-11-13 07:42", msg: "m8", who: "Your orchestrator",
                event: "routed", at: "07:42" }, { line: "2026-11-13 07:42", msg: "m9",
                who: "Your orchestrator", event: "taken", at: "07:46", text: "I have it." },
                { line: "2026-11-13 07:42", msg: "m10", who: "Your orchestrator",
                    event: "taken", at: "07:50", text: "Moved the windows back." }];
            const tookUp = Said.exchange(routed, takenUp, told);
            Expect.same("taken up with words, they show and it still waits on who has it, "
                    + "named as mid-sentence", [tookUp.answers, tookUp.waiting, tookUp.by],
                [["I have it.", "Moved the windows back."], "waiting on your orchestrator",
                    "your orchestrator"]);
            Expect.same("undone, it reads taken back",
                Said.exchange(routed, written.concat([{ line: "2026-11-13 07:42", msg: "m7",
                    event: "undone", at: "07:47" }]), told).stands, "taken back");
            const toDo = Said.followed(asking, [{ words: "how is it going?",
                at: new Date(2026, 10, 13, 7, 42, 20).toISOString(), stage: "replied",
                reply: "For you: call the bank" }]);
            Expect.same("what can't be undone is the answer itself",
                [Said.exchange(toDo, [], told).answers, Said.exchange(toDo, [], told).waiting],
                [["For you: call the bank"], ""]);
            Expect.same("said on an earlier day, its side says which",
                Said.exchange(routed, [], told, () => "yesterday").side, "yesterday 07:42");

            // The void keeps every talk on disk and the node serves them, so the line is made
            // from them and a restart, or a new day, hides nothing (WCJQSG [a2eb9575], Joel
            // at 18:05 on 2026-11-13: "now going in shows me nothing").
            const at = (hour, minute) => new Date(2026, 10, 13, hour, minute, 10).getTime();
            const served = (hour, minute, words, reply) => ({ id: `talk-${at(hour, minute)}`,
                at: new Date(at(hour, minute) + 500).toISOString(), words, stage: "over",
                reply: `Your orchestrator has it \u00b7 undo ${hour}:${minute}` + (reply ?? "") });
            const kept = Said.fromServed("VOID", served(16, 32,
                "windows everywhere\n\nAttached:\n- window: kitty \u00b7 pi"));
            Expect.same("a kept talk is his words, said when its id says, keyed by its undo",
                [kept.line, kept.since, Said.keyOf(kept), kept.said],
                ["windows everywhere", at(16, 32), "2026-11-13 16:32", "answered"]);
            const routedAt = (hour, minute) => ({ line: `2026-11-13 ${hour}:${minute}`,
                msg: `r${hour}${minute}`, who: "Your orchestrator", event: "routed",
                at: `${hour}:${minute}` });
            const answer = (hour, minute) => ({ line: `2026-11-13 ${hour}:${minute}`,
                msg: `a${hour}${minute}`, who: "Your orchestrator", event: "answered",
                at: `${hour}:${minute}`, text: "Done." });
            const lines = [[16, 32], [17, 45], [17, 46], [17, 48], [17, 49]]
                .map(([hour, minute]) => Said.fromServed("VOID",
                    served(hour, minute, `line ${hour}:${minute}`)));
            const allAnswered = lines.map(one => answer(...Said.keyOf(one).slice(11).split(":")
                .map(Number)));
            const events = lines.map(one => routedAt(...Said.keyOf(one).slice(11).split(":")
                .map(Number))).concat(allAnswered);
            const shown = shownLines => shownLines.map(one => one.line);
            Expect.same("five answered lines he hasn't seen all show, newest first",
                shown(Said.gathered([], lines, events, {})),
                ["line 17:49", "line 17:48", "line 17:46", "line 17:45", "line 16:32"]);
            const seen = Said.seenAfter({}, lines, events, at(18, 0));
            Expect.same("once seen, the newest three show",
                shown(Said.gathered([], lines, events, seen)),
                ["line 17:49", "line 17:48", "line 17:46"]);
            const waiting = events.filter(event => event.msg !== "a1632");
            Expect.same("one still waiting shows first, seen or not",
                shown(Said.gathered([], lines, waiting, seen)),
                ["line 16:32", "line 17:49", "line 17:48", "line 17:46"]);
            const later = events.concat([Object.assign(answer(17, 45), { msg: "b1745",
                text: "And merged." })]);
            Expect.same("a new answer to a seen line brings it back first",
                shown(Said.gathered([], lines, later, seen)),
                ["line 17:45", "line 17:49", "line 17:48", "line 17:46"]);
            const yesterday = Said.fromServed("VOID", { id: `talk-${at(9, 5) - 86400000}`,
                words: "from yesterday", stage: "over",
                reply: "Your orchestrator has it \u00b7 undo 9:05" });
            Expect.same("an answer from an earlier day he hasn't seen still shows",
                shown(Said.gathered([], lines.concat([yesterday]), events.concat([
                    { line: "2026-11-12 09:05", msg: "y", who: "Delta", event: "answered",
                        at: "09:10", text: "Fixed." }]), seen)),
                ["from yesterday", "line 17:49", "line 17:48", "line 17:46"]);
            const justSaid = Said.begun("VOID", "line 17:49", "line 17:49", at(17, 49) - 900);
            Expect.same("a line said here and kept by the void shows once",
                shown(Said.gathered([justSaid], lines, events, seen)).length, 3);
            Expect.same("a line said here the void hasn't kept yet waits first",
                shown(Said.gathered([Said.begun("VOID", "new", "new", at(18, 1))], lines,
                    events, seen))[0], "new");
            Expect.same("a talk that failed without a word back, or :take's asking, is no line",
                [Said.unanswered({ stage: "failed", reply: "" }),
                    Said.unanswered({ stage: "over", reply: "Kept." }),
                    Said.unanswered({ words: "undo 7:42", stage: "over", reply: "undone" })],
                [true, false, true]);
            Expect.same("seen is kept a month",
                Object.keys(Said.seenAfter({ "2026-10-01 09:00": 1, "2026-11-12 09:05": 2 }, [],
                    [], at(18, 0))), ["2026-11-12 09:05"]);
            Expect.same("a missing or broken file of what's seen is nothing seen",
                [Said.seenIn(""), Said.seenIn("{\"2026"), Said.seenIn("[1]")], [{}, {}, {}]);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
