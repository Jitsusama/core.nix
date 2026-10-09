// Joel's desktop shell: everything on screen that isn't a window. Each part
// is a file beside this one, and all of them draw from Theme.qml, which
// home.nix writes from jitsusama.theme. Quickshell's own guide to every type:
// https://quickshell.org/docs/
import Quickshell

ShellRoot {
    Notifications {}
    Polkit {}
    Volume {}

    Lock {
        id: lock
    }

    Launcher {
        lock: lock
    }
}
