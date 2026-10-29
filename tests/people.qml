// Drives the launcher's People (modules/people/People.qml) with Tuple as
// its source (modules/tuple/TupleSource.qml) through Quickshell itself,
// over the real tuple-people (people.rs) reading a stand-in for Tuple's
// `contacts follow`, with niri and the launcher stood in. Nothing follows
// Tuple until the launcher opens; then with nothing typed People lists
// Tuple's favourites and then those called lately, saying how each
// stands; typing finds anyone by name or email, those listed first, and a
// stranger saying nothing of how they stand; a contact with no email is
// still one person, called by Tuple's id; stepping in, calling one who
// isn't in a call and joining one who is comes first, the Favourite
// switch after, and their address last; flipping Favourite tells Tuple
// and moves their row at once; a change of presence changes only how they
// stand, everyone kept as they were; stopping the helper stops Tuple's
// follow with it; opening the launcher again starts it again; once
// Tuple's follow ends, its favourites and those called lately stay,
// saying nothing of how they stand; and with Tuple's daemon stopped,
// opening the launcher doesn't run it, which the check counts. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string followed: Quickshell.env("FOLLOWED")
    readonly property string stopTuple: Quickshell.env("STOP_TUPLE")
    property string waiting: "the first standing"
    // Everyone as the first people line made them, to tell whether
    // another came.
    property var first: null

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: launcher

        property bool open: false
        property var entered: null

        function enter(row) {
            entered = row;
        }
    }

    Launcher.People {
        id: people

        launcher: launcher
        settings: ({
            name: "people",
            sources: [{ name: "tuple", order: 10 }, { name: "pages", order: 20 }]
        })
        // Each after the change is done, since a case changes them again.
        onAllChanged: {
            if (root.waiting === "the first standing" && people.all.length > 0)
                Qt.callLater(root.heardFirst);
            else if (root.waiting === "the second standing"
                    && people.all.some(row => row.name === "Ada Brook" && row.side === "in a call"))
                Qt.callLater(root.heardSecond);
        }
    }

    // Tuple's next presence, which the stand-in waits for.
    Process {
        id: touching
        command: ["touch", `${root.runtime}/next`]
    }

    // Whether Tuple's stand-in is following, each waiting up to 2 s for it.
    Process {
        id: present
        command: [root.followed, "yes"]
        onExited: code => {
            if (code === 0)
                Qt.callLater(root.stop);
            else
                root.fail("Tuple's follow never ran");
        }
    }

    Process {
        id: gone
        command: [root.followed, "no"]
        onExited: code => {
            if (code === 0)
                Qt.callLater(root.again);
            else
                root.fail("Tuple's follow outlived the helper");
        }
    }

    // Tuple's daemon stopped, which ends its follow.
    Process {
        id: stopping
        command: [root.stopTuple]
        onExited: code => {
            if (code === 0)
                TuplePeople.following.running = false;
            else
                root.fail("the daemon's stand-in wouldn't stop");
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    // Each case has its own 30 s, from when the test starts waiting for it,
    // so a machine slowed by other work fails no case that came in time.
    onWaitingChanged: deadline.restart()

    Timer {
        id: deadline
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function sides(rows) {
        return rows.map(row => row.side).join(", ");
    }

    function home(name) {
        return people.all.find(row => row.name === name);
    }

    // What Return does first inside someone: call or join them.
    function reach(row) {
        row.verbs()[0].run();
        return Niri.spawned.join(" ");
    }

    Component.onCompleted: Qt.callLater(root.opening)

    function opening() {
        if (TuplePeople.following.running)
            fail("Tuple was followed before the launcher opened");
        else
            launcher.open = true;
    }

    function heardFirst() {
        if (root.waiting !== "the first standing")
            return;
        root.waiting = "the first cases";
        try {
            expect("opening the launcher follows Tuple", TuplePeople.following.running,
                "it doesn't");
            expect("everyone, one with no email too", people.everyone.length === 8,
                people.everyone.length);
            const open = people.all;
            expect("favourites, then those called lately",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore", names(open));
            expect("how each stands", sides(open) === "available, in a call, away, offline", sides(open));
            expect("a favourite wears a star", open[0].glyph === String.fromCodePoint(0xf04ce), open[0].glyph);
            expect("one called lately doesn't", open[3].glyph === String.fromCodePoint(0xf0004), open[3].glyph);

            const eli = people.search("eli@example.com").rows;
            expect("a stranger found by their email", names(eli) === "Eli Fenwick", names(eli));
            expect("who stands unknown says nothing", eli[0].side === "", eli[0].side);
            const gus = people.search("gus").rows;
            expect("one with only an email, named by it",
                names(gus) === "gus@example.com" && gus[0].side === "", `${names(gus)} ${gus[0]?.side}`);
            const zoe = people.search("quill").rows;
            expect("a name with a quote and more than ASCII",
                names(zoe) === "Zo\u00eb \"Zed\" Quill", names(zoe));
            const ada = people.search("ada").rows;
            expect("one listed, found as they stand",
                names(ada) === "Ada Brook" && ada[0].side === "available", `${names(ada)} ${ada[0]?.side}`);
            const all = people.search("example").rows;
            expect("those listed first, then the rest, as Tuple lists them",
                names(all) === "Ada Brook, Ben Carrow, Cal Dunmore, Dee Ellery, Eli Fenwick, "
                    + "gus@example.com, Zo\u00eb \"Zed\" Quill", names(all));

            // A letter more searches only those the letters before found:
            // what they found is hidden here, so a search that walked
            // everyone again would still find Ada.
            people.gather("ad");
            expect("what the letters found is kept", names(people.lately.found) === "Ada Brook",
                names(people.lately.found));
            people.lately.found = [];
            expect("a letter more searches only those", people.gather("ada").rows.length === 0,
                names(people.gather("ada").rows));
            expect("other letters search everyone", names(people.gather("ben").rows) === "Ben Carrow",
                names(people.gather("ben").rows));
            expect("a word more is a letter more",
                names(people.gather("ben c").rows) === "Ben Carrow", names(people.gather("ben c").rows));
            expect("a letter taken away searches everyone",
                names(people.gather("be").rows) === "Ben Carrow", names(people.gather("be").rows));
            people.gather("ad");

            open[0].run();
            expect("Return steps in, never calling", launcher.entered === open[0]
                && Niri.spawned.length === 0, Niri.spawned.join(" "));
            expect("calling one who's available",
                reach(open[0]) === "tuple call start ada@example.com", Niri.spawned.join(" "));
            expect("one in a call is joined", open[1].verbs()[0].name === "Join Their Call",
                open[1].verbs()[0].name);
            expect("joining one in a call", reach(open[1]) === "tuple call join ben@example.com",
                Niri.spawned.join(" "));
            expect("calling one who's offline",
                reach(open[3]) === "tuple call start cal@example.com", Niri.spawned.join(" "));
            const hal = people.search("hal").rows;
            expect("one with no email", names(hal) === "Hal", names(hal));
            expect("calling one with no email by their id",
                reach(hal[0]) === "tuple call start 108", Niri.spawned.join(" "));
            expect("and nothing of theirs to copy",
                names(hal[0].verbs()) === "Call Them, Favourite", names(hal[0].verbs()));

            const steps = open[0].verbs();
            expect("stepping in, their page after the ways to reach them, the address last",
                names(steps) === "Call Them, Open in Pages, Favourite, ada@example.com",
                names(steps));
            steps[1].run();
            expect("their page opens", steps[1].verb === "open" && Niri.spawned.join(" ")
                === "xdg-open https://pages.example.com/ada", Niri.spawned.join(" "));
            expect("one with no page has no row for it",
                !names(open[1].verbs()).includes("Open in"), names(open[1].verbs()));
            expect("a source's facts are theirs",
                open[0].drawer.facts.some(fact => fact.label === "Team" && fact.value === "Shell"),
                JSON.stringify(open[0].drawer));
            expect("a favourite says so", steps[2].side === "yes" && steps[2].verb === "unstar",
                `${steps[2].side} ${steps[2].verb}`);
            expect("their email copies itself", steps[3].verb === "copy", steps[3].verb);

            const making = eli[0].verbs()[1];
            expect("a stranger isn't a favourite", making.name === "Favourite"
                && making.side === "no" && making.verb === "star", `${making.side} ${making.verb}`);
            making.run();
            expect("telling Tuple of a favourite",
                Niri.spawned.join(" ") === "tuple contacts favorite eli@example.com", Niri.spawned.join(" "));
            expect("the switch says so at once", making.side === "yes", making.side);
            expect("a new favourite at home at once",
                names(people.all) === "Ada Brook, Ben Carrow, Dee Ellery, Eli Fenwick, Cal Dunmore",
                names(people.all));
            expect("with a star", home("Eli Fenwick").glyph === String.fromCodePoint(0xf04ce),
                home("Eli Fenwick").glyph);

            home("Dee Ellery").verbs()[1].run();
            expect("telling Tuple of one no longer a favourite",
                Niri.spawned.join(" ") === "tuple contacts unfavorite dee@example.com", Niri.spawned.join(" "));
            expect("one no longer a favourite, with nothing else between them, isn't listed",
                names(people.all) === "Ada Brook, Ben Carrow, Eli Fenwick, Cal Dunmore",
                names(people.all));

            root.first = people.everyone;
            root.waiting = "the second standing";
            touching.running = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function heardSecond() {
        if (root.waiting !== "the second standing")
            return;
        root.waiting = "the follow running";
        try {
            // As Tuple says them now, which never heard of the favourites
            // changed here.
            const open = people.all;
            expect("a favourite gone into a call",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore"
                    && sides(open) === "in a call, in a call, away, offline", `${names(open)}: ${sides(open)}`);
            expect("is joined", open[0].verbs()[0].name === "Join Their Call",
                open[0].verbs()[0].name);
            expect("everyone heard once", people.everyone === root.first, "everyone was made again");
            // Found by a letter more, from what the letters before found
            // when Ada was available, she says how she stands now.
            const ada = people.gather("ada").rows;
            expect("a letter more says how they stand now",
                names(ada) === "Ada Brook" && ada[0].side === "in a call", `${names(ada)} ${ada[0]?.side}`);
            present.running = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function stop() {
        if (root.waiting !== "the follow running")
            return;
        root.waiting = "the follow gone";
        TuplePeople.following.running = false;
        gone.running = true;
    }

    function again() {
        if (root.waiting !== "the follow gone")
            return;
        root.waiting = "everyone again";
        launcher.open = false;
        launcher.open = true;
    }

    // Opening again follows Tuple afresh, which says everyone again.
    Connections {
        target: people

        function onEveryoneChanged() {
            if (root.waiting === "everyone again" && people.everyone !== root.first) {
                root.waiting = "the follow ended";
                stopping.running = true;
            }
        }
    }

    Connections {
        target: TuplePeople

        function onStoppedChanged() {
            if (root.waiting === "Tuple not running" && TuplePeople.stopped)
                Qt.callLater(root.notRunning);
        }
    }

    Connections {
        target: TuplePeople.following

        function onExited() {
            if (root.waiting === "the follow ended")
                Qt.callLater(root.ended);
        }
    }

    function ended() {
        root.waiting = "Tuple not running";
        try {
            const open = people.all;
            expect("those kept once the follow ended",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore", names(open));
            expect("saying nothing of how they stand", open.every(row => row.side === ""), sides(open));
            expect("a follow ended isn't Tuple stopped", !TuplePeople.stopped, "it is");
            launcher.open = false;
            launcher.open = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function notRunning() {
        root.waiting = "the end";
        try {
            expect("those kept still", people.all.length === 4, names(people.all));
            Qt.exit(0);
        } catch (error) {
            fail(error.message);
        }
    }
}
