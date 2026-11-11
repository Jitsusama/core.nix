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
// and the signing line start on the text column. A notice sent with a
// picture of its own, as an avatar, has the picture there instead, two
// pitches square from the title's capitals, as music's cover stands, and
// its mark moves to the signature, before the sender's name. Its top is its
// highest ink, the mark's where it stands above the title's capitals, and
// its foot the signing line's baseline, or the picture's foot below it
// clearing the signature, so the card it's on holds its ink the inset from
// the frame on every side.
//
// A notice sent at low urgency is quieter than the rest by two cues: its
// mark muted rather than in its kind's colour, and its title in the text's
// colour rather than strong. A critical one is told by its card's alert
// edge, so three levels read as three with no colour of their own (VKCF3Z,
// 2026-11-07).
//
// Its words are styled text, carrying only what the notices daemon lets
// through (modules/notices' body.rs);
// a link is underlined in the words' own colour, since colour goes on
// glyphs and never on text, and a click on it says where it goes.
import QtQuick
import "Columns.js" as Columns

Item {
    id: row

    // How wide the card's gutter is: 0 on the pop-up, which isn't a list,
    // or the held list's digits and the tie after them.
    property real gutter: 0
    property string mark
    property color hue: Theme.text
    // The notice's own picture, or none.
    property string picture
    // Whether it was sent at low urgency.
    property bool quiet: false
    property string title
    // What it says, as styled text.
    property string words
    property string from
    property string when

    // A link in its words was clicked.
    signal linked(string href)

    readonly property bool pictured: picture !== ""
    readonly property color markHue: quiet ? Theme.muted : hue
    readonly property var columns: Columns.across(width, gutter, pictured
        ? Columns.ofPicture(cover.width) : Columns.ofMark(Theme.level.name.pitch),
        Theme.space.rung)
    // Where the title's first baseline stands, which its mark centres on.
    readonly property real firstBaseline: who.y + who.baselineOffset
    // The message's last baseline, who's or what's.
    // A picture reaching lower ends it at its foot instead.
    readonly property real ends: Math.max(body.visible
        ? body.y + body.baselineOffset + (body.lineCount - 1) * Theme.level.words.pitch
        : who.y + who.baselineOffset + (who.lineCount - 1) * Theme.level.name.pitch,
        pictured ? cover.y + cover.height : 0)
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
        color: row.markHue
        visible: !row.pictured
    }

    Cover {
        id: cover
        x: row.columns.mark
        y: who.y + who.baselineOffset + capital.tightBoundingRect.y
        source: row.picture
    }

    // Its capitals' top at the row's top, or lower by as far as the mark's
    // ink stands above them, so the row's top is its highest ink. A notice
    // with no mark has no ink there, and so no overhang.
    Text {
        id: who
        x: row.columns.text
        y: (row.pictured ? 0 : mark.overhang) - baselineOffset - capital.tightBoundingRect.y
        width: row.columns.end - x
        text: row.title
        // Plain, as the spec has a summary: only the body carries markup.
        textFormat: Text.PlainText
        color: row.quiet ? Theme.text : Theme.strong
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
        linkColor: color
        onLinkActivated: href => row.linked(href)
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
        y: signature.onPixel(row.ends + row.signed)
        width: row.columns.end - x
        mark: row.pictured ? row.mark : ""
        hue: row.markHue
        name: row.from
        side: row.when
    }
}
