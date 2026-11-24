// Where Start New Work's words can be said, its In rows, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `start-new-work-finds-places` check copies
// modules/quickshell/launcher/Wheres.js, what it imports and
// tests/lib/Expect.js beside this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Wheres.js" as Wheres

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    function cases() {
        const quests = [{ folder: "/home/j/src/core", name: "~/src/core" }];
        const pond = { folder: "/home/j/big/src/areas/system/pond",
            name: "//system/pond", kind: "zone", out: false };
        const unchecked = { folder: "/home/j/big/src/areas/system/reef",
            name: "//system/reef", kind: "zone", out: true };
        const harness = { folder: "/home/j/src/harness", name: "~/src/harness",
            kind: "checkout", out: false };

        // Joel, 24 November: he wanted to start in a big repository's zone, and the
        // list offered only his quests' folders.
        Expect.same("the quests' folders, then the node's places, in its order",
            Wheres.wheres(quests, [pond, harness]), [
                { folder: "/home/j/src/core", name: "~/src/core", start: "/home/j/src/core" },
                { folder: pond.folder, name: "//system/pond", start: "//system/pond" },
                { folder: harness.folder, name: "~/src/harness", start: harness.folder }
            ]);
        Expect.same("a zone not checked out is listed, started by its name",
            Wheres.wheres([], [unchecked]),
            [{ folder: unchecked.folder, name: "//system/reef", start: "//system/reef" }]);
        Expect.same("a place at a quest's folder is listed once, as the quest's",
            Wheres.wheres(quests, [{ folder: "/home/j/src/core", name: "core checkout" }]),
            [{ folder: "/home/j/src/core", name: "~/src/core", start: "/home/j/src/core" }]);
        Expect.same("two places at one folder are listed once, the first",
            Wheres.wheres([], [harness, Object.assign({}, harness, { name: "harness" })])
                .map(one => one.name), ["~/src/harness"]);
        Expect.same("a quest's folder that is a zone is started by the zone's name",
            Wheres.wheres([{ folder: unchecked.folder, name: "~/big/reef" }], [unchecked]),
            [{ folder: unchecked.folder, name: "~/big/reef", start: "//system/reef" }]);
        Expect.same("what the node calls it decides, not how its name opens",
            Wheres.wheres([], [Object.assign({}, pond, { kind: "checkout" }),
                Object.assign({}, harness, { name: "harness", kind: "zone" })])
                .map(one => one.start), [pond.folder, "harness"]);
        const otherPond = Object.assign({}, pond, { folder: "/home/j/big2/src/areas/system/pond" });
        Expect.same("a zone two checkouts share is started by each one's folder",
            Wheres.wheres([], [pond, otherPond]).map(one => one.start),
            [pond.folder, otherPond.folder]);

        // The node's words are drawn, so only plain ones are.
        for (const [why, place] of [
            ["a folder that isn't a whole path", { folder: "src/core", name: "core" }],
            ["a nameless one", { folder: "/home/j/src/core", name: "" }],
            ["one named only blanks", { folder: "/home/j/src/core", name: "   " }],
            ["an escape in its name", { folder: "/home/j/a", name: "a\u001b[2Jb" }],
            ["a turn in its name", { folder: "/home/j/a", name: "a\u202eb" }],
            ["a line end in its folder", { folder: "/home/j/a\nb", name: "a" }],
            ["a name that isn't words", { folder: "/home/j/a", name: 7 }],
            ["none at all", null]
        ]) {
            Expect.same(`${why} is left out`, Wheres.listable([place]), []);
        }
        for (const [why, places] of [["no list", null], ["not a list", { folder: "/a" }],
            ["nothing said", undefined]]) {
            Expect.same(`${why} from the node lists the quests' folders alone`,
                Wheres.wheres(quests, Wheres.listable(places)).map(one => one.name),
                ["~/src/core"]);
        }
        Expect.same("no quests' folders list the node's alone",
            Wheres.wheres(null, [harness]).map(one => one.name), ["~/src/harness"]);

        // A node naming thousands can't slow each letter typed.
        const many = Array.from({ length: 2005 },
            (_, at) => ({ folder: `/home/j/src/r${at}`, name: `~/src/r${at}` }));
        const listed = Wheres.wheres(quests, Wheres.listable(many));
        Expect.same("at most 2000 of the node's are listed, beside the quests'",
            [listed.length, listed[listed.length - 1].name], [2001, "~/src/r1999"]);
    }
}
