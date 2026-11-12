// How a row that ends the session arms before it's done, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `an-ending-row-arms-before-it-ends` check copies Arming.js beside this
// file and runs `qml -platform offscreen arming.qml`. Each case presses
// Return or clicks on a row, as the launcher hears them, and says what each
// press did and whether the row stands armed after it.
import QtQuick
import "Arming.js" as Arming

QtObject {
    // A Return's doubled press comes within this many ms of the first
    // (the key language's wait).
    readonly property int wait: 300

    // What a run of presses did, each { id, keys, at, counted } with the
    // keys the line had taken by then, said as "armed", "ends" or "" for
    // a doubled one, and whether `id` stood armed once they were done.
    function presses(steps) {
        let armed = Arming.rest();
        const acts = [];
        for (const step of steps) {
            const did = Arming.pressed(armed, step.id, { keysTaken: step.keys,
                counted: step.counted ?? 1, now: step.at, wait });
            armed = did.armed;
            acts.push(did.act);
        }
        const last = steps[steps.length - 1];
        return `${acts.join(" ")} | ${Arming.holds(armed, last.id, last.keys) ? "armed" : "not"}`;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            if (got !== wanted)
                failures.push(`${what} gave ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };

        expect("nothing is armed at rest", Arming.holds(Arming.rest(), "", 0), false);
        // The first Return arms it and leaves it armed; the next, with no
        // key between, does it.
        expect("one Return arms", presses([{ id: "reboot", keys: 5, at: 0 }]), "armed | armed");
        expect("a second Return does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 2000 }]),
            "armed ends | not");
        // However long after: there's no time limit.
        expect("a second Return an hour later does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 3600000 }]),
            "armed ends | not");
        // Any other key lets it go, so the Return after arms it again.
        expect("a key between lets it go",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 7, at: 2000 }]),
            "armed armed | armed");
        expect("any key lets it stand unarmed",
            Arming.holds(Arming.pressed(Arming.rest(), "reboot", { keysTaken: 5, counted: 1,
                now: 0, wait }).armed, "reboot", 6), false);
        // A Return on another row arms that one instead.
        expect("another row arms instead",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "logout", keys: 6, at: 2000 }]),
            "armed armed | armed");
        // A Return doubled with the first, a key bounced, does nothing and
        // leaves it armed for a deliberate second.
        expect("a doubled Return does nothing",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 100 }]),
            "armed  | armed");
        expect("a Return after the doubled one does it",
            presses([{ id: "reboot", keys: 5, at: 0 }, { id: "reboot", keys: 6, at: 100 },
                { id: "reboot", keys: 7, at: 2000 }]),
            "armed  ends | not");
        // A click counts no key, so a click on the row a Return armed does
        // it, and a second click does what a second Return does.
        expect("a click after a Return does it",
            presses([{ id: "reboot", keys: 5, at: 0 },
                { id: "reboot", keys: 5, at: 2000, counted: 0 }]),
            "armed ends | not");
        expect("two clicks do it",
            presses([{ id: "reboot", keys: 5, at: 0, counted: 0 },
                { id: "reboot", keys: 5, at: 2000, counted: 0 }]),
            "armed ends | not");
        expect("a click after a key arms it again",
            presses([{ id: "reboot", keys: 5, at: 0 },
                { id: "reboot", keys: 6, at: 2000, counted: 0 }]),
            "armed armed | armed");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        console.log(`${asked - failures.length} of ${asked} passed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
