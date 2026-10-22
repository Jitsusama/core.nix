// Shows the screen's brightness for a moment whenever Joel picks one, as the
// same bar the volume uses. The backlight daemon (modules/backlight) tells a
// pick from its own changes for the light, and writes each pick to a file
// this watches, so the bar shows for the keys and never while the screen
// follows the room.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    property real level: 0
    property bool shown: false

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
            root.shown = true;
            hide.restart();
        }
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = false
    }

    Meter {
        name: "brightness"
        glyph: String.fromCodePoint(0xf00e0)
        visible: root.shown
        level: root.level
    }
}
