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
            const kitty = { app_id: "kitty", title: "π - Bootstrap optimus" };
            const session = { to: "Y2YBTS", name: "Bootstrap optimus" };
            const rows = Said.attached(kitty, "kitty", session);
            Expect.same("a pi window attaches its window and its session",
                rows.map(row => row.text),
                ["kitty · π - Bootstrap optimus", "Bootstrap optimus · Y2YBTS"]);
            Expect.same("a window with no title attaches only what else it has",
                Said.attached({ app_id: "foot", title: "" }, "Foot", null), []);
            Expect.same("none focused attaches nothing", Said.attached(null, "", null), []);
            Expect.same("his words go first, what's attached under the heading the void reads",
                Said.said("  is this the right fix? ", rows),
                "is this the right fix?\n\nAttached:\n- window: kitty · π - Bootstrap optimus\n"
                    + "- pi session: Bootstrap optimus · Y2YBTS");
            Expect.same("nothing attached sends his words alone",
                Said.said("hello", []), "hello");
            Expect.same("the quiet line names who hears it and the window, the session left to "
                + "the window's title", Said.about("void", rows),
                "void · kitty · π - Bootstrap optimus");
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
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
