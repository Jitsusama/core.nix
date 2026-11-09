// Draws a notice as the pop-up draws it, on its card with no gutter, and as
// the held list will, with a gutter, through the one card and the one row
// (Card.qml, NoticeRow.qml), then keeps a picture of each and says where
// their columns and frames were meant to stand.
// modules/quickshell/tests/notice-columns.py reads the pictures back to the
// ink, at the scale the shell ran at.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    // The held list's gutter: two digits and the tie after them, near
    // enough; the case holds for any.
    readonly property int gutter: 40
    readonly property var cards: [pop, held]
    property int grabbed: 0

    FloatingWindow {
        visible: true
        implicitWidth: 2 * 440 + 60
        implicitHeight: 260
        color: "black"

        Card {
            id: pop
            x: 20
            y: 20
            width: 440

            // Capitals alone on the title and the signing line, so its top
            // and its foot are the capitals' and the baseline's, with no
            // descender or ascender past them.
            NoticeRow {
                width: parent.width
                mark: "\uf0e0"
                hue: Theme.program
                title: "BUILD DONE"
                words: "The machine is ready."
                from: "MAIL"
                when: "NOW"
            }
        }

        Card {
            id: held
            x: 480
            y: 20
            width: 440

            NoticeRow {
                width: parent.width
                gutter: root.gutter
                mark: "\uf0e0"
                hue: Theme.program
                title: "BUILD DONE"
                words: "The machine is ready."
                from: "MAIL"
                when: "NOW"
            }
        }
    }

    // Once both are drawn, keep a picture of each and say what they were
    // meant to be.
    Timer {
        interval: 500
        running: true
        onTriggered: {
            for (const [name, card] of [["pop", pop], ["held", held]]) {
                card.grabToImage(picture => {
                    if (!picture.saveToFile(`${Quickshell.shellDir}/${name}.png`)) {
                        console.warn(`FAIL couldn't keep the picture of ${name}`);
                        Qt.exit(1);
                        return;
                    }
                    root.grabbed += 1;
                    if (root.grabbed === 2)
                        root.say();
                });
            }
        }
    }

    function say() {
        console.log(`MEANT ${JSON.stringify({
            border: Theme.border,
            radius: Theme.radius,
            inset: pop.inset,
            text: Theme.level.name.pitch + Theme.space.rung.mark,
            gutter: root.gutter,
            width: pop.width,
            heights: [pop.height, held.height],
        })}`);
        Qt.callLater(() => Qt.exit(0));
    }
}
