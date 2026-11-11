pragma Singleton
// What the clipboard holds now, for everything that acts on a copy:
// Tuple's joining a call whose link was copied (Tuple.qml), and the
// programs' acts in the launcher that take one, GIMP's picture, Neovim's
// path, Chrome's link and the reader's markdown. The clipboard's history
// says what it holds, over the socket it serves the launcher's copies on
// (clipboard/src/main.rs), so a copy starts nothing, and Copied.js says
// what the copy is.
//
// Its types are what the program offering it lists, and its text is read
// only when one of them is text, so a picture isn't read as words. A copy
// a password manager marks secret, or the clipboard cleared, has no types
// and no text, so nothing offers to act on what was copied before it.
// While the history isn't answering, what it said last stands.
//
// A path is acted on only while its file is there, which its folder is
// asked, through a folder model that hears the folder change, so a file
// removed after it was copied takes its act with it, and nothing reads
// the file itself, however large.
import Quickshell
import Quickshell.Io
import QtQuick
import Qt.labs.folderlistmodel
import "Copied.js" as Copying

Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME") ?? ""

    // The types the copy is offered as.
    property var types: []
    // Its text, its lines as they were, or "" for a copy with none.
    property string text: ""

    // The type to ask for its picture, or "" while it holds none.
    readonly property string picture: Copying.picture(types)
    // The one web address it is, or "".
    readonly property string link: Copying.link(text)
    // The file's path it names, whether or not the file is there.
    readonly property var named: Copying.place(text, home)
    // The file's path it names, with the line and column to open it at,
    // while the file is there, else null.
    readonly property var place: named !== null && found !== null
        && found.status === FolderListModel.Ready && found.count > 0 ? named : null

    // The folder asked whether the file named is there, made afresh for
    // each copy, so what it said of the last one is never read as this
    // one's while it looks.
    property var found: null

    Instantiator {
        model: root.named === null ? [] : [root.named]
        onObjectAdded: (index, object) => root.found = object
        onObjectRemoved: (index, object) => {
            if (root.found === object)
                root.found = null;
        }

        FolderListModel {
            required property var modelData

            folder: Copying.folderUrl(modelData.folder)
            nameFilters: [Copying.nameFilter(modelData.name)]
            showDirs: false
            showDotAndDotDot: false
            showHidden: true
            caseSensitive: true
        }
    }

    readonly property string socket:
        `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`

    // Whether the history is connected.
    property bool linked: false

    // A connection of its own, read only for what the clipboard holds now,
    // since the launcher's Copies, which reads the rest, isn't there where
    // the launcher keeps no copies. A fresh one each time, as Copies' is,
    // tried again as Daemon.qml's are: two seconds on and twice as long
    // after each refusal, a minute at most, and one with nothing at its
    // path once the socket appears.
    LazyLoader {
        id: link
        active: true

        Socket {
            path: root.socket
            connected: true
            parser: SplitParser {
                onRead: line => {
                    const copy = Copying.heard(line);
                    if (copy === null)
                        return;
                    root.types = copy.types;
                    root.text = copy.text;
                }
            }
            onConnectionStateChanged: {
                root.linked = connected;
                if (connected)
                    root.refusals = 0;
                else
                    root.tryAgain();
            }
            onError: failed => {
                if (failed !== root.serverNotFound)
                    root.tryAgain();
            }
        }
    }

    // QLocalSocket::ServerNotFoundError: nothing at the socket's path.
    readonly property int serverNotFound: 2

    // The socket's file, watched and never read, so one made after the
    // last try is tried at once; watched only while there's no connection,
    // since the watch covers the whole runtime folder.
    FileView {
        path: root.socket
        preload: false
        printErrors: false
        watchChanges: !root.linked
        onFileChanged: {
            if (!root.linked)
                root.connectAgain();
        }
    }

    function connectAgain() {
        again.stop();
        link.active = false;
        link.active = true;
    }

    // How many times it's been refused since it last answered.
    property int refusals: 0

    // A connection dropped says so twice, as its state and as an error,
    // so a try already waiting counts once.
    function tryAgain() {
        if (again.running)
            return;
        again.interval = Math.min(2000 * 2 ** refusals, 60000);
        refusals += 1;
        again.restart();
    }

    Timer {
        id: again
        interval: 2000
        onTriggered: root.connectAgain()
    }
}
