// Drives the keyboard's backlight in Lights.qml through Quickshell itself,
// over a folder of lights laid out as the kernel's: the keyboard's light is
// found among others, nothing is heard before its firmware changes it, and
// each change written to brightness_hw_changed is heard once with its share,
// off as dark. It exits 0 once every case holds, and otherwise 1, saying
// which case didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string leds: Quickshell.env("XDG_RUNTIME_DIR") + "/leds"
    property int heard: 0

    Connections {
        target: Lights

        function onTurned() {
            root.heard++;
        }
    }

    // Each case: what the firmware writes (null: nothing), and what the
    // lights make of it.
    readonly property var cases: [{
        what: "the keyboard's light found, and nothing heard before it changes",
        writes: null,
        holds: () => Lights.backlight === `${root.leds}/test::kbd_backlight` && root.heard === 0
    }, {
        what: "the key turns it up",
        writes: "2",
        holds: () => root.heard === 1 && Math.abs(Lights.lit - 2 / 3) < 0.001 && !Lights.dark
    }, {
        what: "the key turns it off",
        writes: "0",
        holds: () => root.heard === 2 && Lights.lit === 0 && Lights.dark
    }]

    property int at: 0
    property int left: 0

    // Written in place, as the kernel changes the file.
    FileView {
        id: firmware
        path: `${root.leds}/test::kbd_backlight/brightness_hw_changed`
        atomicWrites: false
        printErrors: false
    }

    function begin() {
        const step = cases[at];
        if (step.writes !== null)
            firmware.setText(step.writes + "\n");
        left = 100;
        waiting.start();
    }

    // Quickshell hears the files in the background, so each case waits for
    // the lights to hold, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            if (step.holds()) {
                stop();
                if (++root.at === root.cases.length) {
                    console.log("PASS every case holds");
                    Qt.exit(0);
                } else {
                    root.begin();
                }
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: backlight "${Lights.backlight}", heard ${root.heard},`
                    + ` lit ${Lights.lit}, dark ${Lights.dark}`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: {
        Lights.leds = root.leds;
        begin();
    }
}
