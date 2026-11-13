import QtQuick
import Quickshell

// Compiles each of the shell's files into Quickshell's cache, creating
// nothing, then quits (./starts-warm.sh runs it before a switch restarts the
// shell). A switch that changes Launcher.qml makes Qt throw away what it
// compiled of every file naming it, 212 of 234 on 12 November, so the new
// shell compiled them all before it listened: 4.0 s on the lab's niri,
// against 1.9 s warmed. Run with the shell's own files, and from its folder,
// it fills the cache under the names the shell reads it by.
ShellRoot {
    Component.onCompleted: {
        const files = JSON.parse(Quickshell.env("WARM_FILES"));
        let refused = 0;
        for (const file of files) {
            const compiled = Qt.createComponent(file, Component.PreferSynchronous);
            if (compiled.status === Component.Error) {
                refused += 1;
                console.warn(`warm: ${file}: ${compiled.errorString()}`);
            }
        }
        console.info(`warm: compiled ${files.length} files, ${refused} refused`);
        // Quickshell drops a quit said before its configuration has loaded,
        // which is still loading here, so it's said on the next turn.
        quitting.running = true;
    }

    Timer {
        id: quitting

        interval: 0
        onTriggered: Qt.quit()
    }
}
