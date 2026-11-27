// Keys pressed on the launcher's command line, CommandLine.qml, through its
// own handlers, each held to what it says the launcher: Escape and Ctrl+C
// let the line go dropped, and Backspace or Ctrl+H on an empty line let it
// go for normal, each naming its key, which the launcher times as a move
// (Launcher.qml's onLetGo, keyActs); Return runs what's typed and lets
// nothing go; and the keys that only edit or choose let nothing go. So
// the key the launcher times can't drift from the key that closed the
// line. The test runner sends no repeat, so a held key's is handed to the
// line's own handler as Qt would give it, and lets nothing go.
// It runs in Qt's test runner over a stand-in Quickshell holding only the
// clipboard, which fails naming each case that doesn't hold.
import QtQuick
import QtTest
import Quickshell

Item {
    id: root

    width: 640
    height: 48

    CommandLine {
        id: line

        anchors.fill: parent
    }

    SignalSpy {
        id: letGo

        target: line
        signalName: "letGo"
    }
    SignalSpy {
        id: ran

        target: line
        signalName: "ran"
    }

    TestCase {
        name: "CommandLineKeys"
        when: windowShown

        function init() {
            line.away = false;
            line.register = false;
            letGo.clear();
            ran.clear();
        }

        // The line holding `typed`, its caret at the end: emptied by
        // Ctrl+U, then written as : opens on a command.
        function typing(typed) {
            line.take("", "");
            keyClick(Qt.Key_End);
            keyClick(Qt.Key_U, Qt.ControlModifier);
            line.take("", typed);
            compare(line.text, typed);
            letGo.clear();
            ran.clear();
        }

        // That the line was let go once, `how`, by the key named `key`.
        function letGoBy(how, key) {
            compare(letGo.count, 1, `${key} lets the line go once`);
            compare(letGo.signalArguments[0][0], how, `${key} lets it go ${how}`);
            compare(letGo.signalArguments[0][1], key, "the key named as the prompt names it");
            compare(ran.count, 0, `${key} runs nothing`);
        }

        function test_escapeDropsTheLineNamingIt() {
            typing("vol");
            keyClick(Qt.Key_Escape);
            letGoBy("dropped", "Escape");
        }

        function test_ctrlCDropsTheLineNamingIt() {
            typing("vol");
            keyClick(Qt.Key_C, Qt.ControlModifier);
            letGoBy("dropped", "Ctrl+c");
        }

        function test_backspaceOnAnEmptyLineGivesNormal() {
            typing("");
            keyClick(Qt.Key_Backspace);
            letGoBy("normal", "Backspace");
        }

        function test_ctrlHOnAnEmptyLineGivesNormal() {
            typing("");
            keyClick(Qt.Key_H, Qt.ControlModifier);
            letGoBy("normal", "Ctrl+h");
        }

        function test_backspaceOnALetterOnlyDeletesIt() {
            typing("ab");
            keyClick(Qt.Key_Backspace);
            compare(line.text, "a");
            compare(letGo.count, 0, "Backspace deleting a letter lets nothing go");
            keyClick(Qt.Key_Backspace);
            compare(line.text, "");
            compare(letGo.count, 0, "Backspace emptying the line lets nothing go");
            keyClick(Qt.Key_Backspace);
            letGoBy("normal", "Backspace");
        }

        function test_returnRunsWhatsTypedAndLetsNothingGo() {
            typing("vol");
            keyClick(Qt.Key_Return);
            compare(ran.count, 1, "Return runs the line once");
            compare(ran.signalArguments[0][0].typed, "vol", "Return runs what's typed");
            compare(letGo.count, 0, "Return lets nothing go");
        }

        function test_editingAndChoosingLetNothingGo() {
            typing("vol up");
            keyClick(Qt.Key_W, Qt.ControlModifier);
            compare(line.text, "vol ");
            keyClick(Qt.Key_U, Qt.ControlModifier);
            compare(line.text, "");
            keyClick(Qt.Key_Tab);
            keyClick(Qt.Key_Down);
            keyClick(Qt.Key_E, Qt.ControlModifier);
            keyClick("x");
            compare(letGo.count, 0, "editing and choosing keys let nothing go");
            compare(ran.count, 0, "editing and choosing keys run nothing");
        }

        // A held key's repeat, as Qt gives it: letting the line go, or
        // running it, is a fresh press's alone.
        function test_aHeldKeysRepeatLetsNothingGoNorRuns() {
            const held = (key, modifiers) => ({ key, modifiers, text: "",
                isAutoRepeat: true, accepted: false });
            typing("vol");
            line.pressed(held(Qt.Key_Escape, Qt.NoModifier));
            line.pressed(held(Qt.Key_C, Qt.ControlModifier));
            line.pressed(held(Qt.Key_Return, Qt.NoModifier));
            // Read before typing() clears both, so these three are proven too.
            compare(letGo.count, 0, "a held Escape, Ctrl+C or Return lets nothing go");
            compare(ran.count, 0, "nor runs what's typed");
            typing("");
            line.pressed(held(Qt.Key_Backspace, Qt.NoModifier));
            line.pressed(held(Qt.Key_H, Qt.ControlModifier));
            compare(letGo.count, 0, "a held key's repeat lets nothing go");
            compare(ran.count, 0, "nor runs the line");
            // Each same event pressed fresh does, so each repeat reaches the
            // guard it's checked against rather than dropping before it.
            const fresh = (key, modifiers) => Object.assign(held(key, modifiers),
                { isAutoRepeat: false });
            typing("vol");
            line.pressed(fresh(Qt.Key_Escape, Qt.NoModifier));
            letGoBy("dropped", "Escape");
            typing("vol");
            line.pressed(fresh(Qt.Key_C, Qt.ControlModifier));
            letGoBy("dropped", "Ctrl+c");
            typing("vol");
            line.pressed(fresh(Qt.Key_Return, Qt.NoModifier));
            compare(ran.count, 1, "Return pressed fresh runs it");
            typing("");
            line.pressed(fresh(Qt.Key_Backspace, Qt.NoModifier));
            letGoBy("normal", "Backspace");
            typing("");
            line.pressed(fresh(Qt.Key_H, Qt.ControlModifier));
            letGoBy("normal", "Ctrl+h");
        }

        function test_aRegistersKeyPutsTheClipboardAndLetsNothingGo() {
            typing("");
            keyClick(Qt.Key_R, Qt.ControlModifier);
            verify(line.register, "Ctrl+R waits for its register");
            keyClick("+");
            compare(line.text, Quickshell.clipboardText.split("\n")[0],
                "+ puts the clipboard's first line in");
            compare(letGo.count, 0, "a register's keys let nothing go");
        }
    }
}
