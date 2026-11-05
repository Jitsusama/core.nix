// A list's gutter in normal: the rows in view numbered at Note, muted,
// relative to the cursor row, whose own number is absolute and in text, as
// Joel's Neovim sets number, relativenumber and cursorlineopt=number; or
// every row absolute, as his u l switches them (the vi-paradigm spec,
// Normal). It is normal's cue beside the rows, so off normal it draws
// nothing, keeping its room so no row moves on a mode change.
//
// It makes a number a slot, a list's height of them, rather than one a
// row, and none at all off normal, so a long list costs nothing more and
// the find costs nothing: the rows in view say where each number goes.
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
    // How many numbers it has made, now: its slots while shown, else none.
    readonly property int made: numbers.count

    // Three digits' room, so a long list never widens it, and a mark
    // before the rows, as a key stands from its word (docs/design.md,
    // The Ladder).
    width: widest("000") + Theme.space.rung.mark

    function widest(text) {
        metrics.text = text;
        return Math.ceil(metrics.advanceWidth);
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
            width: gutter.width - Theme.space.rung.mark
            horizontalAlignment: Text.AlignRight
            y: (number.row?.base ?? 0) - number.baselineOffset
            text: number.row === null ? "" : gutter.numberOf(number.row.at)
            color: number.isCursor ? Theme.text : Theme.muted
            font.family: Theme.level.note.family
            font.pointSize: Theme.level.note.pointSize
        }
    }
}
