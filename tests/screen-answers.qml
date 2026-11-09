// What niri's answers to a screen's change say on the launcher's warning
// line, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-says-what-niri-refuses` check copies
// modules/quickshell/launcher/ScreenAnswers.js and tests/lib/Expect.js
// beside this file and runs `qml -platform offscreen screen-answers.qml`.
// A change niri refuses says what was asked, what wasn't done and niri's
// own words for why, and one to a screen gone says it's unplugged
// (VKCF3Z, 2026-11-09).
import QtQuick
import "Expect.js" as Expect
import "ScreenAnswers.js" as ScreenAnswers

QtObject {
    // A case that doesn't hold throws, naming it, and ends as a failure,
    // rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // Displays' words, as words.json says them.
    readonly property var words: ({
        said: (id, key) => ({
            "not-set": "{thing} not set \u00b7 {why}",
            "not-on": "{thing} not turned on \u00b7 {why}",
            "not-off": "{thing} not turned off \u00b7 {why}",
            "not-unplugged": "{thing} not unplugged \u00b7 {why}",
            isUnplugged: "{name} is unplugged",
            isOn: "{name} is on",
            "the-last-lit": "the last lit screen"
        })[key] ?? ""
    })

    function run() {
        const refused = (thing, done, why) =>
            ScreenAnswers.refusalSaid({ thing, done }, why, words);

        Expect.same("a setting refused says it wasn't set, and niri's why",
            refused("1.5\u00d7", "set", "invalid custom mode"),
            "1.5\u00d7 not set \u00b7 invalid custom mode");
        Expect.same("niri's why begins in lower case, with no full stop closing it",
            refused("2560 \u00d7 1440", "set", "Invalid custom mode."),
            "2560 \u00d7 1440 not set \u00b7 invalid custom mode");
        Expect.same("only its first letter, so a screen's name keeps its case",
            refused("HP", "off", "Output eDP-1 is busy"),
            "HP not turned off \u00b7 output eDP-1 is busy");
        Expect.same("a screen turned on, refused, says so",
            refused("HP", "on", "no"), "HP not turned on \u00b7 no");
        Expect.same("and one unplugged",
            refused("HP", "unplug", "no"), "HP not unplugged \u00b7 no");
        Expect.same("a $ in niri's words reads as niri wrote it",
            refused("1.5\u00d7", "set", "no $& or $1 here"),
            "1.5\u00d7 not set \u00b7 no $& or $1 here");

        Expect.same("the last lit screen isn't turned off, said as niri's refusals are",
            ScreenAnswers.toldSaid("not-off", "HP", words),
            "HP not turned off \u00b7 the last lit screen");
        Expect.same("nor unplugged", ScreenAnswers.toldSaid("not-unplugged", "HP", words),
            "HP not unplugged \u00b7 the last lit screen");
        Expect.same("and a screen already as asked says how it stands",
            ScreenAnswers.toldSaid("isOn", "HP", words), "HP is on");

        Expect.same("a screen gone before the change came is unplugged",
            ScreenAnswers.answerSaid({ OutputConfigChanged: "OutputWasMissing" }, "HP", words),
            "HP is unplugged");
        Expect.same("and a change applied says nothing",
            ScreenAnswers.answerSaid({ OutputConfigChanged: "Applied" }, "HP", words), "");
        Expect.same("nor does an answer that isn't a screen's",
            ScreenAnswers.answerSaid("Handled", "HP", words), "");

        Qt.exit(0);
    }
}
