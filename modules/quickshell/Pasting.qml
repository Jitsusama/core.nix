pragma Singleton
// Pasting into the window the launcher opened over, for whatever the
// launcher pastes: Super+E's emoji, and Super+V's copies, which pasted so
// first (launcher/Copies.qml). What's to paste is put on the clipboard, then,
// once the launcher has closed and niri has given that window its keys back,
// its paste keys are pressed for it through wtype: Ctrl+Shift+V in a
// terminal, whose Ctrl+V is a key of its own, and Ctrl+V anywhere else.
// With no window there, as on an empty workspace, it only copies.
import Quickshell
import QtQuick

Singleton {
    id: pasting

    // The programs whose Ctrl+V is a key of their own, which paste with
    // Ctrl+Shift+V instead: the terminals.
    readonly property var terminals: ["kitty", "foot", "Alacritty", "org.wezfurlong.wezterm",
        "com.mitchellh.ghostty"]

    // The window the launcher opened over, which niri gives the keys back
    // to as it closes; null on an empty workspace, with nowhere to paste.
    function under() {
        const id = Niri.current;
        return id === null ? null : Niri.windows[id] ?? null;
    }

    // Whether Return pastes, or only copies, for the foot to say which.
    function verb() {
        return under() === null ? "copy" : "paste";
    }

    // Copies, and then pastes into the window under, if there is one.
    function paste(copied) {
        const window = under();
        copied();
        if (window === null)
            return;
        into = window.app_id ?? "";
        pressed.restart();
    }

    // The program pasted into, once the launcher has closed.
    property string into: ""

    // The paste keys wait as a window the launcher goes to does
    // (Launcher.qml's later), for niri to have given the window its keys
    // back, or they'd reach the launcher.
    Timer {
        id: pressed
        interval: Theme.motion.short
        onTriggered: {
            const terminal = pasting.terminals.includes(pasting.into);
            Niri.spawn(["wtype", "-M", "ctrl"].concat(terminal ? ["-M", "shift"] : [], ["-k", "v"],
                terminal ? ["-m", "shift"] : [], ["-m", "ctrl"]));
            Report.say(`pastes into ${pasting.into}`);
        }
    }
}
