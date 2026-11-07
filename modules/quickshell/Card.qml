// The one card the launcher, the held notices and a notice are drawn in
// (docs/design.md, Shape and Kinds): a floating surface's frame, whose
// colour says where the keys go, and everything inside it standing the
// inset from the frame's inside edge on every side, measured to the ink
// (The Ladder). Whatever it holds is laid on its content, which starts the
// inset in and the inset down, so a piece sets its first ink at the
// content's top and its left, and reports as its height where its last ink
// ends; the card adds the inset below. No surface reads the inset itself
// (every-card-is-the-one-card), so a change to the card's room is a change
// to every card at once, and none can drift from the others (Joel,
// 2026-11-06).
pragma ComponentBehavior: Bound
import QtQuick

Floating {
    id: card

    // What it holds, at the inset on every side.
    default property alias inside: content.data
    // What lies over the whole card, edge to edge inside its frame, above
    // what it holds: a click that takes all of it.
    property alias whole: over.data
    // How far its content stands in from the frame's inside edge, on every
    // side: the inset rung, for a piece that reaches past the content's edge
    // to the frame, as a row's ground hugging its ink does.
    readonly property int inset: Theme.space.rung.inset
    // How wide its content is, the frame's inside less the inset each side.
    readonly property real contentWidth: content.width

    Item {
        id: room
        width: parent.width
        implicitHeight: content.implicitHeight + 2 * card.inset

        Item {
            id: content
            x: card.inset
            y: card.inset
            width: parent.width - 2 * x
            implicitHeight: childrenRect.height
        }

        Item {
            id: over
            anchors.fill: parent
        }
    }
}
