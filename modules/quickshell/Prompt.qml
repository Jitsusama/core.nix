// The one field every part of the shell asks through: the launcher's command
// line, the lock screen's password and polkit's. A glyph leads it and says
// what it's for, since a line you act on is the one place a glyph starts a
// line (docs/design.md, Shape and Kinds). A secret shows as dots right beside
// the padlock. A ghost after what's typed offers how it could end, or, with
// nothing typed, a hint.
import QtQuick

Item {
    id: prompt

    property string glyph
    property color hue: Theme.muted
    // Room between the glyph and what's typed: the launcher lines what's
    // typed up with its results' column, and a secret sits close by its lock.
    property int glyphWidth: glyphText.implicitWidth + Theme.space.m
    property bool secret: false
    // How what's typed could end, drawn after it in muted type.
    property string ghost
    // Said in muted type while nothing's typed.
    property string hint
    property alias text: input.text

    // Return, or Shift+Return, which asks for it below the focused window.
    signal accepted(bool below)
    // Tab, which takes the ghost.
    signal completed
    signal cancelled

    function clear() {
        input.text = "";
    }

    implicitWidth: 480
    implicitHeight: Theme.space.xxxl

    Text {
        id: glyphText
        x: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        text: prompt.glyph
        color: prompt.hue
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    TextInput {
        id: input
        x: Theme.space.l + prompt.glyphWidth
        width: parent.width - x - Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        focus: true
        clip: true
        echoMode: prompt.secret ? TextInput.Password : TextInput.Normal
        passwordCharacter: "\u25cf"
        passwordMaskDelay: 0
        color: Theme.strong
        selectionColor: Theme.selection
        selectedTextColor: Theme.strong
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
        font.weight: prompt.secret ? Font.Normal : Font.Medium
        // A fifth of the size between dots, so each reads as one character.
        // Points are 4/3 of a pixel at 96 dots an inch.
        font.letterSpacing: prompt.secret ? Theme.mono.large * 4 / 3 * 0.2 : 0
        Keys.onReturnPressed: event => prompt.accepted((event.modifiers & Qt.ShiftModifier) !== 0)
        Keys.onEnterPressed: event => prompt.accepted((event.modifiers & Qt.ShiftModifier) !== 0)
        Keys.onTabPressed: prompt.completed()
        Keys.onEscapePressed: prompt.cancelled()

        // The caret in the accent, since it's where the keys go. It takes
        // no room of its own, so a ghost reads on as one word.
        cursorDelegate: Rectangle {
            width: 2
            color: Theme.accent
            visible: input.cursorVisible
        }

        Text {
            x: input.contentWidth
            anchors.verticalCenter: parent.verticalCenter
            text: input.text === "" ? prompt.hint : prompt.ghost
            visible: text !== "" && (input.text === "" || !prompt.secret)
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }
    }
}
