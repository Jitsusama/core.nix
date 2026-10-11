// Joel's desktop shell: everything on screen that isn't a window. Each part
// is a file beside this one, and all of them draw from Theme.qml, which
// home.nix writes from jitsusama.theme. Quickshell's own guide to every type:
// https://quickshell.org/docs/
import Quickshell

ShellRoot {
    Wallpaper {}
    Notifications {}
    Polkit {}
    Volume {}
    Volume {
        input: true
    }
    Brightness {}
    Power {}
    Status {}

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
}
