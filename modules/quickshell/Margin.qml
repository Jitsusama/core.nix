// The margin: what's on that Joel would otherwise forget, in the top right
// corner, without covering a window (docs/design.md, Shape and Kinds, the
// margin). Each overview piece with a mark that's true right now (the
// camera, the screen shared or recorded, a pi session or a notice waiting,
// the battery low, Keep Awake, do not disturb, the microphone muted) is
// its mark alone, in the gap the windows already leave above them.
// Nothing true, nothing drawn.
//
// Each mark stands in a square cell the gap's height, drawn half that
// (Marks.sized), so it has the same room above, below and either side. The
// cells touch, and the last ends where the windows do, the gap in from the
// screen's edge, so the row lines up with the windows below it and each
// mark is inlaid from that line by its room. A mark is the theme's drawing
// of it (Theme.drawn), made for this size, where a font's glyph would blur;
// one with no drawing is its glyph, its ink fitted to the same height.
//
// They read from the corner inward by how much each asks (Marks.js): what's
// on air nearest the corner, then what wants Joel, then what was left on.
// Colour says only how much: strong for what's on air or wants him, muted
// for what was left on, the alert for what needs him now. Something on air
// that goes stays a while in its place, muted
// (jitsusama.quickshell.margin.linger), so a camera let go is still seen to
// have been on. The marks are lifted off the picture by the theme's text
// shadows, as the clock on the lock screen is.
//
// What each is, in words, and what can be done about it, is the launcher's
// What's on (launcher/WhatsOn.qml): Super+O opens it, and clicking a mark
// opens it on that mark's row. While it's open on a screen, that screen's
// marks grow to twice their size, over the windows' tops, so the eye joins
// the corner to the list, and the mark of the row the keys are on wears the
// accent, as where the keys are always does. A piece whose program has a
// card on screen saying the same, as pi's want or a call ringing, waits for
// the card to go: a card or a mark, never both. The overview draws nothing
// of its own, so the margin stays while it's open, the one place that says
// what's on.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Wayland
import QtQuick
import QtQuick.Shapes
import "Marks.js" as Marks

Scope {
    id: root

    // The launcher, which says what the marks are.
    required property var launcher

    // The right edge everything in the corner shares: the windows' own.
    readonly property int corner: Theme.gap
    // The grid the theme's drawings are made on (jitsusama.theme.glyphs).
    readonly property int grid: 12
    // The cells at rest and grown, which the window is always big enough for.
    readonly property var most: Marks.sized(Theme.gap, true)

    // What the launcher shows, as Marks.pointed reads it: whether it's
    // open, the collection it holds first, and the key of each row it holds
    // and of the one selected.
    readonly property var showing: ({
        open: launcher.open,
        collection: launcher.held.length > 0 ? launcher.held[0].row.from?.settings.name ?? "" : "",
        keys: launcher.held.map(step => step.row.key ?? "")
            .concat(launcher.selected ? [launcher.selected.key ?? ""] : [])
    })
    readonly property bool asked: Marks.asked(showing)
    readonly property string pointed: Marks.pointed(showing)

    // Every piece with a mark that's true now, as { entry, piece, mark,
    // glyph, kind, order, tone }.
    readonly property var lit: Facts.all
        .filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active && !fact.piece.collected)
        .map(fact => ({
            entry: fact.entry,
            piece: fact.piece,
            mark: fact.entry.mark,
            app: fact.piece.app,
            glyph: Theme.glyph[fact.entry.mark],
            kind: fact.piece.kind,
            order: fact.entry.order,
            tone: Marks.tone(fact.piece)
        }))
    // What was on air and has just gone, which stays a while, muted
    // (Marks.lingers), and what was lit before the last change.
    property var kept: []
    property var before: []
    readonly property int linger: Options.margin.linger * 1000
    // What has just come true and waits a moment for its card
    // (Marks.awaited), long enough for a notice to be sent and shown on a
    // busy machine; with no card coming, as for a want in the window Joel's
    // in, the mark is that much late, which nobody waits on.
    property var waits: []
    readonly property int wait: 2000
    readonly property var waiting: waits.map(one => one.mark)
    // The programs with a card on screen, whose pieces' marks wait for it
    // to go, as a card or a mark, never both; none while the launcher has
    // gathered them.
    readonly property var carded: Notices.gathered ? []
        : Object.keys(Notices.live).map(id => Notices.live[id].appName.toLowerCase())
    // What's drawn: what's lit and has no card up, and what lingers, left
    // to right, the corner's last.
    readonly property var on: Marks.arranged(lit
        .filter(thing => !root.waiting.includes(thing.mark))
        .filter(thing => thing.piece.app === ""
            || !root.carded.includes(thing.piece.app.toLowerCase()))
        .concat(kept.map(one => one.thing)))

    // Lingers what just went, holds back what just came for its card, and
    // wakes when the soonest of either is due.
    function settle() {
        const at = Date.now();
        root.kept = Marks.lingers(root.kept, root.before, root.lit, {
            at,
            linger: root.linger
        });
        root.waits = Marks.awaited(root.waits, root.before, root.lit, {
            at,
            wait: root.wait
        });
        root.before = root.lit;
        const times = root.kept.concat(root.waits).map(one => one.until);
        if (times.length === 0) {
            due.stop();
            return;
        }
        const soonest = times.reduce((soon, until) => Math.min(soon, until), times[0]);
        due.interval = Math.max(0, soonest - at);
        due.restart();
    }

    onLitChanged: settle()

    Timer {
        id: due
        onTriggered: root.settle()
    }

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

            // Grown only on the screen What's on is open on, since that's
            // where the eye goes between the two.
            readonly property bool grown: root.asked
                && modelData.name === (root.launcher.screenHere?.name ?? "")
            readonly property string pointed: grown ? root.pointed : ""
            // The cell's size it's going to, and the one drawn, which moves
            // there smoothly; the drawing and its room follow from that.
            readonly property var size: Marks.sized(Theme.gap, grown)
            property real cell: size.cell
            readonly property var drawn: Marks.sized(cell, false)

            Behavior on cell {
                NumberAnimation {
                    duration: Theme.motion.medium
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }

            screen: modelData
            visible: root.on.length > 0
            anchors.top: true
            anchors.right: true
            // Always big enough for the marks grown, so growing never
            // resizes the surface under them; it's clear, and only the
            // marks take the pointer.
            implicitWidth: root.on.length * root.most.cell + root.corner + lifted.reach
            implicitHeight: root.most.cell + lifted.reach
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "margin"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

            // Only the marks take the pointer, so the gap beside them is
            // still the windows'.
            mask: Region {
                item: marks
            }

            // A mark coming or going changes what's drawn while the margin
            // stays up.
            Reported {
                id: reported
                name: "margin"
            }

            Connections {
                target: root

                function onOnChanged() {
                    reported.changed();
                    window.later();
                }
            }

            onSizeChanged: later()
            onPointedChanged: later()

            // Says what's drawn once the pieces' words have caught up.
            function later() {
                if (Options.reports)
                    Qt.callLater(window.tell);
            }

            // What the margin holds, and how each is drawn, so a test sees
            // what Joel sees: the role each wears, and the cell it's grown to.
            function tell() {
                Report.say(`holds margin ${root.on.map(thing => thing.entry.mark).join(" ")}`);
                Report.say(`sets margin ${JSON.stringify(root.on.map(thing => ({
                    mark: thing.entry.mark,
                    kind: thing.kind,
                    tone: thing.tone,
                    hue: thing.mark === window.pointed ? "accent" : thing.tone,
                    cell: window.size.cell,
                    state: thing.piece.value
                })))}`);
            }

            Lifted {
                id: lifted
                x: window.width - root.corner - marks.width

                Row {
                    id: marks

                    Repeater {
                        model: root.on

                        Item {
                            id: slot

                            required property var modelData
                            readonly property string drawing: Theme.drawn[modelData.mark] ?? ""
                            property color hue: modelData.mark === window.pointed
                                ? Theme.accent : Theme[modelData.tone]

                            width: window.cell
                            height: window.cell

                            Behavior on hue {
                                ColorAnimation {
                                    duration: Theme.motion.short
                                }
                            }

                            Shape {
                                visible: slot.drawing !== ""
                                x: window.drawn.room
                                y: window.drawn.room
                                width: root.grid
                                height: root.grid
                                scale: window.drawn.ink / root.grid
                                transformOrigin: Item.TopLeft
                                preferredRendererType: Shape.CurveRenderer

                                ShapePath {
                                    fillColor: slot.hue
                                    strokeWidth: -1
                                    fillRule: ShapePath.OddEvenFill

                                    PathSvg {
                                        path: slot.drawing
                                    }
                                }
                            }

                            // A mark with no drawing: its glyph, measured at
                            // the body's size, then drawn at the size that
                            // makes its ink the drawings' height, centred.
                            TextMetrics {
                                id: measured
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                                text: slot.modelData.glyph
                            }

                            Text {
                                readonly property var set: Marks.fit(measured.tightBoundingRect,
                                    window.drawn.ink, window.cell / 2)

                                visible: slot.drawing === ""
                                x: (window.cell - set.width) / 2 + set.x
                                y: set.baseline - baselineOffset
                                text: slot.modelData.glyph
                                textFormat: Text.PlainText
                                color: slot.hue
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body * set.scale
                            }

                            MouseArea {
                                anchors.fill: parent
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
