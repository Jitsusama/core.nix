pragma Singleton
// What was taken lately, the one reader of the folders captures land in:
// screenshots, recordings and the camera's, for Capture's Recent
// (launcher/Capture.qml) and the image programs' own screenshot rows
// (modules/gimp, modules/swayimg). Each names the folders it shows
// (`want`); they're read once as the shell starts and again as the
// launcher opens (`read`, which Capture asks), and never otherwise, since
// nothing is taken while the launcher shows: Capture closes it first. So
// no folder is watched at rest (the shell's design, RSCH-20261029-0E8050).
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The folders read, each named by what shows it.
    property var folders: []
    // Every file in them, newest first: { path, folder, at }, at in
    // milliseconds since the epoch.
    property var files: []
    // Asked to read while reading, so the last ask always counts.
    property bool again: false

    function want(folder) {
        if (root.folders.includes(folder))
            return;
        root.folders = root.folders.concat([folder]);
        root.read();
    }

    function read() {
        if (root.folders.length === 0)
            return;
        if (finding.running)
            root.again = true;
        else
            finding.running = true;
    }

    // The newest file in a folder whose name passes, or none.
    function newestIn(folder, named) {
        return root.files.find(file => file.folder === folder && named.test(file.path)) ?? null;
    }

    function countIn(folder, named) {
        return root.files.filter(file => file.folder === folder && named.test(file.path)).length;
    }

    // find's lines, a file's time in seconds and its path each.
    function heard(lines) {
        root.files = lines.split("\n").filter(line => line.includes("\t"))
            .map(line => {
                const path = line.slice(line.indexOf("\t") + 1);
                return {
                    path: path,
                    folder: path.slice(0, path.lastIndexOf("/")),
                    at: Number(line.slice(0, line.indexOf("\t"))) * 1000
                };
            })
            .sort((a, b) => b.at - a.at);
    }

    Process {
        id: finding

        command: ["find"].concat(root.folders, ["-maxdepth", "1", "-type", "f", "-not", "-name",
            ".*", "-printf", "%T@\\t%p\\n"])
        onExited: {
            if (root.again) {
                root.again = false;
                finding.running = true;
            }
        }
        stdout: StdioCollector {
            onStreamFinished: root.heard(text)
        }
    }
}
