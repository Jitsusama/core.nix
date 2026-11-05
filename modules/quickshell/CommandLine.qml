// The command line, the : line, as Vim's command line after a colon
// (VKCF3Z's spec, The Command Line): it takes the find line's place while
// it's open, the accent's : leading it in the glyph's column and what's
// typed in the mono at Figure on the names' column, with the mono's bar
// caret in the accent, as insert's is (docs/design.md, Vi's Keys). What its
// menu lists, how typing narrows it and what Return runs are Commands.js's;
// this holds how the keys stand, and CommandMenu.qml draws the menu over the
// rows. The launcher makes it only while : is open (a Loader), so a launcher
// that never sees : makes nothing.
//
// Its keys are Vim's command line's and his cmp's (modules/neovim/keys.nix):
// letters type, ← → Home End move the caret, Backspace, Ctrl+H, Ctrl+W and
// Ctrl+U delete; Tab, Shift+Tab, ↓ ↑, Ctrl+N and Ctrl+P choose a line,
// writing it in; Ctrl+Y takes what's chosen and Ctrl+E puts the menu away;
// Return runs exactly what's typed; Escape and Ctrl+C drop the line, and
// Backspace on an empty line gives normal. Return, Escape and Backspace
// leaving act only on a fresh press, so a held key never runs or leaves
// what it only reached.
pragma ComponentBehavior: Bound
import QtQuick
import "Commands.js" as Commands

Item {
    id: line

    // The menu's lines, as Commands.listed made them when : was pressed.
    property var menu: []
    // A named command's arguments, listed after a space (Commands.narrowed):
    // (entry, arg) => [{ value, note }], or null for one that has none.
    property var argued: null
    // How choosing stands: what was typed, the line chosen and what the
    // line holds (Commands.typing).
    property var choosing: Commands.typing("")
    // Whether Ctrl+Y or Ctrl+E has put the menu away, until the next letter.
    property bool away: false
    // The menu as typed narrows it, which Tab and the arrows choose among.
    readonly property var shown: away ? [] : Commands.narrowed(menu, choosing.typed, argued)
    // The line chosen, or null.
    readonly property var chosenLine: choosing.at >= 0 ? shown[choosing.at] ?? null : null
    readonly property string text: input.text
    readonly property int caret: input.cursorPosition

    // Return, with what it runs (Commands.ran): { entry, bang, arg }, { go }
    // or { error }.
    signal ran(var read)
    // The line let go: "dropped" by Escape or Ctrl+C, putting back what it
    // covered, or "normal" by Backspace on an empty line.
    signal letGo(string how)

    implicitHeight: 4 * Theme.space.beat
    // Its : on the rows' glyph column, an inset in, and what's typed a
    // glyph's column on, on their names' (Choice.qml): a mark's square
    // cell, a line pitch wide, and the mark rung before its text.
    readonly property real typedAt: Theme.space.rung.inset + Theme.level.name.pitch
        + Theme.space.rung.mark

    // Takes the keys, as : opens it, with the range it opens on written in,
    // as a count's .,.+2 or visual's '<,'>.
    function take(range) {
        input.forceActiveFocus();
        if (range !== "")
            write(Commands.typing(range));
    }

    function write(choosing) {
        line.choosing = choosing;
        input.text = choosing.line;
        input.cursorPosition = input.text.length;
    }

    function choose(step) {
        write(Commands.chosen(choosing, step, shown));
    }

    // A key of the line's own, by its name, as Prompt.qml names keys.
    function key(event) {
        const control = (event.modifiers & Qt.ControlModifier) !== 0;
        const letter = control && event.key >= Qt.Key_A && event.key <= Qt.Key_Z
            ? String.fromCharCode(event.key + 32) : "";
        if (control)
            return letter === "" ? "" : `Ctrl+${letter}`;
        switch (event.key) {
        case Qt.Key_Return:
        case Qt.Key_Enter:
            return "Return";
        case Qt.Key_Escape:
            return "Escape";
        case Qt.Key_Tab:
            return (event.modifiers & Qt.ShiftModifier) !== 0 ? "Shift+Tab" : "Tab";
        case Qt.Key_Backtab:
            return "Shift+Tab";
        case Qt.Key_Down:
            return "Down";
        case Qt.Key_Up:
            return "Up";
        case Qt.Key_Backspace:
            return "Backspace";
        default:
            return "";
        }
    }

    function pressed(event) {
        const held = event.isAutoRepeat;
        switch (key(event)) {
        case "Escape":
        case "Ctrl+c":
            if (!held)
                line.letGo("dropped");
            break;
        case "Return":
            if (!held)
                line.ran(Object.assign(Commands.ran(input.text, line.menu),
                    { typed: input.text }));
            break;
        case "Tab":
        case "Down":
        case "Ctrl+n":
            line.choose(1);
            break;
        case "Shift+Tab":
        case "Up":
        case "Ctrl+p":
            line.choose(-1);
            break;
        case "Ctrl+y":
            line.choosing = Commands.taken(line.choosing);
            line.away = true;
            break;
        case "Ctrl+e":
            line.write(Commands.dropped(line.choosing));
            line.away = true;
            break;
        case "Backspace":
        case "Ctrl+h":
            // A held Backspace stops at empty, and only a fresh press
            // leaves the line.
            if (input.text === "") {
                if (!held)
                    line.letGo("normal");
            } else if (input.selectionStart !== input.selectionEnd) {
                input.remove(input.selectionStart, input.selectionEnd);
            } else if (input.cursorPosition > 0) {
                input.remove(input.cursorPosition - 1, input.cursorPosition);
            }
            break;
        case "Ctrl+w":
            input.remove(Commands.wordStart(input.text, input.cursorPosition),
                input.cursorPosition);
            break;
        case "Ctrl+u":
            input.remove(0, input.cursorPosition);
            break;
        default:
            // A letter held types once, as every field of ours does, and
            // the rest are the field's own: letters, the arrows along the
            // line, Home and End.
            if (held && event.text !== "" && event.text >= " ")
                break;
            return;
        }
        event.accepted = true;
    }

    FontMetrics {
        id: face
        font: input.font
    }

    Text {
        id: sigil
        x: Theme.space.rung.inset
        anchors.baseline: input.baseline
        text: ":"
        color: Theme.accent
        font: input.font
    }

    TextInput {
        id: input
        x: line.typedAt
        width: parent.width - x - Theme.space.rung.inset
        anchors.verticalCenter: parent.verticalCenter
        focus: true
        clip: true
        color: Theme.strong
        selectionColor: Theme.selection
        selectedTextColor: Theme.strong
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
        font.features: Theme.level.figure.features
        font.letterSpacing: Theme.level.figure.letterSpacing
        // Qt gives these handlers of their own before Keys.onPressed, so
        // they're handed to the line's here.
        Keys.onReturnPressed: event => line.pressed(event)
        Keys.onEnterPressed: event => line.pressed(event)
        Keys.onTabPressed: event => line.pressed(event)
        Keys.onBacktabPressed: event => line.pressed(event)
        Keys.onEscapePressed: event => line.pressed(event)
        Keys.onPressed: event => line.pressed(event)
        // A letter typed narrows the menu again, from what's typed now.
        onTextEdited: {
            line.choosing = Commands.typing(text);
            line.away = false;
        }

        // The mono's bar, a quarter of a letter wide, as his guicursor's
        // ver25 draws insert's, in the accent, since it's where the keys go.
        cursorDelegate: Rectangle {
            width: Math.max(1, Math.round(face.advanceWidth("M") / 4))
            color: Theme.accent
            visible: input.cursorVisible
        }
    }
}
