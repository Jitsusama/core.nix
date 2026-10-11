// What Joel sees when he opens niri's overview: by default the time and the
// date, and how the battery, the network and the speakers are, then what's
// on only while it's true (Keep Awake, do not disturb, the mic muted), each
// with its mark. It sits on the top layer, which niri keeps above the
// overview where the backdrop's layers zoom out with the workspaces, and
// shows only while the overview is open, letting every click through, so it
// takes nothing while he works. The overview shrinks the workspaces to half
// the screen's width, centred, which leaves the left quarter for it.
//
// It lies over the picture and over any workspace that reaches under it,
// with no box behind it: the theme's halo (jitsusama.theme.wallpaper.halo)
// holds its letters and glyphs off whatever is there, feathered from crisp
// at the ink to wide and soft, so the picture still shows round it. The
// halo is the status drawn again as a silhouette in the shadow colour and
// blurred once for each of its shadows, so each is a shadow of the letters
// alone.
//
// What it shows is jitsusama.quickshell.overview.pieces, rendered into
// Overview.qml and loaded once (Facts.qml). Pieces without a label are
// headings, the time at the size its settings name and the date beneath
// it; the rest are rows, a muted label beside each value. When the left
// quarter is too narrow, pieces give way by their collapse rank, lowest
// first, until the widest fits.
import Quickshell
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects

Scope {
    id: root

    // The halo's blurs are a text shadow's, as a browser draws one. For the
    // same number MultiEffect spreads a third as far, so it's given three
    // times as much, which, measured against Chrome on the same letters,
    // matches it within a tenth. Its spread stops at 64, and past that its
    // multiplier stretches it.
    readonly property real spread: 3

    // How far the halo reaches past the letters, which its window leaves
    // room for: a little past its widest blur, where the last of it fades.
    readonly property int reach: Math.ceil(1.5 * Math.max(0, ...Theme.wallpaper.halo
        .map(shadow => shadow.blur)))

    // The halo's room on the top and left, which stops at the screen's edge.
    readonly property int lead: Math.min(reach, Theme.space.xxxl)

    // A fact shown: whether its tier wants it now, and what it says.
    function wanted(fact) {
        return fact.piece !== null && fact.piece.present
            && (fact.entry.tier === "always" || (fact.entry.tier === "when" && fact.piece.active));
    }

    // A mark's colour: its role's when that isn't plain text, else its kind's.
    function hue(piece) {
        return piece.role !== "text" ? Theme[piece.role] : Theme[piece.hue];
    }

    // Text as StyledText shows it, so a window's title can't be read as markup.
    function plain(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    component Line: Text {
        color: Theme.text
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.body
    }

    // The status once over: in its colours, or as the halo's silhouette.
    // An inline component sees nothing outside itself, so what the window
    // worked out is passed in.
    component Drawn: Column {
        id: drawn

        property bool silhouette: false
        property real cutoff: -Infinity
        property real labelWidth: 0
        readonly property alias headings: headings
        readonly property alias rows: rows

        function ink(colour) {
            return silhouette ? Theme.shadow : colour;
        }

        spacing: Theme.space.xl

        Column {
            id: headings
            spacing: Theme.space.m

            Repeater {
                model: Facts.all.filter(fact => fact.entry.label === undefined)

                Line {
                    required property var modelData
                    readonly property var entry: modelData.entry
                    readonly property bool wanted: root.wanted(modelData)

                    visible: wanted && (entry.collapse === null || entry.collapse >= drawn.cutoff)
                    height: visible ? implicitHeight : 0
                    text: modelData.piece ? modelData.piece.value : ""
                    color: drawn.ink(Theme[modelData.piece ? modelData.piece.role : "text"])
                    font.pointSize: Theme.mono[entry.size ?? "body"]
                }
            }
        }

        Column {
            id: rows
            spacing: Theme.space.xs

            Repeater {
                model: Facts.all.filter(fact => fact.entry.label !== undefined)

                Item {
                    id: row

                    required property var modelData
                    readonly property var entry: modelData.entry
                    readonly property var piece: modelData.piece
                    readonly property bool wanted: root.wanted(modelData)
                    readonly property real labelWidth: label.implicitWidth
                    // A mark goes inline before what it marks, in its colour.
                    readonly property string mark: entry.mark !== null && piece && piece.active
                        ? Theme.glyph[entry.mark] : ""

                    visible: wanted && (entry.collapse === null || entry.collapse >= drawn.cutoff)
                    implicitWidth: drawn.labelWidth + Theme.space.xl + value.implicitWidth
                    implicitHeight: value.implicitHeight
                    width: implicitWidth
                    height: visible ? implicitHeight : 0

                    Line {
                        id: label
                        text: row.entry.label
                        color: drawn.ink(Theme.muted)
                    }

                    Line {
                        id: value
                        x: drawn.labelWidth + Theme.space.xl
                        textFormat: Text.StyledText
                        text: !row.piece ? "" : (row.mark === "" ? ""
                            : `<font color="${drawn.ink(root.hue(row.piece))}">${row.mark}</font> `)
                            + root.plain(row.piece.value)
                        color: drawn.ink(Theme[row.piece ? row.piece.role : "text"])
                    }
                }
            }
        }
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
                const wanted = [...status.headings.children, ...status.rows.children]
                    .filter(item => item.wanted);
                const widest = cut => Math.max(0, ...wanted
                    .filter(item => item.entry.collapse === null || item.entry.collapse >= cut)
                    .map(item => item.implicitWidth));
                const ranks = [...new Set(wanted.map(item => item.entry.collapse)
                    .filter(rank => rank !== null))].sort((a, b) => a - b);
                let cut = -Infinity;
                for (const rank of ranks) {
                    if (widest(cut) <= room)
                        break;
                    cut = rank + 1;
                }
                return cut;
            }

            // Labels line up in one column, as wide as the widest of them.
            readonly property real labelWidth: Math.max(0, ...[...status.rows.children]
                .filter(item => item.wanted)
                .map(item => item.labelWidth))

            screen: modelData
            anchors.top: true
            anchors.left: true
            // The status sits a margin in; its window starts nearer, by the
            // halo's reach or at the screen's edge, so it isn't cut off.
            margins.top: Theme.space.xxxl - root.lead
            margins.left: Theme.space.xxxl - root.lead
            implicitWidth: Math.max(1, root.lead + status.implicitWidth + root.reach)
            implicitHeight: Math.max(1, root.lead + status.implicitHeight + root.reach)
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Top
            WlrLayershell.namespace: "status"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
            // Clicks pass through to the overview's workspaces.
            mask: Region {}
            // It fades with the overview, and is gone once faded, so it
            // draws nothing while Joel works.
            visible: Niri.overview || shown.opacity > 0

            Item {
                id: shown

                x: root.lead
                y: root.lead
                width: status.implicitWidth
                height: status.implicitHeight
                opacity: Niri.overview ? 1 : 0

                Behavior on opacity {
                    NumberAnimation {
                        duration: Niri.overview ? Theme.motion.medium : Theme.motion.short
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: Theme.motion.curve
                    }
                }

                // The silhouette, with room round it for the halo to
                // spread into, so the blur is never cut off at its edge.
                Item {
                    id: padded

                    x: -root.reach
                    y: -root.reach
                    width: status.implicitWidth + 2 * root.reach
                    height: status.implicitHeight + 2 * root.reach

                    Drawn {
                        x: root.reach
                        y: root.reach
                        silhouette: true
                        cutoff: window.cutoff
                        labelWidth: window.labelWidth
                    }
                }

                // The silhouette drawn only into the halo. Hiding it
                // outright would hide its rows too, and they'd give way.
                ShaderEffectSource {
                    id: shape
                    sourceItem: padded
                    hideSource: true
                    anchors.fill: padded
                    visible: false
                }

                Repeater {
                    model: Theme.wallpaper.halo

                    MultiEffect {
                        required property var modelData

                        source: shape
                        anchors.fill: padded
                        autoPaddingEnabled: false
                        blurEnabled: true
                        // MultiEffect's blur is a fraction of the most it may
                        // spread.
                        blurMax: Math.max(1, Math.min(64, Math.ceil(root.spread * modelData.blur)))
                        blurMultiplier: Math.max(0, root.spread * modelData.blur / 64 - 1)
                        blur: modelData.blur === 0 ? 0 : 1
                        opacity: modelData.opacity
                    }
                }

                Drawn {
                    id: status
                    cutoff: window.cutoff
                    labelWidth: window.labelWidth
                }
            }
        }
    }
}
