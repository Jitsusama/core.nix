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

    readonly property rect capital: face.tightBoundingRect("H")
    readonly property rect marked: face.tightBoundingRect(mark)
    readonly property rect named: face.tightBoundingRect(name)
    readonly property rect sided: face.tightBoundingRect(side)

    implicitHeight: capital.height

    FontMetrics {
        id: face
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
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
