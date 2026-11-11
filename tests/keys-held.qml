// When the open launcher holds its keys and when a window took them, run
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-launcher-closes-only-for-keys-it-held` check copies
// modules/quickshell/launcher/KeysHeld.js beside this file and runs it
// offscreen. Each case is an opening's events in the order the shell heard
// them, each the card's activation and the window niri says is focused as
// they stood then, replayed as Launcher.qml's keysMoved does, and says
// whether the launcher closed.
import QtQuick
import "KeysHeld.js" as KeysHeld

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
        if (got !== want)
            failures.push(`${what}: got "${got}", wanted "${want}"`);
    }

    // Replays an opening, from keysHeld false as each opening begins, and
    // answers how it ended: "taken" where it closed, else "held" or "".
    function opening(events) {
        let held = false;
        for (const seen of events) {
            const moved = KeysHeld.moved(held, Object.assign({ lettingGo: false }, seen));
            if (moved === "taken")
                return "taken";
            if (moved === "held")
                held = true;
        }
        return held ? "held" : "";
    }

    // The window the closing gave the keys back to, by niri's id.
    readonly property int elsewhere: 3

    function run() {
        const failures = [];

        // Niri hands the keys over a frame after the opening asks: either
        // its word that no window has them or the card's activation comes
        // first, and either way the opening holds them.
        check(failures, "niri's word first, then the card's activation", opening([
            { active: false, focused: null },
            { active: true, focused: null }
        ]), "held");
        check(failures, "the card's activation first, then niri's word", opening([
            { active: true, focused: elsewhere },
            { active: true, focused: null }
        ]), "held");

        // Reopened at once after a closing (vm-desktop-looks' run 5 at
        // 166.42 s, 2026-11-11): the closing's letting go reached niri only
        // with the opening's first commit, so the card still read active
        // from before as niri's late focus on the window came, then went
        // inactive, then took the keys for this opening.
        check(failures, "an opening right after a closing stays open", opening([
            { active: true, focused: elsewhere },
            { active: false, focused: elsewhere },
            { active: false, focused: null },
            { active: true, focused: null }
        ]), "held");

        // Held, a click on a window takes them and closes it; a notice held
        // over it takes them with no window focused, and it stays open.
        check(failures, "a click on a window closes it", opening([
            { active: true, focused: null },
            { active: false, focused: elsewhere }
        ]), "taken");
        check(failures, "a notice held over it leaves it open", opening([
            { active: true, focused: null },
            { active: false, focused: null },
            { active: true, focused: null }
        ]), "held");
        // Never held, a window having them says nothing.
        check(failures, "a window keeping them before it's held", opening([
            { active: false, focused: elsewhere }
        ]), "");
        // Let go on purpose, as an act that pastes does, it's closed by
        // that act once its frame is drawn, not as taken.
        check(failures, "let go on purpose",
            KeysHeld.moved(true, { active: false, focused: elsewhere, lettingGo: true }), "");

        for (const failure of failures)
            console.log(`FAIL ${failure}`);
        console.log(`keys-held: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
