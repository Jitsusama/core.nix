// How a place stands by its part of the system, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-place-stands-by-its-part`
// check copies modules/quickshell/launcher/SystemPart.js beside this file
// and runs it offscreen. Each case holds VKCF3Z's ruling [a1ef0c0a]: good
// says nothing, not read can't look like good, and what's worth a look says
// the daemon's own side, its mark in the warning's role or the failing one.
import QtQuick
import "Expect.js" as Expect
import "SystemPart.js" as SystemPart

QtObject {
    Component.onCompleted: {
        try {
            run();
            console.log(`a place stands by its part: every case held`);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    function part(health, side, says) {
        return { key: "part:network", kind: "part", part: "network", health, says, side };
    }

    function run() {
        const notRead = "not read";
        const lost = "the modem's downstream channel 16 has lost 878";

        Expect.same("with no daemon answering, the place is as it was",
            SystemPart.standing(part("bad", [lost], "x"), false, notRead), null);
        Expect.same("with the daemon answering but no part yet, the same",
            SystemPart.standing(null, true, notRead), null);
        Expect.same("good says nothing, a holding being the drawer's",
            SystemPart.standing(part("good", [], "15 devices"), true, notRead), null);

        const watch = SystemPart.standing(part("watch", [lost, "and 2 more worth a look"],
            "The modem's downstream channel 16 has lost 878. 2 more are worth a look. 15 devices."),
            true, notRead);
        Expect.same("worth a look says the daemon's side, the first kept longest",
            watch?.sideItems, [{ said: lost, keep: 2 }, { said: "and 2 more worth a look", keep: 1 }]);
        Expect.same("its mark wears the warning's role", watch?.hue, "warning");
        Expect.same("and so does its side", [watch?.warns, watch?.alerts], [true, false]);
        Expect.same("the foot says the whole sentence",
            watch?.state,
            "The modem's downstream channel 16 has lost 878. 2 more are worth a look. 15 devices.");

        const bad = SystemPart.standing(part("bad", ["eero isn't answering"], "eero isn't answering"),
            true, notRead);
        Expect.same("bad wears the role the launcher gives what's failing", bad?.hue, "alert");
        Expect.same("its side too", [bad?.warns, bad?.alerts], [false, true]);

        const unread = SystemPart.standing(part("not read", [], "net couldn't be asked: timed out"),
            true, notRead);
        Expect.same("not read says so, muted, never blank",
            unread?.sideItems, [{ said: notRead, keep: 1, hue: "muted" }]);
        Expect.same("and keeps the place's own mark", unread?.hue, "");
        Expect.same("and its side warns of nothing", [unread?.warns, unread?.alerts],
            [false, false]);
        Expect.same("a health in a word unknown is read as not read",
            SystemPart.standing(part("splendid", [], ""), true, notRead)?.sideItems,
            [{ said: notRead, keep: 1, hue: "muted" }]);

        const unworded = SystemPart.standing(part("watch", undefined, "The modem's lost 878"), true,
            notRead);
        Expect.same("before the daemon words its side, none is made up",
            unworded?.sideItems, []);
        Expect.same("but the mark still warns", unworded?.hue, "warning");

        // A thing's row, as Agents' at home by the mycelium part, keeps
        // its count unless what's wrong is Joel's to act on from it
        // (VKCF3Z, [1130c24f], [b4530faf]).
        const down = "link to studio down";
        const over = SystemPart.overCount(part("bad", [down], "the link to studio is down"),
            true, notRead);
        Expect.same("a concern takes the count's place", over?.sideItems,
            [{ said: down, keep: 1 }]);
        Expect.same("in the failing role, mark and words", [over?.hue, over?.alerts],
            ["alert", true]);
        Expect.same("good keeps the count",
            SystemPart.overCount(part("good", [], "1 link up"), true, notRead), null);
        Expect.same("not read keeps the count, saying nothing it can't act on",
            SystemPart.overCount(part("not read", [], "timed out"), true, notRead), null);
        Expect.same("with no daemon answering, the count is as it was",
            SystemPart.overCount(part("bad", [down], "x"), false, notRead), null);
    }
}
