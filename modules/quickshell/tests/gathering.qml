// How a key's late answers are taken in, run in Qt's own engine, the one
// the shell runs them in. The `late-answers-are-taken-in-once` check copies
// modules/quickshell/launcher/Gathering.qml and tests/lib/Expect.js beside
// this file and runs it offscreen. Three sources answering in one turn
// can't be made to on niri, which holds one that answers late, so each case
// here is answers as Launcher.qml's foundMore hears them, a turn at a time.
import QtQuick
import "Expect.js" as Expect

QtObject {
    id: test

    // What was taken in, in order, as Launcher.qml's moreFoundNow is asked.
    property var taken: []

    property Gathering gathering: Gathering {
        onTakeIn: (fresh, text) => test.taken.push({ fresh: fresh, text: text })
    }

    // Each turn's cases, run once the turn before has ended, as the shell's
    // event loop comes round.
    property var turns: [heardTogether, takenTogether, droppedBySearch, notTakenAfterSearch,
        heardApart, takenApart]
    property int turn: 0

    Component.onCompleted: next()

    // A case that doesn't hold throws, and ends the test as a failure.
    function next() {
        try {
            if (turn === turns.length) {
                console.info(`${turns.length} turns of late answers taken in as they should`);
                Qt.exit(0);
                return;
            }
            turns[turn]();
            turn += 1;
            Qt.callLater(test.next);
        } catch (error) {
            Expect.fail(error.message ?? String(error));
        }
    }

    // Notices, screen text and quests answer "kee" in the one turn, only the
    // second fresh.
    function heardTogether() {
        gathering.found(false, "kee");
        gathering.found(true, "kee");
        gathering.found(false, "kee");
        Expect.same("answers heard in a turn aren't taken in as they come", taken, []);
    }
    function takenTogether() {
        Expect.same("three answers heard in a turn are taken in once, fresh as one came",
            taken, [{ fresh: true, text: "kee" }]);
        taken = [];
    }

    // A key's own search, come after an answer in its turn, took it in.
    function droppedBySearch() {
        gathering.found(true, "keep");
        gathering.drop();
    }
    function notTakenAfterSearch() {
        Expect.same("an answer a key's search took in isn't taken in again", taken, []);
    }

    // Answers a turn apart are each taken in, for what was typed as each came.
    function heardApart() {
        gathering.found(false, "kee");
    }
    function takenApart() {
        Expect.same("an answer heard in its own turn is taken in on its own", taken,
            [{ fresh: false, text: "kee" }]);
        gathering.found(false, "keep");
    }
}
