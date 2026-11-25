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
                Object.assign({}, harness, { kind: "zone" })])
                .map(one => one.start), [pond.folder, harness.folder]);
        Expect.same("a zone whose name the node would take as a folder is started by its folder",
            Wheres.wheres([], [Object.assign({}, harness, { name: "harness", kind: "zone" })])
                .map(one => one.start), [harness.folder]);
        const otherPond = Object.assign({}, pond, { folder: "/home/j/big2/src/areas/system/pond" });
        const big = { folder: "/home/j/big", name: "~/big", kind: "checkout", out: false };
        const big2 = { folder: "/home/j/big2", name: "~/big2", kind: "checkout", out: false };
        Expect.same("a zone two checkouts share is started by each one's folder, named by it",
            Wheres.wheres([], [big, pond, big2, otherPond])
                .filter(one => one.folder.endsWith("/pond"))
                .map(one => [one.name, one.start]), [
                ["~/big/src/areas/system/pond", pond.folder],
                ["~/big2/src/areas/system/pond", otherPond.folder]
            ]);
        Expect.same("one no checkout holds is named by its folder",
            Wheres.wheres([], [pond, otherPond]).map(one => one.name),
            [pond.folder, otherPond.folder]);
        Expect.same("a folder the node calls two zones is the first's, named and started alike",
            Wheres.wheres([], [pond, Object.assign({}, pond, { name: "//system/lake" })]),
            [{ folder: pond.folder, name: "//system/pond", start: "//system/pond" }]);
        Expect.same("a zone's name padded or split by blanks is started by its folder",
            Wheres.wheres([], [Object.assign({}, pond, { name: " //system/pond" }),
                Object.assign({}, unchecked, { name: "//system/reef now" })])
                .map(one => one.start), [pond.folder, unchecked.folder]);
        const otherReef = Object.assign({}, unchecked,
            { folder: "/home/j/big2/src/areas/system/reef" });
        Expect.same("one not checked out that two share is one row, started by name",
            Wheres.wheres([], [unchecked, otherReef]).map(one => one.start),
            ["//system/reef"]);
        Expect.same("a zone the node names twice at one folder is still started by its name",
            Wheres.wheres([], [pond, Object.assign({}, pond)]).map(one => one.start),
            ["//system/pond"]);

        // The node's words are drawn, so only plain ones are.
        for (const [why, place] of [
            ["a folder that isn't a whole path", { folder: "src/core", name: "core" }],
            ["a nameless one", { folder: "/home/j/src/core", name: "" }],
            ["one named only blanks", { folder: "/home/j/src/core", name: "   " }],
            ["an escape in its name", { folder: "/home/j/a", name: "a\u001b[2Jb" }],
            ["a turn in its name", { folder: "/home/j/a", name: "a\u202eb" }],
            ["a line end in its folder", { folder: "/home/j/a\nb", name: "a" }],
            ["a name that isn't words", { folder: "/home/j/a", name: 7 }],
            ["a name of more than 200 characters", { folder: "/home/j/a", name: "a".repeat(201) }],
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
        Expect.same("a name of 200 characters is listed",
            Wheres.listable([{ folder: "/home/j/a", name: "a".repeat(200) }]).length, 1);

        // What each answer to the places ask changes, as Agents.qml's askPlaces keeps it.
        const listing = { places: [harness] };
        Expect.same("an answer to an earlier ask, come after a later one's, is passed over",
            Wheres.heard(2, 1, "", listing), null);
        Expect.same("a list is listed and clears why the last went unlisted",
            Wheres.heard(2, 2, "no home", listing),
            { places: [harness], unlisted: "", journal: null });
        Expect.same("a new why keeps the last list and is journalled",
            Wheres.heard(1, 1, "", { error: { code: "unlisted", message: "no home" } }),
            { places: null, unlisted: "no home", journal: "no home" });
        Expect.same("the same why again isn't journalled again",
            Wheres.heard(1, 1, "no home", { error: { message: "no home" } }).journal, null);
        Expect.same("a refusal giving no why still journals one",
            Wheres.heard(1, 1, "", { error: { message: "" } }).journal,
            "the agents daemon gave no reason");
        Expect.same("no answer at all keeps the last list",
            Wheres.heard(1, 1, "", undefined).places, null);
        Expect.same("no quests' folders list the node's alone",
            Wheres.wheres(null, [harness]).map(one => one.name), ["~/src/harness"]);

        // A node naming thousands can't slow each letter typed.
        const many = Array.from({ length: 2005 },
            (_, at) => ({ folder: `/home/j/src/r${at}`, name: `~/src/r${at}` }));
        const listed = Wheres.wheres(quests, Wheres.listable(many));
        Expect.same("at most 2000 of the node's are listed, beside the quests'",
            [listed.length, listed[listed.length - 1].name], [2001, "~/src/r1999"]);
        Expect.same("and only the first 2000 are read",
            Wheres.listable([null].concat(many)).length, 1999);
    }
}
