// Joel's desktop shell: everything on screen that isn't a window. Each part
// is a file beside this one, and all of them draw from Theme.qml, which
// home.nix writes from jitsusama.theme. Quickshell's own guide to every type:
// https://quickshell.org/docs/
import Quickshell
import QtQuick

ShellRoot {
    Wallpaper {}
    Notifications {
        launcher: launcher
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
        id: grid
    }

    Lock {
        id: lock
    }

    Launcher {
        id: launcher
        lock: lock
        grid: grid
    }

    Margin {
        launcher: launcher
    }

    Arrivals {
        launcher: launcher
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
