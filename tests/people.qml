// Drives Tuple's place in the launcher (modules/tuple/People.qml) through
// Quickshell itself, over the real tuple-people (people.rs) reading a
// stand-in for Tuple's `contacts follow`, with niri, the launcher and
// Tuple's calls (Tuple.qml) stood in. The place leads with what can be
// done about calls now: out of a call, opening Joel's room; in one, its
// controls, which typing there finds by name or keyword, while the main
// search leaves them to Calls.qml; and its row says how Tuple stands.
// Nothing follows Tuple until the launcher opens; then with nothing
// typed it lists the favourites and then those called lately, each by who
// can pair first, saying how each stands; typing finds anyone by name or
// email, those Tuple knows first, as they stand, and a stranger saying
// nothing of how they stand; Return calls one who isn't in a call and joins
// one who is; making one a favourite or not tells Tuple and moves their row
// at once; a change of presence changes only how they stand, everyone kept
// as they were; stopping the helper stops Tuple's follow with it; opening
// the launcher again starts it again; once Tuple's follow ends, its
// favourites and those called lately stay, saying nothing of how they
// stand; and with Tuple's daemon stopped, opening the launcher says Tuple
// isn't running, without running it, which the check counts. It exits 0
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
    }

    Launcher.People {
        id: people

        launcher: launcher
        settings: ({ name: "calls" })
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
                people.following.running = false;
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

    // The place's people, after what can be done about calls.
    function folks() {
        return people.everything().filter(row => row.key.startsWith("person:"));
    }

    // Out of a call, Joel's room is first; in one, its controls are, and
    // Tuple's row says so.
    function moments() {
        const idle = people.everything();
        expect("out of a call, the room leads", idle[0].name === "Open My Tuple Room"
            && idle[1].name === "Ada Brook", names(idle.slice(0, 2)));
        expect("asking who to pair with", people.collection.ask === "who to pair with?",
            people.collection.ask);
        const quiet = () => {};
        Tuple.controls = [
            { key: "tuple:mute", verb: "mute", name: "Mute Me in the Tuple Call",
                keywords: "pair microphone voice", glyph: "m", hue: "action", run: quiet },
            { key: "tuple:share", verb: "share", name: "Share the Screen in the Tuple Call",
                keywords: "pair", glyph: "s", hue: "action", run: quiet },
            { key: "tuple:leave", verb: "leave", name: "Leave the Tuple Call",
                keywords: "pair hang up end", glyph: "l", hue: "action", run: quiet }
        ];
        Tuple.calling = true;
        Tuple.standing = "in a call";
        const calling = people.everything();
        expect("in a call, its controls lead", names(calling.slice(0, 4))
            === "Mute Me in the Tuple Call, Share the Screen in the Tuple Call, "
                + "Leave the Tuple Call, Ada Brook", names(calling.slice(0, 4)));
        expect("they're calls", calling[0].what === "call", calling[0].what);
        expect("Tuple's row says it's in a call", people.counted() === "in a call", people.counted());
        expect("asking what to do", people.collection.ask === "what to do in the call?",
            people.collection.ask);
        const leave = people.search("leave").rows;
        expect("typing in the place finds a control", names(leave) === "Leave the Tuple Call",
            names(leave));
        const hang = people.search("hang up").rows;
        expect("and by what else it's called", names(hang) === "Leave the Tuple Call", names(hang));
        expect("the main search leaves controls to their own rows",
            !people.gather("leave").rows.some(row => row.key.startsWith("tuple:")),
            names(people.gather("leave").rows));
        Tuple.controls = [{ key: "tuple:open", verb: "open", name: "Open My Tuple Room",
            keywords: "pair call start new", glyph: "o", hue: "action", run: quiet }];
        Tuple.calling = false;
        Tuple.standing = "";
    }

    function home(name) {
        return people.everything().find(row => row.name === name);
    }

    Component.onCompleted: Qt.callLater(root.opening)

    function opening() {
        if (people.following.running)
            fail("Tuple was followed before the launcher opened");
        else
            launcher.open = true;
    }

    function heardFirst() {
        if (root.waiting !== "the first standing")
            return;
        root.waiting = "the first cases";
        try {
            expect("opening the launcher follows Tuple", people.following.running, "it doesn't");
            expect("everyone", people.everyone.length === 8, people.everyone.length);
            root.moments();
            const open = root.folks();
            expect("favourites, then those called lately, by who can pair first",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore", names(open));
            expect("how each stands", sides(open) === "available, in a call, away, offline", sides(open));
            expect("a favourite wears a star", open[0].glyph === String.fromCodePoint(0xf04ce), open[0].glyph);
            expect("one called lately doesn't", open[3].glyph === String.fromCodePoint(0xf0004), open[3].glyph);
            expect("the count", people.counted() === "1 available", people.counted());

            const eli = people.search("eli@example.com").rows;
            expect("a stranger found by their email", names(eli) === "Eli Fenwick", names(eli));
            expect("who stands unknown says nothing", eli[0].side === "" && eli[0].verb === "call",
                `${eli[0].side} ${eli[0].verb}`);
            const gus = people.search("gus").rows;
            expect("one with only an email, named by it",
                names(gus) === "gus@example.com" && gus[0].side === "", `${names(gus)} ${gus[0]?.side}`);
            const zoe = people.search("quill").rows;
            expect("a name with a quote and more than ASCII",
                names(zoe) === "Zo\u00eb \"Zed\" Quill", names(zoe));
            const ada = people.search("ada").rows;
            expect("one Tuple knows, found as they stand",
                names(ada) === "Ada Brook" && ada[0].side === "available", `${names(ada)} ${ada[0]?.side}`);
            const all = people.search("example").rows;
            expect("those Tuple knows first, then the rest, as Tuple lists them",
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
            expect("calling one who's available",
                Niri.spawned.join(" ") === "tuple call start ada@example.com", Niri.spawned.join(" "));
            expect("one in a call is joined", open[1].verb === "join", open[1].verb);
            open[1].run();
            expect("joining one in a call",
                Niri.spawned.join(" ") === "tuple call join ben@example.com", Niri.spawned.join(" "));
            open[3].run();
            expect("calling one who's offline",
                Niri.spawned.join(" ") === "tuple call start cal@example.com", Niri.spawned.join(" "));
            const hal = people.search("hal").rows;
            expect("one with no email", names(hal) === "Hal", names(hal));
            hal[0].run();
            expect("calling one with no email by their id",
                Niri.spawned.join(" ") === "tuple call start 108", Niri.spawned.join(" "));
            expect("and nothing of theirs to copy",
                names(hal[0].verbs()) === "Call Them, Make Them a Favourite",
                names(hal[0].verbs()));

            const steps = open[0].verbs();
            expect("stepping in",
                names(steps) === "Call Them, Stop Them Being a Favourite, ada@example.com",
                names(steps));
            expect("their email copies itself", steps[2].verb === "copy", steps[2].verb);
            expect("stepping into one in a call", open[1].verbs()[0].name === "Join Their Call",
                open[1].verbs()[0].name);

            const making = eli[0].verbs()[1];
            expect("a stranger can be a favourite", making.name === "Make Them a Favourite",
                making.name);
            making.run();
            expect("telling Tuple of a favourite",
                Niri.spawned.join(" ") === "tuple contacts favorite eli@example.com", Niri.spawned.join(" "));
            const made = root.folks();
            expect("a new favourite at home at once",
                names(made) === "Ada Brook, Ben Carrow, Dee Ellery, Eli Fenwick, Cal Dunmore", names(made));
            expect("with a star", home("Eli Fenwick").glyph === String.fromCodePoint(0xf04ce),
                home("Eli Fenwick").glyph);

            home("Dee Ellery").verbs()[1].run();
            expect("telling Tuple of one no longer a favourite",
                Niri.spawned.join(" ") === "tuple contacts unfavorite dee@example.com", Niri.spawned.join(" "));
            const unmade = root.folks();
            expect("one no longer a favourite moves at once, still saying how they stand",
                names(unmade) === "Ada Brook, Ben Carrow, Eli Fenwick, Cal Dunmore, Dee Ellery"
                    && home("Dee Ellery").side === "away", `${names(unmade)} ${home("Dee Ellery").side}`);

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
            const open = root.folks();
            expect("a favourite gone into a call",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore"
                    && sides(open) === "in a call, in a call, away, offline", `${names(open)}: ${sides(open)}`);
            expect("is joined", open[0].verb === "join", open[0].verb);
            expect("everyone heard once", people.everyone === root.first, "everyone was made again");
            // Found by a letter more, from what the letters before found
            // when Ada was available, she says how she stands now.
            const ada = people.gather("ada").rows;
            expect("a letter more says how they stand now",
                names(ada) === "Ada Brook" && ada[0].side === "in a call", `${names(ada)} ${ada[0]?.side}`);
            expect("the count", people.counted() === "0 available", people.counted());
            present.running = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function stop() {
        if (root.waiting !== "the follow running")
            return;
        root.waiting = "the follow gone";
        people.following.running = false;
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

        function onStoppedChanged() {
            if (root.waiting === "Tuple not running" && people.stopped)
                Qt.callLater(root.notRunning);
        }
    }

    Connections {
        target: people.following

        function onExited() {
            if (root.waiting === "the follow ended")
                Qt.callLater(root.ended);
        }
    }

    function ended() {
        root.waiting = "Tuple not running";
        try {
            const open = root.folks();
            expect("those kept once the follow ended",
                names(open) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore", names(open));
            expect("saying nothing of how they stand", open.every(row => row.side === ""), sides(open));
            expect("a follow ended isn't Tuple stopped", !people.stopped, people.counted());
            launcher.open = false;
            launcher.open = true;
        } catch (error) {
            fail(error.message);
        }
    }

    function notRunning() {
        root.waiting = "the end";
        try {
            expect("the count", people.counted() === "not running", people.counted());
            expect("those kept still", root.folks().length === 4, names(root.folks()));
            Qt.exit(0);
        } catch (error) {
            fail(error.message);
        }
    }
}
