pragma Singleton
// Pasting into the window the launcher opened over, for whatever the
// launcher pastes: Super+E's emoji, and Super+V's copies, which pasted so
// first (launcher/Copies.qml). The clipboard's history pastes it: told the
// copy, by its number or as text, it puts it on the clipboard itself and,
// once niri says the launcher has let go of the keys, presses the window's
// paste keys through a virtual keyboard: Ctrl+Shift+V in a terminal, whose
// Ctrl+V is a key of its own, and Ctrl+V anywhere else
// (clipboard/src/main.rs). So nothing waits on the shell's thread, which
// can be busy for seconds after an opening, and the window reads the copy
// from the history rather than from the shell. With no history kept, the
// copy goes on the clipboard through Qt and the keys are pressed through
// wtype. With no window there, as on an empty workspace, it only copies.
// Once the history has been silent a second, or a paste it was asked for
// never landed, `pressing` says so, for the places that paste through it
// to say on their own rows and heads (Pasting.js).
import Quickshell
import QtQuick
import "Pasting.js" as Said

Singleton {
    id: pasting

    // The programs whose Ctrl+V is a key of their own, which paste with
    // Ctrl+Shift+V instead: the terminals.
    readonly property var terminals: ["kitty", "foot", "Alacritty", "org.wezfurlong.wezterm",
        "com.mitchellh.ghostty"]

    // The clipboard's history (launcher/Copies.qml), which copies and
    // pastes while it's connected: its tell sends it a line, and linked
    // says whether it's there to hear it. Null while no history is kept.
    property var helper: null

    // Whether the history answers, from the helper, watched so silence is
    // timed from when it began: from the start, until it first links.
    readonly property bool linked: helper !== null && helper.linked
    onLinkedChanged: heed()
    onHelperChanged: heed()

    // When the history went silent, and when a paste it was asked for last
    // never landed; null while it answers, and while none has missed.
    property var lostAt: null
    property var missedAt: null

    // What the places pasting through the history say of it, or nothing.
    property string pressing: ""

    function heed() {
        if (linked) {
            lostAt = null;
            quiet.stop();
        } else if (lostAt === null && helper !== null) {
            lostAt = Date.now();
            quiet.restart();
        }
        say();
    }

    // Told by the history that a paste never landed.
    function missed() {
        missedAt = Date.now();
        forgotten.restart();
        say();
    }

    function say() {
        pressing = Said.says({
            kept: helper !== null,
            lostAt: lostAt,
            missedAt: missedAt
        }, Date.now(), Words.of("clipboard").said ?? {});
    }

    Timer {
        id: quiet
        interval: Said.QUIET
        onTriggered: pasting.say()
    }
    Timer {
        id: forgotten
        interval: Said.MISSED
        onTriggered: pasting.say()
    }

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

    // Whether the history is there to copy and paste.
    function helped() {
        return pasting.helper !== null && pasting.helper.linked;
    }

    // Copies, through the history, which names the copy by what: its
    // number, or text spelled as JSON. Without the history, copied copies
    // it instead.
    function copy(what, copied) {
        if (helped())
            pasting.helper.tell(`copy ${what}`);
        else
            copied();
    }

    // Copies, and then pastes into the window under, if there is one.
    function paste(what, copied) {
        const window = under();
        if (window === null) {
            copy(what, copied);
            return;
        }
        const app = window.app_id ?? "";
        const terminal = pasting.terminals.includes(app);
        if (helped()) {
            // A fresh paste puts down what the last one missed.
            missedAt = null;
            forgotten.stop();
            say();
            pasting.helper.tell(`paste ${what} ${terminal ? "terminal" : "plain"}`);
            Report.say(`pastes into ${app}`);
            return;
        }
        copied();
        into = app;
        pressed.restart();
    }

    // The program pasted into, once the launcher has closed, with no
    // history to paste.
    property string into: ""

    // Without the history, the paste keys wait as a window the launcher
    // goes to does (Launcher.qml's later), for niri to have given the
    // window its keys back, or they'd reach the launcher.
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
