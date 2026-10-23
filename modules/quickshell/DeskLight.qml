// The desk light, a Litra on the monitor (modules/litra): whether it's on,
// how bright and how warm, as `litra watch` says once and again whenever it
// changes, whether its own buttons, a litra command or a camera changed it,
// so nothing is asked on a clock. The graphical role brings litra to every
// machine with this shell; with no light plugged in it says so, an empty
// list, and waits for one.
//
// Changes go through the litra command, which waits for the light's answer,
// since the light keeps only the last of two reports sent close together.
pragma Singleton
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // Each light: { node, on, percent, kelvin }.
    property var lights: []
    readonly property var light: lights.length > 0 ? lights[0] : null
    // What the light can do, as litra takes it.
    readonly property int warmest: 2700
    readonly property int coolest: 6500
    // Says how bright or how warm it just became, but not that it was put
    // out or lit, which a camera does by itself.
    signal brightened
    signal warmed

    function heard(line) {
        let now;
        try {
            now = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (!Array.isArray(now))
            return;
        const was = root.light;
        root.lights = now;
        const is = root.light;
        if (was === null || is === null || was.node !== is.node || !is.on)
            return;
        if (was.percent !== is.percent)
            root.brightened();
        if (was.kelvin !== is.kelvin)
            root.warmed();
    }

    function litra(words) {
        Niri.spawn(["litra"].concat(words));
    }

    // A share from 0 to 100, lighting it if it's out.
    function brighten(percent) {
        const kept = Math.max(0, Math.min(100, Math.round(percent)));
        if (root.light !== null && !root.light.on)
            litra(["on"]);
        litra(["brightness", `${kept}`]);
    }

    // Kelvin from warmest to coolest, in the hundreds litra rounds to.
    function warm(kelvin) {
        const kept = Math.max(warmest, Math.min(coolest, Math.round(kelvin / 100) * 100));
        litra(["temperature", `${kept}`]);
    }

    function turn(on) {
        litra([on ? "on" : "off"]);
    }

    Process {
        id: watching
        command: ["litra", "watch"]
        running: true
        stdout: SplitParser {
            onRead: line => root.heard(line)
        }
        // A watch that ends leaves nothing to vouch for the light; one more
        // try a little later, once, in case it was a light being replugged.
        onExited: {
            root.lights = [];
            if (!again.tried)
                again.start();
        }
    }

    Timer {
        id: again
        property bool tried: false
        interval: 5000
        onTriggered: {
            tried = true;
            watching.running = true;
        }
    }
}
