// How a Spend row reads now while Spend's rows aren't wanted, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `the-desktop-reads-the-spend-watch` check copies
// modules/quickshell/launcher/SpendNow.js and tests/lib/Expect.js beside
// this file and runs it offscreen, before tests/spend-served.qml holds
// Spend.qml's rowNow to the rows `spend-watch serve` serves.
import QtQuick
import "Expect.js" as Expect
import "launcher/SpendNow.js" as SpendNow

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: {
        try {
            cases();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message ?? String(error));
        }
    }

    // Spend standing as `fields` say, over a place serving `served`: by
    // default answering, unshared, its rows not wanted and nothing served.
    function spend(fields, served) {
        const held = served ?? [];
        return Object.assign({
            wanted: false,
            down: false,
            sharing: false,
            answered: true,
            place: "spend",
            serves: record => held.includes(record)
        }, fields);
    }

    function cases() {
        Expect.same("a row it serves reads as served",
            SpendNow.standing("spend:budgets", spend({}, ["row:spend:budgets"])), "served");
        Expect.same("looked for in the place read now, the quiet one while shared",
            SpendNow.standing("spend:budgets",
                spend({ sharing: true, place: "spend-quiet" }, ["row:spend-quiet:budgets"])),
            "served");
        Expect.same("wanted, its rows are as they read now, so it can't say",
            SpendNow.standing("spend:budgets", spend({ wanted: true }, ["row:spend:budgets"])),
            "unsure");
        Expect.same("nor of a key not Spend's", SpendNow.standing("agent:A", spend({})),
            "unsure");
        Expect.same("spend-watch down, Start reads as Spend's down row",
            SpendNow.standing("spend:start", spend({ down: true })), "down");
        Expect.same("and every other as none",
            SpendNow.standing("spend:budgets", spend({ down: true }, ["row:spend:budgets"])),
            "none");
        Expect.same("answered without it, even with no rows, it's none",
            SpendNow.standing("spend:budgets", spend({})), "none");
        Expect.same("before the place's first answer, it can't say",
            SpendNow.standing("spend:budgets", spend({ answered: false })), "unsure");
        // The moment the screen is shared, Spend reads the quiet place, which
        // hasn't answered on this connection yet. Said as can't say, Agents'
        // Budgets would fall back on the rows last made while wanted, and
        // show their amounts on the shared screen (core.nix#6073).
        Expect.same("shared before the quiet place first answers, Budgets is none",
            SpendNow.standing("spend:budgets",
                spend({ sharing: true, answered: false, place: "spend-quiet" })), "none");
        Expect.same("as Spend's record is the place's row of its key",
            SpendNow.recordOf("spend-quiet", "spend:budgets"), "row:spend-quiet:budgets");
    }
}
