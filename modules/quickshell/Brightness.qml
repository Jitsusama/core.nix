// Shows the screen's brightness for a moment whenever Joel picks one, as the
// same bar the volume uses. The backlight daemon (modules/backlight) tells a
// pick from its own changes for the light, and writes each pick to a file
// this watches, so the bar shows for the keys and never while the screen
// follows the room. The keyboard's backlight shows the same way when its key
// changes it (Lights.qml), in the same place: whichever changed last.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    property real level: 0
    // Which light the bar is showing, or "" while it's hidden.
    property string shown: ""

    function show(light) {
        root.shown = light;
        hide.restart();
    }

    FileView {
        path: Quickshell.env("XDG_RUNTIME_DIR") + "/backlight-chosen"
        watchChanges: true
        // Every read blocks: a reload otherwise reads in the background,
        // and text() would still say the pick before.
        blockAllReads: true
        printErrors: false
        // The pick already made when the shell starts isn't news.
        onFileChanged: {
            reload();
            const level = parseFloat(text());
            if (isNaN(level))
                return;
            root.level = level;
            root.show("screen");
        }
    }

    Connections {
        target: Lights

        function onTurned() {
            root.show("keyboard");
        }
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = ""
    }

    Meter {
        name: "brightness"
        glyph: String.fromCodePoint(0xf00e0)
        visible: root.shown === "screen"
        level: root.level
    }

    Meter {
        name: "keyboard-light"
        glyph: String.fromCodePoint(0xf030c)
        visible: root.shown === "keyboard"
        level: Lights.lit
        off: Lights.dark
    }
}
