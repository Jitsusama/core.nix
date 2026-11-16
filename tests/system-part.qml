// How a place stands by its part of the system, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-place-stands-by-its-part`
// check copies modules/quickshell/launcher/SystemPart.js beside this file
// and runs it offscreen. Each case holds VKCF3Z's ruling [a1ef0c0a]: good
// says nothing, not read can't look like good, and what's worth a look says
// the daemon's own side, its mark in the warning's role or the failing one.
import QtQuick
import "SystemPart.js" as SystemPart

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function check(failures, what, got, want) {
        const said = JSON.stringify(got);
        if (said !== JSON.stringify(want))
            failures.push(`${what}: got ${said}, wanted ${JSON.stringify(want)}`);
    }

    function part(health, side, says) {
        return { key: "part:network", kind: "part", part: "network", health, says, side };
    }

    function run() {
        const failures = [];
        const notRead = "not read";
        const lost = "the modem's downstream channel 16 has lost 878";

        check(failures, "with no daemon answering, the place is as it was",
            SystemPart.standing(part("bad", [lost], "x"), false, notRead), null);
        check(failures, "with the daemon answering but no part yet, the same",
            SystemPart.standing(null, true, notRead), null);
        check(failures, "good says nothing, a holding being the drawer's",
            SystemPart.standing(part("good", [], "15 devices"), true, notRead), null);

        const watch = SystemPart.standing(part("watch", [lost, "and 2 more worth a look"],
            "The modem's downstream channel 16 has lost 878; and 2 more worth a look; 15 devices"),
            true, notRead);
        check(failures, "worth a look says the daemon's side, the first kept longest",
            watch?.sideItems, [{ said: lost, keep: 2 }, { said: "and 2 more worth a look", keep: 1 }]);
        check(failures, "its mark wears the warning's role", watch?.hue, "warning");
        check(failures, "and so does its side", [watch?.warns, watch?.alerts], [true, false]);
        check(failures, "the foot says the whole sentence",
            watch?.state,
            "The modem's downstream channel 16 has lost 878; and 2 more worth a look; 15 devices");

        const bad = SystemPart.standing(part("bad", ["eero isn't answering"], "eero isn't answering"),
            true, notRead);
        check(failures, "bad wears the role the launcher gives what's failing", bad?.hue, "alert");
        check(failures, "its side too", [bad?.warns, bad?.alerts], [false, true]);

        const unread = SystemPart.standing(part("not read", [], "net couldn't be asked: timed out"),
            true, notRead);
        check(failures, "not read says so, muted, never blank",
            unread?.sideItems, [{ said: notRead, keep: 1, hue: "muted" }]);
        check(failures, "and keeps the place's own mark", unread?.hue, "");
        check(failures, "and its side warns of nothing", [unread?.warns, unread?.alerts],
            [false, false]);
        check(failures, "a health in a word unknown is read as not read",
            SystemPart.standing(part("splendid", [], ""), true, notRead)?.sideItems,
            [{ said: notRead, keep: 1, hue: "muted" }]);

        const unworded = SystemPart.standing(part("watch", undefined, "The modem's lost 878"), true,
            notRead);
        check(failures, "before the daemon words its side, none is made up",
            unworded?.sideItems, []);
        check(failures, "but the mark still warns", unworded?.hue, "warning");

        if (failures.length > 0) {
            console.warn(failures.join("\n"));
            Qt.exit(1);
            return;
        }
        console.log(`a place stands by its part: every case held`);
        Qt.exit(0);
    }
}
