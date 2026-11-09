// Joel's desktop shell: everything on screen that isn't a window. Each part
// is a file beside this one, and all of them draw from Theme.qml, which
// home.nix writes from jitsusama.theme. Quickshell's own guide to every type:
// https://quickshell.org/docs/
import qs
import Quickshell
import QtQuick

ShellRoot {
    Wallpaper {}
    Notifications {
        launcher: theLauncher
    }
    Polkit {}
    Askpass {}
    Volume {}
    Volume {
        input: true
    }
    Brightness {}
    KeyboardMap {}
    Power {}

    OpenBelow {
        id: theGrid
    }

    Lock {
        id: theLock
    }

    Launcher {
        id: theLauncher
        lock: theLock
        grid: theGrid
    }

    Margin {
        launcher: theLauncher
    }

    // The machine queue is drawn only in the launcher, so it's read only
    // while that's open (Jobs.qml).
    Binding {
        target: Jobs
        property: "showing"
        value: theLauncher.open
    }

    // Now's row tells a timer's time left only while the launcher is open
    // to show it (pieces/Timers.qml).
    Binding {
        target: HeldTimers
        property: "showing"
        value: theLauncher.open
    }

    // A session's step and what it has used are shown only in the launcher,
    // so they're taken only while it's open or walked (PiSource.qml).
    Binding {
        target: PiSource
        property: "showing"
        value: theLauncher.open || theLauncher.cataloguing
    }

    // Sessions opened behind the lock are placed once it's let go, since
    // niri refuses every move while locked (Sessions.qml).
    Binding {
        target: Sessions
        property: "behindTheLock"
        value: theLock.held
    }

    Arrivals {
        launcher: theLauncher
    }

    // What other modules add, such as dictation's look, each its own file.
    Variants {
        model: Options.surfaces

        LazyLoader {
            required property string modelData

            active: true
            source: Qt.resolvedUrl(`surfaces/${modelData}`)
        }
    }
}
