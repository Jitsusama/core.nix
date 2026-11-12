// A shell whose files Qt should compile once and keep: the
// `the-shell-compiles-once` check (tests/compiled-once.sh) starts it again
// and again, and reads what Qt kept of it. It uses a type and a script of
// its own folder, as the shell does, and exits once it has made them.
import Quickshell
import QtQuick
import "kept.js" as Kept

ShellRoot {
    Thing {
        id: thing
    }

    Component.onCompleted: {
        if (thing.said !== Kept.word())
            console.warn(`the shell made something else: ${thing.said}`);
        // Said once the shell has loaded, as before it nothing hears it.
        Qt.callLater(() => Qt.exit(thing.said === Kept.word() ? 0 : 1));
    }
}
