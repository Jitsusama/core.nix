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
        // Its act is keyed by its folder (Agents.qml's newWorkAct), so Joel stays on it
        // when a list landing changes how it starts.
        Expect.same("each row's folder is its own, however the places say to start",
            [[], [unchecked], [unchecked, Object.assign({}, unchecked, { name: "//system/lake" })],
                [pond, Object.assign({}, pond, { folder: "/home/j/big2/src/areas/system/pond" })]]
                .map(places => Wheres.wheres(quests.concat([{ folder: unchecked.folder,
                    name: "~/big/reef" }]), places).map(one => one.folder))
                .map(folders => folders.length === new Set(folders).size
                    && folders.includes(unchecked.folder)), [true, true, true, true]);
        // An In row's field entered before the places landed starts at Return as its
        // row now says (Agents.qml's newWorkAct).
        // Rows picked by their own folder, the one holding the rest listed first so a
        // match by prefix shows, and made as other than they start, so keeping the
        // made start or falling back to the folder shows.
        const system = { folder: "/home/j/big/src/areas/system", name: "~/big/system" };
        const reefQuest = [system, { folder: unchecked.folder, name: "~/big/reef" },
            { folder: pond.folder, name: "~/big/pond" }];
        const madeAs = Wheres.wheres(reefQuest, []).find(one => one.folder === unchecked.folder)
            .start;
        Expect.same("before the places land, a quest's folder starts by its folder", madeAs,
            unchecked.folder);
        const landed = Wheres.wheres(reefQuest, [unchecked, pond]);
        Expect.same("a field entered before places land starts as its row says after",
            Wheres.startAt(landed, unchecked.folder, madeAs), "//system/reef");
        Expect.same("and the other row as its own says",
            Wheres.startAt(landed, pond.folder, "made"), "//system/pond");
        Expect.same("and the folder holding them as its own",
            Wheres.startAt(landed, system.folder, "made"), system.folder);
        Expect.same("and as its own where a row under it is listed first",
            Wheres.startAt(Wheres.wheres(reefQuest.slice(1).concat([system]), [unchecked]),
                system.folder, "made"), system.folder);
        // Listed around it but not at it, none starting as it was made: the folder
        // holding it, one under it, one sharing its last parts, one ending in its whole
        // folder, one its folder ends in, one named as it was made, and a zone last.
        const around = Wheres.wheres([system,
            { folder: unchecked.folder + "/deep", name: "~/big/reef/deep" },
            { folder: "/home/j/big2/src/areas/system/reef", name: "~/big2/reef" },
            { folder: `/mnt${unchecked.folder}`, name: "/mnt/reef" },
            { folder: "/areas/system/reef", name: "/areas/reef" },
            { folder: "/home/j/big/src/areas/system/lake", name: "//made/start" }], [pond]);
        Expect.same("none near it starts as it was made",
            around.filter(one => one.start === "//made/start"), []);
        Expect.same("and as made where its folder is no longer listed, whatever is near it",
            Wheres.startAt(around, unchecked.folder, "//made/start"), "//made/start");
        Expect.same("and its own folder's start where another row starts as it was made",
            Wheres.startAt(landed, pond.folder, "//system/reef"), "//system/pond");
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
        // A row made to start by a zone's name starts by its folder once a second
        // checkout of that zone lands, so the node can't pick the other.
        const madeByName = Wheres.wheres([], [big, pond]).find(one => one.folder === pond.folder)
            .start;
        Expect.same("one checkout's zone starts by its name", madeByName, "//system/pond");
        Expect.same("and by its folder at Return once another checkout holds it too",
            Wheres.startAt(Wheres.wheres([], [big, pond, big2, otherPond]), pond.folder,
                madeByName), pond.folder);
        Expect.same("and the other checkout's by its own folder",
            Wheres.startAt(Wheres.wheres([], [big, pond, big2, otherPond]), otherPond.folder,
                madeByName), otherPond.folder);
        Expect.same("one no checkout holds is named by its folder",
            Wheres.wheres([], [pond, otherPond]).map(one => one.name),
            [pond.folder, otherPond.folder]);
        Expect.same("a folder the node calls two zones is the first's, named and started alike",
            Wheres.wheres([], [pond, Object.assign({}, pond, { name: "//system/lake" })]),
            [{ folder: pond.folder, name: "//system/pond", start: "//system/pond" }]);
        Expect.same("a checked-out zone's name padded by blanks is started by its folder",
            Wheres.wheres([], [Object.assign({}, pond, { name: " //system/pond" })])
                .map(one => one.start), [pond.folder]);
        Expect.same("one not checked out that can't be started by its name isn't listed",
            Wheres.wheres([], [Object.assign({}, unchecked, { name: "//system/reef now" }),
                Object.assign({}, unchecked, { name: "reef" })]), []);
        Expect.same("but a quest's folder there still is, as the quest's",
            Wheres.wheres([{ folder: unchecked.folder, name: "~/big/reef" }],
                [Object.assign({}, unchecked, { name: "reef" })]).map(one => one.start),
            [unchecked.folder]);
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
        Expect.same("a list is listed and clears why the last went unlisted",
            Wheres.heard("no home", listing),
            { places: [harness], unlisted: "", journal: null });
        Expect.same("a new why keeps the last list and is journalled",
            Wheres.heard("", { error: { code: "unlisted", message: "no home" } }),
            { places: null, unlisted: "no home", journal: "no home" });
        Expect.same("the same why again isn't journalled again",
            Wheres.heard("no home", { error: { message: "no home" } }).journal, null);
        Expect.same("a refusal giving no why still journals one",
            Wheres.heard("", { error: { message: "" } }).journal,
            "the agents daemon gave no reason");
        Expect.same("no answer at all keeps the last list, and says the daemon isn't answering",
            Wheres.heard("", null),
            { places: null, unlisted: "the agents daemon isn't answering",
                journal: "the agents daemon isn't answering" });
        Expect.same("a list the same as the last changes nothing drawn",
            Wheres.differs([harness], Wheres.listable([Object.assign({}, harness)])), false);
        Expect.same("a list with a place more is drawn again",
            Wheres.differs([harness], [harness, pond]), true);
        Expect.same("the first list, after none, is drawn", Wheres.differs([], [harness]), true);
        Expect.same("a zone checked out since, the list as long, is drawn again",
            Wheres.differs([pond, unchecked], [pond, Object.assign({}, unchecked, { out: false })]),
            true);
        Expect.same("and so is a place renamed",
            Wheres.differs([pond], [Object.assign({}, pond, { name: "//system/lake" })]), true);
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

        // Joel, 25 November: For You "takes forever to load". Its opening asked for the
        // places, and reading a node's 2000 a character at a time held the shell's thread 5 to
        // 7 s, by his journal; his node's run to 102 characters a place, as these do.
        const node = Array.from({ length: 2000 }, (_, at) => ({
            folder: `/home/j/big/src/areas/clients/team-${at}/service-${at}/components/main`,
            name: `//clients/team-${at}/service-${at}/main`, kind: "zone", out: false
        }));
        // The fastest of 5, so a builder busy with VM tests or a collection
        // in Qt's engine doesn't fail it; rounds stop only past 5 s, which
        // the old reading took on its first.
        const spans = [];
        for (let round = 0; round < 5 && !spans.some(span => span > 5000); round += 1) {
            const begun = Date.now();
            Wheres.differs(Wheres.heard("", { places: node }).places, node);
            spans.push(Date.now() - begun);
        }
        Expect.same("a node's 2000 places are read and weighed in under 50 ms, the fastest of 5",
            Math.min(...spans) < 50 ? "under 50 ms" : `${spans.join(", ")} ms`, "under 50 ms");
        Expect.same("and a place whose name holds a turn among them is still left out",
            Wheres.listable(node.slice(0, 9).concat([{ folder: "/home/j/a", name: "a\u202eb" }]))
                .length, 9);
    }
}
