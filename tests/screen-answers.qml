// What a screen's power says on the launcher's line before anything is
// asked of niri, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `a-screens-power-says-how-it-stands` check copies
// modules/quickshell/launcher/ScreenAnswers.js and tests/lib/Expect.js
// beside this file and runs `qml -platform offscreen screen-answers.qml`.
// The last lit screen isn't turned off or unplugged, said as niri's
// refusals are, and one already as asked says how it stands (VKCF3Z,
// 2026-11-09).
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
            "not-off": "{thing} not turned off \u00b7 {why}",
            "not-unplugged": "{thing} not unplugged \u00b7 {why}",
            isOn: "{name} is on",
            "the-last-lit": "the last lit screen"
        })[key] ?? ""
    })

    function run() {
        Expect.same("the last lit screen isn't turned off, said as niri's refusals are",
            ScreenAnswers.toldSaid("not-off", "HP", words),
            "HP not turned off \u00b7 the last lit screen");
        Expect.same("nor unplugged", ScreenAnswers.toldSaid("not-unplugged", "HP", words),
            "HP not unplugged \u00b7 the last lit screen");
        Expect.same("and a screen already as asked says how it stands",
            ScreenAnswers.toldSaid("isOn", "HP", words), "HP is on");
        Expect.same("a $ in a name reads as written",
            ScreenAnswers.toldSaid("isOn", "$& $1", words), "$& $1 is on");

        Qt.exit(0);
    }
}
