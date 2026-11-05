// A card's signature, as a notice's and what's playing sign theirs: the mark
// of what sent it, the name Joel knows it by, and on the right when it came
// or where it is, at Note. Its gaps are measured to the ink (docs/design.md,
// The Ladder): the mark a mark rung from the name, the name at least an
// items rung from what's on the right, which ends flush with the item. The
// item is as tall as the line's capitals, its top their top and its bottom
// their baseline, so whoever places it places the ink.
import QtQuick

Item {
    id: signature

    property string mark
    property color hue: Theme.text
    property string name
    property string side

    readonly property rect capital: capitalInk.tightBoundingRect
    readonly property rect marked: markInk.tightBoundingRect
    readonly property rect named: nameInk.tightBoundingRect
    readonly property rect sided: sideInk.tightBoundingRect

    implicitHeight: capital.height

    // The face, and where each part's ink starts and ends in it. Each is a
    // TextMetrics, whose bounds follow the face, where a call on FontMetrics
    // measures once, before the face has loaded, and a binding never asks
    // again.
    FontMetrics {
        id: face
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }
    TextMetrics {
        id: capitalInk
        font: face.font
        text: "H"
    }
    TextMetrics {
        id: markInk
        font: face.font
        text: signature.mark
    }
    TextMetrics {
        id: nameInk
        font: face.font
        text: signature.name
    }
    TextMetrics {
        id: sideInk
        font: face.font
        text: signature.side
    }

    Text {
        id: glyph
        x: -signature.marked.x
        y: signature.capital.height - baselineOffset
        text: signature.mark
        color: signature.hue
        font: face.font
    }

    Text {
        x: glyph.x + signature.marked.x + signature.marked.width + Theme.space.rung.mark
            - signature.named.x
        y: signature.capital.height - baselineOffset
        width: Math.max(0, at.x + signature.sided.x - Theme.space.rung.items - x)
        text: signature.name
        textFormat: Text.PlainText
        elide: Text.ElideRight
        color: Theme.level.note.colour
        font: face.font
    }

    Text {
        id: at
        x: signature.width - signature.sided.x - signature.sided.width
        y: signature.capital.height - baselineOffset
        text: signature.side
        textFormat: Text.PlainText
        color: Theme.level.note.colour
        font: face.font
    }
}
