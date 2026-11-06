// A list's gutter: the rows in view numbered at Note, muted, relative to
// the cursor row, whose own number is absolute and in text, as Joel's
// Neovim sets number, relativenumber and cursorlineopt=number; or every row
// absolute, as his u l switches them and as the find and the command line
// number them, as his numbertoggle does (Joel, 2026-11-05: "real numbers in
// find mode and relative numbers in command mode"). Where it numbers
// nothing it keeps no room, since an empty gutter was a gap before the rows.
//
// It makes a number a slot, a list's height of them, rather than one a
// row, so a long list costs nothing more: the rows in view say where each
// number goes.
pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: gutter

    // The rows in view, each its place in the list (`at`, from 0) and its
    // name's baseline down from the gutter's top (`base`).
    property var rows: []
    // How many rows a list's height can show, so how many numbers to make.
    property int slots: 0
    property int current: 0
    property bool relative: true
    property bool shown: false
    // How many digits its room holds: the most its list can number, which
    // the launcher holds while a place is open, so neither a find
    // narrowing nor its first letter widens it.
    property int digits: 3
    // How many numbers it has made, now: its slots while shown, else none.
    readonly property int made: numbers.count

    // Its digits' room and a tie before the rows, as Vim's number column
    // stands a space from the text (VKCF3Z, 2026-11-05), or none where it
    // numbers nothing. Read from the metrics' own property, which follows
    // the face as it loads, where a call in a binding is asked once, before
    // it has.
    width: shown ? Math.ceil(metrics.advanceWidth) + Theme.space.rung.tie : 0

    // How wide a number is in the gutter's face, for a test to measure by.
    function widest(text) {
        probe.text = text;
        return Math.ceil(probe.advanceWidth);
    }

    TextMetrics {
        id: probe
        font: metrics.font
    }

    function numberOf(at) {
        return relative && at !== current ? Math.abs(at - current) : at + 1;
    }

    // Each number drawn, in the rows' order: its text, its ink, its
    // baseline and where it ends, for a test to read.
    function said() {
        const drawn = [];
        for (let at = 0; at < numbers.count; at += 1) {
            const number = numbers.itemAt(at) as Text;
            if (number && number.visible)
                drawn.push({ text: number.text,
                    ink: Qt.colorEqual(number.color, Theme.text) ? "text" : "muted",
                    base: number.y + number.baselineOffset,
                    ends: number.x + number.width });
        }
        return drawn;
    }

    TextMetrics {
        id: metrics
        text: "9".repeat(Math.max(1, gutter.digits))
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }

    Repeater {
        id: numbers
        model: gutter.shown ? gutter.slots : 0

        Text {
            id: number

            required property int index
            readonly property var row: gutter.rows[number.index] ?? null
            readonly property bool isCursor: number.row !== null
                && number.row.at === gutter.current

            visible: number.row !== null
            width: gutter.width - Theme.space.rung.tie
            horizontalAlignment: Text.AlignRight
            y: (number.row?.base ?? 0) - number.baselineOffset
            text: number.row === null ? "" : gutter.numberOf(number.row.at)
            color: number.isCursor ? Theme.text : Theme.muted
            font.family: Theme.level.note.family
            font.pointSize: Theme.level.note.pointSize
        }
    }
}
