pragma Singleton
// What was taken lately, the one reader of the folders captures land in:
// screenshots, recordings and the camera's, for Capture's Recent
// (launcher/Capture.qml) and the image programs' own screenshot rows
// (modules/gimp, modules/swayimg). Each names the folders it shows
// (`want`); they're read once as the shell starts and again as the
// launcher opens (`read`, which Capture asks), and never otherwise, since
// nothing is taken while the launcher shows: Capture closes it first. So
// no folder is watched at rest (the shell's design, RSCH-20261029-0E8050).
//
// It finds screenshots by the words in them too (`search`), asking the
// screen-text daemon (modules/screen-text) over one connection of its own
// through Daemon.qml, made only once the first search is asked, so a
// machine nobody searches, or one without the daemon, holds no socket at
// rest. It watches no record, since a search's answer is all it needs:
// each text is asked once, and the last few answers are kept, so typing
// back over a word asks nothing again.
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

    // The last few searches, newest last: { words, records }, records null
    // while it's asked, each record a screenshot's path and the line that
    // matched.
    property var searches: []
    readonly property int keeps: 4

    // Said once a search's answer comes, for what showed it to look again.
    signal searched(string words)

    // The screenshots whose words hold every one of these, best first, as
    // [{ path, line }], or null while they're asked; asked at most once a
    // text, on the one connection.
    function search(words) {
        const text = root.searchedAs(words);
        if (text === "")
            return [];
        const kept = root.searches.find(search => search.words === text);
        if (kept)
            return kept.records;
        root.searches = root.searches.concat([{ words: text, records: null }])
            .slice(-root.keeps);
        root.pending = root.pending.concat([text]).slice(-root.keeps);
        reader.active = true;
        root.ask();
        return null;
    }

    // Words as they're asked, and as `searched` names them: in lower case,
    // their spaces single.
    function searchedAs(words) {
        return String(words).trim().toLowerCase().split(/\s+/).join(" ");
    }

    // Texts waiting for the connection, asked once it's made, the last few.
    property var pending: []

    function ask() {
        const daemon = reader.item;
        if (!daemon || daemon.wire === null)
            return;
        const asking = root.pending;
        root.pending = [];
        for (const text of asking)
            daemon.find({ ask: "search", kind: "screenshot", words: text }, answer => {
                root.answered(text, answer);
            });
    }

    // A text's answer kept, unless the daemon wasn't there to give one,
    // when it's let go to be asked again.
    function answered(text, answer) {
        if (answer === null || answer.error) {
            root.searches = root.searches.filter(search => search.words !== text);
            return;
        }
        const records = (answer.records ?? []).filter(record => typeof record.path === "string")
            .map(record => ({ path: record.path, line: String(record.line ?? "") }));
        root.searches = root.searches.map(search => search.words !== text ? search
            : { words: text, records: records });
        root.searched(text);
    }

    LazyLoader {
        id: reader

        active: false

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/screen-text.sock`
            kind: "screenshot"
            // No record is watched: a search's answer is given apart.
            watching: ({ keys: [] })
            onWireChanged: root.ask()
        }
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
