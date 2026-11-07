// The one card the launcher, the held notices and a notice are drawn in
// (docs/design.md, Shape and Kinds): a floating surface's frame, whose
// colour says where the keys go, and everything inside it standing the
// inset from the frame's inside edge on every side, measured to the ink
// (The Ladder). Whatever it holds is laid on its content, which starts the
// inset in and the inset down, so a piece sets its first ink at the
// content's top and its left, and reports as its height where its last ink
// ends; the card adds the inset below. A list's card is laid in bands
// instead, its head, rows and foot each centring its own line, so its
// content runs from the frame's top to its foot, still the inset in on
// either side (`bands`). No surface reads the inset itself
// (every-card-is-the-one-card), so a change to the card's room is a change
// to every card at once, and none can drift from the others (Joel,
// 2026-11-06).
pragma ComponentBehavior: Bound
import QtQuick

Floating {
    id: card

    // What it holds, at the inset on every side.
    default property alias inside: content.data
    // What lies under everything it holds, edge to edge inside its frame: a
    // click that takes all of the card but what takes its own, as an act's
    // button would, drawn over it.
    property alias whole: under.data
    // How far its content stands in from the frame's inside edge, on every
    // side: the inset rung, for a piece that reaches past the content's edge
    // to the frame, as a row's ground hugging its ink does.
    readonly property int inset: Theme.space.rung.inset
    // How wide its content is, the frame's inside less the inset each side.
    readonly property real contentWidth: content.width
    // Whether what it holds is laid in bands, as a list's card is: a head,
    // its rows and its foot, each band reaching from the frame's inside
    // edge and centring its own line, as Vim's lines fill its window
    // (Joel, 2026-11-06: rows tile like Vim's cursorline). Its content then
    // starts at the frame's top and ends at its foot, still the inset in on
    // either side, and the card adds no room above or below.
    property bool bands: false
    // How far its content stands down from the frame's inside edge.
    readonly property int above: bands ? 0 : inset

    Item {
        id: room
        width: parent.width
        implicitHeight: content.implicitHeight + 2 * card.above

        Item {
            id: under
            anchors.fill: parent
        }

        Item {
            id: content
            x: card.inset
            y: card.above
            width: parent.width - 2 * x
            implicitHeight: childrenRect.height
        }
    }
}
