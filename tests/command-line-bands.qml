// Draws the launcher's command line (CommandLine.qml) and a line of its
// menu (CommandMenu.qml) in the theme's own faces, and holds each to the
// band rule (docs/design.md, Bands): text between two drawn edges sits
// optically centred, the middles of its capitals and of its x-height
// balanced on the band's middle, rather than its line box centred, which
// leaves it top heavy (Joel, 2026-11-05: "the launcher find box text is
// not vertically aligned within its box, it is more heavily top aligned").
// It writes an x and then an H on the line, and an H and an x on a line of
// the menu, reads each picture Qt grabs back as pixels, finds each letter's
// ink, and holds the mean of their middles within a pixel of the band's.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "Commands.js" as Commands

ShellRoot {
    id: root

    // Where each is drawn in what's grabbed.
    readonly property int lineAt: 10
    readonly property int menuAt: 100
    // The menu's first line, its command an H and its title an x.
    readonly property var letters: [
        { command: "H", short: "H", title: "x", key: "" },
        { command: "x", short: "x", title: "H", key: "" }
    ]
    // The letters written on the line in turn, each grabbed and read.
    readonly property var written: ["x", "H"]
    property int grabbing: 0
    property var middles: []

    FloatingWindow {
        visible: true
        implicitWidth: 2 * room.width
        implicitHeight: room.height
        color: "black"

        Item {
            id: room
            width: 520
            height: 320

            CommandLine {
                id: line
                y: root.lineAt
                width: parent.width
                // As tall as the find line whose place it takes.
                height: Theme.space.xxxl
            }

            CommandMenu {
                id: menu
                y: root.menuAt
                width: parent.width
                height: implicitHeight
                all: root.letters
                lines: root.letters
                split: false
            }
        }

        // Where each grabbed picture is read back, a pixel at a time.
        Canvas {
            id: reader
            x: room.width
            width: room.width
            height: room.height
            property string url: ""
            onImageLoaded: requestPaint()
            // A canvas paints once as it's made, before there's a picture to
            // read, so it reads only once the picture has loaded.
            onPaint: {
                if (url === "" || !isImageLoaded(url))
                    return;
                const context = getContext("2d");
                context.clearRect(0, 0, width, height);
                context.drawImage(url, 0, 0);
                root.read(context);
            }
        }
    }

    // The faces the line and the menu are set in, to find how far across
    // each letter runs.
    TextMetrics {
        id: figure
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
    }

    TextMetrics {
        id: note
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    // How far a letter runs in a face, short of its last column, where the
    // caret may stand.
    function advance(metrics, letter) {
        metrics.text = letter;
        return Math.max(1, Math.floor(metrics.advanceWidth) - 1);
    }

    // The middle of a letter's ink, in rows of the picture, within columns
    // from `left` to `right` and rows from `top` to `bottom`: halfway from
    // the first row any of its pixels is lit, against the black around it,
    // to the last; null where none is.
    function inkMiddle(context, left, right, top, bottom) {
        const across = right - left;
        const pixels = context.getImageData(left, top, across, bottom - top).data;
        const lit = [];
        for (let row = 0; row < bottom - top; row += 1) {
            for (let column = 0; column < across; column += 1) {
                const at = 4 * (row * across + column);
                if (Math.max(pixels[at], pixels[at + 1], pixels[at + 2]) > 96) {
                    lit.push(row);
                    break;
                }
            }
        }
        return lit.length === 0 ? null : top + (lit[0] + lit[lit.length - 1] + 1) / 2;
    }

    // Writes the next letter on the line and grabs it.
    function grab() {
        line.write(Commands.typing(root.written[root.grabbing]));
        Qt.callLater(() => room.grabToImage(grabbed => {
            const file = `${Quickshell.shellDir}/grabbed-${root.grabbing}.png`;
            if (!grabbed.saveToFile(file)) {
                console.warn(`FAIL couldn't keep the grabbed picture at ${file}`);
                Qt.callLater(() => Qt.exit(1));
                return;
            }
            reader.url = `file://${file}`;
            reader.loadImage(reader.url);
        }));
    }

    // Reads the line's letter from a picture, and once the last is read,
    // the menu's from it too.
    function read(context) {
        const typed = Theme.space.l + Theme.space.xxl;
        const letter = root.written[root.grabbing];
        root.middles = root.middles.concat([root.inkMiddle(context, typed,
            typed + root.advance(figure, letter), root.lineAt, root.lineAt + line.height)]);
        root.grabbing += 1;
        if (root.grabbing < root.written.length) {
            root.grab();
            return;
        }
        const rowTop = root.menuAt + Theme.border + Theme.space.xs;
        const rowFoot = rowTop + menu.pitch;
        check([
            ["the : line's letters sit optically centred on it", root.middles,
                root.lineAt + line.height / 2],
            ["a line of the menu sits optically centred on its pitch", [
                root.inkMiddle(context, menu.commandsAt,
                    menu.commandsAt + root.advance(note, "H"), rowTop, rowFoot),
                root.inkMiddle(context, menu.titlesAt,
                    menu.titlesAt + root.advance(note, "x"), rowTop, rowFoot)
            ], rowTop + menu.pitch / 2]
        ]);
    }

    // Each case: what it holds, the two letters' ink middles, and the band's.
    function check(cases) {
        const failed = [];
        for (const [what, found, band] of cases) {
            const read = found.every(middle => middle !== null);
            const mean = read ? (found[0] + found[1]) / 2 : Number.NaN;
            console.log(`${what}: ink middles ${JSON.stringify(found)}, band ${band}`);
            if (!read || Math.abs(mean - band) > 1)
                failed.push(`${what}: the letters' ink middles ${JSON.stringify(found)}, `
                    + `their mean ${mean}, against the band's middle ${band}`);
        }
        for (const failure of failed)
            console.warn(`FAIL ${failure}`);
        Qt.callLater(() => Qt.exit(failed.length === 0 ? 0 : 1));
    }

    // Once both are drawn, grab the first letter.
    Timer {
        interval: 500
        running: true
        onTriggered: root.grab()
    }
}
