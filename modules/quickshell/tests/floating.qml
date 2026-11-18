// Draws a floating surface (Floating.qml) standing over another as the
// launcher's drawer stands over its card on a screen too narrow for it
// beside it: as wide as the card, top aligned with it, its foot on the
// card's rows. It holds the drawer's foot square, so its bottom corners are
// its hairline rather than a rounded notch the card shows through
// (VKCF3Z, 2026-11-05: the shot at 1.5, two colours at each corner), and
// its top round as the card's, with the region niri blurs and the pointer
// reaches squared at its foot alike. The card beneath, standing on
// nothing, keeps its round foot. It reads what's drawn back as pixels,
// through a picture Qt grabs, and exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    readonly property int across: 300
    readonly property int down: 220
    readonly property int over: 150
    readonly property string picture: Quickshell.shellDir + "/grabbed.png"

    FloatingWindow {
        visible: true
        implicitWidth: 2 * root.across + 80
        implicitHeight: root.down + 40
        color: "black"

        // What's grabbed: the card, and the drawer over it holding the keys,
        // its frame the accent over the card's edge.
        Item {
            id: room
            width: root.across + 40
            height: root.down + 40

            Floating {
                id: card
                x: 20
                y: 20
                width: root.across
                height: root.down
            }

            Floating {
                id: drawer
                x: 20
                y: 20
                width: root.across
                height: root.over
                keys: true
                squareFoot: true
            }
        }

        // Where the grabbed picture is read back, a pixel at a time.
        Canvas {
            id: reader
            x: room.width
            width: room.width
            height: room.height
            readonly property string url: `file://${root.picture}`
            onImageLoaded: requestPaint()
            // A canvas paints once as it's made, before there's a picture to
            // read, so it reads only once the picture has loaded.
            onPaint: {
                if (!isImageLoaded(url))
                    return;
                const context = getContext("2d");
                context.drawImage(url, 0, 0);
                root.check(context);
            }
        }
    }

    function check(context) {
        const at = (x, y) => Array.from(context.getImageData(x, y, 1, 1).data);
        const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
        const left = drawer.x;
        const right = drawer.x + drawer.width - 1;
        const top = drawer.y;
        const foot = drawer.y + drawer.height - 1;
        const middle = drawer.y + Math.round(drawer.height / 2);
        const failed = [];
        const expect = (what, holds, shown) => {
            if (!holds)
                failed.push(`${what}: ${shown}`);
        };
        for (const [side, x, inward] of [["left", left, 1], ["right", right, -1]]) {
            const hairline = at(x, middle);
            expect(`the drawer's ${side} side is drawn, its hairline whole`,
                hairline[3] === 255, JSON.stringify(hairline));
            const corner = [at(x, foot), at(x + inward, foot), at(x, foot - 1)];
            expect(`the drawer's bottom ${side} corner is its hairline, square over the card`,
                corner.every(pixel => same(pixel, hairline)),
                `${JSON.stringify(corner)} against its side's ${JSON.stringify(hairline)}`);
            expect(`the drawer's top ${side} corner stays round, as the card's`,
                at(x, top)[3] === 0, JSON.stringify(at(x, top)));
            expect(`the card's bottom ${side} corner, standing on nothing, stays round`,
                at(x, card.y + card.height - 1)[3] === 0,
                JSON.stringify(at(x, card.y + card.height - 1)));
        }
        const region = area => [area.topLeftRadius, area.topRightRadius,
            area.bottomRightRadius, area.bottomLeftRadius];
        const square = [Theme.radius, Theme.radius, 0, 0];
        const round = [Theme.radius, Theme.radius, Theme.radius, Theme.radius];
        expect("the drawer's region is round at its top and square at its foot",
            same(region(drawer.area), square), JSON.stringify(region(drawer.area)));
        expect("the card's region is round at every corner",
            same(region(card.area), round), JSON.stringify(region(card.area)));
        for (const failure of failed)
            console.warn(`FAIL ${failure}`);
        Qt.callLater(() => Qt.exit(failed.length === 0 ? 0 : 1));
    }

    // Once both are drawn, grab them and read the picture back.
    Timer {
        interval: 500
        running: true
        onTriggered: room.grabToImage(grabbed => {
            if (!grabbed.saveToFile(root.picture)) {
                console.warn(`FAIL couldn't keep the grabbed picture at ${root.picture}`);
                Qt.callLater(() => Qt.exit(1));
                return;
            }
            reader.loadImage(reader.url);
        })
    }
}
