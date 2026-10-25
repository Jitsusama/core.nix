// Shows the screen's brightness for a moment whenever Joel picks one, as the
// same bar the volume uses. The backlight daemon (modules/backlight) tells a
// pick from its own changes for the light, and writes each pick to a file
// this watches, so the bar shows for the keys and never while the screen
// follows the room. The keyboard's backlight shows the same way when its key
// changes it (Lights.qml), in the same place: whichever changed last. So
// does the desk light (DeskLight.qml) when its brightness or warmth changes,
// by its own buttons or the launcher, but not when a camera lights it.
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

    Connections {
        target: DeskLight

        function onBrightened() {
            root.show("desk");
        }

        function onWarmed() {
            root.show("desk-warmth");
        }
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = ""
    }

    Meter {
        name: "brightness"
        glyph: Theme.glyph.brightness
        visible: root.shown === "screen"
        level: root.level
    }

    Meter {
        name: "keyboard-light"
        glyph: Theme.glyph.keyboard
        visible: root.shown === "keyboard"
        level: Lights.lit
        off: Lights.dark
    }

    Meter {
        name: "desk-light"
        glyph: Theme.glyph.lamp
        visible: root.shown === "desk"
        level: DeskLight.light === null ? 0 : DeskLight.light.percent / 100
    }

    // From warm, empty, to cool, full, said in kelvin.
    Meter {
        name: "desk-warmth"
        glyph: String.fromCodePoint(0xf05a8)
        visible: root.shown === "desk-warmth"
        level: DeskLight.light === null ? 0
            : (DeskLight.light.kelvin - DeskLight.warmest) / (DeskLight.coolest - DeskLight.warmest)
        reading: DeskLight.light === null ? "" : `${DeskLight.light.kelvin} K`
        widestReading: "6500 K"
    }
}
