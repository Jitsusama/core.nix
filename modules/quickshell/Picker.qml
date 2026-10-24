pragma Singleton
pragma ComponentBehavior: Bound
// Picking part of the screen by the keys, for a screenshot or a recording
// (Recorder.qml), once the Capture collection is asked for a region
// (launcher/Capture.qml). Its keys work as they do on text. Letters pick
// cells of a lettered grid, shown from the start, row then column,
// alphabetically from the top left: one cell's two letters, then another's,
// make the box from one to the other. The arrows move the box a step, and
// Shift and an arrow size it from its bottom right corner, as Shift extends
// a selection. It opens on the middle of the screen, so the first key moves
// something already there. Return takes the box; Escape clears the letters
// typed, or goes back to the Capture collection. The pointer has no part.
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
import "Picking.js" as Picking

Singleton {
    id: root

    // What's being picked for, while it is: { title, glyph, done, back },
    // where done is given the box as wf-recorder and grim take it, and back
    // is called when Escape leaves.
    property var asked: null
    property var box: ({ x: 0, y: 0, width: 0, height: 0 })
    // The letters typed toward a cell, and the first cell, once picked.
    property string typed: ""
    property var first: null
    // Said for the desktop test, which waits on the letters it typed
    // before photographing them.
    onTypedChanged: Report.say(`picker typed ${typed}`)
    readonly property bool lettered: typed !== "" || first !== null
    // Set once the box is taken, so the next frame is drawn empty.
    property bool leaving: false
    readonly property int step: Theme.space.l

    function pick(title: string, glyph: string, done: var, back: var): void {
        typed = "";
        first = null;
        leaving = false;
        asked = { title, glyph, done, back };
    }

    // The box it opens on, once its window knows its screen.
    function start(screen: var): void {
        box = Picking.start(screen);
    }

    function letter(text: string, cells: var): void {
        const next = typed + text;
        if (!Picking.fits(cells, next))
            return;
        if (next.length < 2) {
            typed = next;
            return;
        }
        const cell = Picking.cell(cells, next);
        typed = "";
        if (first === null) {
            first = cell;
        } else {
            box = Picking.spanned(first, cell);
            first = null;
        }
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

    // Escape clears the letters first, so nothing is lost by pressing it
    // once.
    function back(): void {
        if (lettered) {
            typed = "";
            first = null;
            return;
        }
        const back = asked.back;
        asked = null;
        if (back)
            back();
    }

    function take(): void {
        if (first !== null)
            box = first;
        typed = "";
        first = null;
        leaving = true;
    }

    PanelWindow {
        id: window

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
        // The box drawn: the first cell while the second is being typed.
        readonly property var shown: root.first ?? root.box

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

            Keys.onPressed: event => {
                const side = {
                    [Qt.Key_Left]: "left",
                    [Qt.Key_Right]: "right",
                    [Qt.Key_Up]: "up",
                    [Qt.Key_Down]: "down"
                }[event.key];
                const shift = (event.modifiers & Qt.ShiftModifier) !== 0;
                const ctrl = (event.modifiers & Qt.ControlModifier) !== 0;
                if (side !== undefined && !ctrl) {
                    // A cell picked and an arrow pressed carries on from it.
                    if (root.first !== null)
                        root.box = root.first;
                    root.typed = "";
                    root.first = null;
                    root.box = shift ? Picking.sized(root.box, side, root.step, window.bounds)
                        : Picking.moved(root.box, side, root.step, window.bounds);
                } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    root.take();
                } else if (event.key === Qt.Key_Escape) {
                    root.back();
                } else if (event.key === Qt.Key_Backspace) {
                    if (root.typed !== "")
                        root.typed = root.typed.slice(0, -1);
                    else
                        root.first = null;
                } else if (/^[a-z]$/i.test(event.text) && !ctrl) {
                    root.letter(event.text.toLowerCase(), window.cells);
                } else {
                    return;
                }
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

            // The grid's letters, each at its cell's middle, always there so
            // nothing has to be known to use them: quiet until typing begins,
            // then the ones what's typed still leads to in the strong colour.
            Repeater {
                model: window.cells.rows * window.cells.columns

                Rectangle {
                    required property int index

                    readonly property int row: Math.floor(index / window.cells.columns)
                    readonly property int column: index % window.cells.columns
                    readonly property string label: Picking.letter(row) + Picking.letter(column)
                    readonly property bool leads: root.typed !== ""
                        && label.startsWith(root.typed)

                    x: Math.round((column + 0.5) * window.cells.width - width / 2)
                    y: Math.round((row + 0.5) * window.cells.height - height / 2)
                    width: labelText.implicitWidth + 2 * Theme.space.s
                    height: labelText.implicitHeight + Theme.space.xs
                    radius: Theme.inner
                    color: Qt.alpha(Theme.raised, Theme.floating.opacity)
                    opacity: leads ? 1 : root.typed === "" ? 0.7 : 0.3

                    Text {
                        id: labelText
                        anchors.centerIn: parent
                        text: parent.label
                        color: parent.leads ? Theme.strong : Theme.muted
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.body
                    }
                }
            }

            Floating {
                id: panel

                // The launcher's width, or the hints' if they ever need more,
                // so nothing spills past the panel.
                readonly property real size: Math.max(680, hints.implicitWidth
                    + 2 * Theme.space.l)

                x: Math.round((window.width - size) / 2)
                y: window.height - height - Theme.space.huge
                width: size
                keys: true

                Column {
                    width: parent.width

                    // What's being picked for, the letters typed, and the
                    // box's size.
                    Item {
                        width: parent.width
                        height: Theme.space.xxxl + 2 * Theme.inset

                        Text {
                            id: glyph
                            x: Theme.inset + Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.asked ? root.asked.glyph : ""
                            color: Theme.action
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                        }

                        Rectangle {
                            id: chip
                            x: Theme.inset + Theme.space.l + Theme.space.xxl
                            anchors.verticalCenter: parent.verticalCenter
                            width: chipText.implicitWidth + 2 * Theme.space.m
                            height: Theme.space.xxl
                            radius: Theme.inner
                            color: Theme.raised

                            Text {
                                id: chipText
                                anchors.centerIn: parent
                                text: root.asked ? root.asked.title : ""
                                color: Theme.strong
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                            }
                        }

                        Text {
                            x: chip.x + chip.width + Theme.space.m
                            anchors.verticalCenter: parent.verticalCenter
                            readonly property string corner: root.first === null ? "" : "\u2192 "
                            text: `<font color="${Theme.strong}">${corner}${root.typed}</font>`
                                + (root.typed === ""
                                    ? (root.first === null ? "a cell's letters" : "the other's")
                                    : "")
                            textFormat: Text.StyledText
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                            font.weight: Font.Medium
                        }

                        Text {
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.inset + Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            readonly property var measured: window.shown ?? root.box
                            text: `${measured.width}<font color="${Theme.muted}"> \u00d7 </font>`
                                + measured.height
                            textFormat: Text.StyledText
                            color: Theme.text
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                            font.features: { "tnum": 1 }
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.border
                        color: Theme.edge
                    }

                    // What the keys do here, as the launcher's footer says.
                    Item {
                        width: parent.width
                        height: Theme.space.xxl + Theme.space.s

                        KeyHints {
                            id: hints
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            keys: root.lettered
                                ? [["\u21b5", "take"], ["a\u2013z", "row, then column"],
                                    ["\u232b", "undo"], ["Esc", "clear"]]
                                : [["\u21b5", "take"], ["a\u2013z", "cells"],
                                    ["\u2190 \u2192 \u2191 \u2193", "move"],
                                    ["\u21e7", "size"], ["Esc", "back"]]
                        }
                    }
                }
            }
        }
    }
}
