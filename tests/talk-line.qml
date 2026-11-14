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
            // A window as Niri.qml holds it from the windows daemon, its title without the
            // program's name (`named`) and the program's own (ZINAWZ, core #4242).
            const kitty = { app_id: "kitty", title: "π - Tune the launcher",
                named: "π - Tune the launcher", untitled: false, program: "kitty" };
            const session = { to: "TESTQA", name: "Tune the launcher" };
            const rows = Said.attached({ window: kitty, session });
            Expect.same("a pi window attaches its window and its session, each with its mark "
                    + "and its note, the session noted as pi's (VKCF3Z, 2026-11-13 [00c78048])",
                rows.map(row => [row.kind, row.mark, row.text, row.note]),
                [["window", "windows", "Tune the launcher", "kitty"],
                    ["session", "agent", "Tune the launcher", "pi"]]);
            Expect.same("a pi window its session isn't known for yet shows its title without "
                    + "pi's lead", Said.shownRows(Said.attached({ window: kitty }))
                    .map(row => row.text), ["Tune the launcher"]);
            Expect.same("a title that only begins with pi's letter keeps it",
                Said.attached({ window: { title: "π day", named: "π day", program: "Foot" } })[0]
                    .text, "π day");
            const slack = { app_id: "slack", title: "Slack", named: "Slack", untitled: true,
                program: "Slack" };
            Expect.same("a window with no title of its own is said by its program alone, sent "
                    + "as before", Said.attached({ window: slack }).map(row =>
                    [row.text, row.note, row.sent]), [["Slack", "", "Slack · Slack"]]);
            const heard = { app_id: "foot", title: "htop - Foot", named: "htop - Foot",
                untitled: false, program: "" };
            Expect.same("a window niri's stream stands in for, no program said, goes by its "
                    + "app id", Said.attached({ window: heard }).map(row => [row.text, row.note]),
                [["htop - Foot", "foot"]]);
            Expect.same("a window with no title attaches only what else it has",
                Said.attached({ window: { app_id: "foot", title: "", named: "Foot",
                    untitled: true, program: "Foot" } }), []);
            Expect.same("none focused attaches nothing",
                Said.attached({ window: null }), []);
            Expect.same("his words go first, what's attached under the heading the void reads",
                Said.said("  is this the right fix? ", rows),
                "is this the right fix?\n\nAttached:\n- window: kitty · π - Tune the launcher\n"
                    + "- pi session: Tune the launcher · TESTQA");
            Expect.same("nothing attached sends his words alone",
                Said.said("hello", []), "hello");
            Expect.same("a pi window shows its session alone, its title saying the same",
                Said.shownRows(rows).map(row => row.kind), ["session"]);

            // Chrome's tab in front of the window it's said over (brief, slice C), as the
            // programs daemon serves it by niri's window id (FEH1HE, core #4190).
            const chrome = { id: 7, app_id: "google-chrome",
                title: "Pull requests - Google Chrome", named: "Pull requests", untitled: false,
                program: "Google Chrome" };
            Expect.same("the front tab is asked for by its browser and window",
                Said.tabKey("google-chrome", 7), "front:google-chrome:7");
            const tab = Said.tabFrom({ records: [{ key: "front:google-chrome:7", kind: "front",
                app: "google-chrome", window: 7, title: "Pull requests",
                address: "https://github.com/pulls" }] }, "google-chrome", 7);
            Expect.same("the tab its answer names is the window's tab", tab,
                { window: 7, title: "Pull requests", address: "https://github.com/pulls" });
            Expect.same("an answer naming none, as on a tie, gives none",
                Said.tabFrom({ records: [] }, "google-chrome", 7), null);
            Expect.same("an answer for another window gives none",
                Said.tabFrom({ records: [{ key: "front:google-chrome:8", title: "Inbox",
                    address: "https://mail.example" }] }, "google-chrome", 7), null);
            Expect.same("no answer, the daemon not there, gives none",
                Said.tabFrom(null, "google-chrome", 7), null);
            const browsed = Said.attached({ window: chrome, tab });
            Expect.same("a browser's window is titled without its program, and its tab shown "
                    + "in its place", [browsed[0].text, Said.shownRows(browsed).map(row =>
                    [row.mark, row.text, row.note])],
                ["Pull requests", [["web", "Pull requests", "github.com/pulls"]]]);
            Expect.same("the tab goes to the void by its title and address, the window as before",
                Said.said("merge which?", browsed), "merge which?\n\nAttached:\n"
                    + "- window: Google Chrome · Pull requests - Google Chrome\n"
                    + "- tab: Pull requests · https://github.com/pulls");

            // Ctrl+R * and + attach the selection and the clipboard, as Vim's registers.
            const registerWords = { selection: "selection", clipboard: "clipboard",
                lines: "{count} lines" };
            const selected = Said.registered([], "*", "  the beacon's key\n", registerWords);
            Expect.same("* attaches the selection, its words on the line and its kind noted",
                selected.map(row => [row.kind, row.mark, row.text, row.note]),
                [["selection", "lines", "the beacon's key", "selection"]]);
            const both = Said.registered(selected, "+", "\nfn main() {\n    run();\n}",
                registerWords);
            Expect.same("+ attaches the clipboard after it, its first line with words and how "
                + "many lines", both.map(row => [row.kind, row.text, row.note]),
                [["selection", "the beacon's key", "selection"],
                    ["clipboard", "fn main() {", "clipboard · 3 lines"]]);
            Expect.same("the same register again takes its row's place",
                Said.registered(both, "*", "another", registerWords).map(row => row.text),
                ["another", "fn main() {"]);
            Expect.same("an empty register lets go of what it held",
                Said.registered(both, "*", "  \n", registerWords).map(row => row.kind),
                ["clipboard"]);
            Expect.same("any other register attaches nothing",
                Said.registered(both, "a", "text", registerWords), both);
            Expect.same("a register's text goes whole, a line of many under its row, indented",
                Said.said("what's wrong here?", Said.attached({ window: null, registers: both })),
                "what's wrong here?\n\nAttached:\n- selection: the beacon's key\n"
                    + "- clipboard:\n  fn main() {\n      run();\n  }");
            const huge = Said.registered([], "+", "x".repeat(5000), registerWords)[0].sent;
            Expect.same("a huge copy is cut, saying where", [huge.length,
                huge.endsWith("[cut at 4000 characters]")], [4025, true]);

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
            Expect.same("the same words said again later are a line of their own",
                shown(Said.gathered([Said.begun("VOID", "line 17:49", "line 17:49",
                    at(17, 52))], lines, events, seen))[0], "line 17:49");
            Expect.same("and wait while the one before is answered",
                Said.gathered([Said.begun("VOID", "line 17:49", "line 17:49", at(17, 52))],
                    lines, events, seen).length, 4);
            Expect.same("a line said here the void hasn't kept yet waits first",
                shown(Said.gathered([Said.begun("VOID", "new", "new", at(18, 1))], lines,
                    events, seen))[0], "new");
            // The steer inbox can file a later line's reply under an earlier one, which then
            // names the later line's key, and would show its answers (Joel at 23:39 on
            // 2026-11-13: "every entry is supposedly waiting on the same thing").
            const misfiled = [served(18, 56, "the to try view", ""),
                served(18, 58, "the cat drops", "")].map(one => Said.fromServed("VOID",
                Object.assign(one, { reply: "XY9H5C has it \u00b7 undo 18:59" })));
            const routings = [routedAt(18, 57), routedAt(18, 59)];
            Expect.same("each line keeps the void's own routing, never a later line's",
                Said.gathered([], misfiled, routings, {}).map(one => Said.keyOf(one)),
                ["2026-11-13 18:59", "2026-11-13 18:57"]);
            Expect.same("with no routings to read, a line keeps its reply's key",
                Said.gathered([], misfiled, [], {}).map(one => Said.keyOf(one)),
                ["2026-11-13 18:59", "2026-11-13 18:59"]);
            const pickedUp = waiting.concat([{ line: "2026-11-13 16:32", msg: "t1632",
                who: "Your orchestrator", event: "taken", at: "16:40", text: "On it." }]);
            Expect.same("a line taken up whose words he has seen no longer comes first",
                shown(Said.gathered([], lines, pickedUp, seen)),
                ["line 17:49", "line 17:48", "line 17:46"]);
            Expect.same("a talk not delivered, failed without a word back, or :take's asking, "
                + "is no line", [Said.unanswered({ words: "", stage: "", reply: "" }),
                    Said.unanswered({ words: "hi", stage: "failed", reply: "" }),
                    Said.unanswered({ words: "hi", stage: "over", reply: "Kept." }),
                    Said.unanswered({ words: "undo 7:42", stage: "over", reply: "undone" })],
                [true, true, false, true]);
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
