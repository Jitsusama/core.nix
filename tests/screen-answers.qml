// What niri's answers to a screen's change say on the launcher's warning
// line, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-launcher-says-what-niri-refuses` check copies
// modules/quickshell/launcher/ScreenAnswers.js beside this file and runs
// `qml -platform offscreen screen-answers.qml`. A change niri refuses
// says what was asked, what wasn't done and niri's own words for why, and
// one to a screen gone says it's unplugged (VKCF3Z, 2026-11-09).
import QtQuick
import "ScreenAnswers.js" as ScreenAnswers

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

    // Displays' words, as words.json says them.
    readonly property var words: ({
        said: (id, key) => ({
            "not-set": "{thing} not set \u00b7 {why}",
            "not-on": "{thing} not turned on \u00b7 {why}",
            "not-off": "{thing} not turned off \u00b7 {why}",
            "not-unplugged": "{thing} not unplugged \u00b7 {why}",
            isUnplugged: "{name} is unplugged"
        })[key] ?? ""
    })

    function run() {
        const failures = [];
        const refused = (thing, done, why) =>
            ScreenAnswers.refusalSaid({ thing, done }, why, words);

        check(failures, "a setting refused says it wasn't set, and niri's why",
            refused("1.5\u00d7", "set", "invalid custom mode"),
            "1.5\u00d7 not set \u00b7 invalid custom mode");
        check(failures, "niri's why begins in lower case, with no full stop closing it",
            refused("2560 \u00d7 1440", "set", "Invalid custom mode."),
            "2560 \u00d7 1440 not set \u00b7 invalid custom mode");
        check(failures, "only its first letter, so a screen's name keeps its case",
            refused("HP", "off", "Output eDP-1 is busy"),
            "HP not turned off \u00b7 output eDP-1 is busy");
        check(failures, "a screen turned on, refused, says so",
            refused("HP", "on", "no"), "HP not turned on \u00b7 no");
        check(failures, "and one unplugged",
            refused("HP", "unplug", "no"), "HP not unplugged \u00b7 no");

        check(failures, "a screen gone before the change came is unplugged",
            ScreenAnswers.answerSaid({ OutputConfigChanged: "OutputWasMissing" }, "HP", words),
            "HP is unplugged");
        check(failures, "and a change applied says nothing",
            ScreenAnswers.answerSaid({ OutputConfigChanged: "Applied" }, "HP", words), "");
        check(failures, "nor does an answer that isn't a screen's",
            ScreenAnswers.answerSaid("Handled", "HP", words), "");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
