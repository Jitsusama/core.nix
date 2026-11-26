// Keys pressed on a modal Prompt.qml through its own handlers, as Qt hands
// them to the launcher's line, each held to saying it acted (keyActed,
// KeyActs.acts) exactly where it did something: changed the mode, said a
// signal the launcher acts on, or went where the line was told it could.
// So the line's word for a key can't drift from what its handler does
// with it, which tests/query-keys.qml can't see, holding KeyActs to the
// table alone. Tab asks where it goes (intoHere) and no other key does,
// the find's Tab going on to the next row is a move, Ctrl+Y acts only
// where it leaves a place (fillLeaves),
// gv takes back what there is and acts only then, an ask takes the
// operator pressed again before the operator does and is asked and
// answered at the one time the key came, and a register's keys,
// a modifier alone and a letter typed act on nothing. A secret, which has
// no modes, names Escape or Ctrl+C giving it up and says its Return, the
// keys the launcher times there. It runs in Qt's test runner, which fails
// naming each case that doesn't hold.
import QtQuick
import QtTest
import "Confirm.js" as Confirm

Item {
    id: root

    width: 640
    height: 48

    // How often the line asked whether Tab goes in, and what it was told.
    property int intoAsked: 0
    property bool intoGoes: false
    // How often it asked whether Ctrl+Y leaves a place, and what it was told.
    property int fillAsked: 0
    property bool leaving: false

    Prompt {
        id: line

        anchors.fill: parent
        modal: true
        intoHere: () => {
            root.intoAsked += 1;
            return root.intoGoes;
        }
        fillLeaves: () => {
            root.fillAsked += 1;
            return root.leaving;
        }
    }

    SignalSpy {
        id: answered

        target: line
        signalName: "answered"
    }
    SignalSpy {
        id: commanded

        target: line
        signalName: "commanded"
    }
    SignalSpy {
        id: operated

        target: line
        signalName: "operated"
    }
    SignalSpy {
        id: registered

        target: line
        signalName: "registered"
    }
    SignalSpy {
        id: stepped

        target: line
        signalName: "stepped"
    }

    TestCase {
        name: "PromptKeys"
        when: windowShown

        function init() {
            line.confirming = false;
            line.answerTaken = () => true;
            line.drawerHere = false;
            line.reselectable = false;
            line.registers = false;
            line.words = false;
            line.visualOperated = null;
            line.text = "";
            root.intoAsked = 0;
            root.intoGoes = false;
            root.fillAsked = 0;
            root.leaving = false;
            line.take();
            answered.clear();
            commanded.clear();
            operated.clear();
            registered.clear();
            stepped.clear();
        }

        // The find's Escape, which leaves it for normal.
        function toNormal() {
            keyClick(Qt.Key_Escape);
            compare(line.mode, "normal", "Escape leaves the find for normal");
        }

        function test_aLetterTypedActsOnNothing() {
            keyClick("a");
            compare(line.text, "a");
            verify(!line.keyActed, "a letter typed on the find");
        }

        function test_backspaceActsOnlyOnAnEmptyFind() {
            keyClick("a");
            keyClick(Qt.Key_Backspace);
            compare(line.text, "");
            compare(line.mode, "find");
            verify(!line.keyActed, "Backspace deleting a letter");
            keyClick(Qt.Key_Backspace);
            compare(line.mode, "normal");
            verify(line.keyActed, "Backspace leaving an empty find for normal");
        }

        // Each of KeyActs.js's typing names, by the key the find reads as
        // it, moving or editing what's typed and acting on nothing.
        function test_theFindsEditsAndCaretMovesActOnNothing() {
            keyClick("a");
            keyClick("b");
            keyClick(Qt.Key_Space);
            keyClick("c");
            keyClick(Qt.Key_Left);
            compare(line.caret, 3, "Left moves back");
            verify(!line.keyActed, "Left");
            keyClick(Qt.Key_Right);
            compare(line.caret, 4, "Right moves on");
            verify(!line.keyActed, "Right");
            keyClick(Qt.Key_Home);
            compare(line.caret, 0, "Home moves to the start");
            verify(!line.keyActed, "Home");
            keyClick(Qt.Key_End);
            compare(line.caret, 4, "End moves to the end");
            verify(!line.keyActed, "End");
            keyClick(Qt.Key_Y, Qt.ControlModifier);
            verify(!line.keyActed, "Ctrl+Y");
            keyClick(Qt.Key_W, Qt.ControlModifier);
            compare(line.text, "ab ", "Ctrl+W takes the word");
            verify(!line.keyActed, "Ctrl+W");
            keyClick(Qt.Key_U, Qt.ControlModifier);
            compare(line.text, "", "Ctrl+U takes the line");
            verify(!line.keyActed, "Ctrl+U");
            compare(line.mode, "find", "the find stays a find");
        }

        function test_ctrlYActsOnlyLeavingAPlace() {
            keyClick("a");
            keyClick(Qt.Key_Y, Qt.ControlModifier);
            compare(root.fillAsked, 1, "Ctrl+Y asks whether it leaves a place");
            verify(!line.keyActed, "Ctrl+Y finishing what's typed");
            root.leaving = true;
            keyClick(Qt.Key_Y, Qt.ControlModifier);
            verify(line.keyActed, "Ctrl+Y putting a command's start out at the top");
        }

        function test_theFindsTabOnToTheNextRowIsAMove() {
            keyClick(Qt.Key_Tab);
            verify(line.keyActed, "Tab on the find");
            verify(line.keyMoved, "Tab going on to the next row, said as a move");
            root.intoGoes = true;
            keyClick(Qt.Key_Tab);
            verify(line.keyActed, "Tab into a row");
            verify(!line.keyMoved, "Tab into a row, more than a move");
        }

        // Each of KeyActs.js's editing names, by its key on a prompt's words
        // in normal, changing what's typed or its caret and acting on nothing.
        function test_normalsEditsOnWordsActOnNothing() {
            line.words = true;
            keyClick("a");
            keyClick("b");
            keyClick(Qt.Key_Space);
            keyClick("c");
            keyClick("d");
            toNormal();
            const keys = [["0", 0], ["$", 4], ["b", 3], ["h", 2], ["l", 3], ["0", 0],
                ["w", 3], ["0", 0], ["e", 1]];
            for (const [key, caret] of keys) {
                keyClick(key);
                compare(line.caret, caret, key + " moves the caret");
                verify(!line.keyActed, key);
            }
            keyClick("x");
            compare(line.text, "a cd", "x takes the letter");
            verify(!line.keyActed, "x");
            keyClick("u");
            compare(line.text, "ab cd", "u takes it back");
            verify(!line.keyActed, "u");
            keyClick("0");
            keyClick("d");
            keyClick("w");
            compare(line.text, "cd", "dw takes the word");
            verify(!line.keyActed, "dw");
            compare(line.mode, "normal", "normal stays normal");
        }

        function test_escapeActsLeavingTheFindAndNotAtRest() {
            toNormal();
            verify(line.keyActed, "Escape leaving the find");
            keyClick(Qt.Key_Escape);
            compare(line.mode, "normal");
            verify(!line.keyActed, "Escape at rest in normal");
        }

        function test_aMoveActs() {
            toNormal();
            keyClick("j");
            compare(stepped.count, 1);
            compare(stepped.signalArguments[0][0], "next");
            verify(line.keyActed, "j moving down the rows");
            verify(line.keyMoved, "j said as a move");
        }

        function test_hOnlySaysTheWayOut() {
            toNormal();
            keyClick("h");
            verify(!line.keyActed, "h saying the way out");
        }

        function test_onlyTabAsksWhereItGoes() {
            toNormal();
            keyClick("j");
            keyClick("k");
            keyClick("h");
            compare(root.intoAsked, 0, "no key but Tab asks where Tab goes");
            compare(root.fillAsked, 0, "nor where Ctrl+Y goes");
            keyClick(Qt.Key_Tab);
            compare(root.intoAsked, 1, "Tab asks, once");
            verify(!line.keyActed, "Tab with nothing to go into");
            root.intoGoes = true;
            keyClick(Qt.Key_Tab);
            verify(line.keyActed, "Tab into a row with parts");
        }

        function test_theDrawerActsOnlyWhenOut() {
            toNormal();
            keyClick(Qt.Key_W, Qt.ControlModifier);
            verify(!line.keyActed, "Ctrl+W begun");
            keyClick("l");
            compare(line.mode, "normal");
            verify(!line.keyActed, "Ctrl+W l with no drawer out");
            line.drawerHere = true;
            keyClick(Qt.Key_W, Qt.ControlModifier);
            keyClick("l");
            compare(line.mode, "page");
            verify(line.keyActed, "Ctrl+W l into the drawer");
        }

        function test_gvOnTheRowsActsOnlyWithASelection() {
            toNormal();
            keyClick("g");
            verify(!line.keyActed, "g begun");
            keyClick("v");
            compare(line.mode, "normal");
            verify(!line.keyActed, "gv with no selection to take back");
            line.reselectable = true;
            keyClick("g");
            keyClick("v");
            compare(line.mode, "visual");
            verify(line.keyActed, "gv taking a selection back");
        }

        function test_gvInTheDrawerActsOnlyWithAChoice() {
            line.drawerHere = true;
            toNormal();
            keyClick(Qt.Key_W, Qt.ControlModifier);
            keyClick("l");
            keyClick("g");
            keyClick("v");
            compare(line.mode, "page");
            verify(!line.keyActed, "the drawer's gv with nothing chosen before");
            keyClick("v");
            compare(line.mode, "pageVisual");
            keyClick(Qt.Key_Escape);
            compare(line.mode, "page");
            keyClick("g");
            keyClick("v");
            compare(line.mode, "pageVisual");
            verify(line.keyActed, "the drawer's gv taking its choice back");
        }

        function test_aModifierAloneActsOnNothing() {
            toNormal();
            keyPress(Qt.Key_Shift);
            verify(line.keyAlone, "Shift said as alone");
            verify(!line.keyActed, "Shift alone");
            keyRelease(Qt.Key_Shift);
        }

        // Under an ask taking every key, so only the modifier's own guard
        // holds Shift idle: a key with no name reads idle on the rows anyway.
        function test_aModifierAloneActsOnNothingUnderAnAsk() {
            toNormal();
            line.confirming = true;
            keyPress(Qt.Key_Shift);
            verify(!line.keyActed, "Shift alone, though the ask takes every key");
            keyRelease(Qt.Key_Shift);
        }

        function test_aColonOpeningTheCommandLineActs() {
            toNormal();
            keyClick(":");
            compare(commanded.count, 1);
            verify(line.keyActed, "a colon opening the command line");
        }

        function test_aRegistersKeysActOnNothing() {
            line.registers = true;
            line.words = true;
            compare(line.mode, "insert");
            keyClick(Qt.Key_R, Qt.ControlModifier);
            verify(!line.keyActed, "Ctrl+R waiting for a register");
            keyClick("a");
            compare(registered.count, 1);
            compare(registered.signalArguments[0][0], "a");
            compare(line.text, "", "the register's letter typed nothing");
            verify(!line.keyActed, "the register's letter");
        }

        // Under an ask taking every key, so only the register's own guard
        // holds its keys idle: in insert both read idle anyway.
        function test_aRegistersKeysActOnNothingUnderAnAsk() {
            line.registers = true;
            line.words = true;
            line.confirming = true;
            keyClick(Qt.Key_R, Qt.ControlModifier);
            verify(!line.keyActed, "Ctrl+R waiting for a register, though the ask takes it");
            keyClick("a");
            compare(registered.count, 1);
            // Control, pressed alone before R, is the ask's to hear, as a
            // key with no name, which Confirm.answer says is early.
            const heard = [];
            for (let i = 0; i < answered.count; ++i)
                heard.push(answered.signalArguments[i][0]);
            compare(heard.filter(key => key !== ""), [],
                "the register takes its keys before the ask");
            verify(!line.keyActed, "the register's letter, though the ask takes it");
        }

        function test_theOperatorAgainIsSwallowed() {
            toNormal();
            keyClick("V");
            compare(line.mode, "visual");
            keyClick("d");
            compare(operated.count, 1);
            compare(line.mode, "normal");
            verify(line.keyActed, "visual's d taking the selection");
            keyClick("d");
            compare(operated.count, 1, "d again is taken with visual's");
            compare(stepped.count, 0);
            verify(!line.keyActed, "d again, swallowed");
            // Unswallowed, d again would begin an operator, idle too, which j
            // would then end as dj.
            keyClick("j");
            compare(operated.count, 1, "j after it only moves");
            compare(stepped.count, 1);
            compare(stepped.signalArguments[0][0], "next");
        }

        // The ask as the launcher asks it (Launcher.qml's answerTaken and
        // askSaid): any named key past its guard answers, x as no, and a key
        // inside the guard is early and taken as nothing.
        function test_anAskActsWhereItTakesTheKey() {
            toNormal();
            line.confirming = true;
            const ask = { at: Date.now() };
            let guard = 0;
            line.answerTaken = (key, at) => !["", "early"].includes(
                Confirm.answer(ask, key, at, guard));
            keyClick("x");
            compare(answered.count, 1);
            verify(line.keyActed, "x past the guard, answering no");
            guard = 60000;
            keyClick(Qt.Key_Return);
            compare(answered.count, 2);
            verify(!line.keyActed, "Return inside the guard, early");
            guard = 0;
            keyClick(Qt.Key_Return);
            compare(answered.count, 3);
            compare(answered.signalArguments[2][0], "Return");
            verify(line.keyActed, "Return answering the ask");
        }

        // Whether the ask takes a key and its answer are read at one time,
        // so a key on the ask's ready edge can't be timed idle and answer.
        function test_anAskIsAskedAndAnsweredAtOneTime() {
            toNormal();
            line.confirming = true;
            const asked = [];
            line.answerTaken = (key, at) => {
                asked.push(at);
                return true;
            };
            keyClick(Qt.Key_Return);
            compare(asked.length, 1, "the ask asked once");
            compare(answered.count, 1);
            verify(asked[0] > 0, "asked with when the key came");
            compare(answered.signalArguments[0][1], asked[0], "answered at that time");
        }

        function test_anAskTakesTheOperatorAgainBeforeTheOperator() {
            toNormal();
            keyClick("V");
            keyClick("d");
            line.confirming = true;
            keyClick("d");
            compare(answered.count, 1, "the ask hears d again");
            compare(operated.count, 1);
            verify(line.keyActed, "d again, the ask's answer");
        }
    }

    // A secret, a line with no modes, which names the key giving it up and
    // says Return, as the launcher times both (Launcher.qml's onCancelled
    // and onAccepted), and says nothing of the keys that only delete.
    Prompt {
        id: secret

        width: parent.width
        height: parent.height
        secret: true
    }

    SignalSpy {
        id: cancelled

        target: secret
        signalName: "cancelled"
    }
    SignalSpy {
        id: accepted

        target: secret
        signalName: "accepted"
    }

    TestCase {
        name: "SecretKeys"
        when: windowShown

        function init() {
            secret.text = "";
            secret.take();
            cancelled.clear();
            accepted.clear();
        }

        // That the secret was given up once, by the key named `key`.
        function givenUpBy(key) {
            compare(cancelled.count, 1, `${key} gives the secret up once`);
            compare(cancelled.signalArguments[0][0], key, "the key named as the prompt names it");
            compare(accepted.count, 0, `${key} hands over nothing`);
        }

        function test_escapeGivesTheSecretUpNamingIt() {
            keyClick("a");
            keyClick(Qt.Key_Escape);
            givenUpBy("Escape");
        }

        function test_ctrlCGivesTheSecretUpNamingIt() {
            keyClick("a");
            keyClick(Qt.Key_C, Qt.ControlModifier);
            givenUpBy("Ctrl+c");
        }

        function test_returnHandsTheSecretOverOnce() {
            keyClick("a");
            keyClick(Qt.Key_Return);
            compare(accepted.count, 1, "Return hands the secret over once");
            compare(cancelled.count, 0, "Return gives nothing up");
        }

        function test_deletingKeysGiveNothingUp() {
            keyClick("a");
            keyClick("b");
            keyClick("c");
            keyClick(Qt.Key_H, Qt.ControlModifier);
            compare(secret.text, "ab", "Ctrl+H deletes the letter before");
            keyClick(Qt.Key_U, Qt.ControlModifier);
            compare(secret.text, "", "Ctrl+U deletes back to the start");
            keyClick(Qt.Key_Backspace);
            keyClick(Qt.Key_W, Qt.ControlModifier);
            compare(cancelled.count, 0, "deleting keys give nothing up");
            compare(accepted.count, 0, "deleting keys hand nothing over");
        }
    }
}
