pragma Singleton
// Picking part of the screen by the keys, for a screenshot or a recording
// (Recorder.qml). It opens on the whole screen, or on the focused window when
// it floats, since niri says where only those sit. The arrows keep the half
// they point at, Shift and an arrow move the box by its own size, Ctrl and an
// arrow push that edge out a step, and Ctrl, Shift and an arrow pull it in.
// Typing letters picks cells of a lettered grid instead, row then column,
// alphabetically from the top left: one cell's two letters, then another's,
// make the box from one to the other. Return takes the box and Escape leaves
// the grid, or the picker. The pointer has no part in it.
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

    // What's being picked for, while it is: { title, glyph, done }, where
    // done is given the box as wf-recorder and grim take it.
    property var asked: null
    property var box: ({ x: 0, y: 0, width: 0, height: 0 })
    // The letters typed toward a cell, and the first cell, once picked.
    property string typed: ""
    property var first: null
    readonly property bool lettered: typed !== "" || first !== null
    // Set once the box is taken, so the next frame is drawn empty.
    property bool leaving: false
    readonly property int step: Theme.space.l

    function pick(title: string, glyph: string, done: var): void {
        typed = "";
        first = null;
        leaving = false;
        asked = { title, glyph, done };
    }

    // The box it opens on, once its window knows its screen.
    function start(screen: var): void {
        box = { x: 0, y: 0, width: screen.width, height: screen.height };
        const focused = Niri.focused;
        const layout = focused ? focused.layout : null;
        if (layout && layout.tile_pos_in_workspace_view) {
            const [x, y] = layout.tile_pos_in_workspace_view;
            const [dx, dy] = layout.window_offset_in_tile;
            const [width, height] = layout.window_size;
            const left = Math.max(0, Math.round(x + dx)), top = Math.max(0, Math.round(y + dy));
            const right = Math.min(screen.width, Math.round(x + dx) + width);
            const bottom = Math.min(screen.height, Math.round(y + dy) + height);
            if (right - left >= 16 && bottom - top >= 16)
                box = { x: left, y: top, width: right - left, height: bottom - top };
        }
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

    // Escape leaves the grid first, so nothing is lost by pressing it once.
    function back(): void {
        if (lettered) {
            typed = "";
            first = null;
        } else {
            asked = null;
        }
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
        // The box drawn: the first cell while the second is being typed, and
        // none while the first is.
        readonly property var shown: root.lettered ? root.first : root.box

        onVisibleChanged: {
            if (visible)
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
                if (side !== undefined) {
                    if (root.first !== null)
                        root.box = root.first;
                    root.typed = "";
                    root.first = null;
                    root.box = ctrl && shift ? Picking.pulled(root.box, side, root.step)
                        : ctrl ? Picking.pushed(root.box, side, root.step, window.bounds)
                        : shift ? Picking.moved(root.box, side, window.bounds)
                        : Picking.halved(root.box, side);
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

            // The grid's letters, each at its cell's middle, the ones what's
            // typed still leads to in the strong colour.
            Repeater {
                model: root.lettered ? window.cells.rows * window.cells.columns : 0

                Rectangle {
                    required property int index

                    readonly property int row: Math.floor(index / window.cells.columns)
                    readonly property int column: index % window.cells.columns
                    readonly property string label: Picking.letter(row) + Picking.letter(column)
                    readonly property bool leads: label.startsWith(root.typed)

                    x: Math.round((column + 0.5) * window.cells.width - width / 2)
                    y: Math.round((row + 0.5) * window.cells.height - height / 2)
                    width: labelText.implicitWidth + 2 * Theme.space.s
                    height: labelText.implicitHeight + Theme.space.xs
                    radius: Theme.inner
                    color: Qt.alpha(Theme.raised, Theme.floating.opacity)
                    opacity: leads ? 1 : 0.4

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

                readonly property real size: 680

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
                            visible: root.lettered
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

                        Row {
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Theme.space.xl

                            Repeater {
                                model: root.lettered
                                    ? [["a\u2013z", "row, then column"], ["\u232b", "undo"],
                                        ["Esc", "arrows"], ["\u21b5", "take"]]
                                    : [["\u2190 \u2192 \u2191 \u2193", "halve"], ["\u21e7", "move"],
                                        ["Ctrl", "grow"], ["Ctrl \u21e7", "shrink"],
                                        ["a\u2013z", "cells"], ["\u21b5", "take"]]

                                Text {
                                    required property var modelData

                                    text: `<font color="${Theme.text}">${modelData[0]}</font> `
                                        + modelData[1]
                                    textFormat: Text.StyledText
                                    color: Theme.muted
                                    font.family: Theme.mono.family
                                    font.pointSize: Theme.mono.large
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
