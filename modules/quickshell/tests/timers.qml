// Drives the launcher's timers (modules/quickshell/launcher/Timers.qml)
// through Quickshell itself, over a timer as the timers daemon serves it,
// with HeldTimers.qml stood in. A timer's row is keyed as its record is,
// but never as one of the launcher's own kinds, while its acts still ask
// the daemon by the record's own key; and an act the daemon refuses, with
// no row of it held becoming, is told in the foot by the timer's name and
// what wasn't done, a typed start's and the stopwatch's too, rather than
// dropped (launcher/Becoming.js's refusal).
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "launcher/Becoming.js" as Becoming
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property var same: Expect.same

    // The launcher as Timers asks it, shut. A refusal is decided as the
    // launcher decides it, over no row held becoming, and what's told is
    // worded as its toldNot words it.
    QtObject {
        id: theLauncher

        property bool open: false
        // Each refusal, as its row's key, why, and the name and act given.
        property var refusals: []
        // What the foot was told wasn't done, as "what · why".
        property var told: []

        function wants(source) {
            return true;
        }

        function becameNot(key, why, asked) {
            refusals = refusals.concat([[key, why, asked?.name ?? null, asked?.act ?? null]]);
            const refusal = Becoming.refusal({}, key, asked);
            if (refusal?.told)
                toldNot(refusal.told.name, refusal.told.act, why);
        }

        function toldNot(name, act, why) {
            const words = Words.field.becoming ?? {};
            const what = words.refused.replace("{object}", name)
                .replace("{did}", Becoming.did(words.did, act, words.done ?? ""));
            told = told.concat([`${what} \u00b7 ${why}`]);
        }
    }

    Launcher.Timers {
        id: timers

        launcher: theLauncher
        settings: ({ name: "timers" })
    }

    Component.onCompleted: Qt.callLater(() => Expect.run(root.cases))

    // A timer the daemon keeps under a key shaped as one of the launcher's
    // own, running ten minutes from now.
    function claiming(state) {
        const now = Date.now();
        return { key: "place:x", kind: "timer", name: "tea", state: state, span: 600000,
            set: now, ends: now + 600000, left: 600000 };
    }

    function cases() {
        HeldTimers.timers = [claiming("running")];
        const row = timers.everything()[0];
        same("a timer claiming a place is kept under the node's row:", row.key, "row:place:x");

        // Return pauses it, asked by the record's own key, and the daemon's
        // refusal is said in the foot in the vocabulary's words.
        HeldTimers.refusal = { error: "not running", reason: "pausesRunning" };
        row.run();
        same("its pause asks the daemon by the record's key", HeldTimers.asked,
            ["pause place:x"]);
        same("refused, by the row's key, with its name and act", theLauncher.refusals,
            [["row:place:x", "only a running timer pauses", "Tea", "pause"]]);
        same("and told in the foot, though nothing held it becoming", theLauncher.told,
            ["Tea not paused \u00b7 only a running timer pauses"]);

        // Each of its acts inside is told so too, End's on a timer gone.
        theLauncher.told = [];
        HeldTimers.refusal = { error: "no such timer", reason: "gone" };
        for (const act of row.verbs().filter(one => typeof one.run === "function"
            && one.key !== "verb:timer:copy"))
            act.run();
        same("every act inside is told, by what it would have done", theLauncher.told, [
            "Tea not paused \u00b7 that timer is gone",
            "Tea not started again \u00b7 that timer is gone",
            "Tea not given a minute \u00b7 that timer is gone",
            "Tea not ended \u00b7 that timer is gone"
        ]);

        // A typed start the daemon isn't there for is told by the timer's
        // name, as its answer named it.
        theLauncher.told = [];
        HeldTimers.started = null;
        timers.answer("timer 10m tea").run();
        same("a typed start with the daemon away is told", theLauncher.told,
            ["Tea not started \u00b7 the timers aren't answering"]);

        // The stopwatch's Return, refused, is told as the Start its row says.
        theLauncher.told = [];
        HeldTimers.refusal = { error: "running", reason: "runningAlready" };
        HeldTimers.stopwatch = { key: "stopwatch", kind: "stopwatch", state: "unstarted",
            counted: 0, laps: [] };
        timers.everything().find(one => one.key === "stopwatch").run();
        same("the stopwatch's refusal is told", theLauncher.told,
            ["Stopwatch not started \u00b7 the stopwatch is running already"]);

        // Taken, nothing is said.
        theLauncher.told = [];
        HeldTimers.refusal = claiming("paused");
        row.run();
        same("an act taken says nothing", theLauncher.told, []);
    }
}
