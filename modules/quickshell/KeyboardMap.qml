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
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

PanelWindow {
    id: map

    // A key width, and the gap between keys.
    readonly property int unit: Theme.space.xxxl
    readonly property int gap: Theme.space.xs
    readonly property var shown: KeyboardLayout.layers[KeyboardLayout.layer] ?? { name: "", keys: [] }
    readonly property var first: KeyboardLayout.layers[0] ?? { keys: [] }
    readonly property int across: Math.ceil(Math.max(0, ...shown.keys.map(key => key.x + key.w)))
    readonly property int down: Math.ceil(Math.max(0, ...shown.keys.map(key => key.y + key.h)))

    visible: KeyboardLayout.shown
    anchors.bottom: true
    implicitWidth: card.width + 2 * Theme.floating.reach
    implicitHeight: card.height + Theme.space.xxxl + Theme.floating.reach
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
        width: map.across * map.unit + 2 * Theme.space.l

        Column {
            x: Theme.space.l - Theme.border
            width: parent.width - 2 * x
            topPadding: Theme.space.l - Theme.border
            bottomPadding: Theme.space.l - Theme.border
            spacing: Theme.space.l

            Row {
                spacing: Theme.space.xl

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
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.body
                    }
                }
            }

            Item {
                width: map.across * map.unit
                height: map.down * map.unit

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

                        Column {
                            anchors.centerIn: parent

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: key.cap
                                color: !key.own ? Theme.muted : key.marked ? Theme.strong : Theme.text
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                visible: key.holds !== ""
                                text: key.holds
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.small
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
