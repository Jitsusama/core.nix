// The margin: what's on that Joel would otherwise forget, in the top right
// corner, without covering a window (docs/design.md, Shape and Kinds, the
// margin). Each overview piece with a mark that's true right now (the
// camera, the screen shared or recorded, a pi session or a notice waiting,
// the battery low, Keep Awake, do not disturb, the microphone muted) is
// its glyph alone, in the gap the windows already leave above them.
// Nothing true, nothing drawn.
//
// Every glyph's ink stands the same height, the gap less a step, on the
// gap's middle line, however tall its font draws it, so a row of them is
// level and no window moves. They read from the corner inward by how much
// each asks (Marks.js): what's on air nearest the corner, then what wants
// Joel, then what was left on, a step apart. Colour says only how much:
// strong for what's on air or wants him, muted for what was left on, the
// alert for what needs him now. The glyphs are lifted off the picture by
// the theme's text shadows, as the clock on the lock screen is.
//
// What each is, in words, and what can be done about it, is the launcher's
// What's on (launcher/WhatsOn.qml): Super+O opens it, and clicking a glyph
// opens it on that glyph's row. The overview draws nothing of its own, so
// the margin stays while it's open, the one place that says what's on.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Wayland
import QtQuick
import "Marks.js" as Marks

Scope {
    id: root

    // The launcher, which says what the glyphs are.
    required property var launcher

    // The right edge everything in the corner shares: a step inside the
    // windows' own, so nothing lines up with a window's border by chance.
    readonly property int corner: Theme.space.m + Theme.space.s
    // How tall each glyph's ink stands, with a step above and below.
    readonly property int ink: Theme.gap - Theme.space.xs

    // What's on now: every piece with a mark that's true, as { entry,
    // piece, glyph, kind, order, tone }, left to right, the corner's last.
    readonly property var on: Marks.arranged(Facts.all
        .filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active)
        .map(fact => ({
            entry: fact.entry,
            piece: fact.piece,
            glyph: Theme.glyph[fact.entry.mark],
            kind: fact.piece.kind,
            order: fact.entry.order,
            tone: Marks.tone(fact.piece)
        })))

    // Opens What's on with this thing's row selected.
    function show(thing) {
        root.launcher.into("on");
        const row = root.launcher.matches.findIndex(row => row.key === `on:${thing.entry.mark}`);
        if (row >= 0)
            root.launcher.current = row;
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: window

            required property ShellScreen modelData

            screen: modelData
            visible: root.on.length > 0
            anchors.top: true
            anchors.right: true
            implicitWidth: glyphs.width + root.corner + lifted.reach
            implicitHeight: Theme.gap + lifted.reach
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "margin"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

            // Only the glyphs take the pointer, so the gap beside them is
            // still the windows'.
            mask: Region {
                item: glyphs
            }

            // A glyph coming or going changes what's drawn while the margin
            // stays up.
            Reported {
                id: reported
                name: "margin"
            }

            Connections {
                target: root

                function onOnChanged() {
                    reported.changed();
                    if (!Options.reports)
                        return;
                    Report.say(`holds margin ${root.on.map(thing => thing.entry.mark).join(" ")}`);
                    // How each is drawn, so a test sees what Joel sees.
                    Report.say(`sets margin ${JSON.stringify(root.on.map(thing => ({
                        mark: thing.entry.mark,
                        kind: thing.kind,
                        tone: thing.tone,
                        state: thing.piece.value
                    })))}`);
                }
            }

            Lifted {
                id: lifted
                x: window.width - root.corner - glyphs.width

                Row {
                    id: glyphs
                    spacing: Theme.space.s

                    Repeater {
                        model: root.on

                        // A glyph measured at the body's size, then drawn
                        // at the size that makes its ink the set height.
                        Item {
                            id: slot

                            required property var modelData
                            readonly property var set: Marks.fit(measured.tightBoundingRect,
                                root.ink, Theme.gap / 2)

                            width: set.width
                            height: Theme.gap

                            TextMetrics {
                                id: measured
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                                text: slot.modelData.glyph
                            }

                            Text {
                                x: slot.set.x
                                y: slot.set.baseline - baselineOffset
                                text: slot.modelData.glyph
                                textFormat: Text.PlainText
                                color: Theme[slot.modelData.tone]
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body * slot.set.scale
                            }

                            MouseArea {
                                x: -Theme.space.s / 2
                                width: parent.width + Theme.space.s
                                height: parent.height
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.show(slot.modelData)
                            }
                        }
                    }
                }
            }
        }
    }
}
