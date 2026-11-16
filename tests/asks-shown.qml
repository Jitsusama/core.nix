// How long a secret ask's line was on Joel's screen when he said yes, as
// the yes tells the daemon, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `the-asks-say-how-long-each-line-was-shown` check
// copies modules/secret-asks/Shown.js and tests/lib/Expect.js beside this
// file and runs `qml -platform offscreen asks-shown.qml` over asks made up
// here in the form the daemon serves them.
import QtQuick
import "launcher/Shown.js" as Shown
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            unshown();
            shown();
            changed();
            outside();
            refused();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    readonly property string key: "row:asks:0123456789abcdef0123456789abcdef"

    function yes(digest) {
        return { ask: "release", key: "ask:0123456789abcdef0123456789abcdef", digest: digest };
    }

    // The launcher at a moment of the frames' clock, in its third opening.
    function at(now, cataloguing) {
        return { now: now, opening: 3, cataloguing: cataloguing === true };
    }

    function unshown() {
        const said = Shown.stamped(yes("d1"), {}, key, at(5000));
        Expect.same("a yes for a line never presented says no time, so it's refused unread",
            said.shown, undefined);
        Expect.same("and is otherwise sent as served", said.digest, "d1");
        const forged = Object.assign(yes("d1"), { shown: 9000, opening: 3 });
        const sent = Shown.stamped(forged, {}, key, at(5000));
        Expect.same("a time the served ask held is never sent", sent.shown, undefined);
        Expect.same("nor an opening", sent.opening, undefined);
    }

    function shown() {
        const seen = Shown.noted({}, key, "d1", 1200);
        const said = Shown.stamped(yes("d1"), seen, key, at(2012));
        Expect.same("a yes says how long since its line's frame reached the screen", said.shown,
            812);
        Expect.same("and which opening it was shown in", said.opening, 3);
        Expect.same("the served ask is left as it was", yes("d1").shown, undefined);
        const again = Shown.noted(seen, key, "d1", 1900);
        Expect.same("a line presented again counts from its first frame", again, seen);
        Expect.same("so the time isn't restarted", Shown.stamped(yes("d1"), again, key,
            at(2012)).shown, 812);
        Expect.same("a frame presented after the yes's moment says none sooner",
            Shown.stamped(yes("d1"), Shown.noted({}, key, "d1", 2500), key, at(2012)).shown, 0);
    }

    function changed() {
        const seen = Shown.noted({}, key, "d1", 1200);
        const moved = Shown.noted(seen, key, "d2", 4000);
        Expect.same("a line that changed counts again from its new frame",
            Shown.stamped(yes("d2"), moved, key, at(4300)).shown, 300);
        Expect.same("a yes for the line as it was says no time", Shown.stamped(yes("d1"), moved,
            key, at(4300)).shown, undefined);
        Expect.same("nor does a yes for another ask's line", Shown.stamped(yes("d1"), seen,
            "row:asks:ffffffffffffffffffffffffffffffff", at(4300)).shown, undefined);
    }

    function outside() {
        const seen = Shown.noted({}, key, "d1", 1200);
        Expect.same("a yes while the launcher is walked from outside isn't sent at all",
            Shown.stamped(yes("d1"), seen, key, at(9000, true)), null);
    }

    function refused() {
        const no = { ask: "refuse", key: "ask:0123456789abcdef0123456789abcdef", digest: "d1" };
        Expect.same("a no is sent as served, needing nothing read", Shown.stamped(no, {}, key,
            at(5000, true)), no);
    }
}
