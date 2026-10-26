// The margin: what's on that Joel would otherwise forget, at the top centre
// of each screen, without covering a window (docs/design.md, Shape and
// Kinds, the margin). Each overview piece with a mark that's true right now (the
// camera, the screen shared or recorded, a pi session or a notice waiting,
// the battery low, Keep Awake, do not disturb, the microphone muted) is
// its mark alone, in the gap the windows already leave above them.
// Nothing true, nothing drawn.
//
// Each mark stands in a square cell the gap's height, drawn half that
// (Marks.sized), so it has the same room above, below and either side. The
// cells touch, and the row is centred on the screen, on the line the
// launcher is centred on, so it's as far from either side and half as far
// from where Joel looks as a corner would be; one coming or going moves
// the rest smoothly to stay centred. A mark is the theme's drawing of it
// (Theme.drawn), made for this size, where a font's glyph would blur; one
// with no drawing is its glyph, its ink fitted to the same height.
//
// They read from the right by how much each asks (Marks.js): what's on air
// rightmost, then what wants Joel, then what was left on.
// Colour says only how much: strong for what's on air or wants him, muted
// for what was left on, the alert for what needs him now. Something on air
// that goes stays a while in its place, muted
// (jitsusama.quickshell.margin.linger), so a camera let go is still seen to
// have been on. The marks are lifted off the picture by a shade of their
// own (jitsusama.theme.wallpaper.marks), centred and tight, so it's the
// same on every side and stays in the cell, off the windows below.
//
// What each is, in words, and what can be done about it, is the launcher's
// What's on (launcher/WhatsOn.qml): Super+O opens it, and clicking a mark
// opens it on that mark's row. While it's open on a screen, the mark of
// the row the keys are on wears the accent on that screen, as where the
// keys are always does, so the eye joins the row to its mark straight up
// the line they share. Nothing grows, since anything past the gap would lie
// over a window. A piece whose program has a
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

    // The grid the theme's drawings are made on (jitsusama.theme.marks).
    readonly property int grid: 12
    // Each mark's cell, its drawing and the room around it.
    readonly property var size: Marks.sized(Theme.gap)

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
            glyph: Theme.glyph[fact.piece.markGlyph || fact.entry.mark],
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
    // gathered them. One resting off the screen has no card up.
    readonly property var carded: Notices.gathered ? []
        : Object.keys(Notices.live).filter(id => !(id in Notices.resting))
            .map(id => Notices.live[id].appName.toLowerCase())
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
        model: Quickshell.screens.filter(screen => screen.name !== "")

        PanelWindow {
            id: window

            required property ShellScreen modelData

            // Pointed only on the screen What's on is open on, since that's
            // where the eye goes between the two.
            readonly property string pointed: root.asked
                && modelData.name === (root.launcher.screenHere?.name ?? "") ? root.pointed : ""

            screen: modelData
            visible: root.on.length > 0
            // The screen's whole width, so the marks move along it to stay
            // centred and the surface never resizes under them; it's clear,
            // and only the marks take the pointer.
            anchors.top: true
            anchors.left: true
            anchors.right: true
            implicitHeight: root.size.cell + lifted.reach
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "margin"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

            // Only the marks take the pointer, so the gap beside them is
            // still the windows'. A region given an item follows only that
            // item's own place, and the marks sit still inside the lifting
            // that slides them, so this one is given where they are.
            mask: Region {
                x: Math.round(lifted.x)
                width: marks.width
                height: marks.height
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

            onPointedChanged: later()

            // Says what's drawn once the pieces' words have caught up.
            function later() {
                if (Options.reports)
                    Qt.callLater(window.tell);
            }

            // What the margin holds, and how each is drawn, so a test sees
            // what Joel sees: the role each wears, and the cell it stands in.
            function tell() {
                Report.say(`holds margin ${root.on.map(thing => thing.entry.mark).join(" ")}`);
                Report.say(`sets margin ${JSON.stringify(root.on.map(thing => ({
                    mark: thing.entry.mark,
                    kind: thing.kind,
                    tone: thing.tone,
                    hue: thing.mark === window.pointed ? "accent" : thing.tone,
                    cell: root.size.cell,
                    state: thing.piece.value
                })))}`);
            }

            Lifted {
                id: lifted
                x: Math.round((window.width - marks.width) / 2)
                shadows: Theme.wallpaper.marks

                Behavior on x {
                    NumberAnimation {
                        duration: Theme.motion.medium
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: Theme.motion.curve
                    }
                }

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

                            width: root.size.cell
                            height: root.size.cell

                            Behavior on hue {
                                ColorAnimation {
                                    duration: Theme.motion.short
                                }
                            }

                            Shape {
                                visible: slot.drawing !== ""
                                x: root.size.room
                                y: root.size.room
                                width: root.grid
                                height: root.grid
                                scale: root.size.ink / root.grid
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
                                    root.size.ink, root.size.cell / 2)

                                visible: slot.drawing === ""
                                x: (root.size.cell - set.width) / 2 + set.x
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
