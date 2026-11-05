// The keyboard's layout, at the bottom of the screen, for a glance while
// typing (docs/design.md, Shape and Kinds, the map). Super+K brings it up
// and puts it away; it never takes the keys, so typing carries on beneath
// it, and it follows the board: hold a layer and it shows that layer. The
// launcher's Keyboard collection shows a layer it chose instead, with the
// key it found marked in the accent.
//
// The layers' names run along the top, the one shown in the accent while
// no key is marked. Each key sits where it does on the board, its cap in
// the text's colour and what holding it does beneath, smaller and muted. A
// key that falls through shows the first layer's cap, muted, with nothing
// behind it, and an empty one shows nothing. A click puts it away.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

PanelWindow {
    id: map

    Counted { name: "keyboard map" }

    // A key width, a row of four beats, and the gap between keys: the
    // board is one thing and each key bounded by its own cap, so its keys
    // are tied (docs/design.md, Space, The Ladder).
    readonly property int unit: 4 * Theme.space.beat
    readonly property int gap: Theme.space.rung.tie
    readonly property int inset: Theme.space.rung.inset
    // How tall Note's capitals stand, which the names and every cap are
    // set by.
    readonly property real capitals: noteFace.tightBoundingRect("H").height

    FontMetrics {
        id: noteFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }
    readonly property var shown: KeyboardLayout.layers[KeyboardLayout.layer] ?? { name: "", keys: [] }
    readonly property var first: KeyboardLayout.layers[0] ?? { keys: [] }
    // How far the keys reach, in key widths, exactly, so a board whose last
    // key ends half a key short keeps the inset to that key's edge.
    readonly property real across: Math.max(0, ...shown.keys.map(key => key.x + key.w))
    readonly property real down: Math.max(0, ...shown.keys.map(key => key.y + key.h))

    visible: KeyboardLayout.shown
    anchors.bottom: true
    implicitWidth: card.width + 2 * Theme.floating.reach
    // A floating frame's distance from the edge, below it.
    implicitHeight: card.height + Theme.space.rung.screen + Theme.floating.reach
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    mask: card.area
    BackgroundEffect.blurRegion: card.area
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "keyboard"

    IpcHandler {
        target: "keyboard"

        // Shows the layer the board is on, or puts the map away.
        function toggle(): void {
            KeyboardLayout.toggle();
        }
    }

    Reported {
        name: "keyboard"
    }

    Floating {
        id: card
        x: Theme.floating.reach
        y: Theme.floating.reach
        // The keys, less the gap the last leaves after it, the inset from
        // the frame to the ink on every side: the names' capitals under its
        // top, the keys' caps from its sides and bottom.
        width: map.across * map.unit - map.gap + 2 * map.inset + 2 * Theme.border

        Item {
            x: map.inset
            width: parent.width - 2 * x
            implicitHeight: keys.y + keys.height + map.inset

            // The layers' names, things on one line, a heading for the
            // keys under it.
            Row {
                id: names
                x: -noteFace.tightBoundingRect(KeyboardLayout.layers[0]?.name.charAt(0) ?? "").x
                y: map.inset + map.capitals - noteFace.ascent
                spacing: Theme.space.rung.items

                Repeater {
                    model: KeyboardLayout.layers

                    Text {
                        required property var modelData
                        required property int index

                        readonly property bool here: index === KeyboardLayout.layer
                        text: here && KeyboardLayout.chosen < 0 && KeyboardLayer.locked
                            ? `${modelData.name} locked` : modelData.name
                        color: !here ? Theme.muted : KeyboardLayout.marked < 0 ? Theme.accent
                            : Theme.strong
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                    }
                }
            }

            // The keys a beat under the names' baseline: nearer the names
            // than the frame's room, further than key from key.
            Item {
                id: keys
                y: map.inset + map.capitals + Theme.space.beat
                width: map.across * map.unit - map.gap
                height: map.down * map.unit - map.gap

                Repeater {
                    model: map.shown.keys

                    Rectangle {
                        id: key

                        required property var modelData
                        required property int index

                        readonly property bool own: modelData.cap !== undefined
                        readonly property var below: map.first.keys[index] ?? {}
                        readonly property bool marked: index === KeyboardLayout.marked
                        readonly property string cap: own ? modelData.cap
                            : modelData.falls ? below.cap ?? "" : ""
                        readonly property string holds: own ? modelData.holds ?? ""
                            : modelData.falls ? below.holds ?? "" : ""

                        x: modelData.x * map.unit
                        y: modelData.y * map.unit
                        width: modelData.w * map.unit - map.gap
                        height: modelData.h * map.unit - map.gap
                        radius: Theme.inner
                        color: own ? Theme.raised : "transparent"
                        border.width: Theme.border
                        border.color: marked ? Theme.accent : "transparent"

                        // The cap, and what holding it does a line of Note
                        // under it, as lines of one saying, the two centred
                        // in the key by their ink: the cap's capitals' top
                        // to the last line's baseline.
                        Item {
                            id: says
                            anchors.fill: parent
                            readonly property real inked: map.capitals
                                + (key.holds !== "" ? Theme.level.note.pitch : 0)
                            readonly property real capBaseline: (height - inked) / 2
                                + map.capitals

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                y: says.capBaseline - noteFace.ascent
                                text: key.cap
                                color: !key.own ? Theme.muted : key.marked ? Theme.strong : Theme.text
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                y: says.capBaseline + Theme.level.note.pitch - noteFace.ascent
                                visible: key.holds !== ""
                                text: key.holds
                                color: Theme.muted
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }
                        }
                    }
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: KeyboardLayout.hide()
        }
    }
}
