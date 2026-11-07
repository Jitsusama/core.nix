// A notice as a row of its card, the pop-up's and the held list's alike,
// standing in its card's columns (Columns.js) so a notice reads the same
// wherever it's drawn (VKCF3Z, 2026-11-06). Three groups, as Joel reads a
// notice: who or what it's about, what it says, and the signature of what
// sent it with when, each at its level, so the sizes rank rather than
// jostle: who and what at the launcher's size, lines of one saying a pitch
// apart, and the signature a step smaller, a group of its own (Joel,
// 2026-10-24: "the differing font sizes, and too much information on lines
// really felt gross and unrefined"; 2026-11-04: "we are not levraging
// different weights and sizes enough").
//
// The sender's mark stands in the mark column on the title's first line,
// centred on its capitals, as a launcher row's does; the title, its words
// and the signing line start on the text column. Its top is its highest
// ink, the mark's where it stands above the title's capitals, and its foot
// the signing line's baseline, so the card it's on holds its ink the inset
// from the frame on every side.
import QtQuick
import "Columns.js" as Columns

Item {
    id: row

    // How wide the card's gutter is: 0 on the pop-up, which isn't a list,
    // or the held list's digits and the tie after them.
    property real gutter: 0
    property string mark
    property color hue: Theme.text
    property string title
    property string words
    property string from
    property string when

    readonly property var columns: Columns.across(width, gutter,
        Columns.ofMark(Theme.level.name.pitch), Theme.space.rung)
    // Where the title's first baseline stands, which its mark centres on.
    readonly property real firstBaseline: who.y + who.baselineOffset
    // The message's last baseline, who's or what's.
    readonly property real ends: body.visible
        ? body.y + body.baselineOffset + (body.lineCount - 1) * Theme.level.words.pitch
        : who.y + who.baselineOffset + (who.lineCount - 1) * Theme.level.name.pitch
    // How far the signature's capitals clear the message above them: a beat
    // more than Note's pitch from the message's last baseline, less the
    // capitals' own height, so the signature reads as a group of its own,
    // never as the message's last line.
    readonly property real signed: Theme.level.note.pitch + Theme.space.beat
        - signature.capital.height

    implicitHeight: signature.y + signature.height

    // A capital in the title's face, so the room above is measured to its
    // top rather than to its line's.
    TextMetrics {
        id: capital
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
        text: "H"
    }

    Mark {
        id: mark
        x: row.columns.mark
        line: row.firstBaseline
        text: row.mark
        color: row.hue
    }

    // Its capitals' top at the row's top, or lower by as far as the mark's
    // ink stands above them, so the row's top is its highest ink. A notice
    // with no mark has no ink there, and so no overhang.
    Text {
        id: who
        x: row.columns.text
        y: mark.overhang - baselineOffset - capital.tightBoundingRect.y
        width: row.columns.end - x
        text: row.title
        // Plain, as the spec has a summary: only the body carries markup.
        textFormat: Text.PlainText
        color: Theme.strong
        wrapMode: Text.Wrap
        lineHeight: Theme.level.name.pitch
        lineHeightMode: Text.FixedHeight
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
    }

    // What it says, its first line a pitch below who's last, as lines of
    // one saying.
    Text {
        id: body
        x: row.columns.text
        y: who.y + who.baselineOffset + (who.lineCount - 1) * Theme.level.name.pitch
            + Theme.level.words.pitch - baselineOffset
        width: row.columns.end - x
        text: row.words
        visible: row.words !== ""
        color: Theme.text
        wrapMode: Text.Wrap
        maximumLineCount: 6
        elide: Text.ElideRight
        textFormat: Text.StyledText
        lineHeight: Theme.level.words.pitch
        lineHeightMode: Text.FixedHeight
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.weight: Theme.level.words.weight
        font.variableAxes: Theme.level.words.variableAxes
    }

    // What sent it, a group below: the name Joel knows it by, never its
    // program's, its mark already in the mark column; on the right when it
    // came, and whatever else the card says there.
    Signature {
        id: signature
        x: row.columns.text
        y: row.ends + row.signed
        width: row.columns.end - x
        name: row.from
        side: row.when
    }
}
