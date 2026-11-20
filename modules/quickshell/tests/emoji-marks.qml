// Draws the first emoji of each of Emoji's groups through the one mark
// (Mark.qml), each beside a name's capitals on its line, on lines a quarter
// of a point apart, since where a line falls between pixels moves its ink,
// and the theme's home mark beside them as the mark every row already
// stands right. Keeps a picture of each on a clear ground and says what it
// drew, and which of the theme's marks it takes for pictures.
// modules/quickshell/tests/emoji-marks.py reads them back to the ink, at
// the scale the shell ran at.
import Quickshell
import QtQuick
import "MarkSize.js" as MarkSize

ShellRoot {
    id: root

    readonly property var offsets: [0, 0.25, 0.5, 0.75]
    // Each of Unicode's groups' first, as the clipboard history marked a
    // group's row on 19 Nov (clipboard/src/emoji.rs); three below the
    // emoji blocks that are pictures unasked, each filling its box, as a
    // cup doesn't, whose faint steam the ink leaves out; and a heart asking
    // for its picture, which a selector, not its block, makes one.
    readonly property var glyphs: [
        { name: "home", glyph: Theme.glyph.home },
        { name: "smileys", glyph: String.fromCodePoint(0x1f600) },
        { name: "people", glyph: String.fromCodePoint(0x1f44b) },
        { name: "animals", glyph: String.fromCodePoint(0x1f435) },
        { name: "food", glyph: String.fromCodePoint(0x1f347) },
        { name: "travel", glyph: String.fromCodePoint(0x1f30d) },
        { name: "activities", glyph: String.fromCodePoint(0x1f383) },
        { name: "objects", glyph: String.fromCodePoint(0x1f453) },
        { name: "symbols", glyph: String.fromCodePoint(0x1f3e7) },
        { name: "flags", glyph: String.fromCodePoint(0x1f3c1) },
        { name: "ball", glyph: String.fromCodePoint(0x26bd) },
        { name: "star", glyph: String.fromCodePoint(0x2b50) },
        { name: "done", glyph: String.fromCodePoint(0x2705) },
        { name: "heart", glyph: `${String.fromCodePoint(0x2764)}\ufe0f` }
    ]
    // Letters beside the pictures, none of which is one: a star and a heart
    // drawn as text, a key's sign, an ornament and an arrow past the plane,
    // and a capital.
    readonly property var letters: [0x2605, 0x2764, 0x2318, 0x1f650, 0x1f800, 0x41]
    readonly property var cellsMade: [].concat(...offsets.map(by => glyphs.map(one => ({
        name: `${one.name}-${by}`,
        glyph: one.glyph,
        by: by
    }))))
    property int grabbed: 0

    FloatingWindow {
        visible: true
        implicitWidth: 100 * root.glyphs.length
        implicitHeight: 100 * root.offsets.length
        color: "transparent"

        Grid {
            columns: root.glyphs.length

            Repeater {
                id: cells
                model: root.cellsMade

                Item {
                    id: cell
                    required property var modelData
                    width: 100
                    height: 100

                    Mark {
                        x: 10
                        line: 60 + cell.modelData.by
                        text: cell.modelData.glyph
                        color: "white"
                    }
                    // The capitals the mark centres on, as a row's name
                    // draws them.
                    Text {
                        x: 60
                        y: 60 + cell.modelData.by - baselineOffset
                        text: "H"
                        color: "white"
                        font.family: Theme.level.name.family
                        font.pointSize: Theme.level.name.pointSize
                        font.weight: Theme.level.name.weight
                    }
                }
            }
        }
    }

    // Once every cell is drawn, keep a picture of each and say what they are.
    Timer {
        interval: 800
        running: true
        onTriggered: {
            for (let at = 0; at < cells.count; at += 1) {
                const name = root.cellsMade[at].name;
                cells.itemAt(at).grabToImage(picture => {
                    if (!picture.saveToFile(`${Quickshell.shellDir}/${name}.png`)) {
                        console.warn(`FAIL couldn't keep the picture of ${name}`);
                        Qt.exit(1);
                        return;
                    }
                    root.grabbed += 1;
                    if (root.grabbed === cells.count)
                        root.say();
                });
            }
        }
    }

    // The measures MarkSize.js mustn't keep, each named: a picture traced as
    // a letter, its path empty beside a box not yet measured, as a row
    // handed an emoji where a letter was kept on Joel's screen (2026-11-19);
    // a picture's box with no height; and a letter traced with no box.
    function wronglyKept() {
        const face = Theme.level.figure.family;
        const keyOf = text => [face, 1, text].join("\t");
        const nothing = { x: 0, y: 0, width: 0, height: 0 };
        const box = { x: 0, y: -10, width: 10, height: 10 };
        const kept = [
            ["a picture traced", String.fromCodePoint(0x1f600),
                Object.assign({ outline: 1.4142135623730951 }, nothing)],
            ["a picture traced with its box", String.fromCodePoint(0x1f44b),
                Object.assign({ outline: 1.4142135623730951 }, box)],
            ["a picture with no box", String.fromCodePoint(0x1f435),
                Object.assign({ outline: 0, pictured: true }, nothing)],
            ["a letter with no box", "H", Object.assign({ outline: 12 }, nothing)]
        ];
        return kept.filter(([, text, measure]) => {
            MarkSize.measured(keyOf(text), measure, true);
            return MarkSize.known(keyOf(text)) !== null;
        }).map(([what]) => what);
    }

    function say() {
        console.log(`MEANT ${JSON.stringify({
            offsets: root.offsets,
            glyphs: root.glyphs.map(one => one.name),
            pictured: root.glyphs.filter(one => MarkSize.pictured(one.glyph))
                .map(one => one.name),
            lettersPictured: root.letters
                .filter(point => MarkSize.pictured(String.fromCodePoint(point)))
                .map(point => point.toString(16)),
            themePictured: Object.keys(Theme.glyph)
                .filter(name => MarkSize.pictured(Theme.glyph[name])),
            emojiFace: Qt.fontFamilies().includes("Noto Color Emoji"),
            wronglyKept: root.wronglyKept()
        })}`);
        Qt.callLater(() => Qt.exit(0));
    }
}
