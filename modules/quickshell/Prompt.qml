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
pragma ComponentBehavior: Bound
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
    // Whether the left arrow takes what's typed away whole before it goes
    // back out, as the launcher's levels want: what's typed there is a few
    // letters narrowing a list, never text worth walking through, so the
    // arrow is the one key for "not that", and a second press goes up.
    property bool leftClears: false
    // Says how far an aside may run, so a long one never crowds what's typed
    // or where Joel is: the path's steps and room to type come first, and
    // the aside has what they leave, up to two fifths of the line. With the
    // aside first, a step into a thing whose rows have long notes cut every
    // step of the path to a letter (Joel, 2026-10-25: "the breadcrumb
    // quickly goes single letter per entry").
    property real asideMost: Math.max(0, Math.min(width * 0.4, width - 2 * Theme.space.l
        - glyphWidth - Theme.space.m - pathWanted - typingRoom - Theme.space.xl))
    // Whether the aside keeps the room its widest has taken until forget():
    // the launcher's says the selected row's note, which changes with each
    // arrow, and an aside sized to each one moved where what's typed had to
    // end and so every chip's share of the line.
    property bool steady: false
    property real asideWidest: 0
    property alias text: input.text

    // The aside's room: its own width, or, held steady, its widest since
    // forget(), so what's typed ends at the same place whatever it says. Its
    // words keep to the right end of that room, since a short note set at
    // the start of a long one's room sat far in from the edge (Joel,
    // 2026-10-25: "just now is inset an insane amount").
    readonly property real asideRoom: Math.min(prompt.asideMost, steady
        ? Math.max(asideWidest, asideText.implicitWidth) : asideText.implicitWidth)

    // Lets the aside's room go, as a new step starts, keeping what it says now.
    function forget() {
        asideWidest = Math.min(asideText.implicitWidth, asideMost);
    }

    // Where what's typed has to end: before the aside's room, and before
    // caps lock's glyph when it shows.
    readonly property real end: asideRoom === 0 ? width - Theme.space.l
        : asideText.x - Theme.space.xl
            - (capitals ? Theme.space.s + capitalsGlyph.implicitWidth : 0)

    // Return, or Shift+Return, which asks for it below the focused window.
    signal accepted(bool below)
    // Tab, which takes the ghost.
    signal completed
    signal cancelled
    // The right arrow past the end of what's typed, and the left arrow before
    // its start (or anywhere, with leftClears, once what's typed is gone) or
    // Backspace with nothing typed: into what's selected, and back out.
    signal forward
    signal backward

    function clear() {
        input.text = "";
    }

    // Takes the keys back from whatever else in its window had them.
    function take() {
        input.forceActiveFocus();
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
        width: prompt.asideRoom
        // Given no room, it isn't drawn: a text of no width isn't cut but
        // drawn whole, over the hint that took the room.
        visible: prompt.asideRoom > 0
        horizontalAlignment: Text.AlignRight
        elide: Text.ElideRight
        onImplicitWidthChanged: {
            if (prompt.steady)
                prompt.asideWidest = Math.max(prompt.asideWidest,
                    Math.min(implicitWidth, prompt.asideMost));
        }
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

    // What stands between two steps of the path, so they read as one place
    // reached through the other rather than as tags side by side.
    readonly property string between: " \u203a "

    // Each step's words' width: what it needs where it fits, and otherwise
    // an even share of what the shorter ones leave.
    // The room kept for typing: the hint's, or a dozen characters.
    readonly property real typingRoom: Math.max(chipFace.advanceWidth(hint),
        12 * chipFace.averageCharacterWidth)

    // How wide the path wants to be, whole, with the chip's own margins.
    readonly property real pathWanted: chips.length === 0 ? 0
        : chips.reduce((sum, chip) => sum + Math.ceil(chipFace.advanceWidth(chip.text)) + 1, 0)
            + chipFace.advanceWidth(between) * (chips.length - 1) + 2 * Theme.space.m

    readonly property var fitted: {
        const typing = typingRoom;
        let left = Math.max(0, end - Theme.space.l - glyphWidth - Theme.space.m - typing
            - chipFace.advanceWidth(between) * Math.max(0, chips.length - 1)
            - 2 * Theme.space.m);
        const wanted = chips.map(chip => Math.ceil(chipFace.advanceWidth(chip.text)) + 1);
        const given = wanted.slice();
        const order = wanted.map((_, index) => index).sort((a, b) => wanted[a] - wanted[b]);
        order.forEach((index, done) => {
            given[index] = Math.min(wanted[index], Math.floor(left / (order.length - done)));
            left -= given[index];
        });
        return given;
    }

    // Where Joel is, one chip: a part of the frame, so its corners are the
    // part's radius, raised from the surface, each step named as its row
    // was in the strong colour, and the marks between them muted.
    Rectangle {
        id: held
        x: Theme.space.l + prompt.glyphWidth
        anchors.verticalCenter: parent.verticalCenter
        visible: prompt.chips.length > 0
        width: visible ? path.width + 2 * Theme.space.m : 0
        height: Theme.space.xxl
        radius: Theme.inner
        color: Theme.raised

        Row {
            id: path
            anchors.centerIn: parent

            Repeater {
                model: prompt.chips

                Row {
                    id: step

                    required property var modelData
                    required property int index

                    Text {
                        visible: step.index > 0
                        text: prompt.between
                        color: Theme.muted
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.large
                    }

                    Text {
                        width: prompt.fitted[step.index] ?? implicitWidth
                        text: step.modelData.text
                        elide: Text.ElideRight
                        color: Theme.strong
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.large
                    }
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
            // A key whose release comes late, past the repeat delay, while
            // the shell is busy, repeats as if held
            // (modules/keyboard/repeat.nix). No field here wants a character
            // held for many, so only keys that move or delete repeat.
            if (event.isAutoRepeat && event.text !== "" && event.text >= " "
                && event.text !== "\u007f") {
                event.accepted = true;
            } else if (event.key === Qt.Key_Right && none
                && input.cursorPosition === input.text.length) {
                prompt.forward();
                event.accepted = true;
            } else if (event.key === Qt.Key_Left && prompt.leftClears && input.text !== "") {
                input.text = "";
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
        // typed as one word, and ends in an ellipsis where the field does.
        Text {
            x: input.text === "" ? Theme.space.xs : input.contentWidth
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.verticalCenter: parent.verticalCenter
            text: input.text === "" ? prompt.hint : prompt.ghost
            visible: text !== "" && (input.text === "" || !prompt.secret)
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }
    }
}
