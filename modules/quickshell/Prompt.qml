// The one field every part of the shell asks through: the launcher's command
// line, the lock screen's password and polkit's. A glyph leads it and says
// what it's for, since a line you act on is the one place a glyph starts a
// line (docs/design.md, Shape and Kinds). A secret shows as dots right beside
// the padlock. A ghost after what's typed offers how it could end, or, with
// nothing typed, a hint. An aside on the right, muted, says something about
// what's typed, such as where the launcher's selection is among its matches.
// A secret's aside says when caps lock is on, its glyph in the warning
// colour, since a password typed in capitals fails with no reason given.
// Chips before what's typed hold what's being acted on: a thing, then a verb
// waiting for its target. They share what the line has once what's typed
// has room for its hint: a short one keeps its width, the others split what
// remains, and one too long for its share ends in an ellipsis.
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
    // Said in muted type at the right end of the line.
    property string aside
    readonly property bool capitals: secret && Lights.caps
    readonly property string said: capitals ? "caps lock" : aside
    // What's being acted on, each drawn as a chip: { text }.
    property var chips: []
    property alias text: input.text

    // Where what's typed has to end: before the aside, and before caps
    // lock's glyph when it shows.
    readonly property real end: said === "" ? width - Theme.space.l
        : asideText.x - Theme.space.xl
            - (capitals ? Theme.space.s + capitalsGlyph.implicitWidth : 0)

    // Return, or Shift+Return, which asks for it below the focused window.
    signal accepted(bool below)
    // Tab, which takes the ghost.
    signal completed
    signal cancelled
    // The right arrow past the end of what's typed, and the left arrow before
    // its start or Backspace with nothing typed: into what's selected, and
    // back out.
    signal forward
    signal backward

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

    Text {
        id: asideText
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        text: prompt.said
        color: Theme.muted
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
        font.features: { "tnum": 1 }
    }

    Text {
        id: capitalsGlyph
        anchors.right: asideText.left
        anchors.rightMargin: Theme.space.s
        anchors.verticalCenter: parent.verticalCenter
        visible: prompt.capitals
        text: Theme.glyph.capitals
        color: Theme.warning
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    FontMetrics {
        id: chipFace
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    // Each chip's words' width: what it needs where it fits, and otherwise
    // an even share of what the shorter ones leave.
    readonly property var fitted: {
        const typing = Math.max(chipFace.advanceWidth(hint), 12 * chipFace.averageCharacterWidth);
        let left = Math.max(0, end - Theme.space.l - glyphWidth - Theme.space.m - typing
            - Theme.space.s * Math.max(0, chips.length - 1) - 2 * Theme.space.m * chips.length);
        const wanted = chips.map(chip => Math.ceil(chipFace.advanceWidth(chip.text)) + 1);
        const given = wanted.slice();
        const order = wanted.map((_, index) => index).sort((a, b) => wanted[a] - wanted[b]);
        order.forEach((index, done) => {
            given[index] = Math.min(wanted[index], Math.floor(left / (order.length - done)));
            left -= given[index];
        });
        return given;
    }

    // A chip is a part of the frame, so its corners are the part's radius,
    // raised from the surface and set in the strong colour.
    Row {
        id: held
        x: Theme.space.l + prompt.glyphWidth
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.space.s

        Repeater {
            model: prompt.chips

            Rectangle {
                required property var modelData
                required property int index

                width: chipText.width + 2 * Theme.space.m
                height: Theme.space.xxl
                radius: Theme.inner
                color: Theme.raised

                Text {
                    id: chipText
                    anchors.centerIn: parent
                    width: prompt.fitted[index] ?? implicitWidth
                    text: modelData.text
                    elide: Text.ElideRight
                    color: Theme.strong
                    font.family: Theme.mono.family
                    font.pointSize: Theme.mono.large
                }
            }
        }
    }

    TextInput {
        id: input
        x: Theme.space.l + prompt.glyphWidth
            + (prompt.chips.length > 0 ? held.width + Theme.space.m : 0)
        width: prompt.end - x
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
        Keys.onPressed: event => {
            const none = input.selectionStart === input.selectionEnd;
            if (event.key === Qt.Key_Right && none && input.cursorPosition === input.text.length) {
                prompt.forward();
                event.accepted = true;
            } else if ((event.key === Qt.Key_Left && none && input.cursorPosition === 0)
                || (event.key === Qt.Key_Backspace && input.text === "")) {
                prompt.backward();
                event.accepted = true;
            }
        }

        // The caret in the accent, since it's where the keys go. It takes
        // no room of its own, so a ghost reads on as one word.
        cursorDelegate: Rectangle {
            width: 2
            color: Theme.accent
            visible: input.cursorVisible
        }

        // A hint stands just clear of the caret; a ghost runs on from what's
        // typed as one word.
        Text {
            x: input.text === "" ? Theme.space.xs : input.contentWidth
            anchors.verticalCenter: parent.verticalCenter
            text: input.text === "" ? prompt.hint : prompt.ghost
            visible: text !== "" && (input.text === "" || !prompt.secret)
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }
    }
}
