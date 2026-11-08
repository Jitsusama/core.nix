pragma Singleton
pragma ComponentBehavior: Bound
// Picking part of the screen by the keys, for a screenshot or a recording
// (Recorder.qml), once the Capture collection is asked for a region
// (launcher/Capture.qml). Its keys are a Vim user's (VKCF3Z, 2026-11-06).
// It opens in normal on the middle of the screen, so the first key moves
// something already there. h j k l move the box a step, a count before
// them as many, and H J K L size it from its bottom right corner. f shows
// a lettered grid and waits for a cell's two letters, row then column,
// alphabetically from the top left, making the box that cell; a second f
// and cell make it from one to the other. Return takes the box, - goes
// back to the Capture collection, :q closes and Space ? lists the keys,
// staying up as they're used. Escape cancels what's
// begun and otherwise does nothing; the arrows move as h j k l and Shift
// with one sizes, though no hint names them. The pointer has no part.
//
// What the keys do to the box is Picking.js's, tested on its own
// (tests/picking.qml). The box is drawn clear and the rest of the screen
// shaded, and below, a panel like the launcher's says what's being picked,
// the box's size and what the keys do. Before its picture is taken the
// picker clears itself and waits for that frame, so it's never in it.
import qs
import Quickshell
import Quickshell.Wayland
import QtQuick
import "Bands.js" as Bands
import "Picking.js" as Picking

Singleton {
    id: root

    // What's being picked for, while it is: { title, glyph, done, back },
    // where done is given the box as wf-recorder and grim take it, and back
    // is called when - goes back.
    property var asked: null
    property var box: ({ x: 0, y: 0, width: 0, height: 0 })
    // How the keys stand (Picking.rested): a count, an f waiting with the
    // letters typed toward a cell, the first cell once picked, and the
    // command line.
    property var stood: Picking.rested()
    // What's begun, as the head says it, and what the last key said.
    readonly property string typed: Picking.typed(stood)
    readonly property var first: stood.first
    property string said: ""
    // Said for the desktop test, which waits on the keys it typed before
    // shooting them.
    onTypedChanged: Report.say(`picker typed ${typed}`)
    readonly property bool listing: stood.listing
    onListingChanged: Report.say(listing ? "picker lists keys" : "picker hides keys")
    onSaidChanged: {
        if (said !== "")
            Report.say(`picker says ${said}`);
    }
    // Set once the box is taken, so the next frame is drawn empty.
    property bool leaving: false
    // How far an arrow moves the box, in logical pixels. It's a move, not
    // a space between two things drawn, so no rung of the ladder names it.
    readonly property int step: 16

    function pick(title: string, glyph: string, done: var, back: var): void {
        stood = Picking.rested();
        said = "";
        leaving = false;
        asked = { title, glyph, done, back };
    }

    // The box it opens on, once its window knows its screen.
    function start(screen: var): void {
        box = Picking.start(screen);
    }

    // A key, by its name as Picking.pressed reads it, on a screen.
    function press(key: string, cells: var, bounds: var): void {
        const read = Picking.pressed(stood, key, { box, cells, screen: bounds, step });
        stood = read.stood;
        box = read.box;
        said = read.said;
        if (read.act === "take")
            take();
        else if (read.act === "back")
            back();
        else if (read.act === "close")
            asked = null;
    }

    // A screen coming or going (a dock, the lid, a monitor) leaves the box
    // measured on a screen that may be gone, and the window on Qt's
    // stand-in for one, where no surface can be made (Launcher.qml says
    // more). So the pick ends, as the launcher closes then, and Print
    // starts it again on the screens there are.
    Connections {
        target: Quickshell

        function onScreensChanged() {
            root.asked = null;
        }
    }

    // - goes back to the Capture collection it came from.
    function back(): void {
        const back = asked.back;
        asked = null;
        if (back)
            back();
    }

    function take(): void {
        stood = Picking.rested();
        leaving = true;
    }

    PanelWindow {
        id: window

        Counted { name: "picker" }

        visible: root.asked !== null
        anchors.top: true
        anchors.bottom: true
        anchors.left: true
        anchors.right: true
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "picker"

        readonly property var bounds: ({
                x: screen ? screen.x : 0,
                y: screen ? screen.y : 0,
                width: width,
                height: height
            })
        readonly property var cells: Picking.grid(bounds)
        // The box drawn, which is the first cell while the second is typed.
        readonly property var shown: root.box

        // The box it opens on comes from the screen's size, which the window
        // learns only once niri has configured it, a moment after it shows.
        // Started on showing, it opened on an empty box, and Return took
        // nothing.
        readonly property bool sized: visible && width > 0 && height > 0

        onSizedChanged: {
            if (sized)
                root.start(bounds);
        }

        Reported {
            name: "picker"
        }

        // Once the cleared frame is drawn, the window goes, and only then
        // is the program that takes the picture started, so niri has let
        // the window go before anything asks it for the screen.
        Connections {
            target: picking.Window.window
            enabled: root.leaving

            function onFrameSwapped() {
                // A frame swapped as the last one was handled can still
                // arrive, from the render thread, after the picture's
                // been asked for.
                if (root.asked === null)
                    return;
                const done = root.asked.done;
                const geometry = Picking.geometry(root.box, window.bounds);
                root.asked = null;
                root.leaving = false;
                Report.say(`picked ${geometry}`);
                done(geometry);
            }
        }

        Item {
            id: picking

            anchors.fill: parent
            visible: !root.leaving
            focus: true

            // Each key by the name Picking.pressed reads: an arrow, with
            // Shift when held, Return, Escape, Backspace and Space by name, and
            // any other by the text it types. Ctrl and Alt chords are
            // niri's or nobody's, so they pass.
            Keys.onPressed: event => {
                const named = {
                    [Qt.Key_Left]: "Left",
                    [Qt.Key_Right]: "Right",
                    [Qt.Key_Up]: "Up",
                    [Qt.Key_Down]: "Down",
                    [Qt.Key_Return]: "Return",
                    [Qt.Key_Enter]: "Return",
                    [Qt.Key_Escape]: "Escape",
                    [Qt.Key_Backspace]: "Backspace",
                    [Qt.Key_Space]: "Space"
                }[event.key];
                const shift = (event.modifiers & Qt.ShiftModifier) !== 0;
                const chord = (event.modifiers & (Qt.ControlModifier | Qt.AltModifier)) !== 0;
                const arrow = ["Left", "Right", "Up", "Down"].includes(named);
                const key = named === undefined ? event.text
                    : arrow && shift ? `Shift+${named}` : named;
                if (chord || key === "")
                    return;
                root.press(key, window.cells, window.bounds);
                event.accepted = true;
            }

            // The rest of the screen shaded, the box left clear.
            Repeater {
                model: {
                    const b = window.shown;
                    const w = window.width, h = window.height;
                    if (b === null)
                        return [{ x: 0, y: 0, width: w, height: h }];
                    return [
                        { x: 0, y: 0, width: w, height: b.y },
                        { x: 0, y: b.y + b.height, width: w, height: h - b.y - b.height },
                        { x: 0, y: b.y, width: b.x, height: b.height },
                        { x: b.x + b.width, y: b.y, width: w - b.x - b.width, height: b.height }
                    ];
                }

                Rectangle {
                    required property var modelData

                    x: modelData.x
                    y: modelData.y
                    width: modelData.width
                    height: modelData.height
                    color: Qt.alpha(Theme.shadow, 0.5)
                }
            }

            // The box's frame, in the accent since the keys move it, inside
            // a shadow's line so it reads on anything.
            Rectangle {
                visible: window.shown !== null
                x: window.shown ? window.shown.x - Theme.border : 0
                y: window.shown ? window.shown.y - Theme.border : 0
                width: window.shown ? window.shown.width + 2 * Theme.border : 0
                height: window.shown ? window.shown.height + 2 * Theme.border : 0
                color: "transparent"
                border.width: Theme.border
                border.color: Qt.alpha(Theme.shadow, 0.6)

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: Theme.border
                    color: "transparent"
                    border.width: Theme.border
                    border.color: Theme.accent
                }
            }

            // The grid's letters, each at its cell's middle, shown while an
            // f waits for two, as a jump plugin's labels show only once
            // asked for: quiet until typing begins, then the ones what's
            // typed still leads to in the strong colour. Made once, so f
            // only shows them.
            Repeater {
                model: window.cells.rows * window.cells.columns

                Rectangle {
                    required property int index

                    readonly property int row: Math.floor(index / window.cells.columns)
                    readonly property int column: index % window.cells.columns
                    readonly property string label: Picking.letter(row) + Picking.letter(column)
                    readonly property bool leads: root.stood.typed !== ""
                        && label.startsWith(root.stood.typed)

                    visible: root.stood.finding

                    x: Math.round((column + 0.5) * window.cells.width - width / 2)
                    y: Math.round((row + 0.5) * window.cells.height - height / 2)
                    // A mark's room across its letters, a tie's above and
                    // below its line.
                    width: labelText.implicitWidth + 2 * Theme.space.rung.mark
                    height: labelText.implicitHeight + 2 * Theme.space.rung.tie
                    radius: Theme.inner
                    color: Qt.alpha(Theme.raised, Theme.floating.opacity)
                    opacity: leads ? 1 : root.stood.typed === "" ? 0.7 : 0.3

                    Text {
                        id: labelText
                        anchors.centerIn: parent
                        text: parent.label
                        color: parent.leads ? Theme.strong : Theme.level.note.colour
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                        font.weight: Theme.level.note.weight
                    }
                }
            }

            Floating {
                id: panel

                // The launcher's width, or the keys' if they ever need more,
                // with the frame's inset to the ink each side of them, so
                // nothing spills past the panel.
                readonly property real size: Math.max(680, foot.wanted)
                readonly property int inset: Theme.space.rung.inset

                x: Math.round((window.width - size) / 2)
                // A floating frame's distance from the screen's edge.
                y: window.height - height - Theme.space.rung.screen
                width: size
                keys: true

                Column {
                    width: parent.width

                    // What's being picked for, as a launcher place's head
                    // names it, at Title in text; the letters typed, as
                    // the query is, at Figure, or the voice's hint before
                    // any are; and the box's size, a figure too. All sit on
                    // the Title's baseline, its capitals the frame's inset
                    // under its top. No rows under it set its band, so the
                    // rule under it falls where the line reads centred
                    // (docs/design.md: text in a band centres in it; VKCF3Z,
                    // 2026-11-05).
                    Item {
                        width: parent.width
                        // TextMetrics, since a FontMetrics' call in a
                        // binding is asked once, before the face has loaded.
                        readonly property real capitals: titleCaps.tightBoundingRect.height
                        // Whole device pixels tall, counted from the frame's
                        // outer edge, so the rule under it falls clean.
                        height: rule.whole(Bands.around(panel.inset + capitals, capitals,
                            titleSmall.tightBoundingRect.height) + Theme.border) - Theme.border

                        TextMetrics {
                            id: titleCaps
                            text: "H"
                            font.family: Theme.level.title.family
                            font.pointSize: Theme.level.title.pointSize
                            font.variableAxes: Theme.level.title.variableAxes
                        }
                        TextMetrics {
                            id: titleSmall
                            text: "x"
                            font: titleCaps.font
                        }

                        Text {
                            id: glyph
                            x: panel.inset
                            anchors.baseline: title.baseline
                            text: root.asked ? root.asked.glyph : ""
                            color: Theme.action
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize
                            font.weight: Theme.level.figure.weight
                        }

                        Text {
                            id: title
                            // A glyph's column on from the glyph.
                            x: panel.inset + Theme.level.name.pitch + Theme.space.rung.mark
                            y: panel.inset + parent.capitals - baselineOffset
                            text: root.asked ? root.asked.title : ""
                            color: Theme.text
                            font.family: Theme.level.title.family
                            font.pointSize: Theme.level.title.pointSize
                            font.weight: Theme.level.title.weight
                            font.letterSpacing: Theme.level.title.letterSpacing
                            font.variableAxes: Theme.level.title.variableAxes
                        }

                        Text {
                            id: letters
                            // Apart from the title, as a title's hint stands.
                            x: title.x + title.width + Theme.space.rung.apart
                            anchors.baseline: title.baseline
                            text: root.typed
                            textFormat: Text.PlainText
                            color: Theme.strong
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize
                            font.weight: Theme.level.figure.weight
                            font.features: Theme.level.figure.features
                        }

                        Text {
                            x: letters.x + letters.width
                            anchors.baseline: title.baseline
                            // A space after what's typed, as a hint stands
                            // from its word; what the last key said, or
                            // while f waits, which corner its letters pick.
                            leftPadding: root.typed === "" ? 0 : Theme.space.rung.mark
                            text: root.said !== "" ? root.said
                                : !root.stood.finding ? ""
                                : root.first === null ? "a corner" : "the other corner"
                            color: root.said.startsWith("E") ? Theme.alert
                                : Theme.level.voice.colour
                            font.family: Theme.level.voice.family
                            font.pointSize: Theme.level.voice.pointSize
                            font.weight: Theme.level.voice.weight
                            font.italic: Theme.level.voice.italic
                            font.variableAxes: Theme.level.voice.variableAxes
                        }

                        Text {
                            anchors.right: parent.right
                            anchors.rightMargin: panel.inset
                            anchors.baseline: title.baseline
                            readonly property var measured: window.shown ?? root.box
                            text: `${measured.width}<font color="${Theme.muted}"> \u00d7 </font>`
                                + measured.height
                            textFormat: Text.StyledText
                            color: Theme.text
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize
                            font.weight: Theme.level.figure.weight
                            font.features: Theme.level.figure.features
                        }
                    }

                    Hairline {
                        id: rule
                        width: parent.width
                    }

                    // Space ?: every key, under its mode's heading, the
                    // keys in the text colour and what they do muted, as
                    // the launcher lists its own. A beat above and below,
                    // each line a Note pitch, and a beat more before the
                    // next heading.
                    Column {
                        id: listed
                        visible: root.stood.listing
                        width: parent.width
                        topPadding: Theme.space.beat
                        bottomPadding: Theme.space.beat

                        // The keys' column: the widest keys and a group's
                        // room, so every saying starts on one edge.
                        TextMetrics {
                            id: widestKeys
                            text: "Space ?"
                            font.family: Theme.level.note.family
                            font.pointSize: Theme.level.note.pointSize
                            font.weight: Theme.level.note.weight
                        }

                        Repeater {
                            model: root.stood.listing ? Picking.keyList() : []

                            Item {
                                id: keyLine

                                required property var modelData
                                required property int index

                                readonly property bool heading: modelData.heading !== ""

                                width: listed.width
                                height: Theme.level.note.pitch
                                    + (heading && index > 0 ? Theme.space.beat : 0)

                                Text {
                                    x: panel.inset
                                    anchors.bottom: parent.bottom
                                    height: Theme.level.note.pitch
                                    verticalAlignment: Text.AlignVCenter
                                    visible: keyLine.heading
                                    text: keyLine.modelData.heading
                                    color: Theme.level.label.colour
                                    font.family: Theme.level.label.family
                                    font.pointSize: Theme.level.label.pointSize
                                    font.weight: Theme.level.label.weight
                                    font.capitalization: Theme.level.label.capitalization
                                    font.letterSpacing: Theme.level.label.letterSpacing
                                    font.features: Theme.level.label.features
                                }

                                Text {
                                    id: lineKeys
                                    x: panel.inset
                                    anchors.bottom: parent.bottom
                                    height: Theme.level.note.pitch
                                    verticalAlignment: Text.AlignVCenter
                                    visible: !keyLine.heading
                                    text: keyLine.modelData.keys
                                    color: Theme.text
                                    font: widestKeys.font
                                }

                                Text {
                                    x: panel.inset + widestKeys.advanceWidth
                                        + Theme.space.rung.groups
                                    anchors.baseline: lineKeys.baseline
                                    visible: !keyLine.heading
                                    text: keyLine.modelData.does
                                    color: Theme.level.note.colour
                                    font: widestKeys.font
                                }
                            }
                        }
                    }

                    Hairline {
                        visible: root.stood.listing
                        width: parent.width
                    }

                    // What the keys do here, as a prompt's foot says it.
                    PromptFoot {
                        id: foot
                        width: parent.width
                        dotted: true
                        // Return last, as the status line has it, and
                        // Space ? before it for the rest; an f waiting
                        // names the letters it waits for. The arrows move
                        // unnamed, and Escape is never said.
                        keys: root.stood.finding
                            ? [["a\u2013z", "row, then column"], ["\u21b5", "take"]]
                            : [["f", "cells"], ["h j k l", "move"], ["H J K L", "size"],
                                ["Space ?", "keys"], ["\u21b5", "take"]]
                    }
                }
            }
        }
    }
}
