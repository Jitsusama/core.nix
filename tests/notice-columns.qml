// Draws a notice as the arrival line draws it, through the one card and the
// one row (Card.qml, NoticeRow.qml), then keeps a picture of it and says
// where its columns and frame were meant to stand.
// modules/quickshell/tests/notice-columns.py reads the pictures back to the
// ink, at the scale the shell ran at.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    FloatingWindow {
        visible: true
        implicitWidth: 440 + 40
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
    }

    // Once it's drawn, keep a picture of it and say what it was meant to be.
    Timer {
        interval: 500
        running: true
        onTriggered: {
            pop.grabToImage(picture => {
                if (!picture.saveToFile(`${Quickshell.shellDir}/pop.png`)) {
                    console.warn("FAIL couldn't keep the picture of pop");
                    Qt.exit(1);
                    return;
                }
                root.say();
            });
        }
    }

    function say() {
        console.log(`MEANT ${JSON.stringify({
            border: Theme.border,
            radius: Theme.radius,
            inset: pop.inset,
            text: Theme.level.name.pitch + Theme.space.rung.mark,
            width: pop.width,
            height: pop.height,
        })}`);
        Qt.callLater(() => Qt.exit(0));
    }
}
