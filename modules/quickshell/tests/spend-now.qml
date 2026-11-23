// How a Spend row reads now while Spend's rows aren't wanted, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `the-desktop-reads-the-spend-watch` check copies tests/lib/Expect.js
// beside this file, with modules/quickshell/launcher/SpendNow.js under
// launcher/, and runs it offscreen, before tests/spend-served.qml holds
// Spend.qml's rowNow to the rows `spend-watch serve` serves.
import QtQuick
import "Expect.js" as Expect
import "launcher/SpendNow.js" as SpendNow

QtObject {
    Component.onCompleted: Expect.run(cases)

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

    // Spend standing with only `read` readable: every other of how it
    // stands throws once read, so a case shows the guards before it
    // answered without it.
    function readingOnly(read) {
        const unread = what => {
            throw new Error(`${what} was read`);
        };
        return {
            get wanted() {
                return "wanted" in read ? read.wanted : unread("wanted");
            },
            get down() {
                return "down" in read ? read.down : unread("down");
            },
            get sharing() {
                return "sharing" in read ? read.sharing : unread("sharing");
            },
            get place() {
                return "place" in read ? read.place : unread("place");
            },
            serves: record => "serves" in read ? read.serves(record) : unread("what it serves"),
            get answered() {
                return "answered" in read ? read.answered : unread("whether it answered");
            }
        };
    }

    function cases() {
        Expect.same("a row it serves reads as served",
            SpendNow.standing("spend:budgets", spend({}, ["row:spend:budgets"])), "served");
        Expect.same("looked for in the place it's given, as the quiet one Spend gives while shared",
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
        Expect.same("answered without it, it's none",
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
        // Should Spend's wiring leave sharing unsaid, it's read as shared,
        // so no amount shows.
        Expect.same("sharing unsaid, before the place's first answer, Budgets is none",
            SpendNow.standing("spend:budgets", spend({ sharing: undefined, answered: false })),
            "none");
        // Each guard answers before what follows it is read, so wanted wins
        // over all, a key not Spend's over down, down over what the place
        // serves, that over sharing, and sharing over whether it answered.
        Expect.same("wanted, it reads nothing more",
            SpendNow.standing("spend:budgets", readingOnly({ wanted: true })), "unsure");
        Expect.same("a key not Spend's reads nothing more",
            SpendNow.standing("agent:A", readingOnly({ wanted: false })), "unsure");
        Expect.same("down, Start reads nothing of the place",
            SpendNow.standing("spend:start", readingOnly({ wanted: false, down: true })),
            "down");
        Expect.same("nor does any other",
            SpendNow.standing("spend:budgets", readingOnly({ wanted: false, down: true })),
            "none");
        const up = { wanted: false, down: false, place: "spend" };
        Expect.same("served, it reads nothing of sharing",
            SpendNow.standing("spend:budgets", readingOnly(Object.assign({ serves: () => true },
                up))), "served");
        Expect.same("shared, it reads nothing of whether it answered",
            SpendNow.standing("spend:budgets", readingOnly(Object.assign({ serves: () => false,
                sharing: true }, up))), "none");
        Expect.same("as Spend's record is the place's row of its key",
            SpendNow.recordOf("spend-quiet", "spend:budgets"), "row:spend-quiet:budgets");
    }
}
