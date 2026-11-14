// A card's signature, as a notice's and what's playing sign theirs: the mark
// of what sent it, unless it stands in the card's mark column instead, the
// name Joel knows it by, and on the right when it came or where it is, at
// Note, after how many more wait in strong where any do, a dot between
// them: a count is state, and state words are strong (VKCF3Z,
// line-colour-and-space, 2026-11-13). Without a mark, the name's ink starts the line. Its gaps are
// measured to the ink (docs/design.md, The Ladder): the mark a mark rung
// from the name, the name at least an items rung from what's on the right,
// which ends flush with the item. The item is as tall as the line's
// capitals, its top their top and its bottom their baseline, so whoever
// places it places the ink.
import QtQuick

Item {
    id: signature

    property string mark
    property color hue: Theme.text
    property string name
    property string count
    property string side

    // What's drawn on the right in Note's ink: the side, after a dot when a
    // count stands before it.
    readonly property string trail: count === "" ? side : `\u00b7 ${side}`

    readonly property rect capital: drawn(capitalInk.tightBoundingRect)
    readonly property rect marked: drawn(markInk.tightBoundingRect)
    readonly property rect named: drawn(nameInk.tightBoundingRect)
    readonly property rect sided: drawn(sideInk.tightBoundingRect)

    // Each part's ink is measured at the size this screen draws it, and read
    // back in the shell's own pixels: a face's bearings round differently at
    // each size, so a measure at one put the right-hand item's ink a pixel
    // short of the inset at 2 (notices-stand-on-the-cards-columns).
    readonly property real ratio: Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1
    function drawn(ink: rect): rect {
        return Qt.rect(ink.x / ratio, ink.y / ratio, ink.width / ratio, ink.height / ratio);
    }

    implicitHeight: capital.height

    // Where a signature stands, on a whole device pixel, for whoever places
    // one. Its parts' baselines sit on the capitals' foot, measured in
    // device pixels, so a signature at a fractional one leaves each Qt to
    // round its text as it sees fit: Qt 6.12 drew the line's ink a device
    // pixel lower than 6.11 did at 1.5 (notices-stand-on-the-cards-columns).
    function onPixel(at: real): real {
        return Math.round(at * ratio) / ratio;
    }

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
    FontMetrics {
        id: drawnFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize * signature.ratio
        font.weight: Theme.level.note.weight
    }
    TextMetrics {
        id: capitalInk
        font: drawnFace.font
        text: "H"
    }
    TextMetrics {
        id: markInk
        font: drawnFace.font
        text: signature.mark
    }
    TextMetrics {
        id: nameInk
        font: drawnFace.font
        text: signature.name
    }
    TextMetrics {
        id: sideInk
        font: drawnFace.font
        text: signature.trail
    }
    // The count and the space after it, in the shell's own pixels, so the
    // count stands where it did as the start of one string with the side.
    TextMetrics {
        id: countAdvance
        font: face.font
        text: signature.count === "" ? "" : `${signature.count} `
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
        x: signature.mark === "" ? -signature.named.x
            : glyph.x + signature.marked.x + signature.marked.width + Theme.space.rung.mark
                - signature.named.x
        y: signature.capital.height - baselineOffset
        width: Math.max(0, (signature.count === "" ? at.x + signature.sided.x : counted.x)
            - Theme.space.rung.items - x)
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
        text: signature.trail
        textFormat: Text.PlainText
        color: Theme.level.note.colour
        font: face.font
    }

    Text {
        id: counted
        x: at.x - countAdvance.advanceWidth
        y: signature.capital.height - baselineOffset
        visible: signature.count !== ""
        text: signature.count
        textFormat: Text.PlainText
        color: Theme.strong
        font: face.font
    }
}
