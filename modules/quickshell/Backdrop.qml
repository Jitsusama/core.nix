// What Joel sees when he opens niri's overview or moves between workspaces:
// by default the time and the date, and how the battery, the network and the
// speakers are. niri's layer rule in config.kdl puts it in the backdrop
// behind the workspaces, so it shows only then and takes no space while he
// works. The overview shrinks the workspaces to half the screen's width,
// centred, which leaves the left quarter free for it.
//
// What it shows is jitsusama.quickshell.overview.pieces, rendered into
// Overview.qml. Pieces without a label are headings, the time at the size
// its settings name and the date beneath it; the rest are rows, a muted
// label beside each value. When the left quarter is too narrow, pieces give
// way by their collapse rank, lowest first, until the widest fits.
import Quickshell
import Quickshell.Wayland
import QtQuick
// Quickshell finds a folder's types only once a file it loads imports it,
// and the pieces are loaded later, by name.
import qs.pieces

Scope {
    id: root

    component Line: Text {
        color: Theme.text
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.body
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: window

            required property ShellScreen modelData

            // The left quarter, less the margin on either side of it.
            readonly property real room: modelData.width / 4 - 2 * Theme.space.xxxl

            // Everything with a collapse rank below this has given way.
            readonly property real cutoff: {
                const wanted = [...headings.children, ...rows.children].filter(item => item.wanted);
                const widest = cut => Math.max(0, ...wanted
                    .filter(item => item.entry.collapse === null || item.entry.collapse >= cut)
                    .map(item => item.implicitWidth));
                const ranks = [...new Set(wanted.map(item => item.entry.collapse).filter(rank => rank !== null))]
                    .sort((a, b) => a - b);
                let cut = -Infinity;
                for (const rank of ranks) {
                    if (widest(cut) <= room)
                        break;
                    cut = rank + 1;
                }
                return cut;
            }

            // Labels line up in one column, as wide as the widest of them.
            readonly property real labelWidth: Math.max(0, ...[...rows.children]
                .filter(item => item.wanted)
                .map(item => item.labelWidth))

            screen: modelData
            anchors.top: true
            anchors.left: true
            margins.top: Theme.space.xxxl
            margins.left: Theme.space.xxxl
            implicitWidth: Math.max(1, status.implicitWidth)
            implicitHeight: Math.max(1, status.implicitHeight)
            // niri takes only a background surface that ignores the space
            // others reserve into the backdrop.
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Background
            WlrLayershell.namespace: "backdrop"

            // One piece, loaded from its file with its settings, and drawn
            // only while its tier says so and the space allows. An inline
            // component sees nothing outside itself, so the cutoff is
            // passed in.
            component Shown: Item {
                id: shown

                required property var modelData
                required property real cutoff
                readonly property var entry: modelData
                readonly property var piece: loader.item
                readonly property bool wanted: piece !== null && piece.present
                    && (entry.tier === "always" || (entry.tier === "when" && piece.active))
                readonly property string text: !piece ? ""
                    : (entry.mark !== null && piece.active ? Theme.glyph[entry.mark] + " " : "") + piece.value

                visible: wanted && (entry.collapse === null || entry.collapse >= cutoff)

                Loader {
                    id: loader
                    Component.onCompleted: setSource("pieces/" + shown.entry.file, { settings: shown.entry })
                }
            }

            Column {
                id: status
                spacing: Theme.space.xl

                Column {
                    id: headings
                    spacing: Theme.space.m

                    Repeater {
                        model: Overview.pieces.filter(entry => entry.label === undefined)

                        Shown {
                            cutoff: window.cutoff
                            implicitWidth: heading.implicitWidth
                            implicitHeight: heading.implicitHeight
                            width: implicitWidth
                            height: visible ? implicitHeight : 0

                            Line {
                                id: heading
                                text: parent.text
                                color: Theme[parent.piece ? parent.piece.role : "text"]
                                font.pointSize: Theme.mono[parent.entry.size ?? "body"]
                            }
                        }
                    }
                }

                Column {
                    id: rows
                    spacing: Theme.space.xs

                    Repeater {
                        model: Overview.pieces.filter(entry => entry.label !== undefined)

                        Shown {
                            id: row

                            readonly property real labelWidth: label.implicitWidth

                            cutoff: window.cutoff

                            implicitWidth: window.labelWidth + Theme.space.xl + value.implicitWidth
                            implicitHeight: value.implicitHeight
                            width: implicitWidth
                            height: visible ? implicitHeight : 0

                            Line {
                                id: label
                                text: row.entry.label
                                color: Theme.muted
                            }

                            Line {
                                id: value
                                x: window.labelWidth + Theme.space.xl
                                text: row.text
                                color: Theme[row.piece ? row.piece.role : "text"]
                            }
                        }
                    }
                }
            }
        }
    }
}
