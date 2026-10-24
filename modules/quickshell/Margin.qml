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
// alert for what needs him now. Something on air that goes stays a while
// in its place, muted (jitsusama.quickshell.margin.linger), so a camera let
// go is still seen to have been on. The glyphs are lifted off the picture
// by the theme's text shadows, as the clock on the lock screen is.
//
// What each is, in words, and what can be done about it, is the launcher's
// What's on (launcher/WhatsOn.qml): Super+O opens it, and clicking a glyph
// opens it on that glyph's row. A piece whose program has a card on screen
// saying the same, as pi's want or a call ringing, waits for the card to
// go: a card or a mark, never both. The overview draws nothing of its own, so
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
    // in, the glyph is that much late, which nobody waits on.
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
                    // Said once the pieces' words have caught up.
                    if (Options.reports)
                        Qt.callLater(window.tell);
                }
            }

            // What the margin holds, and how each is drawn, so a test sees
            // what Joel sees.
            function tell() {
                Report.say(`holds margin ${root.on.map(thing => thing.entry.mark).join(" ")}`);
                Report.say(`sets margin ${JSON.stringify(root.on.map(thing => ({
                    mark: thing.entry.mark,
                    kind: thing.kind,
                    tone: thing.tone,
                    state: thing.piece.value
                })))}`);
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
